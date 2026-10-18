//! Lokální MCP server Ledge: Claude (Code / Desktop) čte a mění nastavení Ledge — zástupce, rychlé
//! akce, rozložení, proužek, vzhled. Mapa: `docs/codemap/mcp.md`.
//!
//! - Přenos: `POST /mcp` na loopback mostu (`claude/bridge.rs`, 127.0.0.1, odmítá `Origin` a cizí `Host`),
//!   Streamable HTTP bez SSE a bez session (každá zpráva samostatně), odpověď `application/json`.
//! - Bezpečnost: `mcp.enabled` výchozí vypnuto (pak 404), klíč 32 B ve Správci pověření `Ledge/mcp`,
//!   `Authorization: Bearer`, porovnání v konstantním čase, 401 bez detailu. MCP nic nespouští.
//! - Logika: čisté funkce nad `Host` (config, uložení, sken) — `rpc` (JSON-RPC), `tools` (katalog a
//!   volání), `edit` (zástupci, akce), `look` (rozložení, proužek, vzhled), `undo`, `auth`, `proxy`
//!   (`ledge.exe --mcp`). Skutečný `Host` je `host.rs`, testy jedou nad `tests::MemHost`.

pub mod auth;
pub mod discover;
pub mod edit;
pub mod host;
pub mod look;
pub mod proxy;
pub mod round;
pub mod rpc;
pub mod schema;
pub mod tools;
pub mod undo;

#[cfg(test)]
mod tests;

use crate::config::Config;
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Cesta endpointu na mostu.
pub const PATH: &str = "/mcp";
/// Nejvíc bajtů těla požadavku (celé rozložení i se zástupci má desítky kB).
pub const MAX_BODY: usize = 256 * 1024;

/// Sekce configu `mcp`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct McpConfig {
    /// Lokální MCP server pro Claude (`POST http://127.0.0.1:<claude.port>/mcp`, klíč ve Správci pověření
    /// `Ledge/mcp`): Claude smí číst a měnit nastavení Ledge, nic nespouští a nic neposílá ven.
    /// Výchozí vypnuto (pak `/mcp` vrací 404). Zapíná se v Nastavení → Claude.
    pub enabled: bool,
}

/// Co MCP potřebuje od běžícího Ledge (skutečný v `host.rs`, v testech v paměti).
pub trait Host: Send + Sync {
    /// Aktuální config (v paměti Ledge).
    fn config(&self) -> Config;
    /// Uložit STEJNOU cestou jako Nastavení (`save_config` → `apply_config`) + událost pro UI.
    fn save(&self, cfg: Config) -> Result<(), String>;
    /// Verze Ledge.
    fn version(&self) -> String;
    /// Je šuplík právě otevřený?
    fn drawer_open(&self) -> bool;
    /// Otevřít šuplík (bez fokusu), volitelně na záložce. Vrací lidskou větu.
    fn open_drawer(&self, tab: Option<&str>) -> Result<String, String>;
    /// Aplikace ze Start menu a repa na disku (jen čtení).
    fn discover(&self, repos: bool) -> Value;
    /// Návrh projektu ze složky (stejně jako Nastavení → Projekty → Přidat). Výchozí = skutečný disk.
    fn detect(&self, dir: &std::path::Path, taken: &[String]) -> Result<crate::projectdetect::Detected, String> {
        crate::projectdetect::detect(dir, taken)
    }
    /// Příkazy, které Ledge SÁM najde ve složce projektu (`run`, `args`) — jen ty smí MCP zástupci dát.
    fn detected_runs(&self, dir: &std::path::Path) -> Vec<(String, Vec<String>)> {
        edit::detected_runs(dir)
    }
}

/// Stav serveru: klíč (úložiště + cache), zásobník Vrátit, zámek zápisů.
pub struct Server {
    pub keys: auth::Keys,
    pub undo: Mutex<undo::Undo>,
    /// zápisy přes MCP jdou po jednom (načti → změň → ulož se nesmí proplést)
    pub write: Mutex<()>,
}

impl Server {
    pub fn new(vault: Box<dyn crate::accounts::vault::Vault>) -> Self {
        Self { keys: auth::Keys::new(vault), undo: Mutex::new(undo::Undo::default()), write: Mutex::new(()) }
    }
}

/// Požadavek na `/mcp`, jak ho předá most.
pub struct HttpReq<'a> {
    pub method: &'a str,
    pub authorization: Option<&'a str>,
    /// hlavička `MCP-Protocol-Version`
    pub protocol: Option<&'a str>,
    /// tělo; `None` = delší než `MAX_BODY`
    pub body: Option<&'a [u8]>,
}

/// Odpověď mostu. Prázdné tělo = bez `Content-Type`.
#[derive(Debug, PartialEq)]
pub struct HttpResp {
    pub code: u16,
    pub body: String,
    /// doplňková hlavička (`Allow`, `WWW-Authenticate`)
    pub header: Option<(&'static str, &'static str)>,
}

impl HttpResp {
    fn plain(code: u16) -> Self {
        Self { code, body: String::new(), header: None }
    }
}

/// Obsluha HTTP úrovně: vypnuto → 404, klíč → 401, metoda → 405, velikost → 413, verze → 400, pak JSON-RPC.
/// Pořadí je záměrné: vypnutý server o sobě nic neprozradí, bez klíče se nedozvíš ani to, co umí.
pub fn handle_http(srv: &Server, host: &dyn Host, req: &HttpReq) -> HttpResp {
    if !host.config().mcp.enabled {
        return HttpResp::plain(404);
    }
    if !srv.keys.check(req.authorization) {
        return HttpResp { code: 401, body: String::new(), header: Some(("WWW-Authenticate", "Bearer")) };
    }
    if req.method != "POST" {
        // bez SSE: GET (stream ze serveru) ani DELETE (konec session) nejsou
        return HttpResp { code: 405, body: String::new(), header: Some(("Allow", "POST")) };
    }
    let Some(body) = req.body else { return HttpResp::plain(413) };
    if let Some(v) = req.protocol
        && !rpc::VERSIONS.contains(&v.trim())
    {
        return HttpResp { code: 400, body: rpc::error_body(Value::Null, rpc::INVALID_REQUEST, &format!("Unsupported MCP-Protocol-Version: {v}")), header: None };
    }
    let text = match std::str::from_utf8(body) {
        Ok(t) => t,
        Err(_) => return HttpResp { code: 400, body: rpc::error_body(Value::Null, rpc::PARSE_ERROR, "Body is not UTF-8"), header: None },
    };
    let handler = tools::Handler { srv, host };
    match rpc::handle(text, &handler) {
        rpc::Outcome::Reply(b) => HttpResp { code: 200, body: b, header: None },
        rpc::Outcome::BadRequest(b) => HttpResp { code: 400, body: b, header: None },
        rpc::Outcome::Accepted => HttpResp::plain(202),
    }
}
