//! Okno šuplíku (WebView2). Nikdy nekrade fokus, pokud ho šéf neotevřel zkratkou.
//!
//! Dva režimy pohybu:
//! - **morph** (výchozí, pozadí „solid“): okno stojí na místě a průhledné; frontend sám animuje
//!   tvar panelu z proužku do šuplíku a zpět (clip-path + pružina). Rust jen řídí handshake:
//!   `view` (frontend se složí do výchozího tvaru) → `prepared` → okno se ukáže → `play`.
//!   Zavření: `closing` → frontend se stáhne do proužku → `closed` → okno se schová.
//! - **slide** (pozadí „acrylic“): DWM akryl kryje celé okno, tvar animovat nejde → okno vyjíždí.
//!
//! Horní rohy jsou vždy schované nad okrajem obrazovky → panel vypadá přilepený k hraně.

use crate::appearance::{StripPosition, geom};
use crate::drawerfit;
use crate::screen::Monitor;
use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND, DwmSetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const LABEL: &str = "drawer";

/// Chromium bez služeb, které šuplík nepotřebuje (sync, aktualizace komponent, crash reporter…).
/// Stejné argumenty (a výchozí data dir) bere i okno s webem (`webpanel.rs`) — WebView2 pak sdílí
/// jeden proces prohlížeče; s jinými argumenty by nad stejným data direm okno vůbec nevzniklo.
pub(crate) const BROWSER_ARGS: &str = concat!(
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection ",
    // síťová služba v procesu prohlížeče: −1 proces, ~−10 MB private (změřeno 2026-10-17)
    "--enable-features=NetworkServiceInProcess2 ",
    "--disable-background-networking --disable-component-update --disable-sync --disable-default-apps ",
    "--no-first-run --disable-breakpad --renderer-process-limit=1 --disable-gpu-shader-disk-cache ",
    // bez GPU procesu: −60 MB, šuplík je malý panel a akryl kreslí DWM, ne Chromium (změřeno);
    // softwarové skládání pak běží v procesu prohlížeče (`--in-process-gpu`): −1 proces, ~−10 MB
    "--disable-gpu --in-process-gpu"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum View {
    Hidden,
    /// malý pruh se schválením pro Claude
    Peek,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opener {
    Hover,
    Hotkey,
    /// otevřeno přes /ctl — bez fokusu a bez samozavírání (skripty, Claude, testy)
    Remote,
}

/// Zpráva pro frontend: kam se má panel přetvarovat a odkud.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewMsg {
    pub view: View,
    pub morph: bool,
    /// výchozí tvar (logické px): proužek, nebo předchozí pohled
    pub from_w: f64,
    pub from_h: f64,
    /// cílová velikost okna (logické px, bez přesahu nad okrajem)
    pub to_w: f64,
    pub to_h: f64,
    /// strop výšky obsahu na tomhle monitoru (logické px, bez přesahu) — nad ním posouvá frontend
    pub max_h: f64,
}

/// Měření plynulosti animace z frontendu (requestAnimationFrame).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimStats {
    pub kind: String,
    pub frames: u32,
    pub avg_ms: f64,
    pub worst_ms: f64,
    pub duration_ms: f64,
}

struct St {
    view: View,
    opener: Opener,
    /// opener před `take_focus` (klik do poznámky) — `release_focus` ho vrátí
    focus_restore: Option<Opener>,
    /// výšky obsahu v logických px, jak je hlásí frontend
    full_h: f64,
    /// žádost frontendu o širší Full (Nastavení), 0 = šířka z configu (`drawerfit::fit_full`)
    full_w: f64,
    peek_h: f64,
    peek_w: f64,
    rect: RECT,
    last_blur: Option<Instant>,
    /// frontend má posluchače a stav
    ready: bool,
    prepared: bool,
    closed: bool,
    hidden_since: Instant,
    acrylic: bool,
    /// kolik ms trvalo poslední otevření (vč. případného studeného startu) — měření, ne odhad
    last_open_ms: u64,
    last_open_cold: bool,
    anim: Option<AnimStats>,
    /// kde u horní hrany šuplík sedí (`appearance.strip.position`) — osa sdílená s proužkem
    anchor: StripPosition,
}

/// Co udělat, když šuplík ztratí fokus (lib.rs: zavřít, byl-li otevřený zkratkou).
pub type BlurFn = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct Drawer {
    app: AppHandle,
    st: Arc<Mutex<St>>,
    cv: Arc<Condvar>,
    on_blur: Arc<Mutex<Option<BlurFn>>>,
    /// přesah nad horní hranou (logické px) — schová horní zaoblené rohy
    overlap: f64,
}

fn hwnd_of(w: &WebviewWindow) -> Option<HWND> {
    w.hwnd().ok().map(|h| HWND(h.0 as *mut _))
}

impl Drawer {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            st: Arc::new(Mutex::new(St {
                view: View::Hidden,
                opener: Opener::Hover,
                focus_restore: None,
                full_h: 420.0,
                full_w: 0.0,
                peek_h: 72.0,
                peek_w: 680.0,
                rect: RECT::default(),
                last_blur: None,
                ready: false,
                prepared: false,
                closed: false,
                hidden_since: Instant::now(),
                acrylic: false,
                last_open_ms: 0,
                last_open_cold: false,
                anim: None,
                anchor: StripPosition::Center,
            })),
            cv: Arc::new(Condvar::new()),
            on_blur: Arc::new(Mutex::new(None)),
            overlap: 18.0,
        }
    }

    fn window(&self) -> Option<WebviewWindow> {
        self.app.get_webview_window(LABEL)
    }

    pub fn set_on_blur(&self, f: BlurFn) {
        *self.on_blur.lock() = Some(f);
    }

    pub fn last_open(&self) -> (u64, bool) {
        let s = self.st.lock();
        (s.last_open_ms, s.last_open_cold)
    }

    pub fn anim_stats(&self) -> Option<AnimStats> {
        self.st.lock().anim.clone()
    }

    pub fn set_anim_stats(&self, a: AnimStats) {
        self.st.lock().anim = Some(a);
    }

    pub fn is_warm(&self) -> bool {
        self.window().is_some()
    }

    fn morph(&self) -> bool {
        !(self.st.lock().acrylic && crate::screen_glass_capable())
    }

    /// Vytvoří okno šuplíku, pokud neexistuje. Vrací true = právě vzniklo (stránka se teprve načítá).
    pub fn ensure(&self) -> bool {
        if self.window().is_some() {
            return false;
        }
        self.st.lock().ready = false;
        crate::log("drawer: build start");
        let built = WebviewWindowBuilder::new(&self.app, LABEL, WebviewUrl::App("index.html".into()))
            .title("Ledge")
            .inner_size(1080.0, 440.0)
            .visible(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .resizable(false)
            .shadow(false)
            .disable_drag_drop_handler()
            .additional_browser_args(BROWSER_ARGS)
            .build();
        crate::log(&format!("drawer: build done ok={}", built.is_ok()));
        let Ok(w) = built else { return false };
        let blur = self.on_blur.clone();
        w.on_window_event(move |e| {
            if let tauri::WindowEvent::Focused(false) = e
                && let Some(f) = blur.lock().clone()
            {
                f();
            }
        });
        let acrylic = self.st.lock().acrylic;
        self.init(acrylic);
        true
    }

    /// Po delší nečinnosti okno zrušit úplně — WebView2 i jeho procesy zmizí.
    pub fn destroy_if_idle(&self, keep_warm: Duration) -> bool {
        if keep_warm.is_zero() {
            return false;
        }
        let idle = {
            let s = self.st.lock();
            s.view == View::Hidden && s.hidden_since.elapsed() > keep_warm
        };
        if !idle {
            return false;
        }
        let Some(w) = self.window() else { return false };
        self.st.lock().ready = false;
        let _ = w.destroy();
        true
    }

    /// Předehřátí: vytvořit okno, ať je při otevření hotové; připravené pak uspat.
    pub fn warm(&self) {
        if self.ensure() {
            self.st.lock().hidden_since = Instant::now();
            self.wait_flag(|s| s.ready, Duration::from_secs(8));
            if self.view() == View::Hidden {
                self.set_dormant(true);
            }
        }
    }

    /// Potvrzení z frontendu: "ready" (stránka žije), "prepared" (složeno do výchozího tvaru),
    /// "closed" (stažení do proužku doběhlo).
    pub fn ack(&self, kind: &str) {
        let mut s = self.st.lock();
        match kind {
            "ready" => s.ready = true,
            "prepared" => s.prepared = true,
            "closed" => s.closed = true,
            _ => return,
        }
        self.cv.notify_all();
    }

    fn wait_flag(&self, f: impl Fn(&St) -> bool, max: Duration) -> bool {
        let mut s = self.st.lock();
        let deadline = Instant::now() + max;
        while !f(&s) {
            if self.cv.wait_until(&mut s, deadline).timed_out() {
                return f(&s);
            }
        }
        true
    }

    /// Jednorázové nastavení okna: styl bez aktivace, rohy, akryl.
    fn init(&self, acrylic: bool) {
        let Some(w) = self.window() else { return };
        if let Some(h) = hwnd_of(&w) {
            unsafe {
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex | (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize);
            }
        }
        self.set_glass(acrylic);
    }

    pub fn set_glass(&self, acrylic: bool) {
        self.st.lock().acrylic = acrylic;
        let Some(w) = self.window() else { return };
        use tauri::window::{Effect, EffectsBuilder};
        let capable = acrylic && crate::screen_glass_capable();
        if capable {
            let _ = w.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build());
        } else {
            let _ = w.set_effects(None);
        }
        // morph kreslí tvar sám (DWM rohy by ho ořezaly), akryl potřebuje kulaté rohy od DWM
        if let Some(h) = hwnd_of(&w) {
            let pref = if capable { DWMWCP_ROUND } else { DWMWCP_DONOTROUND };
            unsafe {
                let _ = DwmSetWindowAttribute(
                    h,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as *const _,
                    std::mem::size_of_val(&pref) as u32,
                );
            }
        }
        let _ = self.app.emit_to(LABEL, "glass", capable);
    }

    /// Zavřený šuplík: WebView2 dostane nízký paměťový cíl a uspí se (žádné timery, méně RAM).
    /// Změřeno 2026-10-17 (po otevření a zavření): bez obojího ~117 MB private a ~330 MB WS, jen Low
    /// bez uspání ~121 MB, Low + uspání hned ~108–113 MB / ~63 MB WS; uspání o 3 s později nic nepřidá.
    pub fn set_dormant(&self, dormant: bool) {
        let Some(w) = self.window() else { return };
        let _ = w.with_webview(move |wv| unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::*;
            use windows::core::Interface;
            let ctrl = wv.controller();
            let Ok(core) = ctrl.CoreWebView2() else { return };
            if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
                let level = if dormant {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                };
                let _ = c19.SetMemoryUsageTargetLevel(level);
            }
            if let Ok(c3) = core.cast::<ICoreWebView2_3>() {
                if dormant {
                    let _ = ctrl.SetIsVisible(false);
                    let h = webview2_com::TrySuspendCompletedHandler::create(Box::new(|res, ok| {
                        if res.is_err() || !ok {
                            crate::log(&format!("drawer: TrySuspend neprošel ({res:?}, ok={ok})"));
                        }
                        Ok(())
                    }));
                    let _ = c3.TrySuspend(&h);
                } else {
                    let _ = c3.Resume();
                    let _ = ctrl.SetIsVisible(true);
                }
            }
        });
    }

    /// Pozice u horní hrany (vlákno `ui` ji nastaví před každým příkazem z configu).
    pub fn set_anchor(&self, pos: StripPosition) {
        self.st.lock().anchor = pos;
    }

    pub fn view(&self) -> View {
        self.st.lock().view
    }

    pub fn opener(&self) -> Opener {
        self.st.lock().opener
    }

    pub fn rect(&self) -> RECT {
        self.st.lock().rect
    }

    /// Velikost pohledu v logických px, Full oříznutý na monitor (`drawerfit`).
    fn logical_size(&self, view: View, width: f64, mon: &Monitor) -> (f64, f64) {
        let s = self.st.lock();
        match view {
            View::Full => drawerfit::fit_full(s.full_w, width, s.full_h, drawerfit::limits(mon.rect, mon.work, mon.dpi)),
            View::Peek => (s.peek_w, s.peek_h),
            View::Hidden => (0.0, 0.0),
        }
    }

    pub fn report_size(&self, view: View, w: f64, h: f64, mon: &Monitor, width: f64) {
        let changed = {
            let mut s = self.st.lock();
            if !s.ready {
                s.ready = true;
                self.cv.notify_all();
            }
            let changed = match view {
                View::Full => (s.full_h - h).abs() > 0.5 || (s.full_w - w).abs() > 0.5,
                View::Peek => (s.peek_h - h).abs() > 0.5 || (s.peek_w - w).abs() > 0.5,
                View::Hidden => false,
            };
            match view {
                View::Full => {
                    s.full_h = h;
                    s.full_w = w.max(0.0);
                }
                View::Peek => {
                    s.peek_h = h;
                    s.peek_w = w;
                }
                View::Hidden => {}
            }
            changed && s.view == view
        };
        if changed {
            // obsah se změnil za běhu (log, nová karta) → jen dorovnat velikost
            self.place(view, mon, width, false);
        }
    }

    /// `strip` = aktuální velikost proužku v logických px (výchozí tvar morfu).
    pub fn show(&self, view: View, opener: Opener, mon: &Monitor, width: f64, animate_ms: u64, strip: (f64, f64)) {
        let from = self.view();
        if from == view {
            if opener == Opener::Hotkey {
                self.st.lock().opener = Opener::Hotkey;
                self.focus();
            }
            return;
        }
        let (from_w, from_h) = if from == View::Hidden { strip } else { self.logical_size(from, width, mon) };
        {
            let mut s = self.st.lock();
            s.view = view;
            s.opener = opener;
            s.focus_restore = None;
        }
        let t0 = Instant::now();
        let created = self.ensure();
        if created || !self.st.lock().ready {
            crate::log("drawer: wait_ready");
            self.wait_flag(|s| s.ready, Duration::from_millis(4000));
        } else if from == View::Hidden {
            self.set_dormant(false);
        }
        let morph = self.morph() && animate_ms > 0;
        let (to_w, to_h) = self.logical_size(view, width, mon);
        let max_h = drawerfit::limits(mon.rect, mon.work, mon.dpi).max_h;
        self.st.lock().prepared = false;
        let _ = self.app.emit_to(LABEL, "view", ViewMsg { view, morph, from_w, from_h, to_w, to_h, max_h });
        if morph {
            // frontend se složí do tvaru proužku dřív, než okno uvidíš → žádné probliknutí
            self.wait_flag(|s| s.prepared, Duration::from_millis(250));
        }
        let ready_ms = t0.elapsed().as_millis() as u64;
        if morph {
            self.place(view, mon, width, true);
            let _ = self.app.emit_to(LABEL, "play", view);
        } else {
            self.slide_in(view, mon, width, if from == View::Hidden { animate_ms } else { 0 });
        }
        {
            let mut s = self.st.lock();
            s.last_open_ms = ready_ms;
            s.last_open_cold = created;
        }
        if opener == Opener::Hotkey && view == View::Full {
            self.focus();
        }
    }

    /// Aktivovat otevřený Full / peek na výslovný klik (příkaz `focus_drawer`): opener → `Hotkey`, takže
    /// `place` už nevrátí NOACTIVATE, myš mimo okno ho nezavře a zavře ho až blur (`set_on_blur`).
    /// Peek (otázka Clauda → „Jiné…“) blur nezavře — zmizí, až otázka není (`on_claude_change`).
    pub fn take_focus(&self) {
        {
            let mut s = self.st.lock();
            if s.view == View::Hidden {
                return;
            }
            if s.opener != Opener::Hotkey {
                s.focus_restore = Some(s.opener);
            }
            s.opener = Opener::Hotkey;
        }
        self.focus();
    }

    /// Odchod ze stránky poznámky (Zpět, uložení, Esc bez zavření): vrátit opener z doby před
    /// `take_focus`, ať šuplík otevřený najetím zase zavře odjetí myší. Okno zůstane aktivní.
    pub fn release_focus(&self) {
        let mut s = self.st.lock();
        if let Some(o) = s.focus_restore.take() {
            s.opener = o;
        }
    }

    fn focus(&self) {
        let Some(w) = self.window() else { return };
        if let Some(h) = hwnd_of(&w) {
            unsafe {
                // zkratkou otevřený šuplík má dostat klávesnici; odkud fokus bereme, si zapamatovat
                crate::focusreturn::remember();
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !(WS_EX_NOACTIVATE.0 as isize));
                let _ = SetForegroundWindow(h);
            }
        }
        let _ = w.set_focus();
    }

    /// Okno na cílové místo a velikost (bez animace okna). `reveal` = zároveň ukázat.
    fn place(&self, view: View, mon: &Monitor, width: f64, reveal: bool) -> Option<(HWND, i32, i32)> {
        let w = self.window()?;
        let h = hwnd_of(&w)?;
        let scale = mon.dpi as f64 / 96.0;
        let (lw, lh) = self.logical_size(view, width, mon);
        let pw = (lw * scale).round() as i32;
        let overlap = (self.overlap * scale).round() as i32;
        let ph = (lh * scale).round() as i32 + overlap;
        // osa velkého šuplíku (střed / u levé / u pravé hrany) — proužek leží na stejné (`stripview`)
        let full = (width * scale).round() as i32;
        let c2 = geom::anchor_c2(mon.rect.left, mon.rect.right, full, self.st.lock().anchor, mon.dpi);
        let x = geom::span_x(c2, pw);
        let y = mon.rect.top - overlap;
        unsafe {
            if self.opener() != Opener::Hotkey {
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
            }
            let _ = SetWindowPos(h, Some(HWND_TOPMOST), x, y, pw, ph, SWP_NOACTIVATE);
            if reveal && !IsWindowVisible(h).as_bool() {
                let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            }
        }
        self.st.lock().rect = RECT { left: x, top: mon.rect.top, right: x + pw, bottom: y + ph };
        Some((h, x, y))
    }

    /// Akrylový režim: okno vyjede shora.
    fn slide_in(&self, view: View, mon: &Monitor, width: f64, animate_ms: u64) {
        let Some((h, x, y)) = self.place(view, mon, width, false) else { return };
        unsafe {
            let visible = IsWindowVisible(h).as_bool();
            if !visible {
                let mut r = RECT::default();
                let _ = GetWindowRect(h, &mut r);
                let start = if animate_ms > 0 { mon.rect.top - (r.bottom - r.top) } else { y };
                let _ = SetWindowPos(h, Some(HWND_TOPMOST), x, start, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
                if start != y {
                    animate(h, x, start, y, animate_ms);
                }
            }
        }
    }

    pub fn hide(&self, mon: &Monitor, animate_ms: u64, strip: (f64, f64)) {
        if self.view() == View::Hidden {
            return;
        }
        let morph = self.morph() && animate_ms > 0;
        {
            let mut s = self.st.lock();
            s.view = View::Hidden;
            s.closed = false;
        }
        // výslovné zavření: fokus zpět, dokud je Ledge v popředí (blur je teď no-op, view = Hidden)
        crate::focusreturn::give_back();
        let Some(w) = self.window() else { return };
        let Some(h) = hwnd_of(&w) else { return };
        if morph {
            let _ = self.app.emit_to(
                LABEL,
                "closing",
                ViewMsg { view: View::Hidden, morph: true, from_w: 0.0, from_h: 0.0, to_w: strip.0, to_h: strip.1, max_h: 0.0 },
            );
            // stažení do proužku; pojistka, kdyby frontend neodpověděl
            self.wait_flag(|s| s.closed, Duration::from_millis(600));
            unsafe {
                let _ = ShowWindow(h, SW_HIDE);
            }
        } else {
            let _ = self.app.emit_to(LABEL, "view", ViewMsg { view: View::Hidden, morph: false, from_w: 0.0, from_h: 0.0, to_w: 0.0, to_h: 0.0, max_h: 0.0 });
            unsafe {
                let mut r = RECT::default();
                let _ = GetWindowRect(h, &mut r);
                if animate_ms > 0 && IsWindowVisible(h).as_bool() {
                    let end = mon.rect.top - (r.bottom - r.top);
                    animate(h, r.left, r.top, end, animate_ms * 3 / 4);
                }
                let _ = ShowWindow(h, SW_HIDE);
            }
        }
        {
            let mut s = self.st.lock();
            s.rect = RECT::default();
            s.hidden_since = Instant::now();
            // příště se otevírá hlavní stránka — ne v šířce Nastavení
            s.full_w = 0.0;
        }
        self.set_dormant(true);
    }

    pub fn note_blur(&self) {
        self.st.lock().last_blur = Some(Instant::now());
    }
}

/// Plynulé vysunutí (jen akrylový režim): ease-out, ~120 Hz kroky.
fn animate(h: HWND, x: i32, from: i32, to: i32, ms: u64) {
    let t0 = Instant::now();
    let total = Duration::from_millis(ms.max(1));
    loop {
        let t = (t0.elapsed().as_secs_f64() / total.as_secs_f64()).min(1.0);
        let e = 1.0 - (1.0 - t).powi(3);
        let y = from + ((to - from) as f64 * e).round() as i32;
        unsafe {
            let _ = SetWindowPos(h, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER);
        }
        if t >= 1.0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(8));
    }
}
