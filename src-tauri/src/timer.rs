//! Časovač: odpočet (předvolby + vlastní minuty, pauza, +5 min, zrušit) a stopky (start, pauza, reset).
//!
//! Žádný tikající časovač: stav jsou jen časy (ms epochy) — konec odpočtu, virtuální start stopek.
//! Vlákno `emit` se probudí jen v `next_deadline` (konec odpočtu, změna minuty v proužku, konec ukázky
//! „hotovo“), proužek se tak překresluje po minutách; sekundy počítá jen otevřený šuplík sám z časů.
//! Konec odpočtu = `attention::signal` (bliknutí + puls, mrtvý Ledge nic) a volitelný toast (`timer.toast`).
//!
//! Stav se zapisuje do `timer.json` vedle configu jen při akci uživatele (start, pauza…, ne při tiku),
//! takže běžící odpočet přežije restart Ledge (instalace aktualizace, odhlášení). Odpočet, který doběhl,
//! zatímco Ledge neběžel, se po startu jen tiše ukáže jako „hotovo“ (bez pulsu a toastu).

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use crate::strip::{Rgba, Segment};
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MIN: i64 = 60_000;
/// Nejdelší odpočet (minuty) — předvolby, vlastní hodnota i +5 min.
pub const MAX_MIN: u32 = 600;
/// Jak dlouho proužek po doběhnutí ukazuje „hotovo“ (pak se odpočet sám smaže).
pub const DONE_SHOW_MS: i64 = 10 * MIN;
/// Soubor se stavem v `config::config_dir`.
pub(crate) const FILE: &str = "timer.json";
/// Barva upozornění na konec odpočtu (jiná než Claude: jantar = čeká, tyrkys = hotovo).
pub(crate) const COLOR: Rgba = Rgba::BLUE;

/// Sekce `timer` v configu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TimerConfig {
    /// Předvolby odpočtu v minutách (karta Časovač, paleta Ctrl+K): 1–600, nejvýš 6. Výchozí 5/15/25/50.
    pub presets: Vec<u32>,
    /// Oznámení Windows (toast) po doběhnutí odpočtu. Fokus nebere; ve hře, při „nerušit“ a s otevřeným
    /// šuplíkem se neukáže. Výchozí true.
    pub toast: bool,
    /// Když časovač běží, ukáže ho proužek i bez položky `timer` v `strip.items` (na začátku). Výchozí true.
    pub strip: bool,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self { presets: vec![5, 15, 25, 50], toast: true, strip: true }
    }
}

pub const MAX_PRESETS: usize = 6;

pub fn validate(t: &TimerConfig) -> Result<(), FieldError> {
    if t.presets.len() > MAX_PRESETS {
        return Err(fe("timer.presets", trf!("timer.presets: nejvýš {MAX_PRESETS} předvoleb.", "timer.presets: at most {MAX_PRESETS} presets.")));
    }
    if let Some(m) = t.presets.iter().find(|m| !(1..=MAX_MIN).contains(m)) {
        return Err(fe("timer.presets", trf!("timer.presets: {m} min — povoleno 1–{MAX_MIN}.", "timer.presets: {m} min — allowed 1–{MAX_MIN}.")));
    }
    Ok(())
}

// ---------- stav (čistá logika) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Countdown {
    /// běží: konec v ms epochy, celková délka (ms) pro průběh
    Running { end: i64, total: i64 },
    /// pozastavený: kolik zbývá
    Paused { left: i64, total: i64 },
    /// doběhl v `at` (ukáže se „hotovo“ do `DONE_SHOW_MS`)
    Done { at: i64, total: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Stopwatch {
    /// běží: virtuální start (teď − naměřeno)
    Running { start: i64 },
    /// pozastavené: naměřeno ms
    Paused { elapsed: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TimerState {
    pub countdown: Option<Countdown>,
    pub stopwatch: Option<Stopwatch>,
}

/// Akce z UI / palety (`timer` příkaz, tvar `{"op": "start", "minutes": 25}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    Start { minutes: u32 },
    Pause,
    Resume,
    Cancel,
    Add { minutes: u32 },
    SwStart,
    SwPause,
    SwReset,
}

fn clamp_min(m: u32) -> i64 {
    m.clamp(1, MAX_MIN) as i64 * MIN
}

/// Nový stav po akci v čase `now` (ms). Akce, která na stav nesedí (pauza zastaveného…), nic nemění.
pub fn apply(s: TimerState, op: Op, now: i64) -> TimerState {
    use Countdown as C;
    use Stopwatch as W;
    let cap = MAX_MIN as i64 * MIN;
    let mut s = s;
    match op {
        Op::Start { minutes } => {
            let total = clamp_min(minutes);
            s.countdown = Some(C::Running { end: now + total, total });
        }
        Op::Pause => {
            if let Some(C::Running { end, total }) = s.countdown {
                s.countdown = Some(C::Paused { left: (end - now).max(0), total });
            }
        }
        Op::Resume => {
            if let Some(C::Paused { left, total }) = s.countdown {
                s.countdown = Some(C::Running { end: now + left, total });
            }
        }
        Op::Cancel => s.countdown = None,
        Op::Add { minutes } => {
            let add = clamp_min(minutes);
            s.countdown = match s.countdown {
                Some(C::Running { end, total }) => {
                    let left = ((end - now).max(0) + add).min(cap);
                    Some(C::Running { end: now + left, total: (total + add).min(cap).max(left) })
                }
                Some(C::Paused { left, total }) => {
                    let left = (left + add).min(cap);
                    Some(C::Paused { left, total: (total + add).min(cap).max(left) })
                }
                // po doběhnutí „+5 min“ = nový krátký odpočet
                Some(C::Done { .. }) | None => Some(C::Running { end: now + add, total: add }),
            };
        }
        Op::SwStart => {
            s.stopwatch = match s.stopwatch {
                Some(W::Paused { elapsed }) => Some(W::Running { start: now - elapsed }),
                Some(run @ W::Running { .. }) => Some(run),
                None => Some(W::Running { start: now }),
            };
        }
        Op::SwPause => {
            if let Some(W::Running { start }) = s.stopwatch {
                s.stopwatch = Some(W::Paused { elapsed: (now - start).max(0) });
            }
        }
        Op::SwReset => s.stopwatch = None,
    }
    s
}

/// Průchod v čase `now`: doběhnutý odpočet → `Done`, staré „hotovo“ pryč. Vrací nový stav a jestli má
/// konec ohlásit (puls, toast) — jen když doběhl právě teď (ne o víc než `DONE_SHOW_MS`, např. po spánku PC).
pub fn tick(s: TimerState, now: i64) -> (TimerState, bool) {
    let mut s = s;
    let mut fired = false;
    match s.countdown {
        Some(Countdown::Running { end, total }) if end <= now => {
            fired = now - end < DONE_SHOW_MS;
            s.countdown = (now - end < DONE_SHOW_MS).then_some(Countdown::Done { at: end, total });
        }
        Some(Countdown::Done { at, .. }) if now - at >= DONE_SHOW_MS => s.countdown = None,
        _ => {}
    }
    (s, fired)
}

/// Kdy se má `emit` probudit kvůli časovači (ms epochy): konec odpočtu a každá změna zobrazené minuty,
/// konec ukázky „hotovo“, změna minuty stopek. Pozastavený / nic = `None` (v klidu žádné buzení).
pub fn next_deadline(s: &TimerState, now: i64) -> Option<i64> {
    // další okamžik, kdy `x` (ms, roste/klesá s časem) překročí násobek minuty
    let cd = match s.countdown {
        Some(Countdown::Running { end, .. }) => {
            let left = (end - now).max(0);
            let r = left % MIN;
            Some(now + if r == 0 { MIN.min(left.max(1)) } else { r })
        }
        Some(Countdown::Done { at, .. }) => Some(at + DONE_SHOW_MS),
        _ => None,
    };
    let sw = match s.stopwatch {
        Some(Stopwatch::Running { start }) => Some(now + MIN - (now - start).max(0) % MIN),
        _ => None,
    };
    [cd, sw].into_iter().flatten().min().map(|t| t + 1)
}

/// Zbývá z odpočtu (ms); doběhlý = 0.
pub fn left_ms(c: &Countdown, now: i64) -> i64 {
    match *c {
        Countdown::Running { end, .. } => (end - now).max(0),
        Countdown::Paused { left, .. } => left,
        Countdown::Done { .. } => 0,
    }
}

/// Naměřeno stopkami (ms).
pub fn elapsed_ms(w: &Stopwatch, now: i64) -> i64 {
    match *w {
        Stopwatch::Running { start } => (now - start).max(0),
        Stopwatch::Paused { elapsed } => elapsed,
    }
}

/// Zbývající čas po minutách nahoru („18 min“, „1 h 5 min“) — 17:30 zbývá = 18 min, 0:20 = 1 min.
pub fn minutes_text(left_ms: i64) -> String {
    let m = (left_ms.max(0) + MIN - 1) / MIN;
    if m < 60 {
        format!("{m} min")
    } else if m % 60 == 0 {
        format!("{} h", m / 60)
    } else {
        format!("{} h {} min", m / 60, m % 60)
    }
}

/// Naměřený čas stopek po minutách dolů jako h:mm („0:42“, „1:05“).
pub fn clock_text(elapsed_ms: i64) -> String {
    let m = elapsed_ms.max(0) / MIN;
    format!("{}:{:02}", m / 60, m % 60)
}

/// Segmenty proužku: odpočet, pak stopky; nic neběží = prázdné. Pauza = ⏸ místo ⏱.
pub fn strip_segments(s: &TimerState, now: i64) -> Vec<Segment> {
    let mut out = Vec::new();
    let mk = |text: String, dot: Option<Rgba>, pill: bool| Segment { text, color: Rgba::TEXT, dot, strong: false, pill };
    let icon = |running: bool| if running { "⏱" } else { "⏸" };
    match &s.countdown {
        Some(Countdown::Done { .. }) => out.push(mk(format!("⏱ {}", tr("čas vypršel", "time's up")), Some(COLOR), true)),
        Some(c) => out.push(mk(format!("{} {}", icon(matches!(c, Countdown::Running { .. })), minutes_text(left_ms(c, now))), None, false)),
        None => {}
    }
    if let Some(w) = &s.stopwatch {
        out.push(mk(format!("{} {}", icon(matches!(w, Stopwatch::Running { .. })), clock_text(elapsed_ms(w, now))), None, false));
    }
    out
}

/// Běží (nebo je pozastavený / právě doběhl) odpočet či stopky — pro automatickou položku proužku.
pub fn active(s: &TimerState) -> bool {
    s.countdown.is_some() || s.stopwatch.is_some()
}

/// Texty toastu po doběhnutí (titulek, tělo s délkou).
pub fn toast_text(total_ms: i64) -> (String, String) {
    (tr("Časovač doběhl", "Timer finished").to_string(), trf!("Odpočet {} skončil.", "The {} countdown is over.", minutes_text(total_ms)))
}

// ---------- soubor ----------

fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Stav ze souboru (chybějící / rozbitý = nic neběží). Odpočet, který doběhl mezitím, se nehlásí.
pub(crate) fn load(dir: &Path, now: i64) -> TimerState {
    let s: TimerState = std::fs::read(path_in(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    tick(s, now).0
}

/// Zápis stavu (nic neběží = soubor pryč). Zápis přes dočasný soubor, ať ho výpadek nerozbije.
pub(crate) fn save(dir: &Path, s: &TimerState) -> std::io::Result<()> {
    let p = path_in(dir);
    if !active(s) {
        return match std::fs::remove_file(&p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(s).unwrap_or_default())?;
    std::fs::rename(&tmp, &p)
}

// ---------- napojení na jádro ----------

static STATE: Mutex<TimerState> = Mutex::new(TimerState { countdown: None, stopwatch: None });

/// Při startu (`lib.rs`): stav z `timer.json`.
pub(crate) fn init(dir: &Path) {
    *STATE.lock() = load(dir, crate::state::now_ms());
}

/// Stav pro snapshot a proužek (jen paměť).
pub(crate) fn state() -> TimerState {
    *STATE.lock()
}

/// Pro snapshot: stav + předvolby z configu (UI počítá sekundy samo z časů).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerView {
    #[serde(flatten)]
    pub state: TimerState,
    pub presets: Vec<u32>,
}

pub(crate) fn view(cfg: &TimerConfig) -> TimerView {
    TimerView { state: state(), presets: cfg.presets.clone() }
}

/// Akce uživatele: nový stav, zápis souboru, překreslení proužku a šuplíku.
pub(crate) fn act(op: Op) -> TimerState {
    let s = {
        let mut g = STATE.lock();
        *g = apply(*g, op, crate::state::now_ms());
        *g
    };
    if let Err(e) = save(&crate::config::config_dir(), &s) {
        crate::state::log(&format!("timer: {e}"));
    }
    crate::state::mark_dirty();
    s
}

/// Průchod vlákna `emit`: doběhnutý odpočet ohlásí (puls + volitelný toast) a uloží.
pub(crate) fn tick_live(now: i64) {
    let (s, fired, changed) = {
        let mut g = STATE.lock();
        let (s, fired) = tick(*g, now);
        let changed = s != *g;
        *g = s;
        (s, fired, changed)
    };
    if changed && let Err(e) = save(&crate::config::config_dir(), &s) {
        crate::state::log(&format!("timer: {e}"));
    }
    if !fired {
        return;
    }
    let c = crate::state::core();
    let toast_on = c.cfg.read().timer.toast;
    if crate::attention::signal(COLOR, toast_on) {
        let total = match s.countdown {
            Some(Countdown::Done { total, .. }) => total,
            _ => 0,
        };
        let (title, body) = toast_text(total);
        crate::toast::show(&c.app.config().identifier, title, body);
    }
    crate::state::mark_dirty();
}

/// Termín pro `threads::emit::deadlines`.
pub(crate) fn next_deadline_live() -> Option<i64> {
    next_deadline(&STATE.lock(), crate::state::now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;

    const T: i64 = 1_760_000_000_000;

    /// (název, akce v čase od T, očekávaný stav)
    type Case<S> = (&'static str, Vec<(i64, Op)>, Option<S>);

    fn run(ops: &[(i64, Op)]) -> TimerState {
        ops.iter().fold(TimerState::default(), |s, (at, op)| apply(s, *op, T + at))
    }

    #[test]
    fn countdown_ops_table() {
        use Countdown::*;
        use Op::*;
        let m = MIN;
        let cases: Vec<Case<Countdown>> = vec![
            ("start 25", vec![(0, Start { minutes: 25 })], Some(Running { end: T + 25 * m, total: 25 * m })),
            ("0 min = 1 min", vec![(0, Start { minutes: 0 })], Some(Running { end: T + m, total: m })),
            ("strop 600 min", vec![(0, Start { minutes: 9999 })], Some(Running { end: T + 600 * m, total: 600 * m })),
            ("pauza po 10 min", vec![(0, Start { minutes: 25 }), (10 * m, Pause)], Some(Paused { left: 15 * m, total: 25 * m })),
            (
                "pauza, za hodinu pokračovat",
                vec![(0, Start { minutes: 25 }), (10 * m, Pause), (70 * m, Resume)],
                Some(Running { end: T + 85 * m, total: 25 * m }),
            ),
            ("pauza zastaveného nic", vec![(0, Pause)], None),
            ("pokračovat běžícího nic", vec![(0, Start { minutes: 5 }), (m, Resume)], Some(Running { end: T + 5 * m, total: 5 * m })),
            ("zrušit", vec![(0, Start { minutes: 5 }), (m, Cancel)], None),
            ("+5 běžícímu", vec![(0, Start { minutes: 25 }), (5 * m, Add { minutes: 5 })], Some(Running { end: T + 30 * m, total: 30 * m })),
            ("+5 pozastavenému", vec![(0, Start { minutes: 25 }), (5 * m, Pause), (6 * m, Add { minutes: 5 })], Some(Paused { left: 25 * m, total: 30 * m })),
            ("+5 bez odpočtu = nový", vec![(m, Add { minutes: 5 })], Some(Running { end: T + 6 * m, total: 5 * m })),
            ("+5 nad strop", vec![(0, Start { minutes: 598 }), (0, Add { minutes: 5 })], Some(Running { end: T + 600 * m, total: 600 * m })),
            ("nový start přepíše", vec![(0, Start { minutes: 25 }), (m, Start { minutes: 5 })], Some(Running { end: T + 6 * m, total: 5 * m })),
        ];
        for (name, ops, want) in cases {
            assert_eq!(run(&ops).countdown, want, "{name}");
        }
    }

    #[test]
    fn stopwatch_ops_table() {
        use Op::*;
        use Stopwatch::*;
        let s = 1_000;
        let cases: Vec<Case<Stopwatch>> = vec![
            ("start", vec![(0, SwStart)], Some(Running { start: T })),
            ("pauza po 42 s", vec![(0, SwStart), (42 * s, SwPause)], Some(Paused { elapsed: 42 * s })),
            ("pokračovat pokračuje od naměřeného", vec![(0, SwStart), (42 * s, SwPause), (100 * s, SwStart)], Some(Running { start: T + 58 * s })),
            ("start běžících nic", vec![(0, SwStart), (5 * s, SwStart)], Some(Running { start: T })),
            ("reset", vec![(0, SwStart), (5 * s, SwReset)], None),
            ("pauza zastavených nic", vec![(0, SwPause)], None),
        ];
        for (name, ops, want) in cases {
            assert_eq!(run(&ops).stopwatch, want, "{name}");
        }
        // odpočet a stopky jsou nezávislé
        let both = run(&[(0, Op::Start { minutes: 5 }), (0, SwStart), (s, Op::Cancel)]);
        assert_eq!((both.countdown, both.stopwatch), (None, Some(Running { start: T })));
    }

    #[test]
    fn remaining_and_elapsed() {
        let run = Countdown::Running { end: T + 90_000, total: 300_000 };
        assert_eq!(left_ms(&run, T), 90_000);
        assert_eq!(left_ms(&run, T + 100_000), 0, "po konci nula, ne záporné");
        assert_eq!(left_ms(&Countdown::Paused { left: 5, total: 9 }, T + 1_000_000), 5, "pauza stojí");
        assert_eq!(elapsed_ms(&Stopwatch::Running { start: T - 42_000 }, T), 42_000);
        assert_eq!(elapsed_ms(&Stopwatch::Paused { elapsed: 7 }, T + 99), 7);
    }

    #[test]
    fn tick_table() {
        let run = |end: i64| TimerState { countdown: Some(Countdown::Running { end, total: 5 * MIN }), stopwatch: None };
        let done = |at: i64| TimerState { countdown: Some(Countdown::Done { at, total: 5 * MIN }), stopwatch: None };
        let cases: [(&str, TimerState, i64, TimerState, bool); 6] = [
            ("ještě běží", run(T + 1), T, run(T + 1), false),
            ("právě doběhl → ohlásit", run(T), T + 1, done(T), true),
            ("doběhl před 9 min (probuzení) → ohlásit", run(T), T + 9 * MIN, done(T), true),
            ("doběhl před 10 min → tiše pryč", run(T), T + DONE_SHOW_MS, TimerState::default(), false),
            ("hotovo se ukazuje", done(T), T + DONE_SHOW_MS - 1, done(T), false),
            ("hotovo po 10 min zmizí", done(T), T + DONE_SHOW_MS, TimerState::default(), false),
        ];
        for (name, s, now, want, fired) in cases {
            assert_eq!(tick(s, now), (want, fired), "{name}");
        }
    }

    #[test]
    fn deadlines_only_while_running() {
        let st = |c: Option<Countdown>, w: Option<Stopwatch>| TimerState { countdown: c, stopwatch: w };
        let cases: [(&str, TimerState, Option<i64>); 8] = [
            ("nic → žádné buzení", st(None, None), None),
            ("pozastavené → žádné", st(Some(Countdown::Paused { left: 5 * MIN, total: 9 }), Some(Stopwatch::Paused { elapsed: 5 })), None),
            ("zbývá 17:30 → za 30 s (18 → 17 min)", st(Some(Countdown::Running { end: T + 17 * MIN + 30_000, total: 0 }), None), Some(T + 30_000 + 1)),
            ("zbývá přesně 3 min → za minutu", st(Some(Countdown::Running { end: T + 3 * MIN, total: 0 }), None), Some(T + MIN + 1)),
            ("zbývá 20 s → konec", st(Some(Countdown::Running { end: T + 20_000, total: 0 }), None), Some(T + 20_000 + 1)),
            ("hotovo → konec ukázky", st(Some(Countdown::Done { at: T - MIN, total: 0 }), None), Some(T - MIN + DONE_SHOW_MS + 1)),
            ("stopky 0:42,5 → za 17,5 s", st(None, Some(Stopwatch::Running { start: T - 42_500 })), Some(T + 17_500 + 1)),
            (
                "obojí → dřívější",
                st(Some(Countdown::Running { end: T + 50_000, total: 0 }), Some(Stopwatch::Running { start: T - 30_000 })),
                Some(T + 30_000 + 1),
            ),
        ];
        for (name, s, want) in cases {
            assert_eq!(next_deadline(&s, T), want, "{name}");
        }
        // hodina odpočtu = ~60 probuzení navíc, ne 3 600
        let mut s = apply(TimerState::default(), Op::Start { minutes: 60 }, T);
        let (mut now, mut wakes) = (T, 0);
        while let Some(d) = next_deadline(&s, now) {
            now = d;
            s = tick(s, now).0;
            wakes += 1;
            assert!(wakes < 100, "smyčka");
        }
        assert!((60..=63).contains(&wakes), "{wakes} probuzení");
    }

    #[test]
    fn texts_table() {
        let cases: [(i64, &str); 7] =
            [(0, "0 min"), (1, "1 min"), (20_000, "1 min"), (60_000, "1 min"), (17 * MIN + 30_000, "18 min"), (60 * MIN, "1 h"), (65 * MIN, "1 h 5 min")];
        for (ms, want) in cases {
            assert_eq!(minutes_text(ms), want, "{ms}");
        }
        let cases: [(i64, &str); 4] = [(0, "0:00"), (42 * MIN + 59_000, "0:42"), (65 * MIN, "1:05"), (-5, "0:00")];
        for (ms, want) in cases {
            assert_eq!(clock_text(ms), want, "{ms}");
        }
    }

    #[test]
    fn strip_segments_table() {
        let texts = |s: &TimerState, now: i64| strip_segments(s, now).into_iter().map(|x| x.text).collect::<Vec<_>>();
        let s = run(&[(0, Op::Start { minutes: 25 })]);
        assert_eq!(texts(&s, T + 7 * MIN), vec!["⏱ 18 min"]);
        let s = apply(s, Op::Pause, T + 7 * MIN);
        assert_eq!(texts(&s, T + 99 * MIN), vec!["⏸ 18 min"]);
        let s = apply(s, Op::SwStart, T);
        assert_eq!(texts(&s, T + 42 * MIN + 10_000), vec!["⏸ 18 min", "⏱ 0:42"]);
        let done = TimerState { countdown: Some(Countdown::Done { at: T, total: MIN }), stopwatch: None };
        assert_eq!(texts(&done, T), vec!["⏱ čas vypršel"]);
        assert_eq!(with_lang(Lang::En, || texts(&done, T)), vec!["⏱ time's up"]);
        assert!(strip_segments(&TimerState::default(), T).is_empty());
        assert!(!active(&TimerState::default()) && active(&done));
        assert_eq!(toast_text(25 * MIN).1, "Odpočet 25 min skončil.");
    }

    #[test]
    fn op_json_shape() {
        // tvar, který posílá UI (`api.timer`)
        let cases: [(&str, Op); 4] = [
            (r#"{"op":"start","minutes":25}"#, Op::Start { minutes: 25 }),
            (r#"{"op":"add","minutes":5}"#, Op::Add { minutes: 5 }),
            (r#"{"op":"swStart"}"#, Op::SwStart),
            (r#"{"op":"cancel"}"#, Op::Cancel),
        ];
        for (json, want) in cases {
            assert_eq!(serde_json::from_str::<Op>(json).unwrap(), want, "{json}");
        }
        let v = serde_json::to_value(TimerView { state: run(&[(0, Op::Start { minutes: 1 })]), presets: vec![5] }).unwrap();
        assert_eq!(v["countdown"]["state"], "running");
        assert_eq!(v["countdown"]["end"], T + MIN);
        assert!(v["stopwatch"].is_null());
        assert_eq!(v["presets"][0], 5);
    }

    #[test]
    fn config_defaults_and_validation() {
        let c: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert_eq!(c.timer, TimerConfig::default());
        assert_eq!(c.timer.presets, vec![5, 15, 25, 50]);
        assert!(c.timer.toast && c.timer.strip);
        let c: crate::config::Config = serde_json::from_str(r#"{"timer":{"toast":false}}"#).unwrap();
        assert_eq!((c.timer.toast, c.timer.presets.len()), (false, 4), "částečný blok");
        let cases: [(Vec<u32>, bool); 5] = [(vec![], true), (vec![1, 600], true), (vec![0], false), (vec![601], false), (vec![1; 7], false)];
        for (presets, ok) in cases {
            let t = TimerConfig { presets: presets.clone(), ..TimerConfig::default() };
            assert_eq!(validate(&t).is_ok(), ok, "{presets:?}");
        }
    }

    #[test]
    fn persists_in_dir() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load(d.path(), T), TimerState::default(), "chybí soubor");
        let s = run(&[(0, Op::Start { minutes: 25 }), (0, Op::SwStart)]);
        save(d.path(), &s).unwrap();
        assert_eq!(load(d.path(), T + MIN), s, "běžící přežije restart");
        // doběhl, zatímco Ledge neběžel: „hotovo“ jen do DONE_SHOW_MS, pak nic
        assert_eq!(load(d.path(), T + 26 * MIN).countdown, Some(Countdown::Done { at: T + 25 * MIN, total: 25 * MIN }));
        assert_eq!(load(d.path(), T + 25 * MIN + DONE_SHOW_MS).countdown, None);
        // nic neběží → soubor pryč; rozbitý soubor = nic
        save(d.path(), &TimerState::default()).unwrap();
        assert!(!d.path().join(FILE).exists());
        save(d.path(), &TimerState::default()).unwrap();
        std::fs::write(d.path().join(FILE), "nesmysl").unwrap();
        assert_eq!(load(d.path(), T), TimerState::default());
    }
}
