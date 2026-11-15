//! Sdílený stav jádra: `Core` (config, procesy, Claude, proužek, šuplík, cache) za `core()`,
//! příkazy pro vlákno `ui` (`UiCmd`), probuzení emit vlákna (`mark_dirty`) a diagnostický `log`.
//! Závisí jen na modulech domén (config, drawer, procman, claude, notesync, update…), ne na vláknech.

use crate::claude::Claude;
use crate::config::{self, Config};
use crate::drawer::{Drawer, Opener, View};
use crate::gitinfo::GitInfo;
use crate::procman::ProcMan;
use crate::resolve::RepoIndex;
use crate::strip::{self, Rgba, Strip};
use crate::refresh::refresh_logos;
use crate::threads::notes_sync::SyncMsg;
use crate::threads::notes_watch::{NotesMsg, NotesState};
use crate::{icons, notesync, personal, screen, update, weather};
use parking_lot::{Condvar, Mutex, RwLock};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;

// ---------- log ----------

/// Diagnostický log `%APPDATA%/Ledge/ledge.log` (max ~1 MB, pak se přepíše). Release nemá konzoli.
pub(crate) fn log(msg: &str) {
    use std::io::Write;
    let path = config::config_dir().join("ledge.log");
    crate::guard::rotate_log(&path); // L226: nad 1 MB → `ledge.log.1`, ne smazat
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{} {msg}", local_hhmmss());
    }
}

fn local_hhmmss() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let t = unsafe { GetLocalTime() };
    format!("{:02}:{:02}:{:02}.{:03}", t.wHour, t.wMinute, t.wSecond, t.wMilliseconds)
}

// ---------- sdílený stav ----------

pub(crate) enum UiCmd {
    Show(View, Opener),
    /// myš se blíží k okraji → připravit WebView2 předem
    Warm,
    Hide,
    /// výslovné zavření (Esc, ✕, uložení poznámky, zkratka znovu): jako `Hide`, ale fokus se vrátí
    /// oknu, odkud si ho Ledge vzal (`focusreturn`)
    Close,
    Size(View, f64, f64),
    /// velikost z `appearance.size` se změnila (tažení v úpravách, přepnutí záložky s „Pamatovat velikost
    /// pro každou záložku“): otevřený Full dorovnat; `true` = animovaně (`motion::anim_ms_now`)
    Resize(bool),
    /// totéž během tažení (`drawer_size(…, done = false)`): bez `refresh_strip` každý snímek, proužek se
    /// obnoví až po puštění (`Resize(false)`)
    ResizeLive,
    /// rychlá poznámka: Full s fokusem rovnou na stránce „note“
    Note,
    /// klik do šuplíku, který chce psát (stránka „note“): aktivovat okno, blur pak zavře
    Focus,
    /// odchod ze stránky „note“: opener zpět na hodnotu před `Focus`
    Release,
    /// 📌 v docku: upevnit (`true`) / uvolnit (`false`) — `Drawer::pin`
    Pin(bool),
}

pub(crate) struct Core {
    pub(crate) app: AppHandle,
    pub(crate) cfg: RwLock<Config>,
    pub(crate) cfg_error: Mutex<Option<String>>,
    /// Chyba mostu Clauda (`claude::serve` při startu); `apply_config` ji nepřepisuje (to dělá `cfg_error`).
    pub(crate) bridge_error: Mutex<Option<String>>,
    pub(crate) procs: ProcMan,
    pub(crate) claude: Claude,
    pub(crate) strip: Arc<Strip>,
    pub(crate) drawer: Drawer,
    pub(crate) index: Mutex<RepoIndex>,
    pub(crate) git: Mutex<HashMap<String, (Instant, Option<GitInfo>)>>,
    /// loga dlaždic — počítá vlákno `logos` (`refresh_logos`), snapshot čte jen klíč
    pub(crate) logos: Mutex<icons::Store>,
    pub(crate) mode: Mutex<String>,
    pub(crate) dirty: Mutex<bool>,
    pub(crate) wake: Condvar,
    pub(crate) flash: Mutex<Option<(Rgba, Instant)>>,
    /// něco leží přes celou obrazovku NĚKDE (`threads/fswatch.rs`); pozadí a globální zkratky podle toho
    pub(crate) fullscreen: AtomicBool,
    /// stav fullscreenu per monitor (`fswatch::FsMon`); povrchy UI podle toho (`dead_on`, `dead_surface`)
    pub(crate) fs_mon: Mutex<Vec<crate::threads::fswatch::FsMon>>,
    /// monitor (`handle`), na kterém je šuplík otevřený / se právě otevírá (L178); hover smyčka ho nastaví těsně
    /// před `send_ui`, vlákno `ui` ho potvrdí při Show a smaže při schování (`threads/tracker.rs::choose_monitor`)
    pub(crate) opened_on: Mutex<Option<isize>>,
    /// ruční herní režim (`gamemode.rs`) — Ledge mrtvý jako ve fullscreen, dokud se nevypne
    pub(crate) game_mode: AtomicBool,
    /// v popředí je „lištové“ okno (maximalizovaný prohlížeč na monitoru proužku, trigger "push")
    pub(crate) bar_window: AtomicBool,
    pub(crate) approve_keys: AtomicBool,
    pub(crate) ui: Mutex<mpsc::Sender<UiCmd>>,
    pub(crate) notes: Mutex<NotesState>,
    pub(crate) notes_tx: Mutex<mpsc::Sender<NotesMsg>>,
    /// synchronizace poznámek přes git (`notesync.rs`): stav pro UI, kanál vlákna `notes-sync`
    pub(crate) notes_sync: Mutex<notesync::Status>,
    pub(crate) sync_tx: Mutex<mpsc::Sender<SyncMsg>>,
    /// do kdy (ms) jsou změny v kořeni poznámek od syncu samotného (watcher je nehlásí jako lokální)
    pub(crate) sync_quiet_ms: std::sync::atomic::AtomicI64,
    /// stránka, na kterou se má šuplík otevřít (studený WebView událost `page` nestihne)
    pub(crate) pending_page: Mutex<Option<&'static str>>,
    /// právě zaregistrované globální zkratky (akce, zápis) — viz `sync_hotkeys`
    pub(crate) hotkeys: Mutex<Vec<(&'static str, String)>>,
    pub(crate) weather: Mutex<WeatherState>,
    /// klasifikace cílů rychlých odkazů (stat disku) podle targetu — počítá vlákno `links`,
    /// snapshot jen čte; (generace, mapa) — starší výpočet nepřepíše novější
    pub(crate) links: Mutex<(u64, HashMap<String, personal::LinkTarget>)>,
    pub(crate) links_gen: std::sync::atomic::AtomicU64,
    /// kontrola nové verze (vlastní vlákno) a instalace na klik
    pub(crate) updater: update::Updater,
    /// monitor proužku z posledního výčtu (`Core::monitor`)
    pub(crate) monitors: Mutex<MonitorCache>,
}

/// Počasí záložky Osobní — poslední dobrá data drží i přes chybu dalšího pokusu.
#[derive(Default)]
pub(crate) struct WeatherState {
    /// místo, ke kterému patří data (změna místa = zahodit)
    pub(crate) place: Option<(f64, f64)>,
    pub(crate) data: Option<weather::Weather>,
    pub(crate) fetched_ms: Option<i64>,
    pub(crate) attempt_ms: Option<i64>,
    pub(crate) error: Option<String>,
    pub(crate) loading: bool,
    /// neúspěchy za sebou → backoff (`netstate`)
    pub(crate) fails: crate::netstate::Fails,
}

pub(crate) static CORE: OnceLock<Arc<Core>> = OnceLock::new();

pub(crate) fn core() -> &'static Arc<Core> {
    CORE.get().expect("core")
}

#[track_caller]
pub(crate) fn mark_dirty() {
    if let Some(c) = CORE.get() {
        crate::threads::emitgate::note_source(std::panic::Location::caller());
        *c.dirty.lock() = true;
        c.wake.notify_one();
    }
}

/// Win11 akryl jen od buildu 22523 a jen když má šéf zapnuté efekty průhlednosti.
pub(crate) fn screen_glass_capable() -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let build: u32 = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let transparency: u32 = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value("EnableTransparency"))
        .unwrap_or(1);
    build >= 22523 && transparency == 1
}

impl Core {
    /// Je Ledge mrtvý? Fullscreen hra nebo herní režim (`gamemode::is_dead`).
    pub(crate) fn dead(&self) -> bool {
        crate::gamemode::is_dead(self.fullscreen.load(Ordering::Relaxed), self.game_mode.load(Ordering::SeqCst))
    }

    /// Je povrch UI (okno, proužek, oznámení) na monitoru `handle` mrtvý? Fullscreen jen na něm, nebo herní
    /// režim (`threads/fswatch.rs`). Práce na pozadí a zkratky se ptají `dead()` (kdekoli).
    pub(crate) fn dead_on(&self, handle: isize) -> bool {
        let any = self.fullscreen.load(Ordering::Relaxed);
        let gm = self.game_mode.load(Ordering::SeqCst);
        // bez fullscreenu a herního režimu žádný zámek ani výčet
        (any || gm) && crate::threads::fswatch::dead_on(gm, any, &self.fs_mon.lock(), handle)
    }

    /// Povrchy u proužku (šuplík, peek, oznámení, proužek, okna pod proužkem): mrtvé na monitoru proužku.
    pub(crate) fn dead_surface(&self) -> bool {
        if !self.dead() {
            return false;
        }
        self.dead_on(self.monitor().handle)
    }

    /// Monitor bodu (levý horní roh okna) podle mapy fullscreenu; mimo všechny = monitor proužku.
    pub(crate) fn handle_at(&self, x: i32, y: i32) -> isize {
        crate::threads::fswatch::handle_at(&self.fs_mon.lock(), x, y).unwrap_or_else(|| self.monitor().handle)
    }

    /// Co leží přes monitor `handle` — pro lístky a widgety (`pins::pins_yield`).
    pub(crate) fn fs_kind_on(&self, handle: isize) -> crate::pins::FsKind {
        crate::threads::fswatch::kind_on(self.fullscreen.load(Ordering::Relaxed), &self.fs_mon.lock(), handle)
    }

    /// Monitor proužku (`strip.monitor`) z cache `MonitorCache` — výčet jen při změně specu, po události
    /// zobrazení (`display_event`) nebo po pojistné lhůtě `MONITOR_TTL`.
    pub(crate) fn monitor(&self) -> screen::Monitor {
        let cfg = self.cfg.read();
        self.monitors.lock().get(&cfg.strip.monitor, Instant::now(), display_gen(), pick_monitor)
    }

    /// Monitor šuplíku: kde je otevřený (`opened_on`, jen mezi proužky: hlavní + tečky), jinak monitor proužku.
    pub(crate) fn drawer_monitor(&self) -> screen::Monitor {
        let main = self.monitor();
        let Some(h) = *self.opened_on.lock() else { return main };
        if h == main.handle {
            return main;
        }
        let spec = self.cfg.read().strip.monitor.clone();
        crate::stripset::secondary_monitors(&spec).into_iter().find(|m| m.handle == h).unwrap_or(main)
    }

    /// Mezera pod proužkem pro nová okna (`screen::below_strip`): monitor `mon`, odstup `margin` (logické px).
    pub(crate) fn below_strip(&self, mon: &screen::Monitor, margin: f64) -> i32 {
        screen::below_strip(mon.rect.top, mon.work.top, self.strip_logical(mon).1, margin, mon.dpi as f64 / 96.0)
    }

    /// Velikost proužku v logických px (výchozí tvar morfu šuplíku).
    pub(crate) fn strip_logical(&self, mon: &screen::Monitor) -> (f64, f64) {
        let (w, h) = *self.strip.last.lock();
        let scale = mon.dpi as f64 / 96.0;
        if w <= 0 { (300.0, strip::HEIGHT as f64) } else { (w as f64 / scale, h as f64 / scale) }
    }

    pub(crate) fn send_ui(&self, cmd: UiCmd) {
        let _ = self.ui.lock().send(cmd);
    }

    /// Odkud projekt poběží (`resolve::place_of`): `path` vyhrává i neexistující (chyba, ne tichý pád jinam),
    /// bez `path` jediný klon podle `remote`, víc klonů = výběr člověkem. Bez `subdir`.
    pub(crate) fn project_place(&self, p: &config::Project) -> (crate::resolve::Place, Option<PathBuf>) {
        let is_dir = p.path.as_deref().is_some_and(|d| Path::new(d).is_dir());
        let idx = self.index.lock();
        let cands = match (&p.path, p.remote.as_deref()) {
            (None, Some(r)) => idx.candidates(r),
            _ => &[],
        };
        crate::resolve::place_of(p.path.as_deref(), is_dir, cands)
    }

    pub(crate) fn project_dir(&self, p: &config::Project) -> Option<PathBuf> {
        let base = self.project_place(p).1?;
        Some(match &p.subdir {
            Some(s) => base.join(s),
            None => base,
        })
    }

    /// Přestaví index repozitářů bez obnovy log; vrátí počet nalezených repozitářů.
    pub(crate) fn rescan_index(&self) -> usize {
        let roots = self.cfg.read().roots.clone();
        let idx = RepoIndex::build(&roots);
        let n = idx.len();
        *self.index.lock() = idx;
        *LAST_RESCAN.lock() = Some(Instant::now());
        n
    }

    /// Při otevření šuplíku: přestavba indexu na pozadí, nejvýš 1× za `RESCAN_MIN`.
    pub(crate) fn rescan_index_on_open(&self) {
        if !rescan_due(*LAST_RESCAN.lock(), Instant::now(), RESCAN_MIN) {
            return;
        }
        *LAST_RESCAN.lock() = Some(Instant::now());
        let c = core();
        std::thread::spawn(move || {
            c.rescan_index();
        });
    }

    pub(crate) fn rebuild_index(&self) {
        let roots = self.cfg.read().roots.clone();
        *self.index.lock() = RepoIndex::build(&roots);
        *LAST_RESCAN.lock() = Some(Instant::now());
        refresh_logos(true);
    }
}

/// Nejkratší odstup přestaveb indexu repozitářů spouštěných otevřením šuplíku.
pub(crate) const RESCAN_MIN: Duration = Duration::from_secs(60);
static LAST_RESCAN: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);

/// Je čas přestavět index? (nikdy → ano; jinak až po `min`)
pub(crate) fn rescan_due(last: Option<Instant>, now: Instant, min: Duration) -> bool {
    last.is_none_or(|l| now.saturating_duration_since(l) >= min)
}

#[cfg(test)]
mod rescan_tests {
    use super::*;

    #[test]
    fn rescan_due_table() {
        let t0 = Instant::now();
        let s = |n| Duration::from_secs(n);
        assert!(rescan_due(None, t0, s(60)));
        assert!(!rescan_due(Some(t0), t0 + s(59), s(60)));
        assert!(rescan_due(Some(t0), t0 + s(60), s(60)));
        assert!(!rescan_due(Some(t0 + s(5)), t0, s(60)));
    }
}

/// Pojistka výčtu monitorů v `MonitorCache` a vlákna `hover`: změnu zobrazení hlásí okno proužku
/// (WM_DISPLAYCHANGE / WM_DPICHANGED / WM_SETTINGCHANGE SPI_SETWORKAREA → `display_event`), takže tohle je jen
/// záchrana pro zprávu, která nedorazí (dřív polling 3 s).
pub(crate) const MONITOR_TTL: Duration = Duration::from_secs(60);

/// Počítadlo událostí zobrazení: okno proužku ho zvyšuje (`display_event`), `MonitorCache` a vlákno `hover`
/// porovnávají, jestli se od posledního čtení změnilo.
static DISPLAY_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Změnilo se zobrazení (monitor, rozlišení, DPI, pracovní plocha): zneplatní cache monitorů a vlákno `hover`
/// přepočítá proužek, šuplík a okna na ploše. Volá se z wndproc okna proužku — jen atomik, žádný zámek.
pub(crate) fn display_event() {
    DISPLAY_GEN.fetch_add(1, Ordering::Release);
}

pub(crate) fn display_gen() -> u64 {
    DISPLAY_GEN.load(Ordering::Acquire)
}

/// Délka zeleného bliknutí proužku (`Core.flash`) po uložení poznámky / startu serveru.
pub(crate) const FLASH: Duration = Duration::from_secs(4);

/// Poslední výsledek `screen::pick` (EnumDisplayMonitors + DPI každého monitoru) pro daný spec.
/// `refresh_strip` a vlákno `ui` se ptají při každém volání — výčet jen při změně specu nebo po TTL.
#[derive(Default)]
pub(crate) struct MonitorCache(Option<(String, Instant, u64, screen::Monitor)>);

impl MonitorCache {
    pub(crate) fn get(&mut self, spec: &str, now: Instant, gen_now: u64, fetch: impl FnOnce(&str) -> screen::Monitor) -> screen::Monitor {
        if let Some((s, at, g, m)) = &self.0
            && s == spec
            && *g == gen_now
            && now.saturating_duration_since(*at) < MONITOR_TTL
        {
            return *m;
        }
        let m = fetch(spec);
        self.0 = Some((spec.to_string(), now, gen_now, m));
        m
    }
}

/// `screen::pick`, bez monitorů (RDP bez displeje) výchozí 1920×1080.
fn pick_monitor(spec: &str) -> screen::Monitor {
    screen::pick(spec).unwrap_or(screen::Monitor {
        handle: 0,
        rect: windows::Win32::Foundation::RECT { left: 0, top: 0, right: 1920, bottom: 1080 },
        work: windows::Win32::Foundation::RECT { left: 0, top: 0, right: 1920, bottom: 1080 },
        primary: true,
        dpi: 96,
        device: [0; 32],
    })
}

pub(crate) use crate::util::now_ms_i64 as now_ms;

pub(crate) fn find_project(id: &str) -> Result<config::Project, String> {
    core().cfg.read().projects.iter().find(|p| p.id == id).cloned().ok_or_else(|| trf!("Neznámý projekt {id}", "Unknown project {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(handle: isize) -> screen::Monitor {
        screen::Monitor { handle, rect: Default::default(), work: Default::default(), primary: true, dpi: 96, device: [0; 32] }
    }

    #[test]
    fn monitor_cache_refetches_only_on_spec_change_or_ttl() {
        let t0 = Instant::now();
        let mut cache = MonitorCache::default();
        let mut fetches = 0;
        // (spec, čas ms, generace události zobrazení, co by vrátil výčet, čekaný výsledek)
        let steps = [
            ("primary", 0, 0, 1, 1),         // prázdná cache → výčet
            ("primary", 1_000, 0, 2, 1),     // v TTL → z cache
            ("1", 1_500, 0, 3, 3),           // jiný spec → výčet
            ("1", 61_499, 0, 4, 3),          // těsně před pojistkou
            ("1", 61_500, 0, 5, 5),          // pojistná lhůta vypršela → výčet
            ("1", 61_600, 0, 6, 5),          // z cache
            ("1", 61_700, 1, 7, 7),          // událost zobrazení (nová generace) → výčet hned
            ("1", 61_800, 1, 8, 7),          // tatáž generace → z cache
        ];
        for (spec, at, gen_now, fresh, want) in steps {
            let got = cache.get(spec, t0 + Duration::from_millis(at), gen_now, |_| {
                fetches += 1;
                mon(fresh)
            });
            assert_eq!(got.handle, want, "{spec} @ {at} ms");
        }
        assert_eq!(fetches, 4);
    }

    /// Kolik stojí výčet monitorů proti zásahu cache (čte jen Win32 API, žádné okno).
    #[test]
    #[ignore = "mikrobenchmark: cargo test monitor_cache_bench -- --ignored --nocapture"]
    fn monitor_cache_bench() {
        use std::hint::black_box;
        const N: u32 = 2_000;
        let t = Instant::now();
        for _ in 0..N {
            black_box(pick_monitor(black_box("primary")));
        }
        let fetch = t.elapsed() / N;
        let mut cache = MonitorCache::default();
        let now = Instant::now();
        let t = Instant::now();
        for _ in 0..N {
            black_box(cache.get(black_box("primary"), now, 0, pick_monitor));
        }
        let hit = t.elapsed() / N;
        println!("výčet monitorů {fetch:?}/volání, cache {hit:?}/volání");
    }
}
