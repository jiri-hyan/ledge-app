//! Nativní proužek nahoře uprostřed.
//!
//! Žádný WebView: vrstvené okno (UpdateLayeredWindow s per-pixel alfou) kreslené Direct2D +
//! DirectWrite. `WS_EX_TRANSPARENT` → myš jím propadá (záložky prohlížeče pod ním fungují),
//! `WS_EX_NOACTIVATE` → nikdy nebere fokus. Překresluje se jen při změně obsahu.
//! Vzhled (styl `pill`/`line`/`dot` a tvary stylů vlny 6 `term`/`tag`/`glow`/`block`/`drop`, velikost,
//! paleta motivu, barva linky, osa u hrany) přichází v `StripModel` spočítaný čistými funkcemi
//! `appearance::geom`; ozdoby stylu (okraj, stíny, záře, písmo) dává `appearance::stripdeco`.
//! Puls „Claude čeká / hotovo“ (`StripModel.pulse`, `attention::pulse_level`): časovač `WM_TIMER` běží
//! jen po dobu pulsu (`attention::PULSE_MS`), v klidu žádný.
//! Styly zavřeného Ledge (`strip.look` ≠ `classic`, `StripModel.show`) kreslí `stripdraw`; navíc jen
//! plynulá změna šířky ostrůvku (stejný snímkový časovač, ~240 ms po změně obsahu) a střídání řádků
//! `ticker` po 5 s (`ROTATE_TIMER`, jen když je víc než jedna událost).

use crate::appearance::StripStyle;
use crate::appearance::geom::{self, Palette, StripMetrics};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT};
use windows::Win32::Graphics::Direct2D::{
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_SOFTWARE,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_TEXT_METRICS, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, w};

pub const WM_STRIP_REDRAW: u32 = WM_APP + 1;
pub const WM_STRIP_VISIBLE: u32 = WM_APP + 2;
/// konec vlákna a okna vedlejšího proužku (monitor odpojen / volba `strip.monitor` se změnila, `Strip::stop`)
const WM_STRIP_STOP: u32 = WM_APP + 3;
/// id časovače pulsu (jen po dobu `attention::PULSE_MS`)
const PULSE_TIMER: usize = 1;
/// snímek pulsu (~30 fps; software D2D, 24px proužek)
const PULSE_FRAME_MS: u32 = 33;
/// id časovače střídání řádků stylu `ticker` (jen při víc než jedné události, `looks::ROTATE_MS`)
const ROTATE_TIMER: usize = 2;
/// id časovače točení prstenců stylu `rings` (jen dokud Claude pracuje a animace jsou zapnuté)
const SPIN_TIMER: usize = 3;
/// snímek točení (~8 fps: plynulé dost, software D2D 28px kapsle ≈ desetiny ms)
const SPIN_FRAME_MS: u32 = 125;
/// id časovače živé hrany vlastního vzhledu (kolo 28; jen během animace, `stripfx::prepare` / `edgefx::frames`)
const FX_TIMER: usize = 4;
/// snímek živé hrany (25 fps: pomalé přelévání / dech stačí; snímek ≈ 0,55 ms software D2D, `stripfx::bench`)
const FX_FRAME_MS: u32 = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba(pub f32, pub f32, pub f32, pub f32);

impl Rgba {
    pub const TEXT: Rgba = Rgba(0.95, 0.95, 0.97, 1.0);
    pub const DIM: Rgba = Rgba(0.65, 0.67, 0.73, 1.0);
    pub const GREEN: Rgba = Rgba(0.29, 0.87, 0.50, 1.0);
    pub const AMBER: Rgba = Rgba(0.98, 0.75, 0.14, 1.0);
    pub const RED: Rgba = Rgba(0.97, 0.44, 0.44, 1.0);
    pub const VIOLET: Rgba = Rgba(167.0 / 255.0, 139.0 / 255.0, 250.0 / 255.0, 1.0); // #a78bfa = akcent motivu dark (palettes.json)
    pub const BLUE: Rgba = Rgba(96.0 / 255.0, 165.0 / 255.0, 250.0 / 255.0, 1.0); // #60a5fa
    pub const TEAL: Rgba = Rgba(45.0 / 255.0, 212.0 / 255.0, 191.0 / 255.0, 1.0); // #2dd4bf
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub text: String,
    pub color: Rgba,
    pub dot: Option<Rgba>,
    pub strong: bool,
    /// stavový segment v barevné „pilulce“ (tón z barvy tečky)
    pub pill: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StripModel {
    pub visible: bool,
    pub segments: Vec<Segment>,
    /// barevná linka dole (jantar = čeká Claude, zelená = naběhlo, červená = pád)
    pub ambient: Option<Rgba>,
    pub opacity: f32,
    /// jen linka bez obsahu (proužek vypnutý v nastavení)
    pub line_only: bool,
    /// myš je na proužku a odpočítává se otevření → proužek „nažhaví“ (předzvěst otevření)
    pub hot: bool,
    /// obdélník monitoru v px
    pub monitor: RECT,
    pub dpi: u32,
    pub style: StripStyle,
    pub metrics: StripMetrics,
    pub palette: Palette,
    /// barvy podpisové linky (začátek, střed, dozvuk) — `geom::line_stops`
    pub line: [Rgba; 3],
    /// dvojnásobek x osy proužku v px (`geom::anchor_c2`); `None` = střed monitoru
    pub axis2: Option<i32>,
    /// puls linky „Claude čeká / hotovo“: barva + začátek (`attention::pulse`); po termínu nic
    pub pulse: Option<(Rgba, Instant)>,
    /// přihrádka: tah souborů nad proužkem / právě puštěno (`shelf::overlay`)
    pub shelf: crate::shelf::Hint,
    /// styl zavřeného Ledge (`strip.look`)
    pub look: crate::looks::Look,
    /// co styl ukáže (`looks::show`); `None` = klasický proužek ze `segments` (`draw`)
    pub show: Option<crate::looks::Show>,
    /// smí se šířka ostrůvku měnit plynule (animace zapnuté v Ledge i ve Windows)
    pub animate: bool,
    /// místo u hrany (roh u stylu `corner`)
    pub position: crate::appearance::StripPosition,
    /// tvar kapsle (`strip.lookShape`, `looks::frame::shaped`)
    pub shape: crate::looks::Shape,
    /// vlastní vzhled (`strip.look = custom`, už oříznutý `Compose::norm`); jinak `None`
    pub compose: Option<crate::looks::compose::Compose>,
    /// běžící oznámení v proužku (Notch, `notify`); kreslí ho `notify::draw` místo obsahu proužku
    pub notice: Option<crate::notify::Live>,
    /// tečka zmeškaného oznámení (barva události) do příštího otevření šuplíku
    pub missed: Option<Rgba>,
    /// „Vyzkoušet“ živé hrany z editoru (`stripfx::edge_try`, dohraje za `edgefx::TRY_MS`)
    pub fx_try: Option<crate::stripfx::Try>,
    /// Windows nechce rušit (nerušit, prezentace): živá hrana se nehýbe (`toast::accepts_notifications`)
    pub fx_calm: bool,
}

/// Jas běžícího pulsu (barva, 0–1); `None` = puls není / skončil.
pub(crate) fn pulse_now(m: &StripModel) -> Option<(Rgba, f32)> {
    m.pulse.and_then(|(c, t)| crate::attention::pulse_level(t.elapsed().as_millis() as u64).map(|l| (c, l)))
}

impl Default for StripModel {
    fn default() -> Self {
        Self {
            visible: false,
            segments: vec![],
            ambient: None,
            opacity: 0.82,
            line_only: false,
            hot: false,
            monitor: RECT::default(),
            dpi: 96,
            style: StripStyle::Pill,
            metrics: geom::strip_metrics(StripStyle::Pill, crate::appearance::StripSize::Normal),
            palette: geom::palette(crate::appearance::Theme::Dark),
            line: [Rgba::VIOLET, Rgba::BLUE, Rgba::TEAL],
            axis2: None,
            pulse: None,
            shelf: crate::shelf::Hint::None,
            look: crate::looks::Look::Classic,
            show: None,
            animate: false,
            position: crate::appearance::StripPosition::Center,
            shape: crate::looks::Shape::Round,
            compose: None,
            notice: None,
            missed: None,
            fx_try: None,
            fx_calm: false,
        }
    }
}

/// Sdílený stav + okno proužku. Volání z libovolného vlákna (PostMessage).
pub struct Strip {
    pub model: Arc<Mutex<StripModel>>,
    /// skutečný obdélník proužku na obrazovce (px) — pro hover detekci
    pub rect: Arc<Mutex<RECT>>,
    /// poslední vykreslená velikost (px) — i když je proužek zrovna schovaný (výchozí tvar morfu)
    pub last: Arc<Mutex<(i32, i32)>>,
    hwnd: Arc<AtomicIsize>,
    /// vedlejší proužek (tečka na dalším monitoru, `stripset.rs`): bez přihrádky, bez oznámení, `display_event` nevolá
    secondary: bool,
    /// `stop` přišel dřív, než vlákno vytvořilo okno
    stopped: AtomicBool,
}

impl Strip {
    pub fn start() -> Arc<Strip> {
        Self::start_on(false)
    }

    /// Vlákno + okno + vlastní D2D factory (`SINGLE_THREADED` — factory se mezi vlákny nesdílí) pro jeden proužek.
    pub fn start_on(secondary: bool) -> Arc<Strip> {
        let strip = Arc::new(Strip {
            model: Arc::new(Mutex::new(StripModel::default())),
            rect: Arc::new(Mutex::new(RECT::default())),
            last: Arc::new(Mutex::new((0, 0))),
            hwnd: Arc::new(AtomicIsize::new(0)),
            secondary,
            stopped: AtomicBool::new(false),
        });
        let s = strip.clone();
        if crate::qa::isolated() {
            return strip; // QA: bez okna proužku (hwnd 0 → `post` je no-op); nepřekrývá ostrý proužek
        }
        std::thread::Builder::new()
            .name(if secondary { "strip2" } else { "strip" }.into())
            .spawn(move || unsafe { run(s) })
            .ok();
        strip
    }

    /// Nastaví model; překreslí jen při skutečné změně.
    pub fn set(&self, f: impl FnOnce(&mut StripModel)) {
        let changed = {
            let mut m = self.model.lock();
            let before = m.clone();
            f(&mut m);
            *m != before
        };
        if changed {
            self.post(WM_STRIP_REDRAW);
        }
    }

    /// Zruší okno a ukončí vlákno (vedlejší proužek při odpojení monitoru). Po volání se `Strip` už nepoužije.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.post(WM_STRIP_STOP);
    }

    fn post(&self, msg: u32) {
        let h = self.hwnd.load(Ordering::Acquire);
        if h != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(h as *mut _)), msg, WPARAM(0), LPARAM(0));
            }
        }
    }
}

pub(crate) struct Renderer {
    pub(crate) d2d: ID2D1Factory,
    pub(crate) dw: IDWriteFactory,
    pub(crate) target: ID2D1DCRenderTarget,
    pub(crate) fonts: crate::stripdraw::Fonts,
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            // Notch s tlačítkem (jen tehdy okno není WS_EX_TRANSPARENT): tvar bere myš, zbytek propouští
            WM_NCHITTEST => {
                let (x, y) = ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
                let on = crate::notify::hit().shape.is_some_and(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom);
                LRESULT(if on { HTCLIENT as isize } else { HTTRANSPARENT as isize })
            }
            WM_SETCURSOR if crate::notify::hit().shape.is_some() => {
                if let Ok(c) = LoadCursorW(None, IDC_HAND) {
                    SetCursor(Some(c));
                }
                LRESULT(1)
            }
            WM_LBUTTONUP => {
                let mut p = POINT { x: (lp.0 & 0xFFFF) as i16 as i32, y: ((lp.0 >> 16) & 0xFFFF) as i16 as i32 };
                let _ = windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut p);
                crate::notify::click(p.x, p.y);
                LRESULT(0)
            }
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            // změna zobrazení (monitor, DPI, hlavní panel): jen značka, přepočet udělá vlákno `hover`
            // (`state::display_event`); okno je top-level, takže broadcasty dostane (message-only okno ne)
            _ if crate::screen::is_display_msg(msg, wp.0) && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == 0 => {
                crate::state::display_event();
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn run(strip: Arc<Strip>) {
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("LedgeStrip");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | crate::winstyle::no_activate_ex();
        let Ok(hwnd) = CreateWindowExW(ex, class, w!("Ledge"), WS_POPUP, 0, 0, 1, 1, None, None, Some(hinst.into()), None)
        else {
            return;
        };
        // vedlejší okno zprávy o změně zobrazení nehlásí (stačí hlavní, počítadlo se nezdvojí) — značka pro `wndproc`
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, strip.secondary as isize);
        strip.hwnd.store(hwnd.0 as isize, Ordering::Release);
        if strip.stopped.load(Ordering::Acquire) {
            let _ = DestroyWindow(hwnd);
            return;
        }

        let renderer = (|| -> windows::core::Result<Renderer> {
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                // software: 24px proužek nepotřebuje GPU a nenatáhne do procesu ovladač grafiky
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            };
            let target = d2d.CreateDCRenderTarget(&props)?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let fonts = crate::stripdraw::pick_fonts(&dw);
            Ok(Renderer { d2d, dw, target, fonts })
        })();
        let Ok(renderer) = renderer else { return };
        // model, který přišel dřív než okno (`Strip::set` bez hwnd nic neposlal): první vykreslení
        let _ = PostMessageW(Some(hwnd), WM_STRIP_REDRAW, WPARAM(0), LPARAM(0));

        let mut shown = false;
        let mut pulsing = false;
        // styly zavřeného Ledge: střídání řádků (`ticker`, jen při víc událostech) a šířka ostrůvku
        let mut rotating = false;
        let mut spinning = false;
        let mut fxing = false;
        let mut rot: usize = 0;
        let mut anim = crate::looks::frame::WidthAnim::default();
        // živá hrana (kolo 28): hodiny jednorázového naplnění; snímky jen během animace (`stripfx::prepare`)
        let mut fx_clock = crate::stripfx::Clock::default();
        // Notch s tlačítkem: okno dočasně bez WS_EX_TRANSPARENT (jen v klidové fázi tvaru)
        let mut clickable = false;
        let t0 = Instant::now();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_STRIP_STOP {
                break;
            }
            let frame = msg.message == WM_TIMER && msg.wParam.0 == PULSE_TIMER;
            let turn = msg.message == WM_TIMER && msg.wParam.0 == ROTATE_TIMER;
            let spin = msg.message == WM_TIMER && msg.wParam.0 == SPIN_TIMER;
            let fxf = msg.message == WM_TIMER && msg.wParam.0 == FX_TIMER;
            if turn {
                rot = rot.wrapping_add(1);
            }
            if msg.message == WM_STRIP_REDRAW || msg.message == WM_STRIP_VISIBLE || frame || turn || spin || fxf {
                // sloučit nahromaděné požadavky na překreslení
                let mut peek = MSG::default();
                while PeekMessageW(&mut peek, Some(hwnd), WM_STRIP_REDRAW, WM_STRIP_VISIBLE, PM_REMOVE).as_bool() {}
                let mut model = strip.model.lock().clone();
                if !strip.secondary {
                    crate::shelf::overlay(&mut model, crate::state::CORE.get().is_none_or(|c| c.dead_surface()));
                }
                let looked = model.show.is_some() && !model.line_only;
                let fx_run = model.visible && crate::stripfx::prepare(&mut model, &mut fx_clock, t0.elapsed().as_millis() as u64);
                // oznámení (Notch): kreslí se místo obsahu, i u proužku skrytého v klidu
                let mut noted = None;
                if let Some(l) = model.notice.as_ref() {
                    noted = crate::notify::draw::draw(&renderer, hwnd, &model, l);
                    if noted.is_none() {
                        // doběhlo → další z fronty (pošle nové překreslení); teď obyčejný proužek
                        crate::notify::advance(l.acked);
                        model.notice = None;
                    }
                }
                let click = noted.is_some() && crate::notify::hit().button.is_some();
                if click != clickable {
                    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    let t = WS_EX_TRANSPARENT.0 as isize;
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, if click { ex & !t } else { ex | t });
                    clickable = click;
                }
                if !model.visible && noted.is_none() {
                    // skrytý proužek: žádný časovač, příště rovnou v cílové šířce
                    if pulsing {
                        let _ = KillTimer(Some(hwnd), PULSE_TIMER);
                        pulsing = false;
                    }
                    if rotating {
                        let _ = KillTimer(Some(hwnd), ROTATE_TIMER);
                        rotating = false;
                    }
                    if spinning {
                        let _ = KillTimer(Some(hwnd), SPIN_TIMER);
                        spinning = false;
                    }
                    if fxing {
                        let _ = KillTimer(Some(hwnd), FX_TIMER);
                        fxing = false;
                    }
                    anim.reset();
                    if shown {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                        shown = false;
                    }
                    *strip.rect.lock() = RECT::default();
                    continue;
                }
                let drawn = if let Some(win) = noted {
                    // `last` (tvar morfu) zůstává tělo proužku před oznámením
                    *strip.rect.lock() = win;
                    None
                } else if looked {
                    crate::stripdraw::draw(&renderer, hwnd, &model, rot, &mut anim, t0.elapsed().as_millis() as u64)
                } else {
                    anim.reset();
                    crate::stripclassic::draw(&renderer, hwnd, &model)
                };
                // snímkový časovač jen během pulsu nebo změny šířky ostrůvku; střídání jen u víc událostí
                let active = pulse_now(&model).is_some() || anim.running() || noted.is_some();
                if active != pulsing {
                    if active {
                        SetTimer(Some(hwnd), PULSE_TIMER, PULSE_FRAME_MS, None);
                    } else {
                        let _ = KillTimer(Some(hwnd), PULSE_TIMER);
                    }
                    pulsing = active;
                }
                let turns = looked && model.shelf == crate::shelf::Hint::None && model.show.as_ref().is_some_and(|s| s.rotate.len() > 1);
                if turns != rotating {
                    if turns {
                        SetTimer(Some(hwnd), ROTATE_TIMER, crate::looks::ROTATE_MS, None);
                    } else {
                        let _ = KillTimer(Some(hwnd), ROTATE_TIMER);
                    }
                    rotating = turns;
                }
                // točení prstenců: jen `Show.spin` (Claude pracuje) se zapnutými animacemi; jinak žádný časovač
                let spins = looked && model.animate && model.shelf == crate::shelf::Hint::None && model.show.as_ref().is_some_and(|s| s.spin);
                if spins != spinning {
                    if spins {
                        SetTimer(Some(hwnd), SPIN_TIMER, SPIN_FRAME_MS, None);
                    } else {
                        let _ = KillTimer(Some(hwnd), SPIN_TIMER);
                    }
                    spinning = spins;
                }
                // živá hrana: vlastní časovač jen během animace (ne přes oznámení)
                let fxs = fx_run && looked && noted.is_none();
                if fxs != fxing {
                    if fxs {
                        SetTimer(Some(hwnd), FX_TIMER, FX_FRAME_MS, None);
                    } else {
                        let _ = KillTimer(Some(hwnd), FX_TIMER);
                    }
                    fxing = fxs;
                }
                if let Some((r, body)) = drawn {
                    *strip.rect.lock() = r;
                    // výchozí tvar morfu = tělo proužku (bez stínů a záře kolem)
                    *strip.last.lock() = body;
                }
                if !shown {
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    shown = true;
                }
                if !frame && !turn && !spin && !fxf {
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // `Strip::stop`: okno pryč, `renderer` (D2D / DWrite) se uvolní s koncem funkce
        let _ = DestroyWindow(hwnd);
    }
}

pub(crate) fn color(c: Rgba, alpha_mul: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r: c.0, g: c.1, b: c.2, a: c.3 * alpha_mul }
}

pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

pub(crate) unsafe fn text_format(dw: &IDWriteFactory, size: f32, strong: bool, family: &str) -> windows::core::Result<IDWriteTextFormat> {
    unsafe { text_format_w(dw, size, strong, 0, family) }
}

/// Jako `text_format` + tloušťka vlastního vzhledu (0 = dnešní, 1 = polotučné, 2 = tučné; kolo 27).
pub(crate) unsafe fn text_format_w(dw: &IDWriteFactory, size: f32, strong: bool, weight: u8, family: &str) -> windows::core::Result<IDWriteTextFormat> {
    unsafe {
        let family: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
        dw.CreateTextFormat(
            PCWSTR(family.as_ptr()),
            None,
            match (weight, strong) {
                (0, false) => DWRITE_FONT_WEIGHT_MEDIUM,
                (0, true) | (1, false) => DWRITE_FONT_WEIGHT_SEMI_BOLD,
                _ => DWRITE_FONT_WEIGHT_BOLD,
            },
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("cs-CZ"),
        )
    }
}

pub(crate) unsafe fn measure(dw: &IDWriteFactory, fmt: &IDWriteTextFormat, s: &str) -> f32 {
    unsafe {
        let t = wide(s);
        match dw.CreateTextLayout(&t, fmt, 4000.0, 100.0) {
            Ok(layout) => {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() { m.widthIncludingTrailingWhitespace } else { 0.0 }
            }
            Err(_) => 0.0,
        }
    }
}

/// Rozměry proužku v logických px — sdílí je hover detekce i kreslení.
pub const HEIGHT: f32 = 32.0;
