//! HTTP most na 127.0.0.1 (`tiny_http`): `POST /permission`, `POST /question`, `POST /plan`, `POST /event`, `GET /status`,
//! `POST /ctl/<příkaz>`, `/mcp` (MCP server, `crate::mcp`). Odmítá prohlížeč (`Origin`) a cizí `Host`, hook bez
//! platného klíče nedostane rozhodnutí (`hook_admitted`). Každý
//! požadavek ve vlastním vlákně včetně čtení těla; logiku dělá `Claude` (`mod.rs`) a `mcp::handle_http`, tady je jen
//! směrování a odpovědi.

use super::{Claude, rules};
use serde_json::{Value, json};
use std::io::Read;
use std::sync::Arc;

/// Strop těla požadavku mimo `/mcp` (ten má `crate::mcp::MAX_BODY`).
const MAX_BODY: u64 = 1 << 20;

/// Strop deklarované délky těla, nad nímž se požadavek zahodí bez odpovědi (viz `serve`). Hodně nad
/// `MAX_BODY` i `mcp::MAX_BODY`; alokace do této velikosti je neškodná.
const MAX_DECLARED_BODY: usize = 64 << 20;

/// Přesahuje deklarovaná délka těla `MAX_DECLARED_BODY`? Bez hlavičky (`None`) ne.
fn declared_body_too_big(len: Option<usize>) -> bool {
    len.is_some_and(|n| n > MAX_DECLARED_BODY)
}

fn header<'a>(req: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    req.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name)).map(|h| h.value.as_str())
}

fn respond(req: tiny_http::Request, code: u16, body: String) {
    let ct = tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap();
    let _ = req.respond(tiny_http::Response::from_string(body).with_status_code(code).with_header(ct));
}

fn respond_mcp(req: tiny_http::Request, r: crate::mcp::HttpResp) {
    let mut resp = tiny_http::Response::from_string(r.body.clone()).with_status_code(r.code);
    if !r.body.is_empty() {
        resp = resp.with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap());
    }
    if let Some((k, v)) = r.header {
        resp = resp.with_header(tiny_http::Header::from_bytes(k, v).unwrap());
    }
    let _ = req.respond(resp);
}

/// Je hlavička `Host` náš loopback? Přesně `127.0.0.1`, `localhost` nebo `[::1]`, volitelně s číselným
/// portem. Prefix nestačí: `localhost.evil.example` je po DNS rebindingu adresa útočníkovy stránky.
///
/// Požadavek úplně bez `Host` volající propouští: prohlížeč ho posílá vždy, takže rebinding tudy nevede,
/// a klient, který si ho umí vynechat, si ho umí i zapsat správně. Odmítnutí by nic nechránilo.
fn host_allowed(host: &str) -> bool {
    let (name, port) = match host.strip_prefix("[::1]") {
        Some(rest) => ("[::1]", rest),
        None => host.find(':').map_or((host, ""), |i| host.split_at(i)),
    };
    let port_ok = port.is_empty() || port.strip_prefix(':').is_some_and(|p| p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u16>().is_ok());
    port_ok && (name == "[::1]" || name == "127.0.0.1" || name.eq_ignore_ascii_case("localhost"))
}

/// Nese hlavička `Authorization` platný klíč hooků?
fn key_valid(authorization: Option<&str>, key: Option<&str>) -> bool {
    matches!((authorization.and_then(crate::mcp::auth::bearer), key), (Some(given), Some(key)) if crate::accounts::pkce::same(given, key))
}

/// Pustit hook dál? S platným klíčem vždy. Bez něj dostane prázdnou 200, tedy „žádné rozhodnutí“, a terminál
/// se zeptá sám; karta nevznikne, takže povolení bez klíče nejde dostat ani kliknutím.
/// Výjimka: `/event` úplně bez hlavičky projde. Session spuštěné před aktualizací drží hooky ze starého
/// `settings.json` a aktivita jim má běžet dál; událost jen hlásí stav a povolení nevydá (`Stop` umí jen zablokovat).
/// Špatná hlavička neprojde nikde: to není stará verze.
fn hook_admitted(path: &str, authorization: Option<&str>, key: Option<&str>) -> bool {
    let valid = key_valid(authorization, key);
    match path {
        "/permission" | "/question" | "/plan" => valid,
        "/event" => valid || authorization.is_none(),
        _ => true,
    }
}

/// Jednou za běh do logu: rozhodování bez klíče vypadá jako „karta se neukázala“. Klíč se sem nedostane.
fn note_keyless(path: &str, authorization: Option<&str>) {
    #[cfg(not(test))]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        if authorization.is_none() {
            ONCE.call_once(|| crate::state::log(&format!("hook {path} bez klíče: rozhoduje terminál (session spuštěná před aktualizací Ledge?)")));
        }
    }
    #[cfg(test)]
    let _ = (path, authorization);
}

/// Klíč hooků (`hooksetup::hook_key`); `None` = žádný, hook bez klíče neprojde a s klíčem taky ne.
pub type HookKey = Arc<dyn Fn() -> Option<String> + Send + Sync>;

/// Obsluha `POST /ctl/<příkaz>` (otevřít/zavřít šuplík, reload…) — ovládání pro Clauda a skripty.
pub type Ctl = Arc<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>;

/// Obsluha `/mcp` (`mcp::host::endpoint`; vypnuté MCP = 404 uvnitř).
pub type Mcp = Arc<dyn Fn(&crate::mcp::HttpReq) -> crate::mcp::HttpResp + Send + Sync>;

/// Spustí server. `extra` obslouží /status rozšířený o stav Ledge, `ctl` příkazy `/ctl/…`, `mcp` cestu `/mcp`,
/// `hook_key` je klíč hooků.
pub fn serve(port: u16, claude: Claude, extra: Arc<dyn Fn() -> Value + Send + Sync>, ctl: Ctl, mcp: Mcp, hook_key: HookKey) -> Result<(), String> {
    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| crate::errors::any(&trf!("Port {port}", "Port {port}"), &e))?;
    std::thread::Builder::new()
        .name("claude-bridge".into())
        .spawn(move || {
            // tělo se čte až v pracovním vlákně: tiny_http nemá termín čtení, pomalý klient by tady zablokoval
            // všechny hooky; hlavičky má hotové vlákno spojení. Na tomto vlákně se tělo nesmí číst ani
            // nepřímo: každé `respond` (i odmítnutí) a každý zánik `Request` jde přes `spawn`.
            for req in server.incoming_requests() {
                // Obří `Content-Length`: zánik `Request` v tiny_http alokuje `vec![0; zbývající_délka]`, takže
                // hlavička o terabajtu shodí proces. Knihovnu nezáplatujeme; takový požadavek necháme záměrně
                // uniknout (bez odpovědi, spojení visí do zavření klientem). Únik je omezený tím, že jde jen o
                // místní proces, a je menší zlo než pád Ledge.
                if declared_body_too_big(req.body_length()) {
                    std::mem::forget(req);
                    continue;
                }
                // Odmítnutí jde do vlákna jako ostatní: zánik `Request` čte zbytek těla, což u pomalého
                // klienta nemá termín a příjmové vlákno by stálo.
                // prohlížeč (i DNS rebinding) sem nesmí
                if header(&req, "Origin").is_some() {
                    std::thread::spawn(move || respond(req, 403, "{}".into()));
                    continue;
                }
                // chybějící Host: prohlížeč ho posílá vždy, takže to rebinding není (viz `host_allowed`)
                if !header(&req, "Host").is_none_or(host_allowed) {
                    std::thread::spawn(move || respond(req, 403, "{}".into()));
                    continue;
                }
                let url = req.url().to_string();
                let method = req.method().as_str().to_string();
                if url == crate::mcp::PATH {
                    // MCP: tělo jako bajty s vlastním stropem (přes strop = 413), klíč a verze z hlaviček
                    let auth = header(&req, "Authorization").map(str::to_string);
                    let proto = header(&req, "MCP-Protocol-Version").map(str::to_string);
                    let mcp = mcp.clone();
                    let mut req = req;
                    std::thread::spawn(move || {
                        let mut raw = Vec::new();
                        if method == "POST" {
                            let _ = req.as_reader().take(crate::mcp::MAX_BODY as u64 + 1).read_to_end(&mut raw);
                        }
                        let r = mcp(&crate::mcp::HttpReq {
                            method: &method,
                            authorization: auth.as_deref(),
                            protocol: proto.as_deref(),
                            body: (raw.len() <= crate::mcp::MAX_BODY).then_some(raw.as_slice()),
                        });
                        respond_mcp(req, r);
                    });
                    continue;
                }
                let auth = header(&req, "Authorization").map(str::to_string);
                let claude = claude.clone();
                let extra = extra.clone();
                let ctl = ctl.clone();
                let hook_key = hook_key.clone();
                // každý požadavek ve vlastním vlákně — schválení může čekat desítky sekund
                let mut req = req;
                std::thread::spawn(move || {
                    if method == "POST" && !hook_admitted(&url, auth.as_deref(), hook_key().as_deref()) {
                        note_keyless(&url, auth.as_deref());
                        respond(req, 200, String::new());
                        return;
                    }
                    let mut body = String::new();
                    if method == "POST" {
                        let _ = req.as_reader().take(MAX_BODY).read_to_string(&mut body);
                    }
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
                            // událost bez klíče (stará session) hlásí aktivitu, ale neuvolní žádnou kartu
                            let reply = claude.handle_event_as(&parsed, key_valid(auth.as_deref(), hook_key().as_deref()));
                            respond(req, 200, reply.unwrap_or_default());
                        }
                        ("GET", "/status") => {
                            let mut v = claude.status_json();
                            if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra()) {
                                o.extend(e);
                            }
                            // karta s náhledem úpravy `settings.json` by klíč ukázala i tady
                            let mut text = v.to_string();
                            if let Some(k) = hook_key() {
                                text = text.replace(&k, "***");
                            }
                            respond(req, 200, text);
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
        .map_err(|e| crate::errors::io(crate::i18n::tr("Most pro Claude nejde spustit", "Can't start the Claude bridge"), &e))?;
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
        let (port, ()) = crate::ports::with_free_port(|port| {
            serve(port, c.clone(), Arc::new(|| json!({})), Arc::new(|_, _| Ok(json!({}))), Arc::new(|_: &crate::mcp::HttpReq| crate::mcp::HttpResp { code: 404, body: String::new(), header: None }), Arc::new(|| Some(KEY.to_string())))
        });
        (c, port)
    }

    const KEY: &str = "testovaci-klic-hooku";

    fn post(port: u16, path: &str, body: &str, origin: bool) -> (u16, String) {
        let o = if origin { "Origin: http://evil.example\r\n" } else { "" };
        send(port, path, body, &format!("{o}Authorization: Bearer {KEY}\r\n"))
    }

    /// POST s vlastními hlavičkami navíc (každá s `\r\n`).
    fn send(port: u16, path: &str, body: &str, extra: &str) -> (u16, String) {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let code = out[9..12].parse().unwrap();
        let body = out.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (code, body)
    }

    const REQ: &str = r#"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"firebase deploy --only hosting:staging"}}"#;

    #[test]
    fn hook_admitted_table() {
        let good = format!("Bearer {KEY}");
        let good = Some(good.as_str());
        for path in ["/permission", "/question", "/plan"] {
            // rozhodování jen s platným klíčem, nikdy jinak
            for (auth, key, want) in [
                (good, Some(KEY), true),
                (None, Some(KEY), false),
                (Some("Bearer jiny"), Some(KEY), false),
                (Some(KEY), Some(KEY), false),
                (Some("Bearer "), Some(""), false),
                (good, None, false),
                (None, None, false),
            ] {
                assert_eq!(hook_admitted(path, auth, key), want, "{path} {auth:?} {key:?}");
            }
        }
        // událost: stará session bez hlavičky ano, špatná hlavička ne
        for (auth, key, want) in [(good, Some(KEY), true), (None, Some(KEY), true), (None, None, true), (Some("Bearer jiny"), Some(KEY), false), (good, None, false)] {
            assert_eq!(hook_admitted("/event", auth, key), want, "/event {auth:?} {key:?}");
        }
        // jiné cesty se tu neřeší
        for path in ["/ctl/open", "/status", "/neni"] {
            assert!(hook_admitted(path, None, Some(KEY)), "{path}");
        }
    }

    const BAD_KEY: &str = "Authorization: Bearer jiny\r\n";

    /// Bez platného klíče žádná karta, žádné čekání a prázdné tělo: terminál se zeptá sám.
    #[test]
    fn no_decision_without_valid_key() {
        let (c, port) = bridge(10_000);
        const PLAN: &str = r##"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PreToolUse","tool_name":"ExitPlanMode","tool_use_id":"toolu_01","tool_input":{"plan":"# Plán A"}}"##;
        const ASK: &str = r#"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PreToolUse","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"Kam?","header":"Cíl","multiSelect":false,"options":[{"label":"a"},{"label":"b"}]}]}}"#;
        for (path, body) in [("/permission", REQ), ("/plan", PLAN), ("/question", ASK)] {
            for extra in ["", BAD_KEY] {
                let t0 = std::time::Instant::now();
                assert_eq!(send(port, path, body, extra), (200, String::new()), "{path} {extra}");
                assert!(t0.elapsed() < Duration::from_secs(5), "{path}: nesmí čekat na člověka");
                assert!(c.pending().is_empty() && c.questions().is_empty() && c.sessions().is_empty(), "{path} {extra}");
            }
        }
    }

    /// Stará session bez hlavičky dál hlásí aktivitu; špatný klíč se zahodí.
    #[test]
    fn event_without_header_keeps_activity() {
        let (c, port) = bridge(5000);
        let ev = |sid: &str| json!({"hook_event_name":"UserPromptSubmit","session_id":sid,"cwd":"C:/x","prompt":"ahoj"}).to_string();
        assert_eq!(send(port, "/event", &ev("stara"), ""), (200, String::new()));
        assert_eq!(send(port, "/event", &ev("cizi"), BAD_KEY), (200, String::new()));
        let ids: Vec<String> = c.sessions().iter().map(|s| s.id.clone()).collect();
        assert_eq!(ids, ["stara"]);
    }

    /// Klíč se v `/status` neukáže, ani když ho karta nese v náhledu nebo shrnutí.
    #[test]
    fn key_never_in_status() {
        use std::io::{Read, Write};
        let (c, port) = bridge(5000);
        let body = json!({"session_id":"s1","cwd":"C:/x","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":format!("echo Bearer {KEY}")}}).to_string();
        std::thread::spawn(move || post(port, "/permission", &body, false));
        while c.pending().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET /status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.contains("echo Bearer ***"), "{out}");
        assert!(!out.contains(KEY), "{out}");
        c.decide(None, Decision::Deny);
    }

    #[test]
    fn host_allowed_table() {
        for (host, want) in [
            ("127.0.0.1", true),
            ("127.0.0.1:47800", true),
            ("localhost", true),
            ("localhost:47800", true),
            ("LOCALHOST:47800", true),
            ("[::1]", true),
            ("[::1]:47800", true),
            ("localhost.evil.example", false),
            ("localhost.evil.example:47800", false),
            ("127.0.0.1.evil.example", false),
            ("127.0.0.1.evil.example:47800", false),
            ("[::1].evil.example", false),
            ("evil.example", false),
            ("evil.example:47800", false),
            ("localhost:47800.evil.example", false),
            ("localhost:", false),
            ("localhost:99999", false),
            ("localhost:+80", false),
            ("localhost:80:80", false),
            ("[::1]47800", false),
            ("[::2]:47800", false),
            ("127.0.0.2:47800", false),
            ("", false),
        ] {
            assert_eq!(host_allowed(host), want, "{host}");
        }
    }

    /// Syrový požadavek s vlastní hlavičkou `Host` (`None` = bez ní); vrací stavový kód.
    fn get_with_host(port: u16, host: Option<&str>) -> u16 {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let h = host.map(|h| format!("Host: {h}\r\n")).unwrap_or_default();
        write!(s, "GET /status HTTP/1.1\r\n{h}Connection: close\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out[9..12].parse().unwrap()
    }

    /// DNS rebinding přes celý server: cizí jméno s naším prefixem se ke `/status` nedostane.
    #[test]
    fn foreign_host_cannot_read_status() {
        let (_c, port) = bridge(5000);
        for (host, want) in [
            (Some("localhost.evil.example".to_string()), 403),
            (Some(format!("127.0.0.1.evil.example:{port}")), 403),
            (Some(format!("127.0.0.1:{port}")), 200),
            (Some(format!("localhost:{port}")), 200),
            (None, 200),
        ] {
            assert_eq!(get_with_host(port, host.as_deref()), want, "{host:?}");
        }
    }

    /// Klient oznámí tělo a nepošle ho: příjem nesmí stát, ostatní hooky projdou, dokud pomalý visí.
    /// Délka nad 1 kB: kratší tělo si tiny_http načte už ve vlákně spojení a k příjmu by nedošlo.
    #[test]
    fn slow_body_does_not_block_other_requests() {
        use std::io::Write;
        let (_c, port) = bridge(5000);
        for path in ["/mcp", "/event", "/permission", "/ctl/open"] {
            let mut slow = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(slow, "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {KEY}\r\nContent-Length: 5000\r\n\r\n{{").unwrap();
            slow.flush().unwrap();
            std::thread::sleep(Duration::from_millis(50));
            // bez opravy by tohle čekalo do zavření `slow`, tedy navždy
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || tx.send(post(port, "/event", r#"{"hook_event_name":"Stop","session_id":"s9"}"#, false)));
            let (code, _) = rx.recv_timeout(Duration::from_secs(20)).unwrap_or_else(|_| panic!("most stojí na pomalém {path}"));
            assert_eq!(code, 200, "{path}");
        }
    }

    /// Odmítnutý požadavek (`Origin`, cizí `Host`) s nedodaným tělem nesmí zastavit příjem.
    #[test]
    fn rejected_slow_request_does_not_block_receiver() {
        use std::io::Write;
        let (_c, port) = bridge(5000);
        for extra in ["Origin: x\r\n", "Host: evil.example\r\n"] {
            let mut slow = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            // druhý `Host` by se s prvním popral; cizí Host proto jde bez vlastní hlavičky Host
            let host = if extra.starts_with("Host") { String::new() } else { format!("Host: 127.0.0.1:{port}\r\n") };
            write!(slow, "POST /event HTTP/1.1\r\n{host}{extra}Content-Length: 5000\r\n\r\n{{").unwrap();
            slow.flush().unwrap();
            std::thread::sleep(Duration::from_millis(50));
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || tx.send(post(port, "/event", r#"{"hook_event_name":"Stop","session_id":"s9"}"#, false)));
            let (code, _) = rx.recv_timeout(Duration::from_secs(20)).unwrap_or_else(|_| panic!("příjem stojí na odmítnutém {extra:?}"));
            assert_eq!(code, 200, "{extra:?}");
        }
    }

    /// Hranice zahazování obřího `Content-Length`.
    #[test]
    fn declared_body_too_big_table() {
        for (len, want) in [(None, false), (Some(0), false), (Some(MAX_DECLARED_BODY), false), (Some(MAX_DECLARED_BODY + 1), true), (Some(usize::MAX), true)] {
            assert_eq!(declared_body_too_big(len), want, "{len:?}");
        }
    }

    /// `Content-Length` o terabajtu bez těla: požadavek se zahodí, proces žije a další požadavek projde.
    #[test]
    fn huge_content_length_does_not_crash() {
        use std::io::{Read, Write};
        let (_c, port) = bridge(5000);
        let mut evil = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(evil, "GET /status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 1000000000000\r\n\r\n").unwrap();
        evil.flush().unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        write!(s, "GET /status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 200"), "{out}");
        drop(evil);
    }

    /// `/event` bez klíče hlásí aktivitu, ale kartu neuvolní; s klíčem ji uvolní.
    #[test]
    fn keyless_stop_keeps_card_keyed_stop_releases_it() {
        let (c, port) = bridge(10_000);
        let req = std::thread::spawn(move || post(port, "/permission", r#"{"session_id":"s1","cwd":"C:/x","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"echo ahoj"}}"#, false));
        while c.pending().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let stop = json!({"hook_event_name":"Stop","session_id":"s1"}).to_string();
        assert_eq!(send(port, "/event", &stop, ""), (200, String::new()));
        assert_eq!(c.pending().len(), 1, "událost bez klíče uvolnila kartu");
        assert_eq!(sess(&c, "s1").state, SessionState::Idle, "aktivita se nezapsala");
        assert_eq!(send(port, "/event", &stop, &format!("Authorization: Bearer {KEY}\r\n")), (200, String::new()));
        assert!(c.pending().is_empty());
        let (_, body) = req.join().unwrap();
        assert!(body.is_empty());
    }

    /// Stropy délky po přesunu čtení do vlákna: `/mcp` přes `MAX_BODY` dostane `None`, ostatní se uříznou.
    #[test]
    fn body_caps_stay() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let c = Claude::new(Arc::new(|| {}), Duration::from_millis(200)).inline().patient();
        let over = Arc::new(AtomicBool::new(false));
        let seen = over.clone();
        let (port, ()) = crate::ports::with_free_port(|port| {
            let seen = seen.clone();
            serve(port, c.clone(), Arc::new(|| json!({})), Arc::new(|_, _| Ok(json!({}))), Arc::new(move |r: &crate::mcp::HttpReq| {
                seen.store(r.body.is_none(), Ordering::SeqCst);
                crate::mcp::HttpResp { code: 413, body: String::new(), header: None }
            }), Arc::new(|| Some(KEY.to_string())))
        });
        let big = "x".repeat(crate::mcp::MAX_BODY + 1);
        assert_eq!(post(port, crate::mcp::PATH, &big, false).0, 413);
        assert!(over.load(Ordering::SeqCst), "přes strop MCP = bez těla");
        // JSON platný, ale delší než strop: po uříznutí se nepřečte, takže žádná karta ani session
        let long = format!(r#"{{"hook_event_name":"PermissionRequest","session_id":"s1","tool_name":"Bash","tool_input":{{"command":"{}"}}}}"#, "x".repeat(MAX_BODY as usize));
        assert_eq!(post(port, "/permission", &long, false), (200, String::new()));
        assert!(c.pending().is_empty() && c.sessions().is_empty());
    }

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
            c2.handle_event(&json!({"hook_event_name":"PostToolUse","session_id":"s1","tool_name":"Bash","tool_input":{"command":"firebase deploy --only hosting:staging"}}));
        });
        let t0 = std::time::Instant::now();
        let (_, body) = post(port, "/permission", REQ, false);
        assert!(body.is_empty());
        assert!(t0.elapsed() < Duration::from_secs(5));
    }

    /// Karta Bash čeká; které události ji uvolní. `Pass` = prázdné tělo, terminál se zeptá sám.
    #[test]
    fn card_survives_other_tools() {
        let post_tool = |ev: &str, tool: &str, input: Value, use_id: &str| json!({"hook_event_name": ev, "session_id": "s1", "tool_name": tool, "tool_input": input, "tool_use_id": use_id});
        let same = json!({"command": "firebase deploy --only hosting:staging", "description": "nasazení"});
        // (událost, zmizí karta?)
        for (event, gone) in [
            (post_tool("PostToolUse", "Read", json!({"file_path": "C:/x/a.txt"}), "toolu_1"), false),
            (post_tool("PostToolUse", "Bash", json!({"command": "git status"}), "toolu_2"), false),
            (post_tool("PostToolUseFailure", "Bash", json!({"command": "git status"}), "toolu_3"), false),
            (json!({"hook_event_name": "PostToolUse", "session_id": "s1"}), false),
            (json!({"hook_event_name": "PostToolUse", "session_id": "jina", "tool_name": "Bash", "tool_input": same}), false),
            (post_tool("PostToolUse", "Bash", same.clone(), "toolu_4"), true),
            (post_tool("PostToolUseFailure", "Bash", same.clone(), "toolu_5"), true),
            (json!({"hook_event_name": "UserPromptSubmit", "session_id": "s1"}), true),
            (json!({"hook_event_name": "Stop", "session_id": "s1"}), true),
            (json!({"hook_event_name": "SessionEnd", "session_id": "s1"}), true),
        ] {
            let (c, port) = bridge(10_000);
            let req = std::thread::spawn(move || post(port, "/permission", REQ, false));
            while c.pending().is_empty() {
                std::thread::sleep(Duration::from_millis(5));
            }
            c.handle_event(&event);
            assert_eq!(c.pending().is_empty(), gone, "{event}");
            if !gone {
                assert_eq!(sess(&c, "s1").state, SessionState::Waiting, "{event}");
                c.decide(None, Decision::Deny);
            }
            let (_, body) = req.join().unwrap();
            assert_eq!(body.is_empty(), gone, "{event}");
        }
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
