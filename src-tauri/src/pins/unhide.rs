//! Čisté věci schovaných lístků a vypnutí poznámek (tabulkové testy; okna a vlákna jsou v `win.rs`):
//! pravdivý seznam „schováno do…“ pro šuplík (`hidden_refs`), politika vynoření po selhání
//! (`after_attempt` + `apply_verdict` + počítadlo `Tries`) a co s okny při přepnutí `notes.enabled`
//! (`config_plan`). Mapa: `docs/codemap/notes.md`.

use super::{Pin, PinsFile, same_note};
use serde::Serialize;

/// Kolikrát se schovaný lístek zkusí vynořit, než se vzdá (pak se odepne a poloha zůstane v paměti).
pub const MAX_TRIES: u8 = 3;
/// Odstup opakovaného pokusu po selhání (ms) — jede přes existující termín vlákna `emit`, žádný časovač.
pub const RETRY_MS: i64 = 5 * 60_000;

/// Schovaný lístek pro šuplík: kdy se vrátí (ms epochy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenRef {
    pub path: String,
    pub until: i64,
}

/// Připnuté lístky s plánem návratu a bez okna (schovaný lístek okno nemá — jinak by šuplík tvrdil „nepřipnutý“).
pub fn hidden_refs(pins: &[Pin], has_window: impl Fn(&str) -> bool) -> Vec<HiddenRef> {
    pins.iter().filter(|p| p.open && !has_window(&p.path)).filter_map(|p| p.hidden_until.map(|until| HiddenRef { path: p.path.clone(), until })).collect()
}

/// Výsledek jednoho pokusu o vynoření.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// okno stojí, nic dalšího
    Done,
    /// zkusit znovu v `at`; `log` jen u prvního selhání (opakování mlčí)
    Retry { at: i64, log: bool },
    /// vzdát to: odepnout (vždy se zaloguje, jednou)
    GiveUp,
}

/// `tries_before` = kolik pokusů už selhalo před tímto.
pub fn after_attempt(ok: bool, tries_before: u8, now_ms: i64) -> Verdict {
    if ok {
        return Verdict::Done;
    }
    let n = tries_before.saturating_add(1);
    if n >= MAX_TRIES { Verdict::GiveUp } else { Verdict::Retry { at: now_ms + RETRY_MS, log: n == 1 } }
}

/// Zapíše verdikt do stavu: opakování = nový termín v budoucnosti (ne minulý, který by `tick` zkoušel pořád),
/// vzdání = odepnutí (poloha a vzhled zůstanou).
pub fn apply_verdict(f: &mut PinsFile, path: &str, v: Verdict, now_ms: i64) {
    match v {
        Verdict::Done => {}
        Verdict::Retry { at, .. } => {
            if let Some(p) = f.pins.iter_mut().find(|p| p.open && same_note(&p.path, path)) {
                p.hidden_until = Some(at);
            }
        }
        Verdict::GiveUp => {
            f.close(path, now_ms);
        }
    }
}

/// Počet neúspěšných pokusů o vynoření po lístcích (jen paměť).
#[derive(Debug, Default)]
pub struct Tries(Vec<(String, u8)>);

impl Tries {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn get(&self, path: &str) -> u8 {
        self.0.iter().find(|(p, _)| same_note(p, path)).map_or(0, |(_, n)| *n)
    }

    pub fn set(&mut self, path: &str, n: u8) {
        self.0.retain(|(p, _)| !same_note(p, path));
        if n > 0 {
            self.0.push((path.to_string(), n));
        }
    }
}

/// Co s okny lístků při změně configu `notes.enabled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigPlan {
    /// zavřít okna (stav v `note-pins.json` zůstane, `open` se nemění)
    pub close_all: bool,
    /// po zapnutí otevřít zpět připnuté (jako po startu)
    pub restore: bool,
}

pub fn config_plan(old_enabled: bool, new_enabled: bool) -> ConfigPlan {
    ConfigPlan { close_all: old_enabled && !new_enabled, restore: !old_enabled && new_enabled }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(path: &str, open: bool, hidden: Option<i64>) -> Pin {
        Pin { open, hidden_until: hidden, ..Pin::new(path) }
    }

    #[test]
    fn hidden_refs_table() {
        let pins = [pin("a", true, Some(100)), pin("b", true, None), pin("c", false, Some(5)), pin("d", true, Some(200))];
        // d má okno (už se vynořil) → jen a
        let got = hidden_refs(&pins, |p| p == "d");
        assert_eq!(got, vec![HiddenRef { path: "a".into(), until: 100 }]);
        assert!(hidden_refs(&[], |_| false).is_empty());
    }

    #[test]
    fn attempt_table() {
        // (ok, dřívější selhání) → verdikt
        let n = 1_000;
        assert_eq!(after_attempt(true, 0, n), Verdict::Done);
        assert_eq!(after_attempt(true, 2, n), Verdict::Done);
        assert_eq!(after_attempt(false, 0, n), Verdict::Retry { at: n + RETRY_MS, log: true });
        assert_eq!(after_attempt(false, 1, n), Verdict::Retry { at: n + RETRY_MS, log: false });
        assert_eq!(after_attempt(false, 2, n), Verdict::GiveUp);
        assert_eq!(after_attempt(false, u8::MAX, n), Verdict::GiveUp);
    }

    #[test]
    fn failure_sequence_logs_twice_and_ends() {
        // opakované selhání: log při 1. selhání a při vzdání, mezitím ticho; konec = odepnuto
        let mut f = PinsFile { pins: vec![pin("a.md", true, Some(10))], ..Default::default() };
        let mut tries = Tries::new();
        let (mut logs, mut now) = (0, 100);
        for _ in 0..10 {
            if !f.pins[0].open {
                break;
            }
            let before = tries.get("A.md");
            let v = after_attempt(false, before, now);
            match v {
                Verdict::Retry { at, log } => {
                    logs += log as u8;
                    tries.set("a.md", before + 1);
                    assert!(at > now);
                }
                Verdict::GiveUp => logs += 1,
                Verdict::Done => unreachable!(),
            }
            apply_verdict(&mut f, "a.md", v, now);
            if let Some(t) = f.pins[0].hidden_until {
                assert!(t > now, "termín v budoucnosti");
                now = t;
            }
        }
        assert_eq!(logs, 2);
        assert!(!f.pins[0].open && f.pins[0].hidden_until.is_none());
        assert_eq!(f.next_unhide(0), None);
    }

    #[test]
    fn success_leaves_state() {
        let mut f = PinsFile { pins: vec![pin("a.md", true, None)], ..Default::default() };
        let before = f.clone();
        apply_verdict(&mut f, "a.md", Verdict::Done, 5);
        assert_eq!(f, before);
        // Retry u neexistujícího lístku nic nepřidá
        apply_verdict(&mut f, "zzz.md", Verdict::Retry { at: 9, log: true }, 5);
        assert_eq!(f, before);
    }

    #[test]
    fn tries_counter() {
        let mut t = Tries::new();
        assert_eq!(t.get("X.md"), 0);
        t.set("X.md", 2);
        assert_eq!(t.get("x.md"), 2);
        t.set("x.md", 0);
        assert_eq!(t.get("X.md"), 0);
    }

    #[test]
    fn config_plan_table() {
        // (bylo, je) → (zavřít, obnovit)
        let cases = [(true, true, false, false), (true, false, true, false), (false, true, false, true), (false, false, false, false)];
        for (old, new, close_all, restore) in cases {
            assert_eq!(config_plan(old, new), ConfigPlan { close_all, restore }, "{old} → {new}");
        }
    }
}
