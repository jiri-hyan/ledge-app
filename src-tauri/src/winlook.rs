//! Stav Windows pro motiv „Podle Windows“ (`appearance.auto`, kolo 34): světlý / tmavý z `AppsUseLightTheme`
//! (čtení sdílí `winquickwin::RegTheme`) a akcentová barva z `HKCU\…\DWM` (`AccentColor` ABGR, jinak
//! `ColorizationColor` ARGB). Změny hlídá vlákno `winlook` na `RegNotifyChangeKeyValue` (událost jádra, spí v
//! `WaitForMultipleObjects`) — žádný polling ani časovač; běží JEN při zapnutém `auto.on` / `auto.accent` (`sync`).
//! Okna `HWND_MESSAGE` `WM_SETTINGCHANGE` nedostávají, proto registr. Při změně: událost `winlook` všem oknům
//! (šuplík / Studio přepočtou motiv) a překreslení proužku. Testy sem nesahají na registr (jen čisté převody).

use crate::appearance::auto::WinLook;
use crate::winquick::ThemeStore;
use parking_lot::Mutex;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ};

const DWM: &str = r"Software\Microsoft\Windows\DWM";
const REG_NOTIFY_CHANGE_LAST_SET: u32 = 4;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegNotifyChangeKeyValue(hkey: isize, watch_subtree: i32, filter: u32, event: isize, asynchronous: i32) -> i32;
}

/// poslední známý stav (jen při běžícím watcheru)
static CUR: Mutex<Option<WinLook>> = Mutex::new(None);
/// stop událost běžícího watcheru (`None` = neběží)
static STOP: Mutex<Option<isize>> = Mutex::new(None);

/// `AccentColor` (DWORD 0xAABBGGRR) → `#rrggbb`.
pub fn accent_abgr(v: u32) -> String {
    format!("#{:02x}{:02x}{:02x}", v & 255, (v >> 8) & 255, (v >> 16) & 255)
}

/// `ColorizationColor` (DWORD 0xAARRGGBB) → `#rrggbb`.
pub fn accent_argb(v: u32) -> String {
    format!("#{:02x}{:02x}{:02x}", (v >> 16) & 255, (v >> 8) & 255, v & 255)
}

/// Přečte stav z registru (read-only).
pub fn read() -> WinLook {
    let light = crate::winquickwin::RegTheme.apps_light().map(|v| v != 0);
    let dwm = RegKey::predef(HKEY_CURRENT_USER).open_subkey(DWM).ok();
    let accent = dwm.as_ref().and_then(|k| {
        k.get_value::<u32, _>("AccentColor").ok().map(accent_abgr).or_else(|| k.get_value::<u32, _>("ColorizationColor").ok().map(accent_argb))
    });
    WinLook { light, accent }
}

/// Aktuální stav: z paměti, když watcher běží, jinak čerstvě z registru.
pub fn current() -> WinLook {
    CUR.lock().clone().unwrap_or_else(read)
}

/// Spustí / zastaví watcher podle configu (`auto.watching()`); idempotentní, volá se při změně configu a při překreslení proužku.
pub fn sync(want: bool) {
    let mut g = STOP.lock();
    if want && g.is_none() {
        let Ok(ev) = (unsafe { CreateEventW(None, true, false, None) }) else { return };
        *CUR.lock() = Some(read());
        let h = ev.0 as isize;
        if std::thread::Builder::new().name("winlook".into()).spawn(move || run(h)).is_ok() {
            *g = Some(h);
        } else {
            unsafe {
                let _ = CloseHandle(ev);
            }
            *CUR.lock() = None;
        }
    } else if !want && let Some(h) = g.take() {
        *CUR.lock() = None;
        unsafe {
            let _ = SetEvent(HANDLE(h as *mut _));
        }
    }
}

pub fn on_config(_old: &crate::config::Config, new: &crate::config::Config) {
    sync(new.appearance.auto.watching());
}

fn run(stop: isize) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let keys: Vec<RegKey> =
        [crate::winquickwin::PERSONALIZE, DWM].iter().filter_map(|p| hkcu.open_subkey_with_flags(p, KEY_NOTIFY | KEY_READ).ok()).collect();
    let evs: Vec<HANDLE> = keys.iter().filter_map(|_| unsafe { CreateEventW(None, false, false, None).ok() }).collect();
    let stop_h = HANDLE(stop as *mut _);
    if keys.is_empty() || evs.len() != keys.len() {
        close(&evs, stop_h);
        forget_stop(stop);
        return;
    }
    let arm = |i: usize| unsafe {
        RegNotifyChangeKeyValue(keys[i].raw_handle() as isize, 0, REG_NOTIFY_CHANGE_LAST_SET, evs[i].0 as isize, 1);
    };
    (0..keys.len()).for_each(arm);
    let mut all = evs.clone();
    all.push(stop_h);
    loop {
        let r = unsafe { WaitForMultipleObjects(&all, false, INFINITE) };
        let i = r.0.wrapping_sub(WAIT_OBJECT_0.0) as usize;
        if i >= keys.len() {
            break; // stop (nebo chyba čekání)
        }
        arm(i); // nejdřív znovu odebírat, pak číst — změna mezi tím nepropadne
        publish(read());
    }
    close(&evs, stop_h);
    forget_stop(stop);
}

/// Vlákno končí a zavřelo `stop` handle: pokud ho `STOP` ještě drží (předčasný konec, chyba čekání), vynulovat
/// ho i `CUR`, aby `sync(true)` mohl watcher znovu spustit a `sync(false)` nevolal `SetEvent` na mrtvý handle (L172).
fn forget_stop(stop: isize) {
    if clear_if_same(&mut STOP.lock(), stop) {
        *CUR.lock() = None;
    }
}

fn clear_if_same(slot: &mut Option<isize>, stop: isize) -> bool {
    (*slot == Some(stop)).then(|| *slot = None).is_some()
}

fn close(evs: &[HANDLE], stop: HANDLE) {
    unsafe {
        evs.iter().for_each(|e| {
            let _ = CloseHandle(*e);
        });
        let _ = CloseHandle(stop);
    }
}

/// Nový stav → paměť; změnil-li se, událost oknům a překreslení proužku.
fn publish(now: WinLook) {
    {
        let mut c = CUR.lock();
        if c.is_none() || c.as_ref() == Some(&now) {
            return; // watcher už je zastavený, nebo se nic nezměnilo (registr píše víc hodnot naráz)
        }
        *c = Some(now.clone());
    }
    use tauri::Emitter;
    let _ = crate::state::core().app.emit("winlook", &now);
    crate::state::mark_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_exit_clears_only_own_stop_handle() {
        let mut s = Some(7);
        assert!(!clear_if_same(&mut s, 8), "cizí handle se nesmí smazat");
        assert_eq!(s, Some(7));
        assert!(clear_if_same(&mut s, 7));
        assert_eq!(s, None);
        assert!(!clear_if_same(&mut s, 7), "už zastavený (sync(false)) = nic");
    }

    #[test]
    fn accent_dword_order() {
        // 0xAABBGGRR: Windows modrá #0078d4 = R 0x00 G 0x78 B 0xd4
        assert_eq!(accent_abgr(0xffd47800), "#0078d4");
        assert_eq!(accent_argb(0xff0078d4), "#0078d4");
        assert_eq!(accent_abgr(0), "#000000");
        assert_eq!(accent_abgr(0xff00ff00), "#00ff00");
    }
}
