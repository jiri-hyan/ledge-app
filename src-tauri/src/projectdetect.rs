//! Návrh projektu ze složky (Nastavení → Projekty → Přidat): remote z `.git/config` (`resolve::read_origin`),
//! druh, příkaz a port z `package.json` / `vite.config.*`, exe u desktopových appek; logo hledá až
//! `icons.rs`. Jen čte soubory (mělce, žádný spawn) — výsledek je NÁVRH, šéf ho v UI upraví a uloží.
//! Plus seznam naklonovaných rep v `roots`, která ještě nejsou v projektech.

use crate::i18n::tr;
use crate::config::ProjectKind;
use crate::resolve;
use regex::Regex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static PORT_CFG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bport\s*:\s*(\d{2,5})\b").unwrap());
static PORT_ARG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:--port|-p)[ =](\d{2,5})\b").unwrap());

/// Podsložky, kde často žije web, když kořen `package.json` nemá.
const WEB_SUBDIRS: [&str; 5] = ["web", "frontend", "app", "client", "site"];
/// Kde hledat sestavené exe (relativně ke kořeni).
const EXE_DIRS: [&str; 5] = ["src-tauri/target/release", "target/release", "dist", "build", "bin"];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    pub id: String,
    pub name: String,
    pub kind: ProjectKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// lidsky: co se našlo a co je potřeba doplnit
    pub notes: Vec<String>,
}

/// kebab-case id ze jména, unikátní proti `taken` (`-2`, `-3`…).
pub fn make_id(name: &str, taken: &[String]) -> String {
    let mut base = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            base.push(c);
        } else if !base.ends_with('-') && !base.is_empty() {
            base.push('-');
        }
    }
    let base = base.trim_end_matches('-').to_string();
    let base = if base.is_empty() { "projekt".to_string() } else { base };
    let free = |id: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(id));
    if free(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|id| free(id)).unwrap()
}

fn package_manager(dir: &Path) -> &'static str {
    if dir.join("pnpm-lock.yaml").is_file() {
        "pnpm"
    } else if dir.join("yarn.lock").is_file() {
        "yarn"
    } else if dir.join("bun.lockb").is_file() || dir.join("bun.lock").is_file() {
        "bun"
    } else {
        "npm"
    }
}

/// (skript, jeho obsah) z `package.json`: `dev`, jinak `start`.
fn dev_script(pkg: &serde_json::Value) -> Option<(&'static str, String)> {
    let scripts = pkg.get("scripts")?;
    ["dev", "start"].into_iter().find_map(|k| scripts.get(k).and_then(|v| v.as_str()).map(|s| (k, s.to_string())))
}

/// Port z configu Vite (`port: 5180`), z argumentu skriptu (`--port 5180`), jinak výchozí známého nástroje.
pub fn guess_port(vite_cfg: Option<&str>, script: &str) -> Option<u16> {
    let num = |re: &Regex, s: &str| re.captures(s).and_then(|c| c[1].parse::<u16>().ok()).filter(|p| *p >= 80);
    vite_cfg
        .and_then(|t| num(&PORT_CFG_RE, t))
        .or_else(|| num(&PORT_ARG_RE, script))
        .or_else(|| {
            let s = script.to_lowercase();
            if s.contains("vite") {
                Some(5173)
            } else if s.contains("next") {
                Some(3000)
            } else if s.contains("astro") {
                Some(4321)
            } else {
                None
            }
        })
}

fn read_vite_cfg(dir: &Path) -> Option<String> {
    ["vite.config.ts", "vite.config.js", "vite.config.mjs", "vite.config.mts"]
        .iter()
        .find_map(|f| std::fs::read_to_string(dir.join(f)).ok())
}

fn first_exe(root: &Path) -> Option<String> {
    for d in EXE_DIRS.iter().map(|d| root.join(d)).chain(std::iter::once(root.to_path_buf())) {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut exes: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")))
            .filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().contains("uninst")))
            .collect();
        exes.sort();
        if let Some(e) = exes.first() {
            return Some(e.strip_prefix(root).unwrap_or(e).to_string_lossy().replace('/', "\\"));
        }
    }
    None
}

/// Obsazená id = uložená z configu + z rozpracovaného návrhu v Nastavení (bez duplicit).
pub fn merge_taken(mut saved: Vec<String>, draft: Option<Vec<String>>) -> Vec<String> {
    for id in draft.unwrap_or_default() {
        if !saved.iter().any(|t| t.eq_ignore_ascii_case(&id)) {
            saved.push(id);
        }
    }
    saved
}

/// Návrh projektu ze složky. `taken` = id, která už config má.
pub fn detect(dir: &Path, taken: &[String]) -> Result<Detected, String> {
    if !dir.is_dir() {
        return Err(trf!("Složka {} neexistuje.", "Folder {} doesn't exist.", dir.display()));
    }
    let folder = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| tr("Projekt", "Project").into());
    let mut notes = Vec::new();
    let remote = resolve::read_origin(dir);
    if remote.is_none() {
        notes.push(tr("Není to git repo s remote — složka se uloží napevno (na jiném PC ji uprav).", "It's not a git repo with a remote — the folder is saved as is (adjust it on another PC).").into());
    }
    // web v kořeni, nebo v obvyklé podsložce
    let web = std::iter::once((dir.to_path_buf(), None))
        .chain(WEB_SUBDIRS.iter().map(|s| (dir.join(s), Some(s.to_string()))))
        .find_map(|(d, sub)| {
            let text = std::fs::read_to_string(d.join("package.json")).ok()?;
            let pkg: serde_json::Value = serde_json::from_str(&text).ok()?;
            let (name, script) = dev_script(&pkg)?;
            Some((d, sub, name, script))
        });
    let mut out = Detected {
        id: make_id(&folder, taken),
        name: folder,
        kind: ProjectKind::Dev,
        path: dir.to_string_lossy().to_string(),
        remote,
        subdir: None,
        run: None,
        port: None,
        exe: None,
        notes,
    };
    if let Some((d, sub, script_name, script)) = web {
        let pm = package_manager(&d);
        out.run = Some(if pm == "npm" && script_name == "start" { "npm start".into() } else { format!("{pm} run {script_name}") });
        out.port = guess_port(read_vite_cfg(&d).as_deref(), &script);
        out.subdir = sub;
        if out.port.is_none() {
            out.notes.push(tr("Port se nepodařilo zjistit — doplň ho, jinak se přidělí volný.", "Couldn't find out the port — fill it in, otherwise a free one is assigned.").into());
        }
    } else if let Some(exe) = first_exe(dir) {
        out.kind = ProjectKind::App;
        out.exe = Some(exe);
    } else {
        out.notes.push(tr("Nenašel jsem dev skript ani .exe — vyplň příkaz nebo program.", "I found neither a dev script nor an .exe — fill in a command or a program.").into());
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

/// Git repa (přímé podsložky `roots`), která ještě nejsou v projektech — podle složky ani remote.
pub fn candidates(roots: &[String], known_dirs: &[PathBuf], known_remotes: &[String]) -> Vec<Candidate> {
    let norm = |p: &Path| p.to_string_lossy().to_lowercase().trim_end_matches(['\\', '/']).to_string();
    let dirs: Vec<String> = known_dirs.iter().map(|d| norm(d)).collect();
    let remotes: Vec<_> = known_remotes.iter().filter_map(|r| resolve::parse_remote(r)).collect();
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !p.join(".git").exists() || dirs.contains(&norm(&p)) {
                continue;
            }
            let remote = resolve::read_origin(&p);
            if remote.as_deref().and_then(resolve::parse_remote).is_some_and(|r| remotes.contains(&r)) {
                continue;
            }
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            out.push(Candidate { path: p.to_string_lossy().to_string(), name, remote });
            if out.len() >= 200 {
                return out;
            }
        }
    }
    out.sort_by_key(|c| c.name.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_taken_draft_ids_make_next_free() {
        let taken = merge_taken(vec!["app".into()], Some(vec!["App".into(), "app-2".into()]));
        assert_eq!(taken.len(), 2);
        assert_eq!(make_id("app", &taken), "app-3");
        assert_eq!(merge_taken(vec!["app".into()], None), vec!["app".to_string()]);
    }

    #[test]
    fn make_id_table() {
        let taken = vec!["web".to_string(), "web-2".to_string()];
        for (name, want) in [("Atlas", "atlas"), ("Pixel Press", "pixel-press"), ("  --x__y  ", "x-y"), ("web", "web-3"), ("ěšč", "projekt"), ("WEB", "web-3")] {
            assert_eq!(make_id(name, &taken), want, "{name}");
        }
    }

    #[test]
    fn guess_port_table() {
        let cases = [
            (Some("server: { port: 5210, strictPort: true }"), "vite", Some(5210)),
            (None, "vite --port 5180", Some(5180)),
            (None, "next dev -p 3005", Some(3005)),
            (None, "vite", Some(5173)),
            (None, "next dev", Some(3000)),
            (None, "node server.js", None),
            (Some("port: 12"), "node x", None),
        ];
        for (cfg, script, want) in cases {
            assert_eq!(guess_port(cfg, script), want, "{script}");
        }
    }

    fn write(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn detect_web_app_and_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        // web s pnpm a portem ve vite.config, git remote
        let web = tmp.path().join("Moje Appka");
        write(&web.join("package.json"), r#"{"name":"x","scripts":{"dev":"vite","build":"vite build"}}"#);
        write(&web.join("pnpm-lock.yaml"), "");
        write(&web.join("vite.config.ts"), "export default { server: { port: 5300 } }");
        write(&web.join(".git/config"), "[remote \"origin\"]\n\turl = https://github.com/a/moje-appka.git\n");
        let d = detect(&web, &[]).unwrap();
        assert_eq!((d.id.as_str(), d.kind, d.run.as_deref(), d.port), ("moje-appka", ProjectKind::Dev, Some("pnpm run dev"), Some(5300)));
        assert_eq!(d.remote.as_deref(), Some("https://github.com/a/moje-appka.git"));
        assert!(d.notes.is_empty(), "{:?}", d.notes);
        // web v podsložce, npm start
        let sub = tmp.path().join("mono");
        write(&sub.join("web/package.json"), r#"{"scripts":{"start":"node server.js"}}"#);
        let d = detect(&sub, &[]).unwrap();
        assert_eq!((d.subdir.as_deref(), d.run.as_deref(), d.port), (Some("web"), Some("npm start"), None));
        assert!(d.notes.iter().any(|n| n.contains("Port")));
        // desktopová appka se sestaveným exe
        let app = tmp.path().join("hra");
        write(&app.join("target/release/hra.exe"), "MZ");
        write(&app.join("target/release/uninstall.exe"), "MZ");
        let d = detect(&app, &["hra".into()]).unwrap();
        assert_eq!((d.id.as_str(), d.kind, d.exe.as_deref()), ("hra-2", ProjectKind::App, Some(r"target\release\hra.exe")));
        // nic poznatelného
        let empty = tmp.path().join("prazdne");
        std::fs::create_dir_all(&empty).unwrap();
        let d = detect(&empty, &[]).unwrap();
        assert!(d.run.is_none() && d.exe.is_none() && d.notes.len() == 2);
        assert!(detect(&tmp.path().join("neni"), &[]).is_err());
    }

    #[test]
    fn candidates_skip_known() {
        let tmp = tempfile::tempdir().unwrap();
        let mk = |name: &str, remote: &str| {
            let p = tmp.path().join(name);
            write(&p.join(".git/config"), &format!("[remote \"origin\"]\n\turl = {remote}\n"));
            p
        };
        let a = mk("alfa", "https://github.com/x/alfa.git");
        mk("beta", "https://github.com/x/beta.git");
        mk("Gama", "git@github.com:x/gama.git");
        std::fs::create_dir_all(tmp.path().join("neni-repo")).unwrap();
        let roots = vec![tmp.path().to_string_lossy().to_string()];
        let c = candidates(&roots, &[a], &["x/gama".into()]);
        assert_eq!(c.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["beta"]);
        assert_eq!(candidates(&roots, &[], &[]).len(), 3);
    }
}
