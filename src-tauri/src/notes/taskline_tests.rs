use super::*;
use crate::testrng::Rng;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    #[serde(rename = "in")]
    input: String,
    text: String,
    due: Option<String>,
    prio: u8,
    ev: Option<String>,
    #[serde(default)]
    cal: bool,
}

fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!("taskline_cases.json")).unwrap()
}

fn due_str(t: &TaskLine) -> Option<String> {
    t.due.map(|d| due_text(d, t.all_day))
}

/// Tabulka sdílená s TS zrcadlem (`src/notes/taskline.test.mjs`).
#[test]
fn parse_tail_shared_table() {
    for c in cases() {
        let t = parse_tail(&c.input);
        assert_eq!(t.text, c.text, "text: {:?}", c.input);
        assert_eq!(due_str(&t), c.due, "due: {:?}", c.input);
        assert_eq!(t.prio, c.prio, "prio: {:?}", c.input);
        assert_eq!(t.evidence().map(str::to_string), c.ev, "důkaz: {:?}", c.input);
        assert_eq!(t.cal, c.cal, "cal: {:?}", c.input);
        assert_eq!(render(&t), c.input, "round-trip: {:?}", c.input);
    }
}

fn dt(s: &str) -> (NaiveDateTime, bool) {
    match s.len() {
        10 => (NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap().and_time(NaiveTime::default()), true),
        _ => (NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap(), false),
    }
}

#[test]
fn set_due_and_prio_change_only_their_token() {
    let d = |s| Some(dt(s));
    assert_eq!(set_due("Doktor", d("2026-11-14 09:00")), "Doktor ⏰ 2026-11-14 09:00");
    assert_eq!(set_due("Doktor !", d("2026-11-14")), "Doktor ⏰ 2026-11-14 !");
    assert_eq!(set_due("Doktor ⏰ 2026-11-14 09:00 ! — důkaz: x", d("2026-11-15")), "Doktor ⏰ 2026-11-15 ! — důkaz: x");
    assert_eq!(set_due("Doktor ⏰ 2026-11-14 09:00 !", None), "Doktor !");
    assert_eq!(set_due("", d("2026-11-14")), "⏰ 2026-11-14");
    // odložení čtené `📅` ho přepíše na `⏰`, jinak zůstává
    assert_eq!(set_due("a 📅 2026-11-14 !", d("2026-11-16")), "a ⏰ 2026-11-16 !");
    assert_eq!(set_prio("a 📅 2026-11-14", 1), "a 📅 2026-11-14 !");
    assert_eq!(set_prio("a ⏰ 2026-11-14 !!", 0), "a ⏰ 2026-11-14");
    assert_eq!(set_prio("a", 9), "a !!");
    // CRLF, cizí text a koncové mezery zůstanou
    assert_eq!(set_prio("a #t @x/y.ts:3  ", 1), "a #t @x/y.ts:3 !  ");
}

#[test]
fn edit_line_touches_one_line_only() {
    let body = "# h\r\n- [ ] doktor\r\n```\n- [ ] kód\n```\n  1. [~] b ⏰ 2026-11-14 — důkaz: c\n- [x] hotovo\n";
    let out = edit_line(body, 1, |t| set_due(t, Some(dt("2026-11-20 08:30")))).unwrap();
    assert_eq!(out, body.replace("- [ ] doktor", "- [ ] doktor ⏰ 2026-11-20 08:30"));
    let out = edit_line(body, 5, |t| set_due(t, None)).unwrap();
    assert_eq!(out, body.replace("b ⏰ 2026-11-14 — důkaz: c", "b — důkaz: c"));
    assert!(edit_line(body, 3, |t| t.into()).is_err(), "blok kódu");
    assert!(edit_line(body, 0, |t| t.into()).is_err(), "nadpis");
    assert!(edit_line(body, 99, |t| t.into()).is_err());
}

#[test]
fn old_notes_without_tokens_keep_their_text() {
    for s in ["Opravit header @src/Header.tsx:42", "Dokončit login — důkaz: commit abc123", "x - dukaz: y", "bez důkazu - a pak", "", "  "] {
        let t = parse_tail(s);
        assert_eq!((t.due, t.prio), (None, 0), "{s:?}");
        let (txt, ev) = split_evidence(s);
        assert_eq!(t.head, txt, "{s:?}");
        assert!(t.text.len() <= s.len());
        assert_eq!(ev, t.evidence(), "{s:?}");
    }
}

#[test]
fn prio_word_and_connector_scale() {
    for (w, p) in [("!", Some(4)), ("!!", Some(5)), ("!!!", Some(5)), ("!!!!", None), ("!3", Some(3)), ("!6", None), ("Pozor!", None), ("a!b", None), ("!,", Some(4))] {
        assert_eq!(prio_word(w), p, "{w}");
    }
}

/// ~40 000 náhodných složenin: zápis ∘ čtení = identita, `set_*` bez změny je no-op, nastavení se přečte zpět.
#[test]
fn round_trip_property() {
    let bits = [
        "a", "Doktor", " ", " ", " ", "  ", "\t", "⏰", "📅", "2026-11-14", "2026-02-30", "09:00", "9:00", "25:00", "!", "!!", "!!!", "!x", "x!", "Pozor!", "—", "–", "-", "důkaz:", "dukaz:", "Evidence:", " — důkaz: ", "@src/a.ts:4",
        "#t", "é", "🙂", "\r", "`", "http://x/!",
    ];
    let mut rng = Rng(0x7A5C_11E5_0DD5_EED1);
    let target = NaiveDate::from_ymd_opt(2031, 1, 2).unwrap();
    for i in 0..40_000 {
        let s = rng.soup(&bits, 12);
        let t = parse_tail(&s);
        assert_eq!(render(&t), s, "round-trip #{i}: {s:?}");
        assert_eq!(t.head.len() + t.evidence_raw.len() + t.trail.len(), s.len(), "rozklad #{i}: {s:?}");
        assert_eq!(set_prio(&s, t.prio), s, "set_prio no-op #{i}: {s:?}");
        if !t.cal {
            let cur = t.due.map(|d| (d, t.all_day));
            assert_eq!(set_due(&s, cur), s, "set_due no-op #{i}: {s:?}");
        }
        // nastavení se přečte zpět (a nezměněný zbytek zůstane)
        for (d, all) in [(target.and_hms_opt(8, 5, 0).unwrap(), false), (target.and_time(NaiveTime::default()), true)] {
            let n = set_due(&s, Some((d, all)));
            let r = parse_tail(&n);
            assert_eq!((r.due, r.all_day), (Some(d), all), "set_due #{i}: {s:?} → {n:?}");
            assert_eq!(r.evidence_raw, t.evidence_raw, "důkaz #{i}: {s:?}");
        }
        let n = set_prio(&s, 2);
        assert_eq!(parse_tail(&n).prio, 2, "set_prio #{i}: {s:?} → {n:?}");
    }
}

#[derive(Deserialize)]
struct ItemCase {
    #[serde(rename = "in")]
    input: String,
    task: bool,
    #[serde(default)]
    indent: usize,
    #[serde(default)]
    state: String,
    #[serde(default)]
    rest: String,
}

/// Tabulka řádku úkolu sdílená s TS `taskLineOf` (`src/notes/taskline.test.mjs`).
#[test]
fn task_item_shared_table() {
    let cases: Vec<ItemCase> = serde_json::from_str(include_str!("taskitem_cases.json")).unwrap();
    for c in cases {
        let got = crate::notes::md::list_item(&c.input).and_then(|(ind, _, r)| crate::notes::md::task_state(r).map(|(s, t)| (ind, s, t)));
        match got {
            None => assert!(!c.task, "má být úkol: {:?}", c.input),
            Some((ind, s, t)) => {
                assert!(c.task, "nemá být úkol: {:?}", c.input);
                assert_eq!(ind, c.indent, "odsazení: {:?}", c.input);
                assert_eq!(format!("{s:?}").to_lowercase(), c.state, "stav: {:?}", c.input);
                assert_eq!(t, c.rest, "text: {:?}", c.input);
            }
        }
    }
}
