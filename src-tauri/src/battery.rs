//! Baterie (záložka Osobní, karta Čas): `GetSystemPowerStatus` — procenta, nabíjí / v síti / na
//! baterii, zbývající čas, pokud ho Windows odhadne. Stroj bez baterie (desktop, `BatteryFlag` 128)
//! nic neukazuje a nic neodebírá.
//!
//! Žádný polling ani okno navíc: při prvním zobrazení záložky Osobní (`ensure`) se stav přečte a na
//! stroji s baterií se zaregistruje `PowerSettingRegisterNotification` s callbackem
//! (`DEVICE_NOTIFY_CALLBACK`) pro zdroj napájení a procenta baterie — Windows zavolá, když se změní
//! (≈ 1× za procento), callback stav přečte znovu a probudí `emit`. Vypnutý údaj
//! (`drawer.sections.battery`) nebo záložka Osobní → `sync` registraci zruší.

use crate::config::Config;
use crate::state::{core, log, mark_dirty};
use parking_lot::Mutex;
use serde::Serialize;
use std::ffi::c_void;
use std::sync::OnceLock;
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, GetSystemPowerStatus, HPOWERNOTIFY, PowerSettingRegisterNotification, PowerSettingUnregisterNotification,
    SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING};
use windows::Win32::UI::WindowsAndMessaging::{DEVICE_NOTIFY_CALLBACK, PBT_POWERSETTINGCHANGE};

/// Surová pole `SYSTEM_POWER_STATUS` (testovatelné bez Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw {
    /// 0 baterie, 1 síť, 255 neznámo
    pub ac_line: u8,
    /// bity: 1 vysoká, 2 nízká, 4 kritická, 8 nabíjí, 128 bez baterie; 255 neznámo
    pub flag: u8,
    /// 0–100, 255 neznámo
    pub percent: u8,
    /// zbývá v sekundách, `u32::MAX` = neznámo (na síti vždy)
    pub life_s: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryView {
    pub percent: u8,
    /// nabíjí se
    pub charging: bool,
    /// v síti (nabitá baterie v síti = `plugged` bez `charging`)
    pub plugged: bool,
    /// odhad Windows, jen na baterii; `None` = Windows neví (UI nic nevymýšlí)
    pub remaining_s: Option<u32>,
}

/// bit „bez systémové baterie“ — má ho i 255 (neznámo), obojí = nic neukazovat
const NO_BATTERY: u8 = 128;
const CHARGING: u8 = 8;

static VIEW: Mutex<Option<BatteryView>> = Mutex::new(None);
/// zaregistrované odběry (`HPOWERNOTIFY` jako číslo); `Some(prázdné)` = stroj bez baterie, nic neodebírá
static REG: Mutex<Option<Vec<isize>>> = Mutex::new(None);
static WANTED: Mutex<bool> = Mutex::new(false);

// ---------- čistá logika ----------

pub(crate) fn allowed(cfg: &Config) -> bool {
    cfg.modes.personal_enabled && cfg.drawer.sections.battery
}

/// Má stroj systémovou baterii? (desktop hlásí `BatteryFlag` 128)
pub(crate) fn has_battery(r: Raw) -> bool {
    r.flag & NO_BATTERY == 0
}

/// `SYSTEM_POWER_STATUS` → údaj pro UI. Bez baterie, s neznámým stavem (255) nebo neznámými
/// procenty `None`. Zbývající čas jen na baterii (v síti ho Windows občas hlásí, ale neznamená nic).
pub(crate) fn interpret(r: Raw) -> Option<BatteryView> {
    if !has_battery(r) || r.percent > 100 {
        return None;
    }
    let plugged = r.ac_line == 1;
    let charging = r.flag & CHARGING != 0;
    let remaining_s = (!plugged && r.life_s != u32::MAX && r.life_s > 0).then_some(r.life_s);
    Some(BatteryView { percent: r.percent, charging, plugged, remaining_s })
}

// ---------- za běhu ----------

pub(crate) fn view() -> Option<BatteryView> {
    VIEW.lock().clone()
}

fn raw() -> Option<Raw> {
    let mut st = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut st) }.ok()?;
    Some(Raw { ac_line: st.ACLineStatus, flag: st.BatteryFlag, percent: st.BatteryLifePercent, life_s: st.BatteryLifeTime })
}

fn refresh() {
    let v = raw().and_then(interpret);
    let mut g = VIEW.lock();
    if *g != v {
        *g = v;
        drop(g);
        mark_dirty();
    }
}

/// Callback z `PowerSettingRegisterNotification` (vlákno Windows) — jen přečíst stav znovu.
unsafe extern "system" fn on_power(_ctx: *const c_void, kind: u32, _setting: *const c_void) -> u32 {
    if kind == PBT_POWERSETTINGCHANGE {
        refresh();
    }
    0
}

/// Parametry odběru musí žít, dokud registrace trvá — jeden kus na celý běh (adresa jako číslo).
fn params() -> HANDLE {
    static P: OnceLock<usize> = OnceLock::new();
    let addr = *P.get_or_init(|| Box::into_raw(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS { Callback: Some(on_power), Context: std::ptr::null_mut() })) as usize);
    HANDLE(addr as *mut c_void)
}

fn start(reg: &mut Option<Vec<isize>>) {
    if reg.is_some() {
        return;
    }
    let r = raw();
    refresh();
    let mut handles = Vec::new();
    if r.is_some_and(has_battery) {
        for guid in [GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING] {
            let mut h: *mut c_void = std::ptr::null_mut();
            let e = unsafe { PowerSettingRegisterNotification(&guid, DEVICE_NOTIFY_CALLBACK, params(), &mut h) };
            if e == ERROR_SUCCESS {
                handles.push(h as isize);
            } else {
                log(&format!("battery: odběr změn napájení selhal ({})", e.0));
            }
        }
    }
    *reg = Some(handles);
}

fn stop(reg: &mut Option<Vec<isize>>) {
    if let Some(hs) = reg.take() {
        for h in hs {
            let _ = unsafe { PowerSettingUnregisterNotification(HPOWERNOTIFY(h)) };
        }
        *VIEW.lock() = None;
        mark_dirty();
    }
}

pub(crate) fn ensure() {
    let ok = allowed(&core().cfg.read());
    *WANTED.lock() = true;
    if ok {
        start(&mut REG.lock());
    }
}

pub(crate) fn sync(cfg: &Config) {
    let mut reg = REG.lock();
    if !allowed(cfg) {
        stop(&mut reg);
    } else if *WANTED.lock() {
        start(&mut reg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: u32 = u32::MAX;

    #[test]
    fn interpret_power_status() {
        let r = |ac_line, flag, percent, life_s| Raw { ac_line, flag, percent, life_s };
        let v = |percent, charging, plugged, remaining_s| Some(BatteryView { percent, charging, plugged, remaining_s });
        let cases = [
            ("desktop bez baterie", r(1, 128, 255, MAX), None),
            ("desktop, procenta 0", r(1, 128, 0, MAX), None),
            ("neznámá procenta", r(0, 255, 255, MAX), None),
            ("na baterii s odhadem", r(0, 1, 82, 7_800), v(82, false, false, Some(7_800))),
            ("na baterii bez odhadu", r(0, 2, 30, MAX), v(30, false, false, None)),
            ("nízká a kritická", r(0, 2 | 4, 4, 600), v(4, false, false, Some(600))),
            ("nabíjí se v síti", r(1, 8 | 1, 64, MAX), v(64, true, true, None)),
            ("v síti, nabito (bez bitu 8)", r(1, 1, 100, MAX), v(100, false, true, None)),
            ("v síti, Windows přesto dá čas → neukazovat", r(1, 8, 50, 3_600), v(50, true, true, None)),
            ("flag neznámý (255) = nevíme nic", r(0, 255, 55, 1_200), None),
            ("nulový odhad = neznámo", r(0, 1, 90, 0), v(90, false, false, None)),
        ];
        for (name, raw, want) in cases {
            assert_eq!(interpret(raw), want, "{name}");
        }
    }

    #[test]
    fn has_battery_by_flag() {
        let r = |flag| Raw { ac_line: 1, flag, percent: 50, life_s: MAX };
        assert!(!has_battery(r(128)));
        assert!(!has_battery(r(128 | 8)));
        assert!(has_battery(r(0)));
        assert!(has_battery(r(8)));
        assert!(!has_battery(r(255)), "255 (neznámo) má bit 128 → neregistrovat nic");
    }
}
