//! Běží Ledge uvnitř cizího balíčku MSIX („bublina“)?
//!
//! **Proč.** Claude desktop je balíček MSIX (`Claude_pzs8sxrjxfjjc`). Proces spuštěný z něj — terminál
//! v Claude session, instalátor, Ledge spuštěný odtud — dědí kontext balíčku: zápisy do `%APPDATA%`,
//! `%LOCALAPPDATA%` a HKCU jdou do soukromé kopie balíčku
//! (`%LOCALAPPDATA%\Packages\<rodina>\LocalCache\{Roaming,Local}\…`, soukromý hive registru). Explorer
//! při přihlášení tu kopii nevidí → hodnota Run „v bublině“ nic nespustí a config/data žijí jen tam.
//!
//! **Jak (dvě vrstvy).**
//! 1. `GetCurrentPackageFullName` (kernel32): plné jméno balíčku (`Jméno_Verze_Arch_ResourceId_Vydavatel`),
//!    `APPMODEL_ERROR_NO_PACKAGE` = bez identity balíčku.
//! 2. **Sonda virtualizace** ([`probe_virtualization`]) — ZMĚŘENO 18. 10. 2026: procesy z Claude Code
//!    session NEMAJÍ identitu balíčku (API vrací 15700), a přesto se jejich zápis do `%LOCALAPPDATA%`
//!    objevil v `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local`. Samotné API tedy bublinu
//!    nepozná. Sonda zapíše malý soubor do kořene `%LOCALAPPDATA%`, podívá se, jestli se objevil v
//!    `Packages\*\LocalCache\Local` (rodina balíčku = jméno složky), a hned ho smaže. Jednou při startu.
//!
//! Výsledek se po dobu procesu nezmění → zjišťuje se jednou ([`ctx`]). Rozhodnutí jsou čisté funkce
//! ([`classify`], [`combine`], [`parse_full`], [`bubble_twin`], [`after_pid`]); WinAPI je jen v [`query`]
//! a [`wait_for_predecessor`], souborový systém v [`probe_virtualization`].
//!
//! **Cesta ven:** `outside.rs` (Plánovač úloh), nová instance počká na konec staré (`--after-pid`).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windows::Win32::Foundation::{APPMODEL_ERROR_NO_PACKAGE, CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::core::PWSTR;

/// Argument nové instance: počkej, až skončí proces s tímhle PID (stará instance z bubliny).
pub const AFTER_PID: &str = "--after-pid";
/// Jak dlouho nová instance nejvýš čeká na konec staré (pak pokračuje — single-instance ji případně
/// předá staré a skončí, nic se neztratí).
const WAIT_MS: u32 = 15_000;

/// Balíček, ve kterém proces běží.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    /// `Claude_1.0.0.0_x64__pzs8sxrjxfjjc`
    pub full: String,
    /// `Claude_pzs8sxrjxfjjc` — jméno složky v `%LOCALAPPDATA%\Packages`
    pub family: String,
    /// `Claude` — pro text v UI
    pub name: String,
    /// proces má identitu balíčku (API); `false` = jen virtualizace zjištěná sondou (`full` = rodina)
    pub identity: bool,
}

/// Výsledek dotazu na balíček.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ctx {
    Outside,
    Inside(Package),
    /// jiná chyba WinAPI (kód) — bere se jako „mimo“, jen se zaloguje
    Unknown(u32),
}

impl Ctx {
    pub fn package(&self) -> Option<&Package> {
        match self {
            Ctx::Inside(p) => Some(p),
            _ => None,
        }
    }
}

/// Plné jméno balíčku → rodina a jméno. Formát `Jméno_Verze_Arch_ResourceId_Vydavatel` (ResourceId bývá
/// prázdné → `__`); jméno balíčku `_` obsahovat nesmí. Jiný tvar → rodina = celé jméno (nic nehádáme).
pub fn parse_full(full: &str) -> Package {
    let full = full.trim();
    let parts: Vec<&str> = full.split('_').collect();
    let name = parts.first().copied().unwrap_or("").to_string();
    let family = match parts.as_slice() {
        [n, _ver, _arch, _res, publisher] if !n.is_empty() && !publisher.is_empty() => format!("{n}_{publisher}"),
        _ => full.to_string(),
    };
    Package { full: full.to_string(), family, name, identity: true }
}

/// Balíček známý jen z rodiny (sonda virtualizace): `Claude_pzs8sxrjxfjjc` → jméno `Claude`.
pub fn from_family(family: &str) -> Package {
    let family = family.trim();
    let name = family.rsplit_once('_').map(|(n, _)| n).filter(|n| !n.is_empty()).unwrap_or(family);
    Package { full: family.to_string(), family: family.to_string(), name: name.to_string(), identity: false }
}

/// Identita z API + výsledek sondy → kontext. Identita vyhrává; bez ní rozhoduje sonda. Čistá funkce.
pub fn combine(api: Ctx, probed: Option<&str>) -> Ctx {
    match (api, probed) {
        (Ctx::Inside(p), _) => Ctx::Inside(p),
        (_, Some(fam)) if !fam.trim().is_empty() => Ctx::Inside(from_family(fam)),
        (other, _) => other,
    }
}

/// Sonda virtualizace: zapíše `<la>\ledge-pkgprobe-….tmp`, najde jeho fyzickou kopii v
/// `<la>\Packages\<rodina>\LocalCache\Local` a vrátí rodinu; soubor (i kopii) hned smaže. `la` =
/// `%LOCALAPPDATA%` (v testech dočasná složka). Zápis selže → `None` (nic nehádáme).
pub fn probe_virtualization(la: &Path) -> Option<String> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    probe_named(la, &format!("ledge-pkgprobe-{}-{nanos}.tmp", std::process::id()))
}

fn probe_named(la: &Path, name: &str) -> Option<String> {
    let file = la.join(name);
    std::fs::write(&file, b"ledge pkgctx probe").ok()?;
    let found = std::fs::read_dir(la.join("Packages")).ok().and_then(|dir| {
        dir.flatten().find_map(|e| {
            let twin = e.path().join("LocalCache").join("Local").join(name);
            twin.is_file().then(|| (e.file_name().to_string_lossy().into_owned(), twin))
        })
    });
    let _ = std::fs::remove_file(&file);
    found.map(|(family, twin)| {
        // smazání přes virtuální cestu kopii obvykle smaže taky; jistota
        let _ = std::fs::remove_file(twin);
        family
    })
}

/// Celé zjištění: API, a když neřekne „uvnitř“, sonda v `%LOCALAPPDATA%`.
pub fn detect() -> Ctx {
    let api = classify(query());
    if matches!(api, Ctx::Inside(_)) {
        return api;
    }
    let probed = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).and_then(|la| probe_virtualization(&la));
    combine(api, probed.as_deref())
}

/// Výsledek WinAPI (`Ok(plné jméno)` / `Err(kód)`) → kontext. Čistá funkce.
pub fn classify(res: Result<String, u32>) -> Ctx {
    match res {
        Ok(full) if !full.trim().is_empty() => Ctx::Inside(parse_full(&full)),
        Ok(_) => Ctx::Unknown(0),
        Err(code) if code == APPMODEL_ERROR_NO_PACKAGE.0 => Ctx::Outside,
        Err(code) => Ctx::Unknown(code),
    }
}

/// Tenká vrstva nad `GetCurrentPackageFullName` (dvě volání: délka, pak jméno).
pub fn query() -> Result<String, u32> {
    unsafe {
        let mut len = 0u32;
        let first = GetCurrentPackageFullName(&mut len, None);
        if first == APPMODEL_ERROR_NO_PACKAGE {
            return Err(first.0);
        }
        if first != ERROR_INSUFFICIENT_BUFFER && first != ERROR_SUCCESS {
            return Err(first.0);
        }
        let mut buf = vec![0u16; len.max(1) as usize];
        let res = GetCurrentPackageFullName(&mut len, Some(PWSTR(buf.as_mut_ptr())));
        if res != ERROR_SUCCESS {
            return Err(res.0);
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..end]))
    }
}

static CTX: OnceLock<Ctx> = OnceLock::new();

/// Kontext tohoto procesu (zjištěný jednou).
pub fn ctx() -> &'static Ctx {
    CTX.get_or_init(detect)
}

/// Balíček, ve kterém Ledge běží; `None` = mimo bublinu (normální stav).
pub fn current() -> Option<&'static Package> {
    ctx().package()
}

/// Řádek do logu při startu.
pub fn startup_line(c: &Ctx) -> String {
    match c {
        Ctx::Outside => "pkgctx: mimo balíček MSIX".into(),
        Ctx::Inside(p) => format!(
            "pkgctx: běží UVNITŘ balíčku {} ({}) — autostart se nezapisuje, config a data jsou v jeho kopii (LocalCache)",
            p.full,
            if p.identity { "identita balíčku" } else { "bez identity, zjištěno sondou virtualizace" }
        ),
        Ctx::Unknown(code) => format!("pkgctx: balíček nejde zjistit (chyba {code}) — beru jako mimo"),
    }
}

/// Fyzické umístění virtualizované cesty v bublině: `%LOCALAPPDATA%\X` →
/// `%LOCALAPPDATA%\Packages\<rodina>\LocalCache\Local\X`, `%APPDATA%\X` → `…\LocalCache\Roaming\X`.
/// Cesta už uvnitř `%LOCALAPPDATA%\Packages` nebo mimo AppData → `None` (není co překládat). Čistá funkce.
pub fn bubble_twin(path: &Path, local_appdata: &Path, appdata: &Path, family: &str) -> Option<PathBuf> {
    let packages = local_appdata.join("Packages");
    if strip_ci(path, &packages).is_some() {
        return None;
    }
    let cache = packages.join(family).join("LocalCache");
    if let Some(rel) = strip_ci(path, local_appdata) {
        return Some(cache.join("Local").join(rel));
    }
    strip_ci(path, appdata).map(|rel| cache.join("Roaming").join(rel))
}

/// `path` bez předpony `base` (Windows: bez ohledu na velikost písmen a `/` vs `\`).
fn strip_ci(path: &Path, base: &Path) -> Option<PathBuf> {
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_string();
    let (p, b) = (norm(path), norm(base));
    if b.is_empty() || p.len() < b.len() || !p[..b.len()].eq_ignore_ascii_case(&b) {
        return None;
    }
    let rest = &p[b.len()..];
    if rest.is_empty() {
        return Some(PathBuf::new());
    }
    rest.strip_prefix('\\').map(PathBuf::from)
}

/// Fyzická kopie virtualizované cesty pro TENTO proces (jen uvnitř balíčku).
pub fn twin_of(path: &Path) -> Option<PathBuf> {
    let pkg = current()?;
    let la = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    let ra = std::env::var_os("APPDATA").map(PathBuf::from)?;
    bubble_twin(path, &la, &ra, &pkg.family)
}

/// Exe, které spustit mimo bublinu: běží-li Ledge z virtualizované cesty a jeho fyzická kopie v bublině
/// existuje, spustí se ta (mimo bublinu by virtualizovaná cesta nemusela existovat). Jinak běžící exe.
pub fn relaunch_target(running: &Path, twin: Option<PathBuf>, twin_exists: impl Fn(&Path) -> bool) -> PathBuf {
    twin.filter(|t| twin_exists(t)).unwrap_or_else(|| running.to_path_buf())
}

/// `--after-pid 123` / `--after-pid=123` z argumentů. Čistá funkce.
pub fn after_pid<I: IntoIterator<Item = String>>(args: I) -> Option<u32> {
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == AFTER_PID {
            return it.next().and_then(|v| v.trim().parse().ok()).filter(|&p| p != 0);
        }
        if let Some(v) = a.strip_prefix(AFTER_PID).and_then(|r| r.strip_prefix('=')) {
            return v.trim().parse().ok().filter(|&p| p != 0);
        }
    }
    None
}

/// Nová instance spuštěná „Restartovat mimo“: počkej (≤ 15 s), než stará skončí — jinak by ji
/// single-instance plugin chytil a nová by hned skončila. Čeká jen na proces se stejným jménem exe
/// (PID mohl mezitím připadnout cizímu procesu). Volat v `run()` PŘED Tauri builderem.
pub fn wait_for_predecessor() {
    let Some(pid) = after_pid(std::env::args().skip(1)) else { return };
    if pid == std::process::id() {
        return;
    }
    let own = std::env::current_exe().ok().and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_lowercase()));
    let theirs = crate::ports::process_image(pid).and_then(|p| Path::new(&p).file_name().map(|f| f.to_string_lossy().to_lowercase()));
    if own.is_none() || own != theirs {
        return;
    }
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(h, WAIT_MS);
            let _ = CloseHandle(h);
        }
    }
}

/// Stav pro UI (`pkg_context`): balíček a kde fyzicky leží config z bubliny.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub package: Option<Package>,
    /// fyzická složka configu v bublině (jen uvnitř balíčku)
    pub bubble_config_dir: Option<String>,
    /// složka configu, jak ji vidí tento proces (`%APPDATA%\Ledge`)
    pub config_dir: String,
}

pub fn view() -> View {
    let dir = crate::config::config_dir();
    View {
        package: current().cloned(),
        bubble_config_dir: twin_of(&dir).map(|p| p.to_string_lossy().into_owned()),
        config_dir: dir.to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = "Claude_1.0.3218.0_x64__pzs8sxrjxfjjc";

    #[test]
    fn classify_table() {
        let claude = Package { full: FULL.into(), family: "Claude_pzs8sxrjxfjjc".into(), name: "Claude".into(), identity: true };
        #[rustfmt::skip]
        let cases: Vec<(&str, Result<String, u32>, Ctx)> = vec![
            ("mimo balíček",                Err(15700),                 Ctx::Outside),
            ("Claude",                      Ok(FULL.into()),            Ctx::Inside(claude)),
            ("jiná chyba",                  Err(87),                    Ctx::Unknown(87)),
            ("prázdné jméno",               Ok("  ".into()),            Ctx::Unknown(0)),
        ];
        for (name, res, want) in cases {
            assert_eq!(classify(res), want, "{name}");
        }
    }

    #[test]
    fn parse_full_table() {
        #[rustfmt::skip]
        let cases: Vec<(&str, &str, &str)> = vec![
            // (plné jméno, rodina, jméno)
            (FULL,                                                   "Claude_pzs8sxrjxfjjc",                 "Claude"),
            ("Microsoft.WindowsTerminal_1.21.0.0_x64__8wekyb3d8bbwe", "Microsoft.WindowsTerminal_8wekyb3d8bbwe", "Microsoft.WindowsTerminal"),
            ("App_1.0.0.0_neutral_split.scale-100_abc123",          "App_abc123",                           "App"),
            ("divny-tvar",                                           "divny-tvar",                           "divny-tvar"),
            ("A_1_x64__",                                            "A_1_x64__",                            "A"),
        ];
        for (full, family, name) in cases {
            let p = parse_full(full);
            assert_eq!((p.family.as_str(), p.name.as_str()), (family, name), "{full}");
        }
    }

    #[test]
    fn bubble_twin_table() {
        let la = Path::new(r"C:\Users\x\AppData\Local");
        let ra = Path::new(r"C:\Users\x\AppData\Roaming");
        let fam = "Claude_pzs8sxrjxfjjc";
        let cache = r"C:\Users\x\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache";
        #[rustfmt::skip]
        let cases: Vec<(&str, Option<String>)> = vec![
            (r"C:\Users\x\AppData\Local\Ledge\ledge.exe",   Some(format!(r"{cache}\Local\Ledge\ledge.exe"))),
            (r"c:\users\X\appdata\local\Ledge\ledge.exe",   Some(format!(r"{cache}\Local\Ledge\ledge.exe"))),
            (r"C:/Users/x/AppData/Local/Ledge/ledge.exe",   Some(format!(r"{cache}\Local\Ledge\ledge.exe"))),
            (r"C:\Users\x\AppData\Roaming\Ledge",           Some(format!(r"{cache}\Roaming\Ledge"))),
            // už fyzická cesta v bublině / jiný balíček → nic
            (r"C:\Users\x\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\Ledge\ledge.exe", None),
            (r"C:\Users\x\AppData\Local\Packages\Jiny_abc\x.exe", None),
            // mimo AppData (vývojový release, Program Files) → nic
            (r"C:\Users\x\Desktop\Ledge\src-tauri\target\release\ledge.exe", None),
            (r"C:\Users\x\AppData\LocalLow\x.exe",          None),
            (r"C:\Users\x\AppData\Localni\x.exe",           None),
        ];
        for (p, want) in cases {
            assert_eq!(bubble_twin(Path::new(p), la, ra, fam), want.map(PathBuf::from), "{p}");
        }
    }

    #[test]
    fn relaunch_target_prefers_existing_twin() {
        let run = Path::new(r"C:\Users\x\AppData\Local\Ledge\ledge.exe");
        let twin = PathBuf::from(r"C:\Users\x\AppData\Local\Packages\C_p\LocalCache\Local\Ledge\ledge.exe");
        assert_eq!(relaunch_target(run, Some(twin.clone()), |_| true), twin);
        assert_eq!(relaunch_target(run, Some(twin), |_| false), run);
        assert_eq!(relaunch_target(run, None, |_| true), run);
    }

    #[test]
    fn after_pid_table() {
        let a = |v: &[&str]| after_pid(v.iter().map(|s| s.to_string()));
        #[rustfmt::skip]
        let cases: Vec<(&[&str], Option<u32>)> = vec![
            (&[],                                   None),
            (&["--autostart"],                      None),
            (&["--after-pid", "4242"],              Some(4242)),
            (&["--autostart", "--after-pid=17"],    Some(17)),
            (&["--after-pid"],                      None),
            (&["--after-pid", "x"],                 None),
            (&["--after-pid", "0"],                 None),
            (&["--after-pidx=5"],                   None),
        ];
        for (args, want) in cases {
            assert_eq!(a(args), want, "{args:?}");
        }
    }

    #[test]
    fn combine_table() {
        let with_id = Ctx::Inside(parse_full(FULL));
        let probed = Ctx::Inside(from_family("Claude_pzs8sxrjxfjjc"));
        #[rustfmt::skip]
        let cases: Vec<(&str, Ctx, Option<&str>, Ctx)> = vec![
            ("identita vyhrává",            with_id.clone(),  Some("Jiny_x"),               with_id.clone()),
            ("bez identity, sonda našla",   Ctx::Outside,     Some("Claude_pzs8sxrjxfjjc"), probed.clone()),
            ("chyba API, sonda našla",      Ctx::Unknown(5),  Some("Claude_pzs8sxrjxfjjc"), probed),
            ("nikde nic",                   Ctx::Outside,     None,                         Ctx::Outside),
            ("chyba API, sonda nic",        Ctx::Unknown(5),  None,                         Ctx::Unknown(5)),
            ("prázdná rodina",              Ctx::Outside,     Some(" "),                    Ctx::Outside),
        ];
        for (name, api, fam, want) in cases {
            assert_eq!(combine(api, fam), want, "{name}");
        }
        let p = from_family("Claude_pzs8sxrjxfjjc");
        assert_eq!((p.name.as_str(), p.family.as_str(), p.identity), ("Claude", "Claude_pzs8sxrjxfjjc", false));
        assert_eq!(from_family("bezpodtrzitka").name, "bezpodtrzitka");
    }

    #[test]
    fn probe_without_virtualization_finds_nothing_and_cleans_up() {
        // bez virtualizace: soubor zůstane jen v kořeni → nic, a po sondě v kořeni nic nezbude
        let la = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(la.path().join("Packages").join("Jiny_abc").join("LocalCache").join("Local")).unwrap();
        assert_eq!(probe_virtualization(la.path()), None);
        assert_eq!(std::fs::read_dir(la.path()).unwrap().count(), 1, "jen Packages");
        // bez složky Packages → nic
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(probe_virtualization(empty.path()), None);
        assert_eq!(std::fs::read_dir(empty.path()).unwrap().count(), 0);
        // nezapisovatelná cesta → nic (žádný pád)
        assert_eq!(probe_virtualization(&la.path().join("neexistuje").join("x")), None);
    }

    #[test]
    fn probe_with_twin_reports_family_and_removes_both() {
        // virtualizaci nasimulujeme: kopie se známým jménem už v bublině leží
        let la = tempfile::tempdir().unwrap();
        let local = la.path().join("Packages").join("Claude_pzs8sxrjxfjjc").join("LocalCache").join("Local");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::create_dir_all(la.path().join("Packages").join("Jiny_abc")).unwrap();
        std::fs::write(local.join("p.tmp"), b"x").unwrap();
        assert_eq!(probe_named(la.path(), "p.tmp").as_deref(), Some("Claude_pzs8sxrjxfjjc"));
        assert!(!local.join("p.tmp").exists() && !la.path().join("p.tmp").exists());
    }

    #[test]
    fn query_is_consistent() {
        // bez vedlejších účinků: z Claude session (bublina) Inside, z terminálu mimo Outside
        let c = classify(query());
        eprintln!("{}", startup_line(&c));
        if let Ctx::Inside(p) = &c {
            assert!(!p.family.is_empty() && !p.name.is_empty());
        }
    }

    /// Živé ověření celé detekce včetně sondy (zapíše a hned smaže soubor v SKUTEČNÉM `%LOCALAPPDATA%`,
    /// proto `#[ignore]`): `cargo test --lib pkgctx::tests::detect_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn detect_live() {
        eprintln!("{}", startup_line(&detect()));
    }
}
