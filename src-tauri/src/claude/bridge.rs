//! HTTP most na 127.0.0.1 (`tiny_http`): `POST /permission`, `POST /question`, `POST /plan`, `POST /event`, `GET /status`,
//! `POST /ctl/<příkaz>`. Odmítá prohlížeč (`Origin`) a cizí `Host`. Každý požadavek ve vlastním
//! vlákně; logiku dělá `Claude` (`mod.rs`), tady je jen směrování a odpovědi.

use super::{Claude, rules};
use serde_json::{Value, json};
use std::sync::Arc;

fn header<'a>(req: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    req.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name)).map(|h| h.value.as_str())
}

fn respond(req: tiny_http::Request, code: u16, body: String) {
    let ct = tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap();
    let _ = req.respond(tiny_http::Response::from_string(body).with_status_code(code).with_header(ct));
}

/// Obsluha `POST /ctl/<příkaz>` (otevřít/zavřít šuplík, reload…) — ovládání pro Clauda a skripty.
pub type Ctl = Arc<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>;

/// Spustí server. `extra` obslouží /status rozšířený o stav Ledge, `ctl` příkazy `/ctl/…`.
pub fn serve(port: u16, claude: Claude, extra: Arc<dyn Fn() -> Value + Send + Sync>, ctl: Ctl) -> Result<(), String> {
    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| format!("Port {port}: {e}"))?;
    std::thread::Builder::new()
        .name("claude-bridge".into())
        .spawn(move || {
            for mut req in server.incoming_requests() {
                // prohlížeč (i DNS rebinding) sem nesmí
                if header(&req, "Origin").is_some() {
                    respond(req, 403, "{}".into());
                    continue;
                }
                let host_ok = header(&req, "Host")
                    .map(|h| h.starts_with("127.0.0.1") || h.starts_with("localhost") || h.starts_with("[::1]"))
                    .unwrap_or(true);
                if !host_ok {
                    respond(req, 403, "{}".into());
                    continue;
                }
                let url = req.url().to_string();
                let method = req.method().as_str().to_string();
                let mut body = String::new();
                if method == "POST" {
                    use std::io::Read;
                    let _ = req.as_reader().take(1 << 20).read_to_string(&mut body);
                }
                let claude = claude.clone();
                let extra = extra.clone();
                let ctl = ctl.clone();
                // každý požadavek ve vlastním vlákně — schválení může čekat desítky sekund
                std::thread::spawn(move || {
                    let parsed: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    match (method.as_str(), url.as_str()) {
                        // allow / allow + updatedPermissions / deny (+ důvod); `Pass` = prázdné tělo →
                        // žádné rozhodnutí, terminál se zeptá normálně (`rules::body`)
                        ("POST", "/permission") => respond(req, 200, rules::body(&claude.handle_permission(&parsed))),
                        // otázka Clauda: tělo jen s odpovědí z Ledge, jinak prázdné = otázka v okně Claude
                        ("POST", "/question") => respond(req, 200, claude.handle_question(&parsed).unwrap_or_default()),
                        // plán (PreToolUse ExitPlanMode): allow + updatedInput / deny + důvod, jinak prázdné = dialog v Claude
                        ("POST", "/plan") => respond(req, 200, claude.handle_plan(&parsed).unwrap_or_default()),
                        ("POST", "/event") => {
                            // prázdné tělo = žádné rozhodnutí; JSON jen u Stop v režimu block
                            let reply = claude.handle_event(&parsed);
                            respond(req, 200, reply.unwrap_or_default());
                        }
                        ("GET", "/status") => {
                            let mut v = claude.status_json();
                            if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra()) {
                                o.extend(e);
                            }
                            respond(req, 200, v.to_string());
                        }
                        ("POST", p) if p.starts_with("/ctl/") => match ctl(&p[5..], &parsed) {
                            Ok(v) => respond(req, 200, v.to_string()),
                            Err(e) => respond(req, 400, json!({ "error": e }).to_string()),
                        },
                        _ => respond(req, 404, "{}".into()),
                    }
                });
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::testkit::{ev, git, sess, temp_repo_upstream};
    use super::super::{Decision, SessionState};
    use crate::config::EnforceCommit;
    use std::time::Duration;

    fn bridge(timeout_ms: u64) -> (Claude, u16) {
        // inline: baseline z prvního eventu musí být hotová dřív, než přijde Stop
        let c = Claude::new(Arc::new(|| {}), Duration::from_millis(timeout_ms)).inline().patient();
        let port = crate::ports::ephemeral().unwrap();
        serve(port, c.clone(), Arc::new(|| json!({})), Arc::new(|_, _| Ok(json!({})))).unwrap();
        (c, port)
    }

    fn post(port: u16, path: &str, body: &str, origin: bool) -> (u16, String) {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let o = if origin { "Origin: http://evil.example\r\n" } else { "" };
        write!(s, "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{o}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let code = out[9..12].parse().unwrap();
        let body = out.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (code, body)
    }

    const REQ: &str = r#"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"firebase deploy --only hosting:staging"}}"#;

    #[test]
    fn allow_only_on_explicit_decision() {
        let (c, port) = bridge(5000);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(c2.pending()[0].summary, "firebase deploy --only hosting:staging");
            c2.decide(None, Decision::Allow);
        });
        let (code, body) = post(port, "/permission", REQ, false);
        assert_eq!(code, 200);
        assert!(body.contains("\"allow\""));
        assert!(c.pending().is_empty());
    }

    #[test]
    fn deny_carries_message() {
        let (c, port) = bridge(5000);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            c2.decide(None, Decision::Deny);
        });
        let (_, body) = post(port, "/permission", REQ, false);
        assert!(body.contains("\"deny\""));
    }

    /// „Vždy povolit“ z karty: pravidlo z návrhu Clauda, cíl podle volby člověka (ne podle návrhu).
    #[test]
    fn always_allow_sends_rule_from_suggestion() {
        let (c, port) = bridge(5000);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            let p = &c2.pending()[0];
            assert_eq!(p.always.as_ref().map(|a| a.label.as_str()), Some("Bash(npm run lint:*)"));
            let ex = |d| rules::Extra { always: Some(d), ..Default::default() };
            assert!(!c2.decide_with(None, true, &ex(rules::Destination::UserSettings)), "vždy jen pro konkrétní kartu");
            assert!(!c2.decide_with(Some(p.id), false, &ex(rules::Destination::UserSettings)), "vždy + zamítnout nedává smysl");
            assert!(c2.decide_with(Some(p.id), true, &ex(rules::Destination::UserSettings)));
        });
        let req = r#"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npm run lint -- --fix"},
            "permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"npm run lint:*"}],"behavior":"allow","destination":"localSettings"},{"type":"setMode","mode":"acceptEdits","destination":"session"}]}"#;
        let (code, body) = post(port, "/permission", req, false);
        assert_eq!(code, 200);
        let v: Value = serde_json::from_str(&body).unwrap();
        let d = &v["hookSpecificOutput"]["decision"];
        assert_eq!(d["behavior"], "allow");
        assert_eq!(d["updatedPermissions"], json!([{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"npm run lint:*"}],"behavior":"allow","destination":"userSettings"}]));
    }

    #[test]
    fn deny_with_reason_reaches_claude() {
        let (c, port) = bridge(5000);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            let id = c2.pending()[0].id;
            assert!(c2.decide_with(Some(id), false, &rules::Extra { reason: Some("nasazuj jen staging \"x\"".into()), ..Default::default() }));
        });
        let (_, body) = post(port, "/permission", REQ, false);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["hookSpecificOutput"]["decision"], json!({"behavior": "deny", "message": "Zamítnuto v Ledge: nasazuj jen staging \"x\""}));
    }

    #[test]
    fn timeout_returns_empty_body_never_allow() {
        let (c, port) = bridge(300);
        let (code, body) = post(port, "/permission", REQ, false);
        assert_eq!(code, 200);
        assert!(body.is_empty(), "timeout musí vrátit rozhodnutí terminálu, dostal: {body}");
        assert!(c.pending().is_empty());
    }

    #[test]
    fn answered_in_terminal_releases_pending() {
        let (c, port) = bridge(10_000);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            c2.handle_event(&json!({"hook_event_name":"PostToolUse","session_id":"s1"}));
        });
        let t0 = std::time::Instant::now();
        let (_, body) = post(port, "/permission", REQ, false);
        assert!(body.is_empty());
        assert!(t0.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn garbage_body_passes_without_pending() {
        let (c, port) = bridge(5000);
        let (code, body) = post(port, "/permission", "{nejsonový", false);
        assert_eq!(code, 200);
        assert!(body.is_empty());
        assert!(c.pending().is_empty() && c.sessions().is_empty());
    }

    #[test]
    fn browser_requests_are_rejected() {
        let (c, port) = bridge(5000);
        let (code, _) = post(port, "/permission", REQ, true);
        assert_eq!(code, 403);
        assert!(c.pending().is_empty());
    }

    const ASK: &str = r#"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PreToolUse","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"Kam to nasadit?","header":"Cíl","multiSelect":false,"options":[{"label":"staging","description":"testovací"},{"label":"produkce","description":"ostrá"}]},{"question":"Co dál?","header":"Kroky","multiSelect":true,"options":[{"label":"testy"},{"label":"changelog"}]}]}}"#;

    #[test]
    fn question_answered_in_ledge_over_http() {
        let (c, port) = bridge(5000);
        c.set_question_hold(5);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.questions().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            let id = c2.questions()[0].id;
            let r = |l: &[&str], o: Option<&str>| super::super::Reply { labels: l.iter().map(|x| x.to_string()).collect(), other: o.map(str::to_string) };
            c2.answer_question(id, &[r(&[], Some("jen náhled")), r(&["changelog", "testy"], None)]).unwrap();
        });
        let (code, body) = post(port, "/question", ASK, false);
        assert_eq!(code, 200);
        let v: Value = serde_json::from_str(&body).unwrap();
        let out = &v["hookSpecificOutput"];
        assert_eq!((out["hookEventName"].as_str(), out["permissionDecision"].as_str()), (Some("PreToolUse"), Some("allow")));
        let sent: Value = serde_json::from_str(ASK).unwrap();
        assert_eq!(out["updatedInput"]["questions"], sent["tool_input"]["questions"], "původní otázky beze změny");
        assert_eq!(out["updatedInput"]["answers"], json!({"Kam to nasadit?": "jen náhled", "Co dál?": "testy, changelog"}));
    }

    #[test]
    fn question_timeout_and_permission_pass_over_http() {
        let (c, port) = bridge(5000);
        *c.question_hold.lock() = Duration::from_millis(200);
        assert_eq!(post(port, "/question", ASK, false), (200, String::new()));
        assert!(c.questions().is_empty());
        // PermissionRequest pro AskUserQuestion: hned prázdná odpověď, žádná karta
        let perm = ASK.replace("PreToolUse", "PermissionRequest");
        let t0 = std::time::Instant::now();
        assert_eq!(post(port, "/permission", &perm, false), (200, String::new()));
        assert!(t0.elapsed() < Duration::from_secs(2) && c.pending().is_empty());
        assert_eq!(post(port, "/question", ASK, true).0, 403, "prohlížeč ne");
    }

    /// Plán přes `PreToolUse(ExitPlanMode)`: karta „Plán“, schválení = allow + celý vstup; termín = prázdno
    /// a následný `PermissionRequest` téhož plánu kartu nezaloží.
    #[test]
    fn plan_over_http_one_card() {
        const PLAN: &str = r##"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PreToolUse","tool_name":"ExitPlanMode","tool_use_id":"toolu_01","tool_input":{"plan":"# Plán A\n- krok","planFilePath":"C:/x/.claude/plans/a.md"}}"##;
        let (c, port) = bridge(5000);
        let c2 = c.clone();
        std::thread::spawn(move || {
            while c2.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
            let p = &c2.pending()[0];
            assert_eq!((p.summary.as_str(), p.via), ("Plán A", super::super::plan::Via::PreToolUse));
            c2.decide(Some(p.id), Decision::Allow);
        });
        let (code, body) = post(port, "/plan", PLAN, false);
        assert_eq!(code, 200);
        let out = &serde_json::from_str::<Value>(&body).unwrap()["hookSpecificOutput"];
        assert_eq!((out["hookEventName"].as_str(), out["permissionDecision"].as_str()), (Some("PreToolUse"), Some("allow")));
        assert_eq!(out["updatedInput"], serde_json::from_str::<Value>(PLAN).unwrap()["tool_input"]);

        let (c, port) = bridge(200);
        assert_eq!(post(port, "/plan", PLAN, false), (200, String::new()), "termín = dialog v Claude");
        let perm = PLAN.replace("PreToolUse", "PermissionRequest");
        let t0 = std::time::Instant::now();
        assert_eq!(post(port, "/permission", &perm, false), (200, String::new()));
        assert!(t0.elapsed() < Duration::from_secs(2) && c.pending().is_empty(), "druhá karta nevznikla");
    }

    #[test]
    fn stop_block_over_http() {
        let (repo, _remote) = temp_repo_upstream();
        let cwd = repo.path().to_string_lossy().replace('\\', "/");
        let (c, port) = bridge(5000);
        let body = |name: &str, sid: &str, extra: Value| ev(name, sid, &cwd, extra).to_string();
        let dirty = |sid: &str| {
            std::fs::write(repo.path().join(format!("{sid}.txt")), "x").unwrap();
            post(port, "/event", &body("PostToolUse", sid, json!({"tool_name":"Edit","tool_input":{"file_path": format!("{cwd}/{sid}.txt")}})), false)
        };

        // notify (výchozí) → prázdné tělo
        assert_eq!(dirty("n"), (200, String::new()));
        assert_eq!(post(port, "/event", &body("Stop", "n", json!({"stop_hook_active": false})), false), (200, String::new()));

        // block → JSON s důvodem, session zpět v práci
        c.set_enforce(EnforceCommit::Block);
        dirty("b");
        let (code, resp) = post(port, "/event", &body("Stop", "b", json!({"stop_hook_active": false})), false);
        assert_eq!(code, 200);
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["decision"], "block");
        assert!(v["reason"].as_str().unwrap().starts_with("Máš nezacommitované své změny: b.txt. Commitni je"));
        assert_eq!(sess(&c, "b").state, SessionState::Working);

        // stop_hook_active → nikdy neblokovat (smyčka)
        let (code, resp) = post(port, "/event", &body("Stop", "b", json!({"stop_hook_active": true})), false);
        assert_eq!((code, resp.as_str()), (200, ""));
        assert_eq!(sess(&c, "b").uncommitted, vec!["b.txt"]);

        // ostatní eventy ani v block nic nevrací
        assert_eq!(post(port, "/event", &body("UserPromptSubmit", "b", json!({"prompt":"dál"})), false), (200, String::new()));

        // jen nepushnuté commity (větev má upstream) → „Máš nepushnuté své commity (1)“
        git(repo.path(), &["add", "b.txt"]);
        git(repo.path(), &["commit", "-q", "-m", "x"]);
        let (_, resp) = post(port, "/event", &body("Stop", "b", json!({"stop_hook_active": false})), false);
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert!(v["reason"].as_str().unwrap().starts_with("Máš nepushnuté své commity (1). Pushni je."));
        // po pushi čisto → prázdná 200
        git(repo.path(), &["push", "-q", "origin", "HEAD:main"]);
        assert_eq!(post(port, "/event", &body("Stop", "b", json!({"stop_hook_active": false})), false), (200, String::new()));
    }
}
