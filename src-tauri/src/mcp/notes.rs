//! Nástroje MCP pro poznámky (`notes_*`): výpis, čtení, vytvoření, připsání, odškrtnutí úkolu. Čisté
//! funkce nad kořenem poznámek — žádná vlastní logika úložiště: výběr dělá `notes::edit::query`, čtení
//! a zápis `notes::edit::{read, save, toggle_task}` (otisk obsahu = konflikt se syncem se hlásí), vytvoření
//! `notes::create`, cesty hlídá `notes::check_note_path`. Připnutí lístku a přenačtení po zápisu jsou na
//! `Host` (skutečný Ledge: okno lístku, `reload_notes`). Mapa: `docs/codemap/mcp.md`.

use crate::i18n::tr;
use crate::notes::{self, NewNote, brief, edit, md};
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
    for (k, v) in [("status", &n.status), ("model", &n.model)] {
        if let Some(s) = v {
            o[k] = json!(s);
        }
    }
    if let Some(c) = n.cost {
        o["cost"] = json!(c);
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
        only_done: false,
        limit: Some(limit),
        tag: None,
    };
    let r = edit::query(&notes::scan(root), &q, notes::end_of_day_ms(now));
    json!({ "total": r.total, "notes": r.notes.iter().map(row).collect::<Vec<_>>() })
}

/// Bod zadání pro model: řádek (od 0, jak ho chce `notes_toggle_task`), stav, text, důkaz, snímky a odkazy.
fn item_row(it: &brief::Item) -> Value {
    let mut o = json!({ "line": it.line, "done": it.state == md::TaskState::Done, "state": it.state, "text": it.text });
    if let Some(e) = &it.evidence {
        o["evidence"] = json!(e);
    }
    if !it.images.is_empty() {
        o["images"] = json!(it.images.iter().map(|i| json!({ "path": i.path, "caption": i.caption })).collect::<Vec<_>>());
    }
    if !it.refs.is_empty() {
        o["refs"] = json!(it.refs.iter().map(|r| json!({ "raw": r.raw, "path": r.path, "line": r.line, "exists": r.exists })).collect::<Vec<_>>());
    }
    o
}

/// `notes_read`: metadata + tělo (strop `BODY_CAP`) + body s čísly řádků, stavem, důkazem, snímky a
/// odkazy, otázky a nenavázané snímky. `dir_of` = složka projektu podle jeho id (pro odkazy `@cesta`).
pub fn read(root: &Path, a: &Value, dir_of: &dyn Fn(&str) -> Option<PathBuf>) -> Result<Value, String> {
    let d = edit::read(root, &note_path(root, a)?)?;
    let dir = d.view.project.as_deref().and_then(dir_of);
    let d = d.with_refs(dir.as_deref());
    let v = &d.view;
    let truncated = d.body.chars().count() > BODY_CAP;
    let body: String = if truncated { d.body.chars().take(BODY_CAP).collect() } else { d.body.clone() };
    let mut o = json!({ "path": v.path, "title": v.title, "kind": v.kind, "created": v.created, "done": v.done, "body": body, "tasks": d.items.iter().map(item_row).collect::<Vec<_>>() });
    for (k, val) in [("project", &v.project), ("remind", &v.remind), ("updated", &v.updated)] {
        if let Some(s) = val {
            o[k] = json!(s);
        }
    }
    for (k, val) in [("status", &v.status), ("model", &v.model), ("session", &v.session)] {
        if let Some(s) = val {
            o[k] = json!(s);
        }
    }
    for (k, val) in [("cost", v.cost), ("budget", v.budget)] {
        if let Some(n) = val {
            o[k] = json!(n);
        }
    }
    if v.images > 0 {
        o["images"] = json!(v.images);
    }
    if !d.loose_images.is_empty() {
        o["looseImages"] = json!(d.loose_images.iter().map(|i| json!({ "path": i.path, "caption": i.caption })).collect::<Vec<_>>());
    }
    if !d.questions.is_empty() {
        o["questions"] = json!(d.questions.iter().map(|q| json!({ "index": q.index, "text": q.text, "answer": q.answer })).collect::<Vec<_>>());
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
    no_done_items(text)?;
    let n = NewNote { text: text.into(), project: project.map(str::to_string), remind: arg(a, "remind").map(str::to_string), images: Vec::new(), kind: kind.map(str::to_string), title: None };
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
    let body = append(&d.body, text, style);
    if done_count(&body) > done_count(&d.body) {
        return Err(only_user_ticks());
    }
    let req = edit::SaveNote {
        path: path.to_string_lossy().into_owned(),
        expected_hash: d.hash.clone(),
        body,
        project: v.project.clone(),
        kind: Some(v.kind.clone()),
        title: None,
        remind: v.remind.clone(),
        images: Vec::new(),
    };
    match edit::save(root, &req, now)? {
        edit::SaveOutcome::Saved { note } => Ok(trf!("Připsáno ({} řádků v poznámce).", "Appended ({} lines in the note).", note.body.lines().count())),
        edit::SaveOutcome::Changed => Err(changed()),
    }
}

/// Kolik hotových bodů `- [x]` je v textu.
fn done_count(body: &str) -> usize {
    brief::scan_body(body, Path::new("")).items.iter().filter(|i| i.state == md::TaskState::Done).count()
}

fn only_user_ticks() -> String {
    tr(
        "`[x]` dává jen člověk v Ledge — hotový bod navrhni přes notes_propose_done (nastaví [~] a důkaz).",
        "Only the user ticks [x] in Ledge — propose a finished item with notes_propose_done (sets [~] and the evidence).",
    )
    .into()
}

/// Nová poznámka nesmí nést hotové body — `[x]` potvrzuje člověk.
fn no_done_items(text: &str) -> Result<(), String> {
    if done_count(text) > 0 { Err(only_user_ticks()) } else { Ok(()) }
}

/// `notes_toggle_task`: bod podle `text` (a/nebo `line` z `notes_read`, text vyhrává při posunu řádků).
/// Smí jen znovu otevřít (`done:false`); označit hotový nesmí — `[x]` dává jen člověk (viz `propose`).
pub fn toggle(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let line = a.get("line").and_then(Value::as_u64).map(|l| l as usize);
    let path = note_path(root, a)?;
    let d = edit::read(root, &path)?;
    let it = &d.items[brief::find_item(&d.items, line, arg(a, "text"))?];
    let was_done = it.state == md::TaskState::Done;
    if a.get("done").and_then(Value::as_bool).unwrap_or(!was_done) {
        return Err(only_user_ticks());
    }
    if it.state == md::TaskState::Open {
        return Ok(tr("Beze změny — už to tak je.", "No change — it's already like that.").into());
    }
    let n = notes::ops::task_set(root, &path, Some(it.line), Some(&it.text), md::TaskState::Open, None, now)?;
    Ok(trf!("Úkol znovu otevřený ({}/{} hotovo).", "Task reopened ({}/{} done).", n.view.tasks_done, n.view.tasks_total))
}

/// Stavy, které smí nastavit model; `draft`, `ready` a `done` jsou volba člověka v Ledge.
pub const AGENT_STATUSES: [&str; 3] = ["running", "waiting", "review"];

/// `notes_set_status`: jen `running` / `waiting` / `review`.
pub fn set_status(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let st = need(a, "status")?.to_ascii_lowercase();
    if !AGENT_STATUSES.contains(&st.as_str()) {
        return Err(trf!("Stav „{st}“ nastavuje člověk v Ledge; ty smíš {}.", "The status “{st}” is set by the user in Ledge; you may use {}.", AGENT_STATUSES.join(", ")));
    }
    notes::ops::set_status(root, &note_path(root, a)?, &st, now)?;
    Ok(trf!("Stav zadání: {st}.", "Brief status: {st}."))
}

/// `notes_ask`: otázky do sekce `## Otázky` (nejvýš 10, každá do 500 znaků).
pub fn ask(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let qs: Vec<String> = a.get("questions").and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_str).map(brief::one_line).filter(|q| !q.is_empty()).collect()).unwrap_or_default();
    if qs.is_empty() || qs.len() > 10 || qs.iter().any(|q| q.chars().count() > 500) {
        return Err(tr("„questions“: 1–10 neprázdných otázek, každá do 500 znaků.", "“questions”: 1–10 non-empty questions, each up to 500 characters.").into());
    }
    notes::ops::ask(root, &note_path(root, a)?, &qs, now)?;
    Ok(trf!("Přidáno otázek: {}. Odpověď zapíše člověk pod otázku (čti notes_read → questions).", "Questions added: {}. The user answers under each question (read notes_read → questions).", qs.len()))
}

/// `notes_propose_done`: bod → `[~] text — důkaz: …`. Nikdy `[x]`; hotový bod se nesníží.
pub fn propose_done(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let text = need(a, "text")?;
    let evidence = need(a, "evidence")?;
    let line = a.get("line").and_then(Value::as_u64).map(|l| l as usize);
    let path = note_path(root, a)?;
    let d = edit::read(root, &path)?;
    let it = &d.items[brief::find_item(&d.items, line, Some(text))?];
    if it.state == md::TaskState::Done {
        return Err(tr("Bod už člověk potvrdil jako hotový.", "The user already confirmed this item as done.").into());
    }
    notes::ops::task_set(root, &path, Some(it.line), Some(&it.text), md::TaskState::Proposed, Some(evidence), now)?;
    Ok(trf!("Navrženo jako hotové — čeká na potvrzení člověkem: {}", "Proposed as done — waiting for the user to confirm: {}", it.text))
}

/// `notes_log`: řádek `- <čas> text` do sekce `## Log` (do 1000 znaků).
pub fn log(root: &Path, a: &Value, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let text = need(a, "text")?;
    if text.chars().count() > 1000 {
        return Err(tr("Záznam do logu je nejvýš 1000 znaků.", "A log entry is at most 1000 characters.").into());
    }
    notes::ops::log(root, &note_path(root, a)?, text, now)?;
    Ok(tr("Zapsáno do logu.", "Logged.").into())
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
    fn item_rows_have_line_state_evidence_and_text() {
        let body = "# Nákup\n\n- [ ] mléko\n- [x] chleba\n```\n- [ ] kód\n```\n2. [~] tři — důkaz: test a\n  ![snímek](./a.png)";
        let r: Vec<Value> = brief::scan_body(body, Path::new("C:\\n")).items.iter().map(item_row).collect();
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], json!({"line":2,"done":false,"state":"open","text":"mléko"}));
        assert_eq!(r[1], json!({"line":3,"done":true,"state":"done","text":"chleba"}));
        assert_eq!((r[2]["line"].as_u64(), r[2]["state"].as_str(), r[2]["evidence"].as_str(), r[2]["images"][0]["caption"].as_str()), (Some(7), Some("proposed"), Some("test a"), Some("snímek")));
    }
}
