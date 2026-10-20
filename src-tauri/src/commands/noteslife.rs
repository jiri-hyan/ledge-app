//! Tauri příkazy životního cyklu poznámky: archiv a smazání do Koše Windows se Zpět (`notes::lifecycle`).
//! Tenké obálky: ověření cesty a přesuny dělá `lifecycle`, tady je stav čekajících smazání, vlákno lhůty
//! a vedlejší účinky (lístek se zavře, rozepsaná úprava zmizí, poznámky se přenačtou, sync se probudí).

use crate::i18n::tr;
use crate::notes::{self, lifecycle};
use crate::pins::win;
use crate::state::{log, now_ms};
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Smazání čekající na lhůtu (Zpět ještě jde). Kdo id z tohoto seznamu vyjme, rozhoduje: Zpět, nebo Koš.
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Po téhle době jde odklad do Koše. UI nabízí „Zpět“ o dvě sekundy kratší (stihne doběhnout).
const GRACE: Duration = Duration::from_secs(10);

fn off() -> &'static str {
    tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings.")
}

fn take_pending(id: &str) -> bool {
    let mut g = PENDING.lock();
    let before = g.len();
    g.retain(|p| p != id);
    g.len() != before
}

fn new_id() -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    format!("{}-{}", now_ms(), SEQ.fetch_add(1, Ordering::Relaxed))
}

/// Poznámka zmizela ze svého místa (archiv, smazání): lístek se zavře (`keep_pin`: lístek, který akci
/// vyvolal, si okno zavře sám po své liště „Zpět“), rozepsaná úprava zahodí, seznamy se přenačtou
/// a sync dostane vědět.
fn settle(root: &Path, path: &Path, keep_pin: bool) {
    if !keep_pin {
        win::close(&path.to_string_lossy());
    }
    let _ = notes::edit::edit_draft_forget(root, path);
    reload_notes();
    send_sync(SyncMsg::Local);
}

/// Poznámku do archivu (podsložka `_archiv`, i s obrázky). Vrací novou cestu.
#[tauri::command(async)]
pub(crate) fn note_archive(path: String, keep_pin: Option<bool>) -> Result<String, String> {
    let root = notes_root().ok_or_else(off)?;
    let new = lifecycle::archive(&root, Path::new(&path))?;
    settle(&root, Path::new(&path), keep_pin.unwrap_or(false));
    Ok(new.to_string_lossy().into_owned())
}

/// Poznámku z archivu zpátky do její složky. Vrací novou cestu.
#[tauri::command(async)]
pub(crate) fn note_unarchive(path: String) -> Result<String, String> {
    let root = notes_root().ok_or_else(off)?;
    let new = lifecycle::unarchive(&root, Path::new(&path))?;
    reload_notes();
    send_sync(SyncMsg::Local);
    Ok(new.to_string_lossy().into_owned())
}

/// Co UI potřebuje pro lištu „Smazáno · Zpět“.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Deleted {
    id: String,
    title: String,
}

/// Smazání: poznámka (a její vlastní obrázky) jde do odkladu, po lhůtě do Koše Windows. Bez dialogu — je Zpět.
#[tauri::command(async)]
pub(crate) fn note_delete(path: String, keep_pin: Option<bool>) -> Result<Deleted, String> {
    let root = notes_root().ok_or_else(off)?;
    let title = {
        let st = crate::state::core().notes.lock();
        st.all.iter().chain(st.archived.iter()).find(|n| n.path == path).map(|n| n.title.clone()).unwrap_or_default()
    };
    let id = new_id();
    lifecycle::park(&root, Path::new(&path), &id)?;
    PENDING.lock().push(id.clone());
    settle(&root, Path::new(&path), keep_pin.unwrap_or(false));
    spawn_commit(root, id.clone());
    Ok(Deleted { id, title })
}

/// Zpět po smazání (do lhůty). Vrací cestu obnovené poznámky.
#[tauri::command(async)]
pub(crate) fn note_delete_undo(id: String) -> Result<String, String> {
    let root = notes_root().ok_or_else(off)?;
    if !take_pending(&id) {
        return Err(tr("Smazání už je dokončené — poznámku najdeš v Koši Windows.", "The deletion is already final — you'll find the note in the Windows Recycle Bin.").into());
    }
    match lifecycle::unpark(&root, &id) {
        Ok(restored) => {
            reload_notes();
            send_sync(SyncMsg::Local);
            Ok(restored.first().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())
        }
        Err(e) => {
            // původní místo je obsazené: odklad zůstává, lhůta běží dál a pošle ho do Koše
            PENDING.lock().push(id);
            Err(e)
        }
    }
}

/// Po lhůtě: odklad do Koše (vlákno spí, nic nepolluje). Poznámky se přenačtou (selhání Koše = poznámka je zpět).
fn spawn_commit(root: PathBuf, id: String) {
    let _ = std::thread::Builder::new().name("notes-trash".into()).spawn(move || {
        std::thread::sleep(GRACE);
        if !take_pending(&id) {
            return;
        }
        if let Err(e) = lifecycle::commit(&root, &id, &notes::recycle) {
            log(&format!("poznámky: smazání do Koše selhalo, poznámka zůstala: {e}"));
        }
        reload_notes();
    });
}

/// Při startu: zbytky po pádu nebo ukončení Ledge uprostřed lhůty rovnou do Koše.
pub(crate) fn sweep_trash() {
    let Some(root) = notes_root() else { return };
    let done = lifecycle::sweep(&root, &|id| PENDING.lock().iter().any(|p| p == id), &notes::recycle);
    if done > 0 {
        log(&format!("poznámky: {done}× dokončeno smazání z minula (Koš)"));
        reload_notes();
    }
}
