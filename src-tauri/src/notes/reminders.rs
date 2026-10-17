//! Připomínky a karta Poznámky: co je po termínu (`due_split`), obsah karty se stropem (`card`,
//! `CARD_ROWS`) a stav „peek už vyjel“ v `<root>\.state.json` (`reminder_key`, `load_peeked`,
//! `save_peeked`). Závisí na `NoteView` (`store.rs`) a `write_atomic`.

use super::{NoteView, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Připomínky po termínu (nehotové, nejstarší termín první) a nejbližší budoucí termín.
pub fn due_split(notes: &[NoteView], now_ms: i64) -> (Vec<NoteView>, Option<i64>) {
    let mut due: Vec<NoteView> = notes.iter().filter(|n| !n.done && n.remind_ms.is_some_and(|r| r <= now_ms)).cloned().collect();
    due.sort_by_key(|n| n.remind_ms);
    let next = notes.iter().filter(|n| !n.done).filter_map(|n| n.remind_ms).filter(|&r| r > now_ms).min();
    (due, next)
}

/// Kolik řádků má karta Poznámky nejvýš — snapshot nesmí růst s počtem poznámek.
pub const CARD_ROWS: usize = 8;

/// Obsah karty: nehotové připomínky do konce dneška (i po termínu) podle času, pak nejnovější
/// ostatní — dohromady nejvýš `max` řádků. Druhá hodnota = kolik poznámek se nevešlo (zbytek
/// přes `note_list`).
pub fn card(notes: &[NoteView], today_end_ms: i64, max: usize) -> (Vec<NoteView>, usize) {
    let is_today = |n: &NoteView| !n.done && n.remind_ms.is_some_and(|r| r < today_end_ms);
    let mut today: Vec<&NoteView> = notes.iter().filter(|n| is_today(n)).collect();
    today.sort_by_key(|n| n.remind_ms);
    let out: Vec<NoteView> = today.into_iter().chain(notes.iter().filter(|n| !is_today(n))).take(max).cloned().collect();
    let more = notes.len() - out.len();
    (out, more)
}

/// Klíč připomínky pro „peek už vyjel“: cesta + termín (přeplánovaná připomínka vyjede znovu).
pub fn reminder_key(n: &NoteView) -> String {
    format!("{}|{}", n.path, n.remind_ms.unwrap_or(0))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NotesStateFile {
    #[serde(default)]
    peeked: Vec<String>,
}

/// Připomínky, pro které už peek vyjel (`<root>\.state.json`) — po restartu Ledge peek znovu
/// nevyjede, zůstane jen jantarový proužek.
pub fn load_peeked(root: &Path) -> std::collections::HashSet<String> {
    std::fs::read(root.join(".state.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<NotesStateFile>(&b).ok())
        .map(|s| s.peeked.into_iter().collect())
        .unwrap_or_default()
}

/// Uloží jen klíče, které jsou pořád po termínu (hotové / smazané vypadnou → soubor neroste).
pub fn save_peeked(root: &Path, peeked: &std::collections::HashSet<String>, due: &[NoteView]) -> Result<(), String> {
    let mut keep: Vec<String> = due.iter().map(reminder_key).filter(|k| peeked.contains(k)).collect();
    keep.sort();
    let text = serde_json::to_string_pretty(&NotesStateFile { peeked: keep }).map_err(|e| e.to_string())?;
    write_atomic(&root.join(".state.json"), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::super::DEFAULT_KIND;
    use super::super::testkit::tmp_root;
    use super::*;

    fn nv(title: &str, created: i64, remind: Option<i64>, done: bool) -> NoteView {
        NoteView {
            path: format!("C:\\n\\{title}.md"),
            project: None,
            created: String::new(),
            created_ms: created,
            remind: None,
            remind_ms: remind,
            done,
            title: title.into(),
            images: 0,
            kind: DEFAULT_KIND.into(),
            updated: None,
            tasks_done: 0,
            tasks_total: 0,
        }
    }

    #[test]
    fn due_split_table() {
        let notes = [
            nv("po-termínu", 1, Some(100), false),
            nv("hotová", 2, Some(50), true),
            nv("přesně-teď", 3, Some(200), false),
            nv("budoucí", 4, Some(500), false),
            nv("bližší-budoucí", 5, Some(300), false),
            nv("hotová-budoucí", 6, Some(250), true),
            nv("bez", 7, None, false),
        ];
        let (due, next) = due_split(&notes, 200);
        assert_eq!(due.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), vec!["po-termínu", "přesně-teď"]);
        assert_eq!(next, Some(300));
        let (due, next) = due_split(&notes, 10_000);
        assert_eq!(due.len(), 4);
        assert_eq!(next, None);
        assert_eq!(due_split(&[], 0), (vec![], None));
    }

    #[test]
    fn card_puts_today_reminders_first() {
        // seřazené nejnovější první, jak je vrací scan
        let notes = [
            nv("nová", 9, None, false),
            nv("zítra", 8, Some(2_000), false),
            nv("dnes-večer", 7, Some(900), false),
            nv("hotová-dnes", 6, Some(100), true),
            nv("po-termínu", 5, Some(100), false),
            nv("stará", 1, None, false),
        ];
        let (c, more) = card(&notes, 1_000, 8);
        assert_eq!(c.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), vec!["po-termínu", "dnes-večer", "nová", "zítra", "hotová-dnes", "stará"]);
        assert_eq!(more, 0);
        // strop: připomínky mají přednost, zbytek se jen spočítá
        let (c, more) = card(&notes, 1_000, 3);
        assert_eq!(c.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), vec!["po-termínu", "dnes-večer", "nová"]);
        assert_eq!(more, 3);
        let many: Vec<NoteView> = (0..500).map(|i| nv("x", 1_000 - i, Some(10), false)).collect();
        let (c, more) = card(&many, 1_000, CARD_ROWS);
        assert_eq!((c.len(), more), (CARD_ROWS, 500 - CARD_ROWS));
    }

    #[test]
    fn peeked_state_persists_only_due_keys() {
        let root = tmp_root();
        assert!(load_peeked(root.path()).is_empty());
        let due = vec![nv("a", 1, Some(10), false), nv("b", 2, Some(20), false)];
        let mut peeked: std::collections::HashSet<String> = due.iter().map(reminder_key).collect();
        peeked.insert("C:\\n\\hotova.md|5".into());
        save_peeked(root.path(), &peeked, &due).unwrap();
        let back = load_peeked(root.path());
        assert_eq!(back, due.iter().map(reminder_key).collect());
        // přeplánovaná připomínka má jiný klíč → vyjede znovu
        let moved = nv("a", 1, Some(99), false);
        assert!(!back.contains(&reminder_key(&moved)));
    }
}
