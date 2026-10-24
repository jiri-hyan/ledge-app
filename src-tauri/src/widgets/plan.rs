//! Plánovač běhů widgetů — čisté funkce (`live.rs` je jen volá a spí). Pravidla:
//! - běží jen **aktivní** widget: karta na záložce, která je právě vidět (šuplík otevřený), nebo položka
//!   proužku (proužek zapnutý, Ledge není mrtvý — hra na celou obrazovku / herní režim);
//! - nikdy dvakrát souběžně; bez souhlasu / s chybnou hlavičkou vůbec;
//! - další běh = konec posledního + interval + jitter (deterministický podle id, ať se widgety se
//!   stejným intervalem nespouštějí v tutéž chvíli); poprvé (nebo po otevření šuplíku s daty staršími
//!   než interval) hned;
//! - „Obnovit teď“ (`force`) spustí i neaktivní widget (výslovný klik).
//!
//! Když nic aktivní není, vlákno spí bez termínu (žádný časovač v klidu).

use std::time::{Duration, Instant};

/// Prefix id karty / položky proužku widgetu (`widget:<soubor>`).
pub const PREFIX: &str = "widget:";

/// Soubor z id widgetu: jen jméno (bez cesty a `:`), ne skrytý, nejvýš 200 znaků. Sdílí karta
/// (`layout::validate`), položka proužku (`StripItem::parse`) i spuštění (`widgets::fresh`).
pub fn file_of(id: &str) -> Option<&str> {
    id.strip_prefix(PREFIX).filter(|f| !f.is_empty() && f.len() <= 200 && !f.contains(['\\', '/', ':']) && !f.starts_with('.') && !f.chars().any(char::is_control))
}

/// Co je teď vidět.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    /// id karet na záložce, která je teď vybraná
    pub page_cards: Vec<String>,
    /// šuplík je otevřený (plný)
    pub drawer_full: bool,
    /// položky proužku (`strip.items` jako text)
    pub strip_items: Vec<String>,
    pub strip_on: bool,
    /// fullscreen hra / herní režim
    pub dead: bool,
    /// widgety na ploše, které jsou teď vidět (`desk/`) — běží i se zavřeným šuplíkem, ve hře ne
    pub desk: Vec<String>,
}

/// Aktivní widgety (id bez duplicit, v pořadí karta → proužek) a jestli nějaký čeká, až Ledge ožije
/// (`parked` = proužek s widgetem, ale teď hra → vlákno se za chvíli podívá znovu).
pub fn active(c: &Ctx) -> (Vec<String>, bool) {
    let mut out: Vec<String> = Vec::new();
    let widgets = |v: &[String]| v.iter().filter(|id| id.starts_with(PREFIX)).cloned().collect::<Vec<_>>();
    if c.drawer_full && !c.dead {
        out.extend(widgets(&c.page_cards));
    }
    if !c.dead {
        for id in widgets(&c.desk) {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    let strip = widgets(&c.strip_items);
    let parked = c.strip_on && c.dead && !strip.is_empty();
    if c.strip_on && !c.dead {
        for id in strip {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    (out, parked)
}

/// Stav jednoho widgetu pro plánovač.
#[derive(Debug, Clone)]
pub struct Slot {
    pub id: String,
    pub interval: Duration,
    /// platná hlavička + souhlas s tímhle obsahem
    pub ready: bool,
    pub running: bool,
    /// „Obnovit teď“
    pub force: bool,
    pub last_end: Option<Instant>,
}

/// Jitter: 0 až min(interval / 10, 15 s), stabilní pro id (FNV-1a).
pub fn jitter(id: &str, interval: Duration) -> Duration {
    let cap = (interval / 10).min(Duration::from_secs(15)).as_millis() as u64;
    if cap == 0 {
        return Duration::ZERO;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    Duration::from_millis(h % cap)
}

/// Kdy má slot běžet (`None` = nikdy: neplatný nebo běží).
pub fn due_at(s: &Slot, now: Instant) -> Option<Instant> {
    if !s.ready || s.running {
        return None;
    }
    if s.force {
        return Some(now);
    }
    match s.last_end {
        None => Some(now),
        Some(t) => Some(t + s.interval + jitter(&s.id, s.interval)),
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Decision {
    /// spustit teď (v pořadí)
    pub run: Vec<String>,
    /// probudit se nejpozději tehdy (`None` = spát, dokud nepřijde událost)
    pub wake_at: Option<Instant>,
}

/// Co spustit teď a kdy se probudit. `parked` = přidat kontrolu za `recheck` (Ledge mrtvý, ale
/// proužek s widgetem čeká).
pub fn decide(slots: &[Slot], active: &[String], parked: bool, now: Instant, recheck: Duration) -> Decision {
    let mut d = Decision::default();
    for s in slots {
        let on = active.contains(&s.id);
        if !on && !s.force {
            continue;
        }
        let Some(due) = due_at(s, now) else { continue };
        if due <= now {
            d.run.push(s.id.clone());
        } else {
            d.wake_at = Some(d.wake_at.map_or(due, |w| w.min(due)));
        }
    }
    if parked {
        let t = now + recheck;
        d.wake_at = Some(d.wake_at.map_or(t, |w| w.min(t)));
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str) -> String {
        id.to_string()
    }

    #[test]
    fn active_table() {
        let page = vec![s("clock"), s("widget:a.ps1"), s("widget:b.py")];
        let strip = vec![s("clock"), s("widget:b.py"), s("widget:c.cmd")];
        // (popis, šuplík otevřený, proužek zapnutý, mrtvý, aktivní, parked)
        type Case = (&'static str, bool, bool, bool, Vec<&'static str>, bool);
        let cases: Vec<Case> = vec![
            ("vše zavřené, proužek vypnutý = nic", false, false, false, vec![], false),
            ("šuplík otevřený", true, false, false, vec!["widget:a.ps1", "widget:b.py"], false),
            ("jen proužek", false, true, false, vec!["widget:b.py", "widget:c.cmd"], false),
            ("obojí bez duplicit", true, true, false, vec!["widget:a.ps1", "widget:b.py", "widget:c.cmd"], false),
            ("hra: nic, proužek čeká", true, true, true, vec![], true),
            ("hra bez proužku", true, false, true, vec![], false),
        ];
        for (name, full, strip_on, dead, want, parked) in cases {
            let c = Ctx { page_cards: page.clone(), drawer_full: full, strip_items: strip.clone(), strip_on, dead, desk: vec![] };
            let (a, p) = active(&c);
            assert_eq!(a, want.iter().map(|x| s(x)).collect::<Vec<_>>(), "{name}");
            assert_eq!(p, parked, "{name}: parked");
        }
        let none = Ctx { strip_on: true, dead: true, ..Default::default() };
        assert_eq!(active(&none), (vec![], false), "proužek bez widgetu nečeká");
        // widget na ploše: běží se zavřeným šuplíkem, bez duplicit s kartou, ve hře ne
        let desk = vec![s("widget:d.ps1"), s("widget:a.ps1"), s("limits")];
        let c = Ctx { page_cards: page.clone(), drawer_full: true, desk: desk.clone(), ..Default::default() };
        assert_eq!(active(&c).0, vec![s("widget:a.ps1"), s("widget:b.py"), s("widget:d.ps1")], "plocha + karta");
        let c = Ctx { desk: desk.clone(), ..Default::default() };
        assert_eq!(active(&c).0, vec![s("widget:d.ps1"), s("widget:a.ps1")], "jen plocha");
        let c = Ctx { desk, dead: true, ..Default::default() };
        assert_eq!(active(&c), (vec![], false), "plocha ve hře nic");
    }

    #[test]
    fn jitter_is_stable_and_bounded() {
        let i = Duration::from_secs(300);
        assert_eq!(jitter("widget:a.ps1", i), jitter("widget:a.ps1", i));
        for id in ["a", "b", "widget:docker.ps1", "widget:cnb-kurz.py"] {
            assert!(jitter(id, i) < Duration::from_secs(15), "{id}");
            assert!(jitter(id, Duration::from_secs(30)) < Duration::from_secs(3), "{id}: interval/10");
        }
        assert_ne!(jitter("widget:a.ps1", i), jitter("widget:b.ps1", i));
        assert_eq!(jitter("x", Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn decide_table() {
        let now = Instant::now();
        let min = Duration::from_secs(60);
        let slot = |id: &str, ready: bool, running: bool, force: bool, ago: Option<u64>| Slot {
            id: s(id),
            interval: min,
            ready,
            running,
            force,
            last_end: ago.map(|a| now - Duration::from_secs(a)),
        };
        let active = vec![s("w:new"), s("w:old"), s("w:fresh"), s("w:busy"), s("w:noconsent")];
        let slots = vec![
            slot("w:new", true, false, false, None),           // nikdy neběžel → hned
            slot("w:old", true, false, false, Some(600)),      // data starší než interval → hned
            slot("w:fresh", true, false, false, Some(10)),     // za ~50 s + jitter
            slot("w:busy", true, true, false, None),           // běží → nikdy souběžně
            slot("w:noconsent", false, false, false, None),    // bez souhlasu → nikdy
            slot("w:hidden", true, false, false, None),        // není aktivní → ne
            slot("w:forced", true, false, true, Some(1)),      // Obnovit teď i neaktivní
            slot("w:forced-busy", true, true, true, Some(1)),  // ale ne, když běží
        ];
        let d = decide(&slots, &active, false, now, Duration::from_secs(30));
        assert_eq!(d.run, vec![s("w:new"), s("w:old"), s("w:forced")]);
        let wake = d.wake_at.expect("probudit kvůli w:fresh");
        let j = jitter("w:fresh", min);
        assert_eq!(wake, now - Duration::from_secs(10) + min + j);
        // nic aktivního = spát bez termínu
        let d = decide(&slots[..1], &[], false, now, Duration::from_secs(30));
        assert_eq!(d, Decision::default());
        // parked: kontrola za recheck, i když nic jiného
        let d = decide(&[], &[], true, now, Duration::from_secs(30));
        assert_eq!(d.wake_at, Some(now + Duration::from_secs(30)));
    }
}
