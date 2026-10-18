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
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1 << 20) {
        let _ = std::fs::remove_file(&path);
    }
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
    /// rychlá poznámka: Full s fokusem rovnou na stránce „note“
    Note,
    /// klik do šuplíku, který chce psát (stránka „note“): aktivovat okno, blur pak zavře
    Focus,
    /// odchod ze stránky „note“: opener zpět na hodnotu před `Focus`
    Release,
}

pub(crate) struct Core {
    pub(crate) app: AppHandle,
    pub(crate) cfg: RwLock<Config>,
    pub(crate) cfg_error: Mutex<Option<String>>,
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
    pub(crate) fullscreen: AtomicBool,
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
}

pub(crate) static CORE: OnceLock<Arc<Core>> = OnceLock::new();

pub(crate) fn core() -> &'static Arc<Core> {
    CORE.get().expect("core")
}

pub(crate) fn mark_dirty() {
    if let Some(c) = CORE.get() {
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

    /// Monitor proužku (`strip.monitor`) z cache `MonitorCache` — výčet nejvýš jednou za `MONITOR_TTL`.
    pub(crate) fn monitor(&self) -> screen::Monitor {
        let cfg = self.cfg.read();
        self.monitors.lock().get(&cfg.strip.monitor, Instant::now(), pick_monitor)
    }

    /// Jako `monitor`, ale vždy čerstvý výčet (vlákno `hover` co 3 s — tím se pozná změna zobrazení).
    pub(crate) fn monitor_fresh(&self) -> screen::Monitor {
        let cfg = self.cfg.read();
        let mut cache = self.monitors.lock();
        *cache = MonitorCache::default();
        cache.get(&cfg.strip.monitor, Instant::now(), pick_monitor)
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

    pub(crate) fn rebuild_index(&self) {
        let roots = self.cfg.read().roots.clone();
        *self.index.lock() = RepoIndex::build(&roots);
        refresh_logos(true);
    }
}

/// Jak dlouho platí výčet monitorů v `MonitorCache`: připojení monitoru / změnu DPI jádro pozná
/// nejpozději za tuto dobu (WM_DISPLAYCHANGE nikde nechytáme; `hover` navíc obnovuje co 3 s).
pub(crate) const MONITOR_TTL: Duration = Duration::from_secs(3);

/// Délka zeleného bliknutí proužku (`Core.flash`) po uložení poznámky / startu serveru.
pub(crate) const FLASH: Duration = Duration::from_secs(4);

/// Poslední výsledek `screen::pick` (EnumDisplayMonitors + DPI každého monitoru) pro daný spec.
/// `refresh_strip` a vlákno `ui` se ptají při každém volání — výčet jen při změně specu nebo po TTL.
#[derive(Default)]
pub(crate) struct MonitorCache(Option<(String, Instant, screen::Monitor)>);

impl MonitorCache {
    pub(crate) fn get(&mut self, spec: &str, now: Instant, fetch: impl FnOnce(&str) -> screen::Monitor) -> screen::Monitor {
        if let Some((s, at, m)) = &self.0
            && s == spec
            && now.saturating_duration_since(*at) < MONITOR_TTL
        {
            return *m;
        }
        let m = fetch(spec);
        self.0 = Some((spec.to_string(), now, m));
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
    })
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub(crate) fn find_project(id: &str) -> Result<config::Project, String> {
    core().cfg.read().projects.iter().find(|p| p.id == id).cloned().ok_or_else(|| trf!("Neznámý projekt {id}", "Unknown project {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(handle: isize) -> screen::Monitor {
        screen::Monitor { handle, rect: Default::default(), work: Default::default(), primary: true, dpi: 96 }
    }

    #[test]
    fn monitor_cache_refetches_only_on_spec_change_or_ttl() {
        let t0 = Instant::now();
        let mut cache = MonitorCache::default();
        let mut fetches = 0;
        // (spec, čas, co by vrátil výčet, čekaný výsledek)
        let steps = [
            ("primary", 0, 1, 1),    // prázdná cache → výčet
            ("primary", 1_000, 2, 1), // v TTL → z cache
            ("1", 1_500, 3, 3),       // jiný spec → výčet
            ("1", 4_499, 4, 3),       // těsně před TTL
            ("1", 4_500, 5, 5),       // TTL vypršelo → výčet
        ];
        for (spec, at, fresh, want) in steps {
            let got = cache.get(spec, t0 + Duration::from_millis(at), |_| {
                fetches += 1;
                mon(fresh)
            });
            assert_eq!(got.handle, want, "{spec} @ {at} ms");
        }
        assert_eq!(fetches, 3);
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
            black_box(cache.get(black_box("primary"), now, pick_monitor));
        }
        let hit = t.elapsed() / N;
        println!("výčet monitorů {fetch:?}/volání, cache {hit:?}/volání");
    }
}
