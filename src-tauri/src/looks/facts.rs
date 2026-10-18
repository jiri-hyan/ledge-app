//! Převod stavu jádra na `Facts` pro styly zavřeného Ledge — čisté funkce nad hotovými daty z paměti
//! (časovač, právě hraje, limit 5h). Sběr ze stavu jádra dělá `stripview::look_facts`.

use super::{MusicFact, TimerFact};
use crate::i18n::tr;
use crate::media::MediaView;
use crate::timer::{Countdown, TimerState};

/// Odpočet má přednost před stopkami; nic neběží = `None`. Lišta průběhu jen u odpočtu s délkou.
pub fn timer_fact(s: &TimerState, now: i64) -> Option<TimerFact> {
    if let Some(c) = &s.countdown {
        return Some(match *c {
            Countdown::Done { .. } => TimerFact { text: tr("čas vypršel", "time's up").into(), minutes: None, progress: None, done: true },
            Countdown::Running { total, .. } | Countdown::Paused { total, .. } => {
                let left = crate::timer::left_ms(c, now);
                let minutes = ((left.max(0) + 59_999) / 60_000) as u32;
                let progress = (total > 0).then(|| (1.0 - left as f64 / total as f64).clamp(0.0, 1.0) as f32);
                TimerFact { text: crate::timer::minutes_text(left), minutes: Some(minutes), progress, done: false }
            }
        });
    }
    s.stopwatch.as_ref().map(|w| TimerFact { text: crate::timer::clock_text(crate::timer::elapsed_ms(w, now)), minutes: None, progress: None, done: false })
}

/// Jen při přehrávání. Čas a průběh jen z timeline zdroje (po minutách — proužek se po sekundách
/// nepřepočítává).
pub fn music_fact(v: Option<&MediaView>, now: i64) -> Option<MusicFact> {
    let text = crate::mediaconf::strip_text(v, 36)?;
    let tl = v.and_then(|v| v.timeline.as_ref());
    let time = tl.and_then(|t| crate::mediatime::strip_time(t, now, true));
    let progress = tl
        .filter(|t| t.duration_ms > 0)
        .map(|t| (crate::mediatime::position_at(t, now, true) as f64 / t.duration_ms as f64).clamp(0.0, 1.0) as f32);
    Some(MusicFact { text, time, progress })
}

/// Čerstvé procento okna 5h; starý vzorek / po resetu / žádná data = `None` (nic nevymýšlet).
pub fn limit_fact(v: &crate::limitslive::LimitsView, now: i64) -> Option<f64> {
    let (w, at) = (v.five_hour?, v.at?);
    match crate::limits::state(w, at, now) {
        crate::limits::State::Fresh(p) => Some(p),
        _ => None,
    }
}
