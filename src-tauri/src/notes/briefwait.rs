//! Zadání čekající na člověka (V1 „Pruh ví“, k34): `status: waiting` (Claude se ptá v `## Otázky`) a `status: review`
//! (body `[~]` ke kontrole). Čistá funkce nad poznámkami v paměti (jako `taskindex::due_counts`), do snapshotu jde
//! `briefWaiting`; chip pruhu návrhů (`src/suggest/model.ts`) z něj nic nehádá. `done` se nepočítá — nikdo na nic nečeká.

use super::NoteView;
use serde::Serialize;

/// První čekající zadání: nejdřív to, co čeká na odpověď (`waiting`), pak ke kontrole (`review`); uvnitř stavu pořadí poznámek.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct First {
    pub path: String,
    pub title: String,
    /// `waiting` | `review`
    pub status: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefWaiting {
    /// čeká na odpověď na otázku
    pub waiting: usize,
    /// čeká na kontrolu bodů
    pub review: usize,
    pub first: Option<First>,
}

/// Počty a první čekající zadání přes všechny nehotové poznámky.
pub fn brief_waiting(all: &[NoteView]) -> BriefWaiting {
    let mut b = BriefWaiting::default();
    let (mut w, mut r): (Option<&NoteView>, Option<&NoteView>) = (None, None);
    for n in all.iter().filter(|n| !n.done) {
        match n.status.as_deref() {
            Some("waiting") => {
                b.waiting += 1;
                w = w.or(Some(n));
            }
            Some("review") => {
                b.review += 1;
                r = r.or(Some(n));
            }
            _ => {}
        }
    }
    b.first = w.map(|n| (n, "waiting")).or(r.map(|n| (n, "review"))).map(|(n, status)| First { path: n.path.clone(), title: n.title.clone(), status });
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn note(name: &str, status: &str, done: bool) -> NoteView {
        let mut n = super::super::parse_note(Path::new(&format!("C:\\n\\{name}.md")), &format!("---\ncreated: 2026-11-01T10:00:00+01:00\nstatus: {status}\ntitle: {name}\n---\ntext"), None, 0);
        n.done = done;
        n
    }

    #[test]
    fn counts_and_first_table() {
        let none = brief_waiting(&[]);
        assert_eq!((none.waiting, none.review, none.first), (0, 0, None));
        let all = vec![note("run", "running", false), note("rev1", "review", false), note("ask1", "waiting", false), note("ask2", "waiting", false), note("rev2", "review", false), note("fin", "done", false), note("arch", "waiting", true), note("dr", "draft", false)];
        let b = brief_waiting(&all);
        assert_eq!((b.waiting, b.review), (2, 2));
        let f = b.first.unwrap();
        assert_eq!((f.title.as_str(), f.status), ("ask1", "waiting"), "otázka před kontrolou, první v pořadí");
        let only_review = brief_waiting(&all[..2]);
        assert_eq!(only_review.first.map(|f| (f.title, f.status)), Some(("rev1".into(), "review")));
        assert_eq!(brief_waiting(&[note("a", "done", false), note("b", "ready", false)]), BriefWaiting::default());
    }
}
