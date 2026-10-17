//! Lokální řízení přes most Claude (`claude::serve`): doplněk `/status` (`status_extra` — procesy,
//! cesta configu, stav šuplíku) a `/ctl` (`ctl_handler`: open, close, settings, launch, stop, reload).
//! Schvalování akcí Clauda tu záměrně není — povolit smí jen člověk v šuplíku.

use crate::commands::projects::launch;
use crate::drawer::{self, Opener, View};
use crate::settings::{apply_config, load_or_seed};
use crate::state::{UiCmd, core};
use crate::{claude, config};
use serde_json::{Value, json};
use std::sync::Arc;
use tauri::Emitter;

/// Doplněk odpovědi `/status`.
pub(crate) fn status_extra() -> Arc<dyn Fn() -> Value + Send + Sync> {
    Arc::new(|| {
        let c = core();
        let (open_ms, cold) = c.drawer.last_open();
        json!({ "procs": c.procs.snapshot(), "configPath": config::config_path(),
            "drawer": { "view": c.drawer.view(), "warm": c.drawer.is_warm(), "lastOpenMs": open_ms, "lastOpenCold": cold, "anim": c.drawer.anim_stats() } })
    })
}

/// Obsluha `/ctl/<cmd>` (tělo JSON).
pub(crate) fn ctl_handler() -> claude::Ctl {
    Arc::new(|cmd, body| {
        let c = core();
        match cmd {
            "open" => {
                let view = if body.get("view").and_then(Value::as_str) == Some("peek") { View::Peek } else { View::Full };
                c.send_ui(UiCmd::Show(view, Opener::Remote));
                Ok(json!({ "ok": true }))
            }
            "close" => {
                c.send_ui(UiCmd::Hide);
                Ok(json!({ "ok": true }))
            }
            "settings" => {
                c.send_ui(UiCmd::Show(View::Full, Opener::Remote));
                let _ = c.app.emit_to(drawer::LABEL, "page", "settings");
                Ok(json!({ "ok": true }))
            }
            "launch" => {
                let id = body.get("id").and_then(Value::as_str).ok_or("chybí id")?;
                launch(id.to_string()).map(|o| json!(o))
            }
            "stop" => {
                let id = body.get("id").and_then(Value::as_str).ok_or("chybí id")?;
                c.procs.stop(id);
                Ok(json!({ "ok": true }))
            }
            "reload" => {
                let (cfg, err) = load_or_seed();
                match err {
                    Some(e) => Err(e),
                    None => {
                        apply_config(cfg);
                        Ok(json!({ "ok": true }))
                    }
                }
            }
            _ => Err(format!("neznámý příkaz {cmd}")),
        }
    })
}
