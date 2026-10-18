//! Tabulkové testy celé cesty feed → výskyty v okně (`collect`) na uložených vzorcích Google
//! a Outlooku a na malých ručních feedech. Místní pásmo je pevně Praha (ne pásmo stroje).

use super::tz::{Zone, builtin};
use super::*;

const GOOGLE: &[u8] = include_bytes!("../fixtures/calendar-google.ics");
const OUTLOOK: &[u8] = include_bytes!("../fixtures/calendar-outlook.ics");

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
