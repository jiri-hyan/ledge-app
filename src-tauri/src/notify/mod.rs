//! Oznámení Ledge (kolo 26): událost rozbalí proužek nahoře (Notch), ukáže obsah a po `notify.durationS`
//! se složí; Windows toast jen jako záložní cesta (`notify.fallbackToast`) nebo jednou volbou
//! (`notify.surface = toast`).
//!
//! Tok: odesílatel (`attention::on_change`, `calendar::remind::tick`, `timer::tick_live`, `limitslive::tick`)
//! projde dnešní bránou `attention::signal` (hra, „nerušit“, šuplík naplno = nic) a zavolá `emit` → čistá
//! `route::route` → fronta (`queue`, priorita a slučování) → `StripModel.notice` → vlákno `strip` kreslí
//! `draw::draw` podle čisté `choreo::frame`. Snímkový časovač běží jen během oznámení.
//!
//! Tlačítka (Otevřít / Připojit) reagují jen v klidové fázi tvaru (`Frame.settled`); okno zůstává
//! no-activate, klik vede jen tam, kam tlačítko (stejná cesta jako dlaždice v šuplíku). Tlačítko
//! „Povolit“ záměrně neexistuje — povolit akci Clauda smí jen člověk v okně Claude.

pub mod choreo;
pub mod conf;
pub mod draw;
mod queue;
mod route;

pub use conf::{NotifyConfig, NotchStyle, Surface};

use crate::i18n::tr;
use crate::state::{core, log};
use crate::strip::Rgba;
use parking_lot::Mutex;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::RECT;

/// Událost oznámení.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Kind {
    /// Claude čeká na schválení / otázku
    Waiting,
    #[default]
    Done,
    Meeting,
    /// limit 80 / 90 %
    Limit,
    /// časovač doběhl
    Timer,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Waiting, Kind::Done, Kind::Meeting, Kind::Limit, Kind::Timer];
    /// Priorita fronty (nižší = důležitější).
    pub fn rank(self) -> u8 {
        match self {
            Kind::Waiting => 0,
            Kind::Meeting => 1,
            Kind::Limit => 2,
            Kind::Timer => 3,
            Kind::Done => 4,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Kind::Waiting => "waiting",
            Kind::Done => "done",
            Kind::Meeting => "meeting",
            Kind::Limit => "limit",
            Kind::Timer => "timer",
        }
    }
    pub fn from_id(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.id() == s)
    }
    /// Barva události (stejná jako puls proužku).
    pub fn color(self) -> Rgba {
        match self {
            Kind::Waiting | Kind::Limit => Rgba::AMBER,
            Kind::Done => Rgba::GREEN,
            Kind::Meeting => Rgba::BLUE,
            Kind::Timer => Rgba::TEAL,
        }
    }
}

/// Kam vede tlačítko.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// přivést okno session (`commands::sessions::session_open`)
    Session(String),
    /// připojit se ke schůzce (`commands::calendar::calendar_join`, adresa ověřená `calendar::meeting`)
    Join(String),
}

/// Obsah oznámení (z reálného zdroje; co není, je prázdné).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Notice {
    pub kind: Kind,
    pub title: String,
    pub body: String,
    /// popisek tlačítka (jen styly s místem na tlačítko); u ukázky bez `action` klik jen zavře
    pub button: Option<String>,
    pub action: Option<Action>,
    /// schůzka: začátek (ms) pro odpočet prstencem
    pub start_ms: Option<i64>,
    /// limit: procenta (jantarový pruh)
    pub pct: Option<f32>,
    /// sloučených stejných událostí
    pub count: u32,
    /// ukázka ze Studia (nepočítá se do zmeškaných)
    pub preview: bool,
}

/// Běžící oznámení v modelu proužku (`StripModel.notice`).
#[derive(Debug, Clone, PartialEq)]
pub struct Live {
    pub n: Notice,
    pub style: NotchStyle,
    pub hold: u64,
    pub start: Instant,
    /// tvar proužku před oznámením (px) — z něj se rozbaluje
    pub base: (i32, i32),
    /// klik (tlačítko nebo zavření) → nepočítá se jako zmeškané
    pub acked: bool,
}

impl Live {
    pub fn elapsed(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

/// Kde je teď na obrazovce tvar a tlačítko (px); zapisuje `draw`, čte okno proužku a vlákno `hover`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Hit {
    pub shape: Option<RECT>,
    pub button: Option<RECT>,
}

static Q: Mutex<Option<queue::Queue>> = Mutex::new(None);
static HIT: Mutex<Hit> = Mutex::new(Hit { shape: None, button: None });
/// Smí proužek vyjet (z `stripview::refresh_strip`)?
static STAGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn with_q<R>(f: impl FnOnce(&mut queue::Queue) -> R) -> R {
    f(Q.lock().get_or_insert_with(queue::Queue::default))
}

fn live_of(n: Notice, style: Option<NotchStyle>) -> Live {
    let c = core();
    let cfg = c.cfg.read();
    let style = style.unwrap_or_else(|| cfg.notify.style_of(n.kind));
    let hold = cfg.notify.hold_ms(n.kind);
    drop(cfg);
    let base = *c.strip.last.lock();
    Live { n, style, hold, start: Instant::now(), base, acked: false }
}

fn show(next: Option<Notice>) {
    show_as(next, None);
}

fn show_as(next: Option<Notice>, style: Option<NotchStyle>) {
    let live = next.map(|n| live_of(n, style));
    if live.is_none() {
        *HIT.lock() = Hit::default();
    }
    core().strip.set(|m| m.notice = live);
}

/// Kurzor je na monitoru proužku? (bez kurzoru = ano)
fn cursor_on_strip_monitor() -> bool {
    let r = core().monitor().rect;
    crate::screen::cursor().is_none_or(|p| p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom)
}

/// Otevře (nebo už drží) šuplík peek s čekající žádostí / otázkou? (`refresh::on_claude_change`)
fn peek_shows_attention() -> bool {
    use crate::drawer::View;
    let c = core();
    c.claude.attention() && !c.dead() && matches!(c.drawer.view(), View::Hidden | View::Peek)
}

/// Odešli oznámení. Volá se až PO bráně `attention::signal` (ta vrátila true).
pub(crate) fn emit(n: Notice) {
    let c = core();
    let (surface, fallback_toast) = {
        let cfg = c.cfg.read();
        (cfg.notify.surface, cfg.notify.fallback_toast)
    };
    let ctx = route::Ctx {
        allowed: true,
        surface,
        fallback_toast,
        stage: STAGE.load(std::sync::atomic::Ordering::Relaxed),
        same_monitor: surface == Surface::Toast || cursor_on_strip_monitor(),
        covered: n.kind == Kind::Waiting && peek_shows_attention(),
    };
    match route::route(ctx) {
        route::Route::Notch => match with_q(|q| q.push(n.clone())) {
            queue::Push::Show => show(Some(n)),
            queue::Push::MergedLive => {
                let cur = with_q(|q| q.cur.clone());
                core().strip.set(|m| {
                    if let (Some(l), Some(cur)) = (m.notice.as_mut(), cur) {
                        l.n = cur;
                    }
                });
            }
            queue::Push::MergedQueued | queue::Push::Queued => {}
        },
        route::Route::Toast => toast(&n),
        route::Route::Nothing => {}
    }
}

/// Záložní / zvolený toast Windows (dnešní podoba: titulek, tělo, u schůzky tlačítko Připojit protokolem).
fn toast(n: &Notice) {
    let app_id = core().app.config().identifier.clone();
    match &n.action {
        Some(Action::Join(url)) => {
            let uri = crate::calendar::meeting::classify(url).map(|j| crate::calendar::meeting::launch_uri(&j)).unwrap_or_else(|| url.clone());
            let actions = [(tr("Připojit", "Join").to_string(), uri)];
            crate::toast::show_xml(&app_id, crate::toast::toast_xml_actions(&n.title, &n.body, &actions));
        }
        _ => crate::toast::show(&app_id, n.title.clone(), n.body.clone()),
    }
}

/// Vlákno `strip`: běžící oznámení doběhlo → další z fronty (nebo nic).
pub(crate) fn advance(acked: bool) {
    let next = with_q(|q| q.finish(acked));
    show(next);
}

/// `stripview::refresh_strip`: smí proužek vyjet? Zmizel-li (hra, šuplík přes něj), fronta se zahodí;
/// otevřený šuplík smaže tečku zmeškaných. Vrací barvu tečky zmeškaných.
pub(crate) fn stage(stage: bool, drawer_open: bool) -> Option<Rgba> {
    let was = STAGE.swap(stage, std::sync::atomic::Ordering::Relaxed);
    let (missed, drop_live) = with_q(|q| {
        let busy = q.cur.is_some();
        if !stage && busy {
            q.drop_all();
        }
        if drawer_open {
            q.clear_missed();
        }
        (q.missed(), !stage && (busy || was))
    });
    if drop_live {
        *HIT.lock() = Hit::default();
        core().strip.set(|m| m.notice = None);
    }
    missed.map(Kind::color)
}

/// Ukázka ze Studia (tlačítko „Vyzkoušet“): hned v proužku, se smyšleným obsahem; tlačítko jen zavře.
pub(crate) fn preview(kind: Kind, style: Option<NotchStyle>) -> Result<(), String> {
    let c = core();
    if c.dead() {
        return Err(tr("Ledge je teď ztlumený (hra přes celou obrazovku nebo herní režim).", "Ledge is muted right now (fullscreen game or game mode).").into());
    }
    if !STAGE.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(tr("Proužek teď není vidět (vypnutý, nebo je šuplík otevřený přes něj).", "The strip isn't visible right now (turned off, or the drawer covers it).").into());
    }
    let n = Notice { preview: true, ..sample(kind, crate::state::now_ms()) };
    with_q(|q| q.preview(n.clone()));
    show_as(Some(n), style);
    Ok(())
}

/// Smyšlený obsah ukázky (jména jen `app`, odkazy jen `example.com`).
pub fn sample(kind: Kind, now: i64) -> Notice {
    let base = Notice { kind, count: 1, ..Notice::default() };
    match kind {
        Kind::Waiting => Notice { title: tr("Claude čeká na schválení", "Claude is waiting for approval").into(), body: "app · Bash".into(), button: Some(tr("Otevřít", "Open").into()), ..base },
        Kind::Done => Notice { title: tr("Claude je hotový", "Claude is done").into(), body: "app".into(), ..base },
        Kind::Meeting => Notice {
            title: tr("Schůzka za 5 min", "Meeting in 5 min").into(),
            body: tr("Ukázková schůzka", "Sample meeting").into(),
            button: Some(tr("Připojit", "Join").into()),
            start_ms: Some(now + 5 * 60_000),
            ..base
        },
        Kind::Limit => Notice { title: tr("Limit Claude se blíží", "Claude limit is getting close").into(), body: tr("Limit 5h je na 82 % (práh 80 %).", "The 5h limit is at 82% (80% threshold).").into(), pct: Some(82.0), ..base },
        Kind::Timer => Notice { title: tr("Časovač doběhl", "Timer finished").into(), body: tr("Odpočet 25 min skončil.", "The 25 min countdown is over.").into(), ..base },
    }
}

/// Vlákno `hover`: drží kurzor na oznámení s tlačítkem? Pak se šuplík najetím neotevře a oznámení se
/// nesloží, dokud na něm kurzor je (nejméně ještě `HOVER_KEEP`).
pub(crate) fn owns_pointer(x: i32, y: i32) -> bool {
    let hit = *HIT.lock();
    let (Some(shape), Some(_)) = (hit.shape, hit.button) else { return false };
    let inside = x >= shape.left && x < shape.right && y >= shape.top && y < shape.bottom;
    if inside {
        core().strip.set(|m| {
            if let Some(l) = m.notice.as_mut() {
                let keep_from = choreo::OPEN_MS + l.hold.saturating_sub(HOVER_KEEP.as_millis() as u64);
                if l.elapsed() > keep_from {
                    l.start = Instant::now() - Duration::from_millis(keep_from);
                }
            }
        });
    }
    inside
}

/// Kolik oznámení ještě zbývá, když z něj kurzor odjede.
const HOVER_KEEP: Duration = Duration::from_millis(1200);

/// Okno proužku: kde je teď klikací tvar (pro `WM_NCHITTEST`); `None` = propouštět myš.
pub(crate) fn hit() -> Hit {
    *HIT.lock()
}

pub(crate) fn set_hit(h: Hit) {
    *HIT.lock() = h;
}

/// Klik (uvolnění levého tlačítka) v okně proužku na bodě `x, y` (px obrazovky). Tlačítko → jeho cesta;
/// jinde v tvaru → oznámení se hned složí. Nikdy nic nepovoluje.
pub(crate) fn click(x: i32, y: i32) {
    let hit = *HIT.lock();
    let inside = |r: Option<RECT>| r.is_some_and(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom);
    if !inside(hit.shape) {
        return;
    }
    let mut action = None;
    core().strip.set(|m| {
        if let Some(l) = m.notice.as_mut() {
            if inside(hit.button) {
                action = l.n.action.clone();
            }
            l.acked = true;
            // hned do fáze zavírání
            let close_at = choreo::OPEN_MS + l.hold;
            if l.elapsed() < close_at {
                l.start = Instant::now() - Duration::from_millis(close_at);
            }
        }
    });
    *HIT.lock() = Hit::default();
    let Some(action) = action else { return };
    let _ = std::thread::Builder::new().name("notify-click".into()).spawn(move || {
        let r = match action {
            Action::Session(id) => crate::commands::sessions::session_open(id),
            Action::Join(url) => crate::commands::calendar::calendar_join(url),
        };
        if let Err(e) = r {
            log(&format!("notify: klik ({e})"));
        }
    });
}

/// IPC ze Studia: ukázka oznámení `kind` (`waiting` / `done` / `meeting` / `limit` / `timer`), volitelně
/// ve stylu `style` z rozpracovaného nastavení (jinak z uloženého configu).
#[tauri::command]
pub(crate) fn notify_preview(kind: String, style: Option<NotchStyle>) -> Result<(), String> {
    let k = Kind::from_id(&kind).ok_or_else(|| tr("Neznámá událost.", "Unknown event.").to_string())?;
    preview(k, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_and_samples_are_fictional() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_id(k.id()), Some(k));
            let s = sample(k, 0);
            assert_eq!(s.kind, k);
            assert!(s.action.is_none(), "ukázka nikam nevede");
            assert!(!s.title.is_empty());
            // tlačítko jen u Čeká a Schůzky; Povolit nikdy
            assert_eq!(s.button.is_some(), matches!(k, Kind::Waiting | Kind::Meeting), "{k:?}");
            assert!(!s.button.unwrap_or_default().to_lowercase().contains("povol"));
        }
        assert_eq!(Kind::from_id("povolit"), None);
        let ranks: Vec<u8> = [Kind::Waiting, Kind::Meeting, Kind::Limit, Kind::Timer, Kind::Done].iter().map(|k| k.rank()).collect();
        assert!(ranks.windows(2).all(|w| w[0] < w[1]));
    }
}
