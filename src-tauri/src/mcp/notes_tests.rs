//! Testy nástrojů `notes_*` nad `MemHost` s dočasným kořenem poznámek (nikdy skutečný
//! `%USERPROFILE%\Ledge`). Skutečné okno lístku a `reload_notes` jsou jen v `host.rs`.

use super::tests::{MemHost, server};
use super::*;
use crate::notes;
use serde_json::json;
use std::sync::atomic::Ordering;

fn call(srv: &Server, host: &MemHost, name: &str, args: Value) -> (bool, String) {
    let h = tools::Handler { srv, host };
    let r = <tools::Handler as rpc::Tools>::call(&h, name, &args).unwrap();
    (r["isError"].as_bool().unwrap(), r["content"][0]["text"].as_str().unwrap().to_string())
}

/// Host s dočasným kořenem poznámek a dvěma projekty v configu.
fn notes_host() -> (MemHost, tempfile::TempDir) {
    let dir = tempfile::Builder::new().prefix("ledge-mcp-notes-").tempdir_in(std::env::temp_dir()).unwrap();
    let host = MemHost::new(true);
    *host.notes.lock() = Some(dir.path().to_path_buf());
    let p = |id: &str| -> crate::config::Project { serde_json::from_value(json!({ "id": id, "name": id, "kind": "dev" })).unwrap() };
    host.cfg.lock().projects = vec![p("acme"), p("web")];
    (host, dir)
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("není JSON ({e}): {text}"))
}

#[test]
fn notes_tools_off_without_root() {
    let srv = server();
    let host = MemHost::new(true);
    for (t, a) in [("notes_list", json!({})), ("notes_read", json!({ "path": "x.md" })), ("notes_create", json!({ "text": "a" })), ("notes_pin", json!({ "path": "x.md" }))] {
        let (err, text) = call(&srv, &host, t, a);
        assert!(err && text.contains("vypnuté"), "{t}: {text}");
    }
}

#[test]
fn create_list_read_append_toggle() {
    let srv = server();
    let (host, dir) = notes_host();
    // špatný projekt / kategorie / čas / prázdný text = chyba, nic na disku
    for bad in [json!({ "text": "a", "project": "nikdo" }), json!({ "text": "a", "kind": "nic" }), json!({ "text": "a", "remind": "někdy" }), json!({ "text": "  " }), json!({})] {
        let (err, text) = call(&srv, &host, "notes_create", bad.clone());
        assert!(err, "{bad}: {text}");
    }
    assert_eq!(host.note_writes.load(Ordering::SeqCst), 0);
    assert!(notes::scan(dir.path()).is_empty());

    let (err, text) = call(&srv, &host, "notes_create", json!({ "text": "Nákup\n\n- [ ] mléko\n- [x] chleba", "kind": "ukol", "project": "acme", "remind": "2030-01-02T09:00" }));
    assert!(!err, "{text}");
    assert_eq!(host.note_writes.load(Ordering::SeqCst), 1);
    let (_, l) = call(&srv, &host, "notes_list", json!({}));
    assert!(!l.contains('\n') && !l.contains(": "), "kompaktní: {l}");
    let l = parse(&l);
    assert_eq!(l["total"], 1);
    let n = &l["notes"][0];
    assert_eq!((n["title"].as_str(), n["kind"].as_str(), n["project"].as_str(), n["tasks"].as_str()), (Some("Nákup"), Some("ukol"), Some("acme"), Some("1/2")));
    assert!(n["remind"].as_str().unwrap().starts_with("2030-01-02T09:00:00"), "{n}");
    let path = n["path"].as_str().unwrap().to_string();

    // filtry
    assert_eq!(parse(&call(&srv, &host, "notes_list", json!({ "kind": "napad" })).1)["total"], 0);
    assert_eq!(parse(&call(&srv, &host, "notes_list", json!({ "project": "acme" })).1)["total"], 1);
    assert_eq!(parse(&call(&srv, &host, "notes_list", json!({ "project": "_obecne" })).1)["total"], 0);

    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": path })).1);
    assert_eq!(r["tasks"], json!([{ "line": 2, "done": false, "text": "mléko" }, { "line": 3, "done": true, "text": "chleba" }]));
    assert!(r["body"].as_str().unwrap().starts_with("Nákup"));
    // relativní cesta ke kořeni projde taky
    let rel = std::path::Path::new(&path).strip_prefix(dir.path()).unwrap().to_string_lossy().into_owned();
    assert!(!call(&srv, &host, "notes_read", json!({ "path": rel })).0);

    // připsání naváže na seznam; metadata zůstanou
    let (err, text) = call(&srv, &host, "notes_append", json!({ "path": path, "text": "vejce", "style": "task" }));
    assert!(!err, "{text}");
    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": path })).1);
    assert!(r["body"].as_str().unwrap().ends_with("- [x] chleba\n- [ ] vejce"), "{r}");
    assert_eq!((r["kind"].as_str(), r["project"].as_str(), r["done"].as_bool()), (Some("ukol"), Some("acme"), Some(false)));
    assert!(r["remind"].as_str().unwrap().starts_with("2030-01-02T09:00:00"), "připomínka zůstala");
    assert!(call(&srv, &host, "notes_append", json!({ "path": path, "text": "x", "style": "nic" })).0, "neznámý styl");

    // úkol: explicitně i přepnutím; řádek bez úkolu = chyba
    let (err, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 2, "done": true }));
    assert!(!err && text.contains("2/3"), "{text}");
    let (_, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 2, "done": true }));
    assert!(text.contains("Beze změny"), "{text}");
    let (err, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 0 }));
    assert!(err && text.contains('0'), "{text}");
    let (err, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 3 }));
    assert!(!err && text.contains("1/3"), "přepnutí bez done: {text}");
    assert_eq!(host.saves.load(Ordering::SeqCst), 0, "poznámky nesahají na config");
    assert_eq!(srv.undo.lock().len(), 0, "poznámky nejsou ve Vrátit");
}

#[test]
fn paths_stay_inside_root() {
    let srv = server();
    let (host, dir) = notes_host();
    let outside = tempfile::Builder::new().prefix("ledge-mcp-out-").tempdir_in(std::env::temp_dir()).unwrap();
    let evil = outside.path().join("tajne.md");
    std::fs::write(&evil, "tajemství").unwrap();
    let inside = dir.path().join("a.md");
    std::fs::write(&inside, "ahoj\n").unwrap();
    let dotdot = format!("{}/../{}/tajne.md", dir.path().display(), outside.path().file_name().unwrap().to_string_lossy());
    for t in ["notes_read", "notes_append", "notes_toggle_task", "notes_pin"] {
        for p in [evil.to_string_lossy().into_owned(), dotdot.clone(), "..\\x.md".into(), "neni.md".into(), dir.path().join("a.txt").to_string_lossy().into_owned()] {
            let (err, text) = call(&srv, &host, t, json!({ "path": p, "text": "x", "line": 0 }));
            assert!(err && !text.contains("tajemství"), "{t} {p}: {text}");
        }
    }
    assert!(host.pins.lock().is_empty(), "mimo kořen se nic nepřipne");
    assert_eq!(std::fs::read_to_string(&evil).unwrap(), "tajemství");
    // vnitřní poznámka se připne a odepne přes Host
    let p = inside.to_string_lossy().into_owned();
    assert!(!call(&srv, &host, "notes_pin", json!({ "path": p })).0);
    assert!(!call(&srv, &host, "notes_pin", json!({ "path": p, "pinned": false })).0);
    assert_eq!(host.pins.lock().iter().map(|x| x.1).collect::<Vec<_>>(), vec![true, false]);
}

#[test]
fn append_keeps_front_matter_and_read_is_capped() {
    let srv = server();
    let (host, dir) = notes_host();
    let path = dir.path().join("b.md");
    std::fs::write(&path, "---\ncreated: 2026-10-01T10:00:00+02:00\n---\n\nprvní\n").unwrap();
    assert!(!call(&srv, &host, "notes_append", json!({ "path": path.to_string_lossy(), "text": "druhý" })).0);
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("první\n\ndruhý\n") && after.contains("created: 2026-10-01T10:00:00+02:00") && after.contains("updated:"), "{after}");
    // příliš dlouhé tělo se v notes_read ořízne
    std::fs::write(dir.path().join("c.md"), "x".repeat(super::notes::BODY_CAP + 500)).unwrap();
    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": dir.path().join("c.md").to_string_lossy() })).1);
    assert_eq!((r["truncated"].as_bool(), r["body"].as_str().unwrap().chars().count()), (Some(true), super::notes::BODY_CAP));
}
