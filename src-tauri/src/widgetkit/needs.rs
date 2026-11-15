//! Které skupiny dat kit widget potřebuje (plocha podle toho plní `DeskData` okna kitu — jinak by měl
//! kit na ploše jen hodiny). Čistá funkce nad ověřenou šablonou; jména zdrojů zrcadlí `SOURCES` v
//! `src/widgetkit/sources.ts` (hlídá test níže), neznámý zdroj se ignoruje (UI řádek vynechá s poznámkou).

use super::spec::{Acts, Spec};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Needs {
    pub limits: bool,
    pub waiting: bool,
    pub procs: bool,
    pub calendar: bool,
    pub costs: bool,
}

impl Needs {
    pub fn merge(&mut self, o: Needs) {
        self.limits |= o.limits;
        self.waiting |= o.waiting;
        self.procs |= o.procs;
        self.calendar |= o.calendar;
        self.costs |= o.costs;
    }

    fn add_source(&mut self, src: &str) {
        match src {
            "limits.five.pct" | "limits.five.resetMin" | "limits.week.pct" | "limits.week.resetMin" => self.limits = true,
            "waiting.count" => self.waiting = true,
            "procs.count" => self.procs = true,
            "costs.today" | "costs.week" => self.costs = true,
            "calendar.next.title" | "calendar.next.inMin" => self.calendar = true,
            _ => {}
        }
    }

    /// Zdroje z `{zdroj}` / `{zdroj|formát}` v textu (syntaxi už ověřil `spec::template_ok`).
    fn add_template(&mut self, text: &str) {
        for part in text.split('{').skip(1) {
            if let Some((inner, _)) = part.split_once('}') {
                self.add_source(inner.split('|').next().unwrap_or(""));
            }
        }
    }

    fn add_acts(&mut self, a: &Acts) {
        if let Some(c) = &a.copy {
            self.add_template(c);
        }
    }
}

/// Zdroje použité v řádcích (`text`, `bar`, `copy`) a tlačítkách (`copy`) šablony.
pub fn needs(spec: &Spec) -> Needs {
    let mut n = Needs::default();
    for r in &spec.rows {
        if let Some(t) = &r.text {
            n.add_template(t);
        }
        if let Some(b) = &r.bar {
            n.add_source(b);
        }
        n.add_acts(&r.act);
    }
    for b in &spec.buttons {
        n.add_acts(&b.act);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgetkit::spec::parse;

    fn of(json: &str) -> Needs {
        needs(&parse(json).expect("platná šablona"))
    }
    fn flags(n: Needs) -> [bool; 5] {
        [n.limits, n.waiting, n.procs, n.calendar, n.costs]
    }

    #[test]
    fn needs_table() {
        let cases: [(&str, [bool; 5]); 8] = [
            (r#"{"kit":1,"title":"t","rows":[{"text":"{clock.hm}"}]}"#, [false; 5]),
            (r#"{"kit":1,"title":"t","rows":[{"bar":"limits.five.pct"}]}"#, [true, false, false, false, false]),
            (r#"{"kit":1,"title":"t","rows":[{"text":"Čeká {waiting.count} a {procs.count|pct}"}]}"#, [false, true, true, false, false]),
            (r#"{"kit":1,"title":"t","rows":[{"text":"x","copy":"{costs.today|usd}"}]}"#, [false, false, false, false, true]),
            (r#"{"kit":1,"title":"t","rows":[{"text":"x"}],"buttons":[{"label":"b","copy":"{calendar.next.title}"}]}"#, [false, false, false, true, false]),
            (r#"{"kit":1,"title":"t","rows":[{"text":"{neznamy.zdroj} {limits}"},{"bar":"nic.takoveho"}]}"#, [false; 5]),
            (r#"{"kit":1,"title":"t","rows":[{"text":"{limits.week.resetMin}"},{"text":"{calendar.next.inMin}"}]}"#, [true, false, false, true, false]),
            (r#"{"kit":1,"title":"t","rows":[{"bar":"costs.week"},{"text":"{waiting.count}"}]}"#, [false, true, false, false, true]),
        ];
        for (json, want) in cases {
            assert_eq!(flags(of(json)), want, "{json}");
        }
    }

    #[test]
    fn merge_unions() {
        let mut a = of(r#"{"kit":1,"title":"t","rows":[{"bar":"limits.five.pct"}]}"#);
        a.merge(of(r#"{"kit":1,"title":"t","rows":[{"text":"{procs.count}"}]}"#));
        assert_eq!(flags(a), [true, false, true, false, false]);
    }

    /// Každý zdroj z `SOURCES` (UI) kromě hodin musí být zařazen, jinak by na ploše zůstal prázdný.
    #[test]
    fn every_ui_source_is_classified() {
        let ts = include_str!("../../../src/widgetkit/sources.ts");
        let body = ts.split("export const SOURCES").nth(1).expect("SOURCES");
        let keys: Vec<&str> = body.lines().filter_map(|l| l.trim_start().strip_prefix('"')?.split('"').next()).filter(|k| k.contains('.')).collect();
        assert!(keys.len() >= 12, "{keys:?}");
        for k in keys {
            let mut n = Needs::default();
            n.add_source(k);
            assert!(k.starts_with("clock.") || n != Needs::default(), "zdroj {k} není v needs.rs");
        }
    }
}
