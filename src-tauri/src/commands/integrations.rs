//! Tauri příkazy karet GitHub a Systém (`github/`, `sysmon/`). Tenké obálky; obnovu GitHubu hlídá
//! `github::refresh` (odstup, běžící dotaz), vzorkování systému `sysmon::poll` (nejkratší odstup).

use crate::github::{self, GithubView, Mode};
use crate::state::{UiCmd, core, mark_dirty, now_ms};
use crate::sysmon::{self, SystemView};

/// Vzorek systému pro kartu Systém (UI volá po 2 s, jen s otevřeným šuplíkem a viditelnou kartou).
/// `None` = čtení WinAPI selhalo.
#[tauri::command(async)]
pub(crate) fn system_view() -> Option<SystemView> {
    let cfg = core().cfg.read().system.clone();
    sysmon::poll(now_ms(), true, &cfg)
}

/// Klik na CPU / paměť: otevře Správce úloh. Šuplík se zavře jako po jiném spuštění aplikace.
#[tauri::command(async)]
pub(crate) fn system_taskmgr() -> Result<(), String> {
    sysmon::open_task_manager()?;
    core().send_ui(UiCmd::Hide);
    Ok(())
}

/// Data karty GitHub. Obnoví je jen, když to odstup dovolí (`force` = tlačítko Obnovit, nejdřív po 10 s);
/// jinak vrátí paměť. Blokuje až 15 s (dotazy `gh`), proto `async`.
#[tauri::command(async)]
pub(crate) fn github_view(force: bool) -> GithubView {
    let cfg = core().cfg.read().github.clone();
    let v = github::refresh(&cfg, if force { Mode::Force } else { Mode::Card });
    // proužek ukazuje počet „čeká na revizi“ z téhle paměti
    mark_dirty();
    v
}

/// Otevře PR / notifikaci v prohlížeči; jen `https://github.com/…`.
#[tauri::command]
pub(crate) fn github_open(url: String) -> Result<(), String> {
    if !github::open_ok(&url) {
        return Err(crate::i18n::tr("Otevřít jde jen odkaz na github.com.", "Only github.com links can be opened.").into());
    }
    super::drawer::open_url(url)?;
    // jako po jiném otevření odkazu (okno prohlížeče převezme fokus, šuplík se zavře)
    core().send_ui(UiCmd::Hide);
    Ok(())
}

/// „Přihlas se příkazem `gh auth login`“ → tlačítko Kopírovat. Přes Rust: šuplík nemá fokus a WebView
/// bez fokusu zápis do schránky odmítne.
#[tauri::command(async)]
pub(crate) fn github_copy_login() -> Result<(), String> {
    crate::shelf::clip::set_text("gh auth login")
}
