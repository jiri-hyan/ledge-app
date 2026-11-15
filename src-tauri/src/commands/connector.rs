//! Tauri příkazy „Poslat do {název}“ (kontrakt `src/connector/api.ts`): stav a klíč (Správce pověření
//! `Ledge/connector`, nikdy do UI, configu ani logu), náhled odeslání a odeslání. Nic nejde ven bez
//! výslovného `connector_send` po náhledu; náhled sám jen čte seznam členů týmu.

use crate::accounts::vault::WinCred;
use crate::i18n::tr;
use crate::connector::plan::{self, Mode, Plan};
use crate::connector::{client, key, link};
use crate::notes;
use crate::state::{core, log};
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectorState {
    /// klíč je uložen (hodnota se nikdy nevrací)
    has_key: bool,
    /// kam se bude posílat (host, bez cesty; prázdný, když adresa není) — ukáže to náhled i Nastavení
    host: String,
    /// zobrazovaný název z configu (oříznutý, může být prázdný)
    name: String,
    /// adresa MCP je zadaná a platná
    configured: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Preview {
    plan: Plan,
    /// otisk poznámky při náhledu — `connector_send` ho chce zpět (poznámka se mezitím nezměnila)
    hash: String,
    host: String,
    /// zobrazovaný název z configu (jako `ConnectorState::name`)
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendOut {
    results: Vec<client::Outcome>,
    /// odkazy se do poznámky nepodařilo zapsat (úkoly v propojené aplikaci jsou založené)
    link_error: Option<String>,
}

fn host_of(url: &str) -> String {
    url.split("://").nth(1).and_then(|r| r.split(['/', '?', '#']).next()).unwrap_or("").to_string()
}

/// Adresa MCP a (volitelně) webu z configu; nenastavená / neplatná adresa = hláška pro uživatele.
pub(crate) fn endpoint() -> Result<(String, Option<String>), String> {
    let cfg = core().cfg.read();
    let Some(url) = cfg.connector.endpoint() else {
        return Err(tr("Nejdřív nastav adresu propojené aplikace: Nastavení → Claude.", "Set the linked app address first: Settings → Claude.").into());
    };
    if !crate::connector::url_ok(url) {
        return Err(tr("Adresa propojené aplikace v configu (connector.url) není platná.", "The linked app address in the config (connector.url) isn't valid.").into());
    }
    Ok((url.to_string(), cfg.connector.app_base().map(str::to_string)))
}

fn name_of() -> String {
    core().cfg.read().connector.display_name()
}

pub(crate) fn need_key() -> Result<String, String> {
    key::get(&WinCred).ok_or_else(|| tr("Nejdřív ulož klíč propojené aplikace: Nastavení → Claude.", "Save the linked app key first: Settings → Claude.").to_string())
}

fn off() -> String {
    tr("Poznámky jsou vypnuté.", "Notes are turned off.").into()
}

#[tauri::command]
pub(crate) fn connector_state() -> ConnectorState {
    let cfg = core().cfg.read().connector.clone();
    let configured = cfg.configured();
    let host = if configured { host_of(cfg.endpoint().unwrap_or("")) } else { String::new() };
    ConnectorState { has_key: key::get(&WinCred).is_some(), host, name: cfg.display_name(), configured }
}

#[tauri::command]
pub(crate) fn connector_set_key(key: String) -> Result<(), String> {
    key::set(&WinCred, &key)?;
    log("connector: klíč uložen do Správce pověření");
    Ok(())
}

#[tauri::command]
pub(crate) fn connector_clear_key() -> Result<(), String> {
    key::clear(&WinCred)?;
    log("connector: klíč smazán");
    Ok(())
}

/// Plán odeslání poznámky. Síť: jen čtení seznamu členů (čtení). `mode` vynutí poznámku / úkoly.
#[tauri::command]
pub(crate) async fn connector_preview(path: String, mode: Option<Mode>) -> Result<Preview, String> {
    let (url, _) = endpoint()?;
    let key = need_key()?;
    let root = notes_root().ok_or_else(off)?;
    let d = notes::edit::read(&root, Path::new(&path))?;
    let members = client::list_members(&url, &key).await?;
    let mut plan = plan::plan(&path, &d.view.title, &d.body, &members, mode);
    let file = notes::check_note_path(&root, Path::new(&path))?;
    plan.note_sent = std::fs::read_to_string(file).ok().and_then(|t| link::connector_id(&t)).is_some();
    Ok(Preview { plan, hash: d.hash, host: host_of(&url), name: name_of() })
}

/// Odešle vybrané body (`lines`; u poznámky se nepoužijí) a zapíše odkazy zpět do poznámky.
#[tauri::command]
pub(crate) async fn connector_send(path: String, hash: String, mode: Option<Mode>, lines: Vec<usize>) -> Result<SendOut, String> {
    let (url, app) = endpoint()?;
    let key = need_key()?;
    let root = notes_root().ok_or_else(off)?;
    let d = notes::edit::read(&root, Path::new(&path))?;
    if d.hash != hash {
        return Err(tr("Poznámka se mezitím změnila — otevři náhled znovu.", "The note changed meanwhile — open the preview again.").into());
    }
    let members = client::list_members(&url, &key).await?;
    let plan = plan::plan(&path, &d.view.title, &d.body, &members, mode);
    if plan.mode == Mode::Note {
        let file = notes::check_note_path(&root, Path::new(&path))?;
        if std::fs::read_to_string(file).ok().and_then(|t| link::connector_id(&t)).is_some() {
            return Err(tr("Tahle poznámka už byla odeslána (connector: ve frontmatteru).", "This note was already sent (connector: in the frontmatter).").into());
        }
        if d.body.trim().is_empty() || d.body.chars().count() > client::MAX_NOTE {
            return Err(tr("Poznámka je prázdná nebo moc dlouhá (nejvýš 100 000 znaků).", "The note is empty or too long (100,000 characters at most).").into());
        }
    } else if !plan.items.iter().any(|i| !i.sent && lines.contains(&i.line)) {
        return Err(tr("Není co odeslat — vyber aspoň jeden bod, který ještě odeslaný není.", "Nothing to send — pick at least one item that hasn't been sent yet.").into());
    }
    let results = client::send(&url, &key, &plan, &d.body, &lines, app.as_deref().unwrap_or("")).await;
    log(&format!("connector: odesláno {} z {} ({})", results.iter().filter(|o| o.ok).count(), results.len(), if plan.mode == Mode::Note { "poznámka" } else { "úkoly" }));
    // odkazy zpět do poznámky; selhání (poznámka se mezitím změnila) úkoly neruší
    let ok: Vec<client::Outcome> = results.iter().filter(|o| o.ok).cloned().collect();
    let mut link_error = None;
    let mut missed = 0usize;
    if !ok.is_empty() {
        let note_id = ok[0].id.clone();
        let name = name_of();
        let r = notes::ops::rewrite(&root, Path::new(&path), &notes::now_local(), |front, rest| {
            if plan.mode == Mode::Note {
                if let Some(id) = &note_id {
                    notes::set(front, "connector", id);
                }
                Ok(None)
            } else {
                let (b, n, miss) = link::add_links(rest, &ok, &name);
                missed = miss.len();
                Ok((n > 0).then_some(b))
            }
        });
        match r {
            Ok(_) => {
                reload_notes();
                send_sync(SyncMsg::Local);
            }
            Err(e) => link_error = Some(e),
        }
        if link_error.is_none() && missed > 0 {
            link_error = Some(trf!(
                "Úkoly jsou odeslané, ale odkaz se nepodařilo zapsat u {missed} bodů (poznámka se mezitím změnila). Při dalším odeslání se nezdvojí.",
                "The tasks were sent, but the link couldn't be written for {missed} items (the note changed meanwhile). Sending again won't duplicate them."
            ));
        }
    }
    Ok(SendOut { results, link_error })
}
