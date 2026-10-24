//! Tauri příkazy „chytřejšího šuplíku“ (kolo 20): export útrat a limitů do souboru. Logika renderu
//! v `dataexport.rs`; tady jen dialog „Uložit jako“ (rodič = okno šuplíku, ať ho blur nezavře) a zápis.

use crate::dataexport::{self, Format};
use crate::i18n::tr;
use crate::state::{core, now_ms};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

/// Exportuje útratu a limity Claude jako `csv` nebo `md`. Vrací cestu uloženého souboru, `None` = člověk
/// dialog zavřel. Jen data z paměti Ledge; žádná síť, žádný nový sken transcriptů.
#[tauri::command(async)]
pub(crate) fn export_usage(format: String) -> Result<Option<String>, String> {
    let f = Format::parse(&format).ok_or_else(|| tr("Neznámý formát exportu.", "Unknown export format.").to_string())?;
    let costs = crate::costslive::view();
    let limits = crate::limitslive::view();
    if !dataexport::has_data(&costs, &limits) {
        return Err(tr("Zatím není co exportovat — útrata se ještě nespočítala a limity nepřišly.", "Nothing to export yet — usage hasn't been computed and no limits have arrived.").to_string());
    }
    let text = dataexport::render(f, &costs, &limits, now_ms());
    let c = core();
    let name = format!("ledge-usage-{}.{}", chrono::Local::now().format("%Y-%m-%d"), f.ext());
    let label = match f {
        Format::Csv => "CSV",
        Format::Markdown => "Markdown",
    };
    let mut d = c.app.dialog().file().add_filter(label, &[f.ext()]).set_file_name(name);
    if let Some(w) = c.app.get_webview_window(crate::drawer::LABEL) {
        d = d.set_parent(&w);
    }
    let Some(path) = d.blocking_save_file().and_then(|p| p.into_path().ok()) else { return Ok(None) };
    std::fs::write(&path, text.as_bytes()).map_err(|e| crate::errors::any(&path.to_string_lossy(), &e))?;
    Ok(Some(path.to_string_lossy().to_string()))
}
