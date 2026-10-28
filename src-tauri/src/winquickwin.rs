//! Windows API karty Rychlé dlaždice (`winquick.rs` = čistá logika): Known Folders, motiv v registru + rozeslání
//! `WM_SETTINGCHANGE`, rádia Wi-Fi / Bluetooth (`Windows.Devices.Radios`), poslední snímek obrazovky a obrázek do
//! schránky. Všechno jen na volání z příkazu (klik / otevření viditelné karty); žádné vlákno ani watcher.
//! Testy sem nesahají (registr, rádia ani schránka se v nich nemění).

use crate::i18n::tr;
use crate::winquick::{RadioKind, ThemeStore};
use base64::Engine;
use parking_lot::Mutex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind as WinRadioKind, RadioState};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFOHEADER};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_DIB;
use windows::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Screenshots, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE};
use windows::core::w;

// ---------- Known Folders ----------

/// `downloads` / `desktop` / `documents` / `screenshots` → cesta (jen existující složka).
pub fn known_dir(key: &str) -> Option<PathBuf> {
    let id = match key {
        "downloads" => &FOLDERID_Downloads,
        "desktop" => &FOLDERID_Desktop,
        "documents" => &FOLDERID_Documents,
        "screenshots" => &FOLDERID_Screenshots,
        _ => return None,
    };
    let _com = crate::iconimg::Com::init();
    let p = unsafe {
        let pw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let s = pw.to_string().ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(pw.0 as *const _));
        s?
    };
    let p = PathBuf::from(p);
    p.is_dir().then_some(p)
}

// ---------- motiv ----------

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

pub struct RegTheme;

impl ThemeStore for RegTheme {
    fn apps_light(&self) -> Option<u32> {
        let k = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(PERSONALIZE).ok()?;
        k.get_value::<u32, _>("AppsUseLightTheme").ok()
    }

    fn set(&mut self, light: bool) -> Result<(), String> {
        let err = |e: std::io::Error| crate::errors::any(tr("Motiv Windows nejde přepnout", "Can't switch the Windows theme"), &e);
        let k = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey_with_flags(PERSONALIZE, winreg::enums::KEY_SET_VALUE)
            .map_err(err)?;
        let v = light as u32;
        k.set_value("AppsUseLightTheme", &v).map_err(err)?;
        k.set_value("SystemUsesLightTheme", &v).map_err(err)?;
        // jako Nastavení Windows: aplikace a Průzkumník si změnu přečtou
        let wide: Vec<u16> = "ImmersiveColorSet".encode_utf16().chain([0]).collect();
        unsafe {
            let _ = SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, WPARAM(0), LPARAM(wide.as_ptr() as isize), SMTO_ABORTIFHUNG, 300, None);
        }
        Ok(())
    }
}

// ---------- rádia ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadioInfo {
    pub kind: RadioKind,
    pub name: String,
    pub on: bool,
}

fn map_kind(k: WinRadioKind) -> Option<RadioKind> {
    if k == WinRadioKind::WiFi {
        Some(RadioKind::Wifi)
    } else if k == WinRadioKind::Bluetooth {
        Some(RadioKind::Bluetooth)
    } else {
        None
    }
}

/// Wi-Fi a Bluetooth rádia stroje (prázdné = žádné, např. stolní PC s kabelem). Chyba = prázdný seznam.
pub fn radios() -> Vec<RadioInfo> {
    let _com = crate::iconimg::Com::init();
    let run = || -> windows::core::Result<Vec<RadioInfo>> {
        let list = Radio::GetRadiosAsync()?.join()?;
        let mut out = Vec::new();
        for r in list {
            if let Some(kind) = map_kind(r.Kind()?) {
                out.push(RadioInfo { kind, name: r.Name()?.to_string(), on: r.State()? == RadioState::On });
            }
        }
        Ok(out)
    };
    run().unwrap_or_default()
}

pub fn set_radio(kind: RadioKind, on: bool) -> Result<(), String> {
    let _com = crate::iconimg::Com::init();
    let ctx = tr("Rádio nejde přepnout", "Can't switch the radio");
    let denied = || tr("Windows nepovolily ovládání rádií (Nastavení → Soukromí → Rádia).", "Windows denied radio control (Settings → Privacy → Radios).").to_string();
    let access = Radio::RequestAccessAsync().and_then(|a| a.join()).map_err(|e| crate::errors::any(ctx, &e))?;
    if access != RadioAccessStatus::Allowed {
        return Err(denied());
    }
    let list = Radio::GetRadiosAsync().and_then(|a| a.join()).map_err(|e| crate::errors::any(ctx, &e))?;
    let radio = list
        .into_iter()
        .find(|r| r.Kind().ok().and_then(map_kind) == Some(kind))
        .ok_or_else(|| tr("Tenhle stroj takové rádio nemá.", "This machine has no such radio.").to_string())?;
    let st = if on { RadioState::On } else { RadioState::Off };
    let res = radio.SetStateAsync(st).and_then(|a| a.join()).map_err(|e| crate::errors::any(ctx, &e))?;
    if res == RadioAccessStatus::Allowed { Ok(()) } else { Err(denied()) }
}

// ---------- poslední snímek obrazovky ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShotView {
    pub name: String,
    pub path: String,
    /// obrázků ve složce
    pub count: usize,
    /// PNG ≤ 128 px jako data URL; `None` = nejde dekódovat
    pub thumb: Option<String>,
}

type ShotCache = Option<(PathBuf, SystemTime, Option<ShotView>)>;
static SHOT: Mutex<ShotCache> = Mutex::new(None);
const MAX_SHOT_BYTES: u64 = 40 * 1024 * 1024;

fn ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Nejnovější snímek ve složce Snímky obrazovky. Bez watcheru: dokud se čas složky nezměnil, vrací se uložený výsledek.
pub fn latest_shot() -> Option<ShotView> {
    let dir = known_dir("screenshots")?;
    let dir_mtime = std::fs::metadata(&dir).and_then(|m| m.modified()).ok()?;
    if let Some((d, t, v)) = SHOT.lock().as_ref()
        && *d == dir
        && *t == dir_mtime
    {
        return v.clone();
    }
    let view = scan(&dir);
    *SHOT.lock() = Some((dir, dir_mtime, view.clone()));
    view
}

fn scan(dir: &Path) -> Option<ShotView> {
    let entries: Vec<(String, u64)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.metadata().and_then(|m| m.modified()).map(ms).unwrap_or(0)))
        .collect();
    let (best, count) = crate::winquick::newest_image(&entries);
    let name = entries[best?].0.clone();
    let path = dir.join(&name);
    let thumb = std::fs::metadata(&path)
        .ok()
        .filter(|m| m.len() <= MAX_SHOT_BYTES)
        .and_then(|_| std::fs::read(&path).ok())
        .and_then(|b| crate::iconimg::to_png(&b))
        .map(|png| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)));
    Some(ShotView { name, path: path.to_string_lossy().to_string(), count, thumb })
}

// ---------- schránka ----------

fn open_clipboard() -> Result<(), String> {
    for _ in 0..10 {
        if unsafe { OpenClipboard(None) }.is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    Err(tr("Schránka je obsazená jinou aplikací — zkus to znovu.", "The clipboard is busy in another app — try again.").into())
}

/// Vloží blok paměti do otevřené schránky; po úspěchu patří schránce.
unsafe fn put(format: u32, data: &[&[u8]]) -> windows::core::Result<()> {
    unsafe {
        let total: usize = data.iter().map(|d| d.len()).sum();
        let h: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, total)?;
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            let _ = GlobalFree(Some(h));
            return Err(windows::core::Error::from_thread());
        }
        let mut off = 0;
        for d in data {
            std::ptr::copy_nonoverlapping(d.as_ptr(), p.add(off), d.len());
            off += d.len();
        }
        let _ = GlobalUnlock(h);
        if let Err(e) = SetClipboardData(format, Some(HANDLE(h.0))) {
            let _ = GlobalFree(Some(h));
            return Err(e);
        }
        Ok(())
    }
}

/// Obrázek ze souboru do schránky: `CF_DIB` (zdola nahoru) a u PNG i formát „PNG“ (bez ztráty průhlednosti).
pub fn copy_image(path: &Path) -> Result<(), String> {
    let ctx = tr("Obrázek nejde zkopírovat", "Can't copy the image");
    if std::fs::metadata(path).map_err(|e| crate::errors::any(ctx, &e))?.len() > MAX_SHOT_BYTES {
        return Err(tr("Obrázek je příliš velký.", "The image is too large.").into());
    }
    let bytes = std::fs::read(path).map_err(|e| crate::errors::any(ctx, &e))?;
    let (w, h, px) = crate::iconimg::decode_bgra(&bytes, 64 * 1024 * 1024 / 4).ok_or_else(|| tr("Obrázek nejde přečíst.", "Can't read the image.").to_string())?;
    let dib = dib_pixels(w, h, &px);
    let hdr = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w as i32, biHeight: h as i32, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, biSizeImage: dib.len() as u32, ..Default::default() };
    // SAFETY: BITMAPINFOHEADER je POD struktura bez výplně (40 B)
    let hdr_bytes = unsafe { std::slice::from_raw_parts((&hdr as *const BITMAPINFOHEADER).cast::<u8>(), std::mem::size_of::<BITMAPINFOHEADER>()) };
    let is_png = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
    open_clipboard()?;
    let res = unsafe {
        (|| -> windows::core::Result<()> {
            EmptyClipboard()?;
            put(CF_DIB.0 as u32, &[hdr_bytes, &dib])?;
            if is_png {
                let f = RegisterClipboardFormatW(w!("PNG"));
                if f != 0 {
                    put(f, &[&bytes])?;
                }
            }
            Ok(())
        })()
    };
    unsafe {
        let _ = CloseClipboard();
    }
    res.map_err(|e| crate::errors::any(ctx, &e))
}

/// Řádky shora dolů → zdola nahoru (jak čeká `CF_DIB` s kladnou výškou).
pub(crate) fn dib_pixels(w: u32, h: u32, top_down: &[u8]) -> Vec<u8> {
    let row = w as usize * 4;
    let mut out = Vec::with_capacity(top_down.len());
    for y in (0..h as usize).rev() {
        out.extend_from_slice(&top_down[y * row..(y + 1) * row]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dib_rows_are_flipped() {
        // 1×3 px, řádky 0,1,2 → 2,1,0
        let top_down = [0, 0, 0, 255, 1, 1, 1, 255, 2, 2, 2, 255];
        assert_eq!(dib_pixels(1, 3, &top_down), vec![2, 2, 2, 255, 1, 1, 1, 255, 0, 0, 0, 255]);
        assert!(dib_pixels(2, 0, &[]).is_empty());
    }
}
