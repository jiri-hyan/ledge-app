//! „Poslat do {název}“: poznámku (nebo její `- [ ]` body) jedním klikem do propojené aplikace přes její
//! MCP server jako poznámku / úkoly. Adresa a název jsou jen z configu (výchozí prázdné = funkce
//! nenastavená). Mapa: `docs/codemap/connector.md`.
//!
//! - `plan` — čistá funkce: poznámka → plán odeslání (body, řešitel, priorita, idempotenční id).
//! - `client` — JSON-RPC `tools/call` přes HTTPS (MCP, Streamable HTTP bez session), Bearer klíč.
//! - `key` — klíč ve Správci pověření (`Ledge/connector`), nikdy v configu, logu ani hlášce.
//! - `link` — zápis odkazu na založený úkol zpět do poznámky.
//!
//! Bez výslovného potvrzení v náhledu se nic neodešle (outward akce). Žádný časovač, žádná síť,
//! dokud uživatel neklikne.

pub mod client;
pub mod key;
pub mod link;
pub mod plan;

#[cfg(test)]
mod tests;

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Nejdelší zobrazený název propojené aplikace (znaků).
const NAME_MAX: usize = 60;

/// Sekce configu `connector`. Klíč tu není (Správce pověření `Ledge/connector`). Vše prázdné = nenastaveno.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectorConfig {
    /// Adresa MCP serveru propojené aplikace (https; pro vývoj i `http://127.0.0.1…`). Prázdná = nenastaveno.
    pub url: String,
    /// Adresa webu propojené aplikace pro odkaz na založený úkol v poznámce (`https://…`). Prázdná = řádek bez odkazu.
    pub app_url: String,
    /// Zobrazovaný název propojené aplikace („Poslat do {název}“). Prázdný = „propojená aplikace“.
    pub name: String,
}

impl ConnectorConfig {
    /// Adresa MCP bez okrajových mezer; prázdná = `None` (funkce nenastavená).
    pub fn endpoint(&self) -> Option<&str> {
        Some(self.url.trim()).filter(|u| !u.is_empty())
    }

    /// Adresa webu bez koncového `/`; prázdná = `None` (do poznámky řádek bez odkazu).
    pub fn app_base(&self) -> Option<&str> {
        Some(self.app_url.trim().trim_end_matches('/')).filter(|u| !u.is_empty())
    }

    /// Název k zobrazení: oříznutý, jednořádkový, nejvýš `NAME_MAX` znaků; může být prázdný.
    pub fn display_name(&self) -> String {
        let one: String = self.name.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        one.trim().chars().take(NAME_MAX).collect::<String>().trim().to_string()
    }

    /// Je adresa MCP zadaná a platná (smí se volat)?
    pub fn configured(&self) -> bool {
        self.endpoint().is_some_and(url_ok)
    }
}

/// Smí to být adresa propojené aplikace? `https://host[:port][/cesta]`; `http://` jen pro loopback (vývoj, testy).
/// Žádné přihlašovací údaje v adrese, žádné mezery.
pub fn url_ok(u: &str) -> bool {
    let u = u.trim();
    if u.is_empty() {
        return true;
    }
    if u.len() > 300 || u.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let (https, rest) = match (u.strip_prefix("https://"), u.strip_prefix("http://")) {
        (Some(r), _) => (true, r),
        (_, Some(r)) => (false, r),
        _ => return false,
    };
    let auth = rest.split(['/', '?', '#']).next().unwrap_or("");
    if auth.is_empty() || auth.contains('@') {
        return false;
    }
    let host = if let Some(v6) = auth.strip_prefix('[') { v6.split(']').next().unwrap_or("") } else { auth.split(':').next().unwrap_or("") };
    https || matches!(host, "127.0.0.1" | "localhost" | "::1")
}

pub fn validate(c: &ConnectorConfig) -> Result<(), FieldError> {
    if !url_ok(&c.url) {
        return Err(fe("connector.url", tr("connector.url: adresa MCP serveru začíná https:// (http:// jen pro 127.0.0.1) a nesmí mít mezery ani přihlášení.", "connector.url: the MCP server address starts with https:// (http:// only for 127.0.0.1) and has no spaces or credentials.")));
    }
    if !url_ok(&c.app_url) {
        return Err(fe("connector.appUrl", tr("connector.appUrl: adresa webu začíná https:// (http:// jen pro 127.0.0.1).", "connector.appUrl: the web address starts with https:// (http:// only for 127.0.0.1).")));
    }
    Ok(())
}
