//! Tauri příkazy přihrádky (`src/shelf/api.ts`): seznam, ikona, schránka, otevřít / ukázat ve složce,
//! odebrat / vyprázdnit, tah ven. Pracují jen s cestami, které v přihrádce opravdu jsou. Logika
//! v `shelf/` (mapa `docs/codemap/shelf.md`).

use crate::i18n::tr;
use crate::shelf::{self, Format};
use crate::state::{UiCmd, core};
use serde::Serialize;
use std::path::Path;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShelfItemView {
    path: String,
    name: String,
    dir: String,
    /// soubor / složka na disku není (smazaný, přejmenovaný, odpojený disk) — nemaže se sám
    missing: bool,
    added_ms: u64,
    /// akce navíc podle přípony: `text` (Obsah) · `image` (Zmenšit) · chybí
    #[serde(skip_serializing_if = "Option::is_none")]
    extra: Option<shelf::paste::Extra>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShelfList {
    enabled: bool,
    items: Vec<ShelfItemView>,
}

/// Seznam pro kartu (UI ho chce při otevření šuplíku a po události `shelf`). `missing` = stat
/// každé položky (nejvýš `MAX_ITEMS`), jen na vyžádání.
#[tauri::command(async)]
pub(crate) fn shelf_list() -> ShelfList {
    let enabled = core().cfg.read().shelf.enabled;
    let items = if enabled { shelf::items() } else { vec![] };
    let items = items
        .into_iter()
        .map(|i| {
            let (name, dir) = shelf::split(&i.path);
            ShelfItemView { missing: !Path::new(&i.path).exists(), extra: shelf::paste::extra_of(&i.path), path: i.path, name, dir, added_ms: i.added_ms }
        })
        .collect();
    ShelfList { enabled, items }
}

fn known(path: &str) -> Result<String, String> {
    shelf::contains(path).ok_or_else(|| tr("Položka už v přihrádce není.", "The item is no longer on the shelf.").into())
}

fn existing(path: &str) -> Result<String, String> {
    let p = known(path)?;
    if Path::new(&p).exists() { Ok(p) } else { Err(trf!("Soubor chybí: {p}", "The file is missing: {p}")) }
}

/// Ikona ze shellu jako data URL (UI si ji vyžádá jednou na cestu).
#[tauri::command(async)]
pub(crate) fn shelf_icon(path: String) -> Option<String> {
    let p = known(&path).ok()?;
    let png = crate::iconimg::shell_png(Path::new(&p))?;
    Some(format!("data:image/png;base64,{}", crate::icons::b64(&png)))
}

/// Do schránky: `paths` prázdné = celá přihrádka. Vrací počet zkopírovaných cest.
#[tauri::command(async)]
pub(crate) fn shelf_copy(paths: Vec<String>, format: Format) -> Result<usize, String> {
    let all: Vec<String> = shelf::items().into_iter().map(|i| i.path).collect();
    let pick: Vec<String> = if paths.is_empty() { all } else { all.into_iter().filter(|a| paths.iter().any(|p| shelf::same(a, p))).collect() };
    if pick.is_empty() {
        return Err(tr("Přihrádka je prázdná.", "The shelf is empty.").into());
    }
    shelf::clip::set_text(&shelf::clip_text(&pick, format))?;
    Ok(pick.len())
}

/// Otevřít výchozí aplikací (výslovný klik); šuplík se pak zavře jako po spuštění akce.
#[tauri::command(async)]
pub(crate) fn shelf_open(path: String) -> Result<(), String> {
    let p = existing(&path)?;
    let c = core();
    c.app.opener().open_path(&p, None::<&str>).map_err(|e| crate::errors::any(&p, &e))?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Ukázat ve složce (Průzkumník s vybranou položkou).
#[tauri::command(async)]
pub(crate) fn shelf_reveal(path: String) -> Result<(), String> {
    let p = existing(&path)?;
    let c = core();
    c.app.opener().reveal_item_in_dir(&p).map_err(|e| crate::errors::any(&p, &e))?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn shelf_remove(path: String) -> Result<bool, String> {
    shelf::remove_path(&path)
}

#[tauri::command(async)]
pub(crate) fn shelf_clear() -> Result<(), String> {
    shelf::clear()
}

/// Vložit ze schránky (soubory jako odkazy; obrázek / text jako nový soubor v `shelf-drops`). Vrací počet.
#[tauri::command(async)]
pub(crate) fn shelf_paste() -> Result<usize, String> {
    if !core().cfg.read().shelf.enabled {
        return Err(tr("Přihrádka je vypnutá (Nastavení → Šuplík a karty).", "The shelf is turned off (Settings → Drawer and cards).").into());
    }
    shelf::paste::paste_clipboard()
}

/// Textový obsah souboru z přihrádky (do 256 kB, UTF-8) do schránky Windows. Jen položky přihrádky.
#[tauri::command(async)]
pub(crate) fn shelf_copy_content(path: String) -> Result<(), String> {
    let p = existing(&path)?;
    let text = shelf::paste::text_of(Path::new(&p), 256 * 1024)?;
    shelf::clip::set_text(&text)
}

/// Zmenšená kopie obrázku z přihrádky (delší strana ≤ `max_px`, výchozí 1024) jako PNG do `shelf-drops`
/// a do přihrádky; originál zůstane. Vrací cestu kopie.
#[tauri::command(async)]
pub(crate) fn shelf_resize_image(path: String, max_px: Option<u32>) -> Result<String, String> {
    let p = existing(&path)?;
    let out = shelf::paste::resized_copy(Path::new(&p), max_px.unwrap_or(1024).clamp(16, 4096))?;
    Ok(out.to_string_lossy().into_owned())
}

/// Tah položek ven (UI ho zavolá, když se s drženým tlačítkem pohne za práh).
#[tauri::command(async)]
pub(crate) fn shelf_drag_out(app: tauri::AppHandle, paths: Vec<String>) -> Result<(), String> {
    let ok: Vec<String> = paths.iter().filter_map(|p| existing(p).ok()).collect();
    shelf::dragout::start(&app, ok)
}
