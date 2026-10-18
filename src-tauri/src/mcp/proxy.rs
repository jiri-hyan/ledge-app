//! `ledge.exe --mcp`: tenký stdio proxy pro Claude Code / Claude Desktop. Řádky JSON-RPC ze stdin →
//! `POST 127.0.0.1:<claude.port>/mcp` s klíčem ze Správce pověření → odpověď jako řádek na stdout;
//! notifikace (202) bez výstupu. Zpracuje se v `main.rs` PŘED Tauri: žádné okno, žádný single-instance
//! zámek, žádné přeposlání běžící instanci. Klíč nikdy v příkazu ani v configu klienta.
//!
//! GUI subsystém (release) stdio roury dostane i bez konzole — klient spouští s rourami
//! (`STARTF_USESTDHANDLES`), stejně jako u `--statusline` (`statusline.rs`). `run` je čistá funkce nad
//! `BufRead`/`Write` a funkcí pro POST (testy bez sítě i se skutečnou rourou — `tests::real_pipes`).

use crate::i18n::tr;
use serde_json::{Value, json};
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Výsledek jednoho POST.
#[derive(Debug, PartialEq)]
pub enum Post {
    /// 200 + tělo (JSON-RPC odpověď)
    Reply(String),
    /// 202 (notifikace)
    Accepted,
    /// chyba, kterou klient dostane jako JSON-RPC chybu
    Fail(Fail),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Fail {
    /// nic neposlouchá (Ledge neběží nebo je vypnutý most pro Claude)
    NotRunning,
    /// 404 — MCP v Ledge vypnuté (nebo chybí klíč)
    Disabled,
    /// 401 — klíč nesedí
    BadKey,
    /// jiný HTTP kód / rozbitá odpověď
    Other(u16),
}

/// Kód JSON-RPC chyby proxy (rozsah serveru -32000…-32099).
pub const FAIL_CODE: i64 = -32000;

pub fn fail_text(f: Fail) -> String {
    match f {
        Fail::NotRunning => tr(
            "Ledge neběží — spusť Ledge (a zapni v Ledge Nastavení → Claude (MCP)).",
            "Ledge isn't running — start Ledge (and turn it on in Ledge Settings → Claude (MCP)).",
        )
        .into(),
        Fail::Disabled => tr("MCP je v Ledge vypnuté — zapni v Ledge Nastavení → Claude (MCP).", "MCP is off in Ledge — turn it on in Ledge Settings → Claude (MCP).").into(),
        Fail::BadKey => tr(
            "Klíč MCP nesedí — v Ledge Nastavení → Claude (MCP) ho mezitím někdo vyměnil; restartuj klienta.",
            "The MCP key doesn't match — it was replaced in Ledge Settings → Claude (MCP); restart the client.",
        )
        .into(),
        Fail::Other(c) => trf!("Ledge odpověděl chybou {c}.", "Ledge answered with error {c}."),
    }
}

/// Smyčka proxy: řádek po řádku do konce stdin. Chyba spojení u požadavku = JSON-RPC chyba se stejným
/// `id` (klient ví, co se stalo); u notifikace nic. Vrací chybu jen při chybě zápisu na stdout.
pub fn run(input: impl BufRead, mut output: impl Write, post: &dyn Fn(&str) -> Post) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let out = match post(line) {
            Post::Reply(body) => Some(body.trim().to_string()),
            Post::Accepted => None,
            Post::Fail(f) => error_for(line, f),
        };
        if let Some(o) = out {
            output.write_all(o.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// JSON-RPC chyba pro každý požadavek ve zprávě (i v dávce); notifikace nic.
fn error_for(line: &str, f: Fail) -> Option<String> {
    let msg = fail_text(f);
    let err = |id: &Value| json!({ "jsonrpc": "2.0", "id": id, "error": { "code": FAIL_CODE, "message": msg } });
    match serde_json::from_str::<Value>(line) {
        Ok(Value::Array(items)) => {
            let out: Vec<Value> = items.iter().filter_map(|m| m.get("id").filter(|_| m.get("method").is_some()).map(err)).collect();
            (!out.is_empty()).then(|| Value::Array(out).to_string())
        }
        Ok(m) => match m.get("id") {
            Some(id) if m.get("method").is_some() => Some(err(id).to_string()),
            _ => None,
        },
        Err(_) => Some(err(&Value::Null).to_string()),
    }
}

/// Status a tělo HTTP odpovědi (i `Transfer-Encoding: chunked`).
pub fn parse_response(raw: &[u8]) -> Option<(u16, String)> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let code: u16 = head.split_whitespace().nth(1)?.parse().ok()?;
    let body = &raw[split + 4..];
    let chunked = head.lines().any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding:") && l.to_ascii_lowercase().contains("chunked"));
    let body = if chunked { dechunk(body)? } else { body.to_vec() };
    Some((code, String::from_utf8(body).ok()?))
}

fn dechunk(mut b: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let nl = b.windows(2).position(|w| w == b"\r\n")?;
        let size = usize::from_str_radix(std::str::from_utf8(&b[..nl]).ok()?.split(';').next()?.trim(), 16).ok()?;
        b = &b[nl + 2..];
        if size == 0 {
            return Some(out);
        }
        out.extend_from_slice(b.get(..size)?);
        b = b.get(size + 2..)?;
    }
}

/// Jeden POST na most (HTTP/1.0 = bez chunked, spojení se zavře). Termín: spojení 2 s, odpověď 60 s.
pub fn post_http(port: u16, key: Option<&str>, body: &str) -> Post {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_secs(2)) else { return Post::Fail(Fail::NotRunning) };
    let _ = s.set_read_timeout(Some(Duration::from_secs(60)));
    let auth = key.map(|k| format!("Authorization: Bearer {k}\r\n")).unwrap_or_default();
    let req = format!(
        "POST /mcp HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{auth}Content-Length: {}\r\n\r\n",
        body.len()
    );
    if s.write_all(req.as_bytes()).and_then(|_| s.write_all(body.as_bytes())).is_err() {
        return Post::Fail(Fail::NotRunning);
    }
    let mut raw = Vec::new();
    if s.take(16 << 20).read_to_end(&mut raw).is_err() && raw.is_empty() {
        return Post::Fail(Fail::NotRunning);
    }
    classify(parse_response(&raw))
}

/// HTTP odpověď → co dostane klient.
pub fn classify(r: Option<(u16, String)>) -> Post {
    match r {
        Some((200, b)) => Post::Reply(b),
        // 400 nese JSON-RPC chybu (nečitelná zpráva) — předat
        Some((400, b)) if b.trim_start().starts_with('{') => Post::Reply(b),
        Some((202, _)) => Post::Accepted,
        Some((404, _)) => Post::Fail(Fail::Disabled),
        Some((401, _)) => Post::Fail(Fail::BadKey),
        Some((c, _)) => Post::Fail(Fail::Other(c)),
        None => Post::Fail(Fail::Other(0)),
    }
}

/// Vstupní bod `ledge.exe --mcp` (volá `main.rs`). Port a jazyk z configu; klíč ze Správce pověření
/// (po 401 se jednou přečte znovu — klíč mohl někdo vyměnit). Exit kód 0 po konci stdin.
pub fn main() -> i32 {
    let cfg = crate::config::load(&crate::config::config_path()).ok().flatten().unwrap_or_default();
    crate::i18n::set_lang(cfg.ui.lang);
    let port = cfg.claude.port;
    let vault = crate::accounts::vault::WinCred;
    let key = parking_lot::Mutex::new(super::auth::read(&vault));
    let post = |body: &str| {
        let k = key.lock().clone();
        let first = post_http(port, k.as_deref(), body);
        if !matches!(first, Post::Fail(Fail::BadKey) | Post::Fail(Fail::Disabled)) {
            return first;
        }
        // vypnuto → zapnuto nebo vyměněný klíč: přečíst znovu a zkusit jednou
        let fresh = super::auth::read(&vault);
        if fresh.is_some() && fresh != k {
            *key.lock() = fresh.clone();
            return post_http(port, fresh.as_deref(), body);
        }
        first
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match run(stdin.lock(), stdout.lock(), &post) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
#[path = "proxy_tests.rs"]
mod tests;
