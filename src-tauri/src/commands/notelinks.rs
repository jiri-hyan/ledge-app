//! Tauri příkazy Lístků 2.0 nad poznámkami: barva a štítky (`note_set_meta` → `notes::meta`, zápis s
//! otiskem), štítky s počty pro filtr karty (`note_tags`), vazby `[[…]]` (`note_link_resolve`,
//! `note_link_suggest` — z paměti, bez I/O; `note_link_create` = nová obecná poznámka s tím názvem).

use crate::notes;
use crate::state::core;
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use std::path::Path;

fn off() -> &'static str {
    crate::i18n::tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings.")
}

/// Barva / štítky poznámky do frontmatteru (`color:`, `tags:`). Jiný obsah na disku → `changed`.
#[tauri::command(async)]
pub(crate) fn note_set_meta(path: String, expected_hash: String, patch: notes::meta::MetaPatch) -> Result<notes::edit::SaveOutcome, String> {
    let root = notes_root().ok_or_else(off)?;
    let out = notes::meta::set_meta(&root, Path::new(&path), &expected_hash, &patch, &notes::now_local())?;
    if matches!(out, notes::edit::SaveOutcome::Saved { .. }) {
        reload_notes();
        send_sync(SyncMsg::Local);
    }
    Ok(out)
}

#[derive(serde::Serialize)]
pub(crate) struct TagCount {
    tag: String,
    count: usize,
}

/// Štítky nehotových poznámek s počty (nejčastější první) — filtr v kartě Poznámky, našeptávač štítků.
#[tauri::command]
pub(crate) fn note_tags() -> Vec<TagCount> {
    notes::edit::tag_counts(&core().notes.lock().all).into_iter().map(|(tag, count)| TagCount { tag, count }).collect()
}

/// Cesta poznámky pro `[[name]]` (`None` = taková poznámka není → UI nabídne vytvořit).
#[tauri::command]
pub(crate) fn note_link_resolve(name: String) -> Option<String> {
    notes::links::resolve(&core().notes.lock().all, &name).map(|n| n.path.clone())
}

/// Našeptávač při psaní `[[`: nejvýš `limit` (výchozí 8) titulků.
#[tauri::command]
pub(crate) fn note_link_suggest(query: String, limit: Option<usize>) -> Vec<notes::links::LinkHit> {
    notes::links::suggest(&core().notes.lock().all, &query, limit.unwrap_or(8).min(20))
}

/// Nová obecná poznámka s titulkem `name` (klik na `[[name]]`, která zatím neexistuje). Vrací cestu.
#[tauri::command(async)]
pub(crate) fn note_link_create(name: String) -> Result<String, String> {
    let root = notes_root().ok_or_else(off)?;
    let name = name.trim();
    if name.is_empty() || name.contains(['[', ']', '\n', '\r']) {
        return Err(crate::i18n::tr("Neplatný název poznámky.", "Invalid note name.").into());
    }
    if let Some(n) = notes::links::resolve(&core().notes.lock().all, name) {
        return Ok(n.path.clone());
    }
    let path = notes::create(&root, &notes::NewNote { text: name.to_string(), ..Default::default() }, &notes::now_local())?;
    reload_notes();
    send_sync(SyncMsg::Local);
    Ok(path.to_string_lossy().into_owned())
}
