//! Widgety za běhu: stav v paměti (poslední data, chyba, výstup, souhlas), vlákno `widgets` a pohled
//! pro UI / proužek. Vlákno vzniká líně (první widget na záložce nebo v proužku) a spí na condvaru:
//! budí ho otevření šuplíku, přepnutí záložky, změna configu, souhlas, „Obnovit teď“ a konec běhu.
//! Když nic není vidět, spí bez termínu — žádný časovač v klidu. Snapshot čte jen paměť.

use super::consent::{self, Consent, Store};
use super::meta::{Kind, Show};
use super::output::{self, Item, Tone, WidgetData};
use super::{Scan, Widget, WidgetsConfig, plan, run};
use crate::config::Config;
use crate::drawer::View;
use crate::i18n::tr;
use crate::state::{core, mark_dirty, now_ms};
use crate::strip::{Rgba, Segment};
use parking_lot::{Condvar, Mutex};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Sken složky nejvýš jednou za tuhle dobu (otevření šuplíku, přepínání záložek).
const SCAN_TTL: Duration = Duration::from_secs(5);
/// Nejvýš tolik widgetů běží zároveň.
const MAX_PARALLEL: usize = 3;
/// Jak často se podívat, jestli hra skončila (proužek s widgetem čeká).
const RECHECK: Duration = Duration::from_secs(30);

#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub data: Option<WidgetData>,
    pub ok_ms: Option<i64>,
    pub error: Option<String>,
    pub output: Option<String>,
    pub running: bool,
    pub force: bool,
    pub last_end: Option<Instant>,
}

#[derive(Default)]
struct Live {
    scan: Scan,
    scanned: Option<Instant>,
    dir: Option<PathBuf>,
    consent: HashMap<String, Consent>,
    problems: HashMap<String, Option<String>>,
    entries: HashMap<String, Entry>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);
static STORE: Mutex<Store> = Mutex::new(Store::new());
static SIGNAL: Mutex<bool> = Mutex::new(false);
static COND: Condvar = Condvar::new();
static STARTED: AtomicBool = AtomicBool::new(false);
/// Šuplík se právě otevírá (vlákno `ui` budí dřív, než je `View::Full`) — první kolo ho bere jako otevřený.
static OPENING: AtomicBool = AtomicBool::new(false);

fn with<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    let mut g = LIVE.lock();
    f(g.get_or_insert_with(Live::default))
}

/// Id widgetů, které jsou někde umístěné (karta na kterékoli záložce, položka proužku).
pub fn placed(cfg: &Config) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let cards = cfg.layout.pages.iter().flat_map(|p| p.cards.iter().map(|c| c.id.clone()));
    let strip = cfg.strip.items.iter().map(|i| i.key());
    // widgety ze skriptů připnuté na plochu (`desk/`) — snapshot je ukáže, plánovač je spustí, když jsou vidět
    let desk = crate::desk::win::script_ids(false);
    for id in cards.chain(strip).chain(desk) {
        if id.starts_with(plan::PREFIX) && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Probudí vlákno (založí ho, když ještě není).
pub fn wake() {
    if !STARTED.swap(true, Ordering::SeqCst) {
        let ok = std::thread::Builder::new().name("widgets".into()).spawn(thread).is_ok();
        if !ok {
            STARTED.store(false, Ordering::SeqCst);
            return;
        }
    }
    *SIGNAL.lock() = true;
    COND.notify_one();
}

/// Probudí jen, když nějaký widget je umístěný (start, změna configu) — jinak vlákno nevznikne.
pub fn wake_if_used(cfg: &Config) {
    if cfg.widgets.enabled && !placed(cfg).is_empty() {
        wake();
    }
}

/// Otevření šuplíku: widgety na otevřené záložce se obnoví hned (data starší než interval).
pub fn wake_opening(cfg: &Config) {
    if cfg.widgets.enabled && !placed(cfg).is_empty() {
        OPENING.store(true, Ordering::SeqCst);
        wake();
    }
}

/// Změna configu: jiná složka = sken znovu.
pub fn on_config(old: &Config, new: &Config) {
    if old.widgets != new.widgets {
        with(|l| l.scanned = None);
    }
    if old.widgets != new.widgets || old.layout != new.layout || old.strip.items != new.strip.items || old.strip.enabled != new.strip.enabled {
        wake_if_used(new);
    }
}

fn ctx(cfg: &Config) -> plan::Ctx {
    let c = core();
    let mode = c.mode.lock().clone();
    plan::Ctx {
        page_cards: crate::layout::page(&cfg.layout, &mode).map(|p| p.cards.iter().map(|c| c.id.clone()).collect()).unwrap_or_default(),
        drawer_full: OPENING.swap(false, Ordering::SeqCst) || c.drawer.view() == View::Full,
        strip_items: cfg.strip.items.iter().map(|i| i.key()).collect(),
        strip_on: cfg.strip.enabled,
        dead: c.dead(),
        desk: crate::desk::win::script_ids(true),
    }
}

/// Sken složky (disk) + souhlasy a problémy pro každý widget. Volat mimo UI / emit vlákno.
pub fn rescan(cfg: &WidgetsConfig) -> Result<(), String> {
    let dir = super::resolve_dir(cfg)?;
    let prev = with(|l| (l.dir.as_ref() == Some(&dir)).then(|| l.scan.clone()));
    let scan = super::scan_dir(&dir, prev.as_ref());
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let mut consent = HashMap::new();
    let mut problems = HashMap::new();
    {
        let mut store = STORE.lock();
        for w in &scan.widgets {
            let p = super::problem(w, &path_var);
            if p.is_none()
                && let Some(fp) = w.fingerprint()
            {
                consent.insert(w.id.clone(), store.check(&w.path, &fp));
            }
            problems.insert(w.id.clone(), p);
        }
    }
    with(|l| {
        l.entries.retain(|id, _| scan.get(id).is_some());
        l.scan = scan;
        l.dir = Some(dir);
        l.consent = consent;
        l.problems = problems;
        l.scanned = Some(Instant::now());
    });
    mark_dirty();
    Ok(())
}

fn ready(l: &Live, w: &Widget) -> bool {
    w.meta.is_ok() && l.problems.get(&w.id).is_some_and(Option::is_none) && l.consent.get(&w.id) == Some(&Consent::Granted)
}

fn thread() {
    loop {
        let cfg = core().cfg.read().clone();
        let mut wake_at = None;
        if cfg.widgets.enabled {
            let (active, parked) = plan::active(&ctx(&cfg));
            let forced = with(|l| l.entries.values().any(|e| e.force));
            let stale = with(|l| l.scanned.is_none_or(|t| t.elapsed() > SCAN_TTL));
            if (!active.is_empty() || forced)
                && stale
                && let Err(e) = rescan(&cfg.widgets)
            {
                crate::state::log(&format!("widgety: {e}"));
            }
            let (to_run, at, dir) = with(|l| {
                let slots: Vec<plan::Slot> = l
                    .scan
                    .widgets
                    .iter()
                    .map(|w| {
                        let e = l.entries.get(&w.id);
                        plan::Slot {
                            id: w.id.clone(),
                            interval: Duration::from_secs(w.meta.as_ref().map_or(300, |m| m.interval_s).into()),
                            ready: ready(l, w),
                            running: e.is_some_and(|e| e.running),
                            force: e.is_some_and(|e| e.force),
                            last_end: e.and_then(|e| e.last_end),
                        }
                    })
                    .collect();
                let d = plan::decide(&slots, &active, parked, Instant::now(), RECHECK);
                let busy = l.entries.values().filter(|e| e.running).count();
                let run: Vec<String> = d.run.into_iter().take(MAX_PARALLEL.saturating_sub(busy)).collect();
                for id in &run {
                    let e = l.entries.entry(id.clone()).or_default();
                    e.running = true;
                    e.force = false;
                }
                (run, d.wake_at, l.dir.clone())
            });
            wake_at = at;
            if let Some(dir) = dir {
                for id in to_run {
                    let dir = dir.clone();
                    let spawned = std::thread::Builder::new().name("widget-run".into()).spawn(move || run_one(id.clone(), dir));
                    if spawned.is_err() {
                        break;
                    }
                }
                mark_dirty();
            }
        }
        let mut g = SIGNAL.lock();
        if !*g {
            match wake_at {
                Some(t) => {
                    COND.wait_until(&mut g, t);
                }
                None => COND.wait(&mut g),
            }
        }
        *g = false;
    }
}

enum Outcome {
    Problem(String),
    Consent(Consent),
    Ran(Result<run::Ran, (run::Fail, Option<run::Ran>)>),
}

fn attempt(id: &str, dir: &std::path::Path) -> Outcome {
    let w = match super::fresh(dir, id) {
        Ok(w) => w,
        Err(e) => return Outcome::Problem(e),
    };
    let m = match &w.meta {
        Ok(m) => m.clone(),
        Err(e) => return Outcome::Problem(e.clone()),
    };
    let Some(fp) = w.fingerprint() else { return Outcome::Problem(tr("soubor nejde přečíst", "can't read the file").into()) };
    match STORE.lock().check(&w.path, &fp) {
        Consent::Granted => {}
        c => return Outcome::Consent(c),
    }
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    match run::command(&w, &m, &path_var) {
        Ok(mut cmd) => Outcome::Ran(run::execute(&mut cmd, m.timeout_s)),
        Err(e) => Outcome::Problem(e),
    }
}

fn run_one(id: String, dir: PathBuf) {
    let out = attempt(&id, &dir);
    let mut log_line = None;
    with(|l| {
        let mut rescan = false;
        let e = l.entries.entry(id.clone()).or_default();
        e.running = false;
        e.last_end = Some(Instant::now());
        match out {
            Outcome::Ran(Ok(r)) => {
                e.data = Some(output::parse(&r.stdout));
                e.ok_ms = Some(now_ms());
                e.error = None;
                e.output = Some(run::keep(&r));
            }
            Outcome::Ran(Err((f, r))) => {
                let text = run::fail_text(&f);
                if e.error.as_deref() != Some(text.as_str()) {
                    log_line = Some(format!("widget {id}: {text}"));
                }
                e.error = Some(text);
                e.output = r.as_ref().map(run::keep);
            }
            Outcome::Consent(c) => {
                l.consent.insert(id.clone(), c);
                rescan = true;
            }
            Outcome::Problem(p) => {
                l.problems.insert(id.clone(), Some(p));
                rescan = true;
            }
        }
        if rescan {
            l.scanned = None;
        }
    });
    if let Some(line) = log_line {
        crate::state::log(&line);
    }
    mark_dirty();
    wake();
}

// ---------- pohled pro UI ----------

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConsentAsk {
    /// soubor se od souhlasu změnil (jinak nový)
    pub changed: bool,
    pub note: String,
    /// přesně tohle musí UI poslat zpátky (obsahuje otisk obsahu)
    pub confirm_text: String,
    /// začátek skriptu
    pub preview: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WidgetView {
    pub id: String,
    pub file: String,
    pub title: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub show: Show,
    pub interval_s: u32,
    pub kind: Option<Kind>,
    /// `off` vypnuté · `missing` soubor chybí · `problem` chybná hlavička / interpret · `consent` čeká na
    /// souhlas · `loading` první běh · `error` poslední běh selhal · `ok` · `idle` ještě neběžel
    pub status: &'static str,
    pub problem: Option<String>,
    pub consent: Option<ConsentAsk>,
    pub data: Option<WidgetData>,
    pub error: Option<String>,
    pub updated_ms: Option<i64>,
    pub running: bool,
    pub has_output: bool,
    /// je položkou proužku (`strip.items`)
    pub in_strip: bool,
}

fn view_of(l: &Live, id: &str, enabled: bool, strip: &[String]) -> WidgetView {
    let in_strip = strip.iter().any(|s| s == id);
    let file = id.strip_prefix(plan::PREFIX).unwrap_or(id).to_string();
    let e = l.entries.get(id).cloned().unwrap_or_default();
    let Some(w) = l.scan.get(id) else {
        return WidgetView {
            id: id.to_string(),
            title: file.clone(),
            file,
            icon: None,
            description: None,
            show: Show::Both,
            interval_s: 0,
            kind: None,
            status: if !enabled { "off" } else if l.scanned.is_some() { "missing" } else { "idle" },
            problem: None,
            consent: None,
            data: None,
            error: None,
            updated_ms: None,
            running: false,
            has_output: false,
            in_strip,
        };
    };
    let m = w.meta.as_ref().ok();
    let problem = w.meta.as_ref().err().cloned().or_else(|| l.problems.get(id).cloned().flatten());
    let c = l.consent.get(id).copied();
    let consent = (problem.is_none() && c.is_some_and(|c| c != Consent::Granted)).then(|| ConsentAsk {
        changed: c == Some(Consent::Changed),
        note: consent::reason_text(c.unwrap_or(Consent::New)).to_string(),
        confirm_text: w.confirm_text(),
        preview: w.head.clone(),
    });
    let status = if !enabled {
        "off"
    } else if problem.is_some() {
        "problem"
    } else if consent.is_some() {
        "consent"
    } else if e.error.is_some() {
        "error"
    } else if e.data.is_some() {
        "ok"
    } else if e.running {
        "loading"
    } else {
        "idle"
    };
    WidgetView {
        id: id.to_string(),
        file: w.file.clone(),
        title: w.title(),
        icon: m.and_then(|m| m.icon.clone()),
        description: m.and_then(|m| m.description.clone()),
        show: m.map_or(Show::Card, |m| m.show),
        interval_s: m.map_or(0, |m| m.interval_s),
        kind: Some(w.kind),
        status,
        problem,
        consent,
        data: e.data.clone(),
        error: e.error.clone(),
        updated_ms: e.ok_ms,
        running: e.running,
        has_output: e.output.is_some(),
        in_strip,
    }
}

/// Snapshot: jen umístěné widgety, jen paměť.
pub fn snapshot_views(cfg: &Config) -> Vec<WidgetView> {
    let ids = placed(cfg);
    if ids.is_empty() {
        return vec![];
    }
    let strip: Vec<String> = cfg.strip.items.iter().map(|i| i.key()).collect();
    let g = LIVE.lock();
    match g.as_ref() {
        Some(l) => ids.iter().map(|id| view_of(l, id, cfg.widgets.enabled, &strip)).collect(),
        None => {
            let l = Live::default();
            ids.iter().map(|id| view_of(&l, id, cfg.widgets.enabled, &strip)).collect()
        }
    }
}

/// Co ví Nastavení a katalog.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub enabled: bool,
    pub dir: String,
    pub exists: bool,
    pub widgets: Vec<WidgetView>,
    pub skipped: usize,
    pub error: Option<String>,
}

/// Sken (disk) a seznam všech widgetů ve složce.
pub fn listing(full: &Config) -> Listing {
    let cfg = &full.widgets;
    let strip: Vec<String> = full.strip.items.iter().map(|i| i.key()).collect();
    let shown = super::resolve_dir(cfg).map(|d| d.display().to_string()).unwrap_or_default();
    let err = rescan(cfg).err();
    with(|l| Listing {
        enabled: cfg.enabled,
        dir: shown,
        exists: l.scan.exists,
        widgets: l.scan.widgets.iter().map(|w| view_of(l, &w.id, cfg.enabled, &strip)).collect(),
        skipped: l.scan.skipped,
        error: err.or_else(|| l.scan.error.clone()),
    })
}

/// Souhlas z karty / Nastavení: `confirmed` musí sedět s tím, co člověk viděl (otisk obsahu).
pub fn grant(cfg: &WidgetsConfig, id: &str, confirmed: &str) -> Result<(), String> {
    let dir = super::resolve_dir(cfg)?;
    let w = super::fresh(&dir, id)?;
    let fp = w.fingerprint().ok_or_else(|| w.meta.clone().err().unwrap_or_else(|| tr("soubor nejde přečíst", "can't read the file").into()))?;
    if w.confirm_text() != confirmed {
        with(|l| l.scanned = None);
        wake();
        return Err(trf!("{}: skript se mezitím změnil — podívej se na něj znovu.", "{}: the script changed meanwhile — have another look.", w.title()));
    }
    STORE.lock().grant(&w.path, &fp)?;
    with(|l| {
        l.consent.insert(id.to_string(), Consent::Granted);
        l.entries.entry(id.to_string()).or_default().force = true;
        l.scanned = None;
    });
    crate::state::log(&format!("widget {id}: povolen ({})", &fp[..12]));
    wake();
    mark_dirty();
    Ok(())
}

/// „Obnovit teď“ (výslovný klik — i neaktivní widget).
pub fn refresh_now(id: &str) {
    with(|l| {
        let e = l.entries.entry(id.to_string()).or_default();
        if !e.running {
            e.force = true;
        }
        l.scanned = None;
    });
    wake();
}

/// Poslední výstup („Zobrazit výstup“).
pub fn output_of(id: &str) -> Option<String> {
    with(|l| l.entries.get(id).and_then(|e| e.output.clone()))
}

/// Položka `index` z posledních dat, jen když má pořád tenhle `label` (data se mezitím mohla změnit).
pub fn item(id: &str, index: usize, label: &str) -> Option<Item> {
    with(|l| l.entries.get(id)?.data.as_ref()?.items.get(index).filter(|i| i.label == label).cloned())
}

/// Cesta k souboru widgetu (Upravit skript) — `.exe` → jeho hlavička `.exe.ledge`.
pub fn edit_path(cfg: &WidgetsConfig, id: &str) -> Result<PathBuf, String> {
    let dir = super::resolve_dir(cfg)?;
    let file = plan::file_of(id).ok_or_else(|| tr("Neznámý widget.", "Unknown widget."))?;
    let path = dir.join(file);
    let kind = std::path::Path::new(file).extension().and_then(|e| e.to_str()).and_then(Kind::from_ext).ok_or_else(|| tr("Neznámý widget.", "Unknown widget."))?;
    let path = if kind == Kind::Exe { super::sidecar_of(&path) } else { path };
    if path.is_file() { Ok(path) } else { Err(trf!("Soubor {} neexistuje.", "The file {} doesn't exist.", path.display())) }
}

// ---------- proužek ----------

fn dot(t: Tone) -> Rgba {
    match t {
        Tone::Accent | Tone::Violet => Rgba::VIOLET,
        Tone::Text => Rgba::TEXT,
        Tone::Muted => Rgba::DIM,
        Tone::Blue => Rgba::BLUE,
        Tone::Teal => Rgba::TEAL,
        Tone::Green => Rgba::GREEN,
        Tone::Amber => Rgba::AMBER,
        Tone::Red => Rgba::RED,
    }
}

/// Segment položky proužku `widget:<soubor>` z paměti: text z dat (nejvýš 24 znaků), barva jako
/// tečka. Bez souhlasu / bez dat nic; chyba bez dat = „⚠ název“.
pub fn strip_segments(id: &str) -> Vec<Segment> {
    let g = LIVE.lock();
    let Some(l) = g.as_ref() else { return vec![] };
    let Some(w) = l.scan.get(id) else { return vec![] };
    if !ready(l, w) {
        return vec![];
    }
    let e = l.entries.get(id);
    match e.and_then(|e| e.data.as_ref()).and_then(|d| d.strip_text().map(|t| (t, d.color))) {
        Some((text, color)) => vec![Segment { text, color: Rgba::TEXT, dot: color.map(dot), strong: false, pill: color.is_some() }],
        None if e.is_some_and(|e| e.error.is_some() && e.data.is_none()) => {
            let t: String = w.title().chars().take(output::MAX_STRIP - 2).collect();
            vec![Segment { text: format!("⚠ {t}"), color: Rgba::DIM, dot: Some(Rgba::AMBER), strong: false, pill: true }]
        }
        None => vec![],
    }
}
