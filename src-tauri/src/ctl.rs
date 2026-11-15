//! Lokální řízení přes most Claude (`claude::serve`): doplněk `/status` (`status_extra` — procesy jen s povolenými poli,
//! stav šuplíku) a `/ctl` (`ctl_handler`: open, close, settings, launch, stop, reload).
//! Schvalování akcí Clauda tu záměrně není — povolit smí jen člověk v šuplíku.

use crate::commands::projects::launch;
use crate::drawer::{self, Opener, View};
use crate::settings::{apply_config, reload_from_disk};
use crate::state::{UiCmd, core};
use crate::claude;
use serde_json::{Value, json};
use std::sync::Arc;
use tauri::Emitter;

/// Pole procesu, která `/status` smí ukázat (povolený seznam, L247): bez `cmd`, `cwd`, `message`, `lastLine`, `tail`
/// (cesty a výstup procesů). Nové pole `ProcView` je tedy skryté, dokud ho někdo nepřidá sem.
const PROC_ALLOW: &[&str] = &["id", "phase", "pid", "port", "url", "startedAt", "readyAt", "exitCode", "errors", "errorLines"];

/// Procesy pro `/status` jen s povolenými poli.
fn procs_view(procs: Value) -> Value {
    let Value::Array(a) = procs else { return json!([]) };
    Value::Array(
        a.iter()
            .filter_map(Value::as_object)
            .map(|o| Value::Object(o.iter().filter(|(k, _)| PROC_ALLOW.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()))
            .collect(),
    )
}

/// Doplněk odpovědi `/status` (bez `configPath`, procesy jen s povolenými poli — L247).
pub(crate) fn status_extra() -> Arc<dyn Fn() -> Value + Send + Sync> {
    Arc::new(|| {
        let c = core();
        let (open_ms, cold) = c.drawer.last_open();
        json!({ "procs": procs_view(json!(c.procs.snapshot())),
            "drawer": { "view": c.drawer.view(), "warm": c.drawer.is_warm(), "lastOpenMs": open_ms, "lastOpenCold": cold, "anim": c.drawer.anim_stats() },
            "qa": crate::qa::status() })
    })
}

/// Obsluha `/ctl/<cmd>` (tělo JSON).
pub(crate) fn ctl_handler() -> claude::Ctl {
    Arc::new(|cmd, body| {
        let c = core();
        match cmd {
            "open" => {
                // ve hře nic, a volající se to dozví (jako `open_drawer` v MCP)
                if !crate::studio::remote_may_open(c.dead_surface()) {
                    return Err(crate::i18n::tr("Ledge je teď schovaný (hra na celé obrazovce nebo herní režim) — šuplík se neotevře.", "Ledge is hidden right now (a fullscreen game or game mode) — the drawer won't open.").into());
                }
                // "peek" / "island" / "mid" (náhledy po najetí — i k živému ověření bez myši), jinak celý
                let view = crate::hovermode::view_by_name(body.get("view").and_then(Value::as_str));
                c.send_ui(UiCmd::Show(view, Opener::Remote));
                Ok(json!({ "ok": true }))
            }
            "close" => {
                c.send_ui(UiCmd::Hide);
                Ok(json!({ "ok": true }))
            }
            "settings" => {
                // jako dřívější šuplík s Opener::Remote: bez fokusu, ve hře zahozeno
                if crate::studio::remote_may_open(c.dead_surface()) {
                    crate::studio::open_or(None, false, || {
                        let c = core();
                        c.send_ui(UiCmd::Show(View::Full, Opener::Remote));
                        let _ = c.app.emit_to(drawer::LABEL, "page", "settings");
                    });
                }
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
                let cfg = reload_from_disk()?;
                apply_config(cfg);
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("neznámý příkaz {cmd}")),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L247: `/status` neukáže `cmd`, `cwd`, `message`, `lastLine`, `tail` procesů (povolený seznam); skutečný `ProcView` projde filtrem.
    #[test]
    fn status_procs_only_allowed_fields() {
        let raw = json!([{ "id": "a", "phase": "ready", "pid": 1, "port": 3000, "url": "http://localhost:3000", "startedAt": 1, "readyAt": 2,
            "exitCode": null, "message": "m", "lastLine": "l", "errors": 1, "errorLines": ["e"], "cwd": "C:/x", "cmd": "npm run dev", "tail": ["t"] }]);
        let out = procs_view(raw);
        let o = out[0].as_object().unwrap();
        for bad in ["cwd", "cmd", "message", "lastLine", "tail"] {
            assert!(!o.contains_key(bad), "{bad} unikl do /status");
        }
        assert_eq!(o["port"], 3000);
        assert_eq!(o["errorLines"][0], "e");
        assert_eq!(procs_view(json!("x")), json!([]));
        let src = include_str!("ctl.rs");
        let prod = &src[..src.find("#[cfg(test)]").unwrap()];
        assert!(!prod.contains("\"configPath\""), "/status nesmí vracet configPath");
    }

    /// Příkazy z `match cmd { "x" => … }` v `ctl_handler` (jen produkční část souboru).
    fn commands() -> Vec<String> {
        let src = include_str!("ctl.rs");
        let prod = &src[src.find("pub(crate) fn ctl_handler").unwrap()..src.find("#[cfg(test)]").unwrap()];
        prod.lines()
            .filter_map(|l| {
                let l = l.trim_start();
                let rest = l.strip_prefix('"')?;
                let (name, tail) = rest.split_once('"')?;
                tail.trim_start().starts_with("=>").then(|| name.to_string())
            })
            .collect()
    }

    /// Povolit akci Clauda smí jen člověk v šuplíku (nepřekročitelné pravidlo): `/ctl/*` je pevná whitelist
    /// ovládání šuplíku a projektů — žádný příkaz, který by rozhodl o akci Clauda (J7). Přidání větve
    /// `"decide"`/`"approve"`… test shodí; nový neškodný příkaz se vědomě doplní do `ALLOWED`.
    #[test]
    fn ctl_has_no_approval_commands() {
        const ALLOWED: [&str; 6] = ["open", "close", "settings", "launch", "stop", "reload"];
        let cmds = commands();
        assert_eq!(cmds, ALLOWED, "změna příkazů /ctl/* — vědomě uprav ALLOWED (a ověř, že nerozhoduje o akci Clauda)");
        for c in &cmds {
            for bad in ["decide", "approve", "deny", "allow", "answer", "permission", "grant", "confirm"] {
                assert!(!c.contains(bad), "/ctl/{c} vypadá jako rozhodnutí o akci Clauda ({bad})");
            }
        }
    }

    /// Obsluha `/ctl` nesmí sahat na rozhodovací API mostu (`claude::decide` apod.) — hranice je v kódu, ne jen v názvech.
    #[test]
    fn ctl_never_calls_decision_api() {
        let src = include_str!("ctl.rs");
        let prod = &src[..src.find("#[cfg(test)]").unwrap()];
        for api in ["decide(", "decide_with(", "resolve_action", "approve(", "deny(", "answer("] {
            assert!(!prod.contains(api), "ctl.rs volá {api} — rozhodnout smí jen člověk v šuplíku");
        }
    }
}
