//! Testy MCP nad `MemHost` (config v paměti, klíč v `MemVault`): HTTP úroveň (vypnuto, klíč, metoda,
//! velikost, Origin přes skutečný most), katalog nástrojů a každý zápisový nástroj (platný → změna +
//! Vrátit; neplatný → chyba a config beze změny). Nic nesahá na `%APPDATA%\Ledge`, registr ani
//! Správce pověření.

use super::*;
use crate::accounts::vault::MemVault;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MemHost {
    pub cfg: Mutex<Config>,
    pub saves: AtomicUsize,
    /// kořen poznámek (dočasná složka testu); `None` = poznámky vypnuté
    pub notes: Mutex<Option<std::path::PathBuf>>,
    /// kolikrát se po zápisu do poznámek volalo `notes_changed`
    pub note_writes: AtomicUsize,
    /// (cesta, připnout?) z `pin_note`
    pub pins: Mutex<Vec<(std::path::PathBuf, bool)>>,
}

impl MemHost {
    pub fn new(enabled: bool) -> Self {
        let mut cfg = Config::default();
        cfg.mcp.enabled = enabled;
        Self { cfg: Mutex::new(cfg), saves: AtomicUsize::new(0), notes: Mutex::new(None), note_writes: AtomicUsize::new(0), pins: Mutex::new(Vec::new()) }
    }
}

impl Host for MemHost {
    fn config(&self) -> Config {
        self.cfg.lock().clone()
    }
    fn save(&self, cfg: Config) -> Result<(), String> {
        crate::config::validate(&cfg)?;
        let mut c = cfg;
        crate::layout::derive(&mut c);
        *self.cfg.lock() = c;
        self.saves.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn version(&self) -> String {
        "0.0.0-test".into()
    }
    fn drawer_open(&self) -> bool {
        false
    }
    fn open_drawer(&self, tab: Option<&str>) -> Result<String, String> {
        Ok(format!("open {}", tab.unwrap_or("")))
    }
    fn notes_root(&self) -> Option<std::path::PathBuf> {
        self.notes.lock().clone()
    }
    fn notes_changed(&self) {
        self.note_writes.fetch_add(1, Ordering::SeqCst);
    }
    fn pin_note(&self, path: &std::path::Path, pin: bool) -> Result<String, String> {
        self.pins.lock().push((path.to_path_buf(), pin));
        Ok(format!("pin {pin}"))
    }
    fn discover(&self, _repos: bool) -> Value {
        json!({ "apps": [{ "name": "Poznámkový blok", "exe": "C:\\Windows\\notepad.exe", "alreadyInLedge": false }], "repos": [] })
    }
}

pub fn server() -> Server {
    Server::new(Box::new(MemVault::default()))
}

fn req<'a>(method: &'a str, auth: Option<&'a str>, body: &'a [u8]) -> HttpReq<'a> {
    HttpReq { method, authorization: auth, protocol: None, body: Some(body) }
}

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#;

#[test]
fn http_gate_order() {
    let srv = server();
    let key = srv.keys.ensure().unwrap();
    let auth = format!("Bearer {key}");
    let off = MemHost::new(false);
    assert_eq!(handle_http(&srv, &off, &req("POST", Some(&auth), INIT.as_bytes())).code, 404, "vypnuto = 404 i s klíčem");
    let on = MemHost::new(true);
    assert_eq!(handle_http(&srv, &on, &req("POST", None, INIT.as_bytes())).code, 401);
    let bad = handle_http(&srv, &on, &req("POST", Some("Bearer spatny"), INIT.as_bytes()));
    assert_eq!((bad.code, bad.body.as_str()), (401, ""), "401 bez detailu");
    assert_eq!(handle_http(&srv, &on, &req("GET", Some(&auth), b"")).header, Some(("Allow", "POST")));
    assert_eq!(handle_http(&srv, &on, &req("DELETE", Some(&auth), b"")).code, 405);
    assert_eq!(handle_http(&srv, &on, &HttpReq { method: "POST", authorization: Some(&auth), protocol: None, body: None }).code, 413);
    assert_eq!(handle_http(&srv, &on, &HttpReq { method: "POST", authorization: Some(&auth), protocol: Some("1999-01-01"), body: Some(INIT.as_bytes()) }).code, 400);
    assert_eq!(handle_http(&srv, &on, &HttpReq { method: "POST", authorization: Some(&auth), protocol: Some("2025-03-26"), body: Some(INIT.as_bytes()) }).code, 200);
    let ok = handle_http(&srv, &on, &req("POST", Some(&auth), INIT.as_bytes()));
    assert_eq!(ok.code, 200);
    assert!(ok.body.contains("\"protocolVersion\":\"2025-06-18\""), "{}", ok.body);
    assert_eq!(handle_http(&srv, &on, &req("POST", Some(&auth), br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)).code, 202);
    assert_eq!(handle_http(&srv, &on, &req("POST", Some(&auth), b"{nejson")).code, 400);
    assert_eq!(handle_http(&srv, &on, &req("POST", Some(&auth), &[0xff, 0xfe])).code, 400);
    // bez klíče v úložišti nepustí nikoho
    let empty = server();
    assert_eq!(handle_http(&empty, &on, &req("POST", Some("Bearer x"), INIT.as_bytes())).code, 401);
}

/// Skutečný most (`claude::serve`) s MCP endpointem: Origin, Host, klíč, vypnuto, velikost — po síti.
#[test]
fn over_real_bridge() {
    let srv = Arc::new(server());
    let key = srv.keys.ensure().unwrap();
    let host = Arc::new(MemHost::new(true));
    let (port, ()) = crate::ports::with_free_port(|port| {
        let (s2, h2) = (srv.clone(), host.clone());
        let claude = crate::claude::Claude::new(Arc::new(|| {}), std::time::Duration::from_secs(1));
        crate::claude::serve(port, claude, Arc::new(|| json!({})), Arc::new(|_, _| Ok(json!({}))), Arc::new(move |r: &HttpReq| handle_http(&s2, h2.as_ref(), r)))
    });
    let post = |extra: &str, body: &str| {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        (out[9..12].parse::<u16>().unwrap(), out.split("\r\n\r\n").nth(1).unwrap_or("").to_string())
    };
    let auth = format!("Authorization: Bearer {key}\r\n");
    assert_eq!(post("", INIT).0, 401, "bez klíče");
    assert_eq!(post("Authorization: Bearer nic\r\n", INIT).0, 401, "špatný klíč");
    assert_eq!(post(&format!("{auth}Origin: http://evil.example\r\n"), INIT).0, 403, "prohlížeč ne, ani s klíčem");
    let (code, body) = post(&auth, INIT);
    assert_eq!(code, 200);
    assert!(body.contains("\"serverInfo\""), "{body}");
    let (code, body) = post(&auth, r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ledge_status"}}"#);
    assert_eq!(code, 200);
    assert!(body.contains("\\\"tiles\\\""), "{body}");
    let big = format!(r#"{{"jsonrpc":"2.0","id":3,"method":"ping","params":{{"x":"{}"}}}}"#, "a".repeat(MAX_BODY));
    assert_eq!(post(&auth, &big).0, 413);
    host.cfg.lock().mcp.enabled = false;
    assert_eq!(post(&auth, INIT).0, 404, "vypnuto");
}

// ---------- nástroje ----------

fn call(srv: &Server, host: &MemHost, name: &str, args: Value) -> (bool, String) {
    let h = tools::Handler { srv, host };
    let r = <tools::Handler as rpc::Tools>::call(&h, name, &args).unwrap();
    (r["isError"].as_bool().unwrap(), r["content"][0]["text"].as_str().unwrap().to_string())
}

#[test]
fn tools_list_matches_contract() {
    let srv = server();
    let host = MemHost::new(true);
    let list = <tools::Handler as rpc::Tools>::list(&tools::Handler { srv: &srv, host: &host });
    let names: Vec<&str> = list.as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    let contract = [
        "ledge_status", "apps_discover", "tiles_list", "tiles_add", "tiles_update", "tiles_remove", "actions_list", "actions_add", "actions_update",
        "actions_remove", "layout_get", "layout_preview", "layout_apply", "strip_get", "strip_set", "appearance_get", "appearance_set",
        "notes_list", "notes_read", "notes_create", "notes_append", "notes_toggle_task", "notes_pin", "undo_last", "open_drawer",
    ];
    assert_eq!(names, contract);
    for t in list.as_array().unwrap() {
        assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
        let d = t["description"].as_str().unwrap();
        assert!(d.len() > 60 && !d.chars().any(|c| "ěščřžýáíéůúňťď".contains(c)), "anglický popis pro model: {}", t["name"]);
        // každý `$ref` ve schématu míří do `$defs` téhož schématu
        let s = t["inputSchema"].to_string();
        for r in s.split("\"$ref\":\"#/").skip(1) {
            let path = r.split('"').next().unwrap();
            assert!(t["inputSchema"].pointer(&format!("/{path}")).is_some(), "{}: $ref {path}", t["name"]);
        }
    }
    let r = <tools::Handler as rpc::Tools>::call(&tools::Handler { srv: &srv, host: &host }, "launch", &json!({}));
    assert_eq!(r.unwrap_err().0, rpc::INVALID_PARAMS, "neznámý nástroj = chyba protokolu");
    assert!(!names.iter().any(|n| n.contains("launch") || n.contains("run")), "MCP nic nespouští");
}

/// `tools/list` se posílá do kontextu modelu při každém připojení: strop 20 kB (před úpravou ~55 kB).
#[test]
fn tools_list_stays_small() {
    let srv = server();
    let host = MemHost::new(true);
    let list = <tools::Handler as rpc::Tools>::list(&tools::Handler { srv: &srv, host: &host });
    let size = serde_json::to_string(&list).unwrap().len();
    let per: Vec<String> = list.as_array().unwrap().iter().map(|t| format!("{} {}", t["name"].as_str().unwrap(), t.to_string().len())).collect();
    eprintln!("tools/list: {size} B ({})", per.join(", "));
    assert!(size < 20_000, "tools/list má {size} B (strop 20000) — zkrať popisy nebo schémata; po nástrojích: {}", per.join(", "));
}

/// Výstupy jsou kompaktní JSON na jednom řádku a čísla `f32` bez šumu (`1.05`, ne `1.0499999523162842`).
#[test]
fn outputs_are_compact_and_rounded() {
    let srv = server();
    let host = MemHost::new(true);
    host.cfg.lock().appearance.glass.opacity = 1.05_f32;
    host.cfg.lock().appearance.motion.speed = 0.7_f32;
    for t in ["ledge_status", "tiles_list", "actions_list", "layout_get", "strip_get", "appearance_get", "apps_discover"] {
        let (err, text) = call(&srv, &host, t, json!({}));
        assert!(!err && !text.contains('\n') &&!text.contains(": "), "{t}: ne kompaktní: {}", &text[..text.len().min(120)]);
    }
    let (_, a) = call(&srv, &host, "appearance_get", json!({}));
    let v: Value = serde_json::from_str(&a).unwrap();
    assert!(a.contains("\"opacity\":1.05") && a.contains("\"speed\":0.7"), "{a}");
    assert!(!a.contains("999") && !a.contains("000000"), "šum f32: {a}");
    assert!(v.is_object());
}

#[test]
fn read_tools() {
    let srv = server();
    let host = MemHost::new(true);
    for t in ["ledge_status", "apps_discover", "tiles_list", "actions_list", "layout_get", "strip_get", "appearance_get", "open_drawer"] {
        let (err, text) = call(&srv, &host, t, json!({}));
        assert!(!err, "{t}: {text}");
    }
    let (_, st) = call(&srv, &host, "ledge_status", json!({}));
    let v: Value = serde_json::from_str(&st).unwrap();
    assert_eq!((v["version"].as_str(), v["mcp"]["enabled"].as_bool()), (Some("0.0.0-test"), Some(true)));
    let (_, a) = call(&srv, &host, "appearance_get", json!({}));
    assert!(a.contains("\"pastel\"") && a.contains("\"nord\""), "katalog stylů a motivů");
    let (_, l) = call(&srv, &host, "layout_get", json!({}));
    assert!(l.contains("\"projects\"") && l.contains("\"hiddenParts\""));
    assert_eq!(host.saves.load(Ordering::SeqCst), 0, "čtení nic neukládá");
}

/// Platný zápis změní config a `undo_last` ho vrátí; neplatný = chyba, config i počet uložení beze změny.
fn roundtrip(name: &str, good: Value, bad: Value, changed: impl Fn(&Config) -> bool) {
    let srv = server();
    let host = MemHost::new(true);
    let start = host.config();
    let (err, text) = call(&srv, &host, name, bad.clone());
    assert!(err, "{name} {bad}: měla být chyba, dostal {text}");
    assert!(!text.contains(" at line "), "lidská věta: {text}");
    assert_eq!(host.config(), start, "{name}: neplatný vstup config nezměnil");
    assert_eq!(host.saves.load(Ordering::SeqCst), 0);
    let (err, text) = call(&srv, &host, name, good.clone());
    assert!(!err, "{name} {good}: {text}");
    assert!(changed(&host.config()), "{name}: změna se neprojevila");
    assert!(text.contains("undo_last"), "{text}");
    let (err, text) = call(&srv, &host, "undo_last", json!({}));
    assert!(!err, "{text}");
    let mut want = start.clone();
    crate::layout::derive(&mut want);
    assert_eq!(host.config(), want, "{name}: undo vrátil původní stav");
    assert!(call(&srv, &host, "undo_last", json!({})).0, "prázdný zásobník = chyba");
}

/// Zástupce `kind: app` nemá `openBrowser` ani `passPort` (patří dev serveru); aplikace ze Storu jde přes `aumid`.
#[test]
fn app_tiles_have_no_dev_flags() {
    let srv = server();
    let host = MemHost::new(true);
    let flags = |id: &str| host.cfg.lock().projects.iter().find(|p| p.id == id).map(|p| (p.kind, p.pass_port, p.open_browser, p.aumid.clone()));
    // výchozí hodnoty configu jsou true — MCP je pro app vypne, i když je model výslovně zapne
    for args in [
        json!({ "name": "Notepad", "kind": "app", "exe": r"C:\Windows\notepad.exe" }),
        json!({ "name": "Notepad2", "kind": "app", "exe": r"C:\Windows\notepad.exe", "openBrowser": true, "passPort": true }),
    ] {
        let (err, text) = call(&srv, &host, "tiles_add", args);
        assert!(!err, "{text}");
    }
    assert_eq!(flags("notepad"), Some((crate::config::ProjectKind::App, false, false, None)));
    assert_eq!(flags("notepad2"), Some((crate::config::ProjectKind::App, false, false, None)));
    // Store: jen aumid, bez exe
    let aumid = "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App";
    let (err, text) = call(&srv, &host, "tiles_add", json!({ "name": "Kalkulačka", "kind": "app", "aumid": aumid }));
    assert!(!err, "{text}");
    assert_eq!(flags("kalkulacka"), Some((crate::config::ProjectKind::App, false, false, Some(aumid.to_string()))));
    // špatné aumid (cesta, příkaz) se nepřijme; app bez exe / appName / aumid také ne
    for bad in [json!({ "name": "x", "kind": "app", "aumid": r"..\..\calc.exe" }), json!({ "name": "x", "kind": "app", "aumid": "Pkg_abc!App && calc" }), json!({ "name": "x", "kind": "app" })] {
        let (err, text) = call(&srv, &host, "tiles_add", bad.clone());
        assert!(err, "{bad}: {text}");
    }
    // dev zástupce zůstává s výchozími true a po přepnutí na app je vypnutý
    let (err, text) = call(&srv, &host, "tiles_add", json!({ "name": "Web", "kind": "dev", "run": "npm run dev" }));
    assert!(!err, "{text}");
    assert_eq!(flags("web").map(|f| (f.1, f.2)), Some((true, true)));
    let (err, text) = call(&srv, &host, "tiles_update", json!({ "id": "web", "kind": "app", "exe": r"C:\Windows\notepad.exe", "run": null }));
    assert!(!err, "{text}");
    assert_eq!(flags("web").map(|f| (f.0, f.1, f.2)), Some((crate::config::ProjectKind::App, false, false)));
}

#[test]
fn tiles_write_tools() {
    let dir = tempfile::tempdir().unwrap();
    let web = dir.path().join("obchod");
    std::fs::create_dir_all(&web).unwrap();
    std::fs::write(web.join("package.json"), r#"{"scripts":{"dev":"vite --port 5310"}}"#).unwrap();
    let path = web.to_string_lossy().to_string();
    // složka → návrh jako Nastavení (druh, příkaz, port)
    roundtrip("tiles_add", json!({ "name": "Obchod", "path": path }), json!({ "name": "Obchod", "path": path, "run": "calc.exe" }), |c| {
        c.projects.iter().any(|p| p.id == "obchod" && p.run.as_deref() == Some("npm run dev") && p.port == Some(5310))
    });
    roundtrip("tiles_add", json!({ "name": "Poznámkový blok", "kind": "app", "exe": "C:\\Windows\\notepad.exe" }), json!({ "name": "x", "kind": "app", "exe": "C:\\Windows\\notepad.exe", "args": ["/c", "del"] }), |c| {
        c.projects.iter().any(|p| p.id == "poznamkovy-blok")
    });
    for bad in [json!({ "kind": "app" }), json!({ "name": "x", "kind": "neco", "run": "npm run dev" }), json!({ "name": "x", "colour": "#fff" }), json!({ "name": "x", "kind": "dev" }), json!({ "name": "x", "path": "relativni" })] {
        let srv = server();
        let host = MemHost::new(true);
        let (err, text) = call(&srv, &host, "tiles_add", bad.clone());
        assert!(err, "{bad}: {text}");
    }

    // úpravy a odebrání nad existujícím zástupcem
    let mk = || {
        let host = MemHost::new(true);
        host.cfg.lock().projects.push(serde_json::from_value(json!({ "id": "app", "name": "app", "kind": "dev", "path": path, "run": "npm run dev" })).unwrap());
        host
    };
    for (args, bad) in [
        (json!({ "id": "app", "port": 5400, "look": { "icon": { "kind": "emoji", "value": "🛒" }, "color": "#2563eb" } }), json!({ "id": "app", "look": { "color": "modra" } })),
        (json!({ "id": "app", "run": "pnpm dev" }), json!({ "id": "app", "run": "npm run dev & calc" })),
        (json!({ "id": "app", "accent": null, "enabled": false }), json!({ "id": "nic", "port": 1 })),
    ] {
        let srv = server();
        let host = mk();
        let start = host.config();
        assert!(call(&srv, &host, "tiles_update", bad).0);
        assert_eq!(host.config(), start);
        let (err, text) = call(&srv, &host, "tiles_update", args);
        assert!(!err, "{text}");
        assert_ne!(host.config(), start);
        assert!(!call(&srv, &host, "undo_last", json!({})).0);
        assert_eq!(host.config().projects, start.projects);
    }
    let srv = server();
    let host = mk();
    assert!(call(&srv, &host, "tiles_remove", json!({ "id": "nic" })).1.contains("app"), "chyba vyjmenuje existující id");
    assert!(!call(&srv, &host, "tiles_remove", json!({ "id": "app" })).0);
    assert!(host.config().projects.is_empty());
    assert!(!call(&srv, &host, "undo_last", json!({})).0);
    assert_eq!(host.config().projects.len(), 1);
}

#[test]
fn actions_write_tools() {
    roundtrip("actions_add", json!({ "label": "Pošta", "kind": "url", "target": "https://mail.example.com" }), json!({ "label": "Pošta", "kind": "url", "target": "ftp://example.com" }), |c| {
        c.actions.iter().any(|a| a.id == "posta" && a.target == "https://mail.example.com")
    });
    // příkaz projde, ale souhlas nedostane — Ledge se zeptá při prvním spuštění
    let srv = server();
    let host = MemHost::new(true);
    let (err, text) = call(&srv, &host, "actions_add", json!({ "label": "Build", "kind": "command", "target": "npm run build", "cwd": "C:\\code\\app" }));
    assert!(!err && text.contains("souhlas"), "{text}");
    let a = host.config().actions[0].clone();
    assert!(crate::actions::needs_confirm(&a, false), "bez souhlasu se ptá");
    for bad in [json!({ "kind": "url", "target": "https://example.com" }), json!({ "label": "x", "kind": "skript", "target": "x" }), json!({ "label": "x", "kind": "app", "target": "C:\\x\\a.bat" }), json!({ "id": "build", "label": "x", "kind": "url", "target": "https://example.com" })] {
        let before = host.config();
        assert!(call(&srv, &host, "actions_add", bad.clone()).0, "{bad}");
        assert_eq!(host.config(), before);
    }
    let start = host.config();
    assert!(call(&srv, &host, "actions_update", json!({ "id": "build", "timeoutS": 0 })).0);
    assert_eq!(host.config(), start);
    let (err, text) = call(&srv, &host, "actions_update", json!({ "id": "build", "target": "npm run build:prod" }));
    assert!(!err && text.contains("souhlas"), "jiný příkaz = znovu souhlas: {text}");
    assert!(!call(&srv, &host, "undo_last", json!({})).0);
    assert_eq!(host.config(), start);
    assert!(!call(&srv, &host, "actions_remove", json!({ "id": "build" })).0);
    assert!(host.config().actions.is_empty());
    assert!(call(&srv, &host, "actions_remove", json!({ "id": "build" })).0);
}

#[test]
fn layout_tools_use_editor_validation() {
    // stejná fixture jako editor rozložení (`layout.rs`, `src/layout/validate.ts`)
    let cases: Vec<Value> = serde_json::from_str(include_str!("../fixtures/layout-validate.json")).unwrap();
    let srv = server();
    let host = MemHost::new(true);
    for c in &cases {
        let (err, text) = call(&srv, &host, "layout_preview", json!({ "layout": c["layout"] }));
        assert_eq!(err, !c["field"].is_null(), "{}: {text}", c["name"]);
    }
    assert_eq!(host.saves.load(Ordering::SeqCst), 0, "preview nic neukládá");
    let good = json!({ "layout": { "pages": [{ "id": "prace", "name": "", "cards": [{ "id": "projects", "size": "xl" }, { "id": "claude", "size": "m", "h": 2 }] }], "hiddenParts": [] } });
    let bad = json!({ "layout": { "pages": [{ "id": "prace", "name": "", "cards": [{ "id": "kocka", "size": "m" }] }] } });
    roundtrip("layout_apply", good.clone(), bad, |c| c.layout.pages.len() == 1 && !c.drawer.sections.weather);
    let (_, text) = call(&srv, &host, "layout_preview", good);
    assert!(text.contains("projects XL") && text.contains("weather"), "souhrn ukáže karty i co zmizí: {text}");
    // překlep v poli karty = chyba, ne tiché ignorování
    assert!(call(&srv, &host, "layout_preview", json!({ "layout": { "pages": [{ "id": "prace", "cards": [{ "id": "claude", "sise": "m" }] }] } })).1.contains("sise"));
    // bez `upgrades` se převezmou (jednorázové úpravy se nepustí znovu)
    let srv = server();
    let host = MemHost::new(true);
    let ups = host.config().layout.upgrades.clone();
    assert!(!call(&srv, &host, "layout_apply", json!({ "layout": { "pages": [{ "id": "prace", "name": "", "cards": [] }] } })).0);
    assert_eq!(host.config().layout.upgrades, ups);
}

#[test]
fn strip_and_appearance_tools() {
    roundtrip("strip_set", json!({ "items": ["clock", "waiting"], "custom": "Ahoj" }), json!({ "items": ["clock", "kocka"] }), |c| c.strip.custom == "Ahoj" && c.strip.items.len() == 2);
    let srv = server();
    let host = MemHost::new(true);
    let (_, e) = call(&srv, &host, "strip_set", json!({ "custom": "x".repeat(40) }));
    assert!(e.contains("strip.custom"), "{e}");
    assert!(call(&srv, &host, "strip_set", json!({ "itemsRev": 9 })).0);

    roundtrip("appearance_set", json!({ "style": "paper" }), json!({ "style": "zlata" }), |c| c.appearance.style == "paper" && serde_json::to_value(c.appearance.theme).unwrap() == "paper");
    roundtrip("appearance_set", json!({ "theme": "nord", "accent": "#2563eb", "blur": true }), json!({ "accent": "modra" }), |c| c.appearance.accent == "#2563eb" && c.drawer.glass == crate::config::Glass::Acrylic);
    let (_, e) = call(&srv, &host, "appearance_set", json!({ "theme": "fialova" }));
    assert!(e.contains("nord"), "povolené hodnoty v hlášce: {e}");
    let (_, e) = call(&srv, &host, "appearance_set", json!({ "style": "zlata" }));
    assert!(e.contains("pastel"), "{e}");
    // styl a navrch jednotlivé pole
    assert!(!call(&srv, &host, "appearance_set", json!({ "style": "neon", "density": "compact" })).0);
    let a = host.config().appearance;
    assert_eq!((a.style.as_str(), serde_json::to_value(a.density).unwrap()), ("neon", json!("compact")));
}

#[test]
fn no_change_and_undo_refused_after_foreign_edit() {
    let srv = server();
    let host = MemHost::new(true);
    let (err, text) = call(&srv, &host, "strip_set", json!({}));
    assert!(!err && text.contains("Beze změny"), "{text}");
    assert_eq!(srv.undo.lock().len(), 0, "beze změny = nic do Vrátit");
    assert!(!call(&srv, &host, "strip_set", json!({ "custom": "MCP" })).0);
    // uživatel mezitím změnil proužek v Nastavení → undo odmítne a nic nepřepíše
    host.cfg.lock().strip.custom = "ruka".into();
    let (err, text) = call(&srv, &host, "undo_last", json!({}));
    assert!(err && text.contains("strip"), "{text}");
    assert_eq!(host.config().strip.custom, "ruka");
}

#[test]
fn old_config_without_mcp_is_off() {
    let c: Config = serde_json::from_str("{}").unwrap();
    assert!(!c.mcp.enabled);
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.json");
    std::fs::write(&p, include_str!("../fixtures/config-existing-user.json")).unwrap();
    let c = crate::config::load(&p).unwrap().unwrap();
    assert!(!c.mcp.enabled, "existující config uživatele: MCP vypnuté");
    crate::config::save(&p, &c).unwrap();
    assert_eq!(crate::config::load(&p).unwrap().unwrap(), c, "load → save → load beze změny");
    assert!(crate::config::schema_json().contains("\"mcp\""), "schéma configu zná sekci mcp");
}
