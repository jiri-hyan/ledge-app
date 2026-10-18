//! Příkazy Nastavení → Účty: stav, připojení (přihlášení v systémovém prohlížeči), zrušení čekání,
//! odhlášení, seznam kalendářů Google. Logika v `accounts/*`; tajemství do UI nejdou.

use crate::accounts::{Provider, session};
use crate::i18n::tr;

fn provider(id: &str) -> Result<Provider, String> {
    Provider::parse(id).ok_or_else(|| tr("Neznámý účet.", "Unknown account.").to_string())
}

/// Stav obou účtů (přihlášen jako…, oprávnění, chyba). Čte Správce pověření jen poprvé.
#[tauri::command]
pub(crate) async fn accounts_status() -> Vec<session::AccountView> {
    tauri::async_runtime::spawn_blocking(session::status).await.unwrap_or_default()
}

/// Připojí účet: otevře přihlášení v prohlížeči a čeká (nejvýš 5 min). `clientSecret` jen Google.
#[tauri::command]
pub(crate) async fn account_connect(provider_id: String, client_id: String, client_secret: Option<String>, tenant: Option<String>) -> Result<session::AccountView, String> {
    let p = provider(&provider_id)?;
    session::connect(p, client_id, client_secret.unwrap_or_default(), tenant.unwrap_or_default()).await
}

#[tauri::command]
pub(crate) fn account_cancel(provider_id: String) -> Result<(), String> {
    session::cancel(provider(&provider_id)?);
    Ok(())
}

/// Odhlásí: smaže přihlášení ze Správce pověření (Google ho i odvolá).
#[tauri::command]
pub(crate) async fn account_disconnect(provider_id: String) -> Result<(), String> {
    session::disconnect(provider(&provider_id)?).await
}

/// Kalendáře přihlášeného účtu Google pro výběr v Nastavení.
#[tauri::command]
pub(crate) async fn account_google_calendars() -> Result<Vec<crate::calendar::google::CalendarEntry>, String> {
    crate::calendar::google::calendar_list().await
}
