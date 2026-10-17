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
#[tauri::command]
pub(crate) fn check_config(cfg: serde_json::Value) -> Option<FieldError> {
    match serde_json::from_value::<Config>(cfg) {
        Ok(c) => config::validate_field(&c).err(),
        Err(e) => Some(FieldError { field: None, message: trf!("Nastavení má neplatný tvar: {e}", "The settings have an invalid shape: {e}") }),
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
    picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string())
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
    core().app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}
