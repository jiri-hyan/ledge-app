//! Tauri příkazy Nastavení (kromě čtení/uložení configu v `config.rs`): ověření s cestou k poli,
//! výchozí hodnoty, záloha (export / náhled importu), nativní dialogy (`tauri-plugin-dialog`, jen z
//! Rustu — webview žádné dialogové oprávnění nemá), návrh projektu ze složky, náhled loga, info o appce.

use crate::i18n::tr;
use crate::config::{self, Config, Project};
use crate::configerr::FieldError;
use crate::configio::{self, ImportPreview};
use crate::projectdetect::{self, Candidate, Detected};
use crate::state::core;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// Rozpracovaný config z UI → první chyba i s polem (`None` = v pořádku). Nic nezapisuje.
#[tauri::command(async)]
pub(crate) fn check_config(cfg: serde_json::Value) -> Option<FieldError> {
    match serde_json::from_value::<Config>(cfg) {
        Ok(c) => config::validate_field(&c).err(),
        Err(e) => {
            crate::errors::detail("check_config", &e);
            Some(FieldError { field: None, message: tr("Nastavení má neplatný tvar — podrobnosti jsou v logu.", "The settings have an invalid shape — details are in the log.").into() })
        }
    }
}

/// Výchozí config (bez projektů) — „Obnovit výchozí“ po sekcích.
#[tauri::command]
pub(crate) fn config_defaults() -> Config {
    Config::default()
}

#[tauri::command(async)]
pub(crate) fn config_export(path: String) -> Result<(), String> {
    let cfg = core().cfg.read().clone();
    configio::export(&cfg, Path::new(&path))
}

#[tauri::command(async)]
pub(crate) fn config_import_preview(path: String) -> Result<ImportPreview, String> {
    let cur = core().cfg.read().clone();
    configio::preview(&cur, Path::new(&path))
}

/// Nativní dialog nad šuplíkem (vlastník = okno šuplíku → blur ho nezavře, `focus_in_own_popup`).
/// `kind`: `folder` | `file` | `exe` | `image` | `json`. Zrušení = `None`.
#[tauri::command(async)]
pub(crate) fn pick_path(kind: String, title: Option<String>, start: Option<String>) -> Option<String> {
    let c = core();
    let mut d = c.app.dialog().file();
    if let Some(w) = c.app.get_webview_window(crate::drawer::LABEL) {
        d = d.set_parent(&w);
    }
    if let Some(t) = title {
        d = d.set_title(t);
    }
    if let Some(dir) = start.map(PathBuf::from).filter(|p| p.is_dir()) {
        d = d.set_directory(dir);
    }
    let picked = match kind.as_str() {
        "folder" => d.blocking_pick_folder(),
        "exe" => d.add_filter(tr("Programy", "Programs"), &["exe"]).blocking_pick_file(),
        "image" => d.add_filter(tr("Obrázky", "Images"), &["svg", "png", "ico", "jpg", "jpeg", "webp"]).blocking_pick_file(),
        "json" => d.add_filter("JSON", &["json"]).blocking_pick_file(),
        _ => d.blocking_pick_file(),
    };
    let out = picked.and_then(|p| p.into_path().ok());
    // jednorázová stopa pro `launcher_catalog_read`: číst smí jen soubor, který člověk právě vybral v dialogu
    *PICKED_JSON.lock() = out.clone().filter(|_| kind == "json");
    out.map(|p| p.to_string_lossy().to_string())
}

/// Poslední `.json` vybraný v dialogu `pick_path("json")` — `launcher_catalog_read` ho spotřebuje (jednou).
static PICKED_JSON: parking_lot::Mutex<Option<PathBuf>> = parking_lot::Mutex::new(None);

/// Smí se číst `path`? Jen když je to přesně soubor z posledního dialogu; stopa se tím spotřebuje.
fn take_picked(slot: &parking_lot::Mutex<Option<PathBuf>>, path: &Path) -> bool {
    slot.lock().take().is_some_and(|p| p == path)
}

/// Dialog „Uložit jako“ pro export zálohy.
#[tauri::command(async)]
pub(crate) fn pick_save(name: String) -> Option<String> {
    let c = core();
    let mut d = c.app.dialog().file().add_filter("JSON", &["json"]).set_file_name(name);
    if let Some(w) = c.app.get_webview_window(crate::drawer::LABEL) {
        d = d.set_parent(&w);
    }
    d.blocking_save_file().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string())
}

#[tauri::command(async)]
pub(crate) fn project_detect(path: String) -> Result<Detected, String> {
    let taken: Vec<String> = core().cfg.read().projects.iter().map(|p| p.id.clone()).collect();
    projectdetect::detect(Path::new(&path), &taken)
}

/// Naklonovaná repa v `roots`, která ještě nejsou v projektech.
#[tauri::command(async)]
pub(crate) fn project_candidates() -> Vec<Candidate> {
    let c = core();
    let cfg = c.cfg.read().clone();
    let dirs: Vec<PathBuf> = cfg.projects.iter().filter_map(|p| c.project_dir(&Project { subdir: None, ..p.clone() })).collect();
    let remotes: Vec<String> = cfg.projects.iter().filter_map(|p| p.remote.clone()).collect();
    projectdetect::candidates(&cfg.roots, &dirs, &remotes)
}

/// Logo rozpracovaného projektu jako data URL (stejné pořadí jako dlaždice, `icons::resolve`).
#[tauri::command(async)]
pub(crate) fn logo_preview(project: serde_json::Value) -> Option<String> {
    let p: Project = serde_json::from_value(project).ok()?;
    let dir = core().project_dir(&p);
    let exe = p.exe.as_deref().map(|e| {
        let e = Path::new(e);
        if e.is_absolute() { e.to_path_buf() } else { dir.clone().unwrap_or_default().join(e) }
    });
    let src = crate::icons::Source { dir, logo: p.logo.clone(), exe, app_name: p.app_name.clone(), policy: crate::icons::exe_policy(&p) };
    crate::icons::resolve(&src).map(|l| l.data_url())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppInfo {
    version: String,
    config_path: String,
    config_dir: String,
    log_path: String,
}

#[tauri::command]
pub(crate) fn app_info() -> AppInfo {
    let dir = config::config_dir();
    AppInfo {
        version: core().app.package_info().version.to_string(),
        config_path: config::config_path().to_string_lossy().to_string(),
        config_dir: dir.to_string_lossy().to_string(),
        log_path: dir.join("ledge.log").to_string_lossy().to_string(),
    }
}

/// Otevře diagnostický log (`log`) nebo složku configu (`dir`) výchozí aplikací.
#[tauri::command]
pub(crate) fn open_app_file(which: String) -> Result<(), String> {
    let dir = config::config_dir();
    let path = match which.as_str() {
        "log" => dir.join("ledge.log"),
        "dir" => dir,
        _ => return Err(tr("Neznámý soubor.", "Unknown file.").into()),
    };
    if !path.exists() {
        return Err(trf!("{} zatím neexistuje.", "{} doesn't exist yet.", path.display()));
    }
    core().app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| crate::errors::any(tr("Soubor nejde otevřít", "Can't open the file"), &e))
}

/// Strop katalogu Launcheru (bajty) — větší soubor to není.
const LAUNCHER_CATALOG_MAX: u64 = 2 * 1024 * 1024;

/// Kontrola souboru katalogu před čtením: `.json`, obyčejný soubor (ne složka, ne symbolický odkaz), ≤ 2 MB.
fn catalog_file_ok(path: &Path, meta: &std::fs::Metadata) -> Result<(), String> {
    let json = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if !json || !meta.is_file() || meta.file_type().is_symlink() {
        return Err(tr("Vyber soubor catalog.json z Launcheru.", "Pick the catalog.json file from Launcher.").into());
    }
    if meta.len() > LAUNCHER_CATALOG_MAX {
        return Err(tr("Soubor je moc velký na katalog (nejvýš 2 MB).", "The file is too large for a catalog (max 2 MB).").into());
    }
    Ok(())
}

/// Katalog Launcheru (`catalog.json`, vybraný v dialogu) pro import zástupců (`src/settings/launcherImport.ts`).
/// Jen čtení a parsování JSON; nic se nespouští ani neukládá — výsledek jde do návrhu Nastavení.
#[tauri::command(async)]
pub(crate) fn launcher_catalog_read(path: String) -> Result<serde_json::Value, String> {
    if !take_picked(&PICKED_JSON, Path::new(&path)) {
        return Err(tr("Katalog vyber znovu v dialogu (Nastavení → Projekty → Z Launcheru…).", "Pick the catalog again in the dialog (Settings → Projects → From Launcher…).").into());
    }
    read_catalog(Path::new(&path))
}

/// Zástupce z katalogu ke kontrole (jen pole, ze kterých se spouští).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportCmd {
    id: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    run: Option<String>,
    #[serde(default)]
    exe: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportVerdict {
    id: String,
    /// `None` = příkaz projde pojistkou; jinak lidská věta, proč se zástupce importuje vypnutý
    problem: Option<String>,
}

/// Import Launcheru: projdou příkazy zástupců stejnou pojistkou jako MCP (`runguard`)? Co neprojde, UI
/// naimportuje vypnuté a ukáže to v potvrzení.
#[tauri::command(async)]
pub(crate) fn launcher_import_check(items: Vec<ImportCmd>) -> Vec<ImportVerdict> {
    items.into_iter().map(|i| verdict(i, crate::runguard::detected_runs)).collect()
}

fn verdict(i: ImportCmd, detect: impl Fn(&Path) -> Vec<(String, Vec<String>)>) -> ImportVerdict {
    use crate::runguard::{Issue, check};
    let found = || i.path.as_deref().map(|p| detect(Path::new(p))).unwrap_or_default();
    let problem = match check(i.exe.as_deref(), true, i.run.as_deref(), true, &i.args, true, found) {
        Ok(()) => None,
        Err(Issue::Exe(exe)) => Some(trf!("„{exe}“ není cesta k souboru .exe", "“{exe}” isn't a path to an .exe file")),
        Err(Issue::Run { run, .. }) => Some(trf!(
            "příkaz „{run}“ není skript balíčkovače (npm run …) ani příkaz, který Ledge najde ve složce projektu",
            "the command “{run}” isn't a package-manager script (npm run …) or a command Ledge finds in the project folder"
        )),
        Err(Issue::Args) => Some(tr("argumenty, které Ledge pro tuhle složku nenavrhuje", "arguments Ledge doesn't suggest for this folder").into()),
    };
    ImportVerdict { id: i.id, problem }
}

fn read_catalog(p: &Path) -> Result<serde_json::Value, String> {
    let meta = std::fs::symlink_metadata(p).map_err(|e| crate::errors::io(tr("Katalog nejde přečíst", "Can't read the catalog"), &e))?;
    catalog_file_ok(p, &meta)?;
    let data = std::fs::read_to_string(p).map_err(|e| crate::errors::io(tr("Katalog nejde přečíst", "Can't read the catalog"), &e))?;
    serde_json::from_str(data.trim_start_matches('\u{feff}')).map_err(|_| tr("Soubor není platný JSON katalogu Launcheru.", "The file isn't a valid Launcher catalog JSON.").into())
}

#[cfg(test)]
mod launcher_tests {
    use super::*;

    #[test]
    fn catalog_file_checks() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("catalog.json");
        std::fs::write(&ok, "\u{feff}{\"items\":[]}").unwrap();
        let meta = |p: &Path| std::fs::symlink_metadata(p).unwrap();
        assert!(catalog_file_ok(&ok, &meta(&ok)).is_ok());
        assert_eq!(read_catalog(&ok).unwrap(), serde_json::json!({"items": []}), "BOM nevadí");
        let txt = dir.path().join("catalog.txt");
        std::fs::write(&txt, "{}").unwrap();
        assert!(catalog_file_ok(&txt, &meta(&txt)).is_err(), "jen .json");
        let sub = dir.path().join("slozka.json");
        std::fs::create_dir(&sub).unwrap();
        assert!(catalog_file_ok(&sub, &meta(&sub)).is_err(), "složka");
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "{nejson").unwrap();
        assert!(read_catalog(&bad).is_err());
        assert!(read_catalog(&dir.path().join("neni.json")).is_err());
    }

    #[test]
    fn import_commands_go_through_runguard() {
        let cmd = |run: Option<&str>, exe: Option<&str>, args: &[&str]| ImportCmd { id: "x".into(), path: Some("C:/Work/x".into()), run: run.map(Into::into), exe: exe.map(Into::into), args: args.iter().map(|s| s.to_string()).collect() };
        let detect = |_: &Path| vec![("cargo run".to_string(), vec![])];
        let ok = |c| verdict(c, detect).problem.is_none();
        assert!(ok(cmd(Some("npm run dev"), None, &[])));
        assert!(ok(cmd(Some("cargo run"), None, &[])), "nalezený ve složce");
        assert!(ok(cmd(Some("npm run build"), Some("dist/app.exe"), &[])));
        assert!(!ok(cmd(Some("powershell -enc SQBFAFgA"), None, &[])));
        assert!(!ok(cmd(None, Some("tools/run.bat"), &[])));
        assert!(!ok(cmd(None, Some("dist/app.exe"), &["--evil"])));
    }

    #[test]
    fn catalog_only_from_dialog_once() {
        let slot = parking_lot::Mutex::new(None);
        let picked = PathBuf::from("C:/Launcher/catalog.json");
        assert!(!take_picked(&slot, &picked), "bez dialogu nic");
        *slot.lock() = Some(picked.clone());
        assert!(!take_picked(&slot, Path::new("C:/Users/x/.claude/.credentials.json")), "jiná cesta ne");
        assert!(!take_picked(&slot, &picked), "stopa se spotřebuje i neúspěchem");
        *slot.lock() = Some(picked.clone());
        assert!(take_picked(&slot, &picked));
        assert!(!take_picked(&slot, &picked), "jen jednou");
    }
}
