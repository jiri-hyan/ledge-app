//! Tabulkové testy celé cesty feed → výskyty v okně (`collect`) na uložených vzorcích Google
//! a Outlooku a na malých ručních feedech. Místní pásmo je pevně Praha (ne pásmo stroje).

use super::tz::{Zone, builtin};
use super::*;

const GOOGLE: &[u8] = include_bytes!("../fixtures/calendar-google.ics");
const OUTLOOK: &[u8] = include_bytes!("../fixtures/calendar-outlook.ics");
const MEETINGS: &[u8] = include_bytes!("../fixtures/calendar-meetings.ics");

fn prague() -> Zone {
    builtin("Europe/Prague").unwrap()
}

fn utc(y: i32, m: u32, d: u32, h: u32, mi: u32) -> i64 {
    ms(chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, 0).unwrap())
}

/// Neděle 18. 10. 2026 10:00 v Praze (letní čas, +2).
fn now() -> i64 {
    utc(2026, 10, 18, 8, 0)
}

fn iso(ms: i64) -> String {
    naive(ms).format("%Y-%m-%dT%H:%MZ").to_string()
}

/// (začátek UTC, konec UTC, celodenní, název, místo, odhad pásma)
fn rows(p: &Parsed) -> Vec<(String, String, bool, String, String, bool)> {
    p.occs.iter().map(|o| (iso(o.start), iso(o.end), o.all_day, o.title.clone(), o.location.clone(), o.tz_guess)).collect()
}

fn run(ics: &str) -> Vec<(String, String, bool, String, String, bool)> {
    rows(&collect(ics.as_bytes(), now(), &prague()).unwrap())
}

type Row<'a> = (&'a str, &'a str, bool, &'a str, &'a str);

fn same(got: &[(String, String, bool, String, String, bool)], want: &[Row]) {
    let got: Vec<Row> = got.iter().map(|(s, e, a, t, l, _)| (s.as_str(), e.as_str(), *a, t.as_str(), l.as_str())).collect();
    assert_eq!(got, want);
}

#[test]
fn google_sample() {
    let p = collect(GOOGLE, now(), &prague()).unwrap();
    assert_eq!((p.events, p.name.as_str()), (7, "Ukázkový kalendář"));
    let meet = "https://meet.google.com/abc-defg-hij";
    let want: &[Row] = &[
        // celodenní 18.–19. 10. (DTEND výlučně)
        ("2026-10-18T00:00Z", "2026-10-20T00:00Z", true, "Výlet do Brna", "Brno, Česko"),
        // standup 9:15 v Praze = 7:15 UTC v letním čase
        ("2026-10-19T07:15Z", "2026-10-19T07:30Z", false, "Standup", meet),
        // místo jen první řádek; escapovaný středník
        ("2026-10-19T12:00Z", "2026-10-19T13:00Z", false, "Oběd s Petrem; pak káva", "Lokál"),
        // 20. 10. přepsán (RECURRENCE-ID) na 10:00; 21. 10. EXDATE; zrušená schůzka chybí
        ("2026-10-20T08:00Z", "2026-10-20T08:15Z", false, "Standup (posunutý)", meet),
        ("2026-10-22T07:15Z", "2026-10-22T07:30Z", false, "Standup", meet),
        ("2026-10-23T07:15Z", "2026-10-23T07:30Z", false, "Standup", meet),
        // roční narozeniny od 1990
        ("2026-10-24T00:00Z", "2026-10-25T00:00Z", true, "Narozeniny – Jana", ""),
        // po změně času 25. 10. zůstává 9:15 místního = 8:15 UTC
        ("2026-10-26T08:15Z", "2026-10-26T08:30Z", false, "Standup", meet),
        ("2026-10-27T08:15Z", "2026-10-27T08:30Z", false, "Standup", meet),
        ("2026-10-28T08:15Z", "2026-10-28T08:30Z", false, "Standup", meet),
        ("2026-10-29T08:15Z", "2026-10-29T08:30Z", false, "Standup", meet),
        ("2026-10-30T08:15Z", "2026-10-30T08:30Z", false, "Standup", meet),
    ];
    same(&rows(&p), want);
    assert!(p.occs.iter().all(|o| !o.tz_guess));
}

#[test]
fn outlook_sample() {
    let p = collect(OUTLOOK, now(), &prague()).unwrap();
    assert_eq!((p.events, p.name.as_str()), (4, "Kalendář"));
    let r = rows(&p);
    let want: &[Row] = &[
        ("2026-10-19T00:00Z", "2026-10-20T00:00Z", true, "Dovolená", ""),
        // třetí úterý (BYDAY=TU;BYSETPOS=3), Windows název pásma z VTIMEZONE
        ("2026-10-20T12:00Z", "2026-10-20T13:00Z", false, "Měsíční přehled", "Microsoft Teams Meeting"),
        // neznámé pásmo → místní (Praha)
        ("2026-10-22T14:00Z", "2026-10-22T15:00Z", false, "Hovor s klientem", ""),
        // obden týden po změně času
        ("2026-10-28T07:30Z", "2026-10-28T08:00Z", false, "Konzultace", "Kancelář 4.12"),
    ];
    same(&r, want);
    let guess: Vec<bool> = r.iter().map(|x| x.5).collect();
    assert_eq!(guess, [false, false, true, false]);
}

/// Odkaz Připojit na reálných tvarech pozvánek: popis Outlooku s `<https://…>` přes zalomené řádky,
/// `X-MICROSOFT-SKYPETEAMSMEETINGURL`, `X-GOOGLE-CONFERENCE`, Zoom v místě, Safe Links, URL, HTML popis.
#[test]
fn meeting_links_sample() {
    use super::meeting::Provider::*;
    let p = collect(MEETINGS, now(), &prague()).unwrap();
    type Got<'a> = Vec<(String, &'a str, Option<(meeting::Provider, &'a str)>)>;
    let got: Got = p.occs.iter().map(|o| (iso(o.start), o.title.as_str(), o.join.as_ref().map(|j| (j.provider, j.url.as_str())))).collect();
    let teams = "https://teams.microsoft.com/l/meetup-join/19%3ameeting_NjM4ZTk1YjQtZmQ0Ni00ZWM2%40thread.v2/0?context=%7b%22Tid%22%3a%2272f988bf%22%2c%22Oid%22%3a%22a1b2c3%22%7d";
    let want: Got = vec![
        ("2026-10-19T08:00Z".into(), "Týmová porada", Some((Teams, teams))),
        ("2026-10-19T11:00Z".into(), "Oběd", None),
        ("2026-10-19T12:00Z".into(), "Kafe s Janou", Some((Meet, "https://meet.google.com/xyz-abcd-efg"))),
        ("2026-10-20T11:00Z".into(), "Plánování sprintu", Some((Teams, "https://teams.microsoft.com/l/meetup-join/19%3ameeting_Sprint%40thread.v2/0?context=%7b%22Tid%22%3a%2272f988bf%22%7d"))),
        ("2026-10-21T09:00Z".into(), "Zoom s klientem", Some((Zoom, "https://us02web.zoom.us/j/81234567890?pwd=QWErty12"))),
        ("2026-10-22T08:00Z".into(), "Webex školení", Some((Webex, "https://firma.webex.com/firma/j.php?MTID=m0123abcd"))),
        ("2026-10-23T15:00Z".into(), "Retro", Some((Jitsi, "https://meet.jit.si/LedgeRetro"))),
        ("2026-10-24T09:00Z".into(), "Konzultace", Some((Teams, "https://teams.microsoft.com/meet/34567890123?p=AbCdEf123"))),
    ];
    assert_eq!(got, want);
    // popis se v paměti nedrží: místo zůstává jen první řádek
    assert_eq!(p.occs[0].location, "Microsoft Teams Meeting");
}

fn cal(body: &str) -> String {
    format!("BEGIN:VCALENDAR\nVERSION:2.0\n{body}END:VCALENDAR\n")
}

#[test]
fn small_feeds_table() {
    let cases: &[(&str, &str, &[Row])] = &[
        (
            "plovoucí čas = místní, DURATION",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART:20261019T090000\nDURATION:PT45M\nEND:VEVENT\n",
            &[("2026-10-19T07:00Z", "2026-10-19T07:45Z", false, "A", "")],
        ),
        (
            "bez konce = nulová délka; vestavěné pásmo bez VTIMEZONE",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART;TZID=America/New_York:20261019T090000\nEND:VEVENT\n",
            &[("2026-10-19T13:00Z", "2026-10-19T13:00Z", false, "A", "")],
        ),
        (
            "celodenní bez DTEND = 1 den, s DURATION = víc dní",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART;VALUE=DATE:20261020\nEND:VEVENT\nBEGIN:VEVENT\nUID:b\nSUMMARY:B\nDTSTART;VALUE=DATE:20261021\nDURATION:P3D\nEND:VEVENT\n",
            &[("2026-10-20T00:00Z", "2026-10-21T00:00Z", true, "A", ""), ("2026-10-21T00:00Z", "2026-10-24T00:00Z", true, "B", "")],
        ),
        (
            "RDATE přidá, EXDATE v UTC ubere",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART:20261019T080000Z\nDTEND:20261019T090000Z\nRRULE:FREQ=DAILY;COUNT=3\nEXDATE:20261020T080000Z\nRDATE:20261025T080000Z\nEND:VEVENT\n",
            &[
                ("2026-10-19T08:00Z", "2026-10-19T09:00Z", false, "A", ""),
                ("2026-10-21T08:00Z", "2026-10-21T09:00Z", false, "A", ""),
                ("2026-10-25T08:00Z", "2026-10-25T09:00Z", false, "A", ""),
            ],
        ),
        (
            "EXDATE jako datum u časové události",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART;TZID=Europe/Prague:20261019T090000\nRRULE:FREQ=DAILY;COUNT=2\nEXDATE;VALUE=DATE:20261019\nEND:VEVENT\n",
            &[("2026-10-20T07:00Z", "2026-10-20T07:00Z", false, "A", "")],
        ),
        (
            "UNTIL v UTC u pásmové události (včetně)",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART;TZID=Europe/Prague:20261019T090000\nRRULE:FREQ=DAILY;UNTIL=20261020T070000Z\nEND:VEVENT\n",
            &[("2026-10-19T07:00Z", "2026-10-19T07:00Z", false, "A", ""), ("2026-10-20T07:00Z", "2026-10-20T07:00Z", false, "A", "")],
        ),
        (
            "zrušený výskyt přes RECURRENCE-ID + STATUS:CANCELLED",
            "BEGIN:VEVENT\nUID:s\nSUMMARY:S\nDTSTART:20261019T080000Z\nRRULE:FREQ=DAILY;COUNT=3\nEND:VEVENT\nBEGIN:VEVENT\nUID:s\nRECURRENCE-ID:20261020T080000Z\nDTSTART:20261020T080000Z\nSTATUS:CANCELLED\nSUMMARY:S\nEND:VEVENT\n",
            &[("2026-10-19T08:00Z", "2026-10-19T08:00Z", false, "S", ""), ("2026-10-21T08:00Z", "2026-10-21T08:00Z", false, "S", "")],
        ),
        (
            "přepis bez hlavní události se ukáže sám",
            "BEGIN:VEVENT\nUID:x\nRECURRENCE-ID:20261001T080000Z\nDTSTART:20261019T100000Z\nSUMMARY:X\nEND:VEVENT\n",
            &[("2026-10-19T10:00Z", "2026-10-19T10:00Z", false, "X", "")],
        ),
        (
            "nepodporované FREQ = jen jednou v DTSTART",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nDTSTART:20261019T080000Z\nRRULE:FREQ=HOURLY;COUNT=5\nEND:VEVENT\n",
            &[("2026-10-19T08:00Z", "2026-10-19T08:00Z", false, "A", "")],
        ),
        (
            "okno: běžící od včera je vidět, mimo okno ne",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:Probíhá\nDTSTART:20261016T080000Z\nDTEND:20261017T090000Z\nEND:VEVENT\n\
BEGIN:VEVENT\nUID:b\nSUMMARY:Skončilo\nDTSTART:20261016T080000Z\nDTEND:20261017T070000Z\nEND:VEVENT\n\
BEGIN:VEVENT\nUID:c\nSUMMARY:Daleko\nDTSTART:20261102T080000Z\nEND:VEVENT\n",
            &[("2026-10-16T08:00Z", "2026-10-17T09:00Z", false, "Probíhá", "")],
        ),
        (
            "název na jeden řádek, bez názvu prázdný",
            "BEGIN:VEVENT\nUID:a\nSUMMARY:Dva\\nřádky   a  mezery\nDTSTART:20261019T080000Z\nEND:VEVENT\nBEGIN:VEVENT\nUID:b\nDTSTART:20261019T090000Z\nLOCATION:\\n\\nAž třetí řádek\nEND:VEVENT\n",
            &[("2026-10-19T08:00Z", "2026-10-19T08:00Z", false, "Dva řádky a mezery", ""), ("2026-10-19T09:00Z", "2026-10-19T09:00Z", false, "", "Až třetí řádek")],
        ),
    ];
    for (name, body, want) in cases {
        let got = run(&cal(body));
        let got: Vec<Row> = got.iter().map(|(s, e, a, t, l, _)| (s.as_str(), e.as_str(), *a, t.as_str(), l.as_str())).collect();
        assert_eq!(&got, want, "{name}");
    }
}

#[test]
fn errors_and_limits() {
    assert!(collect(b"<!doctype html><title>Sign in</title>", now(), &prague()).is_err());
    // prázdný kalendář = žádné události, ne chyba
    let p = collect(cal("").as_bytes(), now(), &prague()).unwrap();
    assert_eq!((p.events, p.occs.len()), (0, 0));
    // dlouhý název se zkrátí
    let long = "x".repeat(500);
    let r = run(&cal(&format!("BEGIN:VEVENT\nUID:a\nSUMMARY:{long}\nDTSTART:20261019T080000Z\nEND:VEVENT\n")));
    assert_eq!(r[0].3.chars().count(), MAX_TITLE);
    assert!(r[0].3.ends_with('…'));
    // nekonečná událost po minutách se nepodporuje, denní 15 let zpět doběhne a ořízne se na okno
    let p = collect(cal("BEGIN:VEVENT\nUID:a\nDTSTART:20110101T080000Z\nRRULE:FREQ=DAILY\nEND:VEVENT\n").as_bytes(), now(), &prague()).unwrap();
    assert_eq!(p.occs.len(), 15);
}

#[test]
fn clip_table() {
    for (s, max, want) in [("abc", 5, "abc"), ("abcdef", 5, "abcd…"), ("  a \n b  ", 9, "a b"), ("", 3, ""), ("ěščřž", 4, "ěšč…")] {
        assert_eq!(clip(s, max), want, "{s}");
    }
}

use crate::testrng::Rng;

/// Feed z internetu je cizí vstup: žádná kombinace (i s vícebajtovými znaky uprostřed hodnot) nesmí spadnout
/// ani se točit věčně. Výsledek je `Ok`/`Err`, nic víc.
#[test]
fn hostile_feeds_never_panic_or_hang() {
    let t0 = std::time::Instant::now();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let rule_bits = [
        "FREQ=", "DAILY", "WEEKLY", "MONTHLY", "YEARLY", "SECONDLY", ";", "INTERVAL=", "0", "1", "99999999999", "-1", "COUNT=", "4294967295", "UNTIL=", "20261231T225959Z", "20261231", "BYDAY=", "MO", "-1FR", "5SU", "éM", "ěš", "M", "BYMONTHDAY=", "31", "-31", "32", "BYMONTH=", "13", "2", "BYSETPOS=", "-366", "WKST=", "SU", ",", "=", "ů",
    ];
    let value_bits = ["2026", "10", "19", "T", "090000", "Z", "-", "+", "P", "1D", "T1H", "15M", "9999999999999999999", "é", "ů", ":", ";", "\"", "\\", "/", " ", "00", "99", "0230", "20260230", "8"];
    let names = ["DTSTART", "DTEND", "DURATION", "RRULE", "EXDATE", "RDATE", "SUMMARY", "LOCATION", "UID", "RECURRENCE-ID", "TZID", "BEGIN", "END", "X-ĚŠČ"];
    let mut accepted = 0;
    for i in 0..1500 {
        let mut body = String::new();
        for _ in 0..rng.below(3) {
            body.push_str("BEGIN:VEVENT\nUID:a\n");
            for _ in 0..rng.below(7) {
                let name = rng.pick(&names);
                let params = rng.pick(&["", ";TZID=Europe/Prague", ";TZID=\"Zvláštní ů\"", ";VALUE=DATE", ";TZID=", ";X=\"a:b\""]);
                let value = match name {
                    "RRULE" => rng.soup(&rule_bits, 8),
                    "BEGIN" | "END" => rng.pick(&["VEVENT", "VTIMEZONE", "STANDARD", "DAYLIGHT", "VCALENDAR", ""]).to_string(),
                    _ => rng.soup(&value_bits, 10),
                };
                body.push_str(&format!("{name}{params}:{value}\n"));
            }
            if rng.next().is_multiple_of(2) {
                body.push_str("END:VEVENT\n");
            }
        }
        if rng.next().is_multiple_of(4) {
            body.push_str("BEGIN:VTIMEZONE\nTZID:Zvláštní\nBEGIN:STANDARD\n");
            body.push_str(&format!("DTSTART:{}\nTZOFFSETFROM:{}\nTZOFFSETTO:{}\nRRULE:{}\nEND:STANDARD\nEND:VTIMEZONE\n", rng.soup(&value_bits, 6), rng.soup(&value_bits, 3), rng.soup(&value_bits, 3), rng.soup(&rule_bits, 8)));
        }
        let text = cal(&body);
        let mut bytes = text.into_bytes();
        // občas uříznout uprostřed (i vícebajtového znaku) nebo vložit nevalidní bajt
        match rng.next() % 5 {
            0 if !bytes.is_empty() => bytes.truncate(rng.below(bytes.len())),
            1 if !bytes.is_empty() => {
                let at = rng.below(bytes.len());
                bytes.insert(at, 0xff);
            }
            _ => {}
        }
        let r = std::panic::catch_unwind(|| collect(&bytes, now(), &prague()).map(|p| p.occs.len()));
        assert!(r.is_ok(), "panika v kole {i}: {}", String::from_utf8_lossy(&bytes));
        accepted += usize::from(r.is_ok_and(|x| x.is_ok()));
    }
    assert!(accepted > 300, "test by nic neprověřil: přijato jen {accepted} feedů z 1500");
    assert!(t0.elapsed() < std::time::Duration::from_secs(60), "zpracování zhrublo: {:?}", t0.elapsed());
}

/// Hodnoty s vícebajtovým znakem na hranici dvou bajtů (kde se `split_at` / `&s[..n]` láme).
#[test]
fn multibyte_values_do_not_break_parsers() {
    use super::{expand, ics};
    for rule in ["FREQ=WEEKLY;BYDAY=éM", "FREQ=WEEKLY;BYDAY=é", "FREQ=WEEKLY;BYDAY=ůů", "FREQ=DAILY;UNTIL=2026éé", "FREQ=DAILY;INTERVAL=é", "FREQ=MONTHLY;BYMONTHDAY=é"] {
        assert!(expand::parse_rule(rule).is_err(), "{rule}");
    }
    for v in ["éé", "P1é", "-é", "+é1", "PTé", "é"] {
        assert_eq!(ics::parse_duration(v), None, "{v}");
        assert_eq!(ics::parse_offset(v), None, "{v}");
    }
    for v in ["éééé", "2026101é", "20261019Té0000", "ééééééé", "é0261019"] {
        assert_eq!(ics::parse_dt(v, None, false), None, "{v}");
        let _ = ics::parse_dt(v, None, true); // `VALUE=DATE` bere první osmici znaků — jen nesmí spadnout
    }
}
