//! Limity Claude za běhu Ledge: cache vzorku ze dvou souborů (vlastní `ratelimits.json` od wrapperu
//! `statusline.rs`, záložně `~/.claude/usage-log/latest.json`), stav zapojení stavového řádku,
//! připomínka resetu kvóty (peek + zelené bliknutí) a segment proužku. Čistá logika v `limits.rs`.
//!
//! Žádné pollování v klidu: watcher (`notify`) na obou složkách nastaví `CHANGED` a probudí emit
//! vlákno; to volá `tick` (levný stat, čtení jen při změně mtime/délky, jinak nejvýš 1× za
//! `STAT_EVERY` při probuzení, které stejně nastalo) a do `next_wake` přidá `next_deadline` (reset
//! okna na 100 %, konec připomínky). `view` a `strip_segments` čtou jen paměť.

use crate::drawer::{Opener, View};
use crate::limits::{self, Armed, Kind, Sample, Source};
use crate::state::{UiCmd, core, mark_dirty};
use crate::strip::{Rgba, Segment};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Jak dlouho drží peek připomínky resetu.
pub const NOTICE_MS: i64 = 15_000;
/// Stat souborů bez události watcheru nejvýš takhle často (jen při probuzení, které nastalo stejně).
const STAT_EVERY: Duration = Duration::from_secs(30);

type FileKey = Option<(Option<SystemTime>, u64)>;

#[derive(Default)]
struct Live {
    own_key: FileKey,
    log_key: FileKey,
    settings_key: FileKey,
    own: Option<Sample>,
    log: Option<Sample>,
    /// none | foreign | full | error | "" (ještě nečteno)
    status_line: &'static str,
    armed: Armed,
    /// už ohlášené prahy 80 % / 90 % (`limits::alerts_due`)
    alerted: limits::Alerted,
    /// běžící připomínka resetu: okno a do kdy (ms)
    notice: Option<(Kind, i64)>,
    last_stat: Option<Instant>,
    /// historie 5h okna pro tempo (`limits::push_hist`), plní se jen s volbou `claude.limitsPace`
    hist: Vec<(i64, f64)>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);
static CHANGED: AtomicBool = AtomicBool::new(true);

fn with<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    let mut g = LIVE.lock();
    f(g.get_or_insert_with(Live::default))
}

/// `~/.claude/usage-log/latest.json` (cizí nástroj; Ledge ho jen čte, když existuje).
pub fn usage_log_path() -> Option<PathBuf> {
    crate::sessionlive::claude_dir().map(|d| d.join("usage-log").join("latest.json"))
}

fn key(p: &Option<PathBuf>) -> FileKey {
    let m = std::fs::metadata(p.as_ref()?).ok()?;
    Some((m.modified().ok(), m.len()))
}

fn read_sample(p: &Option<PathBuf>, src: Source) -> Option<Sample> {
    limits::from_file(&std::fs::read_to_string(p.as_ref()?).ok()?, src)
}

/// Po zapojení / odpojení v Nastavení: stav přečíst hned při dalším `tick`.
pub fn invalidate() {
    CHANGED.store(true, Ordering::Relaxed);
    with(|l| l.settings_key = Some((None, u64::MAX)));
    mark_dirty();
}

/// Stat + čtení změněných souborů. Vrací „změnilo se něco pro UI?“.
fn reload(l: &mut Live) -> bool {
    let own_p = Some(crate::statusline::sample_path());
    let log_p = usage_log_path();
    let set_p = crate::hooksetup::settings_path();
    let mut changed = false;
    let k = key(&own_p);
    if k != l.own_key {
        l.own_key = k;
        l.own = read_sample(&own_p, Source::StatusLine);
        changed = true;
    }
    let k = key(&log_p);
    if k != l.log_key {
        l.log_key = k;
        l.log = read_sample(&log_p, Source::UsageLog);
        changed = true;
    }
    let k = key(&set_p);
    if k != l.settings_key || l.status_line.is_empty() {
        l.settings_key = k;
        let st = match set_p {
            Some(p) => crate::statuslinesetup::status_at(&p, &crate::statuslinesetup::store_path()).state,
            None => "error",
        };
        changed |= st != l.status_line;
        l.status_line = st;
    }
    changed
}

/// Volá emit vlákno při každém probuzení. Levné: bez události watcheru jen porovnání času.
pub(crate) fn tick(now: i64) {
    let c = core();
    let (enabled, fullscreen, pace, alert) = (c.cfg.read().claude.limits_reminder, c.fullscreen.load(Ordering::Relaxed), c.cfg.read().claude.limits_pace, c.cfg.read().claude.limits_alert);
    let mut alerts: Vec<(Kind, u8, f64)> = Vec::new();
    let mut dirty = false;
    let mut show = false;
    let mut hide = false;
    with(|l| {
        let forced = CHANGED.swap(false, Ordering::Relaxed);
        if forced || l.last_stat.is_none_or(|t| t.elapsed() >= STAT_EVERY) {
            l.last_stat = Some(Instant::now());
            dirty |= reload(l);
        }
        if let Some(s) = limits::fresher(l.own, l.log) {
            limits::arm(&mut l.armed, &s, now);
            if let (true, Some(w)) = (pace, s.five_hour) {
                dirty |= limits::push_hist(&mut l.hist, s.at, w.pct);
            }
        }
        // upozornění 80 / 90 % jen z čerstvého vzorku; vypnuto = nic se nepamatuje
        if alert {
            if let Some(s) = limits::fresher(l.own, l.log).filter(|s| now - s.at <= limits::STALE_MS) {
                alerts = limits::alerts_due(&mut l.alerted, &s);
            }
        } else {
            l.alerted.clear();
        }
        let fired = limits::due(&mut l.armed, now);
        // vypnuto / fullscreen = připomínka propadne (nic se neodkládá)
        if let Some(&k) = fired.first()
            && enabled
            && !fullscreen
        {
            l.notice = Some((k, now + NOTICE_MS));
            show = true;
            dirty = true;
        }
        if l.notice.is_some_and(|(_, until)| until <= now) {
            l.notice = None;
            hide = true;
            dirty = true;
        }
    });
    if let Some(&(k, lvl, pct)) = alerts.iter().max_by_key(|a| a.1)
        && crate::attention::signal(Rgba::AMBER, true)
    {
        let name = match k {
            Kind::FiveHour => crate::i18n::tr("5h", "5h"),
            Kind::SevenDay => crate::i18n::tr("týdenní", "weekly"),
        };
        let title = crate::i18n::tr("Limit Claude se blíží", "Claude limit is getting close").to_string();
        let body = trf!("Limit {} je na {} % (práh {} %).", "The {} limit is at {}% ({}% threshold).", name, pct.round() as i64, lvl);
        crate::toast::show(&c.app.config().identifier, title, body);
    }
    if show {
        *c.flash.lock() = Some((Rgba::GREEN, Instant::now()));
        if c.drawer.view() == View::Hidden {
            c.send_ui(UiCmd::Show(View::Peek, Opener::Hover));
        }
    }
    if hide && c.drawer.view() == View::Peek && !c.claude.attention() && c.notes.lock().due.is_empty() {
        c.send_ui(UiCmd::Hide);
    }
    if dirty {
        mark_dirty();
    }
}

/// Termín pro `threads::emit::next_wake`: reset okna na 100 % nebo konec připomínky.
pub(crate) fn next_deadline() -> Option<i64> {
    with(|l| [limits::next_reset(&l.armed), l.notice.map(|(_, u)| u + 1)].into_iter().flatten().min())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsView {
    /// kdy vznikl zobrazený vzorek (ms); `None` = žádná data
    pub at: Option<i64>,
    pub source: Option<Source>,
    pub five_hour: Option<limits::Window>,
    pub seven_day: Option<limits::Window>,
    /// vzorek starší než tohle UI ukáže jako „staré“ (`limits::STALE_MS`)
    pub stale_ms: i64,
    /// zapojení wrapperu v settings.json: none | foreign | full | error | "" (nečteno)
    pub status_line: &'static str,
    /// běžící připomínka resetu (peek): které okno se obnovilo
    pub notice: Option<Kind>,
    /// volba `claude.limitsPace`: UI kreslí rysku plánu a barví tempo
    pub pace: bool,
    /// vzorky 5h okna `[at, pct]` pro tempo (prázdné bez volby `limitsPace`)
    pub history5h: Vec<[f64; 2]>,
}

/// Pro snapshot — jen paměť.
pub fn view() -> LimitsView {
    let pace = core().cfg.read().claude.limits_pace;
    with(|l| {
        let s = limits::fresher(l.own, l.log);
        LimitsView {
            at: s.map(|s| s.at),
            source: s.map(|s| s.source),
            five_hour: s.and_then(|s| s.five_hour),
            seven_day: s.and_then(|s| s.seven_day),
            stale_ms: limits::STALE_MS,
            status_line: l.status_line,
            notice: l.notice.map(|(k, _)| k),
            pace,
            history5h: if pace { l.hist.iter().map(|&(t, p)| [t as f64, p]).collect() } else { Vec::new() },
        }
    })
}

/// Segment proužku (`strip.items` obsahuje `limits`) — jen paměť; bez čerstvých dat nic.
pub(crate) fn strip_segments(segs: &mut Vec<Segment>) {
    let now = crate::state::now_ms();
    let Some(s) = with(|l| limits::fresher(l.own, l.log)) else { return };
    let clock = |ms: i64| {
        use chrono::{Local, TimeZone};
        Local.timestamp_millis_opt(ms).single().map(|d| d.format("%H:%M").to_string()).unwrap_or_default()
    };
    let Some(text) = limits::strip_text(&s, now, &clock) else { return };
    let dot = match limits::top_fresh(&s, now) {
        Some(p) if p >= 90.0 => Rgba::RED,
        Some(p) if p >= 70.0 => Rgba::AMBER,
        Some(_) => Rgba::GREEN,
        None => Rgba::TEAL,
    };
    segs.push(crate::stripview::seg(text, Rgba::DIM, Some(dot), false, false));
}

/// Watcher složek se vzorky (nerekurzivní, filtr na jméno souboru) → `CHANGED` + probudit emit.
/// Složka usage-log, která ještě neexistuje, se nesleduje — změnu chytí `STAT_EVERY`.
pub(crate) fn start_watch() {
    use notify::{RecursiveMode, Watcher};
    let own = crate::statusline::sample_path();
    let dirs: Vec<PathBuf> = [own.parent().map(PathBuf::from), usage_log_path().and_then(|p| p.parent().map(PathBuf::from))].into_iter().flatten().collect();
    if let Some(d) = dirs.first() {
        let _ = std::fs::create_dir_all(d);
    }
    std::thread::Builder::new()
        .name("limits-watch".into())
        .spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let hit = res.is_ok_and(|e| e.paths.iter().any(|p| p.file_name().is_some_and(|n| n == "ratelimits.json" || n == "latest.json")));
                if hit {
                    let _ = tx.send(());
                }
            });
            let Ok(mut w) = w else { return };
            for d in &dirs {
                if d.is_dir() {
                    let _ = w.watch(d, RecursiveMode::NonRecursive);
                }
            }
            while rx.recv().is_ok() {
                // tmp + rename = víc událostí za sebou → krátký klid
                while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}
                CHANGED.store(true, Ordering::Relaxed);
                mark_dirty();
            }
            drop(w);
        })
        .ok();
}
