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
//!
//! Pohledy (`View`): Hidden, Peek (schválení / připomínka), Full (celý šuplík) a náhledy po najetí
//! Island (`drawer.hover`, čisté rozhodování v `hovermode.rs`). Přechody mezi nimi jdou stejným
//! morfem (výchozí tvar = předchozí pohled). Fokus bere jen Full otevřený zkratkou / výslovný klik.

use crate::qa::QaDataDir as _;
use crate::appearance::{StripPosition, geom};
use crate::drawerfit;
use crate::hovermode::{self, SmallSizes};
use crate::screen::Monitor;
use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND, DwmSetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::*;

mod resize;

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
    /// po najetí (`drawer.hover: "island"`): kapsle s jedním děním (`src/hover/Island.tsx`)
    Island,
    /// střední panel z docku (L147): ~1/3 monitoru, max 3 karty (`drawer.dockCards`, `src/hover/Mid.tsx`)
    Mid,
}

impl View {
    /// Otevřený panel u horní hrany (celý šuplík nebo náhled po najetí): kryje proužek, zavírá ho
    /// odjetí myši (otevřený najetím) a s čekajícím Claudem se stáhne do peeku (`hovermode::after_close`).
    pub fn covers_strip(self) -> bool {
        matches!(self, View::Full | View::Island | View::Mid)
    }

    /// Pohled kreslí karty ze snapshotu (celý šuplík, střední panel): při otevření se obnoví jejich data.
    pub fn has_cards(self) -> bool {
        matches!(self, View::Full | View::Mid)
    }

    /// Schování tohoto pohledu zapíše `hidden_at_ms` (chip „Claude od posledně“): jen plný šuplík a
    /// střední panel; odjezd myši z docku / ostrůvku / peeku ne (L174).
    pub fn records_hide(self) -> bool {
        self.has_cards()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opener {
    Hover,
    Hotkey,
    /// otevřeno přes /ctl — bez fokusu a bez samozavírání (skripty, Claude, testy)
    Remote,
    /// upevněno 📌 (dok): jako `Remote` (neodjede, nezavře blur) a okno smí být aktivní — píše se do něj
    Pinned,
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
    /// náhled po najetí (Island): po kolika ms setrvání se rozbalí celý šuplík, 0 = jen klikem
    pub expand_ms: u64,
    /// `drawer.hover == dock` (L130): `view: island` kreslí dok místo ostrůvku — frontend nečte config při každém vstupu
    pub dock: bool,
    /// karty středního panelu (`drawer.dockCards` po ověření, `hovermode::dock_cards`)
    pub dock_cards: Vec<String>,
    /// okno je upevněné 📌 (`Opener::Pinned`): frontend si podle toho drží tlačítko (jinak by ho každé `view` shodilo)
    pub pinned: bool,
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
    /// peek, ostrůvek / dok, střední panel (`hovermode::SmallSizes`)
    small: SmallSizes,
    /// `drawer.hoverExpandMs` (oříznuté) — jde frontendu ve `ViewMsg`
    expand_ms: u64,
    /// `drawer.hover == dock` (`set_hover_mode`), jde frontendu ve `ViewMsg`
    dock: bool,
    /// `drawer.dockCards` (ověřené), jde frontendu ve `ViewMsg`
    dock_cards: Vec<String>,
    rect: RECT,
    last_blur: Option<Instant>,
    /// frontend má posluchače a stav
    ready: bool,
    prepared: bool,
    closed: bool,
    hidden_since: Instant,
    /// wall-clock ms posledního výslovného schování (`hide`); `None` = od startu ještě neschovaný (chip „Claude od posledně“)
    hidden_at_ms: Option<u64>,
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
                small: SmallSizes::default(),
                expand_ms: hovermode::EXPAND_MS,
                dock: false,
                dock_cards: hovermode::dock_cards(&[]),
                rect: RECT::default(),
                last_blur: None,
                ready: false,
                prepared: false,
                closed: false,
                hidden_since: Instant::now(),
                hidden_at_ms: None,
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
            .additional_browser_args(crate::qa::browser_args())
            .qa_data_dir()
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

    /// Jak dlouho je šuplík schovaný (s); `None` = je vidět.
    pub fn hidden_secs(&self) -> Option<u64> {
        let s = self.st.lock();
        (s.view == View::Hidden).then(|| s.hidden_since.elapsed().as_secs())
    }

    /// Kdy (wall-clock ms) se šuplík naposledy schoval; `None` = ještě ne.
    pub fn hidden_at_ms(&self) -> Option<u64> {
        self.st.lock().hidden_at_ms
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
            crate::winstyle::no_activate(h, true);
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

    /// Prodleva rozbalení náhledu po najetí (`drawer.hoverExpandMs`, vlákno `ui` z configu).
    pub fn set_expand_ms(&self, ms: u64) {
        self.st.lock().expand_ms = hovermode::expand_ms(ms);
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

    /// Velikost pohledu v logických px, Full i náhledy po najetí oříznuté na monitor (`drawerfit`, `hovermode`).
    fn logical_size(&self, view: View, width: f64, mon: &Monitor) -> (f64, f64) {
        let s = self.st.lock();
        let lim = drawerfit::limits(mon.rect, mon.work, mon.dpi);
        match view {
            View::Full => drawerfit::fit_full(s.full_w, width, s.full_h, lim),
            View::Hidden => (0.0, 0.0),
            View::Mid => s.small.get(View::Mid).map_or((0.0, 0.0), |sz| hovermode::fit_mid(sz.1, lim.max_w, lim.max_h)),
            small => s.small.get(small).map_or((0.0, 0.0), |sz| hovermode::fit_small(small, sz, width, lim.max_h)),
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
                View::Full => {
                    let changed = (s.full_h - h).abs() > 0.5 || (s.full_w - w).abs() > 0.5;
                    s.full_h = h;
                    s.full_w = w.max(0.0);
                    changed
                }
                small => s.small.set(small, w, h),
            };
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
                let mut s = self.st.lock();
                // zkratka na upevněný dok: fokus ano, upevnění zůstává
                if s.opener != Opener::Pinned {
                    s.opener = Opener::Hotkey;
                }
                drop(s);
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
        let (expand_ms, dock, dock_cards, pinned) = {
            let mut s = self.st.lock();
            s.prepared = false;
            (s.expand_ms, s.dock, s.dock_cards.clone(), s.opener == Opener::Pinned)
        };
        let _ = self.app.emit_to(LABEL, "view", ViewMsg { view, morph, from_w, from_h, to_w, to_h, max_h, expand_ms, dock, dock_cards, pinned });
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
            // upevněný dok zůstane upevněný (klik do poznámky jen aktivuje okno)
            if s.opener != Opener::Pinned {
                if s.opener != Opener::Hotkey {
                    s.focus_restore = Some(s.opener);
                }
                s.opener = Opener::Hotkey;
            }
        }
        self.focus();
    }

    /// 📌 v docku (`drawer_pin`): upevnit — odjetí myši ani blur dok nezavřou, okno smí být aktivní
    /// (`place` NOACTIVATE nevrací). `false` = uvolnit zpět na zavírání odjetím myši. Bez otevřeného okna nic.
    pub fn pin(&self, on: bool) {
        let mut s = self.st.lock();
        if s.view == View::Hidden {
            return;
        }
        if on {
            s.opener = Opener::Pinned;
            s.focus_restore = None;
        } else if s.opener == Opener::Pinned {
            s.opener = Opener::Hover;
        }
    }

    /// `drawer.dockCards` (ověřené `hovermode::dock_cards`) — vlákno `ui` z configu.
    pub fn set_dock_cards(&self, cards: Vec<String>) {
        self.st.lock().dock_cards = cards;
    }

    /// `drawer.hover` se změnil: dok má jinou výchozí velikost než ostrůvek (`SmallSizes::set_mode`).
    pub fn set_hover_mode(&self, mode: hovermode::HoverMode) {
        let mut s = self.st.lock();
        s.small.set_mode(mode);
        s.dock = mode == hovermode::HoverMode::Dock;
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
                crate::winstyle::no_activate(h, false);
                let _ = SetForegroundWindow(h);
            }
        }
        let _ = w.set_focus();
    }

    /// Cílové místo okna ve fyzických px (i pro `resize.rs`): `(x, y, šířka, výška)` (výška včetně přesahu nad hranou).
    fn target(&self, view: View, mon: &Monitor, width: f64) -> (i32, i32, i32, i32) {
        let scale = mon.dpi as f64 / 96.0;
        let (lw, lh) = self.logical_size(view, width, mon);
        let pw = (lw * scale).round() as i32;
        let overlap = (self.overlap * scale).round() as i32;
        let ph = (lh * scale).round() as i32 + overlap;
        // osa velkého šuplíku (střed / u levé / u pravé hrany) — proužek leží na stejné (`stripview`)
        let full = (width * scale).round() as i32;
        let c2 = geom::anchor_c2(mon.rect.left, mon.rect.right, full, self.st.lock().anchor, mon.dpi);
        (crate::qa::park_x(geom::span_x(c2, pw)), mon.rect.top - overlap, pw, ph)
    }

    /// Okno na cílové místo a velikost (bez animace okna). `reveal` = zároveň ukázat.
    fn place(&self, view: View, mon: &Monitor, width: f64, reveal: bool) -> Option<(HWND, i32, i32)> {
        let w = self.window()?;
        let h = hwnd_of(&w)?;
        let (x, y, pw, ph) = self.target(view, mon, width);
        if view == View::Full && ph < 300 {
            // diagnostika „uříznutého šuplíku“ (0.11.x)
            let (full_h, max_h) = (self.st.lock().full_h, drawerfit::limits(mon.rect, mon.work, mon.dpi).max_h);
            crate::state::log(&format!("drawer: okno Full {pw}×{ph} (full_h {full_h:.0}, max_h {max_h:.0}, dpi {})", mon.dpi));
        }
        unsafe {
            if !matches!(self.opener(), Opener::Hotkey | Opener::Pinned) {
                crate::winstyle::no_activate(h, true);
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
        let was = self.view();
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
                ViewMsg { view: View::Hidden, morph: true, from_w: 0.0, from_h: 0.0, to_w: strip.0, to_h: strip.1, max_h: 0.0, expand_ms: 0, dock: false, dock_cards: Vec::new(), pinned: false },
            );
            // stažení do proužku; pojistka, kdyby frontend neodpověděl
            self.wait_flag(|s| s.closed, Duration::from_millis(600));
            unsafe {
                let _ = ShowWindow(h, SW_HIDE);
            }
        } else {
            let _ = self.app.emit_to(LABEL, "view", ViewMsg { view: View::Hidden, morph: false, from_w: 0.0, from_h: 0.0, to_w: 0.0, to_h: 0.0, max_h: 0.0, expand_ms: 0, dock: false, dock_cards: Vec::new(), pinned: false });
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
            if was.records_hide() {
                s.hidden_at_ms = Some(crate::util::now_ms());
            }
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

#[cfg(test)]
mod hide_tests {
    use super::View;

    #[test]
    fn hide_records_only_full_and_mid() {
        for (v, want) in [
            (View::Full, true),
            (View::Mid, true),
            (View::Island, false),
            (View::Peek, false),
            (View::Hidden, false),
        ] {
            assert_eq!(v.records_hide(), want, "{v:?}");
        }
    }
}
