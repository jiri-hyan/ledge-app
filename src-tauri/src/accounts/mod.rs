//! Účty Microsoft a Google (OAuth 2.0) — základ pro kalendář, poštu a další služby.
//!
//! Ledge je veřejná open-source appka **bez vlastního ověřeného OAuth klienta**: uživatel si přinese
//! vlastní registraci aplikace (Client ID; u Google i „client secret“, který u desktopových klientů
//! tajný není). Výchozí Client ID je prázdné — žádné se nevymýšlí (`docs/ucty.md`).
//!
//! Tok: Authorization Code + PKCE (`pkce.rs`) s jednorázovým loopback serverem na `127.0.0.1`
//! s náhodným portem a termínem (`loopback.rs`), přihlášení v systémovém prohlížeči (ne ve WebView).
//! Refresh token jen ve Windows Credential Manageru (`vault.rs`), access token jen v paměti
//! (`session.rs`), automatická obnova, odhlášení = smazání + revokace (Google; Microsoft ji pro
//! veřejné klienty nemá). Nic z toho do configu ani logu — chyby prochází `oauth::scrub`.
//!
//! Data: kalendář (`calsync.rs` → adaptéry `calendar/graph.rs`, `calendar/google.rs`) a pošta
//! Microsoft (`mail.rs`, karta Pošta, položka proužku `mail`). Síť jen po přihlášení a jen když je
//! karta vidět (pošta v proužku: nejvýš 1× za 5 min).

pub mod calsync;
pub mod conf;
pub mod http;
pub mod loopback;
pub mod mail;
pub mod oauth;
pub mod pkce;
pub mod session;
pub mod vault;

use serde::{Deserialize, Serialize};

/// Poskytovatel účtu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    Microsoft,
    Google,
}

impl Provider {
    pub const ALL: [Provider; 2] = [Provider::Microsoft, Provider::Google];

    pub fn id(self) -> &'static str {
        match self {
            Provider::Microsoft => "microsoft",
            Provider::Google => "google",
        }
    }

    pub fn parse(s: &str) -> Option<Provider> {
        Self::ALL.into_iter().find(|p| p.id() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Microsoft => "Microsoft",
            Provider::Google => "Google",
        }
    }
}

/// Minimální oprávnění Microsoft Graph: obnova bez přihlášení, jméno účtu, čtení kalendáře,
/// základní údaje pošty (předmět, odesílatel — bez těla a příloh).
pub const MS_SCOPES: &str = "offline_access User.Read Calendars.Read Mail.ReadBasic";
/// Google: jen čtení kalendáře. Gmail scopes jsou „restricted“ (bezpečnostní audit) — Ledge je nemá.
pub const GOOGLE_SCOPES: &str = "https://www.googleapis.com/auth/calendar.readonly";

pub const GRAPH: &str = "https://graph.microsoft.com/v1.0";
pub const GCAL: &str = "https://www.googleapis.com/calendar/v3";
