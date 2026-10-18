//! Testy stdio proxy: čistá smyčka nad `Cursor`, odpovědi HTTP (i chunked), chyby pro klienta,
//! a skutečné roury — dítě (tentýž testovací binár) běží proxy nad svým stdin/stdout proti mostu.

use super::*;
use crate::mcp::Host;
use std::io::Cursor;

fn lines(out: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(out).lines().filter(|l| l.starts_with('{') || l.starts_with('[')).map(|l| serde_json::from_str(l).unwrap()).collect()
}

#[test]
fn loop_passes_replies_and_skips_notifications() {
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\r\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n";
    let mut out = Vec::new();
    let post = |b: &str| {
        let v: Value = serde_json::from_str(b).unwrap();
        match v.get("id") {
            Some(id) => Post::Reply(format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{}}}}\n")),
            None => Post::Accepted,
        }
    };
    run(Cursor::new(input), &mut out, &post).unwrap();
    let got = lines(&out);
    assert_eq!(got.iter().map(|v| v["id"].as_i64().unwrap()).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(String::from_utf8(out).unwrap().matches('\n').count(), 2, "jeden řádek na odpověď");
}

#[test]
fn failures_become_jsonrpc_errors_with_the_same_id() {
    for (f, needle) in [(Fail::NotRunning, "Ledge neběží"), (Fail::Disabled, "Nastavení → Claude (MCP)"), (Fail::BadKey, "Klíč"), (Fail::Other(500), "500")] {
        let mut out = Vec::new();
        let input = "{\"jsonrpc\":\"2.0\",\"id\":\"a\",\"method\":\"initialize\",\"params\":{}}\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n[{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"ping\"},{\"jsonrpc\":\"2.0\",\"method\":\"x\"}]\n";
        run(Cursor::new(input), &mut out, &|_: &str| Post::Fail(f)).unwrap();
        let got = lines(&out);
        assert_eq!(got.len(), 2, "notifikace bez výstupu: {got:?}");
        assert_eq!(got[0]["id"], "a");
        assert_eq!(got[0]["error"]["code"], FAIL_CODE);
        assert!(got[0]["error"]["message"].as_str().unwrap().contains(needle), "{f:?}: {}", got[0]);
        assert_eq!(got[1][0]["id"], 5, "dávka: chyba pro každý požadavek");
    }
    let mut out = Vec::new();
    run(Cursor::new("{nejson\n"), &mut out, &|_: &str| Post::Fail(Fail::NotRunning)).unwrap();
    assert!(lines(&out)[0]["id"].is_null());
}

#[test]
fn http_responses() {
    assert_eq!(parse_response(b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}"), Some((200, "{\"a\":1}".into())));
    assert_eq!(parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n"), Some((200, "{\"a\":1}".into())));
    assert_eq!(parse_response(b"HTTP/1.1 202 Accepted\r\n\r\n"), Some((202, String::new())));
    assert_eq!(parse_response(b"nesmysl"), None);
    for (r, want) in [
        (Some((200, "x".to_string())), Post::Reply("x".into())),
        (Some((400, "{\"error\":1}".to_string())), Post::Reply("{\"error\":1}".into())),
        (Some((202, String::new())), Post::Accepted),
        (Some((404, String::new())), Post::Fail(Fail::Disabled)),
        (Some((401, String::new())), Post::Fail(Fail::BadKey)),
        (Some((413, String::new())), Post::Fail(Fail::Other(413))),
        (None, Post::Fail(Fail::Other(0))),
    ] {
        assert_eq!(classify(r.clone()), want, "{r:?}");
    }
    // nic neposlouchá → „Ledge neběží“
    let port = crate::ports::ephemeral().unwrap();
    assert_eq!(post_http(port, Some("k"), "{}"), Post::Fail(Fail::NotRunning));
}

const CHILD_PORT: &str = "LEDGE_MCP_PIPE_PORT";
const CHILD_KEY: &str = "LEDGE_MCP_PIPE_KEY";

/// Dítě pro `real_pipes`: proxy nad skutečným stdin/stdout procesu. V běžném běhu testů nic nedělá.
#[test]
fn pipe_child() {
    let (Ok(port), Ok(key)) = (std::env::var(CHILD_PORT), std::env::var(CHILD_KEY)) else { return };
    let port: u16 = port.parse().unwrap();
    let post = |b: &str| post_http(port, Some(&key), b);
    run(std::io::stdin().lock(), std::io::stdout().lock(), &post).unwrap();
}

/// Skutečné roury: proces s přesměrovaným stdin/stdout (jak ho spouští Claude Code), proxy → most → MCP.
#[test]
fn real_pipes() {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    let srv = Arc::new(super::super::tests::server());
    let key = srv.keys.ensure().unwrap();
    let host = Arc::new(super::super::tests::MemHost::new(true));
    let port = crate::ports::ephemeral().unwrap();
    let claude = crate::claude::Claude::new(Arc::new(|| {}), Duration::from_secs(1));
    let (s2, h2) = (srv.clone(), host.clone());
    crate::claude::serve(port, claude, Arc::new(|| json!({})), Arc::new(|_, _| Ok(json!({}))), Arc::new(move |r: &super::super::HttpReq| super::super::handle_http(&s2, h2.as_ref(), r))).unwrap();

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "mcp::proxy::tests::pipe_child", "--nocapture", "--test-threads=1", "-q"])
        .env(CHILD_PORT, port.to_string())
        .env(CHILD_KEY, &key)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    let mut sin = child.stdin.take().unwrap();
    for line in [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"strip_set","arguments":{"custom":"roura"}}}"#,
    ] {
        sin.write_all(line.as_bytes()).unwrap();
        sin.write_all(b"\n").unwrap();
    }
    drop(sin); // konec stdin = proxy skončí
    let out = child.wait_with_output().unwrap();
    let got = lines(&out.stdout);
    assert_eq!(got.len(), 2, "dvě odpovědi, notifikace nic: {}", String::from_utf8_lossy(&out.stdout));
    assert_eq!(got[0]["result"]["serverInfo"]["name"], "ledge");
    assert_eq!(got[1]["result"]["isError"], false, "{}", got[1]);
    assert_eq!(host.config().strip.custom, "roura", "zápis došel přes rouru → most → MCP");
}
