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

/// Aktivita vedoucí session pro zavřený Ledge: fáze (odhad), jméno souboru, projekt.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityFact {
    pub phase: crate::claude::activity::Phase,
    /// jen jméno souboru („main.rs“), "" = žádný
    pub file: String,
    /// projekt session ("" = neznámý)
    pub project: String,
}

/// `claude::activity::lead` → fakt; `project` dodá volající (jméno projektu podle cwd).
pub fn activity_fact(s: Option<&crate::claude::Session>, project: impl Fn(&str) -> String) -> Option<ActivityFact> {
    let s = s?;
    let a = s.activity.as_ref()?;
    let file = a.file.as_deref().map(crate::claude::activity::base_name).unwrap_or("").to_string();
    Some(ActivityFact { phase: a.phase, file, project: project(&s.cwd) })
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

/// Čerstvé procento týdenního okna (stejná pravidla jako `limit_fact`).
pub fn limit7d_fact(v: &crate::limitslive::LimitsView, now: i64) -> Option<f64> {
    let (w, at) = (v.seven_day?, v.at?);
    match crate::limits::state(w, at, now) {
        crate::limits::State::Fresh(p) => Some(p),
        _ => None,
    }
}

/// Baterie pro styly (`battery::view` je `None` na stroji bez baterie i při vypnutém údaji).
pub fn battery_fact(v: Option<&crate::battery::BatteryView>) -> Option<super::BatteryFact> {
    v.map(|b| super::BatteryFact { percent: b.percent.min(100), charging: b.charging })
}

/// Jak staré počasí z karty Počasí proužek ještě ukáže (proužek sám nic nestahuje; starší = nic).
pub const WEATHER_SHOW_MS: i64 = 3 * 60 * 60 * 1000;

/// Počasí z paměti (`Core.weather`): jen data mladší než `WEATHER_SHOW_MS`, jinak `None`.
pub fn weather_fact(data: Option<&crate::weather::Weather>, fetched_ms: Option<i64>, now: i64) -> Option<super::WeatherFact> {
    let (w, at) = (data?, fetched_ms?);
    if now - at >= WEATHER_SHOW_MS || !w.temp.is_finite() {
        return None;
    }
    Some(super::WeatherFact { temp: w.temp.round() as i32, glyph: super::capsule::weather_glyph(w.code, w.is_day) })
}

/// Jak starý vzorek CPU / paměti proužek ještě ukáže (plánovač bere vzorek po 10 s).
pub const SYSTEM_SHOW_MS: i64 = 30_000;

/// CPU a paměť v % z posledního vzorku `sysmon` (`(cpu, paměť, kdy)`); starší než `SYSTEM_SHOW_MS` = nic.
pub fn system_fact(sample: Option<(Option<f32>, Option<f32>, i64)>, now: i64) -> (Option<f64>, Option<f64>) {
    match sample {
        Some((cpu, mem, at)) if now - at < SYSTEM_SHOW_MS && now >= at => (cpu.map(f64::from), mem.map(f64::from)),
        _ => (None, None),
    }
}

/// Kolik vzorků drží mini graf proužku (~4 min po 10 s).
pub const SPARK_N: usize = 24;
/// Nejkratší odstup dvou bodů grafu (vzorky karty po 2 s by osu zhustily).
pub const SPARK_STEP_MS: i64 = 8_000;

/// Historie CPU / paměti pro styl „průběh“ vlastního vzhledu — jen ze skutečných vzorků `sysmon`, které proužek
/// stejně čte (nic nevzorkuje navíc, žádný časovač). Mezera delší než `SYSTEM_SHOW_MS` = graf od začátku.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SysHist {
    at: Option<i64>,
    pub cpu: Vec<f32>,
    pub mem: Vec<f32>,
}

impl SysHist {
    /// Zapíše vzorek `(cpu, paměť, kdy)`, pokud je nový a aspoň `SPARK_STEP_MS` po předchozím bodu.
    pub fn push(&mut self, sample: Option<(Option<f32>, Option<f32>, i64)>, now: i64) {
        let Some((cpu, mem, at)) = sample.filter(|s| now >= s.2 && now - s.2 < SYSTEM_SHOW_MS) else {
            // žádný čerstvý vzorek: starý graf by lhal
            *self = SysHist::default();
            return;
        };
        if let Some(prev) = self.at {
            if at - prev > SYSTEM_SHOW_MS || at < prev {
                *self = SysHist::default();
            } else if at - prev < SPARK_STEP_MS {
                return;
            }
        }
        self.at = Some(at);
        for (v, h) in [(cpu, &mut self.cpu), (mem, &mut self.mem)] {
            if let Some(v) = v {
                h.push(v.clamp(0.0, 100.0));
                if h.len() > SPARK_N {
                    h.remove(0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sys_hist_real_samples_only() {
        let mut h = SysHist::default();
        // (vzorek, teď, čekané body CPU)
        type Step = (Option<(Option<f32>, Option<f32>, i64)>, i64, usize);
        let steps: [Step; 7] = [
            (Some((Some(10.0), Some(40.0), 1_000)), 1_000, 1),
            (Some((Some(10.0), Some(40.0), 1_000)), 5_000, 1), // stejný vzorek
            (Some((Some(20.0), Some(41.0), 3_000)), 5_000, 1), // karta po 2 s: moc brzy
            (Some((Some(30.0), Some(42.0), 11_000)), 11_000, 2),
            (Some((None, Some(42.0), 21_000)), 21_000, 2),     // CPU čeká na druhý vzorek
            (Some((Some(50.0), Some(43.0), 90_000)), 90_000, 1), // mezera = od začátku
            (None, 91_000, 0),
        ];
        for (i, (s, now, want)) in steps.into_iter().enumerate() {
            h.push(s, now);
            assert_eq!(h.cpu.len(), want, "krok {i}");
        }
        for i in 0..40 {
            h.push(Some((Some(i as f32), Some(200.0), i * 10_000)), i * 10_000);
        }
        assert_eq!((h.cpu.len(), h.mem.len(), h.cpu[SPARK_N - 1], h.mem[0]), (SPARK_N, SPARK_N, 39.0, 100.0));
    }

    #[test]
    fn new_facts_only_fresh_real_data() {
        let now = 10_000_000_000;
        // systém: čerstvý vzorek ano, starý / z budoucnosti ne, CPU bez prvního vzorku = jen paměť
        assert_eq!(system_fact(Some((Some(8.0), Some(46.0), now - 5_000)), now), (Some(8.0), Some(46.0)));
        assert_eq!(system_fact(Some((None, Some(46.0), now)), now), (None, Some(46.0)));
        assert_eq!(system_fact(Some((Some(8.0), None, now)), now), (Some(8.0), None), "neznámá velikost paměti ≠ 0 %");
        assert_eq!(system_fact(Some((Some(8.0), Some(46.0), now - SYSTEM_SHOW_MS)), now), (None, None));
        assert_eq!(system_fact(Some((Some(8.0), Some(46.0), now + 1)), now), (None, None));
        assert_eq!(system_fact(None, now), (None, None));
        // baterie
        let b = crate::battery::BatteryView { percent: 82, charging: true, plugged: true, remaining_s: None };
        assert_eq!(battery_fact(Some(&b)), Some(super::super::BatteryFact { percent: 82, charging: true }));
        assert_eq!(battery_fact(None), None);
        // počasí: čerstvé ano, staré / bez času ne
        let w = crate::weather::parse(include_str!("../fixtures/open-meteo-praha.json")).unwrap();
        let f = weather_fact(Some(&w), Some(now - 60_000), now).unwrap();
        assert_eq!(f.temp, w.temp.round() as i32);
        assert_eq!(weather_fact(Some(&w), Some(now - WEATHER_SHOW_MS), now), None);
        assert_eq!(weather_fact(Some(&w), None, now), None);
        assert_eq!(weather_fact(None, Some(now), now), None);
    }
}
