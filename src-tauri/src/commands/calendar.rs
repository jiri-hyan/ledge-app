//! Příkazy karty Kalendář: zkušební stažení odkazu ICS (Nastavení → Osobní → Kalendáře), Připojit ke
//! schůzce a klasický Outlook v tomto PC (zjištění, načtení po výslovném kliku). Logika v `calendar/*`;
//! nic se neukládá, adresy se nelogují.

use crate::calendar::live::{self, TestResult};
use crate::calendar::{meeting, outlook};
use crate::i18n::tr;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AddTarget {
    /// soubor `.ics` otevřený ve výchozím kalendáři
    Ics,
    /// kalendář propojené aplikace (`create_event`)
    Connector,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalAdd {
    title: String,
    /// `YYYY-MM-DD` (celodenní) | `YYYY-MM-DDTHH:MM` (místní čas) | ISO s offsetem
    start_iso: String,
    dur_min: Option<u32>,
    remind_min: Option<u32>,
    #[serde(default)]
    all_day: bool,
    /// semeno stabilního `UID` (cesta poznámky + zdroj + název, bez času: odložení = aktualizace `.ics`); do
    /// `clientRequestId` propojené aplikace se přidává čas (nemá aktualizaci, nový čas = nová událost, stejný klik = tatáž)
    uid: String,
    target: AddTarget,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalAddOut {
    target: &'static str,
    /// id události v propojené aplikaci (u `.ics` nic)
    id: Option<String>,
    /// aplikace ji už znala (opakovaný klik)
    duplicate: bool,
}

/// Starší `.ics` v dočasné složce se při dalším zápisu smažou (soubor je potřeba jen chvíli, než ho kalendář naimportuje).
const ICS_KEEP_SECS: u64 = 24 * 3600;

fn write_ics(name: &str, text: &str) -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join("Ledge-ics");
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::any(tr("Složku pro událost nejde založit", "Can't create the folder for the event"), &e))?;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for f in rd.flatten() {
            let old = f.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() > ICS_KEEP_SECS);
            if old && f.path().extension().is_some_and(|e| e == "ics") {
                let _ = std::fs::remove_file(f.path());
            }
        }
    }
    let file = dir.join(name);
    std::fs::write(&file, text).map_err(|e| crate::errors::any(tr("Událost nejde uložit", "Can't save the event"), &e))?;
    Ok(file)
}

/// „Do kalendáře“: `ics` zapíše soubor do dočasné složky Ledge a otevře ho výchozí aplikací (otevře se okno cizí
/// aplikace — volá se jen na výslovný klik); `connector` pošle `create_event` s klíčem ze Správce pověření
/// (jen po potvrzení v UI). Žádné nové oprávnění; stejné `uid` = stejná událost.
#[tauri::command]
pub(crate) async fn calendar_add(req: CalAdd) -> Result<CalAddOut, String> {
    use crate::calendar::{tz::SystemLocal, write};
    let title = write::clean_title(&req.title).map_err(|_| tr("Událost potřebuje název.", "The event needs a title.").to_string())?;
    let iso = if req.all_day { req.start_iso.trim().chars().take(10).collect::<String>() } else { req.start_iso.clone() };
    let start = write::parse_start(&iso, &SystemLocal).map_err(|_| tr("Neplatný čas události.", "Invalid event time.").to_string())?;
    if req.uid.trim().is_empty() {
        return Err(tr("Chybí klíč události.", "The event key is missing.").into());
    }
    let ev = write::Ev {
        title: &title,
        start,
        dur_min: write::clamp_dur(req.dur_min.unwrap_or(write::DEFAULT_DUR_MIN)),
        remind_min: write::clamp_remind(req.remind_min.unwrap_or(write::DEFAULT_REMIND_MIN)),
        uid: &write::uid(&req.uid),
    };
    match req.target {
        AddTarget::Ics => {
            let text = write::ics_event(&ev, chrono::Utc::now().naive_utc());
            let file = write_ics(&write::file_name(&req.uid), &text)?;
            let c = crate::state::core();
            c.app.opener().open_path(file.to_string_lossy(), None::<&str>).map_err(|e| crate::errors::any(tr("Kalendář nejde otevřít", "Can't open the calendar"), &e))?;
            c.send_ui(crate::state::UiCmd::Hide);
            Ok(CalAddOut { target: "ics", id: None, duplicate: false })
        }
        AddTarget::Connector => {
            let (url, _) = super::connector::endpoint()?;
            let key = super::connector::need_key()?;
            let args = write::create_event_args(&ev, tr("Z poznámky v Ledge", "From a note in Ledge"), &write::request_id(&format!("{}|{}", req.uid, req.start_iso)));
            let v = crate::connector::client::call(&url, &key, "create_event", &args).await.map_err(|e| {
                if e.contains("create_event") {
                    tr("Klíč propojené aplikace nesmí zapisovat události — vytvoř klíč s právem zápisu událostí.", "The linked app key can't write events — create a key that may write events.").to_string()
                } else {
                    e
                }
            })?;
            let id = v.get("eventId").and_then(|x| x.as_str()).map(str::to_string).ok_or_else(|| tr("Propojená aplikace nevrátila id události.", "The linked app returned no event id.").to_string())?;
            crate::state::log("kalendář: událost odeslána do propojené aplikace");
            Ok(CalAddOut { target: "connector", id: Some(id), duplicate: v.get("deduplicated").and_then(|x| x.as_bool()).unwrap_or(false) })
        }
    }
}
