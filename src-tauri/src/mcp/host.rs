//! Skutečný `Host` nad běžícím Ledge (`state::core`) a endpoint pro most. Netestuje se přímo (sahá
//! na Správce pověření, disk a okno) — logika je v čistých modulech, testy jedou nad `tests::MemHost`.

use super::{Host, HttpReq, HttpResp, Server};
use crate::config::Config;
use crate::drawer::{Opener, View};
use crate::i18n::tr;
use crate::state::{UiCmd, core};
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;

static SERVER: OnceLock<Server> = OnceLock::new();

/// Stav serveru MCP (klíč ze Správce pověření, Vrátit, zámek zápisů) — jeden za proces.
pub fn server() -> &'static Server {
    SERVER.get_or_init(|| Server::new(Box::new(crate::accounts::vault::WinCred)))
}

/// Obsluha `/mcp` pro most (`claude::serve`).
pub fn endpoint() -> crate::claude::Mcp {
    Arc::new(|req: &HttpReq| -> HttpResp { super::handle_http(server(), &LiveHost, req) })
}

/// Událost pro šuplík po zápisu mimo Nastavení — Nastavení se přenačte, nebo ukáže konflikt
/// (`src/mcp/merge.ts`). Stav šuplíku se obnoví sám (`apply_config` → `mark_dirty`).
pub const EVENT: &str = "config-changed";

pub fn notify_ui(source: &str) {
    let _ = core().app.emit_to(crate::drawer::LABEL, EVENT, json!({ "source": source }));
}

pub struct LiveHost;

impl Host for LiveHost {
    fn config(&self) -> Config {
        core().cfg.read().clone()
    }

    fn save(&self, cfg: Config) -> Result<(), String> {
        // přesně tlačítko Uložit v Nastavení: `config::save` (validace, atomický zápis) + `apply_config`
        crate::commands::config::save_config(cfg)?;
        notify_ui("mcp");
        Ok(())
    }

    fn version(&self) -> String {
        crate::update::CURRENT.to_string()
    }

    fn drawer_open(&self) -> bool {
        core().drawer.view() != View::Hidden
    }

    fn open_drawer(&self, tab: Option<&str>) -> Result<String, String> {
        let c = core();
        if c.dead() {
            return Err(tr("Ledge je teď schovaný (hra na celé obrazovce nebo herní režim) — šuplík se neotevře.", "Ledge is hidden right now (a fullscreen game or game mode) — the drawer won't open.").into());
        }
        let mut on = String::new();
        if let Some(t) = tab {
            let cfg = c.cfg.read().clone();
            let Some(p) = cfg.layout.pages.iter().find(|p| p.id == t) else {
                let ids: Vec<&str> = cfg.layout.pages.iter().map(|p| p.id.as_str()).collect();
                return Err(trf!("Záložka „{t}“ není (jsou: {}).", "There's no tab “{t}” (there are: {}).", ids.join(", ")));
            };
            on = crate::layout::title(p);
            crate::commands::drawer::set_mode(t.to_string());
        }
        // `Remote` = bez fokusu a bez samozavírání (jako `/ctl/open`); ve hře ho vlákno `ui` zahodí
        c.send_ui(UiCmd::Show(View::Full, Opener::Remote));
        Ok(if on.is_empty() { tr("Šuplík je otevřený.", "The drawer is open.").into() } else { trf!("Šuplík je otevřený na záložce {on}.", "The drawer is open on the {on} tab.") })
    }

    fn discover(&self, repos: bool) -> Value {
        let apps = crate::commands::onboarding::scan_apps();
        let found = if repos { crate::commands::onboarding::scan_repos(&[]) } else { Vec::new() };
        json!({
            "apps": apps.iter().take(300).map(|a| {
                let mut v = json!({ "name": a.name, "exe": a.exe, "alreadyInLedge": a.already });
                if !a.args.is_empty() {
                    v["args"] = json!(a.args);
                }
                v
            }).collect::<Vec<_>>(),
            "repos": found.iter().take(200).map(|r| json!({
                "name": r.name, "path": r.path, "remote": r.remote, "kind": r.kind, "hint": r.hint,
                "lastActiveMs": r.last_ms, "alreadyInLedge": r.already, "suggestion": r.project,
            })).collect::<Vec<_>>(),
        })
    }
}
