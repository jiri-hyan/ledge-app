//! Text do schránky Windows (`CF_UNICODETEXT`) — „Kopírovat cestu“ v přihrádce. Přes Rust, ne přes
//! `navigator.clipboard`: šuplík nemá fokus (Ledge ho nekrade) a WebView bez fokusu zápis odmítne.

use crate::i18n::tr;
use std::time::Duration;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

pub fn set_text(s: &str) -> Result<(), String> {
    let wide: Vec<u16> = s.encode_utf16().chain([0]).collect();
    unsafe {
        // schránku může mít chvíli otevřenou jiná aplikace
        let mut open = false;
        for _ in 0..10 {
            if OpenClipboard(None).is_ok() {
                open = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        if !open {
            return Err(tr("Schránka je obsazená jinou aplikací — zkus to znovu.", "The clipboard is busy in another app — try again.").into());
        }
        let res = (|| -> windows::core::Result<()> {
            EmptyClipboard()?;
            let h: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2)?;
            let p = GlobalLock(h) as *mut u16;
            if p.is_null() {
                let _ = GlobalFree(Some(h));
                return Err(windows::core::Error::from_thread());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
            let _ = GlobalUnlock(h);
            // po úspěchu paměť patří schránce
            if let Err(e) = SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(h.0))) {
                let _ = GlobalFree(Some(h));
                return Err(e);
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        res.map_err(|e| crate::errors::any(tr("Do schránky to nejde zapsat", "Can't write to the clipboard"), &e))
    }
}
