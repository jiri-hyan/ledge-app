//! Nástroje MCP pro poznámky (`notes_*`): výpis, čtení, vytvoření, připsání, odškrtnutí úkolu. Čisté
//! funkce nad kořenem poznámek — žádná vlastní logika úložiště: výběr dělá `notes::edit::query`, čtení
//! a zápis `notes::edit::{read, save, toggle_task}` (otisk obsahu = konflikt se syncem se hlásí), vytvoření
//! `notes::create`, cesty hlídá `notes::check_note_path`. Připnutí lístku a přenačtení po zápisu jsou na
//! `Host` (skutečný Ledge: okno lístku, `reload_notes`). Mapa: `docs/codemap/mcp.md`.

use crate::i18n::tr;
use crate::notes::{self, NewNote, edit, md};
use chrono::{DateTime, FixedOffset};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Kolik poznámek vrátí `notes_list`, když model nic neřekne / nejvýš.
pub const LIST_DEFAULT: usize = 30;
pub const LIST_MAX: usize = 100;
/// Delší tělo se v `notes_read` ořízne (odpověď jde do kontextu modelu).
pub const BODY_CAP: usize = 30_000;

fn arg<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

fn need<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    arg(a, k).ok_or_else(|| trf!("Chybí „{k}“.", "Missing “{k}”."))
}

pub fn off() -> String {
    tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings.").into()
}

/// Cesta z argumentu: absolutní jak ji vrací `notes_list`, nebo relativní ke kořeni; vždy ověřená.
pub fn note_path(root: &Path, a: &Value) -> Result<PathBuf, String> {
    let raw = need(a, "path")?;
    let p = Path::new(raw);
    let abs = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
    notes::check_note_path(root, &abs)?;
    Ok(abs)
}

fn changed() -> String {
    tr("Poznámka se mezitím změnila na disku (sync nebo úprava v Ledge) — načti ji znovu (notes_read) a zkus to znovu.", "The note changed on disk meanwhile (sync or an edit in Ledge) — read it again (notes_read) and retry.").into()
}

/// Kompaktní řádek poznámky pro výpis.
fn row(n: &notes::NoteView) -> Value {
    let mut o = json!({ "path": n.path, "title": n.title, "kind": n.kind });
    if let Some(p) = &n.project {
        o["project"] = json!(p);
    }
    if let Some(r) = &n.remind {
        o["remind"] = json!(r);
    }
    if n.done {
        o["done"] = json!(true);
    }
    if n.tasks_total > 0 {
        o["tasks"] = json!(format!("{}/{}", n.tasks_done, n.tasks_total));
    }
    o
}

/// `notes_list`: filtr `kind` / `project` (`_obecne` = bez projektu) / `includeDone`; připomínky na dnes
/// první (jako karta). Hotové jsou na konci a jen s `includeDone`.
pub fn list(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Value {
    let limit = a.get("limit").and_then(Value::as_u64).map_or(LIST_DEFAULT, |l| (l as usize).clamp(1, LIST_MAX));
    let q = edit::NoteQuery {
        kind: arg(a, "kind").map(str::to_string),
        project: arg(a, "project").map(str::to_string),
        include_done: a.get("includeDone").and_then(Value::as_bool).unwrap_or(false),
        archived: false,
        limit: Some(limit),
    };
    let r = edit::query(&notes::scan(root), &q, notes::end_of_day_ms(now));
    json!({ "total": r.total, "notes": r.notes.iter().map(row).collect::<Vec<_>>() })
}

/// Úkoly těla: řádek (od 0, jak ho chce `notes_toggle_task`), hotovo?, text.
fn task_rows(body: &str) -> Vec<Value> {
    let lines: Vec<&str> = body.split('\n').collect();
    md::tasks(body)
        .into_iter()
        .map(|(i, done)| {
            let l = lines.get(i).copied().unwrap_or_default().trim_end_matches('\r');
            let text = l.find('[').map_or(l, |at| l.get(at + 3..).unwrap_or_default()).trim();
            json!({ "line": i, "done": done, "text": text })
        })
        .collect()
}

/// `notes_read`: metadata + tělo (strop `BODY_CAP`) + úkoly s čísly řádků.
pub fn read(root: &Path, a: &Value) -> Result<Value, String> {
    let d = edit::read(root, &note_path(root, a)?)?;
    let v = &d.view;
    let truncated = d.body.chars().count() > BODY_CAP;
    let body: String = if truncated { d.body.chars().take(BODY_CAP).collect() } else { d.body.clone() };
    let mut o = json!({ "path": v.path, "title": v.title, "kind": v.kind, "created": v.created, "done": v.done, "body": body, "tasks": task_rows(&d.body) });
    for (k, val) in [("project", &v.project), ("remind", &v.remind), ("updated", &v.updated)] {
        if let Some(s) = val {
            o[k] = json!(s);
        }
    }
    if v.images > 0 {
        o["images"] = json!(v.images);
    }
    if truncated {
        o["truncated"] = json!(true);
    }
    Ok(o)
}

/// `notes_create`: nová poznámka (text, kategorie, projekt, připomínka ISO). `projects` / `kinds` = ids
/// z configu — jinak by vznikla poznámka s neznámým projektem / kategorií, kterou UI neumí zobrazit.
pub fn create(root: &Path, a: &Value, now: &DateTime<FixedOffset>, projects: &[String], kinds: &[String]) -> Result<(PathBuf, String), String> {
    let text = need(a, "text")?;
    let project = arg(a, "project").filter(|p| *p != notes::GENERAL);
    if let Some(p) = project.filter(|p| !projects.iter().any(|x| x == p)) {
        return Err(trf!("Projekt „{p}“ v Ledge není (jsou: {}).", "There's no project “{p}” in Ledge (there are: {}).", projects.join(", ")));
    }
    let kind = arg(a, "kind");
    if let Some(k) = kind.filter(|k| !kinds.iter().any(|x| x == k)) {
        return Err(trf!("Kategorie „{k}“ není (jsou: {}).", "There's no category “{k}” (there are: {}).", kinds.join(", ")));
    }
    let n = NewNote { text: text.into(), project: project.map(str::to_string), remind: arg(a, "remind").map(str::to_string), images: Vec::new(), kind: kind.map(str::to_string) };
    let path = notes::create(root, &n, now)?;
    let msg = trf!("Poznámka uložena: {}", "Note saved: {}", path.display());
    Ok((path, msg))
}

// ---------- připsání ----------

/// Co se připisuje: odstavec, odrážka, úkol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    Text,
    Bullet,
    Task,
}

impl Style {
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s {
            None | Some("text") => Ok(Self::Text),
            Some("bullet") => Ok(Self::Bullet),
            Some("task") => Ok(Self::Task),
            Some(o) => Err(trf!("Neznámý styl „{o}“ (text, bullet, task).", "Unknown style “{o}” (text, bullet, task).")),
        }
    }
}

/// Značka poslední položky seznamu.
#[derive(Debug, PartialEq)]
enum Marker {
    Bullet(char),
    Num(u64, char),
}

/// Odsazení a značka řádku, když je to položka seznamu (`- `, `* `, `+ `, `3. `, `3) `).
fn list_tail(line: &str) -> Option<(&str, Marker)> {
    let t = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - t.len()];
    let mut ch = t.chars();
    let first = ch.next()?;
    if matches!(first, '-' | '*' | '+') && t[1..].starts_with(' ') {
        return Some((indent, Marker::Bullet(first)));
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=9).contains(&digits) {
        let delim = t[digits..].chars().next().filter(|d| matches!(d, '.' | ')'))?;
        if t[digits + 1..].starts_with(' ') {
            return Some((indent, Marker::Num(t[..digits].parse().ok()?, delim)));
        }
    }
    None
}

/// Otevřený blok kódu na konci těla (lichý počet plotů) → jeho značka.
fn open_fence(body: &str) -> Option<&'static str> {
    let mut open: Option<&'static str> = None;
    for l in body.lines() {
        let t = l.trim_start();
        let f = if t.starts_with("```") { "```" } else if t.starts_with("~~~") { "~~~" } else { continue };
        open = if open.is_some() { None } else { Some(f) };
    }
    open
}

/// Model někdy pošle položku i se značkou (`- [ ] koupit mléko`) — značka se zahodí, doplní se naše.
fn bare(line: &str) -> &str {
    let l = line.trim();
    let l = list_tail(l).map_or(l, |(_, _)| l.split_once(' ').map_or(l, |x| x.1).trim_start());
    l.strip_prefix("[ ]").or_else(|| l.strip_prefix("[x]")).map_or(l, str::trim_start)
}

/// Připíše na konec těla. Odstavec za prázdný řádek; položky navážou na poslední seznam (stejná
/// odrážka a odsazení, u číslovaného pokračuje číslování `3.` → `4.`), jinak začnou nový seznam.
pub fn append(body: &str, text: &str, style: Style) -> String {
    let base = body.trim_end();
    let text = text.trim_matches(['\n', '\r']).trim_end();
    let fence = open_fence(base);
    let mut out = base.to_string();
    if let Some(f) = fence {
        out.push('\n');
        out.push_str(f);
    }
    let tail = if fence.is_some() { None } else { base.lines().last().and_then(list_tail) };
    if style == Style::Text {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(text);
        return out;
    }
    out.push_str(if tail.is_some() { "\n" } else if out.is_empty() { "" } else { "\n\n" });
    let items: Vec<&str> = text.lines().map(bare).filter(|l| !l.is_empty()).collect();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match &tail {
            Some((ind, Marker::Num(n, d))) => out.push_str(&format!("{ind}{}{d} ", n + 1 + i as u64)),
            Some((ind, Marker::Bullet(c))) => out.push_str(&format!("{ind}{c} ")),
            None => out.push_str("- "),
        }
        if style == Style::Task {
            out.push_str("[ ] ");
        }
        out.push_str(item);
    }
    out
}

/// `notes_append`: čtení (otisk) → `append` → `edit::save` se stejným otiskem; změna na disku = chyba.
pub fn append_to(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let text = need(a, "text")?;
    let style = Style::parse(arg(a, "style"))?;
    let path = note_path(root, a)?;
    let d = edit::read(root, &path)?;
    let v = &d.view;
    let req = edit::SaveNote {
        path: path.to_string_lossy().into_owned(),
        expected_hash: d.hash.clone(),
        body: append(&d.body, text, style),
        project: v.project.clone(),
        kind: Some(v.kind.clone()),
        remind: v.remind.clone(),
        images: Vec::new(),
    };
    match edit::save(root, &req, now)? {
        edit::SaveOutcome::Saved { note } => Ok(trf!("Připsáno ({} řádků v poznámce).", "Appended ({} lines in the note).", note.body.lines().count())),
        edit::SaveOutcome::Changed => Err(changed()),
    }
}

/// `notes_toggle_task`: řádek úkolu (z `notes_read`), `done` (bez něj se přepne opačně).
pub fn toggle(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let line = a.get("line").and_then(Value::as_u64).ok_or_else(|| tr("Chybí „line“ (číslo řádku úkolu z notes_read).", "Missing “line” (the task's line number from notes_read).").to_string())? as usize;
    let path = note_path(root, a)?;
    let d = edit::read(root, &path)?;
    let now_done = md::tasks(&d.body).into_iter().find(|t| t.0 == line).map(|t| t.1).ok_or_else(|| trf!("Na řádku {line} není úkol (notes_read je vypíše).", "There's no task on line {line} (notes_read lists them)."))?;
    let want = a.get("done").and_then(Value::as_bool).unwrap_or(!now_done);
    if want == now_done {
        return Ok(tr("Beze změny — už to tak je.", "No change — it's already like that.").into());
    }
    match edit::toggle_task(root, &path, line, want, &d.hash, now)? {
        edit::SaveOutcome::Saved { note } => Ok(trf!("Úkol {} ({}/{} hotovo).", "Task {} ({}/{} done).", if want { tr("hotový", "done") } else { tr("znovu otevřený", "reopened") }, note.view.tasks_done, note.view.tasks_total)),
        edit::SaveOutcome::Changed => Err(changed()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_table() {
        let b = Style::Bullet;
        let t = Style::Task;
        let x = Style::Text;
        let cases: &[(&str, &str, Style, &str)] = &[
            ("", "ahoj", x, "ahoj"),
            ("první", "druhý", x, "první\n\ndruhý"),
            ("první\n\n\n", "druhý\n", x, "první\n\ndruhý"),
            ("", "mléko", b, "- mléko"),
            ("Nákup", "mléko", b, "Nákup\n\n- mléko"),
            ("- chleba", "mléko", b, "- chleba\n- mléko"),
            ("* chleba\n", "mléko", b, "* chleba\n* mléko"),
            ("  - vnořené", "další", b, "  - vnořené\n  - další"),
            ("1. a\n2. b", "c", b, "1. a\n2. b\n3. c"),
            ("1) a\n2) b", "c", b, "1) a\n2) b\n3) c"),
            ("1. a\n2. b", "c\nd", b, "1. a\n2. b\n3. c\n4. d"),
            ("9. a", "b", b, "9. a\n10. b"),
            ("  2. vnořené", "x", b, "  2. vnořené\n  3. x"),
            ("Úkoly", "zavolat", t, "Úkoly\n\n- [ ] zavolat"),
            ("- [x] hotovo", "nové", t, "- [x] hotovo\n- [ ] nové"),
            ("1. a", "b", t, "1. a\n2. [ ] b"),
            ("", "- [ ] už s ním", t, "- [ ] už s ním"),
            ("- a", "- b\n\n- c", b, "- a\n- b\n- c"),
            ("```\nkód\n- x", "další", b, "```\nkód\n- x\n```\n\n- další"),
            ("```\nkód\n```", "další", b, "```\nkód\n```\n\n- další"),
            ("text\n- a\n\npoznámka", "b", b, "text\n- a\n\npoznámka\n\n- b"),
        ];
        for (body, add, st, want) in cases {
            assert_eq!(append(body, add, *st), *want, "append({body:?}, {add:?}, {st:?})");
        }
    }

    #[test]
    fn list_tail_table() {
        assert_eq!(list_tail("- a"), Some(("", Marker::Bullet('-'))));
        assert_eq!(list_tail("   + a"), Some(("   ", Marker::Bullet('+'))));
        assert_eq!(list_tail("12. a"), Some(("", Marker::Num(12, '.'))));
        assert_eq!(list_tail("3) a"), Some(("", Marker::Num(3, ')'))));
        for no in ["", "-a", "2024-10-01", "3.14 je pí", "1234567890. moc", "**tučně**", "---", "text"] {
            assert_eq!(list_tail(no), None, "{no:?}");
        }
    }

    #[test]
    fn task_rows_have_line_and_text() {
        let body = "# Nákup\n\n- [ ] mléko\n- [x] chleba\n```\n- [ ] kód\n```\n2. [ ] tři";
        let r = task_rows(body);
        assert_eq!(r, vec![json!({"line":2,"done":false,"text":"mléko"}), json!({"line":3,"done":true,"text":"chleba"}), json!({"line":7,"done":false,"text":"tři"})]);
    }
}
