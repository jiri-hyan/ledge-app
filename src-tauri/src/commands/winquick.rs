//! Příkazy karty Rychlé dlaždice Windows (kolo 23, `winquick.rs` + `winquickwin.rs`). Všechno běží jen na
//! výslovný klik nebo při otevření viditelné karty (`wq_state`); bez karty na záložce příkazy odmítnou.

use crate::config::{self, Config};
use crate::i18n::tr;
use crate::state::{UiCmd, core, find_project};
use crate::winquick::{self, TermKind};
use crate::winquickwin as win;
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FolderView {
    pub key: String,
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WqState {
    pub tiles: Vec<String>,
    pub output_switch: bool,
    pub pinned: Vec<String>,
    pub folders: Vec<FolderView>,
    /// `true` = světlý motiv Windows
    pub light: bool,
    pub radios: Vec<win::RadioInfo>,
    pub shot: Option<win::ShotView>,
}

fn gate() -> Result<winquick::WinQuickConfig, String> {
    let cfg = core().cfg.read();
    if winquick::card_allowed(&cfg.layout) {
        Ok(cfg.winquick.clone())
    } else {
        Err(tr("Karta Rychlé dlaždice Windows není na žádné záložce.", "The Windows quick tiles card is not on any tab.").into())
    }
}

/// Stav karty; čte jen to, co zapnuté dlaždice potřebují (rádia, snímek). Volá UI při zobrazení karty.
#[tauri::command(async)]
pub(crate) fn wq_state() -> Result<WqState, String> {
    let wq = gate()?;
    let has = |id: &str| wq.tiles.iter().any(|t| t == id);
    let mut folders: Vec<FolderView> = ["downloads", "desktop", "documents"]
        .iter()
        .filter_map(|k| win::known_dir(k).map(|p| FolderView { key: (*k).into(), path: p.to_string_lossy().into() }))
        .collect();
    folders.extend(wq.folders.iter().enumerate().map(|(i, p)| FolderView { key: format!("pin:{i}"), path: p.clone() }));
    Ok(WqState {
        light: has("theme") && winquick::is_light(&win::RegTheme),
        radios: if has("wifi") || has("bt") { win::radios() } else { Vec::new() },
        shot: if has("shot") { win::latest_shot() } else { None },
        output_switch: wq.output_switch,
        pinned: wq.folders.clone(),
        folders: if has("dir") { folders } else { Vec::new() },
        tiles: wq.tiles,
    })
}

/// Uloží výběr a pořadí dlaždic, připnuté složky a volbu přepínání výstupu (úprava karty).
#[tauri::command(async)]
pub(crate) fn wq_config_set(tiles: Vec<String>, folders: Vec<String>, output_switch: bool) -> Result<(), String> {
    gate()?;
    let mut cfg: Config = core().cfg.read().clone();
    cfg.winquick = winquick::WinQuickConfig { tiles: winquick::normalize_tiles(&tiles), folders, output_switch };
    winquick::validate(&cfg.winquick).map_err(|e| e.message)?;
    config::save(&config::config_path(), &cfg)?;
    crate::settings::apply_config(cfg);
    Ok(())
}

fn spawn_visible(l: winquick::Launch, flags: u32) -> Result<(), String> {
    let mut cmd = std::process::Command::new(&l.exe);
    cmd.args(&l.args);
    if let Some(d) = &l.cwd {
        cmd.current_dir(d);
    }
    cmd.creation_flags(flags).spawn().map(|_| ()).map_err(|e| crate::errors::spawn(tr("Nejde otevřít", "Cannot open"), &e))?;
    core().send_ui(UiCmd::Hide);
    Ok(())
}

/// Terminál (`wt` / `powershell`) nebo Průzkumník ve složce projektu. Terminál je viditelný záměrně (výslovný klik).
#[tauri::command(async)]
pub(crate) fn wq_terminal(id: String, kind: String) -> Result<(), String> {
    gate()?;
    let kind = winquick::parse_term(&kind).ok_or_else(|| tr("Neznámý cíl.", "Unknown target."))?;
    let p = find_project(&id)?;
    let dir = core().project_dir(&p).ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?;
    let flags = match kind {
        TermKind::Wt => 0x0000_0008,         // DETACHED_PROCESS
        TermKind::PowerShell => 0x0000_0010, // CREATE_NEW_CONSOLE
        TermKind::Explorer => 0,
    };
    spawn_visible(winquick::launch_for(kind, &dir), flags)
}

/// Otevře složku: `downloads` / `desktop` / `documents` nebo `pin:<n>` (připnutá z configu). UI neposílá cestu.
#[tauri::command(async)]
pub(crate) fn wq_folder(key: String) -> Result<(), String> {
    let wq = gate()?;
    let dir: Option<PathBuf> = match key.strip_prefix("pin:") {
        Some(i) => i.parse::<usize>().ok().and_then(|i| wq.folders.get(i)).map(PathBuf::from),
        None => win::known_dir(&key),
    };
    let dir = dir.filter(|d| d.is_dir()).ok_or_else(|| tr("Složka neexistuje.", "The folder does not exist."))?;
    spawn_visible(winquick::launch_for(TermKind::Explorer, &dir), 0)
}

/// Přepne motiv Windows (tmavý ↔ světlý). Vrací `true` = nyní světlý.
#[tauri::command(async)]
pub(crate) fn wq_theme_toggle() -> Result<bool, String> {
    let wq = gate()?;
    if !winquick::tile_enabled(&wq, "theme") {
        return Err(tr("Dlaždice motivu není zapnutá.", "The theme tile is not enabled.").into());
    }
    winquick::toggle_theme(&mut win::RegTheme)
}

/// Zapne / vypne Wi-Fi nebo Bluetooth (`wifi` / `bt`).
#[tauri::command(async)]
pub(crate) fn wq_radio_set(kind: String, on: bool) -> Result<(), String> {
    gate()?;
    let k = winquick::parse_radio(&kind).ok_or_else(|| tr("Neznámé rádio.", "Unknown radio."))?;
    win::set_radio(k, on)
}

/// Stránka Nastavení Windows podle id z `winsettings::QUICK` (`network`, `sound`, `display`, `bluetooth`).
#[tauri::command]
pub(crate) fn wq_settings(page: String) -> Result<(), String> {
    gate()?;
    let uri = crate::winsettings::quick_uri(&page).ok_or_else(|| tr("Neznámá stránka nastavení.", "Unknown settings page."))?;
    core().app.opener().open_url(uri, None::<&str>).map_err(|e| crate::errors::any(tr("Nastavení Windows nejde otevřít", "Cannot open Windows Settings"), &e))?;
    core().send_ui(UiCmd::Hide);
    Ok(())
}

/// Přepnout výstup zvuku jedním klikem — jen s volbou `winquick.outputSwitch` (neoficiální `IPolicyConfig`).
/// Bez volby UI otevře Nastavení zvuku (`wq_settings("sound")`). Selhání = zpráva, nikdy panika.
#[tauri::command(async)]
pub(crate) fn wq_output_set(id: String) -> Result<(), String> {
    // karta Zvuk (přehrávač) smí přepínat i bez karty Rychlé dlaždice: rozhoduje jen volba
    if !winquick::output_set_allowed(&core().cfg.read().winquick) {
        return Err(tr("Přepínání výstupu je vypnuté (volba v úpravách karty).", "Output switching is off (option in the card edit mode).").into());
    }
    crate::volume::set_default_output(&id)
}

/// Poslední snímek: `image` (obrázek do schránky), `path` (cesta), `claude` (`@cesta` pro chat), `folder` (otevřít složku).
/// Vrací text pro hlášku. Nic se nikam samo nevkládá.
#[tauri::command(async)]
pub(crate) fn wq_shot(action: String) -> Result<String, String> {
    gate()?;
    let shot = win::latest_shot().ok_or_else(|| tr("Ve složce Snímky obrazovky nic není.", "There is nothing in the Screenshots folder."))?;
    let path = PathBuf::from(&shot.path);
    match action.as_str() {
        "image" => {
            win::copy_image(&path)?;
            Ok(tr("Obrázek je ve schránce", "Image copied").into())
        }
        "path" => {
            crate::shelf::clip::set_text(&shot.path)?;
            Ok(tr("Cesta je ve schránce", "Path copied").into())
        }
        "claude" => {
            crate::shelf::clip::set_text(&winquick::claude_ref(&shot.path))?;
            Ok(tr("Zkopírováno — vlož do chatu s Claudem", "Copied — paste it into the Claude chat").into())
        }
        "folder" => {
            let dir = path.parent().map(PathBuf::from).ok_or_else(|| tr("Složka neexistuje.", "The folder does not exist."))?;
            spawn_visible(winquick::launch_for(TermKind::Explorer, &dir), 0)?;
            Ok(String::new())
        }
        _ => Err(tr("Neznámá akce.", "Unknown action.").into()),
    }
}
