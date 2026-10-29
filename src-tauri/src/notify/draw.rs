//! Kreslení Notche do vrstveného okna proužku (D2D + DirectWrite jako `stripdraw`). Tvar dává čistá
//! `choreo::frame`, tady jen pixely: tělo u horní hrany (nebo kapka pod ním), záře / vlny, ikona události,
//! text, tlačítko. Zapisuje `notify::Hit` (kde je tvar a tlačítko) — jen v klidové fázi tvaru.

use super::choreo::{self, Frame};
use super::{Hit, Kind, Live, NotchStyle};
use crate::strip::{Renderer, Rgba, StripModel, measure, text_format, wide};
use crate::stripdraw::{arc, brush};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE, D2D1_ROUNDED_RECT, ID2D1DCRenderTarget,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_WORD_WRAPPING_NO_WRAP,
    IDWriteTextFormat,
};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows_numerics::Vector2;

const SANS: &str = "Segoe UI Variable Text";
/// Místo kolem tvaru na záři a vlny (DIP).
const MARGIN: f32 = 18.0;
/// Tělo Notche: skoro černé jako výřez displeje (v každém motivu stejné — je to „díra“ v obrazovce).
const FILL: Rgba = Rgba(0.045, 0.045, 0.06, 0.97);
const INK: Rgba = Rgba(0.07, 0.07, 0.09, 1.0);
const TEXT: Rgba = Rgba(0.96, 0.96, 0.98, 1.0);
const DIM: Rgba = Rgba(0.70, 0.72, 0.78, 1.0);

struct Fm {
    title: IDWriteTextFormat,
    body: IDWriteTextFormat,
    glyph: IDWriteTextFormat,
    button: IDWriteTextFormat,
}

unsafe fn fmts(r: &Renderer, k: f32) -> Option<Fm> {
    unsafe {
        let f = |size: f32, strong: bool| -> Option<IDWriteTextFormat> {
            let x = text_format(&r.dw, size * k, strong, SANS).ok()?;
            let _ = x.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let _ = x.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
            Some(x)
        };
        let glyph = f(13.0, true)?;
        let _ = glyph.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
        let button = f(12.0, true)?;
        let _ = button.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
        Some(Fm { title: f(13.0, true)?, body: f(12.0, false)?, glyph, button })
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F {
    D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h }
}

/// Zaoblený obdélník; `flat_top` = horní rohy „za hranou“ obrazovky (tělo roste z proužku).
fn rr(r: D2D_RECT_F, rad: f32, flat_top: bool) -> D2D1_ROUNDED_RECT {
    let top = if flat_top { r.top - rad - 1.0 } else { r.top };
    D2D1_ROUNDED_RECT { rect: D2D_RECT_F { top, ..r }, radiusX: rad, radiusY: rad }
}

unsafe fn text(t: &ID2D1DCRenderTarget, s: &str, f: &IDWriteTextFormat, r: D2D_RECT_F, c: Rgba, a: f32) {
    unsafe {
        if let Some(b) = brush(t, c, a) {
            t.DrawText(&wide(s), f, &r, &b, D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
        }
    }
}

/// Snímek Notche. `None` = oznámení doběhlo (volající zavolá `notify::advance`).
pub(crate) unsafe fn draw(r: &Renderer, hwnd: HWND, m: &StripModel, l: &Live) -> Option<RECT> {
    unsafe {
        let k = m.dpi as f32 / 96.0;
        let base = if l.base.0 > 0 && l.base.1 > 0 { (l.base.0 as f32 / k, l.base.1 as f32 / k) } else { (0.0, 0.0) };
        let now = l.elapsed();
        let f = choreo::frame(l.style, now, l.hold, base)?;
        let fm = fmts(r, k)?;
        let mon = m.monitor;
        let cx = m.axis2.map(|a| a as f32 / 2.0).unwrap_or((mon.left + mon.right) as f32 / 2.0);
        let (sw, sh, bw, bh, mg) = (f.w * k, f.h * k, base.0 * k, base.1 * k, MARGIN * k);
        // tvar: kapka = tělo proužku + bublina pod ním; jinak jedno tělo u hrany
        let bubble_y = f.drop_y.map(|y| bh + y * k);
        let cw = if bubble_y.is_some() { sw.max(bw) } else { sw };
        let ch = bubble_y.map_or(sh, |y| (y + sh).max(bh));
        let (w, h) = ((cw + 2.0 * mg).ceil() as i32, (ch + mg).ceil() as i32);
        let left = ((cx - w as f32 / 2.0).round() as i32).clamp(mon.left, (mon.right - w).max(mon.left));
        let win = RECT { left, top: mon.top, right: left + w, bottom: mon.top + h };
        let card = match bubble_y {
            Some(y) => rect((w as f32 - sw) / 2.0, y, sw, sh),
            None => rect((w as f32 - sw) / 2.0, 0.0, sw, sh),
        };
        let body = bubble_y.map(|_| rect((w as f32 - bw) / 2.0, 0.0, bw, bh));

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
            return Some(win);
        };
        let old = SelectObject(mem, HGDIOBJ(dib.0));
        let mut button = None;
        if r.target.BindDC(mem, &RECT { left: 0, top: 0, right: w, bottom: h }).is_ok() {
            let t = &r.target;
            t.BeginDraw();
            t.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            button = paint(t, r, &fm, l, &f, k, card, body, now);
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
        // klikací jen stojící tvar s tlačítkem
        let to_screen = |r: D2D_RECT_F| RECT {
            left: win.left + r.left.floor() as i32,
            top: win.top + r.top.max(0.0).floor() as i32,
            right: win.left + r.right.ceil() as i32,
            bottom: win.top + r.bottom.ceil() as i32,
        };
        let hit = match button.filter(|_| f.settled) {
            Some(b) => {
                let shape = match body {
                    Some(bd) => D2D_RECT_F { left: card.left.min(bd.left), top: 0.0, right: card.right.max(bd.right), bottom: card.bottom },
                    None => card,
                };
                Hit { shape: Some(to_screen(shape)), button: Some(to_screen(b)) }
            }
            None => Hit::default(),
        };
        super::set_hit(hit);
        Some(win)
    }
}

/// Vše uvnitř okna; vrací obdélník tlačítka (lokální px), když se kreslilo.
#[allow(clippy::too_many_arguments)]
unsafe fn paint(t: &ID2D1DCRenderTarget, r: &Renderer, fm: &Fm, l: &Live, f: &Frame, k: f32, card: D2D_RECT_F, body: Option<D2D_RECT_F>, now: u64) -> Option<D2D_RECT_F> {
    unsafe {
        let kind = l.n.kind;
        let accent = kind.color();
        let (cw, ch) = (card.right - card.left, card.bottom - card.top);
        let flat = body.is_none();
        let rad = (ch / 2.0).min(20.0 * k);
        let shape = rr(card, rad, flat);
        // záře (styl „jen záře“ a slabě i čekání na tebe)
        let glow = f.glow.max(if kind == Kind::Waiting { 0.35 * f.alpha } else { 0.0 });
        if glow > 0.01 {
            for (wd, a) in [(14.0, 0.12), (8.0, 0.22), (3.5, 0.5)] {
                if let Some(b) = brush(t, accent, a * glow) {
                    t.DrawRoundedRectangle(&shape, &b, wd * k, None);
                }
            }
        }
        // časovač: vlny, které se rozbíhají od tvaru
        if kind == Kind::Timer && f.alpha > 0.01 {
            let ph0 = (now as f32 / 1400.0).fract();
            for i in 0..3 {
                let ph = (ph0 + i as f32 / 3.0).fract();
                let inf = ph * MARGIN * 0.9 * k;
                let wave = D2D_RECT_F { left: card.left - inf, top: card.top - inf, right: card.right + inf, bottom: card.bottom + inf };
                if let Some(b) = brush(t, accent, (1.0 - ph) * 0.55 * f.alpha) {
                    t.DrawRoundedRectangle(&rr(wave, rad + inf, flat), &b, 1.6 * k, None);
                }
            }
        }
        // tělo (u kapky nejdřív bublina, přes ni proužek — bublina „vyjede zpod“ něj)
        if let Some(b) = brush(t, FILL, 1.0) {
            t.FillRoundedRectangle(&shape, &b);
            if let Some(bd) = body.filter(|bd| bd.right - bd.left > 0.5) {
                t.FillRoundedRectangle(&rr(bd, ((bd.bottom - bd.top) / 2.0).min(14.0 * k), true), &b);
            }
        }
        if let Some(b) = brush(t, accent, 0.35 + 0.4 * f.alpha) {
            t.DrawRoundedRectangle(&shape, &b, 1.0 * k, None);
        }
        let a = f.alpha;
        t.PushAxisAlignedClip(&card, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        // limit: jantarový pruh přejede přes tvar (první třetina držení) + plnění podle procent dole
        if kind == Kind::Limit {
            let sweep = (f.progress / 0.35).clamp(0.0, 1.0);
            if sweep > 0.0 && sweep < 1.0 {
                let band = cw * 0.35;
                let x = card.left - band + (cw + band) * sweep;
                if let Some(b) = brush(t, accent, 0.28 * a) {
                    t.FillRectangle(&rect(x, card.top, band, ch), &b);
                }
            }
            let pct = l.n.pct.unwrap_or(0.0).clamp(0.0, 100.0) / 100.0;
            let inset = rad * 0.7;
            if pct > 0.0
                && let Some(b) = brush(t, accent, a)
            {
                t.FillRectangle(&rect(card.left + inset, card.bottom - 4.0 * k, (cw - 2.0 * inset) * pct, 2.5 * k), &b);
            }
        }
        if a <= 0.01 {
            t.PopAxisAlignedClip();
            return None;
        }
        let pad = if l.style == NotchStyle::Unfold { 16.0 * k } else { 8.0 * k };
        let d = (ch - 10.0 * k).clamp(14.0 * k, 34.0 * k);
        let glyph_c = Vector2 { X: if l.style == NotchStyle::Glow { card.left + cw / 2.0 } else { card.left + pad + d / 2.0 }, Y: card.top + ch / 2.0 };
        if cw >= d + 4.0 * k {
            glyph(t, r, fm, l, f, k, glyph_c, d, now);
        }
        let mut button = None;
        let mut right = card.right - pad;
        if l.style.has_button()
            && let Some(label) = l.n.button.as_deref()
        {
            let tw = measure(&r.dw, &fm.button, label);
            let bh = (ch - 12.0 * k).min(28.0 * k);
            let bw = tw + 26.0 * k;
            let br = rect(card.right - pad - bw, card.top + (ch - bh) / 2.0, bw, bh);
            if br.left > glyph_c.X + d {
                if let Some(b) = brush(t, accent, a) {
                    t.FillRoundedRectangle(&rr(br, bh / 2.0, false), &b);
                }
                text(t, label, &fm.button, br, INK, a);
                right = br.left - 10.0 * k;
                button = Some(br);
            }
        }
        if l.style != NotchStyle::Glow {
            let tx = glyph_c.X + d / 2.0 + 10.0 * k;
            let title = if l.n.count > 1 { format!("{} ×{}", l.n.title, l.n.count) } else { l.n.title.clone() };
            if right - tx > 12.0 * k {
                if l.style == NotchStyle::Ticker {
                    // ticker: celý řádek projede zprava doleva během držení
                    let line = if l.n.body.is_empty() { title } else { format!("{title}  ·  {}", l.n.body) };
                    let lw = measure(&r.dw, &fm.title, &line);
                    let x = right - (right - tx + lw) * f.progress;
                    t.PushAxisAlignedClip(&D2D_RECT_F { left: tx, right, ..card }, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                    text(t, &line, &fm.title, rect(x, card.top, lw + 4.0, ch), TEXT, a);
                    t.PopAxisAlignedClip();
                } else if ch >= 44.0 * k {
                    let mid = card.top + ch / 2.0;
                    text(t, &title, &fm.title, D2D_RECT_F { left: tx, top: mid - 19.0 * k, right, bottom: mid }, TEXT, a);
                    text(t, &l.n.body, &fm.body, D2D_RECT_F { left: tx, top: mid + 1.0 * k, right, bottom: mid + 19.0 * k }, DIM, a);
                } else {
                    let tw = measure(&r.dw, &fm.title, &title);
                    text(t, &title, &fm.title, D2D_RECT_F { left: tx, top: card.top, right, bottom: card.bottom }, TEXT, a);
                    let bx = tx + tw + 8.0 * k;
                    if !l.n.body.is_empty() && right - bx > 20.0 * k {
                        text(t, &l.n.body, &fm.body, D2D_RECT_F { left: bx, top: card.top, right, bottom: card.bottom }, DIM, a);
                    }
                }
            }
        }
        t.PopAxisAlignedClip();
        button
    }
}

/// Ikona události v kolečku barvy události: čeká „!“ (dýchá), hotovo = tah fajfky, schůzka = minuty
/// do začátku + prstenec odpočtu, limit „%“, časovač „0:00“.
#[allow(clippy::too_many_arguments)]
unsafe fn glyph(t: &ID2D1DCRenderTarget, r: &Renderer, fm: &Fm, l: &Live, f: &Frame, k: f32, c: Vector2, d: f32, now: u64) {
    unsafe {
        let a = f.alpha;
        let accent = l.n.kind.color();
        let breath = if l.n.kind == Kind::Waiting { 0.92 + 0.08 * ((now as f32 / 900.0) * std::f32::consts::TAU).sin() } else { 1.0 };
        let rad = d / 2.0 * breath;
        if let Some(b) = brush(t, accent, a) {
            t.FillEllipse(&D2D1_ELLIPSE { point: c, radiusX: rad, radiusY: rad }, &b);
        }
        let inner = rect(c.X - rad, c.Y - rad, 2.0 * rad, 2.0 * rad);
        match l.n.kind {
            Kind::Waiting => text(t, "!", &fm.glyph, inner, INK, a),
            Kind::Limit => text(t, "%", &fm.glyph, inner, INK, a),
            Kind::Timer => text(t, "0:00", &fm.body, D2D_RECT_F { left: inner.left - 6.0 * k, right: inner.right + 6.0 * k, ..inner }, INK, a),
            Kind::Done => {
                // fajfka se dotáhne v první pětině držení
                let p = (f.progress * 5.0).clamp(0.0, 1.0);
                let pts = [(-0.26, 0.02), (-0.07, 0.21), (0.27, -0.19)].map(|(x, y)| Vector2 { X: c.X + x * d, Y: c.Y + y * d });
                let l1 = 0.4;
                if let Some(b) = brush(t, INK, a) {
                    let s1 = (p / l1).min(1.0);
                    let lerp = |p0: Vector2, p1: Vector2, s: f32| Vector2 { X: p0.X + (p1.X - p0.X) * s, Y: p0.Y + (p1.Y - p0.Y) * s };
                    if s1 > 0.0 {
                        t.DrawLine(pts[0], lerp(pts[0], pts[1], s1), &b, 2.4 * k, None);
                    }
                    let s2 = ((p - l1) / (1.0 - l1)).clamp(0.0, 1.0);
                    if s2 > 0.0 {
                        t.DrawLine(pts[1], lerp(pts[1], pts[2], s2), &b, 2.4 * k, None);
                    }
                }
            }
            Kind::Meeting => {
                let left_ms = l.n.start_ms.map(|s| s - crate::state::now_ms()).unwrap_or(0).max(0);
                let mins = (left_ms + 59_999) / 60_000;
                text(t, &mins.to_string(), &fm.glyph, inner, INK, a);
                // prstenec: zbývající čas z 15 min, ubývá po směru hodin
                let frac = (left_ms as f32 / 900_000.0).clamp(0.0, 1.0);
                if let Some(b) = brush(t, accent, 0.25 * a) {
                    t.DrawEllipse(&D2D1_ELLIPSE { point: c, radiusX: rad + 3.5 * k, radiusY: rad + 3.5 * k }, &b, 1.5 * k, None);
                }
                if let Some(b) = brush(t, accent, a) {
                    arc(t, r, c, rad + 3.5 * k, 0.0, frac, &b, 2.0 * k);
                }
            }
        }
    }
}

/// Tečka zmeškaného oznámení vpravo dole na těle proužku (lokální px těla `bw × bh`).
pub(crate) unsafe fn missed_dot(t: &ID2D1DCRenderTarget, c: Option<Rgba>, bw: f32, bh: f32, k: f32) {
    unsafe {
        let Some(c) = c else { return };
        if bw < 8.0 * k || bh < 2.0 {
            return;
        }
        let rad = (2.5 * k).min(bh / 3.0);
        let p = Vector2 { X: bw - rad - 3.0 * k, Y: (bh - rad - 2.0 * k).max(rad) };
        if let Some(b) = brush(t, c, 1.0) {
            t.FillEllipse(&D2D1_ELLIPSE { point: p, radiusX: rad, radiusY: rad }, &b);
        }
    }
}
