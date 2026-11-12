//! Brána vlákna `emit` (L67): kdy poslat `state` šuplíku a hlídač výkonu. Čisté funkce + drobný čítač,
//! žádné vlákno ani časovač.
//!
//! - `should_send`: shodný stav jako poslední odeslaný se neposílá (otisk serializovaného JSON).
//! - `min_gap`: odstup mezi snapshoty roste s cenou posledního (8× cena, 120 ms … 2 s) — pomalý snapshot
//!   tak nemůže zabrat víc než ~1/9 jádra, ani kdyby ho něco budilo pořád.
//! - `Tally`: kolik průchodů smyčkou, kolik odesláno, kolik přeskočeno a KDO budil (`mark_dirty` s
//!   `#[track_caller]`); `report` z toho jednou za minutu udělá řádek do `ledge.log`, jen když je co hlásit
//!   (pomalý krok > 50 ms, nebo > 1 průchod/s).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::Location;
use std::time::Duration;

pub(crate) const SLOW_MS: u64 = 50;
const REPORT_EVERY_MS: i64 = 60_000;
const BUSY_PER_MIN: u32 = 60;
const MAX_SOURCES: usize = 40;

pub(crate) fn fingerprint(json: &str) -> u64 {
    let mut h = DefaultHasher::new();
    json.hash(&mut h);
    h.finish()
}

/// Poslat? Ano, když ještě nic neodešlo (`None`) nebo se otisk liší.
pub(crate) fn should_send(last: Option<u64>, now: u64) -> bool {
    last != Some(now)
}

/// Nejmenší odstup do dalšího snapshotu podle ceny posledního.
pub(crate) fn min_gap(cost_ms: u64) -> Duration {
    Duration::from_millis(cost_ms.saturating_mul(8).clamp(120, 2_000))
}

/// Řádek do logu o krocích přes práh (`name=ms`), jinak `None`.
pub(crate) fn slow_line(steps: &[(&str, u64)]) -> Option<String> {
    let slow: Vec<String> = steps.iter().filter(|(_, ms)| *ms > SLOW_MS).map(|(n, ms)| format!("{n}={ms} ms")).collect();
    (!slow.is_empty()).then(|| slow.join(", "))
}

#[derive(Default)]
pub(crate) struct Tally {
    pub(crate) wakes: u32,
    pub(crate) sent: u32,
    pub(crate) same: u32,
    slow: Option<String>,
    last_report_ms: i64,
}

impl Tally {
    pub(crate) fn note_slow(&mut self, steps: &[(&str, u64)]) {
        if let Some(l) = slow_line(steps) {
            self.slow = Some(l);
        }
    }

    /// Řádek k zalogování (a vynulování čítačů), nejdřív po minutě od minulého; `sources` = kdo budil.
    pub(crate) fn report(&mut self, now_ms: i64, sources: impl FnOnce() -> Vec<(String, u32)>) -> Option<String> {
        if now_ms - self.last_report_ms < REPORT_EVERY_MS {
            return None;
        }
        let sources = sources();
        let busy = self.wakes > BUSY_PER_MIN;
        if !busy && self.slow.is_none() {
            self.wakes = 0;
            self.sent = 0;
            self.same = 0;
            self.last_report_ms = now_ms;
            return None;
        }
        let mut src = sources;
        src.sort_by_key(|s| std::cmp::Reverse(s.1));
        let top: Vec<String> = src.iter().take(5).map(|(w, n)| format!("{w}×{n}")).collect();
        let line = format!(
            "emit: {} průchodů, {} odesláno, {} shodných přeskočeno; pomalé: {}; budí: {}",
            self.wakes,
            self.sent,
            self.same,
            self.slow.take().unwrap_or_else(|| "-".into()),
            if top.is_empty() { "-".into() } else { top.join(", ") }
        );
        *self = Tally { last_report_ms: now_ms, ..Tally::default() };
        Some(line)
    }
}

static SOURCES: parking_lot::Mutex<Vec<(&'static Location<'static>, u32)>> = parking_lot::Mutex::new(Vec::new());

/// Zapíše, odkud přišlo `mark_dirty` (jen místo volání, žádné alokace mimo první výskyt).
pub(crate) fn note_source(at: &'static Location<'static>) {
    let mut g = SOURCES.lock();
    if let Some(e) = g.iter_mut().find(|(l, _)| std::ptr::eq(*l, at) || **l == *at) {
        e.1 = e.1.saturating_add(1);
    } else if g.len() < MAX_SOURCES {
        g.push((at, 1));
    }
}

/// Vybere a vynuluje zdroje (`soubor:řádek`).
pub(crate) fn take_sources() -> Vec<(String, u32)> {
    std::mem::take(&mut *SOURCES.lock()).into_iter().map(|(l, n)| (format!("{}:{}", l.file().rsplit(['/', '\\']).next().unwrap_or(""), l.line()), n)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_send_table() {
        let a = fingerprint("{\"a\":1}");
        let b = fingerprint("{\"a\":2}");
        for (last, now, want) in [(None, a, true), (Some(a), a, false), (Some(a), b, true), (Some(b), a, true)] {
            assert_eq!(should_send(last, now), want, "{last:?} {now}");
        }
    }

    #[test]
    fn min_gap_table() {
        for (cost, want) in [(0, 120), (5, 120), (15, 120), (20, 160), (100, 800), (250, 2_000), (u64::MAX, 2_000)] {
            assert_eq!(min_gap(cost).as_millis() as u64, want, "{cost}");
        }
    }

    #[test]
    fn slow_line_table() {
        assert_eq!(slow_line(&[("a", 10), ("b", 50)]), None);
        assert_eq!(slow_line(&[("snapshot", 95), ("emit", 3), ("desk", 51)]).as_deref(), Some("snapshot=95 ms, desk=51 ms"));
    }

    /// Smyčka „UI → příkaz → mark_dirty → state“: dokud se stav nemění, neodchází nic.
    #[test]
    fn feedback_loop_sends_once() {
        let json = "{\"cards\":[1,2,3]}";
        let mut last = None;
        let (mut sent, mut same) = (0, 0);
        for _ in 0..100 {
            let fp = fingerprint(json);
            if should_send(last, fp) {
                last = Some(fp);
                sent += 1;
            } else {
                same += 1;
            }
        }
        assert_eq!((sent, same), (1, 99));
    }

    #[test]
    fn report_rate_limited_and_only_when_noteworthy() {
        let mut t = Tally { last_report_ms: 0, ..Default::default() };
        t.wakes = 10;
        assert_eq!(t.report(120_000, Vec::new), None, "klid → nic");
        t.wakes = 500;
        t.sent = 5;
        t.same = 495;
        assert_eq!(t.report(150_000, Vec::new), None, "méně než minuta od minulého");
        t.wakes = 500;
        t.sent = 5;
        t.same = 495;
        let l = t.report(190_000, || vec![("a.rs:1".into(), 3), ("b.rs:2".into(), 400)]).unwrap();
        assert!(l.contains("500 průchodů") && l.contains("b.rs:2×400, a.rs:1×3"), "{l}");
        t.note_slow(&[("snapshot", 90)]);
        assert!(t.report(260_000, Vec::new).unwrap().contains("snapshot=90 ms"));
        assert_eq!(t.wakes, 0);
    }
}
