//! Příkaz karty Kalendář (Nastavení → Osobní → Kalendáře): jedno zkušební stažení odkazu ICS.
//! Logika v `calendar/live.rs`; nic se neukládá, adresa se neloguje.

use crate::calendar::live::{self, TestResult};

/// Stáhne a rozparsuje odkaz (`webcal://` i `https://`), vrátí počet událostí a název z feedu.
/// Chyba = lidská věta bez adresy.
#[tauri::command]
pub(crate) async fn calendar_test(url: String) -> Result<TestResult, String> {
    live::test(&url).await
}
