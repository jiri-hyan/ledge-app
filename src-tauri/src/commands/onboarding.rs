//! Tauri příkazy průvodce prvním spuštěním (`onboarding/`): sken rep a aplikací, ikony nalezených
//! položek (jen těch, které sken vrátil), detekce Claude Code, výběr složky, motivy a zápis výběru.
//! Skeny a dialog běží v `spawn_blocking` (neblokují vlákna runtime) s termínem; nic z toho se
//! nevolá bez otevřeného průvodce.

use crate::config::{self, Config};
use crate::onboarding::{apply, apps, claude, dialog, repos, themes};
use crate::settings::apply_config;
use crate::state::core;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Klíče (`repo:<cesta>`, `app:<exe>`), pro které smí `onboarding_icons` číst disk — jen co vrátil sken.
static KNOWN: Mutex<Option<HashSet<String>>> = Mutex::new(None);
const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_ICONS: usize = 48;

fn remember(keys: impl Iterator<Item = String>) {
    KNOWN.lock().get_or_insert_with(HashSet::new).extend(keys);
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

/// Blokující práce mimo vlákna runtime; panika = prázdný výsledek.
async fn blocking<T: Send + Default + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(f).await.unwrap_or_default()
}

#[tauri::command]
pub(crate) async fn onboarding_repos(extra: Vec<String>) -> Vec<repos::FoundRepo> {
    blocking(move || scan_repos(&extra)).await
}

fn scan_repos(extra: &[String]) -> Vec<repos::FoundRepo> {
    let c = core();
    let cfg: Config = c.cfg.read().clone();
    let roots = repos::dedupe_dirs(repos::default_roots(env).into_iter().chain(cfg.roots.iter().map(PathBuf::from)).chain(extra.iter().map(PathBuf::from)));
    let known_paths: Vec<String> = cfg.projects.iter().filter_map(|p| c.project_dir(p)).map(|d| repos::norm(&d.to_string_lossy())).collect();
    let known_remotes: Vec<_> = cfg.projects.iter().filter_map(|p| p.remote.as_deref().and_then(crate::resolve::parse_remote)).collect();
    let found = repos::discover(&roots, Instant::now() + SCAN_TIMEOUT, &known_paths, &known_remotes);
    remember(found.iter().map(|r| format!("repo:{}", r.path)));
    found
}

#[tauri::command]
pub(crate) async fn onboarding_apps() -> Vec<apps::FoundApp> {
    blocking(scan_apps).await
}

fn scan_apps() -> Vec<apps::FoundApp> {
    let cfg = core().cfg.read().clone();
    let exes: Vec<String> = cfg.projects.iter().filter_map(|p| p.exe.as_deref()).map(str::to_lowercase).collect();
    let names: Vec<String> = cfg.projects.iter().filter_map(|p| p.app_name.as_deref()).map(str::to_lowercase).collect();
    let found = apps::discover(&apps::start_menu_dirs(env), Instant::now() + SCAN_TIMEOUT, &exes, &names);
    remember(found.iter().map(|a| format!("app:{}", a.exe)));
    found
}

/// Ikony nalezených rep / aplikací jako data URL (PNG ≤ 128 px, SVG). Neznámý klíč = `None`.
#[tauri::command]
pub(crate) async fn onboarding_icons(keys: Vec<String>) -> Vec<Option<String>> {
    blocking(move || icons_for(&keys)).await
}

fn icons_for(keys: &[String]) -> Vec<Option<String>> {
    let known = KNOWN.lock().clone().unwrap_or_default();
    keys.iter()
        .take(MAX_ICONS)
        .map(|k| {
            if !known.contains(k) {
                return None;
            }
            if let Some(path) = k.strip_prefix("repo:") {
                let src = crate::icons::Source { dir: Some(PathBuf::from(path)), logo: None, exe: None, app_name: None, policy: crate::icons::ExeIcon::No };
                return crate::icons::resolve(&src).map(|l| l.data_url());
            }
            let exe = k.strip_prefix("app:")?;
            crate::iconimg::exe_png(Path::new(exe)).map(|b| crate::icons::Logo { mime: "image/png", bytes: b }.data_url())
        })
        .collect()
}

#[tauri::command]
pub(crate) async fn onboarding_claude() -> Option<claude::ClaudeInfo> {
    blocking(|| Some(claude::detect())).await
}

/// Dialog výběru složky (vlastník = šuplík). `None` = zrušeno.
#[tauri::command]
pub(crate) async fn onboarding_pick_folder() -> Option<String> {
    use tauri::Manager;
    let owner = core().app.get_webview_window(crate::drawer::LABEL).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
    blocking(move || dialog::pick_folder(owner).map(|p| p.to_string_lossy().into_owned())).await
}

#[tauri::command]
pub(crate) fn onboarding_themes() -> Vec<themes::ThemeOption> {
    serde_json::from_str(&config::schema_json()).map(|s| themes::theme_options(&s)).unwrap_or_default()
}

/// Zapíše výběr jednoho kroku (nebo `done`) do configu — hned, ať se nic neztratí při zavření šuplíku.
#[tauri::command]
pub(crate) fn onboarding_apply(patch: apply::Patch) -> Result<apply::Applied, String> {
    let cur = core().cfg.read().clone();
    let (next, out) = apply::apply(&cur, &patch)?;
    if next != cur {
        config::save(&config::config_path(), &next)?;
        apply_config(next);
    }
    Ok(out)
}
