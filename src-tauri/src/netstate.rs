//! Společné chování zdrojů dat ze sítě (limity, počasí, kalendář, pošta) při výpadku, po probuzení a při skoku hodin.
//! Čistá politika bez I/O, bez vlákna a bez časovače: jen odpovídá „je čas ptát se?“ ve chvíli, kdy by se `emit`
//! vlákno stejně probudilo (minutový tik, otevření šuplíku, změna záložky).
//!
//! - **Backoff:** po neúspěchu čeká zdroj `retry`, po dalších `2×`, `4×`… nejvýš `BACKOFF_CAP_MS` (`backoff_ms`).
//!   Úspěch čítač nuluje. `Retry-After` (`retry_after`) čekání jen prodlouží, nejvýš o `RETRY_AFTER_CAP_MS`.
//! - **Skok hodin:** čas pokusu / stažení z budoucnosti (hodiny šly zpět) se bere jako neznámý, ne jako „ještě ne“.
//!   Skok dopředu (uspání) je prostě „splatné“.
//! - **Probuzení a návrat sítě:** `note_tick` (volá `emit` při každém průchodu) pozná mezeru > `WAKE_GAP_MS`
//!   mezi průchody (vlákno jinak běží aspoň jednou za minutu) a zvedne `epoch`; totéž udělá první úspěch po
//!   neúspěchu jakéhokoli zdroje (`Fails::ok`). Čítač `Fails` z dřívější epochy se bere jako 0 — všechny zdroje
//!   se po probuzení / návratu sítě vrací k základnímu odstupu místo k odstupu, který naběhl při výpadku.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

/// Strop exponenciálního backoffu.
pub const BACKOFF_CAP_MS: i64 = 15 * 60_000;
/// Strop `Retry-After` (server smí chtít víc, čekat tak dlouho ale nebudeme).
pub const RETRY_AFTER_CAP_MS: i64 = 30 * 60_000;
/// Mezera mezi průchody `emit` smyčky, od které se to bere jako probuzení z uspání / skok hodin.
pub const WAKE_GAP_MS: i64 = 180_000;
/// Čas z budoucnosti o méně než tohle je jitter (jiné vlákno, zaokrouhlení), ne skok hodin.
const SKEW_MS: i64 = 5_000;

/// Politika jednoho zdroje: `every` = stáří dat, od kterého se obnovuje, `retry` = základní odstup po pokusu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub every: i64,
    pub retry: i64,
}

/// Odstup po `fails` neúspěších za sebou: 0 a 1 → `base`, pak 2×, 4×… nejvýš `cap` (a nikdy pod `base`).
pub fn backoff_ms(fails: u32, base: i64, cap: i64) -> i64 {
    let grown = base.saturating_mul(1i64 << fails.saturating_sub(1).min(20));
    grown.min(cap).max(base)
}

fn past(t: Option<i64>, now: i64) -> Option<i64> {
    t.filter(|&t| t <= now + SKEW_MS)
}

/// Nejdřív kdy (ms epochy) je zdroj k obnově: data stará `every` a od posledního pokusu uplynul odstup
/// (`backoff_ms`, případně delší `retry_after`). Nic z toho neznámé = 0 (hned). Jediný zdroj pravdy pro
/// `next_attempt` i pro termíny `next_deadline`.
pub fn next_at(p: &Policy, last_ok: Option<i64>, last_err: Option<i64>, fails: u32, now: i64, retry_after: Option<i64>) -> i64 {
    let wait = backoff_ms(fails, p.retry, BACKOFF_CAP_MS).max(retry_after.unwrap_or(0).clamp(0, RETRY_AFTER_CAP_MS));
    let old = past(last_ok, now).map_or(0, |f| f + p.every);
    let again = past(last_err, now).map_or(0, |a| a + wait);
    old.max(again)
}

/// Je čas ptát se? `last_ok` = čas posledního dobrého stažení, `last_err` = čas posledního pokusu.
pub fn next_attempt(p: &Policy, last_ok: Option<i64>, last_err: Option<i64>, fails: u32, now: i64, retry_after: Option<i64>) -> bool {
    next_at(p, last_ok, last_err, fails, now, retry_after) <= now
}

/// `next_attempt` bez běžícího pokusu (`busy` = pokus doopravdy běží, podle watchdogu zdroje).
pub fn needs_fetch(p: &Policy, busy: bool, last_ok: Option<i64>, last_attempt: Option<i64>, fails: u32, now: i64) -> bool {
    !busy && next_attempt(p, last_ok, last_attempt, fails, now, None)
}

/// Mezera mezi dvěma průchody smyčky = probuzení z uspání nebo skok hodin (zpět i dopředu)?
pub fn is_jump(prev: Option<i64>, now: i64) -> bool {
    prev.is_some_and(|p| now - p > WAKE_GAP_MS || now < p - SKEW_MS)
}

static EPOCH: AtomicU64 = AtomicU64::new(0);
static LAST_TICK: AtomicI64 = AtomicI64::new(i64::MIN);
/// Některý zdroj naposled selhal (a ještě žádný neuspěl) — první úspěch pak znamená „síť je zpět“.
static FAILING: AtomicBool = AtomicBool::new(false);

pub fn epoch() -> u64 {
    EPOCH.load(Ordering::Relaxed)
}

/// Volá `emit` smyčka při každém průchodu (levné: jedna atomika). Probuzení / skok hodin zvedne `epoch`.
pub fn note_tick(now: i64) {
    let prev = LAST_TICK.swap(now, Ordering::Relaxed);
    if is_jump((prev != i64::MIN).then_some(prev), now) {
        EPOCH.fetch_add(1, Ordering::Relaxed);
    }
}

/// Čítač neúspěchů za sebou jednoho zdroje; platí jen v epoše, ve které naběhl.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Fails {
    n: u32,
    epoch: u64,
}

impl Fails {
    /// Pro `static` inicializátory (`Default` není `const`).
    pub const NEW: Fails = Fails { n: 0, epoch: 0 };

    /// Čítač při epoše `epoch` (jiná epocha = 0).
    pub fn at(&self, epoch: u64) -> u32 {
        if self.epoch == epoch { self.n } else { 0 }
    }

    /// Aktuální počet neúspěchů (po probuzení / návratu sítě 0).
    pub fn count(&self) -> u32 {
        self.at(epoch())
    }

    pub fn fail(&mut self) {
        *self = Fails { n: self.count().saturating_add(1), epoch: epoch() };
        FAILING.store(true, Ordering::Relaxed);
    }

    /// Úspěch: nuluje čítač; byl-li předtím nějaký zdroj v neúspěchu, síť je zpět → ostatním zdrojům se backoff zruší.
    pub fn ok(&mut self) {
        *self = Fails::default();
        if FAILING.swap(false, Ordering::Relaxed) {
            EPOCH.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;
    const MIN: i64 = 60_000;
    const P: Policy = Policy { every: 30 * MIN, retry: 2 * MIN };

    #[test]
    fn backoff_grows_and_caps() {
        // (neúspěchy, očekávaný odstup v minutách)
        let cases = [(0, 2), (1, 2), (2, 4), (3, 8), (4, 15), (30, 15), (u32::MAX, 15)];
        for (f, want) in cases {
            assert_eq!(backoff_ms(f, 2 * MIN, BACKOFF_CAP_MS), want * MIN, "fails={f}");
        }
        // základ nad stropem: nikdy pod základ
        assert_eq!(backoff_ms(5, 20 * MIN, BACKOFF_CAP_MS), 20 * MIN);
    }

    /// (poslední dobré, poslední pokus, neúspěchy, retry_after, očekávané due)
    type Case = (Option<i64>, Option<i64>, u32, Option<i64>, bool);

    #[test]
    fn next_attempt_table() {
        let cases: [(&str, Case); 14] = [
            ("nic nebylo → hned", (None, None, 0, None, true)),
            ("pokus před 1 min, bez dat → čeká", (None, Some(NOW - MIN), 0, None, false)),
            ("pokus před 2 min, bez dat → hned", (None, Some(NOW - 2 * MIN), 0, None, true)),
            ("čerstvá data → ne", (Some(NOW - 10 * MIN), Some(NOW - 10 * MIN), 0, None, false)),
            ("stará data a pokus → ano", (Some(NOW - 30 * MIN), Some(NOW - 30 * MIN), 0, None, true)),
            ("3 neúspěchy → 8 min: po 5 ne", (None, Some(NOW - 5 * MIN), 3, None, false)),
            ("3 neúspěchy → 8 min: po 8 ano", (None, Some(NOW - 8 * MIN), 3, None, true)),
            ("strop 15 min", (None, Some(NOW - 14 * MIN), 30, None, false)),
            ("strop 15 min: po 15 ano", (None, Some(NOW - 15 * MIN), 30, None, true)),
            ("Retry-After 10 min prodlouží", (None, Some(NOW - 5 * MIN), 1, Some(10 * MIN), false)),
            ("Retry-After nad stropem se ořízne", (None, Some(NOW - 31 * MIN), 1, Some(600 * MIN), true)),
            ("hodiny zpět: pokus z budoucnosti = neznámý → ano", (None, Some(NOW + 3_600_000), 0, None, true)),
            ("hodiny zpět: stažení z budoucnosti = neznámé → ano", (Some(NOW + 3_600_000), None, 0, None, true)),
            ("jitter do 5 s z budoucnosti se bere jako přítomnost", (None, Some(NOW + 1_000), 0, None, false)),
        ];
        for (name, (ok, err, fails, ra, want)) in cases {
            assert_eq!(next_attempt(&P, ok, err, fails, NOW, ra), want, "{name}");
        }
    }

    #[test]
    fn needs_fetch_not_while_busy() {
        assert!(!needs_fetch(&P, true, None, None, 0, NOW));
        assert!(needs_fetch(&P, false, None, None, 0, NOW));
    }

    #[test]
    fn next_at_is_max_of_age_and_backoff() {
        // data stará 5 min (obnova za 25), pokus před 1 min, 3 neúspěchy (8 min) → za 7 min po pokusu už je čekání jen na stáří
        let (ok, err) = (Some(NOW - 5 * MIN), Some(NOW - MIN));
        assert_eq!(next_at(&P, ok, err, 3, NOW, None), NOW + 25 * MIN);
        // bez dat: jen odstup po pokusu
        assert_eq!(next_at(&P, None, err, 3, NOW, None), NOW - MIN + 8 * MIN);
        assert_eq!(next_at(&P, None, None, 3, NOW, None), 0);
    }

    #[test]
    fn jump_detection() {
        // (předchozí průchod, teď) → skok
        let cases = [
            (None, NOW, false),
            (Some(NOW - 60_000), NOW, false),
            (Some(NOW - WAKE_GAP_MS), NOW, false),
            (Some(NOW - WAKE_GAP_MS - 1), NOW, true),
            (Some(NOW - 8 * 3_600_000), NOW, true),
            (Some(NOW + 3_600_000), NOW, true),
            (Some(NOW + 1_000), NOW, false),
        ];
        for (prev, now, want) in cases {
            assert_eq!(is_jump(prev, now), want, "{prev:?}");
        }
    }

    #[test]
    fn fails_reset_by_epoch() {
        let f = Fails { n: 4, epoch: 7 };
        assert_eq!(f.at(7), 4);
        assert_eq!(f.at(8), 0, "po probuzení / návratu sítě začíná backoff znovu");
        assert_eq!(Fails::default().at(0), 0);
    }
}
