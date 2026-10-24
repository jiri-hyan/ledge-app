//! Hlasitost (záložka Osobní): výchozí výstupní zařízení přes Core Audio (`IMMDeviceEnumerator` →
//! `IAudioEndpointVolume`) — hlasitost 0–100, ztlumení, název zařízení.
//!
//! Žádný polling: vlákno `volume` (COM MTA) vznikne až při prvním zobrazení záložky Osobní
//! (`ensure`), zaregistruje `IAudioEndpointVolumeCallback` (změna hlasitosti zvenku — klávesy,
//! mixér) a `IMMNotificationClient` (jiné výchozí zařízení → přepojit) a jinak spí na kanálu. Callbacky
//! jen přepíšou paměť / pošlou zprávu. Posuvník: UI posílá při tahu nejvýš ~20×/s (`src/media/model.ts`
//! `throttle`), vlákno z nahromaděných zpráv použije jen poslední hodnotu. Vypnutá karta
//! (`drawer.sections.sound`) nebo záložka Osobní → `sync` odběry zruší a vlákno skončí.
//!
//! Kolo 18: seznam aktivních výstupů (`devices`, přepnutí výchozího přes
//! nedokumentované `IPolicyConfig` — totéž, co používá např. EarTrumpet; při selhání otevře Nastavení zvuku
//! Windows) a ztlumení výchozího mikrofonu (`mic_muted`, druhý `IAudioEndpointVolumeCallback` na vstupu —
//! stav se drží událostmi, ne dotazováním).

// metody COM rozhraní `IPolicyConfig` musí mít jména z vtable (SetDefaultEndpoint…)
#![allow(non_snake_case)]

use crate::config::Config;
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::mpsc::{self, Receiver, Sender};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, eCapture, eConsole, eMultimedia, eRender,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::{GUID, PCWSTR, implement};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeView {
    /// 0–100
    pub level: u8,
    pub muted: bool,
    /// název výchozího výstupu („Reproduktory (Realtek…)“); prázdné = nevíme
    pub device: String,
    /// výchozí mikrofon ztlumený; `None` = žádný mikrofon
    pub mic_muted: Option<bool>,
    /// aktivní výstupy (přepínač výstupu); `active` = výchozí
    pub devices: Vec<AudioDevice>,
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

const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

#[windows::core::interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: windows::core::IUnknown {
    unsafe fn GetMixFormat(&self) -> windows::core::HRESULT;
    unsafe fn GetDeviceFormat(&self) -> windows::core::HRESULT;
    unsafe fn ResetDeviceFormat(&self) -> windows::core::HRESULT;
    unsafe fn SetDeviceFormat(&self) -> windows::core::HRESULT;
    unsafe fn GetProcessingPeriod(&self) -> windows::core::HRESULT;
    unsafe fn SetProcessingPeriod(&self) -> windows::core::HRESULT;
    unsafe fn GetShareMode(&self) -> windows::core::HRESULT;
    unsafe fn SetShareMode(&self) -> windows::core::HRESULT;
    unsafe fn GetPropertyValue(&self) -> windows::core::HRESULT;
    unsafe fn SetPropertyValue(&self) -> windows::core::HRESULT;
    unsafe fn SetDefaultEndpoint(&self, wsz_device_id: PCWSTR, e_role: ERole) -> windows::core::HRESULT;
    unsafe fn SetEndpointVisibility(&self) -> windows::core::HRESULT;
}

// ---------- čistá logika ----------

pub(crate) fn allowed(cfg: &Config) -> bool {
    // hlasitost a mikrofon ukazuje i karta Rychlé přepínače (kolo 22, `src/board/TogglesCard.tsx`)
    cfg.modes.personal_enabled && (cfg.drawer.sections.sound || crate::layout::placed(&cfg.layout, crate::layout::TOGGLES_CARD))
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

/// Ovládání zvuku jen se zapnutou kartou Zvuk na záložce Osobní (`allowed`) — jinak ani mikrofon, ani výstup.
fn gate() -> Result<(), String> {
    if allowed(&core().cfg.read()) { Ok(()) } else { Err(tr("Karta Zvuk je vypnutá (Nastavení → Osobní).", "The Sound card is turned off (Settings → Personal).").into()) }
}

/// Výchozí mikrofon (vstup pro komunikaci, jinak pro multimédia).
fn default_mic(en: &IMMDeviceEnumerator) -> windows::core::Result<IMMDevice> {
    unsafe { en.GetDefaultAudioEndpoint(eCapture, eConsole).or_else(|_| en.GetDefaultAudioEndpoint(eCapture, eMultimedia)) }
}

/// Ztlumit / zapnout výchozí mikrofon (klik v kartě). Vrací nový stav; karta ho dostane i událostí (`MicCb`).
pub fn mic_mute_toggle() -> Result<bool, String> {
    gate()?;
    let _com = crate::iconimg::Com::init();
    let ctx = tr("Mikrofon nejde ztlumit", "Can't mute the microphone");
    let muted = unsafe {
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| crate::errors::any(ctx, &e))?;
        let dev = default_mic(&en).map_err(|_| tr("Žádný mikrofon není připojený.", "No microphone is connected.").to_string())?;
        let ep: IAudioEndpointVolume = dev.Activate(CLSCTX_ALL, None).map_err(|e| crate::errors::any(ctx, &e))?;
        let muted = !ep.GetMute().map_err(|e| crate::errors::any(ctx, &e))?.as_bool();
        ep.SetMute(muted, std::ptr::null()).map_err(|e| crate::errors::any(ctx, &e))?;
        muted
    };
    set_mic(Some(muted));
    Ok(muted)
}

/// Smí se přepnout na `id`? Jen zařízení, které jádro samo nabídlo (`VolumeView.devices`).
pub(crate) fn known_device(view: Option<&VolumeView>, id: &str) -> bool {
    !id.is_empty() && view.is_some_and(|v| v.devices.iter().any(|d| d.id == id))
}

/// Přepnout výchozí výstup (klik ve výběru výstupu). Selže-li `IPolicyConfig`, otevře Nastavení zvuku Windows.
pub fn set_default_output(device_id: &str) -> Result<(), String> {
    gate()?;
    if !known_device(VIEW.lock().as_ref(), device_id) {
        return Err(tr("Tohle zařízení už není připojené.", "This device is no longer connected.").into());
    }
    let _com = crate::iconimg::Com::init();
    let wide: Vec<u16> = device_id.encode_utf16().chain([0]).collect();
    let pcw = PCWSTR(wide.as_ptr());
    let res: windows::core::Result<()> = (|| unsafe {
        let policy: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)?;
        policy.SetDefaultEndpoint(pcw, eConsole).ok()?;
        policy.SetDefaultEndpoint(pcw, eMultimedia).ok()?;
        Ok(())
    })();
    if let Err(e) = res {
        log(&format!("volume: přepnutí výstupu selhalo ({e}), otevírám Nastavení zvuku"));
        let _ = tauri_plugin_opener::OpenerExt::opener(&core().app).open_url("ms-settings:sound", None::<&str>);
        return Err(tr("Výstup se nepřepnul — otevřel jsem Nastavení zvuku Windows.", "The output didn't switch — Windows sound settings are open.").into());
    }
    let _ = send(Msg::Rebind);
    Ok(())
}

fn set_mic(muted: Option<bool>) {
    let mut g = VIEW.lock();
    if let Some(v) = g.as_mut()
        && v.mic_muted != muted
    {
        v.mic_muted = muted;
        drop(g);
        mark_dirty();
    }
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

/// Ztlumení mikrofonu zvenku (klávesa na sluchátkách, Teams, mixér) — jen zápis do paměti.
#[implement(IAudioEndpointVolumeCallback)]
struct MicCb;

impl IAudioEndpointVolumeCallback_Impl for MicCb_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        if let Some(d) = unsafe { pnotify.as_ref() } {
            set_mic(Some(d.bMuted.as_bool()));
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
        let _ = self.tx.send(Msg::Rebind);
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.tx.send(Msg::Rebind);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.tx.send(Msg::Rebind);
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> windows::core::Result<()> {
        if (flow == eRender || flow == eCapture) && (role == eMultimedia || role == eConsole) {
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

fn list_output_devices(en: &IMMDeviceEnumerator, default_id: &str) -> Vec<AudioDevice> {
    let mut out = Vec::new();
    unsafe {
        let Ok(col) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
            return out;
        };
        let Ok(count) = col.GetCount() else {
            return out;
        };
        for i in 0..count {
            let Ok(dev) = col.Item(i) else { continue };
            let id = match dev.GetId() {
                Ok(p) => {
                    let s = p.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(p.0 as _));
                    s
                }
                Err(_) => continue,
            };
            let name = device_name(&dev);
            let active = !id.is_empty() && id == default_id;
            out.push(AudioDevice { id, name, active });
        }
    }
    out
}

/// Výchozí mikrofon → odběr změn ztlumení + první stav (`None` = žádný mikrofon).
fn bind_mic(en: &IMMDeviceEnumerator, cb: &IAudioEndpointVolumeCallback) -> (Option<IAudioEndpointVolume>, Option<bool>) {
    unsafe {
        let Ok(dev) = default_mic(en) else { return (None, None) };
        let Ok(ep) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else { return (None, None) };
        let muted = ep.GetMute().ok().map(|b| b.as_bool());
        if ep.RegisterControlChangeNotify(cb).is_err() {
            return (None, muted);
        }
        (Some(ep), muted)
    }
}

/// Výchozí výstup → odběr změn + první stav. Bez zařízení `None` (karta ukáže, že není kam hrát).
/// `mic_muted` = stav mikrofonu z `bind_mic` (do stejného pohledu).
fn bind(en: &IMMDeviceEnumerator, cb: &IAudioEndpointVolumeCallback, mic_muted: Option<bool>) -> Option<IAudioEndpointVolume> {
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
        let default_id = match dev.GetId() {
            Ok(p) => {
                let s = p.to_string().unwrap_or_default();
                CoTaskMemFree(Some(p.0 as _));
                s
            }
            Err(_) => String::new(),
        };
        let devices = list_output_devices(en, &default_id);
        set_view(Some(VolumeView {
            level,
            muted,
            device: device_name(&dev),
            mic_muted,
            devices,
        }));
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
    let mic_cb: IAudioEndpointVolumeCallback = MicCb.into();
    let (mut mic, mic_muted) = bind_mic(&en, &mic_cb);
    let mut ep = bind(&en, &cb, mic_muted);
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
            unbind(mic.take(), &mic_cb);
            let (m, mic_muted) = bind_mic(&en, &mic_cb);
            mic = m;
            ep = bind(&en, &cb, mic_muted);
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
    unbind(mic, &mic_cb);
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
    fn switch_only_to_offered_device() {
        let dev = |id: &str| AudioDevice { id: id.into(), name: format!("Zařízení {id}"), active: false };
        let v = VolumeView { level: 10, muted: false, device: String::new(), mic_muted: None, devices: vec![dev("{0.0.0.00000000}.{a}"), dev("{0.0.0.00000000}.{b}")] };
        assert!(known_device(Some(&v), "{0.0.0.00000000}.{b}"));
        assert!(!known_device(Some(&v), "{0.0.0.00000000}.{c}"), "odpojené / cizí id");
        assert!(!known_device(Some(&v), ""));
        assert!(!known_device(None, "{0.0.0.00000000}.{a}"), "vlákno neběží");
    }

    #[test]
    fn allowed_by_config() {
        let on = Config::default();
        assert!(allowed(&on));
        let off: Config = serde_json::from_str(r#"{"drawer":{"sections":{"sound":false}}}"#).unwrap();
        assert!(!allowed(&off));
        // bez karty Právě hraje, ale s kartou Rychlé přepínače = zvuk se čte
        let mut toggles = off.clone();
        toggles.layout.pages[0].cards.push(serde_json::from_str(r#"{"id":"toggles","size":"s"}"#).unwrap());
        assert!(allowed(&toggles));
    }
}
