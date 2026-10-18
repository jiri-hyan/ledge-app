//! Stav rychlých akcí pro UI: hotové ikony (`STORE`, snapshot nese jen `iconKey`, data URL přes
//! příkaz `action_icon` — vzor `logoKey`/`project_logo`), varování z disku (cesta chybí) a
//! `views` pro snapshot (jen paměť, žádné I/O). `refresh` počítá na krátkém vlákně `action-icons`
//! — při startu, změně `actions` a na „Obnovit ikonu“; favicony přes `actionicons`.

use crate::actions::{self, Action, ActionKind, ShowOn};
use crate::actionicons::{self, Spec};
use crate::actionrun::{self, CONSENTS, RUNNING};
use crate::personal::LinkTarget;
use crate::state::{core, log, mark_dirty};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Default)]
pub struct Store {
    generation: u64,
    /// id akce → (klíč obsahu, data URL)
    icons: HashMap<String, (String, String)>,
    /// id akce → proč nejde spustit (cíl zmizel, typ nesedí)
    problems: HashMap<String, String>,
}

pub static STORE: Mutex<Option<Store>> = Mutex::new(None);
/// Favicony, které se právě stahují (klíč hostu) — dvojí `refresh` nestáhne dvakrát.
static IN_FLIGHT: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

fn with_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    f(STORE.lock().get_or_insert_with(Store::default))
}

fn fnv(b: &[u8]) -> String {
    format!("{:016x}", b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &x| (h ^ x as u64).wrapping_mul(0x0000_0100_0000_01b3)))
}

/// Data URL ikony akce (UI si ho vyžádá jednou na `iconKey`).
pub fn icon_url(id: &str) -> Option<String> {
    with_store(|s| s.icons.get(id).map(|e| e.1.clone()))
}

/// Přepočítá ikony a varování všech akcí na pozadí. `forget` = id akce, jejíž favicon se má
/// stáhnout znovu (výslovný klik).
pub fn refresh(forget: Option<&str>) {
    let list = core().cfg.read().actions.clone();
    if let Some(id) = forget
        && let Some(Spec::Favicon { key, .. }) = list.iter().find(|a| a.id == id).map(actionicons::spec)
    {
        actionicons::forget(&actionicons::cache_dir(), &key);
    }
    let generation = with_store(|s| {
        s.generation += 1;
        s.generation
    });
    std::thread::Builder::new()
        .name("action-icons".into())
        .spawn(move || compute(generation, list))
        .ok();
}

fn compute(generation: u64, list: Vec<Action>) {
    let dir = actionicons::cache_dir();
    let mut icons = HashMap::new();
    let mut problems = HashMap::new();
    let mut fetch = Vec::new();
    // souhlasy načíst teď (disk), aby `views` četl jen paměť
    {
        let mut c = CONSENTS.lock();
        for a in list.iter().filter(|a| a.risky()) {
            c.has(a);
        }
    }
    for a in &list {
        if let Some(p) = problem(a) {
            problems.insert(a.id.clone(), p);
        }
        let spec = actionicons::spec(a);
        let got = match &spec {
            Spec::Favicon { origin, key } => match actionicons::cached(&dir, key) {
                Some(hit) => hit.map(|b| ("image/png", b)),
                None => {
                    fetch.push((origin.clone(), key.clone()));
                    None
                }
            },
            other => actionicons::local(other),
        };
        if let Some((mime, bytes)) = got {
            icons.insert(a.id.clone(), (fnv(&bytes), format!("data:{mime};base64,{}", crate::icons::b64(&bytes))));
        }
    }
    let changed = with_store(|s| {
        if s.generation != generation {
            return false;
        }
        let changed = s.problems != problems || s.icons.len() != icons.len() || icons.iter().any(|(id, (k, _))| s.icons.get(id).map(|e| &e.0) != Some(k));
        s.icons = icons;
        s.problems = problems;
        changed
    });
    if changed {
        mark_dirty();
    }
    for (origin, key) in fetch {
        if !IN_FLIGHT.lock().insert(key.clone()) {
            continue;
        }
        tauri::async_runtime::spawn(async move {
            let png = actionicons::fetch_favicon(&origin).await;
            log(&format!("akce: favicon {origin} {}", if png.is_some() { "staženo" } else { "nenalezeno" }));
            actionicons::store(&actionicons::cache_dir(), &key, png.as_deref());
            IN_FLIGHT.lock().remove(&key);
            refresh(None); // z cache, bez sítě
        });
    }
}

/// Varování z disku: cíl neexistuje / nesedí s typem, pracovní složka chybí. Stat.
fn problem(a: &Action) -> Option<String> {
    if let Some(checked) = actionrun::check_fs(a) {
        if let LinkTarget::Invalid(why) = &checked {
            return Some(why.clone());
        }
        if let Err(e) = actionrun::resolve(a, Some(checked)) {
            return Some(e);
        }
    }
    if matches!(a.kind, ActionKind::App | ActionKind::Command) && a.cwd.as_deref().is_some_and(|c| !c.trim().is_empty()) {
        return actionrun::work_dir(a).err();
    }
    None
}

/// Akce pro UI (dlaždice, paleta).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    id: String,
    label: String,
    kind: ActionKind,
    target: String,
    group: Option<String>,
    show_on: ShowOn,
    icon_key: Option<String>,
    monogram: String,
    color: &'static str,
    /// před spuštěním se ptát (`confirm`, nebo příkaz bez uloženého souhlasu)
    needs_confirm: bool,
    /// co ukázat v potvrzení (u příkazu celá řádka se shellem)
    confirm_text: String,
    /// proč nejde spustit (z ověření na disku); `null` = v pořádku / neověřeno
    problem: Option<String>,
    /// skrytý příkaz právě běží
    running: bool,
    /// vzhled dlaždice (`tilelook::TileLook`); chybí = automaticky
    #[serde(skip_serializing_if = "Option::is_none")]
    look: Option<crate::tilelook::TileLook>,
}

/// Snapshot: jen paměť (store, souhlasy, běžící). Neznámý souhlas = ptát se.
pub fn views(list: &[Action]) -> Vec<ActionView> {
    let running = RUNNING.lock().clone();
    let consents = CONSENTS.lock();
    with_store(|s| {
        list.iter()
            .map(|a| ActionView {
                id: a.id.clone(),
                label: a.label.clone(),
                kind: a.kind,
                target: a.target.clone(),
                group: a.group.clone().filter(|g| !g.trim().is_empty()),
                show_on: a.show_on,
                icon_key: s.icons.get(&a.id).map(|e| e.0.clone()),
                monogram: crate::snapshot::monogram(&a.label),
                color: actions::monogram_color(&a.label),
                needs_confirm: actions::needs_confirm(a, a.risky() && consents.has_cached(a)),
                confirm_text: actions::confirm_text(a),
                problem: s.problems.get(&a.id).cloned(),
                running: running.contains(&a.id),
                look: a.look.clone(),
            })
            .collect()
    })
}
