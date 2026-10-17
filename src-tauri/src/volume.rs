//! Hlasitost (záložka Osobní): výchozí výstupní zařízení přes Core Audio (`IMMDeviceEnumerator` →
//! `IAudioEndpointVolume`) — hlasitost 0–100, ztlumení, název zařízení.
//!
//! Žádný polling: vlákno `volume` (COM MTA) vznikne až při prvním zobrazení záložky Osobní
//! (`ensure`), zaregistruje `IAudioEndpointVolumeCallback` (změna hlasitosti zvenku — klávesy,
//! mixér) a `IMMNotificationClient` (jiné výchozí zařízení → přepojit) a jinak spí na kanálu. Callbacky
//! jen přepíšou paměť / pošlou zprávu. Posuvník: UI posílá při tahu nejvýš ~20×/s (`src/media/model.ts`
//! `throttle`), vlákno z nahromaděných zpráv použije jen poslední hodnotu. Vypnutá karta
//! (`drawer.sections.sound`) nebo záložka Osobní → `sync` odběry zruší a vlákno skončí.

use crate::config::Config;
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::mpsc::{self, Receiver, Sender};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
    MMDeviceEnumerator, eMultimedia, eRender,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::{GUID, PCWSTR, implement};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeView {
    /// 0–100
    pub level: u8,
    pub muted: bool,
    /// název výchozího výstupu („Reproduktory (Realtek…)“); prázdné = nevíme
    pub device: String,
}

enum Msg {
    Set(u8),
    Mute(bool),
    Rebind,
    Stop,
}

struct Live {
    tx: Option<Sender<Msg>>,
    wanted: bool,
}

static LIVE: Mutex<Live> = Mutex::new(Live { tx: None, wanted: false });
static VIEW: Mutex<Option<VolumeView>> = Mutex::new(None);

/// `PKEY_Device_FriendlyName` (functiondiscoverykeys) — bez další feature crate `windows`.
const FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0), pid: 14 };

// ---------- čistá logika ----------

pub(crate) fn allowed(cfg: &Config) -> bool {
    cfg.modes.personal_enabled && cfg.drawer.sections.sound
}

/// Skalár Core Audio (0.0–1.0) → procenta 0–100 (zaokrouhleno; mimo rozsah / NaN oříznuto).
pub(crate) fn to_pct(scalar: f32) -> u8 {
    if scalar.is_nan() {
        return 0;
    }
    (scalar.clamp(0.0, 1.0) * 100.0).round() as u8
}

/// Procenta z posuvníku → skalár Core Audio (víc než 100 = 100).
pub(crate) fn to_scalar(pct: u8) -> f32 {
    f32::from(pct.min(100)) / 100.0
}

// ---------- za běhu ----------

pub(crate) fn view() -> Option<VolumeView> {
    VIEW.lock().clone()
}

pub(crate) fn ensure() {
    let ok = allowed(&core().cfg.read());
    let mut l = LIVE.lock();
    l.wanted = true;
    if ok && l.tx.is_none() {
        l.tx = spawn();
    }
}

pub(crate) fn sync(cfg: &Config) {
    let ok = allowed(cfg);
    let mut l = LIVE.lock();
    if !ok {
        if let Some(tx) = l.tx.take() {
            let _ = tx.send(Msg::Stop);
            set_view(None);
        }
    } else if l.wanted && l.tx.is_none() {
        l.tx = spawn();
    }
}

fn send(m: Msg) -> Result<(), String> {
    LIVE.lock().tx.as_ref().and_then(|tx| tx.send(m).ok()).ok_or_else(|| tr("Ovládání hlasitosti neběží.", "Volume control isn't running.").to_string())
}

/// Posuvník (0–100).
pub(crate) fn set(level: u8) -> Result<(), String> {
    send(Msg::Set(level.min(100)))
}

pub(crate) fn mute(muted: bool) -> Result<(), String> {
    send(Msg::Mute(muted))
}

fn set_view(v: Option<VolumeView>) {
    let mut g = VIEW.lock();
    if *g != v {
        *g = v;
        drop(g);
        mark_dirty();
    }
}

fn update(level: u8, muted: bool) {
    let mut g = VIEW.lock();
    if let Some(v) = g.as_mut()
        && (v.level != level || v.muted != muted)
    {
        v.level = level;
        v.muted = muted;
        drop(g);
        mark_dirty();
    }
}

/// Změna hlasitosti / ztlumení (i naše vlastní) — jen zápis do paměti.
#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCb;

impl IAudioEndpointVolumeCallback_Impl for VolumeCb_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        if let Some(d) = unsafe { pnotify.as_ref() } {
            update(to_pct(d.fMasterVolume), d.bMuted.as_bool());
        }
        Ok(())
    }
}

/// Jiné výchozí zařízení (sluchátka, HDMI…) → vlákno se přepojí. Uvnitř callbacku se nesmí volat
/// enumerátor, proto jen zpráva.
#[implement(IMMNotificationClient)]
struct DeviceCb {
    tx: Sender<Msg>,
}

impl IMMNotificationClient_Impl for DeviceCb_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> windows::core::Result<()> {
        if flow == eRender && role == eMultimedia {
            let _ = self.tx.send(Msg::Rebind);
        }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> windows::core::Result<()> {
        Ok(())
    }
}

fn spawn() -> Option<Sender<Msg>> {
    let (tx, rx) = mpsc::channel();
    let own = tx.clone();
    std::thread::Builder::new().name("volume".into()).spawn(move || run(rx, own)).ok()?;
    Some(tx)
}

fn device_name(dev: &IMMDevice) -> String {
    unsafe {
        let Ok(store) = dev.OpenPropertyStore(STGM_READ) else { return String::new() };
        let Ok(mut pv) = store.GetValue(&FRIENDLY_NAME) else { return String::new() };
        let name = match PropVariantToStringAlloc(&pv) {
            Ok(p) => {
                let s = p.to_string().unwrap_or_default();
                CoTaskMemFree(Some(p.0 as _));
                s
            }
            Err(_) => String::new(),
        };
        let _ = PropVariantClear(&mut pv);
        name
    }
}

/// Výchozí výstup → odběr změn + první stav. Bez zařízení `None` (karta ukáže, že není kam hrát).
fn bind(en: &IMMDeviceEnumerator, cb: &IAudioEndpointVolumeCallback) -> Option<IAudioEndpointVolume> {
    let ep = unsafe {
        let Ok(dev) = en.GetDefaultAudioEndpoint(eRender, eMultimedia) else {
            set_view(None);
            return None;
        };
        let ep: IAudioEndpointVolume = match dev.Activate(CLSCTX_ALL, None) {
            Ok(ep) => ep,
            Err(e) => {
                log(&format!("volume: IAudioEndpointVolume: {e}"));
                set_view(None);
                return None;
            }
        };
        if let Err(e) = ep.RegisterControlChangeNotify(cb) {
            log(&format!("volume: odběr změn: {e}"));
        }
        let level = ep.GetMasterVolumeLevelScalar().map(to_pct).unwrap_or(0);
        let muted = ep.GetMute().is_ok_and(|b| b.as_bool());
        set_view(Some(VolumeView { level, muted, device: device_name(&dev) }));
        ep
    };
    Some(ep)
}

fn unbind(ep: Option<IAudioEndpointVolume>, cb: &IAudioEndpointVolumeCallback) {
    if let Some(ep) = ep {
        let _ = unsafe { ep.UnregisterControlChangeNotify(cb) };
    }
}

fn run(rx: Receiver<Msg>, tx: Sender<Msg>) {
    let _com = crate::iconimg::Com::init();
    let en: IMMDeviceEnumerator = match unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) } {
        Ok(en) => en,
        Err(e) => {
            log(&format!("volume: Core Audio nedostupné: {e}"));
            return;
        }
    };
    let client: IMMNotificationClient = DeviceCb { tx }.into();
    let _ = unsafe { en.RegisterEndpointNotificationCallback(&client) };
    let cb: IAudioEndpointVolumeCallback = VolumeCb.into();
    let mut ep = bind(&en, &cb);
    while let Ok(first) = rx.recv() {
        // z nahromaděných zpráv (tah posuvníku) platí poslední hodnota
        let (mut level, mut muted, mut rebind, mut stop) = (None, None, false, false);
        for m in std::iter::once(first).chain(rx.try_iter()) {
            match m {
                Msg::Set(v) => level = Some(v),
                Msg::Mute(v) => muted = Some(v),
                Msg::Rebind => rebind = true,
                Msg::Stop => stop = true,
            }
        }
        if stop {
            break;
        }
        if rebind {
            unbind(ep.take(), &cb);
            ep = bind(&en, &cb);
        }
        let Some(e) = &ep else { continue };
        unsafe {
            if let Some(v) = level
                && let Err(err) = e.SetMasterVolumeLevelScalar(to_scalar(v), std::ptr::null())
            {
                log(&format!("volume: nastavení hlasitosti: {err}"));
            }
            if let Some(m) = muted
                && let Err(err) = e.SetMute(m, std::ptr::null())
            {
                log(&format!("volume: ztlumení: {err}"));
            }
        }
    }
    unbind(ep, &cb);
    let _ = unsafe { en.UnregisterEndpointNotificationCallback(&client) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_and_percent() {
        let to = [(0.0, 0), (1.0, 100), (0.5, 50), (0.304, 30), (0.306, 31), (0.999, 100), (-0.2, 0), (1.7, 100), (f32::NAN, 0)];
        for (s, want) in to {
            assert_eq!(to_pct(s), want, "{s}");
        }
        let back = [(0, 0.0), (100, 1.0), (37, 0.37), (250, 1.0)];
        for (p, want) in back {
            assert!((to_scalar(p) - want).abs() < 1e-6, "{p}");
        }
        // tam a zpět beze ztráty pro každé procento posuvníku
        for p in 0..=100u8 {
            assert_eq!(to_pct(to_scalar(p)), p);
        }
    }

    #[test]
    fn allowed_by_config() {
        let on = Config::default();
        assert!(allowed(&on));
        let off: Config = serde_json::from_str(r#"{"drawer":{"sections":{"sound":false}}}"#).unwrap();
        assert!(!allowed(&off));
    }
}
