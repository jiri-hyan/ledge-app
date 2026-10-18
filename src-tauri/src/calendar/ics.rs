//! Parser ICS (RFC 5545) — jen to, co karta Kalendář potřebuje: rozbalení zalomených řádků (na bajtech,
//! takže zalomení uprostřed UTF-8 znaku nic nerozbije), parametry s uvozovkami, escapování textu,
//! VEVENT (SUMMARY, LOCATION, DTSTART/DTEND/DURATION, RRULE, RDATE, EXDATE, RECURRENCE-ID, STATUS;
//! z vlastností schůzky, URL, místa a popisu jen odkaz „Připojit“ — `meeting.rs`, popis se nedrží)
//! a VTIMEZONE (STANDARD/DAYLIGHT s TZOFFSETFROM/TO, RRULE, RDATE). Vnořené VALARM se přeskočí.
//!
//! `parse` dostane filtr `keep`: událost, kterou volající nepotřebuje (jednorázová mimo okno), se do
//! výsledku vůbec nedostane — obří feed se v paměti nedrží celý rozparsovaný.

use crate::i18n::tr;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Hodnota data/času z ICS.
#[derive(Debug, Clone, PartialEq)]
pub enum Dt {
    /// `VALUE=DATE` — celodenní (bez pásma)
    Date(NaiveDate),
    /// bez `Z` a bez `TZID` — „plovoucí“ místní čas
    Floating(NaiveDateTime),
    /// s `Z`
    Utc(NaiveDateTime),
    /// s `TZID=…` (název pásma tak, jak je ve feedu)
    Zoned(NaiveDateTime, String),
}

impl Dt {
    /// Čas bez pásma (datum = půlnoc).
    pub fn naive(&self) -> NaiveDateTime {
        match self {
            Dt::Date(d) => d.and_time(NaiveTime::MIN),
            Dt::Floating(t) | Dt::Utc(t) | Dt::Zoned(t, _) => *t,
        }
    }
    pub fn is_date(&self) -> bool {
        matches!(self, Dt::Date(_))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Event {
    pub uid: String,
    pub summary: String,
    pub location: String,
    pub start: Option<Dt>,
    pub end: Option<Dt>,
    /// `DURATION` v sekundách
    pub duration: Option<i64>,
    pub rrule: Option<String>,
    pub rdates: Vec<Dt>,
    pub exdates: Vec<Dt>,
    pub recurrence_id: Option<Dt>,
    /// `STATUS:CANCELLED`
    pub cancelled: bool,
    /// odkaz na schůzku (pořadí zdroje `meeting::RANK_*`, odkaz) — nejlepší z vlastností události
    pub join: Option<(u8, super::meeting::Join)>,
}

/// Jeden přechod pásma (STANDARD nebo DAYLIGHT) z VTIMEZONE.
#[derive(Debug, Clone, PartialEq)]
pub struct TzPart {
    /// `DTSTART` — místní čas před přechodem
    pub start: NaiveDateTime,
    /// `TZOFFSETFROM` / `TZOFFSETTO` v sekundách
    pub from: i32,
    pub to: i32,
    pub rrule: Option<String>,
    pub rdates: Vec<NaiveDateTime>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VTimezone {
    pub tzid: String,
    pub parts: Vec<TzPart>,
}

#[derive(Debug, Default)]
pub struct Feed {
    /// události, které prošly filtrem `keep`
    pub events: Vec<Event>,
    pub zones: Vec<VTimezone>,
    /// kolik VEVENT feed má celkem (i odfiltrovaných) — „Otestovat“ v Nastavení
    pub vevents: usize,
    /// název kalendáře z feedu (`X-WR-CALNAME`), když ho má — výchozí název na kartě
    pub name: String,
}

/// Logické řádky: rozbalí zalomení (CRLF/LF + mezera nebo tab), UTF-8 dekóduje až celý řádek.
pub struct Unfold<'a> {
    rest: &'a [u8],
}

pub fn unfold(bytes: &[u8]) -> Unfold<'_> {
    Unfold { rest: bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes) }
}

fn split_line(s: &[u8]) -> (&[u8], &[u8]) {
    let (line, rest) = match memchr::memchr(b'\n', s) {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, &[][..]),
    };
    (line.strip_suffix(b"\r").unwrap_or(line), rest)
}

impl Iterator for Unfold<'_> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        while !self.rest.is_empty() {
            let (first, rest) = split_line(self.rest);
            self.rest = rest;
            let mut buf = first.to_vec();
            while matches!(self.rest.first(), Some(b' ' | b'\t')) {
                let (more, rest) = split_line(self.rest);
                self.rest = rest;
                buf.extend_from_slice(&more[1..]);
            }
            if !buf.is_empty() {
                return Some(String::from_utf8_lossy(&buf).into_owned());
            }
        }
        None
    }
}

/// Vlastnost `NAME;P=V;P2="a;b":hodnota` — jméno velkými písmeny, parametry bez uvozovek.
#[derive(Debug, PartialEq)]
pub struct Prop<'a> {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: &'a str,
}

impl Prop<'_> {
    pub fn param(&self, k: &str) -> Option<&str> {
        self.params.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }
}

/// Rozdělí podle `sep` mimo uvozovky.
fn split_unquoted(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut quoted, mut from) = (false, 0);
    for (i, c) in s.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if c == sep && !quoted {
            out.push(&s[from..i]);
            from = i + c.len_utf8();
        }
    }
    out.push(&s[from..]);
    out
}

pub fn parse_prop(line: &str) -> Option<Prop<'_>> {
    let mut quoted = false;
    let colon = line.char_indices().find_map(|(i, c)| {
        if c == '"' {
            quoted = !quoted;
        }
        (c == ':' && !quoted).then_some(i)
    })?;
    let head = split_unquoted(&line[..colon], ';');
    let name = head[0].trim().to_ascii_uppercase();
    if name.is_empty() {
        return None;
    }
    let params = head[1..]
        .iter()
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_uppercase(), v.trim().trim_matches('"').to_string()))
        .collect();
    Some(Prop { name, params, value: &line[colon + 1..] })
}

/// Text ICS: `\\` `\;` `\,` a `\n` (nový řádek).
pub fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut it = v.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(x @ (',' | ';' | '\\' | ':' | '"')) => out.push(x),
            Some(x) => {
                out.push('\\');
                out.push(x);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn num(s: &str) -> Option<u32> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok())?
}

fn date8(s: &str) -> Option<NaiveDate> {
    if s.len() != 8 || !s.is_ascii() {
        return None;
    }
    NaiveDate::from_ymd_opt(num(&s[..4])? as i32, num(&s[4..6])?, num(&s[6..8])?)
}

/// `YYYYMMDDTHHMMSS` (bez `Z`).
pub fn datetime15(s: &str) -> Option<NaiveDateTime> {
    if s.len() != 15 || !s.is_ascii() || s.as_bytes()[8] != b'T' {
        return None;
    }
    // přestupná sekunda 60 → 59
    let t = NaiveTime::from_hms_opt(num(&s[9..11])?, num(&s[11..13])?, num(&s[13..15])?.min(59))?;
    Some(date8(&s[..8])?.and_time(t))
}

/// Jedna hodnota data/času; `tzid` = parametr `TZID`, `date` = `VALUE=DATE`.
pub fn parse_dt(v: &str, tzid: Option<&str>, date: bool) -> Option<Dt> {
    let v = v.trim();
    if date || v.len() == 8 {
        return date8(v.get(..8)?).map(Dt::Date);
    }
    if let Some(u) = v.strip_suffix(['Z', 'z']) {
        return datetime15(u).map(Dt::Utc);
    }
    let t = datetime15(v)?;
    Some(match tzid.map(str::trim).filter(|z| !z.is_empty()) {
        Some(z) => Dt::Zoned(t, z.to_string()),
        None => Dt::Floating(t),
    })
}

/// Seznam hodnot (EXDATE, RDATE): čárky, parametry platí pro všechny. `PERIOD` → jen začátek.
fn parse_dt_list(p: &Prop) -> Vec<Dt> {
    let date = p.param("VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE"));
    p.value.split(',').filter_map(|v| parse_dt(v.split('/').next().unwrap_or(v), p.param("TZID"), date)).collect()
}

fn prop_dt(p: &Prop) -> Option<Dt> {
    parse_dt(p.value, p.param("TZID"), p.param("VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE")))
}

/// `DURATION` (`P1D`, `PT1H30M`, `-PT15M`, `P2W`) → sekundy.
pub fn parse_duration(v: &str) -> Option<i64> {
    let v = v.trim();
    let (neg, v) = match v.as_bytes().first() {
        Some(b'-') => (true, &v[1..]),
        Some(b'+') => (false, &v[1..]),
        _ => (false, v),
    };
    let body = v.strip_prefix(['P', 'p'])?;
    let (mut total, mut n, mut time, mut any) = (0i64, None::<i64>, false, false);
    for c in body.chars() {
        if let Some(d) = c.to_digit(10) {
            n = Some(n.unwrap_or(0).checked_mul(10)?.checked_add(d as i64)?);
            continue;
        }
        let unit = match (c.to_ascii_uppercase(), time) {
            ('T', false) => {
                time = true;
                continue;
            }
            ('W', false) => 7 * 86_400,
            ('D', false) => 86_400,
            ('H', true) => 3_600,
            ('M', true) => 60,
            ('S', true) => 1,
            _ => return None,
        };
        total = total.checked_add(n.take()?.checked_mul(unit)?)?;
        any = true;
    }
    (any && n.is_none()).then_some(if neg { -total } else { total })
}

/// `+0100`, `-0500`, `+053000` → sekundy.
pub fn parse_offset(v: &str) -> Option<i32> {
    let v = v.trim();
    let sign = match v.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let d = &v[1..];
    if !(d.len() == 4 || d.len() == 6) || !d.is_ascii() {
        return None;
    }
    let (h, m) = (num(&d[..2])?, num(&d[2..4])?);
    let s = if d.len() == 6 { num(&d[4..6])? } else { 0 };
    (h < 24 && m < 60 && s < 60).then_some(sign * (h * 3600 + m * 60 + s) as i32)
}

#[derive(Default)]
struct PartDraft {
    start: Option<NaiveDateTime>,
    from: Option<i32>,
    to: Option<i32>,
    rrule: Option<String>,
    rdates: Vec<NaiveDateTime>,
}

/// Celý feed. Chybí `BEGIN:VCALENDAR` (třeba přihlašovací HTML stránka) → chyba.
pub fn parse(bytes: &[u8], keep: &mut dyn FnMut(&Event) -> bool) -> Result<Feed, String> {
    let mut feed = Feed::default();
    let mut stack: Vec<String> = Vec::new();
    let mut seen_calendar = false;
    let mut ev = Event::default();
    let mut tz = VTimezone::default();
    let mut part = PartDraft::default();
    for line in unfold(bytes) {
        let Some(p) = parse_prop(&line) else { continue };
        match p.name.as_str() {
            "BEGIN" => {
                let what = p.value.trim().to_ascii_uppercase();
                match what.as_str() {
                    "VCALENDAR" => seen_calendar = true,
                    "VEVENT" if stack.last().is_some_and(|s| s == "VCALENDAR") => ev = Event::default(),
                    "VTIMEZONE" => tz = VTimezone::default(),
                    "STANDARD" | "DAYLIGHT" => part = PartDraft::default(),
                    _ => {}
                }
                stack.push(what);
                continue;
            }
            "END" => {
                let what = p.value.trim().to_ascii_uppercase();
                // nespárované END: zahodit až po odpovídající BEGIN (poškozený feed nespadne)
                let Some(pos) = stack.iter().rposition(|s| *s == what) else { continue };
                stack.truncate(pos);
                match what.as_str() {
                    "VEVENT" if stack.last().is_some_and(|s| s == "VCALENDAR") => {
                        feed.vevents += 1;
                        let done = std::mem::take(&mut ev);
                        if done.start.is_some() && keep(&done) {
                            feed.events.push(done);
                        }
                    }
                    "STANDARD" | "DAYLIGHT" if stack.last().is_some_and(|s| s == "VTIMEZONE") => {
                        let d = std::mem::take(&mut part);
                        if let (Some(start), Some(from), Some(to)) = (d.start, d.from.or(d.to), d.to) {
                            tz.parts.push(TzPart { start, from, to, rrule: d.rrule, rdates: d.rdates });
                        }
                    }
                    "VTIMEZONE" => {
                        let done = std::mem::take(&mut tz);
                        if !done.tzid.is_empty() && !done.parts.is_empty() {
                            feed.zones.push(done);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            _ => {}
        }
        match stack.last().map(String::as_str) {
            Some("VEVENT") if stack.len() >= 2 && stack[stack.len() - 2] == "VCALENDAR" => match p.name.as_str() {
                "UID" => ev.uid = p.value.trim().to_string(),
                "SUMMARY" => ev.summary = unescape(p.value),
                "LOCATION" => {
                    ev.location = unescape(p.value);
                    super::meeting::offer(&mut ev.join, super::meeting::RANK_LOCATION, &ev.location);
                }
                "DTSTART" => ev.start = prop_dt(&p),
                "DTEND" => ev.end = prop_dt(&p),
                "DURATION" => ev.duration = parse_duration(p.value),
                "RRULE" => ev.rrule = Some(p.value.trim().to_string()),
                "RDATE" => ev.rdates.extend(parse_dt_list(&p)),
                "EXDATE" => ev.exdates.extend(parse_dt_list(&p)),
                "RECURRENCE-ID" => ev.recurrence_id = prop_dt(&p),
                "STATUS" => ev.cancelled = p.value.trim().eq_ignore_ascii_case("CANCELLED"),
                other => {
                    if let Some(rank) = super::meeting::rank_of(other) {
                        super::meeting::offer(&mut ev.join, rank, &unescape(p.value));
                    }
                }
            },
            Some("VCALENDAR") if stack.len() == 1 && p.name == "X-WR-CALNAME" => feed.name = unescape(p.value).trim().to_string(),
            Some("VTIMEZONE") => {
                if p.name == "TZID" {
                    tz.tzid = p.value.trim().to_string();
                }
            }
            Some("STANDARD" | "DAYLIGHT") if stack.len() >= 2 && stack[stack.len() - 2] == "VTIMEZONE" => match p.name.as_str() {
                "DTSTART" => part.start = prop_dt(&p).map(|d| d.naive()),
                "TZOFFSETFROM" => part.from = parse_offset(p.value),
                "TZOFFSETTO" => part.to = parse_offset(p.value),
                "RRULE" => part.rrule = Some(p.value.trim().to_string()),
                "RDATE" => part.rdates.extend(parse_dt_list(&p).iter().map(Dt::naive)),
                _ => {}
            },
            _ => {}
        }
    }
    if !seen_calendar {
        return Err(tr(
            "odpověď není kalendář ve formátu iCal (chybí BEGIN:VCALENDAR) — je to tajná adresa ve formátu iCal?",
            "the answer isn't an iCal calendar (BEGIN:VCALENDAR is missing) — is it the secret address in iCal format?",
        )
        .into());
    }
    Ok(feed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn dt(y: i32, m: u32, day: u32, h: u32, mi: u32) -> NaiveDateTime {
        d(y, m, day).and_hms_opt(h, mi, 0).unwrap()
    }
    fn all(bytes: &[u8]) -> Feed {
        parse(bytes, &mut |_| true).unwrap()
    }

    #[test]
    fn unfolding_crlf_lf_tab_and_utf8_split() {
        // „Schůzka“: zalomení uprostřed „ů“ (C5 AF) nesmí znak rozbít
        let mut raw = b"SUMMARY:Sch\xC5\r\n \xAFzka\r\nLOCATION:a\n\tb\r\n\r\nX:1".to_vec();
        raw.splice(0..0, b"\xEF\xBB\xBF".iter().copied());
        let lines: Vec<String> = unfold(&raw).collect();
        assert_eq!(lines, ["SUMMARY:Schůzka", "LOCATION:ab", "X:1"]);
    }

    #[test]
    fn props_params_quotes_and_escapes() {
        let p = parse_prop(r#"DTSTART;TZID="Central Europe Standard Time";X-A=1:20261019T090000"#).unwrap();
        assert_eq!(p.name, "DTSTART");
        assert_eq!(p.param("TZID"), Some("Central Europe Standard Time"));
        assert_eq!(p.param("X-A"), Some("1"));
        assert_eq!(p.value, "20261019T090000");
        // dvojtečka v uvozeném parametru i v hodnotě
        let p = parse_prop(r#"ATTENDEE;CN="Novák: tým":mailto:a@b.cz"#).unwrap();
        assert_eq!((p.param("CN"), p.value), (Some("Novák: tým"), "mailto:a@b.cz"));
        assert!(parse_prop("bez dvojtečky").is_none());
        let cases = [
            (r"Oběd\, pak kafe\; snad", "Oběd, pak kafe; snad"),
            (r"řádek\nřádek\Nřádek", "řádek\nřádek\nřádek"),
            (r"C:\\cesta", r"C:\cesta"),
            (r"neznámé\x", r"neznámé\x"),
            ("konec\\", "konec\\"),
        ];
        for (raw, want) in cases {
            assert_eq!(unescape(raw), want, "{raw}");
        }
    }

    #[test]
    fn date_values_table() {
        let cases: &[(&str, Option<&str>, bool, Option<Dt>)] = &[
            ("20261019", None, true, Some(Dt::Date(d(2026, 10, 19)))),
            ("20261019", None, false, Some(Dt::Date(d(2026, 10, 19)))),
            ("20261019T090000Z", None, false, Some(Dt::Utc(dt(2026, 10, 19, 9, 0)))),
            ("20261019T090000", None, false, Some(Dt::Floating(dt(2026, 10, 19, 9, 0)))),
            ("20261019T090000", Some("Europe/Prague"), false, Some(Dt::Zoned(dt(2026, 10, 19, 9, 0), "Europe/Prague".into()))),
            // Z vyhrává nad TZID
            ("20261019T090000Z", Some("Europe/Prague"), false, Some(Dt::Utc(dt(2026, 10, 19, 9, 0)))),
            ("20261019T235960", None, false, Some(Dt::Floating(d(2026, 10, 19).and_hms_opt(23, 59, 59).unwrap()))),
            ("20261332", None, true, None),
            ("2026-10-19", None, false, None),
            ("20261019T9", None, false, None),
            ("", None, false, None),
        ];
        for (v, tz, date, want) in cases {
            assert_eq!(&parse_dt(v, *tz, *date), want, "{v} {tz:?}");
        }
    }

    #[test]
    fn durations_and_offsets() {
        let cases: &[(&str, Option<i64>)] = &[
            ("PT1H", Some(3600)),
            ("PT1H30M", Some(5400)),
            ("P1D", Some(86_400)),
            ("P1DT2H", Some(93_600)),
            ("P2W", Some(1_209_600)),
            ("-PT15M", Some(-900)),
            ("+PT10S", Some(10)),
            ("PT", None),
            ("P1H", None),
            ("PT5", None),
            ("1H", None),
        ];
        for (v, want) in cases {
            assert_eq!(parse_duration(v), *want, "{v}");
        }
        let cases: &[(&str, Option<i32>)] = &[("+0100", Some(3600)), ("-0500", Some(-18_000)), ("+0530", Some(19_800)), ("+053015", Some(19_815)), ("0100", None), ("+01", None), ("+2500", None)];
        for (v, want) in cases {
            assert_eq!(parse_offset(v), *want, "{v}");
        }
    }

    #[test]
    fn events_alarms_cancelled_and_lists() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Standup\r\nDTSTART;TZID=Europe/Prague:20261019T090000\r\nDTEND;TZID=Europe/Prague:20261019T091500\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE\r\nEXDATE;TZID=Europe/Prague:20261021T090000,20261026T090000\r\nBEGIN:VALARM\r\nSUMMARY:alarm nesmí přepsat název\r\nTRIGGER:-PT10M\r\nEND:VALARM\r\n\
LOCATION:Zasedačka\\, 2. patro\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART;VALUE=DATE:20261020\r\nDURATION:P2D\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nSUMMARY:bez začátku\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let f = all(ics.as_bytes());
        assert_eq!(f.vevents, 3);
        assert_eq!(f.events.len(), 2, "událost bez DTSTART se zahodí");
        let a = &f.events[0];
        assert_eq!((a.uid.as_str(), a.summary.as_str(), a.location.as_str()), ("a", "Standup", "Zasedačka, 2. patro"));
        assert_eq!(a.start, Some(Dt::Zoned(dt(2026, 10, 19, 9, 0), "Europe/Prague".into())));
        assert_eq!(a.rrule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE"));
        assert_eq!(a.exdates.len(), 2);
        assert_eq!(a.exdates[1], Dt::Zoned(dt(2026, 10, 26, 9, 0), "Europe/Prague".into()));
        let b = &f.events[1];
        assert!(b.cancelled);
        assert_eq!((b.start.clone(), b.duration), (Some(Dt::Date(d(2026, 10, 20))), Some(172_800)));
    }

    #[test]
    fn keep_filter_drops_early_and_counts_all() {
        let ics = b"BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:1\nDTSTART:20200101T100000Z\nEND:VEVENT\nBEGIN:VEVENT\nUID:2\nDTSTART:20261019T100000Z\nEND:VEVENT\nEND:VCALENDAR\n";
        let f = parse(b"BEGIN:VCALENDAR\nX-WR-CALNAME:Rodina\\, doma\nBEGIN:VEVENT\nX-WR-CALNAME:ne\nEND:VEVENT\nEND:VCALENDAR\n", &mut |_| true).unwrap();
        assert_eq!(f.name, "Rodina, doma");
        let f = parse(ics, &mut |e| e.uid == "2").unwrap();
        assert_eq!((f.vevents, f.events.len(), f.events[0].uid.as_str()), (2, 1, "2"));
    }

    #[test]
    fn vtimezone_parts() {
        let ics = b"BEGIN:VCALENDAR\nBEGIN:VTIMEZONE\nTZID:Europe/Prague\nBEGIN:DAYLIGHT\nTZOFFSETFROM:+0100\nTZOFFSETTO:+0200\nDTSTART:19700329T020000\nRRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU\nEND:DAYLIGHT\n\
BEGIN:STANDARD\nTZOFFSETFROM:+0200\nTZOFFSETTO:+0100\nDTSTART:19701025T030000\nRRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU\nEND:STANDARD\nEND:VTIMEZONE\nEND:VCALENDAR\n";
        let f = all(ics);
        assert_eq!(f.name, "");
        assert_eq!(f.zones.len(), 1);
        let z = &f.zones[0];
        assert_eq!(z.tzid, "Europe/Prague");
        assert_eq!(z.parts.len(), 2);
        assert_eq!((z.parts[0].from, z.parts[0].to, z.parts[0].start), (3600, 7200, dt(1970, 3, 29, 2, 0)));
        assert_eq!(z.parts[1].rrule.as_deref(), Some("FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU"));
    }

    #[test]
    fn not_a_calendar_and_broken_nesting() {
        assert!(parse(b"<html><body>Sign in</body></html>", &mut |_| true).unwrap_err().contains("BEGIN:VCALENDAR"));
        assert!(parse(b"", &mut |_| true).is_err());
        // nespárované END a neuzavřená událost nespadnou
        let f = all(b"BEGIN:VCALENDAR\nEND:VALARM\nBEGIN:VEVENT\nDTSTART:20261019T100000Z\nEND:VEVENT\nBEGIN:VEVENT\nDTSTART:20261019T100000Z\n");
        assert_eq!((f.vevents, f.events.len()), (1, 1));
    }
}
