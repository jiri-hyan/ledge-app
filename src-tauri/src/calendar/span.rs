//! Délka schůzky pro zavřený proužek (L85, kolo 33): „za 8 min“ do `STRIP_BEFORE_MIN` před začátkem a
//! „zbývá 12 min“ během schůzky. Čistá funkce, zrcadlo TS `meetingSpan` (`src/calendar/model.ts`) —
//! tabulka hranic je v obou testech stejná. Žádný časovač: proužek se přepočítá s minutovými hodinami emit
//! vlákna (`threads/emit.rs` `next_wake`).

use super::Occ;
use crate::i18n::tr;

/// Před začátkem se v proužku ukazuje „za N min“ nejdřív tolik minut předem.
pub const STRIP_BEFORE_MIN: i64 = 10;

/// Délky v celých minutách (zbývá zaokrouhleno nahoru, aspoň 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Before { starts_in: i64, total: i64 },
    Running { run: i64, left: i64, total: i64 },
}

/// Celodenní, nulová a skončená schůzka = `None`; `Before` jen do `within` minut před začátkem.
/// Konec přes půlnoc nevadí (absolutní ms).
pub fn meeting_span(start: i64, end: i64, all_day: bool, now: i64, within: i64) -> Option<Span> {
    if all_day || end <= start || now >= end {
        return None;
    }
    let total = (end - start + 30_000).div_euclid(60_000);
    if now < start {
        let starts_in = (start - now + 59_999) / 60_000;
        return (starts_in <= within).then_some(Span::Before { starts_in, total });
    }
    Some(Span::Running { run: (now - start) / 60_000, left: ((end - now + 59_999) / 60_000).max(1), total })
}

/// Minuty čitelně: „45 min“, „2 h“, „2 h 10 min“ (jako TS `minLabel`).
pub fn min_label(n: i64) -> String {
    let m = n.max(0);
    match (m / 60, m % 60) {
        (0, r) => format!("{r} min"),
        (h, 0) => format!("{h} h"),
        (h, r) => format!("{h} h {r} min"),
    }
}

/// Schůzka pro proužek: běžící dřív než nadcházející, mezi běžícími ta, co skončí první, mezi nadcházejícími
/// ta, co začne první. Titulek zkracuje volající (`remind::short`).
pub fn pick<'a>(occs: impl IntoIterator<Item = &'a Occ>, now: i64) -> Option<(&'a Occ, Span)> {
    occs.into_iter()
        .filter_map(|o| meeting_span(o.start, o.end, o.all_day, now, STRIP_BEFORE_MIN).map(|s| (o, s)))
        .min_by_key(|(o, s)| match s {
            Span::Running { .. } => (0, o.end),
            Span::Before { .. } => (1, o.start),
        })
}

/// Text za titulkem: „zbývá 12 min“ / „za 8 min“.
pub fn tail(s: Span) -> String {
    match s {
        Span::Running { left, .. } => format!("{} {}", tr("zbývá", "left"), min_label(left)),
        Span::Before { starts_in, .. } => format!("{} {}", tr("za", "in"), min_label(starts_in)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: i64 = 60_000;
    const H: i64 = 60 * M;

    /// Stejná tabulka jako `src/calendar/model.test.mjs` („délka schůzky“).
    #[test]
    fn span_table_mirrors_ts() {
        let b = |starts_in, total| Some(Span::Before { starts_in, total });
        let r = |run, left, total| Some(Span::Running { run, left, total });
        // (název, start, end, celodenní, now, čekáno)
        let cases = [
            ("za 8 min na 30 min", 8 * M, 38 * M, false, 0, b(8, 30)),
            ("zaokrouhleno nahoru", 5 * M, 35 * M, false, 1, b(5, 30)),
            ("přesně hodinu před", 60 * M, 90 * M, false, 0, b(60, 30)),
            ("hodinu a minutu před = nic", 61 * M, 90 * M, false, 0, None),
            ("začíná právě teď = běží", 0, 30 * M, false, 0, r(0, 30, 30)),
            ("běží 35 zbývá 12", 0, 47 * M, false, 35 * M, r(35, 12, 47)),
            ("zbývá zaokrouhleno nahoru, aspoň 1", 0, 30 * M, false, 30 * M - 1, r(29, 1, 30)),
            ("konec = skončila", 0, 30 * M, false, 30 * M, None),
            ("nulová délka", 5 * M, 5 * M, false, 0, None),
            ("celodenní", 0, 24 * H, true, 0, None),
            ("přes půlnoc", 23 * H, 25 * H, false, 24 * H + 30 * M, r(90, 30, 120)),
        ];
        for (name, s, e, all_day, now, want) in cases {
            assert_eq!(meeting_span(s, e, all_day, now, 60), want, "{name}");
        }
        assert!(matches!(meeting_span(120 * M, 150 * M, false, 0, 180), Some(Span::Before { .. })), "vlastní okno");
    }

    #[test]
    fn min_label_table() {
        let got: Vec<String> = [0, 45, 60, 90, 130, 120].into_iter().map(min_label).collect();
        assert_eq!(got, ["0 min", "45 min", "1 h", "1 h 30 min", "2 h 10 min", "2 h"]);
    }

    fn occ(title: &str, start: i64, end: i64) -> Occ {
        Occ { start, end, title: title.into(), ..Occ::default() }
    }

    #[test]
    fn pick_prefers_running_then_earliest() {
        let (run_late, run_soon, next, far) = (occ("A", -10 * M, 50 * M), occ("B", -10 * M, 20 * M), occ("C", 3 * M, 30 * M), occ("D", 11 * M, 40 * M));
        let title = |v: &[&Occ]| pick(v.iter().copied(), 0).map(|(o, _)| o.title.clone());
        assert_eq!(title(&[&next, &run_late, &run_soon]), Some("B".into()), "běžící, co skončí první");
        assert_eq!(title(&[&far, &next]), Some("C".into()), "nadcházející do 10 min");
        assert_eq!(title(&[&far]), None, "11 min předem = nic");
        assert_eq!(title(&[]), None);
    }

    #[test]
    fn tail_texts() {
        assert_eq!(tail(Span::Running { run: 35, left: 12, total: 47 }), "zbývá 12 min");
        assert_eq!(tail(Span::Before { starts_in: 8, total: 30 }), "za 8 min");
        assert_eq!(tail(Span::Running { run: 1, left: 90, total: 91 }), "zbývá 1 h 30 min");
    }
}
