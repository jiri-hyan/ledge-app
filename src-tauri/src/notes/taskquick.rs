//! Rychlý úkol z jedné věty (F3): „v 9:00 doktor !“, „zítra doktor !!“, „doktor @Tomáš“, „řešitel Tomáš Hora
//! zavolat“, „tomorrow at 9:00 doctor !“ → `QuickTask {title, whenIso, priority, assignee}` a hotový řádek
//! `- [ ] …` (zápis přes `taskline::render`, jediný formát). Čas čte `when::parse_when` (beze změny, česky;
//! pár anglických slov se předtím přeloží), priorita a řešitel `taskline::parse_line` (stejná pravidla
//! jako `connector::plan`). Čisté funkce bez I/O; UI volá příkaz `note_quick_parse`, zápis dělá volající.

use super::taskline::{self, TaskLine, parse_line, prio_word};
use super::when::parse_when;
use super::{iso, md::TaskState};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickTask {
    /// text bez času, priority a řešitele (bez nich prázdný = původní věta)
    pub title: String,
    /// termín (ISO s offsetem, místní čas), `None` = věta čas neobsahuje
    pub when_iso: Option<String>,
    /// 0, 1 (`!`), 2 (`!!`)
    pub priority: u8,
    /// řešitel, jak je napsaný („Tomáš“, „Tomáš Hora“); neověřuje se proti týmu
    pub assignee: Option<String>,
    /// hotový řádek `- [ ] title [řešitel] ⏰ YYYY-MM-DD HH:MM [!|!!]`
    pub line: String,
}

/// Anglická slova času → česky, ať je `parse_when` pochopí (délku textu nezachovává, proto se z přeloženého
/// textu skládá i zbytek věty). `at` jen před číslem („at 9:00“), ostatní celá slova.
fn en_to_cz(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    let mut out = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        let core = w.trim_end_matches([',', '.']);
        let tail = &w[core.len()..];
        let next_digit = words.get(i + 1).is_some_and(|n| n.starts_with(|c: char| c.is_ascii_digit()));
        let rep = match core.to_lowercase().as_str() {
            "tomorrow" => Some("zítra"),
            "today" => Some("dnes"),
            "at" if next_digit => Some("v"),
            _ => None,
        };
        out.push(rep.map_or_else(|| w.to_string(), |r| format!("{r}{tail}")));
    }
    out.join(" ")
}

/// Rozebere větu. `now` = místní čas.
pub fn parse_quick(text: &str, now: &DateTime<FixedOffset>) -> QuickTask {
    let text = text.trim();
    let tr = en_to_cz(text);
    let when = parse_when(&tr, now);
    let rest = match &when {
        Some(w) => format!("{} {}", &tr[..w.start], &tr[w.end..]),
        None => tr.clone(),
    };
    let parts = parse_line(&rest);
    let mut gone = vec![false; parts.words.len()];
    let mut priority = 0u8;
    for &i in &parts.prio_words {
        gone[i] = true;
        priority = priority.max(match prio_word(&parts.words[i]) {
            Some(5) => 2,
            Some(4) => 1,
            _ => 0,
        });
    }
    let mut assignee = None;
    if let Some(m) = &parts.mention {
        let used = if m.at_sign { 1 } else { m.cand.len().min(if m.cand.len() > 1 && m.cand[1].chars().next().is_some_and(char::is_uppercase) { 2 } else { 1 }) };
        assignee = Some(m.cand[..used].join(" "));
        let end = if m.at_sign { m.at } else { m.at + m.skip + used };
        gone[m.at..=end].iter_mut().for_each(|g| *g = true);
    }
    let kept: Vec<&str> = parts.words.iter().zip(&gone).filter(|(_, g)| !**g).map(|(w, _)| w.as_str()).collect();
    let mut title = kept.join(" ").trim_matches(|c: char| matches!(c, ',' | ';' | ':' | '-' | '–') || c.is_whitespace()).to_string();
    if title.is_empty() {
        title = text.to_string();
    }
    let due = when.as_ref().map(|w| w.at.naive_local());
    let line_text = match &assignee {
        Some(a) if a.contains(' ') => format!("{title} řešitel {a}"),
        Some(a) => format!("{title} @{a}"),
        None => title.clone(),
    };
    let tl = TaskLine { text: &line_text, head: &line_text, due, all_day: false, cal: false, prio: priority, evidence_raw: "", trail: "" };
    let line = format!("- [{}] {}", TaskState::Open.mark(), taskline::render(&tl));
    QuickTask { title, when_iso: when.map(|w| iso(&w.at)), priority, assignee, line }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::parse_time;

    fn now() -> DateTime<FixedOffset> {
        parse_time("2026-11-14T08:00:00+01:00").unwrap()
    }

    #[test]
    fn quick_table() {
        // (věta, název, čas místní ISO bez offsetu, priorita, řešitel)
        let cases = [
            ("v 9:00 doktor !", "doktor", Some("2026-11-14T09:00"), 1, None),
            ("zítra doktor !!", "doktor", Some("2026-11-15T09:00"), 2, None),
            ("doktor @Tomáš", "doktor", None, 0, Some("Tomáš")),
            ("řešitel Tomáš Hora zavolat", "zavolat", None, 0, Some("Tomáš Hora")),
            ("zavolat Honzovi, řešitel Tomáš", "zavolat Honzovi", None, 0, Some("Tomáš")),
            ("tomorrow at 10:30 call the doctor !", "call the doctor", Some("2026-11-15T10:30"), 1, None),
            ("assignee Petr deploy !!", "deploy", None, 2, Some("Petr")),
            ("Pozor! koupit mléko", "Pozor! koupit mléko", None, 0, None),
            ("doktor @src/App.tsx:4", "doktor @src/App.tsx:4", None, 0, None),
            ("zítra", "zítra", Some("2026-11-15T09:00"), 0, None),
            ("", "", None, 0, None),
        ];
        for (text, title, when, prio, who) in cases {
            let q = parse_quick(text, &now());
            assert_eq!(q.title, title, "{text}");
            assert_eq!(q.when_iso.as_deref().map(|s| &s[..16]), when, "{text}");
            assert_eq!(q.priority, prio, "{text}");
            assert_eq!(q.assignee.as_deref(), who, "{text}");
        }
    }

    #[test]
    fn line_is_one_taskline_format() {
        let q = parse_quick("v 9:00 doktor !", &now());
        assert_eq!(q.line, "- [ ] doktor ⏰ 2026-11-14 09:00 !");
        let q = parse_quick("zítra doktor @Tomáš !!", &now());
        assert_eq!(q.line, "- [ ] doktor @Tomáš ⏰ 2026-11-15 09:00 !!");
        assert_eq!(parse_quick("řešitel Tomáš Hora zavolat", &now()).line, "- [ ] zavolat řešitel Tomáš Hora");
        // zapsaný řádek se přečte zpět tím samým parserem
        let back = parse_quick("v 9:00 doktor !", &now());
        let tl = taskline::parse_tail(back.line.strip_prefix("- [ ] ").unwrap());
        assert_eq!((tl.text, tl.prio, tl.due.map(|d| d.to_string())), ("doktor", 1, Some("2026-11-14 09:00:00".into())));
    }
}
