//! Časová pásma událostí — bez databáze pásem (žádná nová crate): přednost má VTIMEZONE z feedu
//! (STANDARD/DAYLIGHT, TZOFFSETTO a RRULE přechodu — Google i Outlook ho posílají), pak malá vestavěná
//! tabulka běžných IANA názvů a Windows názvů z Outlooku (EU a US pravidla letního času, pevné
//! posuny). Neznámé pásmo = čas se bere jako místní a výskyt nese `guess` (UI to přizná, nic
//! nepředstírá). Místní pásmo počítače dodává volající (`LocalTz`), v testech pevné.

use super::expand::{self, Freq, Rule, Until};
use super::ics::{Dt, VTimezone};
use chrono::{Datelike, Duration, NaiveDateTime};
use std::cell::RefCell;
use std::collections::HashMap;

/// Posun místního času od UTC v sekundách (místní = UTC + posun).
pub trait LocalTz {
    /// pro místní čas (v mezeře/překryvu při změně času stačí jedna z hodnot)
    fn offset_local(&self, local: NaiveDateTime) -> i32;
    fn offset_utc(&self, utc: NaiveDateTime) -> i32;
}

/// Místní pásmo Windows (chrono `Local`).
pub struct SystemLocal;

impl LocalTz for SystemLocal {
    fn offset_local(&self, local: NaiveDateTime) -> i32 {
        use chrono::{Local, Offset, TimeZone};
        match Local.offset_from_local_datetime(&local).earliest() {
            Some(o) => o.fix().local_minus_utc(),
            // mezera při přechodu na letní čas: posun z UTC výkladu stejného času
            None => Local.offset_from_utc_datetime(&local).fix().local_minus_utc(),
        }
    }
    fn offset_utc(&self, utc: NaiveDateTime) -> i32 {
        use chrono::{Local, Offset, TimeZone};
        Local.offset_from_utc_datetime(&utc).fix().local_minus_utc()
    }
}

/// Jeden přechod: od `start` (místní čas před přechodem) platí posun `to`, opakuje se podle `rule`.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub start: NaiveDateTime,
    pub from: i32,
    pub to: i32,
    pub rule: Option<Rule>,
    pub rdates: Vec<NaiveDateTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Zone {
    Fixed(i32),
    Rules(Vec<Change>),
}

impl Change {
    /// Nejpozdější nástup tohoto přechodu ≤ `at`.
    fn last_onset(&self, at: NaiveDateTime) -> Option<NaiveDateTime> {
        let mut best = (self.start <= at).then_some(self.start);
        for &r in &self.rdates {
            if r <= at && best.is_none_or(|b| r > b) {
                best = Some(r);
            }
        }
        if let (Some(rule), true) = (&self.rule, self.start <= at) {
            // roční pravidlo bez COUNT: začít dva roky před `at` (Outlook má DTSTART 1601)
            let mut start = self.start;
            if rule.freq == Freq::Yearly && rule.interval == 1 && rule.count.is_none() && at.year() - 2 > start.year() {
                start = start.with_year(at.year() - 2).unwrap_or(start);
            }
            let until = rule.until.as_ref().map(|u| match u {
                Until::Date(d) => d.and_hms_opt(23, 59, 59).unwrap_or_default(),
                Until::Local(t) => *t,
                Until::Utc(t) => *t + Duration::seconds(self.from as i64),
            });
            if let Some(&t) = expand::occurrences(rule, start, until, at - Duration::days(800), at).last()
                && best.is_none_or(|b| t > b)
            {
                best = Some(t);
            }
        }
        best
    }
}

impl Zone {
    pub fn offset_local(&self, local: NaiveDateTime) -> i32 {
        match self {
            Zone::Fixed(o) => *o,
            Zone::Rules(ch) => {
                let mut best: Option<(NaiveDateTime, i32)> = None;
                for c in ch {
                    if let Some(t) = c.last_onset(local)
                        && best.is_none_or(|(b, _)| t > b)
                    {
                        best = Some((t, c.to));
                    }
                }
                // před prvním přechodem: „from“ nejstaršího
                best.map(|(_, o)| o).or_else(|| ch.iter().min_by_key(|c| c.start).map(|c| c.from)).unwrap_or(0)
            }
        }
    }

    pub fn offset_utc(&self, utc: NaiveDateTime) -> i32 {
        match self {
            Zone::Fixed(o) => *o,
            Zone::Rules(ch) => {
                // nástup přechodu v UTC = místní nástup − „from“; platí nejpozdější ≤ `utc`
                let mut best: Option<(NaiveDateTime, i32)> = None;
                for c in ch {
                    let from = Duration::seconds(c.from as i64);
                    if let Some(t) = c.last_onset(utc + from)
                        && best.is_none_or(|(b, _)| t - from > b)
                    {
                        best = Some((t - from, c.to));
                    }
                }
                best.map(|(_, o)| o).or_else(|| ch.iter().min_by_key(|c| c.start).map(|c| c.from)).unwrap_or(0)
            }
        }
    }

    /// VTIMEZONE z feedu. Přechod s nečitelným RRULE platí jen jednou (v DTSTART).
    pub fn from_vtimezone(z: &VTimezone) -> Zone {
        Zone::Rules(
            z.parts
                .iter()
                .map(|p| Change { start: p.start, from: p.from, to: p.to, rule: p.rrule.as_deref().and_then(|r| expand::parse_rule(r).ok()), rdates: p.rdates.clone() })
                .collect(),
        )
    }
}

impl LocalTz for Zone {
    fn offset_local(&self, local: NaiveDateTime) -> i32 {
        Zone::offset_local(self, local)
    }
    fn offset_utc(&self, utc: NaiveDateTime) -> i32 {
        Zone::offset_utc(self, utc)
    }
}

#[derive(Clone, Copy)]
enum Kind {
    /// EU: poslední neděle března a října v 01:00 UTC
    Eu,
    /// USA/Kanada: druhá neděle března a první neděle listopadu ve 2:00 místního
    Us,
    Fixed,
}

/// Vestavěná pásma: (názvy IANA i Windows, pravidlo, standardní posun v minutách).
const BUILTIN: &[(&[&str], Kind, i32)] = &[
    (&["UTC", "Etc/UTC", "GMT", "Etc/GMT", "Z", "Coordinated Universal Time", "UTC+00:00"], Kind::Fixed, 0),
    (&["Greenwich Standard Time", "Atlantic/Reykjavik", "Africa/Abidjan"], Kind::Fixed, 0),
    (
        &[
            "Europe/Prague", "Europe/Bratislava", "Europe/Berlin", "Europe/Vienna", "Europe/Warsaw", "Europe/Budapest", "Europe/Paris",
            "Europe/Rome", "Europe/Madrid", "Europe/Amsterdam", "Europe/Brussels", "Europe/Zurich", "Europe/Stockholm", "Europe/Copenhagen",
            "Europe/Oslo", "Europe/Belgrade", "Europe/Zagreb", "Europe/Ljubljana", "Europe/Luxembourg", "Europe/Monaco", "Europe/Malta",
            "Central Europe Standard Time", "Central European Standard Time", "W. Europe Standard Time", "Romance Standard Time", "CET",
        ],
        Kind::Eu,
        60,
    ),
    (&["Europe/London", "Europe/Dublin", "Europe/Lisbon", "Atlantic/Canary", "GMT Standard Time", "WET"], Kind::Eu, 0),
    (
        &[
            "Europe/Helsinki", "Europe/Kiev", "Europe/Kyiv", "Europe/Riga", "Europe/Tallinn", "Europe/Vilnius", "Europe/Athens",
            "Europe/Bucharest", "Europe/Sofia", "FLE Standard Time", "GTB Standard Time", "E. Europe Standard Time", "EET",
        ],
        Kind::Eu,
        120,
    ),
    (&["America/New_York", "America/Toronto", "America/Detroit", "Eastern Standard Time", "US/Eastern"], Kind::Us, -300),
    (&["America/Chicago", "America/Winnipeg", "Central Standard Time", "US/Central"], Kind::Us, -360),
    (&["America/Denver", "America/Edmonton", "Mountain Standard Time", "US/Mountain"], Kind::Us, -420),
    (&["America/Los_Angeles", "America/Vancouver", "Pacific Standard Time", "US/Pacific"], Kind::Us, -480),
    (&["America/Phoenix", "US Mountain Standard Time"], Kind::Fixed, -420),
    (&["Europe/Moscow", "Russian Standard Time"], Kind::Fixed, 180),
    (&["Europe/Istanbul", "Turkey Standard Time"], Kind::Fixed, 180),
    (&["Asia/Dubai", "Arabian Standard Time"], Kind::Fixed, 240),
    (&["Asia/Kolkata", "Asia/Calcutta", "India Standard Time"], Kind::Fixed, 330),
    (&["Asia/Shanghai", "Asia/Hong_Kong", "China Standard Time"], Kind::Fixed, 480),
    (&["Asia/Singapore", "Singapore Standard Time"], Kind::Fixed, 480),
    (&["Asia/Tokyo", "Tokyo Standard Time"], Kind::Fixed, 540),
    (&["Asia/Seoul", "Korea Standard Time"], Kind::Fixed, 540),
    (&["Australia/Brisbane", "E. Australia Standard Time"], Kind::Fixed, 600),
];

fn rule(s: &str) -> Option<Rule> {
    expand::parse_rule(s).ok()
}

/// Vestavěné pásmo podle názvu (bez ohledu na velikost písmen; Mozilla prefix `/mozilla.org/…/` se ořízne).
pub fn builtin(name: &str) -> Option<Zone> {
    let n = name.trim().trim_matches('"');
    let n = n.rsplit_once("/mozilla.org/").map_or(n, |(_, rest)| rest.split_once('/').map_or(rest, |(_, z)| z));
    let &(_, kind, min) = BUILTIN.iter().find(|(names, ..)| names.iter().any(|x| x.eq_ignore_ascii_case(n)))?;
    let std = min * 60;
    let at = |y: i32, m: u32, d: u32, h: u32| chrono::NaiveDate::from_ymd_opt(y, m, d).and_then(|x| x.and_hms_opt(h, 0, 0));
    Some(match kind {
        Kind::Fixed => Zone::Fixed(std),
        Kind::Eu => {
            // 01:00 UTC vyjádřené v místním čase před přechodem
            let h = |off: i32| (1 + off / 3600).rem_euclid(24) as u32;
            Zone::Rules(vec![
                Change { start: at(1996, 3, 31, h(std))?, from: std, to: std + 3600, rule: rule("FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU"), rdates: vec![] },
                Change { start: at(1996, 10, 27, h(std + 3600))?, from: std + 3600, to: std, rule: rule("FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU"), rdates: vec![] },
            ])
        }
        Kind::Us => Zone::Rules(vec![
            Change { start: at(2007, 3, 11, 2)?, from: std, to: std + 3600, rule: rule("FREQ=YEARLY;BYMONTH=3;BYDAY=2SU"), rdates: vec![] },
            Change { start: at(2007, 11, 4, 2)?, from: std + 3600, to: std, rule: rule("FREQ=YEARLY;BYMONTH=11;BYDAY=1SU"), rdates: vec![] },
        ]),
    })
}

/// Pásmo pro výskyt události.
#[derive(Clone, Copy)]
pub enum Tz<'a> {
    Utc,
    /// místní čas počítače; `guess` = TZID jsme neznali
    Local(bool),
    Zone(&'a Zone),
}

impl Tz<'_> {
    pub fn guess(&self) -> bool {
        matches!(self, Tz::Local(true))
    }
}

/// Pásma jednoho feedu + místní pásmo. Vestavěná pásma se dohledají líně a pamatují.
pub struct Zones<'a> {
    feed: HashMap<String, Zone>,
    builtin: RefCell<HashMap<String, Option<Box<Zone>>>>,
    pub local: &'a dyn LocalTz,
}

impl<'a> Zones<'a> {
    pub fn new(zones: &[VTimezone], local: &'a dyn LocalTz) -> Self {
        let feed = zones.iter().map(|z| (z.tzid.clone(), Zone::from_vtimezone(z))).collect();
        Zones { feed, builtin: RefCell::new(HashMap::new()), local }
    }

    /// Posun pásma `tz` pro místní čas `t` (v jeho pásmu).
    fn offset_local(&self, tz: Tz, t: NaiveDateTime) -> i32 {
        match tz {
            Tz::Utc => 0,
            Tz::Local(_) => self.local.offset_local(t),
            Tz::Zone(z) => z.offset_local(t),
        }
    }

    pub fn to_utc(&self, tz: Tz, t: NaiveDateTime) -> NaiveDateTime {
        t - Duration::seconds(self.offset_local(tz, t) as i64)
    }

    pub fn local_from_utc(&self, tz: Tz, utc: NaiveDateTime) -> NaiveDateTime {
        let o = match tz {
            Tz::Utc => 0,
            Tz::Local(_) => self.local.offset_utc(utc),
            Tz::Zone(z) => z.offset_utc(utc),
        };
        utc + Duration::seconds(o as i64)
    }

    /// Hodnota z ICS → (pásmo, místní čas v něm). Datum = místní půlnoc.
    pub fn resolve<'s>(&'s self, dt: &Dt, keep: &'s mut Option<Box<Zone>>) -> (Tz<'s>, NaiveDateTime) {
        match dt {
            Dt::Utc(t) => (Tz::Utc, *t),
            Dt::Date(_) | Dt::Floating(_) => (Tz::Local(false), dt.naive()),
            Dt::Zoned(t, id) => (self.zone(id, keep), *t),
        }
    }

    /// Pásmo podle TZID. Vestavěné se zkopíruje do `keep` (držák volajícího), feedové se půjčí.
    pub fn zone<'s>(&'s self, id: &str, keep: &'s mut Option<Box<Zone>>) -> Tz<'s> {
        if let Some(z) = self.feed.get(id).or_else(|| self.feed.get(id.trim().trim_matches('"'))) {
            return Tz::Zone(z);
        }
        let mut cache = self.builtin.borrow_mut();
        let found = cache.entry(id.to_string()).or_insert_with(|| builtin(id).map(Box::new));
        match found {
            Some(z) => {
                *keep = Some(z.clone());
                let kept: &'s Option<Box<Zone>> = keep;
                Tz::Zone(kept.as_deref().expect("právě vloženo"))
            }
            None => Tz::Local(true),
        }
    }

    /// Hodnota z ICS rovnou na UTC (EXDATE, RECURRENCE-ID, UNTIL). Datum = místní půlnoc.
    pub fn utc_of(&self, dt: &Dt) -> NaiveDateTime {
        let mut keep = None;
        let (tz, t) = self.resolve(dt, &mut keep);
        self.to_utc(tz, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::ics;

    fn at(y: i32, m: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, 0).unwrap()
    }

    /// (místní čas, posun v hodinách) pro Prahu — přechody 29. 3. 2026 2:00 a 25. 10. 2026 3:00.
    type At = (i32, u32, u32, u32, u32);
    const PRAGUE: &[(At, i32)] = &[
        ((2026, 1, 15, 12, 0), 1),
        ((2026, 3, 29, 1, 59), 1),
        ((2026, 3, 29, 3, 0), 2),
        ((2026, 7, 1, 12, 0), 2),
        ((2026, 10, 25, 1, 0), 2),
        ((2026, 10, 25, 4, 0), 1),
        ((2026, 12, 24, 18, 0), 1),
        ((2031, 6, 1, 0, 0), 2),
    ];

    #[test]
    fn builtin_eu_us_fixed() {
        let p = builtin("Europe/Prague").unwrap();
        for &((y, m, d, h, mi), want) in PRAGUE {
            assert_eq!(p.offset_local(at(y, m, d, h, mi)), want * 3600, "{y}-{m}-{d} {h}:{mi}");
        }
        // Windows název z Outlooku = totéž
        assert_eq!(builtin("Central Europe Standard Time").unwrap(), p);
        assert_eq!(builtin("\"central europe standard time\"").unwrap(), p);
        assert_eq!(builtin("/mozilla.org/20050126_1/Europe/Prague").unwrap(), p);
        let ny = builtin("Eastern Standard Time").unwrap();
        let cases = [((2026, 3, 8, 1, 0), -5), ((2026, 3, 8, 3, 0), -4), ((2026, 10, 31, 12, 0), -4), ((2026, 11, 1, 3, 0), -5), ((2027, 1, 1, 0, 0), -5)];
        for ((y, m, d, h, mi), want) in cases {
            assert_eq!(ny.offset_local(at(y, m, d, h, mi)), want * 3600, "NY {y}-{m}-{d} {h}");
        }
        assert_eq!(builtin("Europe/London").unwrap().offset_local(at(2026, 7, 1, 12, 0)), 3600);
        assert_eq!(builtin("Asia/Kolkata"), Some(Zone::Fixed(19_800)));
        assert_eq!(builtin("UTC"), Some(Zone::Fixed(0)));
        assert!(builtin("Mars/Olympus_Mons").is_none());
        assert!(builtin("(UTC+01:00) Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna").is_none());
    }

    #[test]
    fn offset_from_utc_side() {
        let p = builtin("Europe/Prague").unwrap();
        let cases = [((2026, 3, 29, 0, 59), 1), ((2026, 3, 29, 1, 0), 2), ((2026, 10, 25, 0, 59), 2), ((2026, 10, 25, 1, 0), 1)];
        for ((y, m, d, h, mi), want) in cases {
            assert_eq!(p.offset_utc(at(y, m, d, h, mi)), want * 3600, "UTC {y}-{m}-{d} {h}:{mi}");
        }
    }

    /// VTIMEZONE tak, jak ho posílá Outlook (DTSTART 1601, Windows název) a Google (IANA, 1970).
    #[test]
    fn vtimezone_from_outlook_and_google() {
        let outlook = b"BEGIN:VCALENDAR\r\nBEGIN:VTIMEZONE\r\nTZID:W. Europe Standard Time\r\nBEGIN:STANDARD\r\nDTSTART:16010101T030000\r\nTZOFFSETFROM:+0200\r\nTZOFFSETTO:+0100\r\n\
RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=10\r\nEND:STANDARD\r\nBEGIN:DAYLIGHT\r\nDTSTART:16010101T020000\r\nTZOFFSETFROM:+0100\r\nTZOFFSETTO:+0200\r\n\
RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=3\r\nEND:DAYLIGHT\r\nEND:VTIMEZONE\r\nEND:VCALENDAR\r\n";
        let google = b"BEGIN:VCALENDAR\nBEGIN:VTIMEZONE\nTZID:Europe/Prague\nX-LIC-LOCATION:Europe/Prague\nBEGIN:DAYLIGHT\nTZOFFSETFROM:+0100\nTZOFFSETTO:+0200\nTZNAME:GMT+2\nDTSTART:19700329T020000\n\
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU\nEND:DAYLIGHT\nBEGIN:STANDARD\nTZOFFSETFROM:+0200\nTZOFFSETTO:+0100\nTZNAME:GMT+1\nDTSTART:19701025T030000\nRRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU\nEND:STANDARD\nEND:VTIMEZONE\nEND:VCALENDAR\n";
        for raw in [&outlook[..], &google[..]] {
            let f = ics::parse(raw, &mut |_| true).unwrap();
            let z = Zone::from_vtimezone(&f.zones[0]);
            for &((y, m, d, h, mi), want) in PRAGUE {
                assert_eq!(z.offset_local(at(y, m, d, h, mi)), want * 3600, "{} {y}-{m}-{d} {h}:{mi}", f.zones[0].tzid);
            }
        }
    }

    #[test]
    fn zones_resolve_feed_first_then_builtin_then_local() {
        // feed říká, že „Europe/Prague“ je pevně +5 (podivné, ale feed má přednost)
        let vt = VTimezone { tzid: "Europe/Prague".into(), parts: vec![ics::TzPart { start: at(1970, 1, 1, 0, 0), from: 18_000, to: 18_000, rrule: None, rdates: vec![] }] };
        let local = Zone::Fixed(3600);
        let zs = Zones::new(&[vt], &local);
        let t = at(2026, 7, 1, 12, 0);
        assert_eq!(zs.utc_of(&Dt::Zoned(t, "Europe/Prague".into())), at(2026, 7, 1, 7, 0));
        assert_eq!(zs.utc_of(&Dt::Zoned(t, "Europe/London".into())), at(2026, 7, 1, 11, 0));
        assert_eq!(zs.utc_of(&Dt::Utc(t)), t);
        assert_eq!(zs.utc_of(&Dt::Floating(t)), at(2026, 7, 1, 11, 0));
        assert_eq!(zs.utc_of(&Dt::Zoned(t, "Neznámé/Pásmo".into())), at(2026, 7, 1, 11, 0));
        let mut keep = None;
        assert!(zs.zone("Neznámé/Pásmo", &mut keep).guess());
        let mut keep = None;
        assert!(!zs.zone("Asia/Tokyo", &mut keep).guess());
        // tam a zpět
        let mut keep = None;
        let tz = zs.zone("America/New_York", &mut keep);
        assert_eq!(zs.local_from_utc(tz, zs.to_utc(tz, t)), t);
    }
}
