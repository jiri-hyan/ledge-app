//! Průvodce, krok Projekty: mělký sken obvyklých složek na git repa (hloubka ≤ 2, bez `node_modules`,
//! worktree se `.git` souborem se přeskočí) a návrh projektu z toho, co v repu leží (package.json
//! skripty → dev server s portem z vite configu; electron / Cargo.toml / .sln → aplikace).
//! Jen čte soubory s rozpočtem a termínem — žádný git, žádný spawn. Rozhodování je čisté ([`suggest`]).

use crate::config::{Project, ProjectKind};
use regex::Regex;
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Instant, UNIX_EPOCH};

/// Složky, do kterých sken nevstupuje (závislosti, výstupy buildů, systémové).
const SKIP: &[&str] = &["node_modules", "target", "dist", "build", "out", "bin", "obj", "vendor", "__pycache__", "venv", "appdata", "$recycle.bin"];
/// Nejvíc navštívených složek na jeden sken (pojistka proti obřím Dokumentům).
const MAX_DIRS: usize = 6000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FoundRepo {
    /// jméno složky
    pub name: String,
    pub path: String,
    pub remote: Option<String>,
    /// poslední pohyb HEAD (commit, pull, checkout) z reflogu, jinak změna indexu; `None` = nevím
    pub last_ms: Option<i64>,
    /// "dev" | "app" | "none"
    pub kind: &'static str,
    /// co bylo poznáno (např. „vite · pnpm · port 5173“) nebo proč nejde přidat
    pub hint: String,
    /// hotový návrh projektu; `None` = nevím, jak repo spustit
    pub project: Option<Project>,
    /// projekt se stejnou složkou nebo remote už v Ledge je
    pub already: bool,
}

/// Obvyklá místa s repy pro `USERPROFILE` (a OneDrive, kam Windows přesměrovává Plochu a Dokumenty).
pub fn default_roots(var: impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = var("USERPROFILE").filter(|h| !h.trim().is_empty()) {
        let h = PathBuf::from(home);
        for sub in ["Desktop", "Documents", "source\\repos", "repos", "dev", "projects"] {
            out.push(h.join(sub));
        }
    }
    if let Some(od) = var("OneDrive").filter(|h| !h.trim().is_empty()) {
        let od = PathBuf::from(od);
        out.push(od.join("Desktop"));
        out.push(od.join("Documents"));
    }
    out
}

/// Bez duplicit (velikost písmen, koncové lomítko), jen existující složky, v původním pořadí.
pub fn dedupe_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    dirs.into_iter()
        .filter(|d| d.is_dir())
        .filter(|d| seen.insert(norm(&d.to_string_lossy())))
        .collect()
}

/// Porovnatelný tvar cesty: malá písmena, `/` → `\`, bez koncového lomítka.
pub fn norm(p: &str) -> String {
    p.trim().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

fn skip_dir(name: &str) -> bool {
    name.starts_with('.') || SKIP.contains(&name.to_lowercase().as_str())
}

/// Najde repa (složka s `.git` adresářem) v kořenech a jejich podsložkách do hloubky 2.
pub fn scan(roots: &[PathBuf], deadline: Instant) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut budget = MAX_DIRS;
    for root in roots {
        walk(root, 0, &mut out, &mut seen, &mut budget, deadline);
    }
    out
}

fn walk(dir: &Path, depth: u8, out: &mut Vec<PathBuf>, seen: &mut std::collections::HashSet<String>, budget: &mut usize, deadline: Instant) {
    if *budget == 0 || Instant::now() >= deadline || !seen.insert(norm(&dir.to_string_lossy())) {
        return;
    }
    *budget -= 1;
    let git = dir.join(".git");
    if git.is_dir() {
        out.push(dir.to_path_buf());
        return;
    }
    if git.is_file() || depth >= 2 {
        return; // linked worktree / submodul — patří hlavnímu checkoutu
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for ent in rd.flatten() {
        let Ok(ft) = ent.file_type() else { continue };
        // symlinky a junctiony (např. „Dokumenty/Moje hudba“) se neprocházejí — smyčky, cizí disky
        if !ft.is_dir() || ft.is_symlink() || skip_dir(&ent.file_name().to_string_lossy()) {
            continue;
        }
        walk(&ent.path(), depth + 1, out, seen, budget, deadline);
    }
}

// ---------- fakta o repu ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pm {
    #[default]
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

/// Co o repu víme ze souborů (vstup čisté [`suggest`]).
#[derive(Debug, Default, Clone)]
pub struct Facts {
    pub package: Option<serde_json::Value>,
    pub pm: Pm,
    /// obsah `vite.config.*`
    pub vite_config: Option<String>,
    /// obsah `Cargo.toml` v kořeni
    pub cargo: Option<String>,
    pub has_sln: bool,
    /// sestavené `.exe` u .sln (relativně k repu, nejnovější)
    pub sln_exe: Option<String>,
}

fn read_small(p: &Path, max: u64) -> Option<String> {
    let meta = std::fs::metadata(p).ok()?;
    (meta.is_file() && meta.len() <= max).then(|| std::fs::read_to_string(p).ok()).flatten()
}

pub fn facts(dir: &Path) -> Facts {
    let package = read_small(&dir.join("package.json"), 256 * 1024).and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok());
    let declared = package.as_ref().and_then(|p: &serde_json::Value| p.get("packageManager")).and_then(|v| v.as_str()).unwrap_or("");
    let pm = match () {
        _ if declared.starts_with("pnpm") || dir.join("pnpm-lock.yaml").is_file() => Pm::Pnpm,
        _ if declared.starts_with("yarn") || dir.join("yarn.lock").is_file() => Pm::Yarn,
        _ if declared.starts_with("bun") || dir.join("bun.lockb").is_file() || dir.join("bun.lock").is_file() => Pm::Bun,
        _ => Pm::Npm,
    };
    let vite_config = ["ts", "js", "mjs", "mts", "cjs"].iter().find_map(|e| read_small(&dir.join(format!("vite.config.{e}")), 64 * 1024));
    let cargo = read_small(&dir.join("Cargo.toml"), 64 * 1024);
    let has_sln = std::fs::read_dir(dir).is_ok_and(|rd| rd.flatten().any(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("sln"))));
    let sln_exe = if has_sln { newest_built_exe(dir) } else { None };
    Facts { package, pm, vite_config, cargo, has_sln, sln_exe }
}

/// Nejnovější `.exe` v `bin\Release|Debug` nebo `x64\Release` (do hloubky 5, s rozpočtem).
fn newest_built_exe(dir: &Path) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![(dir.to_path_buf(), 0u8)];
    let mut budget = 800;
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            budget -= 1;
            if budget == 0 {
                return best.and_then(|(_, p)| rel(dir, &p));
            }
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_lowercase();
            if ft.is_dir() && !ft.is_symlink() && depth < 5 && !name.starts_with('.') && !matches!(name.as_str(), "node_modules" | "obj" | "packages") {
                stack.push((p, depth + 1));
            } else if ft.is_file() && name.ends_with(".exe") && built_dir(&p) {
                let t = e.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
                if best.as_ref().is_none_or(|(bt, _)| t > *bt) {
                    best = Some((t, p));
                }
            }
        }
    }
    best.and_then(|(_, p)| rel(dir, &p))
}

fn built_dir(p: &Path) -> bool {
    let s = norm(&p.to_string_lossy());
    ["\\bin\\release\\", "\\bin\\debug\\", "\\x64\\release\\"].iter().any(|k| s.contains(k))
}

fn rel(base: &Path, p: &Path) -> Option<String> {
    p.strip_prefix(base).ok().map(|r| r.to_string_lossy().into_owned())
}

// ---------- návrh projektu (čisté) ----------

static SCRIPT_PORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:--port|\s-p)[ =](\d{2,5})\b").unwrap());
static CONFIG_PORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bport\s*:\s*(\d{2,5})\b").unwrap());

/// `name` ze sekce `[package]` v Cargo.toml (workspace bez balíčku → `None`).
pub fn cargo_name(toml: &str) -> Option<String> {
    let mut in_pkg = false;
    for l in toml.lines().map(str::trim) {
        if l.starts_with('[') {
            in_pkg = l == "[package]";
        } else if in_pkg && let Some(v) = l.strip_prefix("name").map(str::trim_start).and_then(|r| r.strip_prefix('=')) {
            return Some(v.trim().trim_matches('"').to_string()).filter(|n| !n.is_empty());
        }
    }
    None
}

fn port_of(re: &Regex, s: &str) -> Option<u16> {
    re.captures(s).and_then(|c| c[1].parse().ok()).filter(|p| *p >= 80)
}

/// Port dev serveru: z příkazu ve skriptu (pevný → nepředávat), z vite configu, z výchozí hodnoty
/// nástroje (vite 5173, next 3000, astro 4321). `(port, předat --port)`.
pub fn dev_port(script: &str, vite_config: Option<&str>) -> (Option<u16>, bool) {
    if let Some(p) = port_of(&SCRIPT_PORT, &format!(" {script}")) {
        return (Some(p), false);
    }
    if let Some(p) = vite_config.and_then(|c| port_of(&CONFIG_PORT, c)) {
        return (Some(p), true);
    }
    let has = |tool: &str| script.split(|c: char| c.is_whitespace() || c == '&' || c == ';').any(|w| w == tool);
    let default = if has("vite") {
        Some(5173)
    } else if script.contains("next dev") {
        Some(3000)
    } else if script.contains("astro dev") {
        Some(4321)
    } else {
        None
    };
    (default, default.is_some())
}

fn run_script(pm: Pm, script: &str) -> String {
    match pm {
        Pm::Npm if script == "start" => "npm start".into(),
        Pm::Npm => format!("npm run {script}"),
        Pm::Pnpm => format!("pnpm {script}"),
        Pm::Yarn => format!("yarn {script}"),
        Pm::Bun => format!("bun run {script}"),
    }
}

fn pm_name(pm: Pm) -> &'static str {
    match pm {
        Pm::Npm => "npm",
        Pm::Pnpm => "pnpm",
        Pm::Yarn => "yarn",
        Pm::Bun => "bun",
    }
}

/// Id projektu z názvu: malá ASCII písmena, číslice a `-` (diakritika pryč).
pub fn slug(name: &str) -> String {
    const FROM: &str = "áäčďéěëíňóöřšťúůüýžÁÄČĎÉĚËÍŇÓÖŘŠŤÚŮÜÝŽ";
    const TO: &str = "aacdeeeinoorstuuuyzAACDEEEINOORSTUUUYZ";
    let mut s = String::new();
    for c in name.chars() {
        let c = FROM.chars().position(|f| f == c).and_then(|i| TO.chars().nth(i)).unwrap_or(c);
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_matches('-');
    if s.is_empty() { "projekt".into() } else { s.chars().take(48).collect::<String>().trim_end_matches('-').to_string() }
}

fn base_project(name: &str, path: &str, remote: Option<&str>, kind: ProjectKind) -> Project {
    Project {
        id: slug(name),
        name: name.into(),
        enabled: true,
        kind,
        path: Some(path.into()),
        remote: remote.map(Into::into),
        subdir: None,
        run: None,
        port: None,
        pass_port: false,
        open_browser: kind == ProjectKind::Dev,
        exe: None,
        app_name: None,
        args: vec![],
        accent: None,
        monogram: None,
        logo: None,
        links: vec![],
        look: None,
    }
}

/// Návrh projektu z fakt o repu: `(druh, popis, projekt)`. Čistá funkce.
pub fn suggest(f: &Facts, name: &str, path: &str, remote: Option<&str>) -> (&'static str, String, Option<Project>) {
    let pkg = f.package.as_ref();
    let script = |k: &str| pkg.and_then(|p| p.pointer(&format!("/scripts/{k}"))).and_then(|v| v.as_str()).map(str::to_string);
    let dep = |k: &str| pkg.is_some_and(|p| ["dependencies", "devDependencies"].iter().any(|d| p.get(d).and_then(|o| o.get(k)).is_some()));
    if dep("electron") {
        let mut p = base_project(name, path, remote, ProjectKind::App);
        p.exe = Some("node_modules/electron/dist/electron.exe".into());
        p.args = vec![".".into()];
        p.run = script("build").map(|_| run_script(f.pm, "build"));
        return ("app", format!("electron · {}", pm_name(f.pm)), Some(p));
    }
    if let Some((key, cmd)) = ["dev", "start"].iter().find_map(|k| script(k).map(|c| (*k, c))) {
        let mut p = base_project(name, path, remote, ProjectKind::Dev);
        let (port, pass) = dev_port(&cmd, f.vite_config.as_deref());
        p.run = Some(run_script(f.pm, key));
        p.port = port;
        p.pass_port = pass;
        let port_txt = port.map(|n| format!(" · port {n}")).unwrap_or_default();
        return ("dev", format!("{} · {}{port_txt}", run_script(f.pm, key), pm_name(f.pm)), Some(p));
    }
    if let Some(bin) = f.cargo.as_deref().and_then(cargo_name) {
        let mut p = base_project(name, path, remote, ProjectKind::App);
        p.exe = Some(format!("target/release/{bin}.exe"));
        p.run = Some("cargo build --release".into());
        return ("app", format!("Rust · {bin}.exe"), Some(p));
    }
    if f.has_sln {
        return match &f.sln_exe {
            Some(exe) => {
                let mut p = base_project(name, path, remote, ProjectKind::App);
                p.exe = Some(exe.replace('\\', "/"));
                ("app", format!(".NET · {}", exe.rsplit('\\').next().unwrap_or(exe)), Some(p))
            }
            None => ("app", crate::i18n::tr("Visual Studio řešení bez sestaveného .exe — sestav ho a přidej později", "Visual Studio solution without a built .exe — build it and add it later").into(), None),
        };
    }
    ("none", crate::i18n::tr("nevím, jak ho spustit (chybí skript dev/start, Cargo.toml i .sln)", "I don't know how to run it (no dev/start script, Cargo.toml or .sln)").into(), None)
}

// ---------- čas poslední aktivity ----------

static REFLOG_TS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"> (\d{9,11}) [+-]\d{4}\t").unwrap());

/// Čas posledního řádku reflogu (`.git/logs/HEAD`) v ms. Čistá funkce nad koncem souboru.
pub fn reflog_last_ms(tail: &str) -> Option<i64> {
    tail.lines().rev().find_map(|l| REFLOG_TS.captures(l)).and_then(|c| c[1].parse::<i64>().ok()).map(|s| s * 1000)
}

fn last_activity(repo: &Path) -> Option<i64> {
    let git = repo.join(".git");
    let from_reflog = (|| {
        let mut f = std::fs::File::open(git.join("logs").join("HEAD")).ok()?;
        let len = f.metadata().ok()?.len();
        f.seek(SeekFrom::Start(len.saturating_sub(4096))).ok()?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).ok()?;
        reflog_last_ms(&String::from_utf8_lossy(&buf))
    })();
    from_reflog.or_else(|| {
        let t = std::fs::metadata(git.join("index")).and_then(|m| m.modified()).ok()?;
        Some(t.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64)
    })
}

/// Celý krok: sken + fakta + návrh, seřazeno od naposledy měněného. `known` = (cesty, remote)
/// projektů, které už v Ledge jsou (normalizované přes [`norm`] a `resolve::parse_remote`).
pub fn discover(roots: &[PathBuf], deadline: Instant, known_paths: &[String], known_remotes: &[crate::resolve::OwnerRepo]) -> Vec<FoundRepo> {
    let mut out: Vec<FoundRepo> = scan(roots, deadline)
        .into_iter()
        .map(|dir| {
            let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let path = dir.to_string_lossy().into_owned();
            let remote = crate::resolve::read_origin(&dir);
            let (kind, hint, project) = suggest(&facts(&dir), &name, &path, remote.as_deref());
            let already = known_paths.contains(&norm(&path))
                || remote.as_deref().and_then(crate::resolve::parse_remote).is_some_and(|r| known_remotes.contains(&r));
            FoundRepo { name, last_ms: last_activity(&dir), path, remote, kind, hint, project, already }
        })
        .collect();
    out.sort_by(|a, b| b.last_ms.cmp(&a.last_ms).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn put(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn scan_depth_skips_and_worktrees() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        std::fs::create_dir_all(r.join("a/.git")).unwrap(); // hloubka 1
        std::fs::create_dir_all(r.join("group/b/.git")).unwrap(); // hloubka 2
        std::fs::create_dir_all(r.join("x/y/c/.git")).unwrap(); // hloubka 3 → ne
        std::fs::create_dir_all(r.join("node_modules/d/.git")).unwrap();
        std::fs::create_dir_all(r.join(".hidden/e/.git")).unwrap();
        std::fs::create_dir_all(r.join("a/nested/.git")).unwrap(); // uvnitř repa → ne
        put(&r.join("wt/.git"), "gitdir: C:/x/.git/worktrees/wt\n"); // worktree → ne
        let found = scan(&[r.to_path_buf(), r.to_path_buf()], Instant::now() + Duration::from_secs(10));
        let mut names: Vec<String> = found.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn suggest_table() {
        let pkg = |j: &str| Some(serde_json::from_str(j).unwrap());
        let f = |package, pm, vite: Option<&str>| Facts { package, pm, vite_config: vite.map(String::from), ..Default::default() };
        type Case<'a> = (Facts, &'a str, Option<&'a str>, Option<u16>, bool);
        #[rustfmt::skip]
        let cases: Vec<Case> = vec![
            (f(pkg(r#"{"scripts":{"dev":"vite"}}"#), Pm::Npm, None),                      "dev", Some("npm run dev"), Some(5173), true),
            (f(pkg(r#"{"scripts":{"dev":"vite"}}"#), Pm::Pnpm, Some("server: { port: 4321 }")), "dev", Some("pnpm dev"), Some(4321), true),
            (f(pkg(r#"{"scripts":{"dev":"vite --port 5210"}}"#), Pm::Npm, None),          "dev", Some("npm run dev"), Some(5210), false),
            (f(pkg(r#"{"scripts":{"dev":"next dev -p 3005"}}"#), Pm::Yarn, None),         "dev", Some("yarn dev"), Some(3005), false),
            (f(pkg(r#"{"scripts":{"dev":"next dev"}}"#), Pm::Npm, None),                  "dev", Some("npm run dev"), Some(3000), true),
            (f(pkg(r#"{"scripts":{"start":"node server.js"}}"#), Pm::Npm, None),          "dev", Some("npm start"), None, false),
            (f(pkg(r#"{"scripts":{"start":"x"}}"#), Pm::Bun, None),                       "dev", Some("bun run start"), None, false),
            (f(pkg(r#"{"scripts":{"build":"tsc"},"devDependencies":{"electron":"1"}}"#), Pm::Npm, None), "app", Some("npm run build"), None, false),
            (f(pkg(r#"{"scripts":{"test":"x"}}"#), Pm::Npm, None),                        "none", None, None, false),
            (f(None, Pm::Npm, None),                                                       "none", None, None, false),
        ];
        for (facts, kind, run, port, pass) in cases {
            let (k, _hint, p) = suggest(&facts, "Moje Appka", r"C:\r\moje", Some("acme/app"));
            assert_eq!(k, kind, "{:?}", facts.package);
            if kind == "none" {
                assert!(p.is_none());
                continue;
            }
            let p = p.unwrap();
            assert_eq!((p.run.as_deref(), p.port, p.pass_port), (run, port, pass), "{:?}", facts.package);
            assert_eq!((p.id.as_str(), p.path.as_deref(), p.remote.as_deref()), ("moje-appka", Some(r"C:\r\moje"), Some("acme/app")));
            let cfg = crate::config::Config { projects: vec![p], ..Default::default() };
            crate::config::validate(&cfg).unwrap();
        }
    }

    #[test]
    fn suggest_rust_and_dotnet() {
        let cargo = Facts { cargo: Some("[workspace]\nname = \"ws\"\n\n[package]\nauthors = [\"A\"]\nname = \"tool\"\n[dependencies]\nname = \"x\"\n".into()), ..Default::default() };
        let (k, _, p) = suggest(&cargo, "tool", r"C:\r\tool", None);
        let p = p.unwrap();
        assert_eq!((k, p.kind, p.exe.as_deref(), p.run.as_deref()), ("app", ProjectKind::App, Some("target/release/tool.exe"), Some("cargo build --release")));
        let sln = Facts { has_sln: true, sln_exe: Some(r"App\bin\Release\net8.0\App.exe".into()), ..Default::default() };
        assert_eq!(suggest(&sln, "App", r"C:\r\app", None).2.unwrap().exe.as_deref(), Some("App/bin/Release/net8.0/App.exe"));
        let unbuilt = Facts { has_sln: true, ..Default::default() };
        assert!(suggest(&unbuilt, "App", r"C:\r\app", None).2.is_none());
    }

    #[test]
    fn facts_read_from_disk() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path();
        put(&d.join("package.json"), "\u{feff}{\"packageManager\":\"pnpm@9\",\"scripts\":{\"dev\":\"vite\"}}");
        put(&d.join("vite.config.ts"), "export default { server: { port: 5300 } }");
        let f = facts(d);
        assert_eq!(f.pm, Pm::Pnpm);
        assert_eq!(dev_port("vite", f.vite_config.as_deref()), (Some(5300), true));
        put(&d.join("x.sln"), "");
        put(&d.join("App/bin/Release/App.exe"), "MZ");
        put(&d.join("App/obj/Release/App.exe"), "MZ");
        assert_eq!(facts(d).sln_exe.as_deref(), Some(r"App\bin\Release\App.exe"));
    }

    #[test]
    fn slug_and_reflog() {
        for (i, o) in [("Acme.Web", "acme-web"), ("Účetnictví 2026", "ucetnictvi-2026"), ("--", "projekt"), ("a__b", "a-b")] {
            assert_eq!(slug(i), o);
        }
        let tail = "abc def A <a@b> 1700000000 +0200\tcommit: x\n0 1 A <a@b> 1760000000 +0100\tcheckout: moving\n";
        assert_eq!(reflog_last_ms(tail), Some(1_760_000_000_000));
        assert_eq!(reflog_last_ms("nesmysl"), None);
    }

    #[test]
    fn default_roots_from_env() {
        let var = |k: &str| match k {
            "USERPROFILE" => Some(r"C:\Users\ada".to_string()),
            "OneDrive" => Some(r"C:\Users\ada\OneDrive".to_string()),
            _ => None,
        };
        let r = default_roots(var);
        assert!(r.contains(&PathBuf::from(r"C:\Users\ada\source\repos")));
        assert!(r.contains(&PathBuf::from(r"C:\Users\ada\OneDrive\Documents")));
        assert_eq!(r.len(), 8);
        assert!(default_roots(|_| None).is_empty());
    }
}
