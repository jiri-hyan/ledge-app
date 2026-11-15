//! Rozšíření `ledge.exe --ext <id>` (kolo 33): samostatné procesy (tapeta, prohlížeč…) spouštěné a hlídané
//! Ledge. Kontrakt a toky: `docs/codemap/ext.md`. Tenhle soubor = registr manifestů, napojení na aplikaci
//! (`LiveEnv`, singleton supervisoru) a háčky pro zbytek kódu (`wire_up`, `on_config`, `on_dead_changed`,
//! `notify`, `wake`, `status`). Bez zapnutého rozšíření (`extensions: []`) tu neběží žádné vlákno ani časovač.

pub mod child;
pub mod conf;
pub mod host;
pub mod measure;
pub mod policy;
pub mod sup;
pub mod wall;
pub mod wire;

#[cfg(test)]
mod echo;
#[cfg(test)]
mod tests;

pub use conf::ExtConfig;
use policy::{Manifest, Req, Start, Timing};
use serde_json::Value;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use sup::{Env, ExtStatus, Sup};

/// Rozšíření, která tahle verze Ledge zná. Nové: řádek sem + tělo v `child::body_for` + složka `ext/<id>/`.
pub const REGISTRY: &[Manifest] = &[
    // živá tapeta (E2/E3): běží stále, ve hře pauza, po 10 min pauzy konec procesu (RAM zpět)
    Manifest { id: "wall", start: Start::Always, idle_quit_min: None, pause_quit_min: Some(10), wants: &["media"] },
];

/// Odstup prvního spuštění po startu Ledge (ne v hrotu startu).
const STARTUP_DELAY: Duration = Duration::from_secs(3);

static SUP: OnceLock<Arc<Sup>> = OnceLock::new();

/// Napojení supervisoru na Ledge: čte `Core`, `media`, `studio`.
struct LiveEnv;

impl Env for LiveEnv {
    fn cfg_for(&self, id: &str) -> Value {
        let path = crate::actionrun::local_dir().join("ext").join(format!("{id}.json"));
        std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| serde_json::json!({}))
    }
    fn lang(&self) -> String {
        serde_json::to_value(crate::i18n::lang()).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "cs".into())
    }
    fn dead(&self) -> bool {
        crate::state::CORE.get().is_some_and(|c| c.dead())
    }
    fn topic_data(&self, topic: &str) -> Option<Value> {
        match topic {
            "media" => serde_json::to_value(crate::media::view()).ok(),
            _ => None,
        }
    }
    fn handle_req(&self, id: &str, req: Req) {
        use crate::media::Control;
        use policy::MediaAct;
        let r = match req {
            Req::Media(a) => crate::media::control(match a {
                MediaAct::Toggle => Control::Toggle,
                MediaAct::Next => Control::Next,
                MediaAct::Prev => Control::Prev,
            }),
            Req::OpenSettings => crate::studio::studio_open(None),
            // obal alba (PNG ≤ 128 px jako data URL z `media::cover`) jde zpět jako `cmd{cover,{key,url}}`; čeká se na
            // vlákno `media`, proto mimo smyčku supervisoru. Jiný klíč než aktuální skladba = `None` = nic se nepošle.
            Req::Cover(key) => {
                let id = id.to_string();
                std::thread::spawn(move || {
                    if let Some(url) = crate::media::cover(&key) {
                        sup().send_cmd(&id, "cover", serde_json::json!({"key": key, "url": url}));
                    }
                });
                return;
            }
        };
        if let Err(e) = r {
            crate::log(&format!("ext {id}: požadavek selhal: {e}"));
        }
    }
    fn log(&self, msg: &str) {
        crate::log(msg);
    }
}

fn launcher() -> sup::Launcher {
    Arc::new(|id: &str| {
        let exe = std::env::current_exe().unwrap_or_else(|_| "ledge.exe".into());
        let mut c = Command::new(exe);
        c.arg("--ext").arg(id);
        c
    })
}

/// Supervisor vzniká líně — až když je co spouštět.
fn sup() -> &'static Arc<Sup> {
    SUP.get_or_init(|| Sup::new(Arc::new(LiveEnv), launcher(), REGISTRY.to_vec(), Timing::default()))
}

// ---------- háčky pro zbytek aplikace ----------

/// Start Ledge (`lib.rs` setup): jsou-li v configu rozšíření, za pár sekund je spustí. Jinak nedělá nic.
pub(crate) fn wire_up() {
    let enabled = crate::state::core().cfg.read().extensions.0.clone();
    if enabled.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(STARTUP_DELAY);
        sup().sync(&enabled);
    });
}

/// Konec Ledge (`lib.rs` `RunEvent::Exit`): rozšíření dostanou `quit` a ~1,5 s na řádné zavření. Bez supervisoru
/// (nic nikdy nezapnuto) jen přečte jedno `OnceLock`.
pub(crate) fn shutdown() {
    if let Some(s) = SUP.get() {
        s.shutdown(Duration::from_millis(1500));
    }
}

/// Změna configu (`settings.rs`): zapnout / vypnout rozšíření.
pub(crate) fn on_config(old: &crate::config::Config, new: &crate::config::Config) {
    if old.extensions == new.extensions {
        return;
    }
    sup().sync(&new.extensions.0);
}

/// Začala / skončila hra nebo herní režim (`threads/hover.rs`, `gamemode::set`): pauza rozšíření.
/// Bez supervisoru (nic nikdy nezapnuto) jen přečte jedno `OnceLock`.
pub(crate) fn on_dead_changed() {
    if let Some(s) = SUP.get() {
        s.on_dead_changed();
    }
}

/// Změnila se data tématu (`media`…): odběratelé si natáhnou čerstvá. Bez odběratele nic.
#[allow(dead_code)] // háček pro E2 (`media.rs` ho zavolá při `Msg::Changed`)
pub(crate) fn notify(topic: &str) {
    if let Some(s) = SUP.get() {
        s.notify(topic);
    }
}

/// Nastavení změnilo pozadí tapety (`commands::ext::ext_wall_bg`): běžící `wall` si znovu přečte kopii fotky.
/// Neběží-li, nic — při příštím startu si fotku přečte samo.
pub(crate) fn wall_bg_changed() {
    if let Some(s) = SUP.get() {
        s.send_cmd("wall", "bg", serde_json::json!({}));
    }
}

/// `OnDemand` probuzení rozšíření z kliku nebo události.
#[allow(dead_code)] // použije E1 / E5 (příkaz z UI, akce `kind`)
pub(crate) fn wake(id: &str, reason: &str) -> Result<(), String> {
    sup().wake(id, reason)
}

/// Stav všech známých rozšíření (`status_measured` → `ext_status`). Bez supervisoru vrací „vypnuto“.
pub(crate) fn status() -> Vec<ExtStatus> {
    match SUP.get() {
        Some(s) => s.status(),
        None => REGISTRY.iter().map(|m| ExtStatus {
            id: m.id.to_string(),
            enabled: false,
            phase: policy::Phase::Idle,
            on_demand: m.start == Start::OnDemand,
            pid: None,
            error: None,
            tail: Vec::new(),
            pause_why: None,
        }).collect(),
    }
}

/// Stav rozšíření + právě změřená cena (`ext_status`, blok Rozšíření v Nastavení).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtView {
    #[serde(flatten)]
    pub st: ExtStatus,
    /// `None` = neběží (UI ukáže „—“, nikdy odhad)
    pub cost: Option<measure::Cost>,
}

/// `status()` + změření běžících rozšíření. Měří se jen tady, při volání z UI — žádný časovač.
pub(crate) fn status_measured() -> Vec<ExtView> {
    status()
        .into_iter()
        .map(|st| {
            let cost = match (st.pid, job(&st.id)) {
                (Some(pid), Some(j)) => measure::measure(&st.id, &j, pid),
                _ => None,
            };
            if cost.is_none() {
                measure::forget(&st.id);
            }
            ExtView { st, cost }
        })
        .collect()
}

/// Job běžícího rozšíření (měření). `None` = neběží.
pub(crate) fn job(id: &str) -> Option<Arc<crate::procman::Job>> {
    SUP.get()?.job(id)
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique_and_safe() {
        let mut seen = std::collections::HashSet::new();
        for m in REGISTRY {
            assert!(seen.insert(m.id), "duplicitní id {}", m.id);
            // id jde do názvu mutexu a souboru `ext/<id>.json`
            assert!(!m.id.is_empty() && m.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "{}", m.id);
        }
    }

    #[test]
    fn status_without_supervisor_lists_all_off() {
        let st = status();
        assert_eq!(st.len(), REGISTRY.len());
        assert!(st.iter().all(|s| !s.enabled && s.phase == policy::Phase::Idle && s.pid.is_none()));
    }
}
