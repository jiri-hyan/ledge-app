//! Kreslení živé hrany vlastního zavřeného Ledge (kolo 28, logika v `looks::edgefx`): svit pod tělem (záře,
//! rozsvícení, halo přelévání / dechu / komety / naplnění), linka efektu (přelévání, čárky, kometa, dech,
//! naplnění, obrys přechodem) a přechod pozadí se třemi barvami / přeléváním. `prepare` před každým snímkem
//! vybere hranu podle stavu Clauda nebo „Vyzkoušet“ (`edge_try`) a řekne vláknu proužku, jestli má běžet
//! časovač `FX_TIMER` (25 fps) — jen během animace; v klidu žádný. Štětce jen na snímek.

use crate::looks::composedeco::{self, Deco, Edge, FillDir, Paint};
use crate::looks::edgefx::{self, LiveState};
use crate::strip::{Rgba, StripModel, color};
use crate::stripcompose::{as_brush, radial};
use crate::stripdraw::brush;
use std::time::Instant;
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_GRADIENT_STOP};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ELLIPSE, D2D1_EXTEND_MODE, D2D1_EXTEND_MODE_CLAMP, D2D1_EXTEND_MODE_MIRROR, D2D1_EXTEND_MODE_WRAP, D2D1_GAMMA_2_2,
    D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES, D2D1_ROUNDED_RECT, ID2D1Brush, ID2D1DCRenderTarget,
};
use windows_numerics::Vector2;

/// „Vyzkoušet“ z editoru: stav Clauda, rozpracovaná výzdoba (jinak uložená) a začátek.
#[derive(Debug, Clone, PartialEq)]
pub struct Try {
    pub state: LiveState,
    pub deco: Option<Deco>,
    pub at: Instant,
}

/// Hodiny jednorázového naplnění (vlákno proužku): začátek a ke kterému „Vyzkoušet“ patří.
#[derive(Default)]
pub(crate) struct Clock {
    t0: Option<u64>,
    tried: Option<Instant>,
}

/// Před kreslením: hrana podle stavu / „Vyzkoušet“, čas snímku a průběh naplnění do `model.show.paint`
/// (kopie modelu vlákna). Vrací, jestli má běžet snímkový časovač (`edgefx::frames`).
pub(crate) fn prepare(m: &mut StripModel, ck: &mut Clock, now: u64) -> bool {
    let tr = m.fx_try.take().filter(|t| t.at.elapsed().as_millis() < edgefx::TRY_MS as u128);
    let animate = m.animate && !m.fx_calm;
    let Some(p) = m.show.as_mut().and_then(|s| s.paint.as_mut()) else {
        ck.t0 = None;
        return false;
    };
    let mut base = *p;
    if let Some(Try { state, deco: Some(d), .. }) = &tr {
        composedeco::restyle(&mut base, d, edgefx::state_color(*state));
    }
    let mut q = edgefx::apply(&base, tr.as_ref().map(|t| t.state));
    let tried = tr.as_ref().map(|t| t.at);
    if ck.tried != tried {
        (ck.tried, ck.t0) = (tried, None);
    }
    let sweeping = if q.edge == Edge::Sweep && animate {
        let s = edgefx::sweep(now.saturating_sub(*ck.t0.get_or_insert(now)));
        q.fx.sweep = s;
        s < 1.0
    } else {
        if q.edge != Edge::Sweep {
            ck.t0 = None;
        }
        q.fx.sweep = 1.0;
        false
    };
    let run = edgefx::frames(&q, tr.is_some(), sweeping, animate);
    q.fx.ms = (run && animate).then_some(now);
    *p = q;
    run
}

/// IPC z editoru vzhledu: stav `state` (`work` / `wait` / `done` / `limit` / `idle`) na `edgefx::TRY_MS`
/// v proužku, volitelně s rozpracovanou výzdobou `deco`. Jen u vlastního vzhledu; ve hře nic.
#[tauri::command]
pub(crate) fn edge_try(state: String, deco: Option<Deco>) -> Result<(), String> {
    use crate::i18n::tr;
    let st = LiveState::from_id(&state).ok_or_else(|| tr("Neznámý stav.", "Unknown state.").to_string())?;
    let c = crate::state::core();
    if c.dead() {
        return Err(tr("Ledge je teď ztlumený (hra přes celou obrazovku nebo herní režim).", "Ledge is muted right now (fullscreen game or game mode).").into());
    }
    c.strip.set(|m| m.fx_try = Some(Try { state: st, deco, at: Instant::now() }));
    Ok(())
}

/// Lineární přechod přes body `stops` (pozice, barva, krytí) v režimu opakování `mode`.
unsafe fn grad(t: &ID2D1DCRenderTarget, stops: &[(f32, Rgba, f32)], from: Vector2, to: Vector2, mode: D2D1_EXTEND_MODE) -> Option<ID2D1Brush> {
    unsafe {
        let st: Vec<D2D1_GRADIENT_STOP> = stops.iter().map(|(p, c, a)| D2D1_GRADIENT_STOP { position: *p, color: color(*c, *a) }).collect();
        let coll = t.CreateGradientStopCollection(&st, D2D1_GAMMA_2_2, mode).ok()?;
        as_brush(t.CreateLinearGradientBrush(&D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: from, endPoint: to }, None, &coll).ok())
    }
}

/// Barvy hrany po sobě (2 nebo 3).
fn trio(p: &Paint) -> Vec<Rgba> {
    match p.c3 {
        Some(c3) => vec![p.c1, p.c2, c3],
        None => vec![p.c1, p.c2],
    }
}

/// Body přechodu rovnoměrně; `cycle` = zpět na první (bez švu při opakování).
fn even(cs: &[Rgba], a: f32, cycle: bool) -> Vec<(f32, Rgba, f32)> {
    let mut v: Vec<Rgba> = cs.to_vec();
    if cycle {
        v.push(cs[0]);
    }
    let n = (v.len() - 1).max(1) as f32;
    v.into_iter().enumerate().map(|(i, c)| (i as f32 / n, c, a)).collect()
}

/// Svit pod tělem (kreslí se první, tělo ho nahoře zakryje). Okno má rezervu `edgefx::reserve`.
pub(crate) unsafe fn under(t: &ID2D1DCRenderTarget, p: &Paint, rects: &[D2D1_ROUNDED_RECT], bhf: f32, scale: f32) {
    unsafe {
        let pk = edgefx::power_k(p.fx.power);
        let ry = edgefx::halo_px(p.edge, p.fx.power) * scale;
        if ry <= 0.0 {
            return;
        }
        for b in rects {
            let (l, r) = (b.rect.left, b.rect.right);
            let w = r - l;
            let (x0, x1) = edgefx::span_x(p.span, l, r, b.radiusX);
            let (cx, lw) = ((x0 + x1) / 2.0, x1 - x0);
            let at = |x: f32| Vector2 { X: x, Y: bhf };
            match p.edge {
                // dnešní záře (síla 2 = krytí 0,55)
                Edge::Glow => radial(t, p.c1, 0.55 * pk, at(l + w / 2.0), w * 0.55, ry),
                Edge::Light => {
                    radial(t, p.c1, (0.5 * pk).min(0.8), at(l + w / 2.0), w * 0.55, ry);
                    radial(t, p.c2, 0.22 * pk, at(l + w / 2.0), w * 0.3, ry * 0.55);
                }
                Edge::Breath => radial(t, p.c1, 0.6 * pk * edgefx::breath(p.fx.ms, p.fx.speed), at(cx), (lw * 0.6).max(w * 0.25), ry),
                Edge::Flow => radial(t, p.c2, 0.3 * pk, at(cx), lw * 0.55, ry),
                Edge::Comet => {
                    let (pos, _) = edgefx::comet(p.fx.ms, p.fx.speed);
                    radial(t, p.c1, 0.7 * pk, at(x0 + pos * lw), 16.0 * scale, ry);
                }
                Edge::Sweep => {
                    let hx = x0 + p.fx.sweep * lw;
                    radial(t, p.c1, 0.3 * pk, at((x0 + hx) / 2.0), ((hx - x0) * 0.55).max(1.0), ry);
                    if p.fx.sweep < 1.0 {
                        radial(t, p.c1, 0.75 * pk, at(hx), 18.0 * scale, ry);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Linka efektu kola 28 a přechod se třemi barvami na jednom těle; `false` = starý akcent (`stripcompose::edge`).
pub(crate) unsafe fn line(t: &ID2D1DCRenderTarget, p: &Paint, b: &D2D1_ROUNDED_RECT, lw: f32, scale: f32) -> bool {
    unsafe {
        let (l, rt, bot) = (b.rect.left, b.rect.right, b.rect.bottom);
        let (x0, x1) = edgefx::span_x(p.span, l, rt, b.radiusX);
        let len = (x1 - x0).max(1.0);
        let rect = |a: f32, z: f32| D2D_RECT_F { left: a, top: bot - lw, right: z, bottom: bot };
        let h = |x: f32| Vector2 { X: x, Y: 0.0 };
        match p.edge {
            Edge::Gradient if p.c3.is_some() => {
                if let Some(br) = grad(t, &even(&trio(p), 1.0, false), h(x0), h(x1), D2D1_EXTEND_MODE_CLAMP) {
                    t.FillRectangle(&rect(x0, x1), &br);
                }
            }
            Edge::Flow => {
                let from = x0 + edgefx::flow_phase(p.fx.ms, p.fx.speed) * len;
                if let Some(br) = grad(t, &even(&trio(p), 1.0, true), h(from), h(from + len), D2D1_EXTEND_MODE_WRAP) {
                    t.FillRectangle(&rect(x0, x1), &br);
                }
            }
            Edge::Dashes => {
                let n = edgefx::dash_n(len, scale);
                let frac = p.fx.frac.filter(|_| p.fx.state == LiveState::Limit);
                let alpha = edgefx::dashes(n, frac, p.fx.ms.map(|_| edgefx::flow_phase(p.fx.ms, p.fx.speed)));
                if let Some(br) = grad(t, &even(&trio(p), 1.0, false), h(x0), h(x1), D2D1_EXTEND_MODE_CLAMP) {
                    let seg = len / n as f32;
                    let dash = (seg * 0.62).max(1.0);
                    for (i, a) in alpha.into_iter().enumerate() {
                        let x = x0 + i as f32 * seg + (seg - dash) / 2.0;
                        br.SetOpacity(a);
                        t.FillRectangle(&rect(x, x + dash), &br);
                    }
                }
            }
            Edge::Comet => {
                let (pos, dir) = edgefx::comet(p.fx.ms, p.fx.speed);
                if let Some(br) = brush(t, p.c1, 0.15) {
                    t.FillRectangle(&rect(x0, x1), &br);
                }
                let hx = x0 + pos * len;
                let tail = hx - dir * len * 0.3;
                if let Some(br) = grad(t, &[(0.0, p.c1, 0.0), (1.0, p.c1, 1.0)], h(tail), h(hx), D2D1_EXTEND_MODE_CLAMP) {
                    t.FillRectangle(&rect(tail.min(hx).max(x0), tail.max(hx).min(x1)), &br);
                }
                if let Some(br) = brush(t, p.c2, 1.0) {
                    let rr = lw.max(1.5 * scale);
                    t.FillEllipse(&D2D1_ELLIPSE { point: Vector2 { X: hx, Y: bot - lw / 2.0 }, radiusX: rr * 1.6, radiusY: rr }, &br);
                }
            }
            Edge::Light => {
                let (a, z) = edgefx::span_x(None, l, rt, b.radiusX);
                if let Some(br) = grad(t, &even(&trio(p), (0.7 * edgefx::power_k(p.fx.power)).min(1.0), false), h(a), h(z), D2D1_EXTEND_MODE_CLAMP) {
                    t.FillRectangle(&rect(a, z), &br);
                }
            }
            Edge::Breath => {
                if let Some(br) = grad(t, &even(&trio(p), edgefx::breath(p.fx.ms, p.fx.speed), false), h(x0), h(x1), D2D1_EXTEND_MODE_CLAMP) {
                    t.FillRectangle(&rect(x0, x1), &br);
                }
            }
            Edge::Sweep => {
                let hx = x0 + p.fx.sweep * len;
                if let Some(br) = brush(t, p.c1, 0.12) {
                    t.FillRectangle(&rect(x0, x1), &br);
                }
                if let Some(br) = brush(t, p.c1, 1.0) {
                    t.FillRectangle(&rect(x0, hx), &br);
                }
            }
            Edge::OutlineGradient => {
                if let Some(br) = grad(t, &even(&trio(p), 0.95, false), h(l), h(rt), D2D1_EXTEND_MODE_CLAMP) {
                    let inner = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: l + lw / 2.0, right: rt - lw / 2.0, bottom: bot - lw / 2.0, ..b.rect }, ..*b };
                    t.DrawRoundedRectangle(&inner, &br, lw, None);
                }
            }
            _ => return false,
        }
        true
    }
}

/// Přechod pozadí se třemi barvami / přeléváním (`None` = obyčejný přechod dvou barev jako dřív).
pub(crate) unsafe fn fill(t: &ID2D1DCRenderTarget, p: &Paint, bwf: f32, bhf: f32) -> Option<ID2D1Brush> {
    unsafe {
        if p.f3.is_none() && !p.fill_flow {
            return None;
        }
        let (a, b) = match p.dir {
            FillDir::Right => (Vector2 { X: 0.0, Y: 0.0 }, Vector2 { X: bwf, Y: 0.0 }),
            FillDir::Down => (Vector2 { X: 0.0, Y: 0.0 }, Vector2 { X: 0.0, Y: bhf }),
            FillDir::Diag => (Vector2 { X: 0.0, Y: 0.0 }, Vector2 { X: bwf, Y: bhf }),
        };
        let cs: Vec<Rgba> = [Some(p.f1), p.f3, Some(p.f2)].into_iter().flatten().collect();
        if !p.fill_flow {
            return grad(t, &even(&cs, 1.0, false), a, b, D2D1_EXTEND_MODE_CLAMP);
        }
        // pomalé přelévání: zrcadlené opakování posunuté o fázi (třikrát pomaleji než hrana)
        let ph = edgefx::flow_phase(p.fx.ms.map(|m| m / 3), p.fx.speed) * 2.0;
        let (dx, dy) = (b.X - a.X, b.Y - a.Y);
        let s = Vector2 { X: a.X - ph * dx, Y: a.Y - ph * dy };
        grad(t, &even(&cs, 1.0, false), s, Vector2 { X: s.X + dx, Y: s.Y + dy }, D2D1_EXTEND_MODE_MIRROR)
    }
}

#[cfg(test)]
mod bench {
    //! Odhad ceny snímku živé hrany bez okna: offscreen DIB jako `stripdraw::draw` (nový DIB na snímek, tělo,
    //! 3 texty, svit + linka efektu). Spustit ručně: `cargo test --lib stripfx::bench -- --ignored --nocapture`.
    use super::*;
    use crate::looks::Facts;
    use crate::looks::composedeco::{Fill, Sep, paint};
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT};
    use windows::Win32::Graphics::Direct2D::{
        D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1CreateFactory,
        ID2D1Factory,
    };
    use windows::Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, DWRITE_MEASURING_MODE_NATURAL, DWriteCreateFactory, IDWriteFactory};
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
    use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ, SelectObject};

    #[test]
    #[ignore]
    fn frame_cost() {
        unsafe {
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            };
            let t = d2d.CreateDCRenderTarget(&props).unwrap();
            let fmt = crate::strip::text_format(&dw, 13.0, false, "Segoe UI").unwrap();
            let (w, h, bh) = (420, 48, 32.0);
            let body = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: 7.0, top: -12.0, right: 413.0, bottom: bh }, radiusX: 11.0, radiusY: 11.0 };
            let f = Facts { working: 1, accent: Rgba::VIOLET, ..Facts::default() };
            for edge in [Edge::Glow, Edge::Flow, Edge::Breath, Edge::Comet, Edge::Dashes, Edge::Light] {
                let d = Deco { edge, color: "aurora".into(), len: crate::looks::composedeco::EdgeLen::Wide, ..Deco::default() };
                let mut p = paint(&d, &Fill::default(), Sep::None, &f);
                let n = 300u64;
                let t0 = Instant::now();
                for i in 0..n {
                    p.fx.ms = Some(i * 33);
                    let mem = CreateCompatibleDC(None);
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
                        ..Default::default()
                    };
                    let mut bits = std::ptr::null_mut();
                    let dib = CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
                    let old = SelectObject(mem, HGDIOBJ(dib.0));
                    t.BindDC(mem, &RECT { left: 0, top: 0, right: w, bottom: h }).unwrap();
                    t.BeginDraw();
                    t.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
                    under(&t, &p, &[body], bh, 1.0);
                    if let Some(b) = brush(&t, Rgba(0.0, 0.0, 0.0, 1.0), 1.0) {
                        t.FillRoundedRectangle(&body, &b);
                    }
                    if let Some(b) = brush(&t, Rgba::TEXT, 1.0) {
                        for (k, s) in ["Claude pracuje · 2", "14:32", "5h 42 %"].iter().enumerate() {
                            let x = 20.0 + k as f32 * 130.0;
                            let r = D2D_RECT_F { left: x, top: 0.0, right: x + 125.0, bottom: bh };
                            t.DrawText(&crate::strip::wide(s), &fmt, &r, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                        }
                    }
                    line(&t, &p, &body, 2.0, 1.0);
                    let _ = t.EndDraw(None, None);
                    SelectObject(mem, old);
                    let _ = DeleteObject(HGDIOBJ(dib.0));
                    let _ = DeleteDC(mem);
                }
                let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
                // CPU při 30 fps = ms snímku × 30 / 1000 s (jedno jádro); bez UpdateLayeredWindow (kompozice DWM)
                println!("{edge:?}: {ms:.3} ms/snímek ≈ {:.2} % jednoho jádra při 30 fps", ms * 30.0 / 10.0);
            }
        }
    }
}
