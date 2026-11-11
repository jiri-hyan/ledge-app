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

    let (err, text) = call(&srv, &host, "notes_create", json!({ "text": "Nákup\n\n- [ ] mléko\n- [ ] chleba", "kind": "ukol", "project": "acme", "remind": "2030-01-02T09:00" }));
    assert!(!err, "{text}");
    assert_eq!(host.note_writes.load(Ordering::SeqCst), 1);
    let (_, l) = call(&srv, &host, "notes_list", json!({}));
    assert!(!l.contains('\n') && !l.contains(": "), "kompaktní: {l}");
    let l = parse(&l);
    assert_eq!(l["total"], 1);
    let n = &l["notes"][0];
    assert_eq!((n["title"].as_str(), n["kind"].as_str(), n["project"].as_str(), n["tasks"].as_str()), (Some("Nákup"), Some("ukol"), Some("acme"), Some("0/2")));
    assert!(n["remind"].as_str().unwrap().starts_with("2030-01-02T09:00:00"), "{n}");
    let path = n["path"].as_str().unwrap().to_string();

    // filtry
    assert_eq!(parse(&call(&srv, &host, "notes_list", json!({ "kind": "napad" })).1)["total"], 0);
    assert_eq!(parse(&call(&srv, &host, "notes_list", json!({ "project": "acme" })).1)["total"], 1);
    assert_eq!(parse(&call(&srv, &host, "notes_list", json!({ "project": "_obecne" })).1)["total"], 0);

    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": path })).1);
    assert_eq!(r["tasks"], json!([{ "line": 2, "done": false, "state": "open", "text": "mléko" }, { "line": 3, "done": false, "state": "open", "text": "chleba" }]));
    assert!(r["body"].as_str().unwrap().starts_with("Nákup"));
    // relativní cesta ke kořeni projde taky
    let rel = std::path::Path::new(&path).strip_prefix(dir.path()).unwrap().to_string_lossy().into_owned();
    assert!(!call(&srv, &host, "notes_read", json!({ "path": rel })).0);

    // připsání naváže na seznam; metadata zůstanou
    let (err, text) = call(&srv, &host, "notes_append", json!({ "path": path, "text": "vejce", "style": "task" }));
    assert!(!err, "{text}");
    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": path })).1);
    assert!(r["body"].as_str().unwrap().ends_with("- [ ] chleba\n- [ ] vejce"), "{r}");
    assert_eq!((r["kind"].as_str(), r["project"].as_str(), r["done"].as_bool()), (Some("ukol"), Some("acme"), Some(false)));
    assert!(r["remind"].as_str().unwrap().starts_with("2030-01-02T09:00:00"), "připomínka zůstala");
    assert!(call(&srv, &host, "notes_append", json!({ "path": path, "text": "x", "style": "nic" })).0, "neznámý styl");

    // úkol: model `[x]` nikdy nedá (ani explicitně, ani přepnutím); řádek bez úkolu = chyba
    let (err, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 2, "done": true }));
    assert!(err && text.contains("notes_propose_done"), "{text}");
    let (err, _) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 2 }));
    assert!(err, "přepnutí otevřeného by bylo [x]");
    let (_, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "text": "mléko", "done": false }));
    assert!(text.contains("Beze změny"), "{text}");
    let (err, text) = call(&srv, &host, "notes_toggle_task", json!({ "path": path, "line": 0 }));
    assert!(err && text.contains('0'), "{text}");
    assert!(!std::fs::read_to_string(&path).unwrap().contains("[x]"));
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
    for t in ["notes_read", "notes_append", "notes_toggle_task", "notes_pin", "notes_set_status", "notes_ask", "notes_propose_done", "notes_log"] {
        for p in [evil.to_string_lossy().into_owned(), dotdot.clone(), "..\\x.md".into(), "neni.md".into(), dir.path().join("a.txt").to_string_lossy().into_owned()] {
            let (err, text) = call(&srv, &host, t, json!({ "path": p, "text": "x", "line": 0, "status": "review", "questions": ["q"], "evidence": "e" }));
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

const BRIEF: &str = "---\ncreated: 2026-11-11T10:00:00+01:00\nproject: acme\nstatus: ready\nx-vlastni: ahoj\n---\n\n# Cíl\n- [ ] Opravit header @src/Header.tsx:42\n  ![Rozbitý header](./a-1.png)\n- [ ] Dokončit login\n\n![volný snímek](./b.png)\n";

fn brief_host() -> (MemHost, tempfile::TempDir, String, tempfile::TempDir) {
    let (host, dir) = notes_host();
    let proj = tempfile::Builder::new().prefix("ledge-mcp-proj-").tempdir_in(std::env::temp_dir()).unwrap();
    std::fs::create_dir_all(proj.path().join("src")).unwrap();
    std::fs::write(proj.path().join("src/Header.tsx"), "x").unwrap();
    host.cfg.lock().projects[0].path = Some(proj.path().to_string_lossy().into_owned());
    let path = dir.path().join("2026-11-11-100000-zadani.md");
    std::fs::write(&path, BRIEF).unwrap();
    (host, dir, path.to_string_lossy().into_owned(), proj)
}

#[test]
fn read_returns_states_evidence_images_refs_and_questions() {
    let srv = server();
    let (host, _dir, path, proj) = brief_host();
    assert!(!call(&srv, &host, "notes_propose_done", json!({ "path": path, "text": "Dokončit login", "evidence": "commit abc123" })).0);
    assert!(!call(&srv, &host, "notes_ask", json!({ "path": path, "questions": ["Který branch?", "Smím měnit schéma?"] })).0);
    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": path })).1);
    assert_eq!((r["status"].as_str(), r["project"].as_str()), (Some("ready"), Some("acme")));
    let t = &r["tasks"];
    assert_eq!((t[0]["state"].as_str(), t[0]["line"].as_u64()), (Some("open"), Some(1)));
    assert_eq!(t[0]["images"][0]["caption"], "Rozbitý header");
    assert!(t[0]["images"][0]["path"].as_str().unwrap().ends_with("a-1.png"));
    let rf = &t[0]["refs"][0];
    assert_eq!((rf["raw"].as_str(), rf["line"].as_u64(), rf["exists"].as_bool()), (Some("@src/Header.tsx:42"), Some(42), Some(true)));
    assert!(rf["path"].as_str().unwrap().starts_with(&*proj.path().to_string_lossy()));
    assert_eq!((t[1]["state"].as_str(), t[1]["evidence"].as_str(), t[1]["done"].as_bool()), (Some("proposed"), Some("commit abc123"), Some(false)));
    assert_eq!(r["looseImages"][0]["caption"], "volný snímek");
    assert_eq!(r["questions"], json!([{ "index": 0, "text": "Který branch?", "answer": null }, { "index": 1, "text": "Smím měnit schéma?", "answer": null }]));
    // čtení nic nezměnilo; jen sekce Otázky a [~] přibyly, neznámé klíče a zbytek těla zůstaly
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("x-vlastni: ahoj\n") && text.contains("- [~] Dokončit login — důkaz: commit abc123\n"), "{text}");
    assert!(text.contains("## Otázky\n\n- ❓ Který branch?\n- ❓ Smím měnit schéma?"), "{text}");
}

#[test]
fn model_can_never_write_done_over_mcp() {
    let srv = server();
    let (host, dir, path, _proj) = brief_host();
    let p = path.as_str();
    let tries: Vec<(&str, Value)> = vec![
        ("notes_toggle_task", json!({ "path": p, "line": 1, "done": true })),
        ("notes_toggle_task", json!({ "path": p, "text": "Dokončit login", "done": true })),
        ("notes_toggle_task", json!({ "path": p, "line": 1 })),
        ("notes_append", json!({ "path": p, "text": "- [x] podvod" })),
        ("notes_append", json!({ "path": p, "text": "hotovo\n- [X] podvod" })),
        ("notes_append", json!({ "path": p, "text": "- [x] podvod", "style": "bullet" })),
        ("notes_create", json!({ "text": "- [x] hotovo hned" })),
        ("notes_set_status", json!({ "path": p, "status": "done" })),
        ("notes_set_status", json!({ "path": p, "status": "draft" })),
    ];
    for (tool, args) in tries {
        let (err, text) = call(&srv, &host, tool, args.clone());
        // `bullet` s `[x]` se do bodu neshodí — odrážka s textem, ne úkol, a to je v pořádku
        if tool == "notes_append" && args["style"] == "bullet" {
            assert!(!err);
            continue;
        }
        assert!(err, "{tool} {args}: {text}");
    }
    // propose nikdy nevyrobí [x], ani s důkazem napsaným jako [x]
    assert!(!call(&srv, &host, "notes_propose_done", json!({ "path": p, "text": "Dokončit login", "evidence": "[x] hotovo" })).0);
    for f in std::fs::read_dir(dir.path()).unwrap().flatten() {
        let t = std::fs::read_to_string(f.path()).unwrap_or_default();
        let sc = notes::brief::scan_body(&t, std::path::Path::new(""));
        assert!(sc.items.iter().all(|i| i.state != notes::md::TaskState::Done), "{}: {t}", f.path().display());
    }
    assert!(std::fs::read_to_string(&path).unwrap().contains("status: ready"), "stav zůstal");
}

#[test]
fn propose_set_status_log_and_answer_flow() {
    let srv = server();
    let (host, _dir, path, _proj) = brief_host();
    let p = path.as_str();
    // nenalezený / nejednoznačný bod, chybějící důkaz
    for bad in [json!({ "path": p, "text": "Nic", "evidence": "x" }), json!({ "path": p, "text": "Dokončit login" }), json!({ "path": p, "evidence": "x" })] {
        assert!(call(&srv, &host, "notes_propose_done", bad).0);
    }
    // line je jen nápověda (posunutý řádek se dohledá podle textu)
    let (err, text) = call(&srv, &host, "notes_propose_done", json!({ "path": p, "text": "Opravit header @src/Header.tsx:42", "line": 7, "evidence": "test `header_ok`" }));
    assert!(!err, "{text}");
    // člověk potvrdí (Tauri cesta) — model už bod nesníží
    let root = host.notes.lock().clone().unwrap();
    notes::ops::task_set(&root, std::path::Path::new(p), Some(1), Some("Opravit header @src/Header.tsx:42"), notes::md::TaskState::Done, None, &notes::now_local()).unwrap();
    let (err, text) = call(&srv, &host, "notes_propose_done", json!({ "path": p, "text": "Opravit header @src/Header.tsx:42", "evidence": "znovu" }));
    assert!(err && (text.contains("potvrdil") || text.contains("confirmed")), "{text}");
    // stavy: jen running / waiting / review
    for ok in ["running", "waiting", "review"] {
        assert!(!call(&srv, &host, "notes_set_status", json!({ "path": p, "status": ok })).0, "{ok}");
    }
    assert!(std::fs::read_to_string(&path).unwrap().contains("status: review\n"));
    // log: sekce se založí, pak se jen připisuje
    assert!(!call(&srv, &host, "notes_log", json!({ "path": p, "text": "začal jsem\nna headeru" })).0);
    assert!(!call(&srv, &host, "notes_log", json!({ "path": p, "text": "hotovo" })).0);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("## Log\n\n- ") && text.contains(" začal jsem na headeru\n- ") && text.trim_end().ends_with(" hotovo"), "{text}");
    assert!(call(&srv, &host, "notes_log", json!({ "path": p, "text": "x".repeat(1001) })).0);
    // otázky: meze
    assert!(call(&srv, &host, "notes_ask", json!({ "path": p, "questions": [] })).0);
    assert!(call(&srv, &host, "notes_ask", json!({ "path": p, "questions": ["  "] })).0);
    assert!(call(&srv, &host, "notes_ask", json!({ "path": p })).0);
    assert!(call(&srv, &host, "notes_ask", json!({ "path": p, "questions": vec!["q"; 11] })).0);
    // odpověď člověka se čte zpět přes notes_read
    assert!(!call(&srv, &host, "notes_ask", json!({ "path": p, "questions": ["Smím?"] })).0);
    notes::ops::answer_question(&root, std::path::Path::new(p), 0, "ano", &notes::now_local()).unwrap();
    let r = parse(&call(&srv, &host, "notes_read", json!({ "path": p })).1);
    assert_eq!(r["questions"][0]["answer"], "ano");
    assert_eq!(host.saves.load(Ordering::SeqCst), 0);
}
