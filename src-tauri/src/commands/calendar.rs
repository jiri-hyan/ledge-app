//! Příkazy karty Kalendář: zkušební stažení odkazu ICS (Nastavení → Osobní → Kalendáře), Připojit ke
//! schůzce a klasický Outlook v tomto PC (zjištění, načtení po výslovném kliku). Logika v `calendar/*`;
//! nic se neukládá, adresy se nelogují.

use crate::calendar::live::{self, TestResult};
use crate::calendar::{meeting, outlook};
use crate::i18n::tr;
use serde::Serialize;
use tauri_plugin_opener::OpenerExt;

/// Stáhne a rozparsuje odkaz (`webcal://` i `https://`), vrátí počet událostí a název z feedu.
/// Chyba = lidská věta bez adresy.
#[tauri::command]
pub(crate) async fn calendar_test(url: String) -> Result<TestResult, String> {
    live::test(&url).await
}

/// Klik na Připojit: otevře odkaz na schůzku přes systém (Teams v aplikaci, když je to bezpečné —
/// `meeting::launch_uri`). Jen odkaz známé služby (`meeting::classify`), nic jiného se neotevře.
#[tauri::command]
pub(crate) fn calendar_join(url: String) -> Result<(), String> {
    let Some(j) = meeting::classify(&url) else {
        return Err(tr("Tohle není odkaz na schůzku Teams, Meet, Zoom, Webex ani Jitsi.", "This isn't a Teams, Meet, Zoom, Webex or Jitsi meeting link.").into());
    };
    let opener = crate::state::core().app.opener();
    let uri = meeting::launch_uri(&j);
    // aplikace Teams odmítla protokol → prohlížeč
    opener.open_url(&uri, None::<&str>).or_else(|_| opener.open_url(&j.url, None::<&str>)).map_err(|e| {
        crate::state::log(&format!("kalendář: Připojit ({e})"));
        tr("Odkaz na schůzku se nepodařilo otevřít.", "Couldn't open the meeting link.").to_string()
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutlookProbe {
    /// klasický Outlook je nainstalovaný (registr, nic se nespouští)
    installed: bool,
    /// `OUTLOOK.EXE` právě běží
    running: bool,
}

/// Nastavení → Kalendáře → „Outlook v tomto PC“: je klasický Outlook? (nic nespouští)
#[tauri::command(async)]
pub(crate) fn calendar_outlook_probe() -> OutlookProbe {
    OutlookProbe { installed: outlook::installed(), running: outlook::running() }
}

/// Výslovný klik „Načíst z Outlooku“: smí Outlook spustit na pozadí; vrátí počet nadcházejících výskytů.
#[tauri::command]
pub(crate) async fn calendar_outlook_load() -> Result<usize, String> {
    live::load_outlook().await
}
