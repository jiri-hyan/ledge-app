//! Tauri příkazy Nastavení pro rozšíření (`ext/`). Tenké obálky: měření běží jen při volání.

use crate::ext::{self, ExtView, wall::bg};
use crate::i18n::tr;
use tauri_plugin_dialog::DialogExt;

/// Stav rozšíření + změřená RAM a CPU běžících (UI volá po 5 s, jen s viditelným blokem Rozšíření).
#[tauri::command(async)]
pub(crate) fn ext_status() -> Vec<ExtView> {
    ext::status_measured()
}

/// Pozadí tapety z vlastní fotky. `op`: `get` (název vybrané fotky, `None` = přechod) · `pick` (dialog Otevřít →
/// kontrola, zmenšení, kopie do složky rozšíření; zrušení dialogu nic nemění) · `clear` (zpět přechod). Cestu
/// UI neposílá — soubor vybírá jen dialog tady v jádru. Vrací aktuální název fotky.
#[tauri::command(async)]
pub(crate) fn ext_wall_bg(window: tauri::WebviewWindow, op: String) -> Result<Option<String>, String> {
    let dir = crate::actionrun::local_dir().join("ext").join("wall");
    match op.as_str() {
        "get" => Ok(bg::name(&dir)),
        "clear" => {
            bg::clear(&dir);
            ext::wall_bg_changed();
            Ok(None)
        }
        "pick" => {
            let picked = crate::state::core()
                .app
                .dialog()
                .file()
                .set_parent(&window)
                .set_title(tr("Pozadí tapety", "Wallpaper background"))
                .add_filter(tr("Obrázky", "Images"), &["jpg", "jpeg", "png", "webp"])
                .blocking_pick_file()
                .and_then(|p| p.into_path().ok());
            let Some(src) = picked else { return Ok(bg::name(&dir)) };
            let name = bg::set_from(&dir, &src)?;
            ext::wall_bg_changed();
            Ok(Some(name))
        }
        _ => Err(format!("neznámá operace {op}")),
    }
}
