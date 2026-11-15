//! Index otevřených bodů napříč poznámkami (čisté funkce, bez I/O): `index` ho staví při `parse_note`
//! (`NoteView::items`, `#[serde(skip)]` — do snapshotu nejde), `list` skládá `note_tasks` z paměti, `due_counts`
//! dává dvě čísla pro snapshot (`notesTaskCounts`). Gramatika řádku je v `taskline.rs`.
//!
//! Definice čísla na záložce: otevřené body s termínem do konce dneška (po termínu + dnes), bez bodů už
//! odeslaných do propojené aplikace a bez navržených `[~]`. Celodenní termín (`⏰ YYYY-MM-DD`) je „dnes“ celý den
//! a po termínu až od půlnoci; `due_ms` celodenního je 9:00 (jako `parse_time` u samotného data), čas, kdy
//! se ozve připomínka.

use super::md::{TaskState, task_lines};
use super::taskline::{SENT_MARK, parse_tail};
use super::{NoteView, local_from_naive};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Nejvýš bodů v indexu jedné poznámky a znaků textu bodu.
pub const MAX_ITEMS: usize = 50;
pub const MAX_TEXT: usize = 160;
/// Kolik řádků vrátí `note_tasks` bez udání `limit`, a strop.
pub const DEFAULT_LIMIT: usize = 200;
pub const MAX_LIMIT: usize = 1000;

#[derive(Debug, Clone, PartialEq)]
pub struct TaskItem {
    /// řádek těla od 0 (jako `md::tasks`)
    pub line: u32,
    /// `Open` nebo `Proposed` (hotové se neindexují)
    pub state: TaskState,
    /// text bez termínu, priority a důkazu, ≤ `MAX_TEXT` znaků
    pub text: String,
    pub prio: u8,
    pub all_day: bool,
    /// termín (ms); celodenní = 9:00 toho dne
    pub due_ms: Option<i64>,
    /// od kdy je po termínu (ms): timed = `due_ms`, celodenní = půlnoc po dni
    pub late_ms: Option<i64>,
    /// pod bodem je odkaz „↳ [název]…“ (předáno, nepočítá se)
    pub forwarded: bool,
}

/// Výsledek `index`: body, nejbližší termín otevřeného bodu a počty úkolů (`tasks_done` / `tasks_total`).
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub items: Arc<[TaskItem]>,
    pub next_due_ms: Option<i64>,
    pub done: usize,
    pub total: usize,
}

fn clip(s: &str) -> String {
    if s.chars().nth(MAX_TEXT).is_none() {
        return s.to_string();
    }
    let mut o: String = s.chars().take(MAX_TEXT - 1).collect();
    o.push('…');
    o
}

/// Odkaz do propojené aplikace: první řádek za bodem, který není snímek `![…](…)`, začíná `SENT_MARK` (prázdný řádek řetěz přeruší;
/// TS zrcadlo `forwardedAt` v `src/notes/taskline.ts`).
fn forwarded(lines: &[&str], at: usize) -> bool {
    let mut it = lines.iter().skip(at + 1).map(|l| l.trim());
    it.find(|l| !(l.starts_with("![") && l.ends_with(')'))).is_some_and(|l| l.starts_with(SENT_MARK))
}

/// Body těla poznámky (bez frontmatteru, jako `edit::body_view(...).1`).
pub fn index(body: &str) -> Index {
    let (mut done, mut total) = (0, 0);
    let mut items: Vec<TaskItem> = Vec::new();
    let mut next: Option<i64> = None;
    let mut lines: Option<Vec<&str>> = None;
    for (i, state, after) in task_lines(body) {
        total += 1;
        if state == TaskState::Done {
            done += 1;
            continue;
        }
        let tl = parse_tail(after);
        let (due_ms, late_ms) = tl.due.map_or((None, None), |d| {
            let at = if tl.all_day { d.date().and_hms_opt(9, 0, 0).unwrap_or(d) } else { d };
            let late = if tl.all_day { d.date().succ_opt().map(|n| n.and_time(NaiveTime::default())).unwrap_or(d) } else { d };
            (local_from_naive(at).map(|x| x.timestamp_millis()), local_from_naive(late).map(|x| x.timestamp_millis()))
        });
        if tl.due.is_none() && items.len() >= MAX_ITEMS {
            continue;
        }
        let sent = forwarded(lines.get_or_insert_with(|| body.split('\n').map(|l| l.trim_end_matches('\r')).collect()), i);
        if state == TaskState::Open && !sent {
            next = next.into_iter().chain(due_ms).min();
        }
        if items.len() < MAX_ITEMS {
            items.push(TaskItem { line: i as u32, state, text: clip(tl.text), prio: tl.prio, all_day: tl.all_day, due_ms, late_ms, forwarded: sent });
        }
    }
    Index { items: items.into(), next_due_ms: next, done, total }
}

// ---------- dotaz z paměti ----------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskQuery {
    /// id projektu, `_obecne` = bez projektu, `None` = všechny
    pub project: Option<String>,
    /// nejvýš řádků (s termínem, pak navržené, pak bez termínu); výchozí `DEFAULT_LIMIT`
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    pub path: String,
    /// řádek těla od 0 (pro `note_toggle_task` / `note_task_snooze`)
    pub line: u32,
    pub text: String,
    pub state: TaskState,
    pub due_ms: Option<i64>,
    pub all_day: bool,
    /// 0, 1 (`!`), 2 (`!!`)
    pub prio: u8,
    pub project: Option<String>,
    pub note_title: String,
    pub kind: String,
    /// předáno do propojené aplikace — ukázat šedě, nepočítá se
    pub forwarded: bool,
}

/// Počty přes všechny poznámky filtru (ne jen vrácené řádky).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCounts {
    pub overdue: usize,
    pub today: usize,
    pub proposed: usize,
    pub undated: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskList {
    pub rows: Vec<TaskRow>,
    pub counts: TaskCounts,
}

/// Dvě čísla do snapshotu: otevřené body po termínu a s termínem dnes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueCounts {
    pub overdue: usize,
    pub today: usize,
}

/// Řádek s termínem a pořadím pro stabilní řazení.
type Keyed<'a> = ((Option<i64>, usize), (&'a NoteView, &'a TaskItem));

#[derive(PartialEq)]
enum Bucket {
    Overdue,
    Today,
    Later,
    Undated,
    Proposed,
    Forwarded,
}

fn bucket(t: &TaskItem, now: i64, eod: i64) -> Bucket {
    match (t.state, t.forwarded, t.due_ms, t.late_ms) {
        (TaskState::Proposed, ..) => Bucket::Proposed,
        (_, true, ..) => Bucket::Forwarded,
        (_, _, _, Some(l)) if l <= now => Bucket::Overdue,
        (_, _, Some(d), _) if d < eod => Bucket::Today,
        (_, _, Some(_), _) => Bucket::Later,
        _ => Bucket::Undated,
    }
}

fn in_project(n: &NoteView, q: &TaskQuery) -> bool {
    match q.project.as_deref() {
        None => true,
        Some("_obecne") => n.project.is_none(),
        Some(p) => n.project.as_deref() == Some(p),
    }
}

/// Dvě čísla pro snapshot (`notesTaskCounts`) přes všechny aktivní poznámky.
pub fn due_counts(all: &[NoteView], now: i64, eod: i64) -> DueCounts {
    let mut c = DueCounts::default();
    for n in all.iter().filter(|n| !n.done) {
        for t in n.items.iter() {
            match bucket(t, now, eod) {
                Bucket::Overdue => c.overdue += 1,
                Bucket::Today => c.today += 1,
                _ => {}
            }
        }
    }
    c
}

/// Otevřené body napříč poznámkami: nejdřív s termínem (podle času), pak navržené `[~]`, pak bez termínu
/// (nejnovější poznámka první). Hotové poznámky se přeskočí. Klonují se jen vrácené řádky.
pub fn list(all: &[NoteView], q: &TaskQuery, now: i64, eod: i64) -> TaskList {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let mut counts = TaskCounts::default();
    let (mut dated, mut proposed, mut undated): (Vec<(&NoteView, &TaskItem)>, Vec<_>, Vec<_>) = (Vec::new(), Vec::new(), Vec::new());
    let mut seq = 0usize;
    let mut order: Vec<(Option<i64>, usize)> = Vec::new();
    for n in all.iter().filter(|n| !n.done && in_project(n, q)) {
        for t in n.items.iter() {
            let b = bucket(t, now, eod);
            match b {
                Bucket::Overdue => counts.overdue += 1,
                Bucket::Today => counts.today += 1,
                Bucket::Proposed => counts.proposed += 1,
                Bucket::Undated => counts.undated += 1,
                _ => {}
            }
            match b {
                Bucket::Proposed => proposed.push((n, t)),
                _ if t.due_ms.is_some() => {
                    dated.push((n, t));
                    order.push((t.due_ms, seq));
                    seq += 1;
                }
                _ => undated.push((n, t)),
            }
        }
    }
    // jen `limit` nejbližších: výběr O(n) místo třídění všeho; shodný čas drží pořadí poznámek
    let mut keyed: Vec<Keyed> = order.into_iter().zip(dated).collect();
    if keyed.len() > limit && limit > 0 {
        keyed.select_nth_unstable_by_key(limit - 1, |(k, _)| *k);
        keyed.truncate(limit);
    }
    keyed.sort_by_key(|(k, _)| *k);
    let dated = keyed.into_iter().map(|(_, r)| r);
    let row = |(n, t): (&NoteView, &TaskItem)| TaskRow {
        path: n.path.clone(),
        line: t.line,
        text: t.text.clone(),
        state: t.state,
        due_ms: t.due_ms,
        all_day: t.all_day,
        prio: t.prio,
        project: n.project.clone(),
        note_title: n.title.clone(),
        kind: n.kind.clone(),
        forwarded: t.forwarded,
    };
    let rows = dated.chain(proposed).chain(undated).take(limit).map(row).collect();
    TaskList { rows, counts }
}

#[cfg(test)]
#[path = "taskindex_tests.rs"]
mod tests;
