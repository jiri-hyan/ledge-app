//! Přetažení položky z přihrádky ven (do Průzkumníka, terminálu s Claude Code, editoru…).
//!
//! UI pozná začátek tahu (`pointermove` s drženým tlačítkem za prahem, `src/shelf/model.ts`) a zavolá
//! `shelf_drag_out`; tady se na hlavním vlákně (STA s OLE, vlastní okno šuplíku — vzor pluginu
//! `tauri-plugin-drag`) spustí `SHDoDragDrop` s datovým objektem shellu (`SHCreateDataObject` z PIDL
//! cest → `CF_HDROP` + formáty shellu, náhled tahu dodá shell). Výchozí zdroj shellu hlídá tlačítko
//! a Esc. Nabízí se jen kopie / odkaz — nikdy přesun (cíl by soubor odnesl z původního místa).
//! Nic se neposílá do žádné session samo: cíl si soubor vezme až po puštění člověkem.

use crate::state::log;
use tauri::AppHandle;
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK, IDropSource, OleInitialize};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHCreateDataObject, SHDoDragDrop};
use windows::core::HSTRING;

/// Spustí tah ven, pokud je tlačítko pořád dole (jinak by se „pustilo“ hned pod kurzorem).
pub fn start(app: &AppHandle, paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() || !crate::screen::primary_button_down() {
        return Ok(());
    }
    app.run_on_main_thread(move || {
        if let Err(e) = unsafe { drag(&paths) } {
            log(&format!("shelf: tah ven {e}"));
        }
    })
    .map_err(|e| e.to_string())
}

unsafe fn drag(paths: &[String]) -> windows::core::Result<()> {
    unsafe {
        // hlavní vlákno Tauri je STA s OLE; podruhé = S_FALSE (v pořádku)
        OleInitialize(None)?;
        let pidls: Vec<*mut ITEMIDLIST> = paths.iter().map(|p| ILCreateFromPathW(&HSTRING::from(p.as_str()))).filter(|p| !p.is_null()).collect();
        if pidls.is_empty() {
            return Ok(());
        }
        let list: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| *p as *const ITEMIDLIST).collect();
        // bez složky = absolutní PIDL (položky můžou být z různých složek)
        let res = SHCreateDataObject::<_, IDataObject>(None, Some(&list), None::<&IDataObject>)
            .and_then(|data| SHDoDragDrop(None, &data, None::<&IDropSource>, DROPEFFECT_COPY | DROPEFFECT_LINK).map(|_| ()));
        for p in pidls {
            ILFree(Some(p));
        }
        res
    }
}
