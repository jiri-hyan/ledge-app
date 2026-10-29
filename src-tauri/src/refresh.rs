//! Obnovy na pozadí a reakce na události: počasí, klasifikace odkazů, loga, git stav projektů,
//! změny Claude (`on_claude_change`) a procesů (`on_proc_event`). Výsledky jdou přes `mark_dirty`.

use crate::drawer::{Opener, View};
use crate::procman::ProcEvent;
use crate::state::{UiCmd, WeatherState, core, log, mark_dirty, now_ms};
use crate::strip::Rgba;
use crate::hotkeys::sync_approve_keys;
use crate::{attention, config, gitinfo, icons, personal, sessionlive, weather};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri_plugin_opener::OpenerExt;

/// Stáhne počasí, když je šuplík otevřený (`opening` = volající ho právě otevírá) na záložce s kartou
/// Počasí (`layout::page_has`), místo je nastavené a data jsou stará (`weather::needs_fetch`). Výsledek přes `mark_dirty` → emit
/// vlákno. Jinak nic — žádný poll.
pub(crate) fn maybe_refresh_weather(opening: bool) {
    let c = core();
    let place = {
        let cfg = c.cfg.read();
        // jen když je karta Počasí na právě otevřené záložce (skrytá = nic se nestahuje)
        let on_personal = crate::layout::page_has(&cfg.layout, &c.mode.lock(), "weather");
        let open = opening || c.drawer.view() == View::Full;
        // proužek počasí používá (trio / vlastní s dílem Počasí): stahuje se i bez karty a se zavřeným šuplíkem,
        // jen ne za fullscreen hry (`dead`); `needs_fetch` drží ≥ 30 min a backoff, volá se z existujícího tiku
        let for_strip = weather::strip_wants_weather(&cfg) && !c.dead();
        match cfg.personal.weather.coords() {
            Some(p) if (on_personal && open) || for_strip => p,
            _ => return,
        }
    };
    let now = now_ms();
    {
        let mut st = c.weather.lock();
        if st.place != Some(place) {
            *st = WeatherState { place: Some(place), ..Default::default() };
        }
        // watchdog: pokus bez odpovědi déle než 2× timeout = neúspěch (zaseklý/spadlý task)
        if st.loading && !weather::loading_live(st.loading, st.attempt_ms, now) {
            st.loading = false;
            st.error = Some(weather::stuck().into());
            log("počasí: pokus bez odpovědi, beru jako neúspěch");
            drop(st);
            mark_dirty();
            st = c.weather.lock();
        }
        if !weather::needs_fetch(now, st.fetched_ms, st.attempt_ms, st.loading) {
            return;
        }
        st.loading = true;
        st.attempt_ms = Some(now);
    }
    mark_dirty();
    tauri::async_runtime::spawn(async move {
        let res = weather::fetch(place.0, place.1).await;
        let c = core();
        {
            let mut st = c.weather.lock();
            if st.place != Some(place) || st.attempt_ms != Some(now) {
                return; // mezitím jiné místo nebo novější pokus (tenhle už watchdog odepsal)
            }
            st.loading = false;
            match res {
                Ok(w) => {
                    st.data = Some(w);
                    st.fetched_ms = Some(now_ms());
                    st.error = None;
                }
                Err(e) => {
                    log(&format!("počasí: {e}"));
                    st.error = Some(e);
                }
            }
        }
        mark_dirty();
    });
}

/// Klasifikace cílů odkazů (stat disku) na pozadí — nikdy na emit/UI vlákně. Při startu a změně
/// odkazů v configu; `open_link` cíl před otevřením ověří znovu.
pub(crate) fn reclassify_links() {
    let c = core();
    let generation = c.links_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let targets: Vec<String> = c.cfg.read().personal.links.iter().map(|l| l.target.clone()).collect();
    std::thread::Builder::new()
        .name("links".into())
        .spawn(move || {
            let map: HashMap<String, personal::LinkTarget> = targets
                .into_iter()
                .map(|t| {
                    let r = personal::classify_fs(&t);
                    (t, r)
                })
                .collect();
            let c = core();
            let mut cache = c.links.lock();
            if generation > cache.0 {
                *cache = (generation, map);
                drop(cache);
                mark_dirty();
            }
        })
        .ok();
}

// ---------- reakce na změny ----------

pub(crate) fn on_claude_change() {
    let c = core();
    // hook od session, kterou seznam procesů ještě nezná → přenačíst (jen při otevřeném šuplíku)
    if c.drawer.view() == View::Full && c.claude.sessions().iter().any(|s| !s.ended && sessionlive::unseen(&s.id, s.updated_at)) {
        sessionlive::request(sessionlive::Wake::Hook);
    }
    // puls proužku / toast při „čeká na tebe“ a „hotovo“ (`attention.rs`)
    attention::on_change();
    // schválení i otázka Clauda → peek
    let pending = c.claude.attention();
    sync_approve_keys();
    match (pending, c.drawer.view()) {
        (true, View::Hidden) if !c.dead() => c.send_ui(UiCmd::Show(View::Peek, Opener::Hover)),
        // peek drží i připomínka po termínu
        (false, View::Peek) if c.notes.lock().due.is_empty() => c.send_ui(UiCmd::Hide),
        _ => {}
    }
    mark_dirty();
}

pub(crate) fn on_proc_event(e: ProcEvent) {
    match e {
        ProcEvent::Changed => mark_dirty(),
        ProcEvent::Ready { url, open_browser, .. } => {
            *core().flash.lock() = Some((Rgba::GREEN, Instant::now()));
            if open_browser {
                let _ = core().app.opener().open_url(url, None::<&str>);
            }
            mark_dirty();
        }
        ProcEvent::Failed { id, message } => {
            // selhání nikdy potichu: červený stav na dlaždici vždy (snapshot), toast jen když šuplík
            // není otevřený naplno a Ledge není „mrtvý“ (fullscreen hra) — `attention::signal`
            if attention::signal(Rgba::RED, true) {
                let c = core();
                let name = c.cfg.read().projects.iter().find(|p| p.id == id).map_or(id.clone(), |p| p.name.clone());
                // zpráva sama říká co (nenaběhl / skončil s kódem / nenaběhl do N s)
                crate::toast::show(&c.app.config().identifier, name, message);
            }
            mark_dirty();
        }
    }
}

/// Loga projektů na pozadí (`icons.rs`): `force` po změně configu / repo indexu, jinak (otevření
/// šuplíku) nejvýš jednou za `icons::STALE`. Snapshot čte jen hotovou mapu — nic z disku.
pub(crate) fn refresh_logos(force: bool) {
    let c = core();
    let Some(generation) = c.logos.lock().begin(force, Instant::now()) else { return };
    let projects: Vec<config::Project> = c.cfg.read().projects.iter().filter(|p| p.enabled).cloned().collect();
    let jobs: Vec<(String, icons::Source)> = projects
        .into_iter()
        .map(|p| {
            let dir = c.project_dir(&p);
            let policy = icons::exe_policy(&p);
            let exe = p.exe.as_ref().filter(|_| policy == icons::ExeIcon::Fallback).and_then(|e| {
                let e = PathBuf::from(e);
                if e.is_absolute() { Some(e) } else { dir.as_ref().map(|d| d.join(e)) }
            });
            let app_name = p.app_name.filter(|_| policy == icons::ExeIcon::First);
            (p.id, icons::Source { dir, logo: p.logo, exe, app_name, policy })
        })
        .collect();
    std::thread::Builder::new()
        .name("logos".into())
        .spawn(move || {
            let results = jobs.into_iter().map(|(id, src)| (id, icons::resolve(&src))).collect();
            if core().logos.lock().finish(generation, results, Instant::now()) {
                mark_dirty();
            }
        })
        .ok();
}

pub(crate) fn refresh_git_async() {
    std::thread::spawn(|| {
        let c = core();
        // ukončené nedokončené Claude session: šéf mohl mezitím commitnout ručně
        c.claude.recheck_ended();
        let projects = c.cfg.read().projects.clone();
        let mut changed = false;
        for p in projects.iter().filter(|p| p.enabled) {
            let fresh = c.git.lock().get(&p.id).is_some_and(|(t, _)| t.elapsed() < Duration::from_secs(30));
            if fresh {
                continue;
            }
            let info = c.project_place(p).1.and_then(|d| gitinfo::status(&d));
            c.git.lock().insert(p.id.clone(), (Instant::now(), info));
            changed = true;
        }
        if changed {
            mark_dirty();
        }
    });
}
