//! Kreslení stylů zavřeného Ledge (`strip.look` mimo `classic`) do vrstveného okna proužku: D2D +
//! DirectWrite jako `strip::draw`, data z čistých funkcí `looks::show` (co) a `looks::frame` (tvar,
//! místo). Ikony ze systémového písma Segoe Fluent Icons (Windows 11), záložně Segoe MDL2 Assets,
//! jinak obyčejné znaky (`Glyph::fallback`) — žádné soubory navíc. Kreslí se jen při změně modelu,
//! při plynulé změně šířky ostrůvku (`frame::WidthAnim`, ~240 ms), při pulsu a u prstenců stylu `rings`
//! jen dokud Claude pracuje (`Show.spin`, ~8 snímků/s, bez animací vůbec); v klidu nic.

use crate::looks::frame::{self, Placed, Spec, WidthAnim};
use crate::looks::{Glyph, Look, Part, Show};
use crate::strip::{Renderer, Rgba, StripModel, color, measure, text_format, wide};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_SMALL, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE,
    D2D1_ROUNDED_RECT, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, ID2D1DCRenderTarget,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, IDWriteFactory, IDWriteFontCollection,
    IDWriteTextFormat,
};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{BOOL, PCWSTR};
use windows_numerics::Vector2;

const SANS: &str = "Segoe UI Variable Text";

/// Systémová písma stylů (zjištěná jednou při startu vlákna `strip`).
pub(crate) struct Fonts {
    /// písmo ikon; `None` = v systému není → `Glyph::fallback`
    pub icons: Option<&'static str>,
    pub mono: &'static str,
    pub display: &'static str,
}

/// První rodina ze seznamu, kterou systém má (DirectWrite, kolekce systémových písem).
unsafe fn first_family(dw: &IDWriteFactory, names: &[&'static str]) -> Option<&'static str> {
    unsafe {
        let mut coll: Option<IDWriteFontCollection> = None;
        dw.GetSystemFontCollection(&mut coll, false).ok()?;
        let coll = coll?;
        names.iter().copied().find(|n| {
            let w: Vec<u16> = n.encode_utf16().chain(std::iter::once(0)).collect();
            let (mut i, mut exists) = (0u32, BOOL(0));
            coll.FindFamilyName(PCWSTR(w.as_ptr()), &mut i, &mut exists).is_ok() && exists.as_bool()
        })
    }
}

pub(crate) unsafe fn pick_fonts(dw: &IDWriteFactory) -> Fonts {
    unsafe {
        Fonts {
            icons: first_family(dw, &["Segoe Fluent Icons", "Segoe MDL2 Assets"]),
            mono: first_family(dw, &["Cascadia Mono", "Consolas"]).unwrap_or("Consolas"),
            display: first_family(dw, &["Segoe UI Variable Display", "Segoe UI"]).unwrap_or("Segoe UI"),
        }
    }
}

pub(crate) struct Fmts {
    pub(crate) text: IDWriteTextFormat,
    strong: IDWriteTextFormat,
    icon: Option<IDWriteTextFormat>,
    /// náhradní znaky ikon, když písmo ikon chybí
    icon_alt: IDWriteTextFormat,
    badge: IDWriteTextFormat,
}

unsafe fn fmts(r: &Renderer, look: Look, sp: &Spec, ks: f32) -> Option<Fmts> {
    unsafe {
        let family = if sp.mono { r.fonts.mono } else if sp.display { r.fonts.display } else { SANS };
        let size = sp.font * ks;
        let icon_size = if look == Look::Icons { sp.font * ks } else { sp.font * 1.1 * ks };
        let f = |size: f32, strong: bool, family: &str| -> Option<IDWriteTextFormat> {
            let x = text_format(&r.dw, size, strong, family).ok()?;
            let _ = x.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            Some(x)
        };
        Some(Fmts {
            text: f(size, false, family)?,
            strong: f(size, true, family)?,
            icon: r.fonts.icons.and_then(|fam| f(icon_size, false, fam)),
            icon_alt: f(icon_size, false, "Segoe UI Symbol")?,
            badge: f(9.0 * ks, true, SANS)?,
        })
    }
}

/// Šířka části (px) a u ikony šířka odznaku.
unsafe fn part_w(r: &Renderer, fm: &Fmts, p: &Part, sp: &Spec, ks: f32) -> f32 {
    unsafe {
        match p {
            Part::Text { text, strong, .. } => measure(&r.dw, if *strong { &fm.strong } else { &fm.text }, text),
            Part::Dot(_) => sp.dot * ks,
            Part::Icon { glyph, badge, .. } => {
                let w = icon_w(r, fm, *glyph);
                w + badge.as_ref().map(|b| (measure(&r.dw, &fm.badge, b) + 4.0 * ks) * 0.6).unwrap_or(0.0)
            }
            Part::Sep => 1.0,
            Part::Battery { .. } => BATT_W * ks,
            Part::Ring { .. } => ring_d(sp) * ks,
            Part::Meter { .. } => METER_W * ks,
            Part::Spark { .. } | Part::Under { .. } => crate::stripcompose::extra_w(r, fm, p, ks),
        }
    }
}

/// Průměr prstence (vlastní vzhled má vlastní, ostatní styly = `dot`).
fn ring_d(sp: &Spec) -> f32 {
    if sp.ring > 0.0 { sp.ring } else { sp.dot }
}

/// Šířka a výška sloupku „malý graf“ vlastního vzhledu (logické px).
const METER_W: f32 = 24.0;
const METER_H: f32 = 5.0;

/// Šířka a výška tvaru baterie (logické px, bez „čudlíku“).
const BATT_W: f32 = 20.0;
const BATT_H: f32 = 10.0;
/// Jedna otáčka točícího se oblouku prstence (ms).
pub(crate) const SPIN_MS: u64 = 1400;

/// Bod na kružnici; úhel 0 = nahoře, po směru hodinových ručiček.
fn on_circle(c: Vector2, r: f32, turn: f32) -> Vector2 {
    let a = turn * std::f32::consts::TAU;
    Vector2 { X: c.X + r * a.sin(), Y: c.Y - r * a.cos() }
}

/// Oblouk od `from` do `to` (podíl otáčky, 0 = nahoře) jako tah štětcem.
#[allow(clippy::too_many_arguments)]
unsafe fn arc(t: &ID2D1DCRenderTarget, r: &Renderer, c: Vector2, rad: f32, from: f32, to: f32, b: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush, w: f32) {
    unsafe {
        let span = (to - from).clamp(0.0, 1.0);
        if span <= 0.001 {
            return;
        }
        if span >= 0.999 {
            t.DrawEllipse(&D2D1_ELLIPSE { point: c, radiusX: rad, radiusY: rad }, b, w, None);
            return;
        }
        let Ok(path) = r.d2d.CreatePathGeometry() else { return };
        let Ok(sink) = path.Open() else { return };
        sink.BeginFigure(on_circle(c, rad, from), windows::Win32::Graphics::Direct2D::Common::D2D1_FIGURE_BEGIN_HOLLOW);
        sink.AddArc(&D2D1_ARC_SEGMENT {
            point: on_circle(c, rad, from + span),
            size: D2D_SIZE_F { width: rad, height: rad },
            rotationAngle: 0.0,
            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
            arcSize: if span > 0.5 { windows::Win32::Graphics::Direct2D::D2D1_ARC_SIZE_LARGE } else { D2D1_ARC_SIZE_SMALL },
        });
        sink.EndFigure(windows::Win32::Graphics::Direct2D::Common::D2D1_FIGURE_END_OPEN);
        if sink.Close().is_ok() {
            t.DrawGeometry(&path, b, w, None);
        }
    }
}

unsafe fn icon_w(r: &Renderer, fm: &Fmts, g: Glyph) -> f32 {
    unsafe {
        match &fm.icon {
            Some(f) => measure(&r.dw, f, &g.code().to_string()),
            None => measure(&r.dw, &fm.icon_alt, g.fallback()),
        }
    }
}

pub(crate) unsafe fn row_w(r: &Renderer, fm: &Fmts, parts: &[Part], sp: &Spec, ks: f32) -> f32 {
    unsafe {
        let gap = sp.gap * ks;
        let sum: f32 = parts.iter().map(|p| part_w(r, fm, p, sp, ks)).sum();
        sum + gap * parts.len().saturating_sub(1) as f32
    }
}

pub(crate) fn brush(t: &ID2D1DCRenderTarget, c: Rgba, a: f32) -> Option<windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush> {
    unsafe { t.CreateSolidColorBrush(&color(c, a), None).ok() }
}

/// Části vedle sebe od `x`, svisle na středu pásu `y0..y1`. `spin` = natočení točícího se oblouku
/// prstenců (podíl otáčky), `None` = netočí se.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn draw_parts(t: &ID2D1DCRenderTarget, r: &Renderer, fm: &Fmts, parts: &[Part], sp: &Spec, ks: f32, mut x: f32, y0: f32, y1: f32, spin: Option<f32>) {
    unsafe {
        let mid = (y0 + y1) / 2.0;
        let gap = sp.gap * ks;
        for p in parts {
            let w = part_w(r, fm, p, sp, ks);
            match p {
                Part::Text { text, color: c, strong } => {
                    if let Some(b) = brush(t, *c, 1.0) {
                        let lr = D2D_RECT_F { left: x, top: y0, right: x + w + 2.0, bottom: y1 };
                        t.DrawText(&wide(text), if *strong { &fm.strong } else { &fm.text }, &lr, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                    }
                }
                Part::Dot(c) => {
                    let d = sp.dot * ks;
                    let center = Vector2 { X: x + d / 2.0, Y: mid };
                    if let Some(b) = brush(t, *c, 0.25) {
                        t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: d * 0.85, radiusY: d * 0.85 }, &b);
                    }
                    if let Some(b) = brush(t, *c, 1.0) {
                        t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: d / 2.0, radiusY: d / 2.0 }, &b);
                    }
                }
                Part::Icon { glyph, color: c, badge } => {
                    let iw = icon_w(r, fm, *glyph);
                    if let Some(b) = brush(t, *c, 1.0) {
                        let lr = D2D_RECT_F { left: x, top: y0, right: x + iw + 2.0, bottom: y1 };
                        match &fm.icon {
                            Some(f) => t.DrawText(&wide(&glyph.code().to_string()), f, &lr, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL),
                            None => t.DrawText(&wide(glyph.fallback()), &fm.icon_alt, &lr, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL),
                        }
                    }
                    if let Some(n) = badge {
                        // odznak vpravo nahoře u ikony, v barvě ikony (tlumená → jantar), černé číslo
                        let bw = measure(&r.dw, &fm.badge, n) + 4.0 * ks;
                        let bh = 11.0 * ks;
                        let bx = x + iw - 3.0 * ks;
                        let by = (mid - sp.font * ks * 0.55 - 1.0 * ks).max(0.0);
                        let tone = if *c == Rgba::DIM { Rgba::AMBER } else { *c };
                        if let Some(b) = brush(t, tone, 1.0) {
                            let rr = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: bx, top: by, right: bx + bw, bottom: by + bh }, radiusX: bh / 2.0, radiusY: bh / 2.0 };
                            t.FillRoundedRectangle(&rr, &b);
                        }
                        if let Some(b) = brush(t, Rgba(0.0, 0.0, 0.0, 1.0), 1.0) {
                            let lr = D2D_RECT_F { left: bx + 2.0 * ks, top: by, right: bx + bw, bottom: by + bh };
                            t.DrawText(&wide(n), &fm.badge, &lr, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                        }
                    }
                }
                Part::Sep => {
                    // stavová lišta (mono) = tmavě zelená jako dřív, kapsle (trojice) = jemná bílá
                    let tone = if sp.mono { Rgba(0.11, 0.16, 0.125, 1.0) } else { Rgba(1.0, 1.0, 1.0, 0.2) };
                    if let Some(b) = brush(t, tone, 1.0) {
                        t.FillRectangle(&D2D_RECT_F { left: x, top: mid - 6.0 * ks, right: x + 1.0, bottom: mid + 6.0 * ks }, &b);
                    }
                }
                Part::Battery { percent, charging } => {
                    // obrys, „čudlík“ vpravo a výplň podle procent (≤ 20 % červeně, nabíjení zeleně)
                    let (bw, bh) = ((BATT_W - 2.0) * ks, BATT_H * ks);
                    let top = mid - bh / 2.0;
                    let rr = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x + 0.5, top, right: x + bw, bottom: top + bh }, radiusX: 2.5 * ks, radiusY: 2.5 * ks };
                    if let Some(b) = brush(t, Rgba::TEXT, 0.7) {
                        t.DrawRoundedRectangle(&rr, &b, 1.0 * ks, None);
                        t.FillRectangle(&D2D_RECT_F { left: x + bw + 0.5, top: mid - 2.0 * ks, right: x + bw + 2.0 * ks, bottom: mid + 2.0 * ks }, &b);
                    }
                    let tone = if *charging { Rgba::GREEN } else if *percent <= 20 { Rgba::RED } else { Rgba::TEXT };
                    let inner = (bw - 3.0 * ks) * (*percent as f32 / 100.0).clamp(0.0, 1.0);
                    if inner > 0.5
                        && let Some(b) = brush(t, tone, 1.0)
                    {
                        let ir = D2D_RECT_F { left: x + 2.0 * ks, top: top + 1.5 * ks, right: x + 2.0 * ks + inner, bottom: top + bh - 1.5 * ks };
                        t.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: ir, radiusX: 1.2 * ks, radiusY: 1.2 * ks }, &b);
                    }
                }
                Part::Ring { fill, color: c } => {
                    // dráha, výplň od vrcholu po směru hodin (jen ze skutečného limitu), při práci krátký
                    // točící se oblouk vně
                    let d = ring_d(sp) * ks;
                    let center = Vector2 { X: x + d / 2.0, Y: mid };
                    let sw = 2.2 * ks;
                    let rad = d / 2.0 - sw / 2.0;
                    if let Some(b) = brush(t, *c, 0.25) {
                        t.DrawEllipse(&D2D1_ELLIPSE { point: center, radiusX: rad, radiusY: rad }, &b, sw, None);
                    }
                    if let (Some(f), Some(b)) = (fill, brush(t, *c, 1.0)) {
                        arc(t, r, center, rad, 0.0, *f, &b, sw);
                    }
                    if let (Some(turn), Some(b)) = (spin, brush(t, Rgba::TEXT, 0.9)) {
                        arc(t, r, center, rad, turn, turn + 0.22, &b, sw * 0.7);
                    }
                }
                Part::Meter { fill, color: c } => {
                    // dráha + výplň podle skutečné hodnoty (vlastní vzhled, styl „malý graf“)
                    let (mw, mh) = (METER_W * ks, METER_H * ks);
                    let track = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x, top: mid - mh / 2.0, right: x + mw, bottom: mid + mh / 2.0 }, radiusX: mh / 2.0, radiusY: mh / 2.0 };
                    if let Some(b) = brush(t, *c, 0.25) {
                        t.FillRoundedRectangle(&track, &b);
                    }
                    let fw = mw * fill.clamp(0.0, 1.0);
                    if fw > 0.5
                        && let Some(b) = brush(t, *c, 1.0)
                    {
                        t.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: D2D_RECT_F { right: x + fw.max(mh), ..track.rect }, ..track }, &b);
                    }
                }
                Part::Spark { .. } | Part::Under { .. } => crate::stripcompose::draw_extra(t, r, fm, p, sp, ks, x, y0, y1, w),
            }
            x += w + gap;
        }
    }
}

/// Oblá „ouška“ výřezu: vyduté čtvrtkruhy u horní hrany vlevo a vpravo od těla.
pub(crate) unsafe fn ears(t: &ID2D1DCRenderTarget, r: &Renderer, bw: f32, e: f32, fill: Rgba) {
    unsafe {
        let Ok(path) = r.d2d.CreatePathGeometry() else { return };
        let Ok(sink) = path.Open() else { return };
        let arc = |to: Vector2, dir| D2D1_ARC_SEGMENT { point: to, size: D2D_SIZE_F { width: e, height: e }, rotationAngle: 0.0, sweepDirection: dir, arcSize: D2D1_ARC_SIZE_SMALL };
        // levé: (−e,0) → (0,0) → (0,e) → oblouk zpět k (−e,0)
        sink.BeginFigure(Vector2 { X: -e, Y: 0.0 }, D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(Vector2 { X: 0.5, Y: 0.0 });
        sink.AddLine(Vector2 { X: 0.5, Y: e });
        sink.AddArc(&arc(Vector2 { X: -e, Y: 0.0 }, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        // pravé zrcadlově
        sink.BeginFigure(Vector2 { X: bw + e, Y: 0.0 }, D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(Vector2 { X: bw - 0.5, Y: 0.0 });
        sink.AddLine(Vector2 { X: bw - 0.5, Y: e });
        sink.AddArc(&arc(Vector2 { X: bw + e, Y: 0.0 }, D2D1_SWEEP_DIRECTION_CLOCKWISE));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        if sink.Close().is_ok()
            && let Some(b) = brush(t, fill, 1.0)
        {
            t.FillGeometry(&path, &b, None);
        }
    }
}

/// Nažhavení (myš na proužku): linka akcentu dole uprostřed — nad stálou linkou vlastního vzhledu.
fn hot_line(t: &ID2D1DCRenderTarget, m: &StripModel, bwf: f32, bhf: f32, scale: f32) {
    if m.hot
        && let Some(b) = brush(t, m.line[0], 0.9)
    {
        let lh = 2.0 * scale;
        unsafe { t.FillRectangle(&D2D_RECT_F { left: bwf * 0.2, top: bhf - lh, right: bwf * 0.8, bottom: bhf }, &b) };
    }
}

/// Překryv přihrádky (tah souborů) ve tvaru stylu: segment z `shelf::overlay` → tečka + text.
fn shelf_show(m: &StripModel, show: &Show) -> Show {
    let Some(s) = m.segments.first() else { return show.clone() };
    let c = s.dot.unwrap_or(Rgba::TEAL);
    Show {
        parts: vec![Part::Dot(c), Part::Text { text: s.text.clone(), color: s.color, strong: s.strong }],
        line: vec![(1.0, c)],
        ..Show::default()
    }
}

/// Vykreslí styl a umístí okno. Vrací obdélník okna na obrazovce a tělo pro morf (px).
/// `rot` = pořadí střídaného řádku (`ticker`), `anim` = šířka ostrůvku, `now` = ms vlákna.
pub(crate) unsafe fn draw(r: &Renderer, hwnd: HWND, m: &StripModel, rot: usize, anim: &mut WidthAnim, now: u64) -> Option<(RECT, (i32, i32))> {
    unsafe {
        let look = m.look;
        let base = m.show.as_ref()?;
        let shelf = m.shelf != crate::shelf::Hint::None;
        let show = if shelf { shelf_show(m, base) } else { base.clone() };
        // vlastní vzhled (kolo 23): tvar a rozměry ze `strip.compose`; umístí se jako linka / roh / na ose
        let custom = m.compose.as_ref().filter(|_| look == Look::Custom);
        let at = custom.map(|c| c.place_look()).unwrap_or(look);
        let rotated: Option<&[Part]> = (!show.rotate.is_empty()).then(|| show.rotate[rot % show.rotate.len()].as_slice());
        // vlastní: `parts` = vlevo, střed se střídá; přihrádka (tah souborů) doprostřed
        let (parts, center): (&[Part], &[Part]) = match (custom.is_some(), shelf) {
            (true, true) => (&[], &show.parts),
            (true, false) => (&show.parts, rotated.unwrap_or(&show.center)),
            _ => (rotated.unwrap_or(&show.parts), &[]),
        };
        let sp = match custom {
            Some(c) => crate::looks::compose::spec(c),
            None => frame::shaped(look, frame::spec(look, &show), m.shape),
        };
        let scale = m.dpi as f32 / 96.0;
        let ks = scale * m.metrics.k;
        let fm = fmts(r, look, &sp, ks)?;
        // natočení prstenců: jen když Claude pracuje a animace jsou zapnuté (jinak statický oblouk nic)
        let spin = (show.spin && m.animate).then(|| (now % SPIN_MS) as f32 / SPIN_MS as f32);
        let pulse = crate::strip::pulse_now(m);
        let mon = m.monitor;
        let axis2 = m.axis2.unwrap_or(2 * mon.left + (mon.right - mon.left));

        let left_w = row_w(r, &fm, parts, &sp, ks);
        let right_w = row_w(r, &fm, &show.right, &sp, ks);
        let center_w = row_w(r, &fm, center, &sp, ks);
        // vlastní (kolo 25): díly zón s oddělením a výzdobou; co se nevejde na monitor, schová se od konce zóny
        let lay = match (custom.and(base.paint), at) {
            (Some(p), at) if at != Look::Line => {
                let seg1: &[crate::looks::Seg] = match (&show.segs[1], show.rotate.len()) {
                    (s, n) if n > 0 && s.len() == n => std::slice::from_ref(&s[rot % n]),
                    (s, _) => s,
                };
                let avail = (mon.right - mon.left) as f32 - 2.0 * sp.pad * ks;
                Some(crate::stripcompose::layout(r, &fm, &sp, ks, p, [(parts, &show.segs[0]), (center, seg1), (&show.right, &show.segs[2])], avail))
            }
            _ => None,
        };
        let content = match look {
            Look::Custom if lay.is_some() => lay.as_ref().map(|l| l.content).unwrap_or_default(),
            // střed na ose těla, boky stejně široké (širší z nich)
            Look::Custom if center_w > 0.0 => {
                let side = left_w.max(right_w);
                center_w + if side > 0.0 { 2.0 * (side + sp.gap * ks) } else { 0.0 }
            }
            Look::Custom => left_w + right_w + if left_w > 0.0 && right_w > 0.0 { sp.gap * ks } else { 0.0 },
            Look::Progress | Look::Bar if right_w > 0.0 => left_w + 2.0 * sp.gap * ks + right_w,
            Look::Dots => parts.len() as f32 * sp.dot * ks + parts.len().saturating_sub(1) as f32 * sp.gap * ks,
            _ => left_w,
        };
        let natural = frame::body_width(at, content, &sp, ks, mon);
        // vlastní vzhled nejvýš na šířku monitoru (šířka až 1200 × velikost)
        let natural = if custom.is_some() { natural.min((mon.right - mon.left).max(1)) } else { natural };
        let bw = if look == Look::Island && !shelf {
            anim.step(natural as f32, m.animate, now).round() as i32
        } else {
            anim.reset();
            natural
        };
        let line_k = if at == Look::Line { scale } else { ks };
        let bh = ((sp.h * line_k).round() as i32).max(1);
        let Placed { mut win, ox, oy, bw, bh } = frame::place(at, m.position, mon, axis2, bw, bh, &sp, ks);
        // záře pod tělem potřebuje pár px okna navíc (jen s `deco.edge: glow`)
        if lay.as_ref().is_some_and(|l| l.paint.edge == crate::looks::composedeco::Edge::Glow) {
            win.bottom += (crate::stripcompose::GLOW_PX * scale).ceil() as i32;
        }
        let (w, h) = (win.right - win.left, win.bottom - win.top);
        let (bwf, bhf) = (bw as f32, bh as f32);

        // DIB pro vrstvené okno
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let Ok(dib) = CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            return None;
        };
        let old = SelectObject(mem, HGDIOBJ(dib.0));
        let rect = RECT { left: 0, top: 0, right: w, bottom: h };
        if r.target.BindDC(mem, &rect).is_ok() {
            let t = &r.target;
            t.BeginDraw();
            t.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            t.SetTransform(&windows_numerics::Matrix3x2::translation(ox as f32, oy as f32));
            let radius = sp.r * ks;
            let body = if sp.pill {
                let pr = if sp.pill_r > 0.0 { (sp.pill_r * ks).min(bhf / 2.0) } else { bhf / 2.0 };
                D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: 0.5, top: 0.5, right: bwf - 0.5, bottom: bhf - 0.5 }, radiusX: pr, radiusY: pr }
            } else {
                // horní rohy „za hranou“: obdélník začíná nad obrazovkou
                D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: 0.0, top: -radius - 1.0, right: bwf, bottom: bhf }, radiusX: radius, radiusY: radius }
            };
            if at == Look::Line {
                let total: f32 = show.line.iter().map(|x| x.0).sum::<f32>().max(0.001);
                let a = if m.hot { 1.0 } else if show.idle { 0.55 } else { 0.95 };
                let mut x = 0.0;
                for (wgt, c) in &show.line {
                    let lw = bwf * wgt / total;
                    if let Some(b) = brush(t, *c, a) {
                        t.FillRectangle(&D2D_RECT_F { left: x, top: 0.0, right: x + lw, bottom: bhf }, &b);
                    }
                    x += lw;
                }
                if let Some((c, l)) = pulse
                    && let Some(b) = brush(t, c, 0.25 + 0.6 * l)
                {
                    t.FillRectangle(&D2D_RECT_F { left: 0.0, top: 0.0, right: bwf, bottom: bhf }, &b);
                }
            } else if look == Look::Dots {
                let d = sp.dot * ks;
                let gap = sp.gap * ks;
                let mid = bhf / 2.0;
                let mut x = (bwf - content) / 2.0;
                for p in parts {
                    if let Part::Dot(c) = p {
                        let center = Vector2 { X: x + d / 2.0, Y: mid };
                        // tmavý lem: tečky jsou čitelné na světlé ploše i bez těla
                        if let Some(b) = brush(t, Rgba(0.0, 0.0, 0.0, 1.0), 0.35) {
                            t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: d / 2.0 + 2.0 * ks, radiusY: d / 2.0 + 2.0 * ks }, &b);
                        }
                        let a = if m.hot { 1.0 } else { 0.95 };
                        if let Some(b) = brush(t, *c, a) {
                            t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: d / 2.0, radiusY: d / 2.0 }, &b);
                        }
                    }
                    x += d + gap;
                }
            } else if let Some(l) = lay.as_ref() {
                let bodies = crate::stripcompose::paint(t, r, &fm, &sp, ks, scale, l, body, bwf, bhf, spin);
                if let Some((c, l)) = pulse
                    && let Some(b) = brush(t, c, 0.35 + 0.65 * l)
                {
                    for x in &bodies {
                        t.DrawRoundedRectangle(x, &b, 1.5 * scale, None);
                    }
                }
                hot_line(t, m, bwf, bhf, scale);
            } else {
                if sp.fill.3 > 0.0
                    && let Some(b) = brush(t, sp.fill, 1.0)
                {
                    t.FillRoundedRectangle(&body, &b);
                    if sp.ears > 0.0 {
                        ears(t, r, bwf, sp.ears * ks, sp.fill);
                    }
                }
                if let Some(e) = sp.edge
                    && let Some(b) = brush(t, e, 1.0)
                {
                    if sp.edge_bottom {
                        t.FillRectangle(&D2D_RECT_F { left: 0.0, top: bhf - 1.0, right: bwf, bottom: bhf }, &b);
                    } else {
                        t.DrawRoundedRectangle(&body, &b, 1.0, None);
                    }
                }
                // obsah oříznutý na tělo (ostrůvek během změny šířky)
                t.PushAxisAlignedClip(&D2D_RECT_F { left: 0.0, top: 0.0, right: bwf, bottom: bhf }, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                let pad = sp.pad * ks;
                match look {
                    Look::Bar => {
                        draw_parts(t, r, &fm, parts, &sp, ks, pad, 0.0, bhf, None);
                        draw_parts(t, r, &fm, &show.right, &sp, ks, bwf - pad - right_w, 0.0, bhf, None);
                    }
                    Look::Progress => {
                        let row_b = if show.progress.is_some() { bhf - 8.0 * ks } else { bhf };
                        draw_parts(t, r, &fm, parts, &sp, ks, pad, 1.0 * ks, row_b, None);
                        draw_parts(t, r, &fm, &show.right, &sp, ks, bwf - pad - right_w, 1.0 * ks, row_b, None);
                        if let Some((frac, c)) = show.progress {
                            let (ty, th) = (bhf - 7.0 * ks, 3.0 * ks);
                            let track = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: pad, top: ty, right: bwf - pad, bottom: ty + th }, radiusX: th / 2.0, radiusY: th / 2.0 };
                            if let Some(b) = brush(t, Rgba(1.0, 1.0, 1.0, 1.0), 0.12) {
                                t.FillRoundedRectangle(&track, &b);
                            }
                            let fw = (bwf - 2.0 * pad) * frac.clamp(0.0, 1.0);
                            if fw > 0.5
                                && let Some(b) = brush(t, c, 1.0)
                            {
                                t.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: D2D_RECT_F { right: pad + fw.max(th), ..track.rect }, ..track }, &b);
                            }
                        }
                    }
                    Look::Custom => {
                        draw_parts(t, r, &fm, parts, &sp, ks, pad, 0.0, bhf, spin);
                        draw_parts(t, r, &fm, center, &sp, ks, (bwf - center_w) / 2.0, 0.0, bhf, spin);
                        draw_parts(t, r, &fm, &show.right, &sp, ks, bwf - pad - right_w, 0.0, bhf, spin);
                    }
                    _ => draw_parts(t, r, &fm, parts, &sp, ks, (bwf - content) / 2.0, 0.0, bhf, spin),
                }
                t.PopAxisAlignedClip();
                // puls „čeká / hotovo“: obrys těla v barvě události; nažhavení: linka akcentu dole
                if let Some((c, l)) = pulse
                    && let Some(b) = brush(t, c, 0.35 + 0.65 * l)
                {
                    t.DrawRoundedRectangle(&body, &b, 1.5 * scale, None);
                }
                hot_line(t, m, bwf, bhf, scale);
            }
            t.SetTransform(&windows_numerics::Matrix3x2::identity());
            let _ = t.EndDraw(None, None);
        }
        let pos = POINT { x: win.left, y: win.top };
        let size = SIZE { cx: w, cy: h };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        let _ = UpdateLayeredWindow(hwnd, Some(screen), Some(&pos), Some(&size), Some(mem), Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(dib.0));
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        Some((win, frame::morph_body(at, bw, bh + oy, scale)))
    }
}
