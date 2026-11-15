use super::*;
use crate::testrng::Rng;
use chrono::NaiveDate;
use std::path::Path;

fn at(d: &str, h: u32, m: u32) -> i64 {
    local_from_naive(NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap().and_hms_opt(h, m, 0).unwrap()).unwrap().timestamp_millis()
}

fn note(name: &str, body: &str) -> NoteView {
    super::super::parse_note(Path::new(&format!("C:\\n\\{name}.md")), &format!("---\ncreated: 2026-11-01T10:00:00+01:00\n---\n{body}"), None, 0)
}

#[test]
fn index_table() {
    let body = "# h\n- [ ] doktor ⏰ 2026-11-14 09:00 !\n- [~] opravit ⏰ 2026-11-14 — důkaz: commit 1\n- [x] hotovo ⏰ 2026-11-01\n\
                1. [ ] faktura ⏰ 2026-11-15 !!\n- [ ] odesláno\n  ↳ [Demo](https://example.test/x)\n```\n- [ ] kód ⏰ 2026-11-01\n```\n- [ ] bez termínu\r\n";
    let ix = index(body);
    let got: Vec<_> = ix.items.iter().map(|t| (t.line, t.state, t.text.as_str(), t.prio, t.all_day, t.due_ms.is_some(), t.forwarded)).collect();
    use TaskState::*;
    assert_eq!(
        got,
        vec![
            (1, Open, "doktor", 1, false, true, false),
            (2, Proposed, "opravit", 0, true, true, false),
            (4, Open, "faktura", 2, true, true, false),
            (5, Open, "odesláno", 0, false, false, true),
            (10, Open, "bez termínu", 0, false, false, false),
        ]
    );
    assert_eq!((ix.done, ix.total), (1, 6));
    assert_eq!(ix.next_due_ms, Some(at("2026-11-14", 9, 0)), "navržený [~] a odeslaný se do nejbližšího termínu nepočítají");
    // celodenní: dueMs 9:00, po termínu od půlnoci
    let t = &ix.items[2];
    assert_eq!((t.due_ms, t.late_ms), (Some(at("2026-11-15", 9, 0)), Some(at("2026-11-16", 0, 0))));
}

#[test]
fn index_caps_items_and_text_but_keeps_all_dates() {
    let mut body: String = (0..80).map(|i| format!("- [ ] x{i}\n")).collect();
    body.push_str("- [ ] pozdní termín ⏰ 2026-12-24\n");
    body.push_str(&format!("- [ ] {}\n", "ž".repeat(300)));
    let ix = index(&body);
    assert_eq!(ix.items.len(), MAX_ITEMS);
    assert_eq!(ix.total, 82);
    assert_eq!(ix.next_due_ms, Some(at("2026-12-24", 9, 0)), "termín za stropem 50 bodů nezmizí z nejbližšího");
    let long = index(&format!("- [ ] {}\n", "ž".repeat(300)));
    assert_eq!(long.items[0].text.chars().count(), MAX_TEXT);
    assert!(long.items[0].text.ends_with('…'));
}

/// Staré poznámky bez tokenů: počty a body stejné jako `md::tasks` (L96, chování beze změny).
#[test]
fn counts_equal_md_tasks_on_random_bodies() {
    let bits = ["- [ ] ", "- [x] ", "- [~] ", "1. [ ] ", "text", " ", "\n", "\r\n", "```\n", "  ", "! ", "— důkaz: x", "é", "- ", "[ ] "];
    let mut rng = Rng(0x1D3A_77C0_55AA_0001);
    for i in 0..3000 {
        let body = rng.soup(&bits, 20);
        let t = crate::notes::md::tasks(&body);
        let ix = index(&body);
        assert_eq!((ix.done, ix.total), (t.iter().filter(|x| x.1).count(), t.len()), "#{i}: {body:?}");
        assert!(ix.items.iter().all(|x| t.iter().any(|y| y.0 == x.line as usize && !y.1)), "#{i}: {body:?}");
        assert!(ix.items.iter().all(|x| x.due_ms.is_none() && x.prio == 0 || body.contains('!')), "#{i}: {body:?}");
    }
}

fn pool() -> Vec<NoteView> {
    vec![
        note("a", "- [ ] dnes odpoledne ⏰ 2026-11-14 15:00 !\n- [ ] dnes celý den ⏰ 2026-11-14\n- [ ] včera ⏰ 2026-11-13\n- [ ] volné\n"),
        note("b", "- [ ] hodinu zpět ⏰ 2026-11-14 08:00\n- [~] navržené ⏰ 2026-11-12\n- [ ] příští týden ⏰ 2026-11-21\n- [ ] předáno ⏰ 2026-11-10\n  ↳ [Demo]\n"),
        NoteView { done: true, ..note("c", "- [ ] hotová poznámka ⏰ 2026-11-01\n") },
    ]
}

#[test]
fn list_buckets_counts_and_order() {
    let all = pool();
    let (now, eod) = (at("2026-11-14", 9, 0), at("2026-11-15", 0, 0));
    let l = list(&all, &TaskQuery::default(), now, eod);
    assert_eq!(l.counts, TaskCounts { overdue: 2, today: 2, proposed: 1, undated: 1 });
    // „předáno“ (↳ [) je v řádcích, ale nepočítá se
    let texts: Vec<_> = l.rows.iter().map(|r| (r.text.as_str(), r.forwarded)).collect();
    assert_eq!(
        texts,
        [("předáno", true), ("včera", false), ("hodinu zpět", false), ("dnes celý den", false), ("dnes odpoledne", false), ("příští týden", false), ("navržené", false), ("volné", false)]
    );
    assert_eq!(due_counts(&all, now, eod), DueCounts { overdue: 2, today: 2 });
    // celodenní „dnes“ je po termínu až od půlnoci
    let next_day = due_counts(&all, at("2026-11-15", 0, 0), at("2026-11-16", 0, 0));
    assert_eq!(next_day.overdue, 4);
    // filtr projektu a limit
    assert_eq!(list(&all, &TaskQuery { project: Some("x".into()), limit: None }, now, eod).rows.len(), 0);
    assert_eq!(list(&all, &TaskQuery { project: Some("_obecne".into()), limit: Some(2) }, now, eod).rows.len(), 2);
    assert_eq!(list(&[], &TaskQuery::default(), now, eod), TaskList { rows: vec![], counts: TaskCounts::default() });
}

#[test]
fn list_over_20000_items_is_fast() {
    let body: String = (0..50).map(|i| format!("- [ ] bod {i} ⏰ 2026-11-{:02} 10:00 !\n", 1 + i % 28)).collect();
    let all: Vec<NoteView> = (0..400).map(|i| note(&format!("n{i}"), &body)).collect();
    assert_eq!(all.iter().map(|n| n.items.len()).sum::<usize>(), 20_000);
    let (now, eod) = (at("2026-11-14", 9, 0), at("2026-11-15", 0, 0));
    let best = (0..5)
        .map(|_| {
            let t = std::time::Instant::now();
            let l = list(&all, &TaskQuery::default(), now, eod);
            assert_eq!(l.rows.len(), DEFAULT_LIMIT);
            t.elapsed()
        })
        .min()
        .unwrap();
    // Rozpočet 10 ms platí pro release; ladicí build na zatíženém stroji (brány, souběžní stavitelé) dostane 3× rezervu.
    let budget = std::time::Duration::from_millis(if cfg!(debug_assertions) { 30 } else { 10 });
    assert!(best < budget, "note_tasks nad 20 000 body: {best:?} (rozpočet {budget:?})");
    eprintln!("note_tasks (20 000 bodů): {best:?}");
}
