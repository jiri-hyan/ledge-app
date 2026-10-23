//! Config „Právě hraje“ (`media` v configu: vzhled karty, čas a ozdoba v proužku) a čistá logika
//! segmentu proužku (`media.rs` → `strip_now`).
//!
//! Vzhled `strip` („Jen v proužku“) nemá v šuplíku kartu, jen tenký ovladač, a skladbu ukáže proužek
//! i bez položky `media` v `strip.items` (`with_strip_item`) — jinak by nebyla vidět nikde. Čas v proužku
//! je po minutách („1/4 min“): proužek se přepočítá jen při změně minuty (`mediatime::next_minute`),
//! sekundy by ho budily každou sekundu i se zavřeným šuplíkem. Ozdoba ekvalizéru je jen statický znak
//! (skutečný zvuk neznázorňuje); rozpohybovaná je jen v šuplíku (CSS, jen když hraje a je vidět).
//!
//! **Oblíbené** (`media.favorites`): playlisty a stanice jako dlaždice v kartě Právě hraje. Ledge
//! nemá vlastní přehrávač — položka jen otevře odkaz (`spotify:…` nebo `http(s)://…`) v aplikaci, která
//! ho obsluhuje; ovládání pak jde přes GSMTC jako dosud. Validace je tvarová (schéma, host, délka),
//! disk ani síť se nezkoumají. UI má zrcadlo `favoriteSource` v `src/media/favorites.ts` (stejná tabulka
//! v testu obou stran).

use crate::config::StripItem;
use crate::configerr::{FieldError, fe};
use crate::media::MediaView;
use crate::strip::{Rgba, Segment};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Délka „Název · Interpret“ v proužku (znaky); čas se přidá za ni.
pub const STRIP_CHARS: usize = 36;
/// Ozdoba ekvalizéru místo „♪“ (jen vzhled `strip` se zapnutým `equalizer`).
pub const EQ_GLYPH: &str = "▂▅▃";

/// Vzhled karty Právě hraje (`media.variant`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Variant {
    /// Kompaktní řádek: malý obal, název, tenký průběh, tlačítka v jednom řádku.
    Compact,
    /// Karta s průběhem (výchozí): obal, název, interpret, lišta s časem, ovládání.
    #[default]
    Card,
    /// Velký obal: rozmazaný obal přes celou kartu.
    Cover,
    /// Vinyl: deska s obalem uprostřed, točí se jen při přehrávání.
    Vinyl,
    /// Jen v proužku: v šuplíku jen tenký ovladač, skladba v proužku (i bez položky `media`).
    Strip,
}

/// Čas skladby v proužku (`media.stripTime`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StripTime {
    /// Jen u vzhledu „Jen v proužku“ (výchozí).
    #[default]
    Auto,
    /// Vždy, když přehrávač hlásí průběh.
    On,
    /// Nikdy.
    Off,
}

/// Nejvíc oblíbených položek.
pub const MAX_FAVORITES: usize = 24;
/// Nejdelší název a odkaz oblíbené položky (znaky).
pub const MAX_FAVORITE_NAME: usize = 60;
pub const MAX_FAVORITE_URI: usize = 2048;

/// Oblíbená položka (`media.favorites[]`): název na dlaždici a odkaz, který se otevře.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    pub name: String,
    /// `spotify:playlist:…` / `spotify:album:…` / … nebo `https://…` (Spotify, YouTube Music, jiný web)
    pub uri: String,
}

/// Odkud oblíbená položka je (ikona dlaždice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Spotify,
    YtMusic,
    /// jakýkoli jiný http(s) odkaz
    Web,
}

/// Druhy objektů ve Spotify URI (`spotify:<druh>:<id>`).
const SPOTIFY_KINDS: [&str; 7] = ["playlist", "album", "artist", "track", "show", "episode", "user"];

/// Povolený odkaz oblíbené položky a jeho zdroj; `None` = nepovolený tvar (jiné schéma, mezera, jméno
/// v adrese, prázdný host…). Čistá funkce — rozhoduje validace i otevření.
pub(crate) fn source_of(uri: &str) -> Option<Source> {
    if uri.is_empty() || uri.chars().count() > MAX_FAVORITE_URI || uri.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let lower = uri.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("spotify:") {
        let mut parts = rest.split(':');
        let (kind, id) = (parts.next()?, parts.next()?);
        let ok = parts.next().is_none() && SPOTIFY_KINDS.contains(&kind) && !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric());
        return ok.then_some(Source::Spotify);
    }
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    // jméno@host v adrese je klasický trik na zaměnění cíle
    if authority.contains('@') {
        return None;
    }
    let host = match authority.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => h,
        Some(_) => return None,
        None => authority,
    };
    if host.is_empty() || host.starts_with('.') || host.starts_with('-') || !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return None;
    }
    Some(match host {
        "open.spotify.com" | "play.spotify.com" | "spotify.link" => Source::Spotify,
        "music.youtube.com" => Source::YtMusic,
        _ => Source::Web,
    })
}

/// Validace `media.favorites`: strop, neprázdný název, povolený odkaz. Chyba nese cestu k poli.
pub(crate) fn validate(m: &MediaConfig) -> Result<(), FieldError> {
    if m.favorites.len() > MAX_FAVORITES {
        return Err(fe("media.favorites", trf!("media.favorites: nejvýš {MAX_FAVORITES} položek (je jich {}).", "media.favorites: at most {MAX_FAVORITES} items (there are {}).", m.favorites.len())));
    }
    for (i, f) in m.favorites.iter().enumerate() {
        let n = i + 1;
        if f.name.trim().is_empty() {
            return Err(fe(format!("media.favorites.{i}.name"), trf!("Oblíbená položka č. {n} nemá název.", "Favorite no. {n} has no name.")));
        }
        if f.name.chars().count() > MAX_FAVORITE_NAME {
            return Err(fe(format!("media.favorites.{i}.name"), trf!("Oblíbená položka „{}“: název je delší než {MAX_FAVORITE_NAME} znaků.", "Favorite “{}”: the name is longer than {MAX_FAVORITE_NAME} characters.", f.name)));
        }
        if source_of(&f.uri).is_none() {
            return Err(fe(
                format!("media.favorites.{i}.uri"),
                trf!(
                    "Oblíbená položka „{}“: odkaz musí být spotify:playlist:… (album, artist, track, show, episode) nebo http(s)://… bez jména v adrese.",
                    "Favorite “{}”: the link must be spotify:playlist:… (album, artist, track, show, episode) or http(s)://… without a user name in the address.",
                    f.name
                ),
            ));
        }
    }
    Ok(())
}

/// Sekce `media` v configu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MediaConfig {
    /// Vzhled karty: `compact` | `card` (výchozí) | `cover` | `vinyl` | `strip` (jen v proužku).
    pub variant: Variant,
    /// Čas skladby v proužku po minutách („1/4 min“): `auto` (výchozí, jen u `strip`) | `on` | `off`.
    pub strip_time: StripTime,
    /// Ozdoba ekvalizéru u vzhledu `strip` (znak v proužku, animace v šuplíku). Jen dekorace — neukazuje
    /// skutečný zvuk. Výchozí true.
    pub equalizer: bool,
    /// Oblíbené playlisty a stanice (dlaždice v kartě Právě hraje, nejvýš 24). Klik otevře odkaz
    /// v Spotify / prohlížeči; Ledge nic nepřehrává sám. Výchozí prázdné (do souboru se nepíše).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub favorites: Vec<Favorite>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self { variant: Variant::Card, strip_time: StripTime::Auto, equalizer: true, favorites: Vec::new() }
    }
}

/// Ukazuje proužek čas skladby?
pub(crate) fn shows_time(m: &MediaConfig) -> bool {
    match m.strip_time {
        StripTime::Auto => m.variant == Variant::Strip,
        StripTime::On => true,
        StripTime::Off => false,
    }
}

/// Ozdoba ekvalizéru (jen vzhled „Jen v proužku“).
pub(crate) fn shows_eq(m: &MediaConfig) -> bool {
    m.variant == Variant::Strip && m.equalizer
}

/// Položky proužku: vzhled `strip` doplní `media` na konec, když tam není.
pub(crate) fn with_strip_item(mut items: Vec<StripItem>, m: &MediaConfig) -> Vec<StripItem> {
    if m.variant == Variant::Strip && !items.contains(&StripItem::Media) {
        items.push(StripItem::Media);
    }
    items
}

/// Chce položku `media` proužek? (výslovně v `strip.items`, nebo vzhled `strip`)
pub(crate) fn strip_has_item(items: &[StripItem], m: &MediaConfig) -> bool {
    items.contains(&StripItem::Media) || m.variant == Variant::Strip
}

/// Krátký text pro proužek: jen když hraje, „Název · Interpret“ zkrácené na `max` znaků s „…“.
/// Pauza / nic = `None`.
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

/// Segment proužku „♪ Název · Interpret[ · 1/4 min]“; pauza / nic = žádný segment. Čas jen s timeline
/// a `shows_time`, ozdoba `EQ_GLYPH` místo „♪“ podle `shows_eq`.
pub(crate) fn strip_segment(v: Option<&MediaView>, m: &MediaConfig, now_ms: i64) -> Option<Segment> {
    let text = strip_text(v, STRIP_CHARS)?;
    let time = v.and_then(|v| v.timeline.as_ref()).filter(|_| shows_time(m)).and_then(|t| crate::mediatime::strip_time(t, now_ms, true));
    let icon = if shows_eq(m) { EQ_GLYPH } else { "♪" };
    let text = match time {
        Some(t) => format!("{icon} {text} · {t}"),
        None => format!("{icon} {text}"),
    };
    Some(Segment { text, color: Rgba::TEXT, dot: None, strong: false, pill: false })
}

/// Budík vlákna `emit` pro čas v proužku: další změna minuty, jen když proužek čas ukazuje a hraje.
pub(crate) fn next_deadline(v: Option<&MediaView>, m: &MediaConfig, strip_on: bool, now_ms: i64) -> Option<i64> {
    let v = v.filter(|_| strip_on && shows_time(m))?;
    crate::mediatime::next_minute(v.timeline.as_ref()?, now_ms, v.status == "playing")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mediatime::Timeline;

    const NOW: i64 = 1_760_000_000_000;

    fn mv(title: &str, artist: &str, status: &'static str) -> MediaView {
        MediaView { title: title.into(), artist: artist.into(), app: "Spotify".into(), status, can_toggle: true, can_prev: true, can_next: true, cover_key: None, timeline: None, shuffle: None, repeat: None }
    }
    fn with_tl(mut v: MediaView) -> MediaView {
        v.timeline = Some(Timeline { position_ms: 102_000, duration_ms: 243_000, updated_ms: NOW, rate: 1.0, can_seek: true, start_ticks: 0 });
        v
    }
    fn cfg(variant: Variant, strip_time: StripTime, equalizer: bool) -> MediaConfig {
        MediaConfig { variant, strip_time, equalizer, favorites: Vec::new() }
    }

    #[test]
    fn serde_defaults_and_old_config() {
        // starý config bez `media` = karta s průběhem, čas v proužku jen u „Jen v proužku“, ozdoba zapnutá
        let c: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert_eq!(c.media, MediaConfig::default());
        assert_eq!((c.media.variant, c.media.strip_time, c.media.equalizer), (Variant::Card, StripTime::Auto, true));
        // skutečný tvar configu z vlny 5 (proužek s `media`, karta zapnutá): beze změny chování
        let old: crate::config::Config = serde_json::from_str(
            r#"{"strip":{"enabled":true,"items":["mode","clock","media"],"itemsRev":2},"drawer":{"sections":{"nowPlaying":true,"sound":true,"battery":true}},"modes":{"personalEnabled":true},"personal":{"links":[]}}"#,
        )
        .unwrap();
        assert_eq!(old.media, MediaConfig::default());
        assert_eq!(with_strip_item(old.strip.items.clone(), &old.media), old.strip.items);
        assert!(!shows_time(&old.media) && !shows_eq(&old.media));
        let m: MediaConfig = serde_json::from_str(r#"{"variant":"vinyl","stripTime":"on","equalizer":false}"#).unwrap();
        assert_eq!(m, cfg(Variant::Vinyl, StripTime::On, false));
        let v = serde_json::to_value(cfg(Variant::Strip, StripTime::Off, true)).unwrap();
        assert_eq!(v, serde_json::json!({ "variant": "strip", "stripTime": "off", "equalizer": true }));
        for s in ["compact", "card", "cover", "vinyl", "strip"] {
            assert!(serde_json::from_str::<Variant>(&format!("\"{s}\"")).is_ok(), "{s}");
        }
        assert!(serde_json::from_str::<Variant>("\"big\"").is_err());
        assert!(crate::config::schema_json().contains("\"stripTime\""));
    }

    #[test]
    fn time_eq_and_strip_item_table() {
        use StripItem::*;
        // (název, config, čas v proužku, ozdoba, položky z [Clock] / [Clock, Media])
        type Case<'a> = (&'a str, MediaConfig, bool, bool, [&'a [StripItem]; 2]);
        let cases: [Case; 6] = [
            ("výchozí karta", MediaConfig::default(), false, false, [&[Clock], &[Clock, Media]]),
            ("karta, čas vždy", cfg(Variant::Card, StripTime::On, true), true, false, [&[Clock], &[Clock, Media]]),
            ("jen v proužku", cfg(Variant::Strip, StripTime::Auto, true), true, true, [&[Clock, Media], &[Clock, Media]]),
            ("jen v proužku bez času a ozdoby", cfg(Variant::Strip, StripTime::Off, false), false, false, [&[Clock, Media], &[Clock, Media]]),
            ("vinyl auto = bez času", cfg(Variant::Vinyl, StripTime::Auto, true), false, false, [&[Clock], &[Clock, Media]]),
            ("kompaktní, čas vždy", cfg(Variant::Compact, StripTime::On, false), true, false, [&[Clock], &[Clock, Media]]),
        ];
        for (name, m, time, eq, [a, b]) in cases {
            assert_eq!((shows_time(&m), shows_eq(&m)), (time, eq), "{name}");
            assert_eq!(with_strip_item(vec![Clock], &m), a, "{name}");
            assert_eq!(with_strip_item(vec![Clock, Media], &m), b, "{name}");
            assert_eq!(strip_has_item(&[Clock], &m), a.contains(&Media), "{name}");
        }
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
        let card = MediaConfig::default();
        let strip = cfg(Variant::Strip, StripTime::Auto, true);
        let strip_plain = cfg(Variant::Strip, StripTime::Off, false);
        // (název, view, config, očekávaný text segmentu)
        let cases: [(&str, Option<MediaView>, &MediaConfig, Option<&str>); 9] = [
            ("hraje", Some(mv("Song", "Band", "playing")), &card, Some("♪ Song · Band")),
            ("pauza", Some(mv("Song", "Band", "paused")), &card, None),
            ("zastaveno", Some(mv("Song", "Band", "stopped")), &card, None),
            ("nic nehraje", None, &card, None),
            ("hraje bez titulku i interpreta", Some(mv("", "", "playing")), &card, None),
            ("karta: timeline bez času", Some(with_tl(mv("Song", "Band", "playing"))), &card, Some("♪ Song · Band")),
            ("jen v proužku: ozdoba a čas", Some(with_tl(mv("Song", "Band", "playing"))), &strip, Some("▂▅▃ Song · Band · 1/4 min")),
            ("jen v proužku bez timeline: bez času", Some(mv("Song", "Band", "playing")), &strip, Some("▂▅▃ Song · Band")),
            ("jen v proužku, vypnuto", Some(with_tl(mv("Song", "Band", "playing"))), &strip_plain, Some("♪ Song · Band")),
        ];
        for (name, v, m, want) in cases {
            let s = strip_segment(v.as_ref(), m, NOW);
            assert_eq!(s.as_ref().map(|s| s.text.as_str()), want, "{name}");
            if let Some(s) = s {
                assert!(s.text.chars().count() <= STRIP_CHARS + 16, "{name}");
                assert_eq!((s.dot, s.pill), (None, false), "{name}");
            }
        }
    }

    #[test]
    fn next_deadline_only_when_time_shows_and_plays() {
        let strip = cfg(Variant::Strip, StripTime::Auto, true);
        let playing = with_tl(mv("Song", "Band", "playing"));
        assert_eq!(next_deadline(Some(&playing), &strip, true, NOW), Some(NOW + 18_001));
        assert_eq!(next_deadline(Some(&playing), &strip, false, NOW), None, "vypnutý proužek");
        assert_eq!(next_deadline(Some(&playing), &MediaConfig::default(), true, NOW), None, "čas se neukazuje");
        assert_eq!(next_deadline(Some(&with_tl(mv("Song", "Band", "paused"))), &strip, true, NOW), None, "pauza");
        assert_eq!(next_deadline(Some(&mv("Song", "Band", "playing")), &strip, true, NOW), None, "bez timeline");
        assert_eq!(next_deadline(None, &strip, true, NOW), None);
    }

    /// Tabulka sdílená s `src/media/favorites.test.mjs` (zrcadlo `favoriteSource`): stejné řádky, stejné výsledky.
    pub(crate) const SOURCE_TABLE: [(&str, Option<Source>); 24] = [
        ("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M", Some(Source::Spotify)),
        ("spotify:album:4aawyAB9vmqN3uQ7FjRGTy", Some(Source::Spotify)),
        ("SPOTIFY:track:abc123", Some(Source::Spotify)),
        ("spotify:playlist:", None),
        ("spotify:playlist", None),
        ("spotify:playlist:abc:play", None),
        ("spotify:radio:abc", None),
        ("spotify:playlist:a-b", None),
        ("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=x", Some(Source::Spotify)),
        ("https://music.youtube.com/playlist?list=PLabc", Some(Source::YtMusic)),
        ("https://music.youtube.com:443/watch?v=1", Some(Source::YtMusic)),
        ("https://www.youtube.com/watch?v=1", Some(Source::Web)),
        ("http://radia.cz/stream", Some(Source::Web)),
        ("https://user:pw@music.youtube.com/", None),
        ("https://music.youtube.com@evil.example/", None),
        ("https://", None),
        ("https:///x", None),
        ("ftp://example.com/a", None),
        ("file:///C:/Windows/notepad.exe", None),
        ("javascript:alert(1)", None),
        (r"C:\Windows\notepad.exe", None),
        ("https://example.com/a b", None),
        ("", None),
        ("https://exa_mple.com", None),
    ];

    #[test]
    fn favorite_source_table() {
        for (uri, want) in SOURCE_TABLE {
            assert_eq!(source_of(uri), want, "{uri}");
        }
        let long = format!("https://example.com/{}", "a".repeat(MAX_FAVORITE_URI));
        assert_eq!(source_of(&long), None, "příliš dlouhý odkaz");
    }

    fn fav(name: &str, uri: &str) -> Favorite {
        Favorite { name: name.into(), uri: uri.into() }
    }

    #[test]
    fn favorites_validation_table() {
        let with = |f: Vec<Favorite>| MediaConfig { favorites: f, ..MediaConfig::default() };
        let many: Vec<Favorite> = (0..=MAX_FAVORITES).map(|i| fav(&format!("n{i}"), "spotify:playlist:abc")).collect();
        let full: Vec<Favorite> = (0..MAX_FAVORITES).map(|i| fav(&format!("n{i}"), "spotify:playlist:abc")).collect();
        // (název, config, cesta chyby nebo None = platí)
        let cases: Vec<(&str, MediaConfig, Option<&str>)> = vec![
            ("bez oblíbených", MediaConfig::default(), None),
            ("playlist a stanice", with(vec![fav("Ráno", "spotify:playlist:abc"), fav("Rádio", "https://music.youtube.com/watch?v=1")]), None),
            ("na stropu", with(full), None),
            ("přes strop", with(many), Some("media.favorites")),
            ("prázdný název", with(vec![fav("  ", "spotify:playlist:abc")]), Some("media.favorites.0.name")),
            ("dlouhý název", with(vec![fav(&"x".repeat(MAX_FAVORITE_NAME + 1), "spotify:playlist:abc")]), Some("media.favorites.0.name")),
            ("cizí schéma", with(vec![fav("ok", "spotify:playlist:abc"), fav("zlé", "file:///C:/x.exe")]), Some("media.favorites.1.uri")),
        ];
        for (name, m, want) in cases {
            let got = validate(&m).err();
            assert_eq!(got.as_ref().and_then(|e| e.field.as_deref()), want, "{name}");
        }
    }

    #[test]
    fn favorites_serde_old_config_and_roundtrip() {
        // starý config bez `favorites` = prázdné, do souboru se nepíše
        let m: MediaConfig = serde_json::from_str(r#"{"variant":"vinyl"}"#).unwrap();
        assert!(m.favorites.is_empty());
        assert!(serde_json::to_value(&m).unwrap().get("favorites").is_none());
        let m: MediaConfig = serde_json::from_str(r#"{"favorites":[{"name":"Ráno","uri":"spotify:playlist:abc"}]}"#).unwrap();
        assert_eq!(m.favorites, vec![fav("Ráno", "spotify:playlist:abc")]);
        let back: MediaConfig = serde_json::from_value(serde_json::to_value(&m).unwrap()).unwrap();
        assert_eq!(back, m);
        assert!(crate::config::schema_json().contains("\"favorites\""));
        // chyba z validace configu nese cestu k poli
        let mut c = crate::config::Config::default();
        c.media.favorites = vec![fav("zlé", "javascript:alert(1)")];
        let e = crate::config::validate_field(&c).unwrap_err();
        assert_eq!(e.field.as_deref(), Some("media.favorites.0.uri"));
    }
}
