//! Prostředí tapety, které rozhoduje o pauze a úspoře (kolo 34). Žádný polling: obojí chodí jako událost do
//! skrytého okna `LedgeWallWatcher` (`WTSRegisterSessionNotification` → zamčení stanice, `RegisterPowerSettingNotification`
//! → zdroj napájení). Fullscreen okno / hra / herní režim sem nepatří — to už zná jádro (`Core::dead`) a posílá
//! rozšíření `state{pause,why:"game"}` (`ext::on_dead_changed`), takže se nic nedetekuje dvakrát.
//!
//! - zamčená obrazovka = plocha je zakrytá → stejná pauza jako ve hře (skrýt, `TrySuspend`, žádné animace);
//! - baterie = plocha je vidět, takže se NESKRÝVÁ; scéna přejde do úsporného režimu (`eco`: bez `backdrop-filter`
//!   a bez animace průběhu — nejdražší věci pro GPU).

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, HPOWERNOTIFY, POWERBROADCAST_SETTING, RegisterPowerSettingNotification, SYSTEM_POWER_STATUS, UnregisterPowerSettingNotification,
};
use windows::Win32::System::RemoteDesktop::{NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification};
use windows::Win32::System::SystemServices::GUID_ACDC_POWER_SOURCE;
use windows::Win32::UI::WindowsAndMessaging::{DEVICE_NOTIFY_WINDOW_HANDLE, PBT_POWERSETTINGCHANGE, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE};

const WTS_SESSION_LOCK: usize = 7;
const WTS_SESSION_UNLOCK: usize = 8;

/// Sdílené příznaky: píše je obsluha watcheru, čte vlákno okna po `WM_ENV`.
#[derive(Default)]
pub struct Flags {
    pub locked: AtomicBool,
    pub battery: AtomicBool,
}

/// Zamčení / odemčení z `WM_WTSSESSION_CHANGE` (`wparam`); jiná změna relace (přihlášení, odpojení…) = `None`.
pub fn lock_change(wparam: usize) -> Option<bool> {
    match wparam {
        WTS_SESSION_LOCK => Some(true),
        WTS_SESSION_UNLOCK => Some(false),
        _ => None,
    }
}

/// `GetSystemPowerStatus`: jede stroj na baterii? Stroj bez baterie (bit 128) a neznámý stav (255) = ne.
pub fn on_battery(ac_line: u8, flag: u8) -> bool {
    ac_line == 0 && flag & 128 == 0
}

/// Data oznámení `GUID_ACDC_POWER_SOURCE`: 0 síť, 1 baterie, 2 záložní zdroj (UPS = stále „ne ze sítě“).
pub fn battery_from_source(v: u32) -> bool {
    v == 1 || v == 2
}

/// Má být okno skryté a stránka uspaná? Pauza z jádra (hra / fullscreen) nebo zamčená obrazovka.
pub fn hidden(core_pause: bool, locked: bool) -> bool {
    core_pause || locked
}

/// Čerstvý stav napájení (jednorázově při startu; dál chodí události).
pub fn read_battery() -> bool {
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).is_ok() && on_battery(s.ACLineStatus, s.BatteryFlag) }
}

/// Zpracuje zprávu watcheru. `true` = změnil se některý příznak (vlákno okna se má probudit).
pub fn handle(flags: &Flags, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    match msg {
        WM_WTSSESSION_CHANGE => match lock_change(wp.0) {
            Some(v) => flags.locked.swap(v, Ordering::SeqCst) != v,
            None => false,
        },
        WM_POWERBROADCAST if wp.0 as u32 == PBT_POWERSETTINGCHANGE => {
            let p = lp.0 as *const POWERBROADCAST_SETTING;
            if p.is_null() {
                return false;
            }
            // SAFETY: Windows posílá platnou strukturu po dobu zpracování zprávy; čteme jen GUID, délku a první 4 bajty dat
            let (guid, len, data) = unsafe { ((*p).PowerSetting, (*p).DataLength, std::ptr::addr_of!((*p).Data) as *const u8) };
            if guid != GUID_ACDC_POWER_SOURCE || len < 4 {
                return false;
            }
            let v = unsafe { std::ptr::read_unaligned(data as *const u32) };
            let on = battery_from_source(v);
            flags.battery.swap(on, Ordering::SeqCst) != on
        }
        _ => false,
    }
}

/// Odběr událostí pro `hwnd`; `Drop` ho zruší. Selhání jednoho odběru nevadí (jen se ta změna nepozná).
pub struct Subscription {
    hwnd: HWND,
    power: Mutex<Option<HPOWERNOTIFY>>,
    wts: bool,
}

// HPOWERNOTIFY je jen číslo (handle), používá se z vlákna okna
unsafe impl Send for Subscription {}
unsafe impl Sync for Subscription {}

pub fn subscribe(hwnd: HWND) -> Subscription {
    unsafe {
        let wts = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION).is_ok();
        let power = RegisterPowerSettingNotification(HANDLE(hwnd.0), &GUID_ACDC_POWER_SOURCE, DEVICE_NOTIFY_WINDOW_HANDLE).ok();
        Subscription { hwnd, power: Mutex::new(power), wts }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            if self.wts {
                let _ = WTSUnRegisterSessionNotification(self.hwnd);
            }
            if let Some(p) = self.power.lock().take() {
                let _ = UnregisterPowerSettingNotification(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_change_table() {
        let cases = [(7, Some(true), "zamčení"), (8, Some(false), "odemčení"), (1, None, "připojení konzole"), (5, None, "přihlášení"), (0, None, "nic")];
        for (wp, want, name) in cases {
            assert_eq!(lock_change(wp), want, "{name}");
        }
    }

    #[test]
    fn on_battery_table() {
        let cases = [
            (0, 0, true, "baterie, notebook"),
            (0, 8, true, "baterie, nabíjí (nesmysl, ale ne desktop)"),
            (1, 0, false, "síť"),
            (1, 8, false, "síť, nabíjí"),
            (255, 255, false, "neznámo"),
            (0, 128, false, "bez baterie (desktop)"),
            (0, 255, false, "neznámo i baterie"),
        ];
        for (ac, flag, want, name) in cases {
            assert_eq!(on_battery(ac, flag), want, "{name}");
        }
    }

    #[test]
    fn battery_from_source_table() {
        assert!(!battery_from_source(0));
        assert!(battery_from_source(1));
        assert!(battery_from_source(2));
        assert!(!battery_from_source(3));
    }

    #[test]
    fn hidden_table() {
        assert!(!hidden(false, false));
        assert!(hidden(true, false), "hra");
        assert!(hidden(false, true), "zamčeno");
        assert!(hidden(true, true));
    }

    #[test]
    fn handle_reports_only_real_changes() {
        let f = Flags::default();
        let z = LPARAM(0);
        assert!(handle(&f, WM_WTSSESSION_CHANGE, WPARAM(7), z), "zamčeno = změna");
        assert!(f.locked.load(Ordering::SeqCst));
        assert!(!handle(&f, WM_WTSSESSION_CHANGE, WPARAM(7), z), "znovu zamčeno = beze změny");
        assert!(!handle(&f, WM_WTSSESSION_CHANGE, WPARAM(5), z), "jiná událost relace");
        assert!(handle(&f, WM_WTSSESSION_CHANGE, WPARAM(8), z));
        assert!(!f.locked.load(Ordering::SeqCst));
        assert!(!handle(&f, WM_POWERBROADCAST, WPARAM(PBT_POWERSETTINGCHANGE as usize), z), "prázdný ukazatel");
        assert!(!handle(&f, 0x0001, WPARAM(0), z), "cizí zpráva");
    }

    #[test]
    fn handle_reads_power_setting_payload() {
        // POWERBROADCAST_SETTING { GUID (16), DataLength (4), Data[] } složená bajt po bajtu (u32 data = 4 bajty)
        let build = |guid: windows::core::GUID, v: u32| -> Vec<u8> {
            let mut b = vec![0u8; 24];
            unsafe { std::ptr::write_unaligned(b.as_mut_ptr() as *mut windows::core::GUID, guid) };
            b[16..20].copy_from_slice(&4u32.to_le_bytes());
            b[20..24].copy_from_slice(&v.to_le_bytes());
            b
        };
        let f = Flags::default();
        let wp = WPARAM(PBT_POWERSETTINGCHANGE as usize);
        let call = |b: &Vec<u8>| handle(&f, WM_POWERBROADCAST, wp, LPARAM(b.as_ptr() as isize));
        assert!(call(&build(GUID_ACDC_POWER_SOURCE, 1)), "baterie");
        assert!(f.battery.load(Ordering::SeqCst));
        assert!(call(&build(GUID_ACDC_POWER_SOURCE, 0)), "síť");
        assert!(!f.battery.load(Ordering::SeqCst));
        assert!(!call(&build(windows::core::GUID::zeroed(), 1)), "cizí nastavení");
        assert!(!f.battery.load(Ordering::SeqCst));
    }
}
