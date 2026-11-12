//! Stahování kalendáře bez viditelné karty kvůli připomínkám (kolo 31b, L43). Čisté funkce: kdy se smí stahovat
//! (`mode`), jak často (`gate`) a kdy se má emit vlákno probudit (`due_at`). Žádné vlákno ani časovač — jen
//! termín pro stávající `next_deadline` a stávající `maybe_refresh` po probuzení.

use super::live::{MAX_AGE_MS, RETRY_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// nic: karta není vidět a žádný kalendář nemá připomínku (nebo je Ledge mrtvý)
    Skip,
    /// karta je vidět: dnešní chování (retry po `RETRY_MS`)
    Visible,
    /// karta není vidět, ale je zapnutá připomínka: jen její kalendáře, nejvýš 1× za `MAX_AGE_MS`
    Background,
}

/// Má se stahovat? Vidět = vždy (dnešní chování, `dead` se tu nikdy nekontrolovalo); jinak jen s připomínkou a živý Ledge.
pub fn mode(visible: bool, remind: bool, dead: bool) -> Mode {
    if visible {
        Mode::Visible
    } else if remind && !dead {
        Mode::Background
    } else {
        Mode::Skip
    }
}

/// Odstup mezi pokusy v daném režimu.
pub fn retry_ms(m: Mode) -> i64 {
    if m == Mode::Background { MAX_AGE_MS } else { RETRY_MS }
}

/// Smí se začít nový pokus (poslední pokus je starší než odstup režimu)? `Skip` nikdy.
pub fn gate(m: Mode, now: i64, attempt_ms: Option<i64>) -> bool {
    m != Mode::Skip && attempt_ms.is_none_or(|a| now - a >= retry_ms(m))
}

/// Kdy nejdřív bude zdroj k stažení: data stará `MAX_AGE_MS` a od posledního pokusu uplynul odstup. Bez dat a bez pokusu = 0 (hned).
pub fn due_at(fetched_ms: Option<i64>, attempt_ms: Option<i64>, retry: i64) -> i64 {
    let old = fetched_ms.map_or(0, |f| f + MAX_AGE_MS);
    let again = attempt_ms.map_or(0, |a| a + retry);
    old.max(again)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_table() {
        use Mode::*;
        // (vidět, připomínka, mrtvý) → režim
        let cases = [
            (true, false, false, Visible),
            (true, true, false, Visible),
            (true, false, true, Visible),
            (false, true, false, Background),
            (false, true, true, Skip),
            (false, false, false, Skip),
            (false, false, true, Skip),
        ];
        for (v, r, d, want) in cases {
            assert_eq!(mode(v, r, d), want, "visible={v} remind={r} dead={d}");
        }
    }

    #[test]
    fn gate_keeps_background_at_least_15_min() {
        let now = 10_000_000;
        assert!(gate(Mode::Background, now, None));
        assert!(!gate(Mode::Background, now, Some(now - MAX_AGE_MS + 1)), "pozadí: ne dřív než 15 min");
        assert!(gate(Mode::Background, now, Some(now - MAX_AGE_MS)));
        assert!(!gate(Mode::Visible, now, Some(now - RETRY_MS + 1)));
        assert!(gate(Mode::Visible, now, Some(now - RETRY_MS)));
        assert!(!gate(Mode::Skip, now, None));
        const { assert!(MAX_AGE_MS >= 15 * 60_000) };
    }

    #[test]
    fn due_at_table() {
        let t = 1_000_000;
        let r = MAX_AGE_MS;
        let cases = [
            (None, None, 0),
            (Some(t), Some(t), t + MAX_AGE_MS),
            (Some(t), None, t + MAX_AGE_MS),
            (None, Some(t), t + r),
            (Some(t - 3 * MAX_AGE_MS), Some(t), t + r), // stará data, ale pokus čerstvý → počká
        ];
        for (f, a, want) in cases {
            assert_eq!(due_at(f, a, r), want, "{f:?} {a:?}");
        }
    }
}
