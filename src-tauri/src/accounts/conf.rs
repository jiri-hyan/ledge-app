//! Config účtů (`accounts`). Jen to, co tajné není: Client ID vlastní registrace aplikace, tenant,
//! co Ledge z účtu čte, barva a vybrané kalendáře Google. Přihlášení samo (refresh token, u Google
//! i client secret) je ve Správci pověření (`vault.rs`). Výchozí = nic nepřihlášeno, žádná síť.

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsConfig {
    /// Microsoft (Outlook, Microsoft 365): kalendář a počet nepřečtených v poště přes Microsoft Graph.
    pub microsoft: MicrosoftAccount,
    /// Google: jen čtení kalendáře (Google Calendar API).
    pub google: GoogleAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MicrosoftAccount {
    /// Application (client) ID z Azure Portal → App registrations (GUID). Prázdné = nenastaveno;
    /// Ledge žádné výchozí nemá (`docs/ucty.md`).
    pub client_id: String,
    /// Kdo se smí přihlásit: `common` (osobní i pracovní účty, výchozí), `organizations`, `consumers`
    /// nebo ID / doména tenantu.
    pub tenant: String,
    /// Události z kalendáře účtu v kartě Kalendář.
    pub calendar: bool,
    /// Pošta: počet nepřečtených v Doručené poště a 5 nejnovějších předmětů (karta Pošta, položka proužku `mail`).
    pub mail: bool,
    /// Barva kalendáře účtu `#rrggbb`; `null` = výchozí modrá.
    pub color: Option<String>,
}

impl Default for MicrosoftAccount {
    fn default() -> Self {
        Self { client_id: String::new(), tenant: "common".into(), calendar: true, mail: true, color: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GoogleAccount {
    /// Client ID z Google Cloud Console → OAuth client typu „Desktop app“ (`….apps.googleusercontent.com`).
    /// Client secret k němu se ukládá jen do Správce pověření Windows.
    pub client_id: String,
    /// Události z kalendáře účtu v kartě Kalendář.
    pub calendar: bool,
    /// Id kalendářů, které se mají ukázat; prázdné = jen hlavní (`primary`). Nejvýš 10.
    pub calendars: Vec<String>,
    /// Barva kalendáře účtu `#rrggbb`; `null` = výchozí zelená.
    pub color: Option<String>,
}

impl Default for GoogleAccount {
    fn default() -> Self {
        Self { client_id: String::new(), calendar: true, calendars: Vec::new(), color: None }
    }
}

pub const MAX_GOOGLE_CALENDARS: usize = 10;

/// Microsoft Client ID = GUID (8-4-4-4-12 hex).
pub fn ms_client_ok(id: &str) -> bool {
    let p: Vec<&str> = id.split('-').collect();
    p.len() == 5 && p.iter().zip([8, 4, 4, 4, 12]).all(|(s, n)| s.len() == n && s.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Google Client ID: `<číslo>-<znaky>.apps.googleusercontent.com`.
pub fn google_client_ok(id: &str) -> bool {
    id.strip_suffix(".apps.googleusercontent.com")
        .is_some_and(|h| !h.is_empty() && h.len() <= 120 && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

/// Tenant: `common` / `organizations` / `consumers`, GUID nebo doména.
pub fn tenant_ok(t: &str) -> bool {
    (1..=64).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Client secret Google: neprázdný jeden řádek bez mezer.
pub fn secret_ok(s: &str) -> bool {
    (1..=200).contains(&s.len()) && !s.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn color_ok(c: &Option<String>) -> bool {
    c.as_ref().is_none_or(|c| c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()))
}

pub fn validate(a: &AccountsConfig) -> Result<(), FieldError> {
    let m = &a.microsoft;
    if !m.client_id.trim().is_empty() && !ms_client_ok(m.client_id.trim()) {
        return Err(fe(
            "accounts.microsoft.clientId",
            tr(
                "accounts.microsoft.clientId: Application (client) ID má tvar 00000000-0000-0000-0000-000000000000 (najdeš ho v Azure Portal v přehledu registrace).",
                "accounts.microsoft.clientId: the Application (client) ID looks like 00000000-0000-0000-0000-000000000000 (it's on the registration's Overview in Azure Portal).",
            ),
        ));
    }
    if !tenant_ok(m.tenant.trim()) {
        return Err(fe("accounts.microsoft.tenant", tr("accounts.microsoft.tenant: common, organizations, consumers nebo ID tenantu.", "accounts.microsoft.tenant: common, organizations, consumers or a tenant ID.")));
    }
    let g = &a.google;
    if !g.client_id.trim().is_empty() && !google_client_ok(g.client_id.trim()) {
        return Err(fe(
            "accounts.google.clientId",
            tr("accounts.google.clientId: Client ID končí na .apps.googleusercontent.com.", "accounts.google.clientId: the Client ID ends in .apps.googleusercontent.com."),
        ));
    }
    if g.calendars.len() > MAX_GOOGLE_CALENDARS || g.calendars.iter().any(|c| c.is_empty() || c.len() > 256) {
        return Err(fe("accounts.google.calendars", trf!("accounts.google.calendars: nejvýš {MAX_GOOGLE_CALENDARS} kalendářů.", "accounts.google.calendars: at most {MAX_GOOGLE_CALENDARS} calendars.")));
    }
    for (path, c) in [("accounts.microsoft.color", &m.color), ("accounts.google.color", &g.color)] {
        if !color_ok(c) {
            return Err(fe(path, trf!("{path}: barva musí být #rrggbb.", "{path}: the colour must be #rrggbb.")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_table() {
        assert!(ms_client_ok("11111111-2222-3333-4444-55555555abcd"));
        for bad in ["", "1111-2222", "11111111-2222-3333-4444-55555555abcz", "{11111111-2222-3333-4444-555555555555}"] {
            assert!(!ms_client_ok(bad), "{bad}");
        }
        assert!(google_client_ok("123456789012-abcdefghijklmnop0123456789abcdef.apps.googleusercontent.com"));
        for bad in ["", ".apps.googleusercontent.com", "123.apps.example.com", "a b.apps.googleusercontent.com"] {
            assert!(!google_client_ok(bad), "{bad}");
        }
        for ok in ["common", "organizations", "consumers", "contoso.onmicrosoft.com", "11111111-2222-3333-4444-555555555555"] {
            assert!(tenant_ok(ok), "{ok}");
        }
        assert!(!tenant_ok("") && !tenant_ok("a/b") && !tenant_ok("a b"));
        assert!(secret_ok("GOCSPX-abc_DEF") && !secret_ok("") && !secret_ok("a b"));
    }

    #[test]
    fn validate_table_and_old_config() {
        let ok = AccountsConfig::default();
        assert!(validate(&ok).is_ok());
        let path = |a: AccountsConfig| validate(&a).err().and_then(|e| e.field);
        let mut a = ok.clone();
        a.microsoft.client_id = "nesmysl".into();
        assert_eq!(path(a).as_deref(), Some("accounts.microsoft.clientId"));
        let mut a = ok.clone();
        a.microsoft.tenant = "".into();
        assert_eq!(path(a).as_deref(), Some("accounts.microsoft.tenant"));
        let mut a = ok.clone();
        a.google.client_id = "x.apps.example.com".into();
        assert_eq!(path(a).as_deref(), Some("accounts.google.clientId"));
        let mut a = ok.clone();
        a.google.calendars = vec!["x".into(); MAX_GOOGLE_CALENDARS + 1];
        assert_eq!(path(a).as_deref(), Some("accounts.google.calendars"));
        let mut a = ok.clone();
        a.google.color = Some("zelená".into());
        assert_eq!(path(a).as_deref(), Some("accounts.google.color"));
        // starý config bez `accounts` / s částečným tvarem = výchozí (nic nepřihlášeno, čtení zapnuté)
        let a: AccountsConfig = serde_json::from_str(r#"{"microsoft":{"clientId":"11111111-2222-3333-4444-555555555555"}}"#).unwrap();
        assert_eq!((a.microsoft.tenant.as_str(), a.microsoft.mail, a.google.calendar), ("common", true, true));
    }
}
