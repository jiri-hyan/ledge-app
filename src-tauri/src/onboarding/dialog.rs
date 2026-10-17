//! Výběr složky (`IFileOpenDialog` + `FOS_PICKFOLDERS`) pro krok Projekty — jen na výslovný klik.
//! Běží na vlastním STA vlákně; vlastníkem je okno šuplíku, takže blur šuplík nezavře
//! (`commands::drawer::focus_in_own_popup` pozná okno vlastněné šuplíkem).

use std::path::PathBuf;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::{FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};

/// Otevře dialog a počká na výběr. `None` = zrušeno / chyba. Blokuje — volat mimo UI vlákno.
pub fn pick_folder(owner: Option<isize>) -> Option<PathBuf> {
    std::thread::Builder::new()
        .name("pick-folder".into())
        .spawn(move || unsafe {
            let inited = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
            let res = (|| -> Option<PathBuf> {
                let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
                let opts = dlg.GetOptions().ok()?;
                dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST).ok()?;
                dlg.Show(owner.map(|h| HWND(h as *mut _))).ok()?; // zrušení = chyba
                let item = dlg.GetResult().ok()?;
                let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
                let path = name.to_string().ok();
                CoTaskMemFree(Some(name.0 as *const _));
                path.map(PathBuf::from)
            })();
            if inited {
                CoUninitialize();
            }
            res
        })
        .ok()?
        .join()
        .ok()
        .flatten()
}
