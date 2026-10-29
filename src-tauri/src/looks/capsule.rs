//! Kapsle zavřeného Ledge z kola 22 (čisté funkce, kreslí `stripdraw`): `minimum` = baterie a čas,
//! `trio` = počasí | čas a datum | CPU, paměť, baterie, `rings` = stav Clauda (prstence limitů 5h / týden
//! a čas; točí se jen při práci, oranžově = čeká na tebe). Data jen z `Facts` — co chybí, se nekreslí
//! (žádné vymyšlené procento: bez limitu je prstenec jen prázdná dráha).

use super::{Facts, Glyph, OK, Part, RUN, Show, State, WAIT, done_text, icon, state_of, strong, text, wait_lead, wait_text};
use crate::looks::{IDLE, Look};
use crate::strip::Rgba;

/// Oranžová prstenců „čeká na tebe“ (odlišná od jantaru limitu ≥ 80 %); obrys těla pulzuje přes `attention::pulse`.
pub const CALL: Rgba = Rgba(1.0, 0.56, 0.2, 1.0);

/// Od kolika % je prstenec limitu jantarový / červený (jako upozornění na limit).
pub const WARN_PCT: f64 = 80.0;
pub const FULL_PCT: f64 = 100.0;

/// „42 %“ (CPU, paměť, baterie).
use crate::util::pct_text as pct;

/// Barva prstence podle čerpání limitu (akcent → jantar od 80 % → červená od 100 %).
pub fn ring_color(p: f64, accent: Rgba) -> Rgba {
    if p >= FULL_PCT {
        Rgba::RED
    } else if p >= WARN_PCT {
        Rgba::AMBER
    } else {
        accent
    }
}

/// Den a datum z `Facts.date` („14:32 · ne 18. 10.“ → „ne 18. 10.“); bez času celé.
pub fn day_of(date: &str) -> &str {
    date.split_once(" · ").map(|(_, d)| d).unwrap_or(date)
}

/// WMO kód počasí → ikona (jasno a skoro jasno = slunce / měsíc, jinak mrak).
pub fn weather_glyph(code: i64, is_day: bool) -> Glyph {
    match (code, is_day) {
        (0 | 1, true) => Glyph::Sun,
        (0 | 1, false) => Glyph::Moon,
        _ => Glyph::Cloud,
    }
}

/// Baterie: tvar + procenta (nic, když stroj baterii nemá nebo ji uživatel skryl).
fn battery(f: &Facts) -> Vec<Part> {
    f.battery
        .map(|b| vec![Part::Battery { percent: b.percent, charging: b.charging }, text(pct(b.percent as f64), Rgba::TEXT)])
        .unwrap_or_default()
}

/// Tečka / ikona stavu před časem (klid a hudba = nic).
fn lead(st: State, f: &Facts) -> Option<Part> {
    match st {
        State::Wait => Some(wait_lead(f)),
        State::Done => Some(Part::Dot(OK)),
        State::Work => Some(Part::Dot(RUN)),
        State::Timer => Some(icon(Glyph::Timer, f.accent)),
        State::Music | State::Idle => None,
    }
}

/// Skupiny vedle sebe, mezi neprázdnými oddělovač.
fn joined(groups: Vec<Vec<Part>>) -> Vec<Part> {
    let mut out = Vec::new();
    for g in groups.into_iter().filter(|g| !g.is_empty()) {
        if !out.is_empty() {
            out.push(Part::Sep);
        }
        out.extend(g);
    }
    out
}

/// Obsah kapslí `minimum`, `trio`, `rings` (volá `looks::show`).
pub fn fill(look: Look, f: &Facts, s: &mut Show) {
    let st = state_of(f);
    let clock = (!f.clock.is_empty()).then(|| strong(f.clock.clone(), Rgba::TEXT));
    match look {
        Look::Minimum => {
            s.parts.extend(lead(st, f));
            s.parts.extend(battery(f));
            s.parts.extend(clock);
        }
        Look::Trio => {
            let weather = f.weather.map(|w| vec![icon(w.glyph, Rgba::TEXT), strong(format!("{}°", w.temp), Rgba::TEXT)]).unwrap_or_default();
            let middle = match st {
                State::Wait => vec![wait_lead(f), strong(wait_text(f), WAIT)],
                State::Done => vec![icon(Glyph::Check, OK), text(done_text(f), Rgba::TEXT)],
                _ => {
                    let mut v: Vec<Part> = lead(st, f).into_iter().collect();
                    v.extend(clock);
                    let day = day_of(&f.date);
                    if !day.is_empty() {
                        v.push(text(day, Rgba::DIM));
                    }
                    v
                }
            };
            let mut system = Vec::new();
            if let Some(c) = f.cpu {
                system.push(text(format!("CPU {}", pct(c)), Rgba::TEXT));
            }
            if let Some(m) = f.mem {
                system.push(text(format!("RAM {}", pct(m)), Rgba::TEXT));
            }
            system.extend(battery(f));
            s.parts = joined(vec![weather, middle, system]);
        }
        Look::Rings => {
            let wait = st == State::Wait;
            let mut rings: Vec<Part> = [f.limit5h, f.limit7d]
                .into_iter()
                .flatten()
                .map(|p| Part::Ring { fill: Some((p / 100.0).clamp(0.0, 1.0) as f32), color: if wait { CALL } else { ring_color(p, f.accent) } })
                .collect();
            if rings.is_empty() {
                // limit neznáme: jen dráha (barva = stav), žádné číslo
                let c = if wait { CALL } else if f.working > 0 { RUN } else { IDLE };
                rings.push(Part::Ring { fill: None, color: c });
            }
            s.parts = rings;
            if st == State::Done {
                s.parts.push(icon(Glyph::Check, OK));
            }
            s.parts.extend(clock);
            s.spin = f.working > 0 && !wait;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::looks::{BatteryFact, WeatherFact, show, without};
    use crate::config::StripItem;

    fn sig(s: &Show) -> String {
        let col = |c: Rgba| match c {
            c if c == RUN => "run",
            // WAIT = Rgba::AMBER (jantar čekání i limitu ≥ 80 %)
            c if c == WAIT => "amber",
            c if c == CALL => "call",
            c if c == OK => "ok",
            c if c == IDLE => "idle",
            c if c == Rgba::VIOLET => "acc",
            c if c == Rgba::RED => "red",
            c if c == Rgba::DIM => "dim",
            c if c == Rgba::TEXT => "text",
            _ => "?",
        };
        let mut out: Vec<String> = s
            .parts
            .iter()
            .map(|p| match p {
                Part::Text { text, strong: true, .. } => format!("*{text}*"),
                Part::Text { text, .. } => text.clone(),
                Part::Dot(c) => format!("●{}", col(*c)),
                Part::Icon { glyph, color, .. } => format!("[{glyph:?}:{}]", col(*color)),
                Part::Sep => "|".into(),
                Part::Battery { percent, charging } => format!("▭{percent}{}", if *charging { "+" } else { "" }),
                Part::Ring { fill: Some(x), color } => format!("◯{:.0}:{}", x * 100.0, col(*color)),
                Part::Ring { fill: None, color } => format!("◯-:{}", col(*color)),
                Part::Meter { fill, color } => format!("▮{:.0}:{}", fill * 100.0, col(*color)),
                Part::Spark { .. } | Part::Under { .. } => "?".into(),
            })
            .collect();
        if s.spin {
            out.push("⟳".into());
        }
        out.join(" ")
    }

    fn base() -> Facts {
        Facts { clock: "14:32".into(), date: "14:32 · ne 18. 10.".into(), accent: Rgba::VIOLET, ..Facts::default() }
    }
    fn full() -> Facts {
        Facts {
            limit5h: Some(42.4),
            limit7d: Some(86.0),
            battery: Some(BatteryFact { percent: 92, charging: false }),
            weather: Some(WeatherFact { temp: 29, glyph: Glyph::Sun }),
            cpu: Some(8.2),
            mem: Some(46.0),
            ..base()
        }
    }

    #[test]
    fn table_three_capsules_x_states() {
        let work = |mut f: Facts| {
            f.working = 2;
            f
        };
        let wait = |mut f: Facts| {
            f.waiting = 1;
            f.waiting_where = "app".into();
            f
        };
        let done = |mut f: Facts| {
            f.done = 1;
            f.done_where = "app".into();
            f
        };
        // (popis, styl, fakta, podpis)
        let cases: Vec<(&str, Look, Facts, &str)> = vec![
            ("minimum klid", Look::Minimum, full(), "▭92 92 % *14:32*"),
            ("minimum bez baterie (desktop)", Look::Minimum, base(), "*14:32*"),
            ("minimum pracuje", Look::Minimum, work(full()), "●run ▭92 92 % *14:32*"),
            ("minimum čeká", Look::Minimum, wait(full()), "●amber ▭92 92 % *14:32*"),
            ("trio vše", Look::Trio, full(), "[Sun:text] *29°* | *14:32* ne 18. 10. | CPU 8 % RAM 46 % ▭92 92 %"),
            ("trio bez dat = jen čas a den", Look::Trio, base(), "*14:32* ne 18. 10."),
            ("trio čeká: střed = čekání", Look::Trio, wait(full()), "[Sun:text] *29°* | ●amber *Čeká na tebe · app* | CPU 8 % RAM 46 % ▭92 92 %"),
            ("trio hotovo", Look::Trio, done(base()), "[Check:ok] Hotovo: app"),
            ("rings dva limity, klid", Look::Rings, full(), "◯42:acc ◯86:amber *14:32*"),
            ("rings pracuje = točí se", Look::Rings, work(full()), "◯42:acc ◯86:amber *14:32* ⟳"),
            ("rings čeká = oranžově, netočí se", Look::Rings, wait(work(full())), "◯42:call ◯86:call *14:32*"),
            ("rings bez limitu = prázdná dráha", Look::Rings, base(), "◯-:idle *14:32*"),
            ("rings bez limitu, pracuje", Look::Rings, work(base()), "◯-:run *14:32* ⟳"),
            ("rings hotovo", Look::Rings, done(base()), "◯-:idle [Check:ok] *14:32*"),
        ];
        for (name, look, f, want) in cases {
            assert_eq!(sig(&show(look, &f)), want, "{name}");
        }
    }

    #[test]
    fn hidden_sources_are_not_drawn() {
        let f = without(full(), &[StripItem::System, StripItem::Limits, StripItem::Clock]);
        assert_eq!(sig(&show(Look::Trio, &f)), "[Sun:text] *29°* | ▭92 92 %");
        assert_eq!(sig(&show(Look::Rings, &f)), "◯-:idle");
        // nabíjení = „+“ u tvaru; 100 % a víc limitu = červená
        let mut g = base();
        g.battery = Some(BatteryFact { percent: 40, charging: true });
        g.limit5h = Some(130.0);
        assert_eq!(sig(&show(Look::Minimum, &g)), "▭40+ 40 % *14:32*");
        assert_eq!(sig(&show(Look::Rings, &g)), "◯100:red *14:32*");
    }

    #[test]
    fn helpers() {
        assert_eq!(day_of("14:32 · ne 18. 10."), "ne 18. 10.");
        assert_eq!(day_of(""), "");
        assert_eq!(day_of("ne 18. 10."), "ne 18. 10.");
        let acc = Rgba::VIOLET;
        for (p, want) in [(0.0, acc), (79.9, acc), (80.0, Rgba::AMBER), (99.0, Rgba::AMBER), (100.0, Rgba::RED)] {
            assert_eq!(ring_color(p, acc), want, "{p}");
        }
        for (code, day, want) in [(0, true, Glyph::Sun), (1, false, Glyph::Moon), (3, true, Glyph::Cloud), (61, false, Glyph::Cloud), (95, true, Glyph::Cloud)] {
            assert_eq!(weather_glyph(code, day), want, "{code}");
        }
        assert_eq!(pct(8.2), "8 %");
        assert_eq!(pct(120.0), "100 %");
    }
}
