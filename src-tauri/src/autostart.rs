//! Spuštění po přihlášení do Windows: hodnota `Ledge` v `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! **Kam Run ukazuje.** Existuje-li nainstalovaná kopie (NSIS instalátor), Run míří **vždy na ni** — i když
//! právě běží vývojový release z `target\release`; ten Run nikdy nepřepíše na sebe, takže `cargo build` /
//! `tauri build` nemusí kvůli zamčenému exe zastavovat „produkční“ Ledge. Neexistuje-li, zapíše se cesta
//! k právě běžícímu exe (dřívější chování) — po přesunu se sama opraví při dalším startu.
//! Rozhodnutí je čistá funkce [`decide`]; registr čte/píše jen [`apply`], a to jen v release buildu.
//!
//! **Kam NSIS `installMode: currentUser` instaluje** (ověřeno ve zdroji, ne odhadnuto): šablona
//! `installer.nsi` z tauri-bundleru (zabudovaná v `@tauri-apps/cli` 2.12.3, viz
//! <https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi>)
//! nastaví `StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"` → `%LOCALAPPDATA%\Ledge\ledge.exe`
//! (`productName` = „Ledge“, hlavní binárka = jméno cargo balíčku, `mainBinaryName` nenastaveno).
//! Pak volá `RestorePreviousInstallLocation` (předchozí volba adresáře vyhrává) a uživatel může adresář
//! v průvodci změnit. Skutečné umístění proto čteme z toho, co instalátor zapíše:
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Ledge` → `InstallLocation` (v uvozovkách)
//! a `MainBinaryName`; `%LOCALAPPDATA%\Ledge` je až záloha. Dokumentace:
//! <https://v2.tauri.app/distribute/windows-installer/> („installed in the `%LOCALAPPDATA%` folder“).
//! Odinstalace (ne update) hodnotu Run `Ledge` sama maže (`DeleteRegValue HKCU …\Run "${PRODUCTNAME}"`).
//!
//! **Přejmenování Notch → Ledge (0.2.0).** Starý produkt zapisoval hodnotu Run `Notch`. [`apply`] ji
//! smaže, když míří na `notch.exe` (rozhodnutí [`drop_legacy_run`]) — jinak by po přihlášení startoval
//! i starý Notch a oba by chtěly port mostu 47800. Klíč Uninstall starého Notche nechává být.
//!
//! **Bublina MSIX** (`pkgctx.rs`): běží-li Ledge uvnitř cizího balíčku (spuštěný z Claude desktop),
//! zápis do Run by šel do soukromého hivu balíčku a Explorer by ho při přihlášení neviděl — [`gate`]
//! pak registr nechá být (nic nezapíše ani nesmaže). Nastavení to ukáže u přepínače a nabídne
//! „Restartovat mimo“; instance mimo bublinu pak Run zapíše při startu.

use crate::i18n::tr;
use std::path::{Path, PathBuf};

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "Ledge";
/// Hodnota Run starého produktu (před přejmenováním na Ledge) a jeho binárka.
const LEGACY_NAME: &str = "Notch";
const LEGACY_BIN: &str = "notch.exe";
/// Klíč, který zapisuje NSIS šablona Tauri (`UNINSTKEY` = `…\Uninstall\${PRODUCTNAME}`).
const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Ledge";
const PRODUCT_DIR: &str = "Ledge";
const DEFAULT_BIN: &str = "ledge.exe";

pub fn command_for(exe: &Path) -> String {
    format!("\"{}\" --autostart", exe.display())
}

/// Co udělat s hodnotou Run.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Write(String),
    Delete,
    Keep,
}

/// Politika autostartu — čistá funkce, bez registru.
/// `running` = cesta běžícího exe, `installed` = nainstalovaná kopie (jen pokud na disku existuje),
/// `current` = stávající hodnota Run `Ledge`.
pub fn decide(
    running: &Path,
    installed: Option<&Path>,
    current: Option<&str>,
    enabled: bool,
) -> Action {
    if enabled {
        let want = command_for(installed.unwrap_or(running));
        if current == Some(want.as_str()) {
            Action::Keep
        } else {
            Action::Write(want)
        }
    } else {
        match current {
            Some(cmd) if belongs_to_us(cmd, running, installed) => Action::Delete,
            _ => Action::Keep,
        }
    }
}

/// Patří hodnota Run Ledge? Ano, když míří na běžící exe, na nainstalovanou kopii, nebo na exe
/// stejného jména spouštěné s naším `--autostart` (starší cesta, např. dřív `target\release`).
fn belongs_to_us(cmd: &str, running: &Path, installed: Option<&Path>) -> bool {
    let Some(exe) = exe_of(cmd) else { return false };
    let exe = Path::new(exe);
    if same_path(exe, running) || installed.is_some_and(|i| same_path(exe, i)) {
        return true;
    }
    let same_name = match (exe.file_name(), running.file_name()) {
        (Some(a), Some(b)) => a
            .to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy()),
        _ => false,
    };
    same_name && cmd.trim_end().ends_with(" --autostart")
}

/// Smazat starou hodnotu Run `Notch`? Jen když míří na `notch.exe` (v jakékoli složce, bez ohledu na
/// velikost písmen) — cizí program pod stejným jménem hodnoty necháme být. Čistá funkce, bez registru.
pub fn drop_legacy_run(current: Option<&str>) -> bool {
    current
        .and_then(exe_of)
        .and_then(|exe| Path::new(exe).file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case(LEGACY_BIN)))
        .unwrap_or(false)
}

/// Cesta k exe z příkazové řádky (`"C:\…\x.exe" args` nebo `C:\x.exe args`).
fn exe_of(cmd: &str) -> Option<&str> {
    let cmd = cmd.trim_start();
    let exe = if let Some(rest) = cmd.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        cmd.split(' ').next()?
    };
    (!exe.is_empty()).then_some(exe)
}

/// Windows cesty: bez ohledu na velikost písmen a `/` vs `\`.
fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    norm(a) == norm(b)
}

/// Kandidáti na nainstalovanou kopii v pořadí přednosti — čistá funkce, existenci ověřuje volající.
/// `install_location`/`main_binary` = hodnoty z klíče Uninstall, `local_appdata` = `%LOCALAPPDATA%`.
pub fn install_candidates(
    install_location: Option<&str>,
    main_binary: Option<&str>,
    local_appdata: Option<&Path>,
) -> Vec<PathBuf> {
    let bin = main_binary
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .unwrap_or(DEFAULT_BIN);
    let mut out = Vec::new();
    if let Some(dir) = install_location
        .map(|d| d.trim().trim_matches('"'))
        .filter(|d| !d.is_empty())
    {
        out.push(Path::new(dir).join(bin));
    }
    if let Some(la) = local_appdata {
        let p = la.join(PRODUCT_DIR).join(DEFAULT_BIN);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

fn installed_exe() -> Option<PathBuf> {
    let uninst = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(UNINSTALL)
        .ok();
    let loc: Option<String> = uninst
        .as_ref()
        .and_then(|k| k.get_value("InstallLocation").ok());
    let bin: Option<String> = uninst
        .as_ref()
        .and_then(|k| k.get_value("MainBinaryName").ok());
    let la = crate::dirs::localappdata();
    install_candidates(loc.as_deref(), bin.as_deref(), la.as_deref())
        .into_iter()
        .find(|p| p.is_file())
}

/// Kam tichý instalátor (`/S`) Ledge nainstaluje a odkud ho po aktualizaci spustit: existující
/// instalace (instalátor ji přes `RestorePreviousInstallLocation` přepíše na místě), jinak výchozí
/// `%LOCALAPPDATA%\Ledge\ledge.exe` (první spuštění instalátoru z vývojové kopie).
pub(crate) fn install_target() -> Option<PathBuf> {
    installed_exe().or_else(|| {
        let la = crate::dirs::localappdata();
        install_candidates(None, None, la.as_deref()).into_iter().next()
    })
}

/// Smí [`apply`] sáhnout do registru?
#[derive(Debug, PartialEq, Eq)]
pub enum Gate {
    Registry,
    /// ladicí build / testy — registr nikdy
    DebugBuild,
    /// uvnitř balíčku MSIX (plné jméno) — zápis by šel do bubliny, Explorer by ho neviděl
    InPackage(String),
}

/// Politika „sáhnout do registru?“ — čistá funkce. Ladicí build vyhrává (testy nikdy nezapisují).
pub fn gate(debug: bool, package: Option<&str>) -> Gate {
    if debug {
        Gate::DebugBuild
    } else if let Some(p) = package {
        Gate::InPackage(p.to_string())
    } else {
        Gate::Registry
    }
}

pub fn apply(enabled: bool) -> Result<(), String> {
    // ladicí build (tauri dev) ani testy se do registru nikdy nedostanou — rozhoduje profil, ne cesta:
    // testy ze sdíleného `target-agents\debug` cestou `\target\debug\` neprošly a přepsaly šéfův autostart
    let debug = cfg!(debug_assertions) || cfg!(test);
    // ladicí build ani balíček nezjišťuje (sonda by v testech zapsala do skutečného %LOCALAPPDATA%)
    let package = if debug { None } else { crate::pkgctx::current().map(|p| p.full.as_str()) };
    match gate(debug, package) {
        Gate::Registry => {}
        Gate::DebugBuild => return Ok(()),
        Gate::InPackage(p) => {
            crate::log(&format!("autostart: běží uvnitř balíčku {p} — Run nezapisuji ani nemažu (Explorer by ho neviděl)"));
            return Ok(());
        }
    }
    let exe = std::env::current_exe().map_err(|e| crate::errors::io(tr("Spuštění s Windows nejde nastavit", "Can't set up start with Windows"), &e))?;
    let installed = installed_exe();
    let (run, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(RUN)
        .map_err(|e| crate::errors::io(tr("Spuštění s Windows nejde nastavit", "Can't set up start with Windows"), &e))?;
    let current: Option<String> = run.get_value(NAME).ok();
    match decide(&exe, installed.as_deref(), current.as_deref(), enabled) {
        Action::Keep => {}
        Action::Write(want) => run.set_value(NAME, &want).map_err(|e| crate::errors::io(tr("Spuštění s Windows nejde nastavit", "Can't set up start with Windows"), &e))?,
        Action::Delete => match run.delete_value(NAME) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(crate::errors::io(tr("Spuštění s Windows nejde vypnout", "Can't turn off start with Windows"), &e)),
        },
    }
    let legacy: Option<String> = run.get_value(LEGACY_NAME).ok();
    if drop_legacy_run(legacy.as_deref()) {
        match run.delete_value(LEGACY_NAME) {
            Ok(()) => crate::log(&format!("autostart: smazána stará hodnota Run {LEGACY_NAME} = {}", legacy.unwrap_or_default())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(crate::errors::io(tr("Starou položku spuštění s Windows nejde odstranit", "Can't remove the old start-with-Windows entry"), &e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEV: &str = r"C:\Users\x\Desktop\notch\src-tauri\target\release\ledge.exe";
    const INST: &str = r"C:\Users\x\AppData\Local\Ledge\ledge.exe";
    const OLD: &str = r"C:\Users\x\old\target\release\ledge.exe";

    fn cmd(p: &str) -> String {
        command_for(Path::new(p))
    }

    #[test]
    fn command_quotes_path_with_spaces() {
        let c = command_for(std::path::Path::new(r"C:\Program Files\Ledge\ledge.exe"));
        assert_eq!(c, r#""C:\Program Files\Ledge\ledge.exe" --autostart"#);
    }

    #[test]
    fn policy_table() {
        let dev = Path::new(DEV);
        let inst = Path::new(INST);
        let foreign = r#""C:\Tools\other.exe" --tray"#;
        type Case<'a> = (&'a str, &'a Path, Option<&'a Path>, Option<String>, bool, Action);
        // (běží, nainstalovaná, stávající Run, enabled) → akce
        #[rustfmt::skip]
        let cases: Vec<Case> = vec![
            // nainstalovaná existuje → Run vždy na ni, i z dev release; dev nikdy nezapíše sebe
            ("dev, inst, Run prázdný",            dev,  Some(inst), None,            true,  Action::Write(cmd(INST))),
            ("dev, inst, Run na dev",             dev,  Some(inst), Some(cmd(DEV)),  true,  Action::Write(cmd(INST))),
            ("dev, inst, Run už na inst",         dev,  Some(inst), Some(cmd(INST)), true,  Action::Keep),
            ("inst běží, Run na starou cestu",    inst, Some(inst), Some(cmd(OLD)),  true,  Action::Write(cmd(INST))),
            ("inst běží, Run sedí",               inst, Some(inst), Some(cmd(INST)), true,  Action::Keep),
            // bez instalace → dnešní chování (běžící exe)
            ("dev bez inst, Run prázdný",         dev,  None,       None,            true,  Action::Write(cmd(DEV))),
            ("dev bez inst, Run sedí",            dev,  None,       Some(cmd(DEV)),  true,  Action::Keep),
            ("dev bez inst, Run na starou cestu", dev,  None,       Some(cmd(OLD)),  true,  Action::Write(cmd(DEV))),
            // enabled=false maže jen hodnotu, která patří Ledge
            ("vyp, Run na inst (z dev)",          dev,  Some(inst), Some(cmd(INST)), false, Action::Delete),
            ("vyp, Run na dev",                   dev,  None,       Some(cmd(DEV)),  false, Action::Delete),
            ("vyp, starý ledge.exe --autostart",  inst, Some(inst), Some(cmd(OLD)),  false, Action::Delete),
            ("vyp, Run na inst jinými písmeny",   inst, Some(inst), Some(cmd(&INST.to_uppercase())), false, Action::Delete),
            ("vyp, Run cizí",                     dev,  Some(inst), Some(foreign.into()), false, Action::Keep),
            ("vyp, ledge.exe bez --autostart",    dev,  None,       Some(r#""D:\jiny\ledge.exe""#.into()), false, Action::Keep),
            ("vyp, Run prázdný",                  dev,  Some(inst), None,            false, Action::Keep),
        ];
        for (name, running, installed, current, enabled, want) in cases {
            assert_eq!(
                decide(running, installed, current.as_deref(), enabled),
                want,
                "{name}"
            );
        }
    }

    #[test]
    fn exe_of_parses_quoted_and_bare() {
        assert_eq!(
            exe_of(r#""C:\Program Files\Ledge\ledge.exe" --autostart"#),
            Some(r"C:\Program Files\Ledge\ledge.exe")
        );
        assert_eq!(
            exe_of(r"C:\n\ledge.exe --autostart"),
            Some(r"C:\n\ledge.exe")
        );
        assert_eq!(exe_of(""), None);
        assert_eq!(exe_of(r#""""#), None);
    }

    #[test]
    fn install_candidates_prefer_uninstall_key() {
        let la = Path::new(r"C:\Users\x\AppData\Local");
        // InstallLocation zapisuje šablona v uvozovkách
        assert_eq!(
            install_candidates(Some(r#""D:\Apps\Ledge""#), Some("ledge.exe"), Some(la)),
            vec![
                PathBuf::from(r"D:\Apps\Ledge\ledge.exe"),
                PathBuf::from(INST)
            ]
        );
        // výchozí umístění se neduplikuje
        assert_eq!(
            install_candidates(Some(r#""C:\Users\x\AppData\Local\Ledge""#), None, Some(la)),
            vec![PathBuf::from(INST)]
        );
        // bez klíče Uninstall → jen %LOCALAPPDATA%\Ledge
        assert_eq!(
            install_candidates(None, None, Some(la)),
            vec![PathBuf::from(INST)]
        );
        assert_eq!(
            install_candidates(Some("  "), Some(""), None),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn legacy_run_table() {
        // stará hodnota Run `Notch` → smazat?
        #[rustfmt::skip]
        let cases: Vec<(&str, Option<&str>, bool)> = vec![
            ("žádná hodnota",                 None,                                                                    false),
            ("nainstalovaný notch.exe",       Some(r#""C:\Users\x\AppData\Local\Notch\notch.exe" --autostart"#),      true),
            ("dev notch.exe bez uvozovek",    Some(r"C:\n\target\release\notch.exe --autostart"),                     true),
            ("NOTCH.EXE velkými, bez args",   Some(r#""D:\Apps\NOTCH.EXE""#),                                         true),
            ("cizí program pod jménem Notch", Some(r#""C:\Tools\notchy.exe" --tray"#),                                false),
            ("hodnota už míří na ledge.exe",  Some(r#""C:\Users\x\AppData\Local\Ledge\ledge.exe" --autostart"#),      false),
            ("prázdný řetězec",               Some(""),                                                                false),
        ];
        for (name, current, want) in cases {
            assert_eq!(drop_legacy_run(current), want, "{name}");
        }
    }

    #[test]
    fn gate_table() {
        #[rustfmt::skip]
        let cases: Vec<(&str, bool, Option<&str>, Gate)> = vec![
            ("release mimo balíček",   false, None,                             Gate::Registry),
            ("release v bublině",      false, Some("Claude_1_x64__pzs8sxrjxfjjc"), Gate::InPackage("Claude_1_x64__pzs8sxrjxfjjc".into())),
            ("debug mimo",             true,  None,                             Gate::DebugBuild),
            ("debug v bublině",        true,  Some("Claude_1_x64__p"),          Gate::DebugBuild),
        ];
        for (name, debug, pkg, want) in cases {
            assert_eq!(gate(debug, pkg), want, "{name}");
        }
    }

    #[test]
    fn debug_build_never_touches_registry() {
        // testy běží z target\debug → apply musí být no-op (žádná hodnota Run se nezmění)
        let before: Option<String> = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN)
            .ok()
            .and_then(|k| k.get_value(NAME).ok());
        apply(true).unwrap();
        let after: Option<String> = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN)
            .ok()
            .and_then(|k| k.get_value(NAME).ok());
        assert_eq!(before, after);
    }
}
