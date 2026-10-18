//! Tauri příkazy historie schránky (`src/cliphist/api.ts`): seznam (jen náhledy), vložení zpět do
//! schránky, připnout / smazat položku, vymazat historii. Logika v `cliphist/` (mapa
//! `docs/codemap/clipboard.md`).

use crate::cliphist::apps::{self, AppChoice};
use crate::cliphist::{self, ClipList};

#[tauri::command]
pub(crate) fn clip_list() -> ClipList {
    cliphist::list()
}

/// Zapíše položku do schránky (vlastní zápis — nový záznam nevznikne).
#[tauri::command(async)]
pub(crate) fn clip_paste(id: u64) -> Result<(), String> {
    cliphist::paste(id)
}

#[tauri::command]
pub(crate) fn clip_pin(id: u64, on: bool) -> Result<(), String> {
    cliphist::pin(id, on)
}

#[tauri::command]
pub(crate) fn clip_remove(id: u64) -> Result<(), String> {
    cliphist::remove(id)
}

/// Smaže nepřipnuté položky.
#[tauri::command]
pub(crate) fn clip_clear() {
    cliphist::clear()
}

/// Zapnutí přímo z palety (prázdný stav „Historie schránky je vypnutá — Zapnout“): uloží
/// `clipboard.enabled: true` do configu stejně jako Nastavení.
#[tauri::command]
pub(crate) fn clip_enable() -> Result<(), String> {
    let mut cfg = crate::state::core().cfg.read().clone();
    if cfg.clipboard.enabled {
        return Ok(());
    }
    cfg.clipboard.enabled = true;
    crate::config::save(&crate::config::config_path(), &cfg)?;
    crate::settings::apply_config(cfg);
    Ok(())
}

#[derive(serde::Serialize)]
pub(crate) struct ClipApps {
    /// `exes` z configu s lidskými názvy (pořadí zachované)
    listed: Vec<AppChoice>,
    /// běžící programy (jen s `running: true` — klik na „Přidat běžící program“)
    running: Vec<AppChoice>,
}

/// Vyloučené aplikace pro Nastavení: názvy k uloženým exe + na vyžádání běžící programy.
#[tauri::command(async)]
pub(crate) fn clip_apps(exes: Vec<String>, running: bool) -> ClipApps {
    ClipApps { listed: exes.iter().map(|x| apps::choice(x)).collect(), running: if running { apps::running() } else { vec![] } }
}
