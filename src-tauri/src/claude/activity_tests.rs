//! Živá aktivita (`activity.rs`): fáze z nástrojů, příkazy testů, chyba, soubor, poslední zpráva z konce
//! přepisu (prázdný přepis → nic), vedoucí session a průchod hooky přes `Claude::handle_event`.

use super::activity::{self, Activity, Phase};
use super::*;
use serde_json::json;

#[test]
fn check_commands_table() {
    let yes = [
        "cargo test -p app",
        "CARGO_TARGET_DIR=C:/t cargo clippy --all-targets",
        "cd app && npm run build",
        "npm test",
        "pnpm run test:unit",
        "npx vitest run",
        "pytest -q tests/",
        "python -m pytest",
        "go test ./...",
        "node scripts/test.mjs",
        "tsc --noEmit",
        "C:\\tools\\cargo.exe build --release",
        "git status; cargo check",
    ];
    let no = ["cat tests/a.txt", "git commit -m \"test\"", "ls build", "echo cargo test", "npm install", "node app.js", "", "grep -rn test src"];
    for c in yes {
        assert!(activity::is_check_command(c), "{c}");
    }
    for c in no {
        assert!(!activity::is_check_command(c), "{c}");
    }
}

#[test]
fn phase_of_tools_table() {
    let n = Value::Null;
    let cases: &[(&str, Value, Value, Phase)] = &[
        ("Read", json!({"file_path":"C:/r/a.rs"}), n.clone(), Phase::Reading),
        ("Grep", json!({"pattern":"x"}), n.clone(), Phase::Reading),
        ("Glob", json!({"pattern":"*.rs"}), n.clone(), Phase::Reading),
        ("Edit", json!({"file_path":"C:/r/a.rs"}), n.clone(), Phase::Editing),
        ("Write", json!({"file_path":"C:/r/a.rs"}), n.clone(), Phase::Editing),
        ("MultiEdit", json!({"file_path":"C:/r/a.rs"}), n.clone(), Phase::Editing),
        ("Bash", json!({"command":"cargo test"}), json!({"stdout":"ok"}), Phase::Testing),
        ("PowerShell", json!({"command":"npm run build"}), n.clone(), Phase::Testing),
        ("Bash", json!({"command":"git status"}), n.clone(), Phase::Working),
        ("WebFetch", json!({"url":"https://example.com"}), n.clone(), Phase::Working),
        ("Task", json!({}), n.clone(), Phase::Working),
        // chyba jen ze skutečného pole výsledku
        ("Bash", json!({"command":"cargo test"}), json!({"exit_code":101}), Phase::Error),
        ("Bash", json!({"command":"ls"}), json!({"exitCode":2}), Phase::Error),
        ("Bash", json!({"command":"cargo test"}), json!({"exit_code":0}), Phase::Testing),
        ("Edit", json!({"file_path":"a"}), json!({"is_error":true}), Phase::Error),
        ("Bash", json!({"command":"cargo test"}), json!({"stderr":"error: x"}), Phase::Testing),
    ];
    for (tool, input, resp, want) in cases {
        assert_eq!(activity::phase_of(tool, input, resp), *want, "{tool} {input} {resp}");
    }
}

#[test]
fn steps_files_and_messages() {
    assert_eq!(activity::file_of(&json!({"file_path":" C:/r/a.rs "})).as_deref(), Some("C:/r/a.rs"));
    assert_eq!(activity::file_of(&json!({"notebook_path":"C:/r/n.ipynb"})).as_deref(), Some("C:/r/n.ipynb"));
    assert_eq!(activity::file_of(&json!({"file_path":"  "})), None);
    assert_eq!(activity::file_of(&json!({"command":"x"})), None);
    for (p, b) in [("C:\\r\\src\\main.rs", "main.rs"), ("/r/a.ts", "a.ts"), ("a.md", "a.md"), ("C:/r/dir/", "dir")] {
        assert_eq!(activity::base_name(p), b);
    }
    assert_eq!(activity::clean_message("  Hotovo,\n\n48 testů prošlo. ").as_deref(), Some("Hotovo, 48 testů prošlo."));
    assert_eq!(activity::clean_message("<system-reminder>x</system-reminder>"), None);
    assert_eq!(activity::clean_message(""), None);
    let long = activity::clean_message(&"a".repeat(500)).unwrap();
    assert_eq!(long.chars().count(), activity::MSG_MAX + 1);
}

fn line(v: Value) -> String {
    v.to_string()
}

#[test]
fn last_message_from_tail() {
    let text = |t: &str| line(json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":t}]}}));
    let tool = line(json!({"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{}}]}}));
    let user = line(json!({"type":"user","message":{"content":[{"type":"tool_result","content":"assistant"}]}}));
    let side = line(json!({"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"subagent"}]}}));
    let cases: Vec<(String, bool, Option<&str>)> = vec![
        // poslední text Clauda, i když po něm přišel nástroj a výsledek
        (format!("{}\n{}\n{}\n{}\n", text("první"), text("Hotovo, testy prošly."), tool, user), true, Some("Hotovo, testy prošly.")),
        // subagent se přeskočí
        (format!("{}\n{}\n", text("hlavní"), side), true, Some("hlavní")),
        // přepis bez zprávy Clauda → nic
        (format!("{}\n{}\n", tool, user), true, None),
        (String::new(), true, None),
        ("není json\n{\"type\":\"assistant\"".into(), true, None),
        // výřez od půlky: useknutý první řádek se nečte
        (format!("{}\n{}\n", text("useknutý"), tool), false, None),
        (format!("xx\n{}\n", text("celý")), false, Some("celý")),
    ];
    for (tail, whole, want) in cases {
        assert_eq!(activity::last_message(tail.as_bytes(), whole).as_deref(), want, "{tail}");
    }
}

#[test]
fn read_last_message_only_allowed_and_tail() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("projects");
    std::fs::create_dir_all(root.join("app")).unwrap();
    let p = root.join("app").join("s.jsonl");
    // dlouhý začátek (víc než TAIL_MAX) se nečte; zpráva na konci ano
    let filler = line(json!({"type":"user","message":{"content":"x".repeat(1000)}}));
    let mut body = String::new();
    for _ in 0..(activity::TAIL_MAX as usize / 1000 + 10) {
        body.push_str(&filler);
        body.push('\n');
    }
    body.push_str(&line(json!({"type":"assistant","message":{"content":[{"type":"text","text":"Hotovo."}]}})));
    body.push('\n');
    std::fs::write(&p, &body).unwrap();
    assert_eq!(activity::read_last_message(&p, Some(&root)).as_deref(), Some("Hotovo."));
    // mimo kořen / jiná přípona / chybí → nic
    let other = dir.path().join("s.jsonl");
    std::fs::write(&other, &body).unwrap();
    assert_eq!(activity::read_last_message(&other, Some(&root)), None);
    assert_eq!(activity::read_last_message(&root.join("app").join("s.txt"), Some(&root)), None);
    assert_eq!(activity::read_last_message(&root.join("app").join("chybi.jsonl"), Some(&root)), None);
    // přepis bez zprávy Clauda → prázdno
    std::fs::write(&p, format!("{filler}\n")).unwrap();
    assert_eq!(activity::read_last_message(&p, Some(&root)), None);
}

#[test]
fn next_by_events() {
    let ev = |e: &str, b: Value| -> Value {
        let mut b = b;
        b["hook_event_name"] = json!(e);
        b
    };
    let a = activity::next(None, "UserPromptSubmit", &ev("UserPromptSubmit", json!({})), 1).unwrap();
    assert_eq!((a.phase, a.file.as_deref()), (Phase::Working, None));
    let a = activity::next(Some(&a), "PostToolUse", &json!({"tool_name":"Edit","tool_input":{"file_path":"C:/r/a.rs"}}), 2).unwrap();
    assert_eq!((a.phase, a.file.as_deref(), a.tool.as_str()), (Phase::Editing, Some("C:/r/a.rs"), "Edit"));
    // Bash soubor nemá → zůstane poslední
    let a = activity::next(Some(&a), "PostToolUse", &json!({"tool_name":"Bash","tool_input":{"command":"cargo test"}}), 3).unwrap();
    assert_eq!((a.phase, a.file.as_deref()), (Phase::Testing, Some("C:/r/a.rs")));
    let a2 = activity::next(Some(&a), "PostToolUseFailure", &json!({"tool_name":"Bash","tool_input":{"command":"cargo test"}}), 4).unwrap();
    assert_eq!(a2.phase, Phase::Error);
    let a = activity::next(Some(&a), "Stop", &json!({"last_assistant_message":"Hotovo,\n testy prošly."}), 5).unwrap();
    assert_eq!((a.phase, a.message.as_deref(), a.at), (Phase::Done, Some("Hotovo, testy prošly."), 5));
    // nový tah: zpráva zůstane (je to pořád poslední zpráva), soubor se vynuluje
    let b = activity::next(Some(&a), "UserPromptSubmit", &json!({}), 6).unwrap();
    assert_eq!((b.phase, b.file, b.message.as_deref()), (Phase::Working, None, Some("Hotovo, testy prošly.")));
    // jiné události aktivitu nemění
    for e in ["SessionStart", "Notification", "SubagentStart", "SubagentStop", "SessionEnd"] {
        assert_eq!(activity::next(Some(&a), e, &json!({}), 9), None, "{e}");
    }
    assert!(activity::hook_has_message(&json!({"last_assistant_message":"ok"})));
    assert!(!activity::hook_has_message(&json!({"last_assistant_message":"  "})));
    assert!(!activity::hook_has_message(&json!({})));
}

fn sess_with(id: &str, state: SessionState, ended: bool, a: Option<Activity>) -> Session {
    let mut s = Session::new(id, "C:/r", state, 0);
    s.ended = ended;
    s.activity = a;
    s
}
fn a(phase: Phase, at: u64) -> Option<Activity> {
    Some(Activity { phase, tool: String::new(), file: None, message: None, at })
}

#[test]
fn lead_picks_freshest_live_phase() {
    let now = 1_000_000;
    let done_ms = 300_000;
    let ss = vec![
        sess_with("old-done", SessionState::Idle, false, a(Phase::Done, now - done_ms)),
        sess_with("edit", SessionState::Working, false, a(Phase::Editing, now - 50)),
        sess_with("ended", SessionState::Working, true, a(Phase::Testing, now - 1)),
        sess_with("idle-edit", SessionState::Idle, false, a(Phase::Editing, now - 2)),
        sess_with("none", SessionState::Working, false, None),
    ];
    assert_eq!(activity::lead(&ss, now, done_ms).map(|s| s.id.as_str()), Some("edit"));
    let ss2 = vec![sess_with("done", SessionState::Idle, false, a(Phase::Done, now - 10)), sess_with("edit", SessionState::Working, false, a(Phase::Editing, now - 50))];
    assert_eq!(activity::lead(&ss2, now, done_ms).map(|s| s.id.as_str()), Some("done"));
    // žádné sessions / jen staré → nic
    assert!(activity::lead(&Vec::<Session>::new(), now, done_ms).is_none());
    assert!(activity::lead(&ss[..1], now, done_ms).is_none());
}

#[test]
fn hooks_fill_activity_without_tail_by_default() {
    let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline();
    c.handle_event(&json!({"hook_event_name":"UserPromptSubmit","session_id":"a","cwd":"C:/r","prompt":"oprav to"}));
    c.handle_event(&json!({"hook_event_name":"PostToolUse","session_id":"a","tool_name":"Read","tool_input":{"file_path":"C:/r/src/main.rs"}}));
    let s = c.sessions().into_iter().find(|s| s.id == "a").unwrap();
    assert_eq!(s.activity.as_ref().map(|a| (a.phase, a.file.as_deref())), Some((Phase::Reading, Some("C:/r/src/main.rs"))));
    // Stop bez zprávy v hooku a s vypnutým čtením přepisu: zpráva zůstane prázdná (žádné čtení disku)
    let dir = tempfile::tempdir().unwrap();
    let tp = dir.path().join("s.jsonl");
    std::fs::write(&tp, "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"z přepisu\"}]}}\n").unwrap();
    c.handle_event(&json!({"hook_event_name":"Stop","session_id":"a","transcript_path":tp.to_string_lossy()}));
    let s = c.sessions().into_iter().find(|s| s.id == "a").unwrap();
    assert_eq!(s.activity.as_ref().map(|a| (a.phase, a.message.clone())), Some((Phase::Done, None)));
    // zpráva z hooku se vezme vždy (bez čtení)
    c.handle_event(&json!({"hook_event_name":"Stop","session_id":"a","last_assistant_message":"Hotovo."}));
    let s = c.sessions().into_iter().find(|s| s.id == "a").unwrap();
    assert_eq!(s.activity.and_then(|a| a.message).as_deref(), Some("Hotovo."));
}

#[test]
fn store_message_only_for_same_turn() {
    let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline();
    c.handle_event(&json!({"hook_event_name":"Stop","session_id":"a","cwd":"C:/r"}));
    let at = c.sessions()[0].activity.as_ref().unwrap().at;
    c.store_message("a", at + 1, Some("pozdě".into()));
    assert_eq!(c.sessions()[0].activity.as_ref().unwrap().message, None, "mezitím jiný tah → nezapsat");
    c.store_message("a", at, Some("z přepisu".into()));
    assert_eq!(c.sessions()[0].activity.as_ref().unwrap().message.as_deref(), Some("z přepisu"));
    c.store_message("a", at, None);
    assert_eq!(c.sessions()[0].activity.as_ref().unwrap().message.as_deref(), Some("z přepisu"));
}

#[test]
fn at_changes_only_with_phase() {
    let edit = |f: &str| json!({"tool_name":"Edit","tool_input":{"file_path":f}});
    let a = activity::next(None, "PostToolUse", &edit("C:/r/a.rs"), 10).unwrap();
    let b = activity::next(Some(&a), "PostToolUse", &edit("C:/r/b.rs"), 20).unwrap();
    assert_eq!((b.at, b.file.as_deref()), (10, Some("C:/r/b.rs")), "stejná fáze: čas zůstane, soubor se obnoví");
    let c = activity::next(Some(&b), "PostToolUse", &json!({"tool_name":"Read","tool_input":{}}), 30).unwrap();
    assert_eq!((c.phase, c.at), (Phase::Reading, 30), "nová fáze: nový čas");
}

#[test]
fn status_redacts_message_and_full_path() {
    let act = Activity { phase: Phase::Editing, tool: "Edit".into(), file: Some("C:/uzivatel/app/src/a.rs".into()), message: Some("tajná zpráva".into()), at: 1 };
    let mut s = sess_with("x", SessionState::Working, false, Some(act));
    s.transcript = Some("C:/uzivatel/.claude/projects/app/s.jsonl".into());
    let mut v = json!([s]);
    activity::redact(&mut v);
    let text = v.to_string();
    assert_eq!(v[0]["activity"]["file"], "a.rs");
    assert!(!text.contains("tajná") && !text.contains("uzivatel") && !text.contains("transcript"), "{text}");
}
