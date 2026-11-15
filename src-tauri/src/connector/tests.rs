//! Testy „Poslat do {název}“: plán (tabulky CZ), klient proti lokálnímu falešnému serveru (žádný
//! skutečný endpoint, žádný skutečný Správce pověření), odkazy zpět do poznámky, config.

use super::client::{self, Outcome};
use super::link;
use super::plan::{Assignee, Member, Mode, SENT_MARK, plan, request_id};
use super::{ConnectorConfig, url_ok, validate};
use serde_json::{Value, json};

fn team() -> Vec<Member> {
    [("tomas.hron@firma.cz", "Tomáš Hron"), ("jana.novakova@firma.cz", "Jana Nováková"), ("tomas.novak@firma.cz", "Tomáš Novák"), ("petr@firma.cz", "Petr Černý")]
        .iter()
        .map(|(e, n)| Member { email: e.to_string(), name: n.to_string() })
        .collect()
}

fn res(email: &str, name: &str) -> Assignee {
    Assignee::Resolved { email: email.into(), name: name.into() }
}

const P: &str = "C:\\n\\a.md";

// ---------- plán ----------

#[test]
fn note_without_checklist_is_one_note() {
    let p = plan(P, "Nápad", "Zavolat Řehořovi kvůli fakturám\n", &team(), None);
    assert_eq!((p.mode, p.has_tasks, p.items.len()), (Mode::Note, false, 0));
    assert_eq!(p.note_title, "Nápad");
    // hotové, navržené a body v kódu se nepočítají
    let p = plan(P, "x", "- [x] hotovo\n- [~] navrženo\n```\n- [ ] v kódu\n```\n", &team(), None);
    assert_eq!((p.mode, p.has_tasks), (Mode::Note, false));
}

#[test]
fn checklist_gives_one_task_per_open_item() {
    let body = "# Týden\n- [ ] Zavolat Řehořovi\n- [x] Hotovo\n- [ ] Poslat fakturu !\n* [ ] Třetí\n1. [ ] Čtvrtý\n";
    let p = plan(P, "Týden", body, &team(), None);
    assert_eq!(p.mode, Mode::Tasks);
    let got: Vec<_> = p.items.iter().map(|i| (i.line, i.title.as_str(), i.priority)).collect();
    assert_eq!(got, [(1, "Zavolat Řehořovi", None), (3, "Poslat fakturu", Some(4)), (4, "Třetí", None), (5, "Čtvrtý", None)]);
    // vynucený režim poznámky: žádné body
    let n = plan(P, "Týden", body, &team(), Some(Mode::Note));
    assert_eq!((n.mode, n.has_tasks, n.items.len()), (Mode::Note, true, 0));
    // vynucené úkoly bez bodů: prázdný seznam, ne pád
    assert!(plan(P, "x", "text", &team(), Some(Mode::Tasks)).items.is_empty());
}

#[test]
fn assignee_table() {
    let jh = res("tomas.hron@firma.cz", "Tomáš Hron");
    let cases: &[(&str, &str, Assignee)] = &[
        ("Zavolat Řehořovi, řešitel Tomáš Hron", "Zavolat Řehořovi", jh.clone()),
        ("řešitel Tomáš Hron zavolat Řehořovi", "zavolat Řehořovi", jh.clone()),
        ("Zavolat řešitel: tomas hron", "Zavolat", jh.clone()),
        ("Zavolat ŘEŠITEL TOMÁŠ HRON", "Zavolat", jh.clone()),
        ("Zavolat assignee Tomas Hron !", "Zavolat", jh.clone()),
        ("Zavolat @tomas.hron", "Zavolat", jh.clone()),
        ("Zavolat @tomashron", "Zavolat", jh.clone()),
        ("Zavolat @hron", "Zavolat", jh.clone()),
        ("řešitel je Petr Černý, zavolat", "zavolat", res("petr@firma.cz", "Petr Černý")),
        ("Zavolat řešitel Petr", "Zavolat", res("petr@firma.cz", "Petr Černý")),
        ("Zavolat řešitel Cerny", "Zavolat", res("petr@firma.cz", "Petr Černý")),
        // nejednoznačné: dva Tomáš — nehádá se, fráze zůstane v názvu
        ("Zavolat řešitel Tomáš", "Zavolat řešitel Tomáš", Assignee::Ambiguous { query: "Tomáš".into(), names: vec!["Tomáš Hron".into(), "Tomáš Novák".into()] }),
        ("Zavolat @tomas", "Zavolat @tomas", Assignee::Ambiguous { query: "tomas".into(), names: vec!["Tomáš Hron".into(), "Tomáš Novák".into()] }),
        // nenalezené
        ("Zavolat řešitel Pepa Zdepa", "Zavolat řešitel Pepa Zdepa", Assignee::Unknown { query: "Pepa Zdepa".into() }),
        ("Zavolat @nikdo", "Zavolat @nikdo", Assignee::Unknown { query: "nikdo".into() }),
        // předpona není shoda (celá slova): „Pet“ ≠ Petr
        ("Zavolat řešitel Pet", "Zavolat řešitel Pet", Assignee::Unknown { query: "Pet".into() }),
        // žádný řešitel; soubor `@src/…`, e-mail a slovo „řešitel“ bez jména nejsou zmínka
        ("Opravit @src/Header.tsx:42", "Opravit @src/Header.tsx:42", Assignee::Unset),
        ("Napsat na jan@firma.cz", "Napsat na jan@firma.cz", Assignee::Unset),
        ("Najít řešitel 42", "Najít řešitel 42", Assignee::Unset),
        ("Najít řešitele", "Najít řešitele", Assignee::Unset),
    ];
    for (text, title, want) in cases {
        let p = plan(P, "n", &format!("- [ ] {text}\n"), &team(), None);
        assert_eq!((p.items[0].title.as_str(), &p.items[0].assignee), (*title, want), "{text}");
    }
}

#[test]
fn note_level_assignee_is_default_but_item_wins() {
    let body = "Řešitel Jana Nováková\n\n- [ ] Jedna\n- [ ] Dvě, řešitel Petr Černý\n";
    let p = plan(P, "n", body, &team(), None);
    assert_eq!(p.items[0].assignee, res("jana.novakova@firma.cz", "Jana Nováková"));
    assert_eq!(p.items[1].assignee, res("petr@firma.cz", "Petr Černý"));
    assert_eq!(p.items[1].title, "Dvě");
    // bez členů (neúplný seznam) nic nehádá
    let p = plan(P, "n", "- [ ] Jedna, řešitel Tomáš Hron\n", &[], None);
    assert_eq!(p.items[0].assignee, Assignee::Unknown { query: "Tomáš Hron".into() });
}

#[test]
fn priority_table() {
    let cases: &[(&str, Option<u8>, &str)] = &[
        ("Zavolat !", Some(4), "Zavolat"),
        ("Zavolat !!", Some(5), "Zavolat"),
        ("! Zavolat", Some(4), "Zavolat"),
        ("Zavolat !2", Some(2), "Zavolat"),
        ("Zavolat !5,", Some(5), "Zavolat"),
        ("Zavolat !6", None, "Zavolat !6"),
        ("Zavolat !!!!", None, "Zavolat !!!!"),
        ("Pozor! Zavolat", None, "Pozor! Zavolat"),
        ("Zavolat https://a.cz/!x", None, "Zavolat https://a.cz/!x"),
        ("Zavolat 5 !", Some(4), "Zavolat 5"),
        ("Zavolat ! !!", Some(4), "Zavolat"),
    ];
    for (text, prio, title) in cases {
        let p = plan(P, "n", &format!("- [ ] {text}\n"), &team(), None);
        assert_eq!((p.items[0].priority, p.items[0].title.as_str()), (*prio, *title), "{text}");
    }
}

#[test]
fn request_ids_are_stable_and_distinct() {
    let a = request_id(P, 0, "Zavolat");
    assert_eq!(a, request_id(P, 0, "  Zavolat \n"));
    assert!(a.starts_with("ledge-") && (8..=64).contains(&a.len()), "{a}");
    assert_ne!(a, request_id(P, 1, "Zavolat"));
    assert_ne!(a, request_id(P, 0, "Zavolat jinak"));
    assert_ne!(a, request_id("C:\\n\\b.md", 0, "Zavolat"));
    // jiný tvar téže cesty (lomítka, prefix verbatim, velikost písmen) = stejná identita
    assert_eq!(a, request_id("c:/N/A.md", 0, "Zavolat"));
    assert_eq!(a, request_id("\\\\?\\C:\\n\\a.md", 0, "Zavolat"));
    assert!(!a.contains("hron") && !a.contains('\\'), "id nenese cestu");
    let p = plan(P, "n", "- [ ] A\n- [ ] A\n", &team(), None);
    assert_ne!(p.items[0].request_id, p.items[1].request_id, "stejný text podruhé = jiný úkol");
    assert_eq!(p.items[0].request_id, plan(P, "n", "- [ ] A\n- [ ] A\n", &team(), None).items[0].request_id);
}

#[test]
fn sent_marker_and_cap() {
    let body = "- [ ] Odeslaný\n  ![s](./a.png)\n  ↳ [Demo](https://x.cz/?tab=tasks&task=abc)\n- [ ] Nový\n".to_string();
    let p = plan(P, "n", &body, &team(), None);
    assert_eq!(p.items.iter().map(|i| i.sent).collect::<Vec<_>>(), [true, false]);
    let many: String = (0..25).map(|i| format!("- [ ] Úkol {i}\n")).collect();
    let p = plan(P, "n", &many, &team(), None);
    assert_eq!((p.items.len(), p.truncated), (20, true));
    let p = plan(P, "n", "\r\n- [ ] CRLF\r\n- [ ] Dva\r\n", &team(), None);
    assert_eq!(p.items.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), ["CRLF", "Dva"]);
}

// ---------- klient: čisté části ----------

#[test]
fn request_body_is_jsonrpc_tools_call() {
    let v: Value = serde_json::from_str(&client::request_body(7, "create_task", &json!({ "title": "A" }))).unwrap();
    assert_eq!(v, json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "name": "create_task", "arguments": { "title": "A" } } }));
}

fn ok_reply(text: &str) -> Vec<u8> {
    json!({ "jsonrpc": "2.0", "id": 1, "result": { "content": [{ "type": "text", "text": text }] } }).to_string().into_bytes()
}

#[test]
fn parse_reply_table() {
    const K: &str = "klic_TAJNYKLIC0123456789";
    let r = client::parse_reply(200, &ok_reply(r#"{"ok":true,"taskId":"t1","url":"?tab=tasks&task=t1"}"#), K).unwrap();
    assert_eq!(r["taskId"], "t1");
    // text, který není JSON, zůstane řetězcem
    assert_eq!(client::parse_reply(200, &ok_reply("hotovo"), K).unwrap(), json!("hotovo"));
    // SSE rámec
    let sse = format!("event: message\ndata: {}\n\n", String::from_utf8(ok_reply(r#"{"ok":true}"#)).unwrap());
    assert_eq!(client::parse_reply(200, sse.as_bytes(), K).unwrap()["ok"], true);
    // chyba nástroje: česká hláška serveru, klíč se vymaže
    let e = json!({ "result": { "isError": true, "content": [{ "type": "text", "text": format!("Neplatný vstup: assignee klic_TAJNYKLIC0123456789 není člen.") }] } }).to_string();
    let err = client::parse_reply(200, e.as_bytes(), K).unwrap_err();
    assert!(err.contains("Neplatný vstup") && !err.contains("TAJNY"), "{err}");
    // JSON-RPC chyba
    let e = json!({ "jsonrpc": "2.0", "error": { "code": -32000, "message": "Požadavek je příliš velký." }, "id": null }).to_string();
    assert_eq!(client::parse_reply(413, e.as_bytes(), K).unwrap_err(), "Požadavek je příliš velký.");
    // stavy HTTP lidsky, bez kódů a adres
    for (code, frag) in [(401, "odmítla klíč"), (403, "oprávnění"), (429, "limit"), (503, "potíže")] {
        let e = client::parse_reply(code, b"{}", K).unwrap_err();
        assert!(e.contains(frag) && !e.contains(&code.to_string()) && !e.contains("://"), "{code}: {e}");
    }
    // smetí a HTML místo JSON
    assert!(client::parse_reply(200, b"<html>", K).unwrap_err().contains("nečekaně"));
    assert!(client::parse_reply(200, b"{}", K).is_err());
    // dlouhá chyba se ořízne
    let long = json!({ "result": { "isError": true, "content": [{ "type": "text", "text": "x".repeat(5000) }] } }).to_string();
    assert!(client::parse_reply(200, long.as_bytes(), K).unwrap_err().chars().count() <= 300);
}

#[test]
fn task_and_note_args() {
    let p = plan(P, "Týden", "- [ ] Zavolat, řešitel Tomáš Hron !2\n- [ ] Druhý řešitel Tomáš\n- [ ] Třetí\n", &team(), None);
    let a = client::task_args(&p.items[0], &p.note_title);
    assert_eq!((a["title"].as_str(), a["assignee"].as_str(), a["priority"].as_u64()), (Some("Zavolat"), Some("tomas.hron@firma.cz"), Some(2)));
    assert_eq!(a["clientRequestId"], p.items[0].request_id.as_str());
    assert!(a["description"].as_str().unwrap().contains("Týden"));
    // nejednoznačný řešitel se neposílá; bez priority se pole vynechá
    let a = client::task_args(&p.items[1], "x");
    assert!(a.get("assignee").is_none() && a.get("priority").is_none(), "{a}");
    let a = client::task_args(&p.items[2], "x");
    assert!(a.get("assignee").is_none());
    let long = plan(P, "n", &format!("- [ ] {}\n", "ž".repeat(900)), &team(), None);
    assert_eq!(client::task_args(&long.items[0], "n")["title"].as_str().unwrap().chars().count(), 500);
    let n = client::note_args(&plan(P, "Nápad", "text", &team(), None), "text");
    assert_eq!((n["title"].as_str(), n["bodyMarkdown"].as_str()), (Some("Nápad"), Some("text")));
    assert!(n["clientRequestId"].as_str().unwrap().len() <= 64);
}

#[test]
fn members_and_links() {
    let v = json!({ "total": 3, "members": [{ "email": "a@x.cz", "name": "A B", "roles": [] }, { "email": "", "name": "Bez" }, { "name": "Bez e-mailu" }, { "email": "c@x.cz" }] });
    let m = client::members_of(&v);
    assert_eq!(m.iter().map(|m| m.email.as_str()).collect::<Vec<_>>(), ["a@x.cz", "c@x.cz"]);
    assert!(client::members_of(&json!("nic")).is_empty());
    let b = "https://example.test";
    assert_eq!(client::task_link(b, "?tab=tasks&task=t1").as_deref(), Some("https://example.test/?tab=tasks&task=t1"));
    assert_eq!(client::task_link("https://example.test/", "?tab=tasks&task=t1").as_deref(), Some("https://example.test/?tab=tasks&task=t1"));
    for bad in ["javascript:alert(1)", "https://evil.cz/", "?x=1)[a](b", "?a b", "task=1", ""] {
        assert_eq!(client::task_link(b, bad), None, "{bad}");
    }
    assert_eq!(client::task_link("ftp://x", "?a=1"), None);
}

// ---------- klient: falešný server ----------

type Seen = std::sync::Arc<parking_lot::Mutex<Vec<(String, String, Value)>>>;

/// Lokální falešný MCP server: odpoví na `list_team_members`, `create_task`, `create_note`;
/// `create_task` s titulkem „Chyba“ vrátí chybu nástroje, s `clientRequestId` už viděným = `deduplicated`.
fn fake_server(status: u16) -> (String, Seen) {
    let srv = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://127.0.0.1:{}/mcpServer", srv.server_addr().to_ip().unwrap().port());
    let seen: Seen = Default::default();
    let log = seen.clone();
    std::thread::spawn(move || {
        let mut ids = std::collections::HashSet::new();
        for mut rq in srv.incoming_requests() {
            let mut body = String::new();
            let _ = std::io::Read::read_to_string(rq.as_reader(), &mut body);
            let auth = rq.headers().iter().find(|h| h.field.equiv("Authorization")).map(|h| h.value.to_string()).unwrap_or_default();
            let accept = rq.headers().iter().find(|h| h.field.equiv("Accept")).map(|h| h.value.to_string()).unwrap_or_default();
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            log.lock().push((auth, accept, v.clone()));
            let tool = v["params"]["name"].as_str().unwrap_or("").to_string();
            let args = &v["params"]["arguments"];
            let payload = match tool.as_str() {
                "list_team_members" => json!({ "members": [{ "email": "tomas.hron@firma.cz", "name": "Tomáš Hron" }] }),
                "create_task" if args["title"] == "Chyba" => json!({ "__err": "Zápis odmítnut: nemáš právo." }),
                "create_task" => {
                    let id = args["clientRequestId"].as_str().unwrap_or("").to_string();
                    let dup = !ids.insert(id.clone());
                    json!({ "ok": true, "taskId": format!("t_{}", &id[id.len().saturating_sub(6)..]), "url": "?tab=tasks&task=t1", "deduplicated": dup })
                }
                "create_note" => json!({ "ok": true, "noteId": "n1" }),
                _ => json!({}),
            };
            let result = match payload.get("__err") {
                Some(e) => json!({ "isError": true, "content": [{ "type": "text", "text": e }] }),
                None => json!({ "content": [{ "type": "text", "text": payload.to_string() }] }),
            };
            let out = json!({ "jsonrpc": "2.0", "id": v["id"], "result": result }).to_string();
            let resp = tiny_http::Response::from_string(out).with_status_code(status).with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap());
            let _ = rq.respond(resp);
        }
    });
    (url, seen)
}

#[test]
fn send_against_fake_server_with_idempotency() {
    const K: &str = "klic_0123456789abcdef0123456789abcdef";
    let (url, seen) = fake_server(200);
    let app = "https://example.test";
    tauri::async_runtime::block_on(async {
        let members = client::list_members(&url, K).await.unwrap();
        assert_eq!(members.len(), 1);
        let body = "- [ ] Zavolat, řešitel Tomáš Hron !\n- [ ] Chyba\n- [ ] Třetí\n";
        let p = plan(P, "Týden", body, &members, None);
        let out = client::send(&url, K, &p, body, &[0, 1, 2], app).await;
        assert_eq!(out.iter().map(|o| (o.line, o.ok)).collect::<Vec<_>>(), [(0, true), (1, false), (2, true)]);
        assert_eq!(out[0].url.as_deref(), Some("https://example.test/?tab=tasks&task=t1"));
        assert!(out[1].error.as_deref().unwrap().contains("nemáš právo"));
        // opakovaný klik: stejné clientRequestId → server hlásí duplicitu, nic se nezdvojí
        let again = client::send(&url, K, &p, body, &[0], app).await;
        assert!(again[0].ok && again[0].duplicate, "{again:?}");
        // vybrané řádky a už odeslané body se neposílají
        let none = client::send(&url, K, &p, body, &[], app).await;
        assert!(none.is_empty());
        // poznámka
        let pn = plan(P, "Nápad", "Jen text", &members, None);
        let o = client::send(&url, K, &pn, "Jen text", &[], app).await;
        assert_eq!((o[0].ok, o[0].id.as_deref()), (true, Some("n1")));
    });
    let seen = seen.lock();
    assert!(seen.iter().all(|(a, acc, _)| a == &format!("Bearer {K}") && acc.contains("application/json") && acc.contains("text/event-stream")));
    let tools: Vec<&str> = seen.iter().map(|(_, _, v)| v["params"]["name"].as_str().unwrap()).collect();
    assert_eq!(tools, ["list_team_members", "create_task", "create_task", "create_task", "create_task", "create_note"]);
    assert_eq!(seen[1].2["params"]["arguments"]["assignee"], "tomas.hron@firma.cz");
    assert_eq!(seen[1].2["params"]["arguments"]["priority"], 4);
}

#[test]
fn auth_error_stops_the_loop_and_hides_key() {
    const K: &str = "klic_0123456789abcdef0123456789abcdef";
    let (url, seen) = fake_server(401);
    let app = "https://example.test";
    tauri::async_runtime::block_on(async {
        let body = "- [ ] A\n- [ ] B\n- [ ] C\n";
        let p = plan(P, "n", body, &[], None);
        let out = client::send(&url, K, &p, body, &[0, 1, 2], app).await;
        assert_eq!(out.len(), 1, "po 401 se nepokračuje");
        let e = out[0].error.as_deref().unwrap();
        assert!(!e.contains(K) && !e.contains("127.0.0.1") && e.contains("odmítla klíč"), "{e}");
        assert!(client::list_members(&url, K).await.is_err());
        // neplatná adresa se neodešle vůbec
        let bad = client::call("http://evil.example/mcp", K, "x", &json!({})).await.unwrap_err();
        assert!(bad.contains("connector.url"));
    });
    assert_eq!(seen.lock().len(), 2);
}

// ---------- odkazy zpět do poznámky ----------

fn done(line: usize, url: Option<&str>) -> Outcome {
    Outcome { line, ok: true, id: Some("t1".into()), url: url.map(str::to_string), error: None, duplicate: false, text: String::new() }
}

#[test]
fn links_go_under_item_and_its_images_valid_markdown() {
    let body = "# Týden\n- [ ] První\n  ![s](./a.png)\n- [ ] Druhý\n  1. [ ] vnořený\n1. [ ] Číslovaný\n";
    let u = "https://example.test/?tab=tasks&task=t1";
    let (b, n, _) = link::add_links(body, &[done(1, Some(u)), done(3, None), done(5, Some(u))], "Demo");
    assert_eq!(n, 3);
    assert_eq!(
        b,
        format!("# Týden\n- [ ] První\n  ![s](./a.png)\n  ↳ [Demo]({u})\n- [ ] Druhý\n  ↳ [Demo] t1\n  1. [ ] vnořený\n1. [ ] Číslovaný\n   ↳ [Demo]({u})\n")
    );
    // po zápisu je bod `sent` a obrázek zůstal u bodu
    let p = plan(P, "n", &b, &team(), None);
    assert_eq!(p.items.iter().map(|i| i.sent).collect::<Vec<_>>(), [true, true, false, true]);
    // druhé doplnění nic nepřidá; neúspěch, mizející řádek a CRLF
    assert_eq!(link::add_links(&b, &[done(1, Some(u))], "Demo").1, 0);
    let mut bad = done(1, None);
    bad.ok = false;
    assert_eq!(link::add_links(body, &[bad, done(0, None), done(99, None)], "Demo").1, 0);
    let (c, _, _) = link::add_links("- [ ] A\r\n- [ ] B\r\n", &[done(0, None)], "Demo");
    assert_eq!(c, "- [ ] A\r\n  ↳ [Demo] t1\r\n- [ ] B\r\n");
    // poslední řádek bez koncového \n
    assert_eq!(link::add_links("- [ ] A", &[done(0, None)], "Demo").0, "- [ ] A\n  ↳ [Demo] t1");
    // odkaz se v náhledu vykreslí jako https odkaz
    let html = crate::notes::md::to_html(&b);
    assert!(html.contains("href=\"https://example.test/?tab=tasks&amp;task=t1\""), "{html}");
}

#[test]
fn request_id_survives_moving_item_and_snooze() {
    let before = plan(P, "n", "- [ ] Zavolat ⏰ 2026-11-20\n- [ ] Jiné\n", &team(), None);
    // nad bod přibyly řádky, bod se posunul a termín se odložil
    let after = plan(P, "n", "# Nadpis\n\npřidáno\n- [ ] Jiné\n- [ ] Zavolat ⏰ 2026-11-27\n", &team(), None);
    let id = |p: &super::plan::Plan, t: &str| p.items.iter().find(|i| i.title.starts_with(t)).unwrap().request_id.clone();
    assert_eq!(id(&before, "Zavolat"), id(&after, "Zavolat"));
    assert_eq!(id(&before, "Jiné"), id(&after, "Jiné"));
}

#[test]
fn links_follow_item_text_when_lines_shifted_and_never_hit_another_item() {
    // plán byl z těla, kde body byly na řádcích 0 a 1; mezitím přibyl řádek nahoře
    let body = "poznámka navíc\n- [ ] První\n- [ ] Druhý\n";
    let mut first = done(0, Some("https://x.cz/?task=a"));
    first.text = "První".into();
    let mut second = done(1, Some("https://x.cz/?task=b"));
    second.text = "Druhý".into();
    let (b, n, missed) = link::add_links(body, &[first, second], "Demo");
    assert_eq!((n, missed.len()), (2, 0));
    assert_eq!(b, "poznámka navíc\n- [ ] První\n  ↳ [Demo](https://x.cz/?task=a)\n- [ ] Druhý\n  ↳ [Demo](https://x.cz/?task=b)\n");
    // bod mezitím zmizel / změnil text: nezapíše se na jiný bod, vrátí se jako nenalezený
    let mut gone = done(1, Some("https://x.cz/?task=c"));
    gone.text = "Smazaný".into();
    let (b2, n2, missed2) = link::add_links(body, &[gone], "Demo");
    assert_eq!((b2.as_str(), n2, missed2), (body, 0, vec![1]));
    // termín `⏰` odložený po odeslání bod nezmění
    let mut snoozed = done(0, None);
    snoozed.text = "Zavolat".into();
    assert_eq!(link::add_links("- [ ] Zavolat ⏰ 2026-12-01\n", &[snoozed], "Demo").1, 1);
    // dva stejné texty: rozhodne řádek, při nesedícím řádku se nehádá
    let dup = "- [ ] A\n- [ ] A\n";
    let mut d = done(1, None);
    d.text = "A".into();
    let (b3, n3, _) = link::add_links(dup, &[d.clone()], "Demo");
    assert_eq!((b3, n3), ("- [ ] A\n- [ ] A\n  ↳ [Demo] t1\n".to_string(), 1));
    d.line = 7;
    assert_eq!(link::add_links(dup, &[d], "Demo").2, vec![7]);
}

#[test]
fn connector_id_from_frontmatter() {
    assert_eq!(link::connector_id("---\ncreated: x\nconnector: n1\n---\n\ntext").as_deref(), Some("n1"));
    assert_eq!(link::connector_id("---\ncreated: x\n---\n\nconnector: n1"), None);
    assert_eq!(link::connector_id("bez frontmatteru"), None);
}

// ---------- config ----------

#[test]
fn old_config_gets_empty_defaults_and_validates() {
    let cfg: crate::config::Config = serde_json::from_str(r#"{"claude":{"enabled":true,"port":47800,"approvalTimeoutS":45}}"#).unwrap();
    assert_eq!(cfg.connector, ConnectorConfig::default());
    assert_eq!((cfg.connector.endpoint(), cfg.connector.app_base()), (None, None));
    assert!((cfg.connector.url.as_str(), cfg.connector.app_url.as_str(), cfg.connector.name.as_str()) == ("", "", ""));
    assert!(!cfg.connector.configured());
    assert!(crate::config::validate(&cfg).is_ok());
    let cfg: crate::config::Config = serde_json::from_str(r#"{"connector":{"url":"  ","appUrl":" / "}}"#).unwrap();
    assert_eq!((cfg.connector.endpoint(), cfg.connector.app_base(), cfg.connector.configured()), (None, None, false));
    let cfg: crate::config::Config = serde_json::from_str(r#"{"connector":{"url":"http://127.0.0.1:5001/x/mcp","appUrl":"https://staging.example.test/","name":"  Demo  "}}"#).unwrap();
    assert_eq!((cfg.connector.endpoint(), cfg.connector.app_base()), (Some("http://127.0.0.1:5001/x/mcp"), Some("https://staging.example.test")));
    assert_eq!((cfg.connector.display_name().as_str(), cfg.connector.configured()), ("Demo", true));
    assert!(crate::config::validate(&cfg).is_ok());
    let mut bad = cfg.clone();
    bad.connector.url = "http://evil.example/mcp".into();
    assert!(crate::config::validate(&bad).unwrap_err().contains("connector.url"));
    assert!(crate::config::schema_json().contains("appUrl"));
}

/// Starý config se sekcí pod jiným (dnes neznámým) klíčem se načte bez chyby a nová sekce je prázdná.
#[test]
fn config_with_unknown_legacy_section_loads() {
    let cfg: crate::config::Config = serde_json::from_str(r#"{"legacyThing":{"url":"https://example.test/mcp","appUrl":"https://example.test"},"claude":{"enabled":true}}"#).unwrap();
    assert_eq!(cfg.connector, ConnectorConfig::default());
    assert!(crate::config::validate(&cfg).is_ok());
}

#[test]
fn display_name_table() {
    for (raw, want) in [("", ""), ("  Demo ", "Demo"), ("A\nB\tC", "A B C"), ("[x]", "[x]"), (&"é".repeat(80), &"é".repeat(60))] {
        let c = ConnectorConfig { name: raw.to_string(), ..Default::default() };
        assert_eq!(c.display_name(), want, "{raw:?}");
    }
}

#[test]
fn url_table() {
    for (u, ok) in [
        ("", true),
        ("https://mcp.example.test/v1/mcp", true),
        ("https://host:8443/a?b=1", true),
        ("http://127.0.0.1:5001/p/mcp", true),
        ("http://localhost/x", true),
        ("http://[::1]:5001/x", true),
        ("http://example.test/x", false),
        ("http://127.0.0.1.evil.cz/x", false),
        ("https://user:pw@host/x", false),
        ("https://", false),
        ("ftp://host", false),
        ("https://host/a b", false),
        ("javascript:alert(1)", false),
        ("host.cz/mcp", false),
    ] {
        assert_eq!(url_ok(u), ok, "{u}");
    }
    assert!(validate(&ConnectorConfig::default()).is_ok());
}

// ---------- čistý rozbor řádku (bez členů týmu) ----------

#[test]
fn parse_line_table() {
    use super::plan::{LineParts, Mention, parse_line};
    let w = |s: &str| s.split_whitespace().map(str::to_string).collect::<Vec<_>>();
    let m = |at, skip, cand: &[&str], at_sign| Some(Mention { at, skip, cand: cand.iter().map(|c| c.to_string()).collect(), at_sign });
    let cases: Vec<(&str, LineParts)> = vec![
        ("Zavolat", LineParts { words: w("Zavolat"), prio_words: vec![], priority: None, mention: None }),
        ("Zavolat !!", LineParts { words: w("Zavolat !!"), prio_words: vec![1], priority: Some(5), mention: None }),
        ("! Zavolat !2", LineParts { words: w("! Zavolat !2"), prio_words: vec![0, 2], priority: Some(4), mention: None }),
        ("Pozor! Zavolat", LineParts { words: w("Pozor! Zavolat"), prio_words: vec![], priority: None, mention: None }),
        ("Zavolat, řešitel Tomáš Hron", LineParts { words: w("Zavolat, řešitel Tomáš Hron"), prio_words: vec![], priority: None, mention: m(1, 0, &["Tomáš", "Hron"], false) }),
        ("řešitel je Petr, zavolat", LineParts { words: w("řešitel je Petr, zavolat"), prio_words: vec![], priority: None, mention: m(0, 1, &["Petr"], false) }),
        ("Zavolat @tomas !", LineParts { words: w("Zavolat @tomas !"), prio_words: vec![2], priority: Some(4), mention: m(1, 0, &["tomas"], true) }),
        ("Opravit @src/H.tsx:4 řešitel 42", LineParts { words: w("Opravit @src/H.tsx:4 řešitel 42"), prio_words: vec![], priority: None, mention: None }),
    ];
    for (text, want) in cases {
        assert_eq!(parse_line(text), want, "{text}");
    }
}

// ---------- značka odeslaného bodu (SENT_MARK) ----------

#[derive(serde::Deserialize)]
struct MarkBody {
    body: String,
    sent: Vec<bool>,
}

#[derive(serde::Deserialize)]
struct MarkCases {
    marked: Vec<String>,
    plain: Vec<String>,
    bodies: Vec<MarkBody>,
}

fn mark_cases() -> MarkCases {
    serde_json::from_str(include_str!("../notes/sent_mark_cases.json")).unwrap()
}

#[test]
fn sent_mark_table_shared_with_ts() {
    let c = mark_cases();
    for l in &c.marked {
        assert!(l.starts_with(SENT_MARK), "{l}");
    }
    for l in &c.plain {
        assert!(!l.starts_with(SENT_MARK), "{l}");
    }
    for (i, b) in c.bodies.iter().enumerate() {
        let p = plan(P, "n", &b.body, &team(), None);
        assert_eq!(p.items.iter().map(|x| x.sent).collect::<Vec<_>>(), b.sent, "plan #{i}: {:?}", b.body);
        let ix = crate::notes::taskindex::index(&b.body);
        assert_eq!(ix.items.iter().map(|x| x.forwarded).collect::<Vec<_>>(), b.sent, "index #{i}: {:?}", b.body);
    }
}

/// Co vyrobí `link_line`, to plán i index úkolů poznají jako odeslané (pro libovolný název).
#[test]
fn written_link_line_is_recognized_as_sent() {
    let with = |url: Option<&str>, id: Option<&str>| Outcome { line: 0, ok: true, id: id.map(str::to_string), url: url.map(str::to_string), error: None, duplicate: false, text: String::new() };
    for name in ["Demo", "", "  ", "[Dem]o", "Česká aplikace", "A\nB"] {
        for o in [with(Some("https://example.test/x"), Some("t1")), with(None, Some("t1")), with(None, None)] {
            let line = link::link_line(&o, name);
            assert!(line.starts_with(SENT_MARK) && !line.contains('\n'), "{line:?}");
            let body = format!("- [ ] A\n  {line}\n- [ ] B\n");
            let p = plan(P, "n", &body, &team(), None);
            assert_eq!(p.items.iter().map(|x| x.sent).collect::<Vec<_>>(), [true, false], "{line:?}");
            let ix = crate::notes::taskindex::index(&body);
            assert_eq!(ix.items.iter().map(|x| x.forwarded).collect::<Vec<_>>(), [true, false], "{line:?}");
            // znovu se nepřidá
            assert_eq!(link::add_links(&body, &[done(0, Some("https://example.test/y"))], name).1, 0, "{line:?}");
        }
    }
}

#[test]
fn link_line_shape_table() {
    let o = |url: Option<&str>, id: Option<&str>| Outcome { line: 0, ok: true, id: id.map(str::to_string), url: url.map(str::to_string), error: None, duplicate: false, text: String::new() };
    let u = Some("https://example.test/x");
    for (out, name, want) in [
        (o(u, Some("t")), "Demo", "↳ [Demo](https://example.test/x)"),
        (o(u, None), "", "↳ [Úkol](https://example.test/x)"),
        (o(None, Some("t1")), "Demo", "↳ [Demo] t1"),
        (o(None, Some("t1")), " [ ] ", "↳ [Úkol] t1"),
        (o(None, None), "Demo", "↳ [Demo]"),
        (o(None, None), "A[b]c", "↳ [Abc]"),
    ] {
        assert_eq!(link::link_line(&out, name), want);
    }
}
