//! Rozšíření `wall` — živá tapeta: WebView2 jako dítě `WorkerW` přes primární monitor (kolo 33, E2).
//! `Body::run` běží na hlavním vlákně procesu `ledge.exe --ext wall`: najde strom oken Progman/WorkerW
//! (`attach.rs` rozhoduje, co s oknem, `geom.rs` kam ho dát), vytvoří okno + WebView2 (`host/`), načte scénu
//! `wall.html` a pak jen čeká na zprávy: Ledge→ext (pauza, data, příkazy), stránka→Ledge (`req`),
//! `TaskbarCreated` (restart Průzkumníka) a změna monitorů → nové připnutí. Žádný vlastní časovač v klidu
//! (opakování po selhání připnutí jen dokud Průzkumník chybí).
//!
//! Zálohu `IDesktopWallpaper` (statický snímek při selhání připnutí) zatím nemáme: TODO fáze 1b — mění tapetu
//! uživatele, proto se bez jeho souhlasu nezkouší (kolo 33 zakazuje).

#![allow(unsafe_op_in_unsafe_fn)]

pub mod attach;
pub mod bg;
pub mod diag;
pub mod env;
pub mod geom;
#[cfg(test)]
mod live;

use super::child::{Body, InitData, Io, Out};
use super::host::webview::{self, Page};
use super::host::win;
use super::wire::{Down, Up};
use attach::{Action, Ours, ParentKind, Tree, attach_plan, choose_parent};
use geom::{R, rel_rect};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PCWSTR, w};

/// Scéna tapety (E3 ji nahradí). Vývoj bez překladu: `LEDGE_WALL_HTML=<cesta k souboru>`.
const WALL_HTML: &str = include_str!("wall.html");

/// Vlákno okna se probouzí: fronta událostí (Ledge, stránka) / nové připnutí.
pub const WM_WAKE: u32 = WM_APP + 1;
pub const WM_REATTACH: u32 = WM_APP + 2;
/// změna prostředí (zamčená obrazovka, baterie) — `env::handle` v obsluze watcheru
pub const WM_ENV: u32 = WM_APP + 3;
const RETRY_TIMER: usize = 1;
const RETRY_MS: u32 = 2000;

pub struct Wall;

impl Body for Wall {
    fn wants(&self) -> Vec<String> {
        vec!["media".into()]
    }
    fn run(&mut self, io: Io) -> i32 {
        run_wall(io)
    }
}

enum Event {
    Down(Down),
    Page(String),
}
type Queue = Arc<Mutex<VecDeque<Event>>>;

/// Čistě: má zpráva watcheru vyvolat nové připnutí? (`TaskbarCreated` = nový Průzkumník, změna monitorů,
/// časovač opakování.)
pub fn wants_reattach(msg: u32, wparam: usize, taskbar_created: u32) -> bool {
    msg == taskbar_created || msg == WM_DISPLAYCHANGE || (msg == WM_TIMER && wparam == RETRY_TIMER)
}

/// Skryté top-level okno (ne message-only — ty nevidí broadcast `TaskbarCreated`) probouzející vlákno `tid`.
/// Přijímá i zamčení stanice a změnu napájení (`env`, po odběru `env::subscribe`).
pub fn create_watcher(tid: u32, taskbar_created: u32, flags: Arc<env::Flags>) -> Option<HWND> {
    win::create_window(
        "LedgeWallWatcher",
        WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        WS_POPUP,
        (0, 0, 0, 0),
        None,
        Box::new(move |_, m, wp, lp| {
            if wants_reattach(m, wp.0, taskbar_created) {
                win::post_thread(tid, WM_REATTACH);
                return Some(LRESULT(0));
            }
            if env::handle(&flags, m, wp, lp) {
                win::post_thread(tid, WM_ENV);
                return Some(LRESULT(0));
            }
            None
        }),
    )
}

// ---------- strom oken ----------

#[derive(Clone, Copy, Default)]
struct Handles {
    progman: HWND,
    workerw_child: HWND,
    workerw_top: HWND,
}

fn child_of(p: HWND, class: PCWSTR) -> HWND {
    if p.0.is_null() {
        return HWND::default();
    }
    unsafe { FindWindowExW(Some(p), None, class, None) }.unwrap_or_default()
}

fn find_progman() -> HWND {
    unsafe extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
        if win::class_name(h) == "Progman" {
            *(l.0 as *mut HWND) = h;
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut r = HWND::default();
    // FindWindow z procesu spuštěného nástrojem vracelo NULL (spike) — EnumWindows ne
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut r as *mut _ as isize));
    }
    r
}

/// Starší sestavení: top-level `WorkerW` bez `SHELLDLL_DefView`, přes celou plochu.
fn find_top_workerw() -> HWND {
    let mut h = unsafe { FindWindowExW(None, None, w!("WorkerW"), None) }.unwrap_or_default();
    while !h.0.is_null() {
        let (l, _, r, _) = win::window_rect(h);
        if child_of(h, w!("SHELLDLL_DefView")).0.is_null() && r - l > 1000 {
            return h;
        }
        h = unsafe { FindWindowExW(None, Some(h), w!("WorkerW"), None) }.unwrap_or_default();
    }
    HWND::default()
}

fn handles() -> Handles {
    let progman = find_progman();
    Handles { progman, workerw_child: child_of(progman, w!("WorkerW")), workerw_top: find_top_workerw() }
}

fn tree_of(h: &Handles) -> Tree {
    Tree { progman: !h.progman.0.is_null(), workerw_child: !h.workerw_child.0.is_null(), workerw_top: !h.workerw_top.0.is_null(), ours: None }
}

/// Existující `WorkerW` nebo (jednou) výzva Progmanu `0x052C`, aby ho vytvořil.
fn ensure_workerw() -> Handles {
    let h = handles();
    let t = tree_of(&h);
    if !t.progman || t.workerw_child || t.workerw_top {
        return h;
    }
    let mut res = 0usize;
    unsafe {
        let _ = SendMessageTimeoutW(h.progman, 0x052C, WPARAM(0xD), LPARAM(1), SMTO_NORMAL, 1000, Some(&mut res));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    handles()
}

fn parent_hwnd(k: ParentKind, h: &Handles) -> HWND {
    match k {
        ParentKind::WorkerWChild => h.workerw_child,
        ParentKind::WorkerWTop => h.workerw_top,
        ParentKind::ProgmanBehindIcons => h.progman,
    }
}

fn primary_monitor() -> R {
    unsafe {
        let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(m, &mut mi).as_bool() {
            let r = mi.rcMonitor;
            return R::new(r.left, r.top, r.right, r.bottom);
        }
    }
    R::new(0, 0, 1920, 1080)
}

/// Poloha okna uvnitř rodiče (obrazovkový počátek klientské oblasti rodiče).
fn target_rect(parent: HWND) -> R {
    let mut o = POINT { x: 0, y: 0 };
    unsafe {
        let _ = ClientToScreen(parent, &mut o);
    }
    rel_rect(primary_monitor(), (o.x, o.y))
}

/// Okno pod ikonami: dítě Progmanu hned za `SHELLDLL_DefView`, jinak nahoře ve svém rodiči.
fn z_after(kind: ParentKind, h: &Handles) -> HWND {
    if kind == ParentKind::ProgmanBehindIcons {
        let dv = child_of(h.progman, w!("SHELLDLL_DefView"));
        if !dv.0.is_null() {
            return dv;
        }
        return HWND_BOTTOM;
    }
    HWND_TOP
}

// ---------- stav ----------

struct Live {
    hwnd: HWND,
    kind: ParentKind,
    page: Page,
}

struct State {
    out: Out,
    env: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment,
    init: InitData,
    html: String,
    queue: Queue,
    tid: u32,
    watcher: HWND,
    live: Option<Live>,
    /// pauza z jádra (hra / fullscreen); zamčení řeší `flags`
    paused: bool,
    flags: Arc<env::Flags>,
    dir: std::path::PathBuf,
    topics: HashMap<String, (u64, Value)>,
    ready_sent: bool,
    retrying: bool,
}

impl State {
    fn set_retry(&mut self, on: bool) {
        if on == self.retrying {
            return;
        }
        self.retrying = on;
        unsafe {
            if on {
                SetTimer(Some(self.watcher), RETRY_TIMER, RETRY_MS, None);
            } else {
                let _ = KillTimer(Some(self.watcher), RETRY_TIMER);
            }
        }
    }

    fn layout(&self, h: &Handles) {
        let Some(l) = &self.live else { return };
        let r = target_rect(parent_hwnd(l.kind, h));
        unsafe {
            let _ = SetWindowPos(l.hwnd, Some(z_after(l.kind, h)), r.l, r.t, r.w(), r.h(), SWP_NOACTIVATE);
        }
        l.page.fit(l.hwnd);
        diag::set_target(l.hwnd, parent_hwnd(l.kind, h));
    }

    /// Srovná okno se stromem podle `attach_plan`.
    fn sync(&mut self) {
        let h = ensure_workerw();
        let mut tree = tree_of(&h);
        if let (Some(l), Some(want)) = (&self.live, choose_parent(&tree)) {
            let alive = unsafe { IsWindow(Some(l.hwnd)).as_bool() };
            let same = alive && unsafe { GetParent(l.hwnd) }.is_ok_and(|p| p == parent_hwnd(want, &h));
            tree.ours = Some(Ours { alive, parent: l.kind, same_parent: same });
        }
        match attach_plan(&tree) {
            Action::Fallback => {
                eprintln!("wall: Průzkumník (Progman) není k dispozici, zkusím znovu");
                self.set_retry(true);
            }
            Action::Reuse => {
                self.set_retry(false);
                self.layout(&h);
            }
            Action::Reparent(k) => {
                self.set_retry(false);
                if let Some(l) = &mut self.live {
                    unsafe {
                        let _ = SetParent(l.hwnd, Some(parent_hwnd(k, &h)));
                    }
                    l.kind = k;
                }
                self.layout(&h);
            }
            Action::Recreate(k) => {
                self.set_retry(false);
                self.drop_live();
                self.create(k, &h);
            }
        }
        self.drain();
    }

    fn drop_live(&mut self) {
        if let Some(l) = self.live.take() {
            l.page.close();
            unsafe {
                let _ = DestroyWindow(l.hwnd);
            }
        }
    }

    fn create(&mut self, kind: ParentKind, h: &Handles) {
        let parent = parent_hwnd(kind, h);
        let r = target_rect(parent);
        let Some(hwnd) = win::create_window(
            "LedgeWallWindow",
            WS_EX_NOACTIVATE,
            WS_CHILD | WS_CLIPSIBLINGS,
            (r.l, r.t, r.w(), r.h()),
            Some(parent),
            Box::new(|_, m, _, _| match m {
                WM_MOUSEACTIVATE => Some(LRESULT(MA_NOACTIVATE as isize)),
                WM_ERASEBKGND => Some(LRESULT(1)),
                _ => None,
            }),
        ) else {
            self.out.err("okno tapety se nepodařilo vytvořit");
            return;
        };
        unsafe {
            let _ = SetWindowPos(hwnd, Some(z_after(kind, h)), r.l, r.t, r.w(), r.h(), SWP_NOACTIVATE);
        }
        let (q, tid) = (self.queue.clone(), self.tid);
        let page = Page::attach(&self.env, hwnd, &self.html, move |s| {
            q.lock().push_back(Event::Page(s));
            win::post_thread(tid, WM_WAKE);
        });
        let Some(page) = page else {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            self.out.err("WebView2 se nepodařilo načíst scénu tapety");
            return;
        };
        if self.hidden() {
            page.set_paused(true);
        } else {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        self.live = Some(Live { hwnd, kind, page });
        diag::set_target(hwnd, parent);
        if !self.ready_sent {
            self.ready_sent = self.out.ready();
        }
    }

    /// Skryté a uspané: pauza z jádra nebo zamčená obrazovka.
    fn hidden(&self) -> bool {
        env::hidden(self.paused, self.flags.locked.load(std::sync::atomic::Ordering::SeqCst))
    }

    fn eco(&self) -> bool {
        self.flags.battery.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Změna zamčení / napájení: pauza podle `hidden()`, stránce `eco`.
    fn on_env(&self) {
        self.apply_pause();
        self.post(json!({"t": "eco", "on": self.eco()}));
    }

    /// Pozadí z fotky (nebo `url: null` = přechod). Čte jen vlastní kopii `ext/wall/bg.jpg`.
    fn send_bg(&self) {
        self.post(json!({"t": "bg", "url": bg::load_url(&self.dir)}));
    }

    fn apply_pause(&self) {
        let Some(l) = &self.live else { return };
        let hidden = self.hidden();
        unsafe {
            if hidden {
                let _ = ShowWindow(l.hwnd, SW_HIDE);
            }
        }
        l.page.set_paused(hidden);
        if !hidden {
            unsafe {
                let _ = ShowWindow(l.hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }

    fn post(&self, v: Value) {
        if let Some(l) = &self.live {
            l.page.post_json(&v.to_string());
        }
    }

    fn send_init(&self) {
        self.post(json!({"t": "init", "cfg": self.init.cfg, "lang": self.init.lang, "paused": self.hidden(), "eco": self.eco()}));
        self.send_bg();
        for (topic, (rev, d)) in &self.topics {
            self.post(json!({"t": "data", "topic": topic, "rev": rev, "d": d}));
        }
    }

    fn on_page(&self, s: &str) {
        let Ok(m) = serde_json::from_str::<Value>(s) else { return };
        let get = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_string);
        match get("t").as_deref() {
            Some("ready") => self.send_init(),
            Some("req") => {
                if let Some(what) = get("what") {
                    self.out.send(&Up::Req { what, act: get("do"), key: get("key") });
                }
            }
            Some("active") => {
                self.out.active();
            }
            Some("log") => eprintln!("wall(stránka): {}", get("msg").unwrap_or_default()),
            _ => {}
        }
    }

    /// `false` = konec (přišel `quit`).
    fn on_down(&mut self, d: Down) -> bool {
        match d {
            Down::Quit => return false,
            Down::State { pause, .. } => {
                self.paused = pause;
                self.apply_pause();
            }
            Down::Data { topic, rev, d } => {
                self.post(json!({"t": "data", "topic": topic, "rev": rev, "d": d}));
                self.topics.insert(topic, (rev, d));
            }
            // `bg`: Nastavení změnilo fotku — znovu přečíst kopii (ostatní příkazy, třeba `cover`, jdou stránce)
            Down::Cmd { name, .. } if name == "bg" => self.send_bg(),
            Down::Cmd { name, args } => self.post(json!({"t": "cmd", "name": name, "args": args})),
            Down::Init { .. } => {}
        }
        true
    }

    /// Vyprázdní frontu událostí; `false` = skončit.
    fn drain(&mut self) -> bool {
        loop {
            let ev = self.queue.lock().pop_front();
            let Some(ev) = ev else { return true };
            let go = match ev {
                Event::Page(s) => {
                    self.on_page(&s);
                    true
                }
                Event::Down(d) => self.on_down(d),
            };
            if !go {
                // Quit zůstává ve frontě pro smyčku: znovu vložit, ať ho vidí i `run_wall`
                self.queue.lock().push_front(Event::Down(Down::Quit));
                win::post_thread(self.tid, WM_WAKE);
                return false;
            }
        }
    }
}

fn html() -> String {
    std::env::var("LEDGE_WALL_HTML").ok().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_else(|| WALL_HTML.to_string())
}

fn run_wall(io: Io) -> i32 {
    win::init_dpi();
    webview::init_com();
    let dir = crate::actionrun::local_dir().join("ext").join("wall");
    if io.init.cfg.get("diag").and_then(Value::as_bool).unwrap_or(false) {
        diag::start(&dir.join("diag.log"));
    }
    let Some(env) = webview::create_env(&dir.join("wv2")) else {
        io.out.err("WebView2 runtime není dostupný");
        return 3;
    };
    let tid = win::thread_id();
    let taskbar = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    let flags = Arc::new(env::Flags::default());
    flags.battery.store(env::read_battery(), std::sync::atomic::Ordering::SeqCst);
    let Some(watcher) = create_watcher(tid, taskbar, flags.clone()) else {
        io.out.err("skryté okno pro TaskbarCreated se nepodařilo vytvořit");
        return 3;
    };
    let queue: Queue = Arc::new(Mutex::new(VecDeque::new()));
    // most Ledge → vlákno okna: zprávy do fronty a probuzení smyčky
    {
        let (q, rx) = (queue.clone(), io.rx);
        std::thread::spawn(move || {
            while let Ok(m) = rx.recv() {
                let quit = m == Down::Quit;
                q.lock().push_back(Event::Down(m));
                win::post_thread(tid, WM_WAKE);
                if quit {
                    break;
                }
            }
        });
    }
    let mut st = State {
        out: io.out,
        env,
        paused: io.init.dead,
        flags,
        dir,
        init: io.init,
        html: html(),
        queue,
        tid,
        watcher,
        live: None,
        topics: HashMap::new(),
        ready_sent: false,
        retrying: false,
    };
    let sub = env::subscribe(watcher);
    if st.drain() {
        st.sync();
        if st.drain() {
            win::message_loop(|m, _, _| match m {
                WM_WAKE => st.drain(),
                WM_REATTACH => {
                    st.sync();
                    true
                }
                WM_ENV => {
                    st.on_env();
                    true
                }
                _ => true,
            });
        }
    }
    st.drop_live();
    drop(sub);
    unsafe {
        let _ = DestroyWindow(watcher);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wants_reattach_table() {
        const TB: u32 = 0xC123;
        let cases = [
            (TB, 0, true, "TaskbarCreated"),
            (WM_DISPLAYCHANGE, 0, true, "změna monitorů"),
            (WM_TIMER, RETRY_TIMER, true, "časovač opakování"),
            (WM_TIMER, 99, false, "cizí časovač"),
            (WM_PAINT, 0, false, "malování"),
            (WM_USER + 5, 0, false, "cizí zpráva"),
        ];
        for (m, wp, want, name) in cases {
            assert_eq!(wants_reattach(m, wp, TB), want, "{name}");
        }
    }

    /// Simulace restartu Průzkumníka vlastní zprávou: watcher musí probudit vlákno, cizí zpráva ne.
    #[test]
    fn watcher_turns_taskbar_created_into_reattach_message() {
        let taskbar = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        assert!(taskbar != 0);
        let tid = win::thread_id();
        let Some(wh) = create_watcher(tid, taskbar, Arc::new(env::Flags::default())) else { panic!("watcher nevznikl") };
        let peek = || {
            let mut m = MSG::default();
            unsafe { PeekMessageW(&mut m, None, WM_REATTACH, WM_REATTACH, PM_REMOVE).as_bool() }
        };
        assert!(!peek(), "nic zatím nepřišlo");
        unsafe {
            SendMessageW(wh, WM_USER + 5, None, None);
        }
        assert!(!peek(), "cizí zpráva nesmí vyvolat připnutí");
        unsafe {
            SendMessageW(wh, taskbar, None, None);
        }
        assert!(peek(), "TaskbarCreated → WM_REATTACH");
        unsafe {
            let _ = DestroyWindow(wh);
        }
    }

    #[test]
    fn scene_contract_is_embedded() {
        // kontrakt se stránkou (E3 ho drží): stránka hlásí 'ready' a poslouchá 'init' / 'data'
        for k in ["chrome.webview", "'ready'", "visibilitychange", "'init'", "'data'", "'bg'", "'eco'", "what: 'cover'"] {
            assert!(WALL_HTML.contains(k), "wall.html chybí {k}");
        }
    }
}
