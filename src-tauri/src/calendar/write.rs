//! Zápis do kalendáře bez nového oprávnění (L74): z úkolu / připomínky vznikne soubor `.ics` (otevře se ve
//! výchozím kalendáři) nebo událost v propojené aplikaci (`create_event`). Tady jen ČISTÁ logika — čas, uid, text
//! VEVENT (RFC 5545) a argumenty `create_event`; zápis souboru a síť jsou v `commands/calendar.rs`.
//! Čas: místní čas bez offsetu se převede podle pásma počítače (`LocalTz`), s offsetem / `Z` se bere jak je;
//! do `.ics` jde vždy UTC (`…Z`), do propojené aplikace ISO s offsetem. Celodenní = `VALUE=DATE` / `date`.

use super::tz::LocalTz;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Výchozí délka a předstih (UI posílá tytéž hodnoty; Rust je jen ořezává do rozumu).
pub const DEFAULT_DUR_MIN: u32 = 30;
pub const DEFAULT_REMIND_MIN: u32 = 10;
/// Nejdelší událost 24 h, nejdelší předstih týden (`0` = bez připomínky).
const MAX_DUR_MIN: u32 = 1440;
const MAX_REMIND_MIN: u32 = 7 * 1440;
/// Propojená aplikace: `name` ≤ 500 znaků.
const MAX_TITLE: usize = 500;

/// Kdy událost začíná.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// celodenní
    Day(NaiveDate),
    /// okamžik v UTC + posun místa zadání v sekundách (pro ISO s offsetem)
    At { utc: NaiveDateTime, offset: i32 },
}

/// Vstup `ics_event` / `create_event_args`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ev<'a> {
    pub title: &'a str,
    pub start: Start,
    pub dur_min: u32,
    /// `0` = bez připomínky
    pub remind_min: u32,
    pub uid: &'a str,
}

/// `YYYY-MM-DD` | `YYYY-MM-DDTHH:MM[:SS]` (místní; i s mezerou místo `T`) | totéž s `Z` / `±HH:MM` / `±HHMM`.
pub fn parse_start(iso: &str, local: &dyn LocalTz) -> Result<Start, String> {
    let bad = || format!("neplatný čas události: {iso}");
    let s = iso.trim();
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(Start::Day(d));
    }
    let (body, off) = split_offset(s);
    let naive = ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(body, f).ok())
        .ok_or_else(bad)?;
    let o = off.unwrap_or_else(|| local.offset_local(naive));
    Ok(Start::At { utc: naive - Duration::seconds(o as i64), offset: o })
}

/// Odřízne příponu pásma: (zbytek, posun v sekundách). `+`/`-` se hledá až za datem, ať ho nezmýlí pomlčky.
fn split_offset(s: &str) -> (&str, Option<i32>) {
    if let Some(r) = s.strip_suffix(['Z', 'z']) {
        return (r, Some(0));
    }
    let from = s.find(['T', ' ']).unwrap_or(0);
    if let Some(i) = s[from..].rfind(['+', '-']).map(|i| i + from) {
        let sign = if s.as_bytes()[i] == b'-' { -1 } else { 1 };
        let digits: String = s[i + 1..].chars().filter(|c| *c != ':').collect();
        if digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit()) {
            let (h, m): (i32, i32) = (digits[..2].parse().unwrap_or(99), digits[2..].parse().unwrap_or(99));
            if h < 24 && m < 60 {
                return (&s[..i], Some(sign * (h * 3600 + m * 60)));
            }
        }
    }
    (s, None)
}

/// Stabilní `UID` z libovolného semene (cesta poznámky + řádek / text): opakovaný klik dá tentýž.
pub fn uid(seed: &str) -> String {
    format!("{}@ledge", hex16(seed))
}

/// `clientRequestId` pro `create_event` (8–128 znaků): tentýž klik = tatáž událost, ne duplicita.
pub fn request_id(seed: &str) -> String {
    format!("ledge-cal-{}", hex16(seed))
}

/// Jméno souboru `.ics` (stejné semeno = stejný soubor, přepíše se).
pub fn file_name(seed: &str) -> String {
    format!("ledge-{}.ics", hex16(seed))
}

fn hex16(seed: &str) -> String {
    Sha256::digest(seed.trim().as_bytes())[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// Meze vstupu: délka 1–1440 min (0 → výchozí), předstih 0–10 080 min.
pub fn clamp_dur(m: u32) -> u32 {
    if m == 0 { DEFAULT_DUR_MIN } else { m.min(MAX_DUR_MIN) }
}
pub fn clamp_remind(m: u32) -> u32 {
    m.min(MAX_REMIND_MIN)
}

/// Název: bez zalomení a krajních mezer, 1–500 znaků.
pub fn clean_title(t: &str) -> Result<String, String> {
    let one = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.is_empty() {
        return Err("událost bez názvu".into());
    }
    Ok(one.chars().take(MAX_TITLE).collect())
}

/// Text hodnoty dle RFC 5545 §3.3.11: `\` `;` `,` a zalomení.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            ';' => o.push_str("\\;"),
            ',' => o.push_str("\\,"),
            '\n' => o.push_str("\\n"),
            '\r' => {}
            c => o.push(c),
        }
    }
    o
}

/// Zalomení řádku na 75 oktetů (RFC 5545 §3.1): pokračování začíná mezerou (ta se do 75 počítá), znak UTF-8 se nikdy nerozdělí.
pub fn fold(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + line.len() / 70 * 3 + 2);
    let mut used = 0;
    for c in line.chars() {
        let n = c.len_utf8();
        if used + n > 75 {
            out.push_str("\r\n ");
            used = 1;
        }
        out.push(c);
        used += n;
    }
    out.push_str("\r\n");
    out
}

fn stamp(t: NaiveDateTime) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Začátek a konec: celodenní = následující den (DTEND je exkluzivní), jinak začátek + délka.
fn ends(ev: &Ev) -> (String, String) {
    match ev.start {
        Start::Day(d) => (format!("DTSTART;VALUE=DATE:{}", d.format("%Y%m%d")), format!("DTEND;VALUE=DATE:{}", (d + Duration::days(1)).format("%Y%m%d"))),
        Start::At { utc, .. } => (format!("DTSTART:{}", stamp(utc)), format!("DTEND:{}", stamp(utc + Duration::minutes(clamp_dur(ev.dur_min) as i64)))),
    }
}

/// Celý soubor `.ics` s jednou událostí a (jen u časované a `remind_min > 0`) upozorněním `VALARM`.
/// `now` = `DTSTAMP` (UTC) — předává volající, ať je funkce čistá. Celodenní událost upozornění nemá
/// (předstih před půlnocí nedává smysl).
pub fn ics_event(ev: &Ev, now: NaiveDateTime) -> String {
    let (s, e) = ends(ev);
    let title = clean_title(ev.title).unwrap_or_default();
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".into(),
        "PRODID:-//Ledge//Kalendar//CS".into(),
        "CALSCALE:GREGORIAN".into(),
        "BEGIN:VEVENT".into(),
        format!("UID:{}", ev.uid),
        format!("DTSTAMP:{}", stamp(now)),
        // stejné UID se po odložení času aktualizuje: klient bere vyšší SEQUENCE jako novější verzi (minuty od epochy)
        format!("SEQUENCE:{}", now.and_utc().timestamp() / 60),
        s,
        e,
        format!("SUMMARY:{}", esc(&title)),
    ];
    if matches!(ev.start, Start::At { .. }) && ev.remind_min > 0 {
        lines.extend([
            "BEGIN:VALARM".into(),
            "ACTION:DISPLAY".into(),
            format!("DESCRIPTION:{}", esc(&title)),
            format!("TRIGGER:-PT{}M", clamp_remind(ev.remind_min)),
            "END:VALARM".into(),
        ]);
    }
    lines.extend(["END:VEVENT".into(), "END:VCALENDAR".into()]);
    lines.iter().map(|l| fold(l)).collect()
}

/// Argumenty `create_event` propojené aplikace:
/// `name`, `date` (celodenní, `YYYY-MM-DD`) nebo `startDateTime` + `endDateTime` (ISO s offsetem),
/// `description`, `clientRequestId`. Typ, priorita a stav se nechají na výchozích hodnotách aplikace.
pub fn create_event_args(ev: &Ev, description: &str, request_id: &str) -> Value {
    let mut a = json!({ "name": clean_title(ev.title).unwrap_or_default(), "description": description, "clientRequestId": request_id });
    match ev.start {
        Start::Day(d) => a["date"] = json!(d.format("%Y-%m-%d").to_string()),
        Start::At { utc, offset } => {
            let end = utc + Duration::minutes(clamp_dur(ev.dur_min) as i64);
            a["startDateTime"] = json!(iso_offset(utc, offset));
            a["endDateTime"] = json!(iso_offset(end, offset));
        }
    }
    a
}

/// `YYYY-MM-DDTHH:MM:SS±HH:MM` — UTC okamžik zapsaný v pásmu s posunem `offset`.
pub fn iso_offset(utc: NaiveDateTime, offset: i32) -> String {
    let local = utc + Duration::seconds(offset as i64);
    let (sign, o) = if offset < 0 { ('-', -offset) } else { ('+', offset) };
    format!("{}{sign}{:02}:{:02}", local.format("%Y-%m-%dT%H:%M:%S"), o / 3600, o % 3600 / 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pevné pásmo „Praha“ pro testy: duben–září +02:00, jinak +01:00 (stačí pro tabulky níž).
    struct Cet;
    impl LocalTz for Cet {
        fn offset_local(&self, t: NaiveDateTime) -> i32 {
            use chrono::Datelike;
            if (4..=9).contains(&t.month()) { 7200 } else { 3600 }
        }
        fn offset_utc(&self, t: NaiveDateTime) -> i32 {
            self.offset_local(t)
        }
    }

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn parse_start_table() {
        let at = |utc: &str, offset| Ok(Start::At { utc: dt(utc), offset });
        let cases: Vec<(&str, Result<Start, ()>)> = vec![
            ("2026-11-14", Ok(Start::Day(NaiveDate::from_ymd_opt(2026, 11, 14).unwrap()))),
            ("2026-11-14T09:00", at("2026-11-14 08:00:00", 3600)),
            ("2026-11-14 09:00", at("2026-11-14 08:00:00", 3600)),
            ("2026-07-01T09:00", at("2026-07-01 07:00:00", 7200)),
            ("2026-07-01T09:00:30", at("2026-07-01 07:00:30", 7200)),
            ("2026-07-01T09:00:00+02:00", at("2026-07-01 07:00:00", 7200)),
            ("2026-07-01T07:00:00Z", at("2026-07-01 07:00:00", 0)),
            ("2026-07-01T09:00-0530", at("2026-07-01 14:30:00", -19800)),
            ("  2026-07-01T09:00  ", at("2026-07-01 07:00:00", 7200)),
            ("zítra", Err(())),
            ("2026-02-30", Err(())),
            ("2026-13-01", Err(())),
            ("2026-07-01T25:00", Err(())),
            ("2026-07-01T09:00+99:00", Err(())),
            ("", Err(())),
        ];
        for (i, want) in cases {
            assert_eq!(parse_start(i, &Cet).map_err(|_| ()), want, "{i}");
        }
    }

    #[test]
    fn uid_is_stable() {
        assert_eq!(uid("poznamka.md|3|Doktor"), uid("  poznamka.md|3|Doktor "));
        assert_ne!(uid("a.md|1"), uid("a.md|2"));
        assert!(uid("x").ends_with("@ledge") && uid("x").len() == 38);
        let r = request_id("x");
        assert!(r.starts_with("ledge-cal-") && (8..=128).contains(&r.len()), "{r}");
        assert_eq!(file_name("x"), format!("ledge-{}.ics", &uid("x")[..32]));
        assert!(!file_name("..\\x/y").contains(['\\', '/']));
    }

    #[test]
    fn escape_and_title() {
        assert_eq!(esc("a,b;c\\d\ne\r"), "a\\,b\\;c\\\\d\\ne");
        assert_eq!(clean_title("  Ahoj\n  svět \t").unwrap(), "Ahoj svět");
        assert!(clean_title(" \n ").is_err());
        assert_eq!(clean_title(&"ž".repeat(600)).unwrap().chars().count(), 500);
        assert_eq!((clamp_dur(0), clamp_dur(45), clamp_dur(5000)), (30, 45, 1440));
        assert_eq!((clamp_remind(0), clamp_remind(10), clamp_remind(999_999)), (0, 10, 10_080));
    }

    fn unfold(s: &str) -> String {
        s.replace("\r\n ", "")
    }

    #[test]
    fn fold_75_octets_utf8() {
        for line in ["x".repeat(200), "SUMMARY:".to_string() + &"Žluťoučký kůň úpěl ďábelské ódy, ".repeat(8), "é".repeat(80), String::new()] {
            let f = fold(&line);
            assert!(f.ends_with("\r\n"));
            for phys in f.split("\r\n").filter(|p| !p.is_empty()) {
                assert!(phys.len() <= 75, "{} B: {phys}", phys.len());
            }
            assert_eq!(unfold(&f), format!("{line}\r\n"));
        }
        assert_eq!(fold("a".repeat(75).as_str()), format!("{}\r\n", "a".repeat(75)));
        assert_eq!(fold("a".repeat(76).as_str()).matches("\r\n ").count(), 1);
    }

    fn timed(title: &str, start: &str, dur: u32, remind: u32) -> String {
        let s = parse_start(start, &Cet).unwrap();
        ics_event(&Ev { title, start: s, dur_min: dur, remind_min: remind, uid: "u1@ledge" }, dt("2026-11-13 10:00:00"))
    }

    #[test]
    fn ics_exact_timed() {
        let want = [
            "BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Ledge//Kalendar//CS", "CALSCALE:GREGORIAN", "BEGIN:VEVENT",
            "UID:u1@ledge", "DTSTAMP:20261113T100000Z", "SEQUENCE:29909400", "DTSTART:20261114T080000Z", "DTEND:20261114T083000Z",
            "SUMMARY:Doktor\\, Praha\\; zubař", "BEGIN:VALARM", "ACTION:DISPLAY", "DESCRIPTION:Doktor\\, Praha\\; zubař",
            "TRIGGER:-PT10M", "END:VALARM", "END:VEVENT", "END:VCALENDAR", "",
        ]
        .join("\r\n");
        assert_eq!(timed("Doktor, Praha; zubař", "2026-11-14T09:00", 30, 10), want);
    }

    #[test]
    fn ics_sequence_grows_so_a_rescheduled_event_updates_not_duplicates() {
        let seq = |t: &str| {
            let s = ics_event(&Ev { title: "X", start: parse_start("2026-11-14T09:00", &Cet).unwrap(), dur_min: 30, remind_min: 0, uid: "u1@ledge" }, dt(t));
            s.lines().find_map(|l| l.strip_prefix("SEQUENCE:").and_then(|n| n.parse::<i64>().ok())).unwrap()
        };
        assert!(seq("2026-11-13 10:00:00") < seq("2026-11-13 10:05:00"));
        assert!(seq("2026-11-13 10:05:00") < seq("2026-11-14 08:00:00"));
    }

    #[test]
    fn ics_variants() {
        // přes půlnoc: 23:45 Z + 30 min → další den
        let t = timed("Pozdě", "2026-11-14T23:45:00Z", 30, 0);
        assert!(t.contains("DTSTART:20261114T234500Z\r\n") && t.contains("DTEND:20261115T001500Z\r\n"), "{t}");
        assert!(!t.contains("VALARM"), "remind 0 = bez upozornění");
        // letní čas: 23:50 Praha = 21:50 UTC
        assert!(timed("x", "2026-07-01T23:50", 30, 5).contains("DTSTART:20260701T215000Z\r\n"));
        // celodenní: VALUE=DATE, konec = další den, bez VALARM
        let d = timed("Svátek", "2026-12-24", 30, 10);
        assert!(d.contains("DTSTART;VALUE=DATE:20261224\r\n") && d.contains("DTEND;VALUE=DATE:20261225\r\n"), "{d}");
        assert!(!d.contains("VALARM") && !d.contains("DTSTART:"), "{d}");
        // měsíc / rok přes hranici
        assert!(timed("Silvestr", "2026-12-31", 30, 0).contains("DTEND;VALUE=DATE:20270101\r\n"));
        // zalomení dlouhého názvu; každý řádek ≤ 75 B, odvinutý SUMMARY je celý
        let long = "Žluťoučký kůň úpěl ďábelské ódy a ".repeat(6);
        let l = timed(&long, "2026-11-14T09:00", 30, 10);
        assert!(l.split("\r\n").all(|p| p.len() <= 75));
        assert!(unfold(&l).contains(&format!("SUMMARY:{}\r\n", esc(&clean_title(&long).unwrap()))));
        // jen CRLF, žádný holý LF
        assert!(!l.replace("\r\n", "").contains('\n'));
        // délka 0 → výchozích 30 min
        assert!(timed("x", "2026-11-14T09:00", 0, 0).contains("DTEND:20261114T083000Z"));
    }

    #[test]
    fn connector_args() {
        let s = parse_start("2026-11-14T09:00", &Cet).unwrap();
        let ev = Ev { title: " Doktor ", start: s, dur_min: 45, remind_min: 10, uid: "u" };
        let a = create_event_args(&ev, "Z poznámky", "ledge-cal-abc");
        assert_eq!(
            a,
            json!({
                "name": "Doktor", "description": "Z poznámky", "clientRequestId": "ledge-cal-abc",
                "startDateTime": "2026-11-14T09:00:00+01:00", "endDateTime": "2026-11-14T09:45:00+01:00",
            })
        );
        let day = Ev { start: parse_start("2026-12-24", &Cet).unwrap(), ..ev.clone() };
        let b = create_event_args(&day, "", "ledge-cal-abc");
        assert_eq!(b["date"], "2026-12-24");
        assert!(b.get("startDateTime").is_none() && b.get("endDateTime").is_none());
        // záporný posun a půlhodinové pásmo
        assert_eq!(iso_offset(dt("2026-07-01 14:30:00"), -19800), "2026-07-01T09:00:00-05:30");
        assert_eq!(iso_offset(dt("2026-07-01 07:00:00"), 0), "2026-07-01T07:00:00+00:00");
    }
}
