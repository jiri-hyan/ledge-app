//! Právě hraje (záložka Osobní): aktuální mediální session Windows přes GSMTC
//! (`GlobalSystemMediaTransportControlsSessionManager`, WinRT z crate `windows`) — název, interpret,
//! aplikace, hraje/pauza; ovládání přehrát/pauza, další, předchozí; malý obal na žádost UI.
//!
//! Žádný polling: vlákno `media` vznikne až při prvním zobrazení záložky Osobní (`ensure`, příkaz
//! `personal_live`) nebo hned, když je v proužku položka `media` (`sync` při startu a změně configu).
//! Drží odběr `CurrentSessionChanged` a u aktuální session `MediaPropertiesChanged` a
//! `PlaybackInfoChanged` (timeline záměrně NE — pozice se mění pořád = CPU) a jinak spí na kanálu.
//! Handler události jen pošle zprávu; vlastnosti čte vlákno (s termínem `WAIT`); změna → `mark_dirty`
//! → vlákno `emit` překreslí šuplík i proužek. Když data nepotřebuje karta (`drawer.sections.nowPlaying`,
//! záložka Osobní) ani proužek → `sync` vlákno ukončí a odběry zruší. `view` a `strip_now` čtou jen
//! paměť. Obal (`cover`) se čte a zmenšuje (WIC, ≤ 128 px PNG) až na žádost UI s novým `coverKey` —
//! jeden na skladbu, jen s otevřeným šuplíkem.

use crate::config::Config;
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty};
use crate::strip::{Rgba, Segment};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
};

/// Nárazové události (změna skladby = několik `MediaPropertiesChanged` za sebou) → krátký klid.
const DEBOUNCE: Duration = Duration::from_millis(150);
/// Termín na asynchronní dotaz do přehrávače (zaseklá aplikace nesmí zablokovat vlákno).
const WAIT: Duration = Duration::from_secs(3);
/// Větší obal se nečte (běžný je 300×300 JPEG, desítky kB).
const MAX_COVER_BYTES: u64 = 4 << 20;
/// Délka textu pro proužek (`strip_text`) ve znacích.
pub const STRIP_CHARS: usize = 36;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaView {
    pub title: String,
    pub artist: String,
    /// čitelný název zdrojové aplikace (`app_name` z AUMID); prázdné = neznámá
    pub app: String,
    /// playing | paused | stopped | changing
    pub status: &'static str,
    pub can_toggle: bool,
    pub can_prev: bool,
    pub can_next: bool,
    /// klíč obalu pro `media_cover`; `None` = session obal nemá
    pub cover_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Control {
    Toggle,
    Next,
    Prev,
}

enum Msg {
    Session,
    Changed,
    Control(Control),
    Cover(String, mpsc::SyncSender<Option<String>>),
    Stop,
}

struct Live {
    tx: Option<Sender<Msg>>,
    /// UI záložku Osobní už ukázalo — po zapnutí karty v Nastavení se vlákno spustí hned
    wanted: bool,
}

static LIVE: Mutex<Live> = Mutex::new(Live { tx: None, wanted: false });
static VIEW: Mutex<Option<MediaView>> = Mutex::new(None);
/// (klíč, data URL) posledního obalu
static COVER: Mutex<Option<(String, String)>> = Mutex::new(None);

// ---------- čistá logika ----------

/// Smí karta běžet? (záložka Osobní a karta zapnuté)
pub(crate) fn allowed(cfg: &Config) -> bool {
    cfg.modes.personal_enabled && cfg.drawer.sections.now_playing
}

/// Chce data proužek? (zapnutý proužek s položkou `media`; nezávisle na záložce Osobní)
pub(crate) fn strip_wants(cfg: &Config) -> bool {
    cfg.strip.enabled && cfg.strip.items.contains(&crate::config::StripItem::Media)
}

/// Má vlákno běžet? Karta (UI už Osobní ukázalo = `wanted`, karta smí běžet) NEBO položka proužku.
pub(crate) fn needed(cfg: &Config, wanted: bool) -> bool {
    (wanted && allowed(cfg)) || strip_wants(cfg)
}

/// `GlobalSystemMediaTransportControlsSessionPlaybackStatus` → kód pro UI; `Closed` (0) = nic.
pub(crate) fn status_of(code: i32) -> Option<&'static str> {
    match code {
        0 => None,
        2 => Some("changing"),
        4 => Some("playing"),
        5 => Some("paused"),
        _ => Some("stopped"), // 1 Opened, 3 Stopped, neznámé
    }
}

/// Známé přehrávače: podřetězec AUMID (malými písmeny) → název. Pořadí = priorita.
const KNOWN_APPS: &[(&str, &str)] = &[
    ("spotify", "Spotify"),
    ("msedge", "Edge"),
    ("microsoftedge", "Edge"),
    ("chrome", "Chrome"),
    ("firefox", "Firefox"),
    // Firefox se hlásí hashem instalace — tohle je výchozí složka `Program Files\Mozilla Firefox`
    ("308046b0af4a39cb", "Firefox"),
    ("brave", "Brave"),
    ("vivaldi", "Vivaldi"),
    ("opera", "Opera"),
    ("zunemusic", "Media Player"),
    ("zunevideo", "Movies & TV"),
    ("applemusic", "Apple Music"),
    ("itunes", "iTunes"),
    ("deezer", "Deezer"),
    ("tidal", "TIDAL"),
    ("discord", "Discord"),
    ("vlc", "VLC"),
    ("foobar2000", "foobar2000"),
    ("musicbee", "MusicBee"),
    ("aimp", "AIMP"),
    ("potplayer", "PotPlayer"),
    ("mpc-hc", "MPC-HC"),
    ("winamp", "Winamp"),
];

/// Zdrojový AUMID session (`SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify`, `Chrome`, `vlc.exe`,
/// cesta k exe…) → čitelný název. Neznámé: část za `!`, jméno souboru bez `.exe`, poslední tečková
/// část, první písmeno velké. Hash (samé hex číslice) nebo prázdno = `""` (UI aplikaci neukáže).
pub(crate) fn app_name(aumid: &str) -> String {
    let lower = aumid.to_ascii_lowercase();
    if let Some((_, name)) = KNOWN_APPS.iter().find(|(k, _)| lower.contains(k)) {
        return (*name).to_string();
    }
    let s = aumid.rsplit('!').next().unwrap_or("");
    let s = s.rsplit(['\\', '/']).next().unwrap_or("");
    let s = if s.len() > 4 && s[s.len() - 4..].eq_ignore_ascii_case(".exe") { &s[..s.len() - 4] } else { s };
    let s = s.rsplit('.').next().unwrap_or("").trim();
    if s.is_empty() || (s.len() >= 12 && s.chars().all(|c| c.is_ascii_hexdigit())) {
        return String::new();
    }
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Klíč obalu: změní se se skladbou a s tím, že přehrávač obal dodal (často až druhou událostí).
pub(crate) fn cover_key(aumid: &str, title: &str, artist: &str, album: &str, has_thumb: bool) -> Option<String> {
    use std::hash::{Hash, Hasher};
    if !has_thumb {
        return None;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (aumid, title, artist, album).hash(&mut h);
    Some(format!("{:016x}", h.finish()))
}

/// Krátký text pro proužek (segment „právě hraje“ přidá `stripview`): jen když hraje,
/// „Název · Interpret“ zkrácené na `max` znaků s „…“. Pauza / nic = `None`.
pub(crate) fn strip_text(v: Option<&MediaView>, max: usize) -> Option<String> {
    let v = v.filter(|v| v.status == "playing")?;
    let full = match (v.title.trim(), v.artist.trim()) {
        ("", "") => return None,
        (t, "") => t.to_string(),
        ("", a) => a.to_string(),
        (t, a) => format!("{t} · {a}"),
    };
    if full.chars().count() <= max {
        return Some(full);
    }
    let cut: String = full.chars().take(max.saturating_sub(1)).collect();
    Some(format!("{}…", cut.trim_end()))
}

/// Segment proužku „♪ Název · Interpret“ (`strip_text`, `STRIP_CHARS`); pauza / nic = žádný segment.
pub(crate) fn strip_segment(v: Option<&MediaView>) -> Option<Segment> {
    strip_text(v, STRIP_CHARS).map(|t| Segment { text: format!("♪ {t}"), color: Rgba::TEXT, dot: None, strong: false, pill: false })
}

// ---------- za běhu ----------

/// Pro snapshot — jen paměť.
pub(crate) fn view() -> Option<MediaView> {
    VIEW.lock().clone()
}

/// Segment proužku z paměti (viz `strip_segment`).
pub(crate) fn strip_now() -> Option<Segment> {
    strip_segment(VIEW.lock().as_ref())
}

/// UI ukázalo záložku Osobní: spustit vlákno, pokud je potřeba a ještě neběží.
pub(crate) fn ensure() {
    let ok = needed(&core().cfg.read(), true);
    let mut l = LIVE.lock();
    l.wanted = true;
    if ok && l.tx.is_none() {
        l.tx = spawn();
    }
}

/// Při startu a po změně configu: nikdo data nepotřebuje = konec vlákna i odběrů; karta (už chtěná)
/// nebo položka proužku = start.
pub(crate) fn sync(cfg: &Config) {
    let mut l = LIVE.lock();
    if !needed(cfg, l.wanted) {
        if let Some(tx) = l.tx.take() {
            let _ = tx.send(Msg::Stop);
            *VIEW.lock() = None;
            *COVER.lock() = None;
            mark_dirty();
        }
    } else if l.tx.is_none() {
        l.tx = spawn();
    }
}

fn send(m: Msg) -> Result<(), String> {
    LIVE.lock().tx.as_ref().and_then(|tx| tx.send(m).ok()).ok_or_else(|| tr("Ovládání médií neběží.", "Media control isn't running.").to_string())
}

/// Tlačítko v kartě (jen výslovný klik).
pub(crate) fn control(c: Control) -> Result<(), String> {
    send(Msg::Control(c))
}

/// Data URL obalu pro `key` z `MediaView.cover_key`; jiný klíč (skladba se mezitím změnila) = `None`.
pub(crate) fn cover(key: &str) -> Option<String> {
    if let Some((k, url)) = COVER.lock().as_ref()
        && k == key
    {
        return Some(url.clone());
    }
    let (tx, rx) = mpsc::sync_channel(1);
    send(Msg::Cover(key.to_string(), tx)).ok()?;
    rx.recv_timeout(WAIT + WAIT).ok().flatten()
}

fn spawn() -> Option<Sender<Msg>> {
    let (tx, rx) = mpsc::channel();
    let own = tx.clone();
    std::thread::Builder::new().name("media".into()).spawn(move || run(rx, own)).ok()?;
    Some(tx)
}

/// Čeká na asynchronní operaci WinRT nejvýš `WAIT` (výsledek musí být `Send`).
macro_rules! wait {
    ($op:expr) => {{
        let (tx, rx) = mpsc::sync_channel(1);
        match $op.when(move |r| {
            let _ = tx.send(r.ok());
        }) {
            Ok(()) => rx.recv_timeout(WAIT).ok().flatten(),
            Err(_) => None,
        }
    }};
}

/// Aktuální session a tokeny jejích odběrů.
struct Sub {
    session: Session,
    props: Option<i64>,
    playback: Option<i64>,
}

fn unsubscribe(sub: Option<Sub>) {
    if let Some(s) = sub {
        if let Some(t) = s.props {
            let _ = s.session.RemoveMediaPropertiesChanged(t);
        }
        if let Some(t) = s.playback {
            let _ = s.session.RemovePlaybackInfoChanged(t);
        }
    }
}

fn subscribe(mgr: &Manager, tx: &Sender<Msg>) -> Option<Sub> {
    let session = mgr.GetCurrentSession().ok()?;
    let (a, b) = (tx.clone(), tx.clone());
    let props = session
        .MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
            let _ = a.send(Msg::Changed);
            Ok(())
        }))
        .ok();
    let playback = session
        .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
            let _ = b.send(Msg::Changed);
            Ok(())
        }))
        .ok();
    Some(Sub { session, props, playback })
}

fn props(s: &Session) -> Option<Props> {
    wait!(s.TryGetMediaPropertiesAsync().ok()?)
}

fn read(s: &Session) -> Option<MediaView> {
    let info = s.GetPlaybackInfo().ok();
    let status = status_of(info.as_ref().and_then(|i| i.PlaybackStatus().ok()).map_or(0, |st| st.0))?;
    let ctl = info.as_ref().and_then(|i| i.Controls().ok());
    let aumid = s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default();
    let p = props(s);
    let text = |f: &dyn Fn(&Props) -> windows::core::Result<windows::core::HSTRING>| p.as_ref().and_then(|p| f(p).ok()).map(|h| h.to_string().trim().to_string()).unwrap_or_default();
    let title = text(&|p| p.Title());
    let artist = match text(&|p| p.Artist()) {
        a if a.is_empty() => text(&|p| p.AlbumArtist()),
        a => a,
    };
    let album = text(&|p| p.AlbumTitle());
    let has_thumb = p.as_ref().is_some_and(|p| p.Thumbnail().is_ok());
    // otevřená session bez titulku, která nehraje (zavřené video, prázdný přehrávač) = nic
    if title.is_empty() && artist.is_empty() && status != "playing" {
        return None;
    }
    let on = |f: fn(&windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackControls) -> windows::core::Result<bool>| {
        ctl.as_ref().and_then(|c| f(c).ok()).unwrap_or(false)
    };
    let can_toggle = on(|c| c.IsPlayPauseToggleEnabled()) || on(|c| c.IsPlayEnabled()) || on(|c| c.IsPauseEnabled());
    Some(MediaView {
        cover_key: cover_key(&aumid, &title, &artist, &album, has_thumb),
        app: app_name(&aumid),
        title,
        artist,
        status,
        can_toggle,
        can_prev: on(|c| c.IsPreviousEnabled()),
        can_next: on(|c| c.IsNextEnabled()),
    })
}

fn publish(sub: Option<&Sub>) {
    let v = sub.and_then(|s| read(&s.session));
    let mut g = VIEW.lock();
    if *g != v {
        *g = v;
        drop(g);
        mark_dirty();
    }
}

fn do_control(sub: Option<&Sub>, c: Control) {
    let Some(s) = sub else { return };
    // operace doběhne i po zahození; výsledek přijde událostí PlaybackInfoChanged
    let r = match c {
        Control::Toggle => s.session.TryTogglePlayPauseAsync(),
        Control::Next => s.session.TrySkipNextAsync(),
        Control::Prev => s.session.TrySkipPreviousAsync(),
    };
    if let Err(e) = r {
        log(&format!("media: ovládání {c:?} selhalo: {e}"));
    }
}

/// Obal aktuální skladby → PNG ≤ 128 px (WIC) jako data URL; klíč musí sedět na to, co UI vidí.
fn load_cover(sub: Option<&Sub>, key: &str) -> Option<String> {
    use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
    if VIEW.lock().as_ref().and_then(|v| v.cover_key.as_deref()) != Some(key) {
        return None;
    }
    let p = props(&sub?.session)?;
    let stream = p.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_COVER_BYTES {
        return None;
    }
    let buf = Buffer::Create(size as u32).ok()?;
    let filled = stream.ReadAsync(&buf, size as u32, InputStreamOptions::None).ok()?.join().ok()?;
    let reader = DataReader::FromBuffer(&filled).ok()?;
    let mut bytes = vec![0u8; reader.UnconsumedBufferLength().ok()? as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    let buf = bytes;
    let png = crate::iconimg::to_png(&buf)?;
    let url = format!("data:image/png;base64,{}", crate::icons::b64(&png));
    *COVER.lock() = Some((key.to_string(), url.clone()));
    Some(url)
}

fn run(rx: Receiver<Msg>, tx: Sender<Msg>) {
    let _com = crate::iconimg::Com::init();
    let mgr = match Manager::RequestAsync().and_then(|op| op.join()) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("media: GSMTC nedostupné: {e}"));
            return;
        }
    };
    let t = tx.clone();
    let session_tok = mgr
        .CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = t.send(Msg::Session);
            Ok(())
        }))
        .ok();
    let mut sub: Option<Sub> = None;
    let (mut new_session, mut changed) = (true, false);
    loop {
        if new_session {
            unsubscribe(sub.take());
            sub = subscribe(&mgr, &tx);
            changed = true;
        }
        if changed {
            publish(sub.as_ref());
        }
        (new_session, changed) = (false, false);
        let Ok(first) = rx.recv() else { break };
        let mut batch = vec![first];
        if matches!(batch[0], Msg::Session | Msg::Changed) {
            let until = Instant::now() + DEBOUNCE;
            while let Some(left) = until.checked_duration_since(Instant::now()) {
                match rx.recv_timeout(left) {
                    Ok(m) => batch.push(m),
                    Err(_) => break,
                }
            }
        }
        let mut stop = false;
        for m in batch {
            match m {
                Msg::Session => new_session = true,
                Msg::Changed => changed = true,
                Msg::Control(c) => do_control(sub.as_ref(), c),
                Msg::Cover(key, reply) => {
                    let _ = reply.send(load_cover(sub.as_ref(), &key));
                }
                Msg::Stop => stop = true,
            }
        }
        if stop {
            break;
        }
    }
    unsubscribe(sub);
    if let Some(t) = session_tok {
        let _ = mgr.RemoveCurrentSessionChanged(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_maps_aumid() {
        let cases = [
            ("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify", "Spotify"),
            ("Spotify.exe", "Spotify"),
            ("Chrome", "Chrome"),
            ("MSEdge", "Edge"),
            ("Microsoft.MicrosoftEdge.Stable_8wekyb3d8bbwe!App", "Edge"),
            ("308046B0AF4A39CB", "Firefox"),
            ("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic", "Media Player"),
            ("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe", "VLC"),
            ("Brave", "Brave"),
            // neznámé: část za `!`, poslední tečková část, velké první písmeno
            ("Contoso.Player_abc123!App.Mixer", "Mixer"),
            ("C:\\Tools\\tunebox.EXE", "Tunebox"),
            ("strawberry", "Strawberry"),
            ("ABCDEF0123456789", ""),
            ("", ""),
            ("!", ""),
        ];
        for (aumid, want) in cases {
            assert_eq!(app_name(aumid), want, "{aumid}");
        }
    }

    #[test]
    fn status_codes() {
        let cases = [(0, None), (1, Some("stopped")), (2, Some("changing")), (3, Some("stopped")), (4, Some("playing")), (5, Some("paused")), (9, Some("stopped"))];
        for (code, want) in cases {
            assert_eq!(status_of(code), want, "{code}");
        }
    }

    fn mv(title: &str, artist: &str, status: &'static str) -> MediaView {
        MediaView { title: title.into(), artist: artist.into(), app: "Spotify".into(), status, can_toggle: true, can_prev: true, can_next: true, cover_key: None }
    }

    #[test]
    fn strip_text_only_while_playing() {
        let long = "Velmi dlouhý název skladby, který se do proužku nevejde";
        let cases: [(Option<MediaView>, Option<&str>); 7] = [
            (None, None),
            (Some(mv("Song", "Band", "paused")), None),
            (Some(mv("Song", "Band", "playing")), Some("Song · Band")),
            (Some(mv("Song", "", "playing")), Some("Song")),
            (Some(mv("", "Band", "playing")), Some("Band")),
            (Some(mv(" ", " ", "playing")), None),
            (Some(mv(long, "X", "playing")), Some("Velmi dlouhý název skladby, který…")),
        ];
        for (v, want) in cases {
            let got = strip_text(v.as_ref(), 34);
            assert_eq!(got.as_deref(), want, "{v:?}");
            assert!(got.is_none_or(|g| g.chars().count() <= 34));
        }
    }

    #[test]
    fn strip_segment_table() {
        // (název, stav, očekávaný text segmentu)
        let cases: [(&str, Option<MediaView>, Option<&str>); 5] = [
            ("hraje", Some(mv("Song", "Band", "playing")), Some("♪ Song · Band")),
            ("pauza", Some(mv("Song", "Band", "paused")), None),
            ("zastaveno", Some(mv("Song", "Band", "stopped")), None),
            ("nic nehraje", None, None),
            ("hraje bez titulku i interpreta", Some(mv("", "", "playing")), None),
        ];
        for (name, v, want) in cases {
            let s = strip_segment(v.as_ref());
            assert_eq!(s.as_ref().map(|s| s.text.as_str()), want, "{name}");
            if let Some(s) = s {
                assert!(s.text.chars().count() <= STRIP_CHARS + 2, "{name}");
                assert_eq!((s.dot, s.pill), (None, false), "{name}");
            }
        }
    }

    #[test]
    fn needed_by_card_or_strip() {
        let cfg = |json: &str| -> Config { serde_json::from_str(json).unwrap() };
        let plain = cfg("{}");
        let strip = cfg(r#"{"strip":{"items":["clock","media"]}}"#);
        let strip_off = cfg(r#"{"strip":{"enabled":false,"items":["media"]}}"#);
        let strip_no_card = cfg(r#"{"strip":{"items":["media"]},"modes":{"personalEnabled":false}}"#);
        // (název, config, UI už Osobní ukázalo, má běžet)
        let cases: [(&str, &Config, bool, bool); 7] = [
            ("výchozí, Osobní neukázané: líně nic", &plain, false, false),
            ("výchozí, Osobní ukázané: karta", &plain, true, true),
            ("položka v proužku bez Osobní", &strip, false, true),
            ("vypnutý proužek s položkou: nic", &strip_off, false, false),
            ("vypnutý proužek, karta ukázaná", &strip_off, true, true),
            ("bez záložky Osobní, jen proužek", &strip_no_card, false, true),
            ("bez záložky Osobní, ukázaná: proužek", &strip_no_card, true, true),
        ];
        for (name, c, wanted, want) in cases {
            assert_eq!(needed(c, wanted), want, "{name}");
        }
    }

    #[test]
    fn cover_key_changes_with_track_and_thumbnail() {
        assert_eq!(cover_key("a", "t", "x", "al", false), None);
        let k = cover_key("a", "t", "x", "al", true).unwrap();
        assert_eq!(k.len(), 16);
        assert_eq!(cover_key("a", "t", "x", "al", true).as_deref(), Some(k.as_str()));
        assert_ne!(cover_key("a", "t2", "x", "al", true).as_deref(), Some(k.as_str()));
        assert_ne!(cover_key("b", "t", "x", "al", true).as_deref(), Some(k.as_str()));
    }

    #[test]
    fn allowed_follows_config_and_old_configs_default_on() {
        let old: Config = serde_json::from_str(r#"{"drawer":{"sections":{"projects":true,"running":true,"claude":true,"notes":true}}}"#).unwrap();
        assert!(allowed(&old));
        assert!(old.drawer.sections.sound && old.drawer.sections.battery);
        let off: Config = serde_json::from_str(r#"{"drawer":{"sections":{"nowPlaying":false,"sound":false,"battery":false}}}"#).unwrap();
        assert!(!allowed(&off));
        assert!(!off.drawer.sections.sound && !off.drawer.sections.battery);
        let no_personal: Config = serde_json::from_str(r#"{"modes":{"personalEnabled":false}}"#).unwrap();
        assert!(!allowed(&no_personal));
    }
}
