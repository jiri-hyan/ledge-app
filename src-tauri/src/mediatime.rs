//! Průběh skladby a režimy přehrávání pro „Právě hraje“ (`media.rs`): timeline GSMTC (pozice, délka,
//! `LastUpdatedTime`), náhodně / opakovat a přetáčení.
//!
//! Windows hlásí timeline jen při změně (`TimelinePropertiesChanged` — přehrávač ji posílá při
//! přehrání, pauze, přetočení, některé i průběžně). Ledge si nikdy nic neptá v intervalu: drží poslední
//! hlášenou pozici s časem hlášení a průběh dopočítává UI (`src/media/timeline.ts`) jen s otevřeným
//! šuplíkem. Přehrávač, který timeline posílá každou sekundu, by jinak budil šuplík i proužek — proto
//! `merge` hlášení, které jen potvrzuje dopočítaný průběh (`same_course`), zahodí (žádná změna = žádné
//! `mark_dirty`). Zdroj bez timeline (délka 0, některé prohlížeče) = `None`: lišta se neukáže, nic se
//! nevymýšlí.

use serde::{Deserialize, Serialize};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionPlaybackInfo as Info,
};
use windows::Media::MediaPlaybackAutoRepeatMode as RepeatMode;

/// 100ns ticky WinRT → ms.
const TICKS_PER_MS: i64 = 10_000;
/// `DateTime.UniversalTime` (100ns od 1601-01-01) → unixové ms: posun epoch v ms.
const EPOCH_1601_MS: i64 = 11_644_473_600_000;
/// Kratší „skladba“ není skladba (živý stream, reklama bez délky) — lištu neukazovat.
const MIN_DURATION_MS: i64 = 1_000;
/// Nové hlášení, které od dopočítané pozice ujede o víc, je skok (přetočení, zaseknutí).
const DRIFT_MS: i64 = 1_500;
/// Hlášení starší než tohle při změně hraje/pauza = přehrávač timeline neobnovil.
const STALE_MS: i64 = 2_000;
/// Rozumné rychlosti přehrávání; mimo = 1× (přehrávač hlásí nesmysl nebo 0 při pauze).
const RATE_RANGE: std::ops::RangeInclusive<f64> = 0.1..=4.0;

/// Poslední hlášená pozice skladby (vše v ms; `updated_ms` = unixový čas hlášení).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    /// pozice v čase `updated_ms` (0 ≤ pozice ≤ délka)
    pub position_ms: i64,
    pub duration_ms: i64,
    /// kdy pozice platila (unixové ms); UI od něj dopočítává, jen když hraje
    pub updated_ms: i64,
    /// rychlost přehrávání (1 = normální)
    pub rate: f64,
    /// přehrávač dovolí přetáčet (`IsPlaybackPositionEnabled` a rozsah přetáčení)
    pub can_seek: bool,
    /// `StartTime` v ticích — přetáčení posílá absolutní pozici (UI ho nepotřebuje)
    #[serde(skip)]
    pub start_ticks: i64,
}

/// Opakování (`AutoRepeatMode`) — `none` | `track` | `list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Repeat {
    None,
    Track,
    List,
}

/// Náhodně / opakovat aktuální session; `None` = zdroj to nepodporuje (UI tlačítko neukáže).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Modes {
    pub shuffle: Option<bool>,
    pub repeat: Option<Repeat>,
}

// ---------- čistá logika ----------

/// `DateTime.UniversalTime` → unixové ms.
pub(crate) fn universal_to_unix_ms(ut: i64) -> i64 {
    ut / TICKS_PER_MS - EPOCH_1601_MS
}

/// Surová timeline (ticky) → `Timeline`. Délka < 1 s = zdroj timeline nedává → `None`. Neplatný čas
/// hlášení (0, z budoucnosti) = teď. Rychlost mimo rozumný rozsah = 1×.
#[allow(clippy::too_many_arguments)]
pub(crate) fn from_ticks(start: i64, end: i64, pos: i64, updated_ut: i64, min_seek: i64, max_seek: i64, seek_enabled: bool, rate: Option<f64>, now_ms: i64) -> Option<Timeline> {
    let duration_ms = (end - start) / TICKS_PER_MS;
    if duration_ms < MIN_DURATION_MS {
        return None;
    }
    let updated = universal_to_unix_ms(updated_ut);
    let updated_ms = if updated_ut <= 0 || updated > now_ms + 5_000 { now_ms } else { updated };
    Some(Timeline {
        position_ms: ((pos - start) / TICKS_PER_MS).clamp(0, duration_ms),
        duration_ms,
        updated_ms,
        rate: rate.filter(|r| r.is_finite() && RATE_RANGE.contains(r)).unwrap_or(1.0),
        can_seek: seek_enabled && max_seek > min_seek,
        start_ticks: start,
    })
}

/// Pozice v čase `now`: hraje = poslední pozice + uplynulý čas × rychlost, jinak stojí; ořez na délku.
/// (Zrcadlo `positionAt` v `src/media/timeline.ts`.)
pub(crate) fn position_at(t: &Timeline, now_ms: i64, playing: bool) -> i64 {
    let run = if playing { ((now_ms - t.updated_ms).max(0) as f64 * t.rate) as i64 } else { 0 };
    (t.position_ms + run).clamp(0, t.duration_ms)
}

/// Nové hlášení jen potvrzuje dosavadní průběh (stejná délka, rychlost, přetáčení a pozice do `DRIFT_MS`).
pub(crate) fn same_course(old: &Timeline, new: &Timeline, playing: bool) -> bool {
    (old.duration_ms - new.duration_ms).abs() <= 1_000
        && old.rate == new.rate
        && old.can_seek == new.can_seek
        && (position_at(old, new.updated_ms, playing) - new.position_ms).abs() <= DRIFT_MS
}

/// Sloučí nové hlášení s předchozím stavem téže skladby (`prev` = timeline a zda hrála).
/// - stav beze změny a hlášení jen potvrzuje průběh → staré (snapshot se nezmění, nic se nebudí);
/// - hrála → pauza a přehrávač timeline neobnovil (staré hlášení) → zmrazit na dopočítané pozici teď;
/// - pauza → hraje se starým hlášením → pozice platí od teď (jinak by lišta skočila o délku pauzy).
pub(crate) fn merge(prev: Option<(&Timeline, bool)>, new: Option<Timeline>, playing: bool, now_ms: i64) -> Option<Timeline> {
    let mut new = new?;
    let Some((old, was_playing)) = prev else { return Some(new) };
    let stale = new.updated_ms + STALE_MS < now_ms;
    if was_playing == playing {
        return Some(if same_course(old, &new, playing) { *old } else { new });
    }
    if stale {
        if was_playing {
            new.position_ms = position_at(&new, now_ms, true);
        }
        new.updated_ms = now_ms;
    }
    Some(new)
}

/// `AutoRepeatMode` → `Repeat`; neznámá hodnota = `None` (tlačítko se neukáže).
pub(crate) fn repeat_of(code: i32) -> Option<Repeat> {
    match code {
        0 => Some(Repeat::None),
        1 => Some(Repeat::Track),
        2 => Some(Repeat::List),
        _ => None,
    }
}

/// Cílová pozice přetočení (ms od začátku skladby) → absolutní ticky pro `TryChangePlaybackPositionAsync`;
/// ořez na délku. Bez přetáčení = `None`.
pub(crate) fn seek_ticks(t: &Timeline, ms: i64) -> Option<i64> {
    t.can_seek.then(|| t.start_ticks + ms.clamp(0, t.duration_ms) * TICKS_PER_MS)
}

/// Kdy se v proužku změní celá minuta uplynulého času (`None` = nehraje / bez timeline). Pro budík
/// vlákna `emit` — jinak se proužek přepočítá jen na začátku minuty hodin.
pub(crate) fn next_minute(t: &Timeline, now_ms: i64, playing: bool) -> Option<i64> {
    if !playing {
        return None;
    }
    let pos = position_at(t, now_ms, true);
    if pos >= t.duration_ms {
        return None;
    }
    let left = 60_000 - pos % 60_000;
    Some(now_ms + (left as f64 / t.rate).ceil() as i64 + 1)
}

/// Čas skladby do proužku po minutách („1/4 min“): uplynulé celé minuty / délka. Skladba kratší než
/// minuta = nic (proužek se nepřepočítává po sekundách — idle CPU).
pub(crate) fn strip_time(t: &Timeline, now_ms: i64, playing: bool) -> Option<String> {
    if t.duration_ms < 60_000 {
        return None;
    }
    Some(format!("{}/{} min", position_at(t, now_ms, playing) / 60_000, t.duration_ms / 60_000))
}

// ---------- WinRT (vlákno `media`) ----------

/// Timeline aktuální session (synchronní čtení vlastností, žádné čekání na přehrávač).
pub(crate) fn read(s: &Session, info: Option<&Info>, now_ms: i64) -> Option<Timeline> {
    let tl = s.GetTimelineProperties().ok()?;
    let span = |r: windows::core::Result<windows::Foundation::TimeSpan>| r.map(|t| t.Duration).unwrap_or(0);
    let seek_enabled = info.and_then(|i| i.Controls().ok()).and_then(|c| c.IsPlaybackPositionEnabled().ok()).unwrap_or(false);
    let rate = info.and_then(|i| i.PlaybackRate().ok()).and_then(|r| r.Value().ok());
    from_ticks(
        span(tl.StartTime()),
        span(tl.EndTime()),
        span(tl.Position()),
        tl.LastUpdatedTime().map(|d| d.UniversalTime).unwrap_or(0),
        span(tl.MinSeekTime()),
        span(tl.MaxSeekTime()),
        seek_enabled,
        rate,
        now_ms,
    )
}

/// Náhodně / opakovat — jen když je přehrávač povolí (`IsShuffleEnabled` / `IsRepeatEnabled`) a hlásí stav.
pub(crate) fn modes(info: Option<&Info>) -> Modes {
    let Some(info) = info else { return Modes::default() };
    let ctl = info.Controls().ok();
    let on = |f: fn(&windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackControls) -> windows::core::Result<bool>| ctl.as_ref().and_then(|c| f(c).ok()).unwrap_or(false);
    Modes {
        shuffle: if on(|c| c.IsShuffleEnabled()) { info.IsShuffleActive().ok().and_then(|r| r.Value().ok()) } else { None },
        repeat: if on(|c| c.IsRepeatEnabled()) { info.AutoRepeatMode().ok().and_then(|r| r.Value().ok()).and_then(|m| repeat_of(m.0)) } else { None },
    }
}

/// Přetočit (výsledek přijde událostí timeline); bez přetáčení chyba pro UI.
pub(crate) fn seek(s: &Session, t: Option<&Timeline>, ms: i64) -> windows::core::Result<()> {
    match t.and_then(|t| seek_ticks(t, ms)) {
        Some(ticks) => s.TryChangePlaybackPositionAsync(ticks).map(|_| ()),
        None => Err(windows::core::Error::empty()),
    }
}

pub(crate) fn set_shuffle(s: &Session, on: bool) -> windows::core::Result<()> {
    s.TryChangeShuffleActiveAsync(on).map(|_| ())
}

pub(crate) fn set_repeat(s: &Session, r: Repeat) -> windows::core::Result<()> {
    let m = match r {
        Repeat::None => RepeatMode::None,
        Repeat::Track => RepeatMode::Track,
        Repeat::List => RepeatMode::List,
    };
    s.TryChangeAutoRepeatModeAsync(m).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_760_000_000_000;
    const T: i64 = TICKS_PER_MS;

    fn tl(pos: i64, dur: i64, updated: i64) -> Timeline {
        Timeline { position_ms: pos, duration_ms: dur, updated_ms: updated, rate: 1.0, can_seek: true, start_ticks: 0 }
    }

    #[test]
    fn universal_time_to_unix() {
        // 1970-01-01 = 116444736000000000 ticků od 1601
        assert_eq!(universal_to_unix_ms(116_444_736_000_000_000), 0);
        assert_eq!(universal_to_unix_ms(116_444_736_000_000_000 + 1_234 * T), 1_234);
    }

    #[test]
    fn from_ticks_table() {
        let ut = |ms: i64| (ms + EPOCH_1601_MS) * T;
        // (název, start, konec, pozice, hlášeno, min, max, přetáčení, rychlost, očekávané (pozice, délka, hlášeno, rychlost, přetáčení))
        type Want = Option<(i64, i64, i64, f64, bool)>;
        type Case = (&'static str, [i64; 6], bool, Option<f64>, Want);
        let cases: [Case; 9] = [
            ("běžná skladba", [0, 243_000 * T, 102_000 * T, ut(NOW - 500), 0, 243_000 * T], true, Some(1.0), Some((102_000, 243_000, NOW - 500, 1.0, true))),
            ("bez délky (prohlížeč) = nic", [0, 0, 5_000 * T, ut(NOW), 0, 0], true, None, None),
            ("kratší než sekunda = nic", [0, 900 * T, 0, ut(NOW), 0, 900 * T], true, None, None),
            ("posunutý začátek", [10_000 * T, 70_000 * T, 40_000 * T, ut(NOW), 0, 1], false, None, Some((30_000, 60_000, NOW, 1.0, false))),
            ("pozice za koncem = konec", [0, 60_000 * T, 99_000 * T, ut(NOW), 0, 60_000 * T], true, None, Some((60_000, 60_000, NOW, 1.0, true))),
            ("záporná pozice = 0", [0, 60_000 * T, -5 * T, ut(NOW), 0, 60_000 * T], true, None, Some((0, 60_000, NOW, 1.0, true))),
            ("čas hlášení 0 = teď", [0, 60_000 * T, 1_000 * T, 0, 0, 60_000 * T], true, None, Some((1_000, 60_000, NOW, 1.0, true))),
            ("čas z budoucnosti = teď; rychlost 0 = 1×", [0, 60_000 * T, 1_000 * T, ut(NOW + 60_000), 0, 60_000 * T], true, Some(0.0), Some((1_000, 60_000, NOW, 1.0, true))),
            ("rozsah přetáčení prázdný = nejde; 1,5×", [0, 60_000 * T, 0, ut(NOW), 5, 5], true, Some(1.5), Some((0, 60_000, NOW, 1.5, false))),
        ];
        for (name, [s, e, p, u, mn, mx], seek, rate, want) in cases {
            let got = from_ticks(s, e, p, u, mn, mx, seek, rate, NOW).map(|t| (t.position_ms, t.duration_ms, t.updated_ms, t.rate, t.can_seek));
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn position_at_table() {
        let fast = Timeline { rate: 2.0, ..tl(10_000, 60_000, NOW - 5_000) };
        // (název, timeline, teď, hraje, pozice)
        let cases: [(&str, Timeline, i64, bool, i64); 7] = [
            ("hraje: + uplynulý čas", tl(10_000, 60_000, NOW - 5_000), NOW, true, 15_000),
            ("pauza: stojí", tl(10_000, 60_000, NOW - 5_000), NOW, false, 10_000),
            ("ořez na délku", tl(58_000, 60_000, NOW - 9_000), NOW, true, 60_000),
            ("hodiny před hlášením: neubírá", tl(10_000, 60_000, NOW + 3_000), NOW, true, 10_000),
            ("rychlost 2×", fast, NOW, true, 20_000),
            ("přesně v čase hlášení", tl(0, 60_000, NOW), NOW, true, 0),
            ("dlouho po konci", tl(1_000, 60_000, NOW - 3_600_000), NOW, true, 60_000),
        ];
        for (name, t, now, playing, want) in cases {
            assert_eq!(position_at(&t, now, playing), want, "{name}");
        }
    }

    #[test]
    fn merge_table() {
        let old = tl(10_000, 200_000, NOW - 10_000);
        // (název, předchozí (timeline, hrála), nové, hraje teď, očekávané (pozice, hlášeno))
        type Case = (&'static str, Option<(Timeline, bool)>, Option<Timeline>, bool, Option<(i64, i64)>);
        let cases: [Case; 9] = [
            ("bez timeline = nic", Some((old, true)), None, true, None),
            ("nová skladba: jak hlášeno", None, Some(tl(0, 180_000, NOW)), true, Some((0, NOW))),
            ("průběžné hlášení potvrzuje průběh: staré", Some((old, true)), Some(tl(20_200, 200_000, NOW)), true, Some((10_000, NOW - 10_000))),
            ("přetočení za chodu: nové", Some((old, true)), Some(tl(90_000, 200_000, NOW)), true, Some((90_000, NOW))),
            ("pauza se svěžím hlášením: nové", Some((old, true)), Some(tl(19_900, 200_000, NOW - 100)), false, Some((19_900, NOW - 100))),
            ("pauza, přehrávač neobnovil: zmrazit na dopočítané", Some((old, true)), Some(old), false, Some((20_000, NOW))),
            ("pokračování po pauze se starým hlášením: od teď", Some((tl(20_000, 200_000, NOW - 60_000), false)), Some(tl(20_000, 200_000, NOW - 60_000)), true, Some((20_000, NOW))),
            ("pokračování se svěžím hlášením: jak hlášeno", Some((tl(20_000, 200_000, NOW - 60_000), false)), Some(tl(20_000, 200_000, NOW - 200)), true, Some((20_000, NOW - 200))),
            ("pauza beze změny: staré", Some((tl(20_000, 200_000, NOW - 60_000), false)), Some(tl(20_000, 200_000, NOW - 5)), false, Some((20_000, NOW - 60_000))),
        ];
        for (name, prev, new, playing, want) in cases {
            let got = merge(prev.as_ref().map(|(t, p)| (t, *p)), new, playing, NOW).map(|t| (t.position_ms, t.updated_ms));
            assert_eq!(got, want, "{name}");
        }
        // jiná délka (jiná verze skladby) = nové i při stejné pozici
        let other = tl(20_000, 230_000, NOW);
        assert_eq!(merge(Some((&old, true)), Some(other), true, NOW), Some(other));
    }

    #[test]
    fn repeat_seek_and_minutes() {
        assert_eq!([0, 1, 2, 3, -1].map(repeat_of), [Some(Repeat::None), Some(Repeat::Track), Some(Repeat::List), None, None]);
        assert_eq!(serde_json::to_value(Repeat::List).unwrap(), "list");
        assert_eq!(serde_json::from_str::<Repeat>("\"track\"").unwrap(), Repeat::Track);
        let t = Timeline { start_ticks: 7, ..tl(0, 60_000, NOW) };
        assert_eq!(seek_ticks(&t, 30_000), Some(7 + 30_000 * T));
        assert_eq!(seek_ticks(&t, 99_000), Some(7 + 60_000 * T), "ořez na délku");
        assert_eq!(seek_ticks(&t, -5), Some(7), "ořez na začátek");
        assert_eq!(seek_ticks(&Timeline { can_seek: false, ..t }, 1_000), None);
        // minuty do proužku
        let song = tl(102_000, 243_000, NOW);
        assert_eq!(strip_time(&song, NOW, true).as_deref(), Some("1/4 min"));
        assert_eq!(strip_time(&song, NOW + 18_000, true).as_deref(), Some("2/4 min"));
        assert_eq!(strip_time(&song, NOW + 18_000, false).as_deref(), Some("1/4 min"), "pauza stojí");
        assert_eq!(strip_time(&tl(0, 59_000, NOW), NOW, true), None, "kratší než minuta");
        // budík další minuty: za 18 s (+1 ms), pauza / konec nic, 2× rychleji = za 9 s
        assert_eq!(next_minute(&song, NOW, true), Some(NOW + 18_001));
        assert_eq!(next_minute(&song, NOW, false), None);
        assert_eq!(next_minute(&tl(243_000, 243_000, NOW), NOW, true), None);
        assert_eq!(next_minute(&Timeline { rate: 2.0, ..song }, NOW, true), Some(NOW + 9_001));
    }
}
