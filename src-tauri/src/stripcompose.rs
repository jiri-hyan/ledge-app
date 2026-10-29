//! Kreslení výzdoby vlastního zavřeného Ledge (kolo 25, `strip.compose.deco` / `fill` / `sep`) pro `stripdraw`:
//! pozadí těla (černá, sklo, tón akcentu, přechod, nic + stín textu), díly zón s oddělovači (mezera, tečka,
//! čárka, kapsle, tři ostrůvky), barevný podklad dílu, stálý akcent hrany (linka, obrys, záře, přechod) a nové
//! části `Spark` / `Under`. Rozložení je z čistých funkcí `looks::composedeco` (`joint`, `fit`, `content_w`).
//! Kreslí se jen při změně modelu jako zbytek proužku — žádný časovač, štětce jen na snímek.

use crate::looks::composedeco::{self, Edge, FillDir, FillKind, Paint, Sep};
use crate::looks::frame::Spec;
use crate::looks::{Part, Seg};
use crate::strip::{Renderer, Rgba, color, measure, wide};
use crate::stripdraw::{Fmts, brush, draw_parts, row_w};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_GRADIENT_STOP};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_EXTEND_MODE_CLAMP, D2D1_GAMMA_2_2,
    D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES, D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES, D2D1_ROUNDED_RECT, ID2D1Brush, ID2D1DCRenderTarget,
};
use windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL;
use windows::core::Interface;
use windows_numerics::Vector2;

/// Šířka mini grafu (logické px).
pub(crate) const SPARK_W: f32 = 30.0;
/// Dosah záře pod tělem (logické px) — o tolik je okno vyšší jen s `edge: glow`.
pub(crate) const GLOW_PX: f32 = 8.0;

/// Díl zóny: jeho části, barevný podklad a šířka i s vnitřním okrajem kapsle.
pub(crate) struct Chunk<'a> {
    pub parts: &'a [Part],
    pub chip: Option<Rgba>,
    pub w: f32,
}

pub(crate) struct Layout<'a> {
    pub zones: [Vec<Chunk<'a>>; 3],
    pub zw: [f32; 3],
    /// šířka obsahu pro `frame::body_width` (u ostrůvků bez vnějších okrajů těla)
    pub content: f32,
    pub paint: Paint,
    joint: f32,
    /// ostrůvky: (oba okraje zóny, mezera mezi ostrůvky) v px
    islands: Option<(f32, f32)>,
}

/// Části zóny rozdělené na díly podle `Seg` (nesedí-li počty, např. přihrádka, je celá zóna jeden díl).
fn split<'a>(parts: &'a [Part], segs: &[Seg]) -> Vec<(&'a [Part], Option<Rgba>)> {
    if parts.is_empty() {
        return Vec::new();
    }
    if segs.iter().map(|s| s.n).sum::<usize>() != parts.len() || segs.iter().any(|s| s.n == 0) {
        return vec![(parts, None)];
    }
    let mut at = 0;
    segs.iter()
        .map(|s| {
            let x = (&parts[at..at + s.n], s.chip);
            at += s.n;
            x
        })
        .collect()
}

/// Rozložení zón: šířky dílů, co se vejde do `avail` (px obsahu; schovává se od konce zóny), šířka obsahu.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn layout<'a>(r: &Renderer, fm: &Fmts, sp: &Spec, ks: f32, paint: Paint, zones: [(&'a [Part], &[Seg]); 3], avail: f32) -> Layout<'a> {
    unsafe {
        let gap = sp.gap * ks;
        let joint = composedeco::joint(paint.sep, gap, ks);
        let cp = composedeco::chip_pad(gap);
        let islands = (paint.sep == Sep::Islands).then(|| (2.0 * sp.pad * ks, (sp.h * 0.4 * ks).max(6.0 * ks)));
        let mut out: [Vec<Chunk<'a>>; 3] = Default::default();
        for (i, (parts, segs)) in zones.into_iter().enumerate() {
            out[i] = split(parts, segs)
                .into_iter()
                .map(|(p, chip)| {
                    let pad = if chip.is_some() || paint.sep == Sep::Pills { 2.0 * cp } else { 0.0 };
                    Chunk { parts: p, chip, w: row_w(r, fm, p, sp, ks) + pad }
                })
                .collect();
        }
        let ws: [Vec<f32>; 3] = [0, 1, 2].map(|i| out[i].iter().map(|c| c.w).collect());
        let keep = composedeco::fit([&ws[0], &ws[1], &ws[2]], joint, gap, islands, avail);
        for (z, k) in out.iter_mut().zip(keep) {
            z.truncate(k);
        }
        let zw = [0, 1, 2].map(|i| composedeco::zone_w(&ws[i][..keep[i]], joint));
        let full = composedeco::content_w(zw, gap, islands);
        // u ostrůvků jsou okraje zón už v `full`; `body_width` přičte okraje těla, ty se tu odečtou
        let content = if islands.is_some() { (full - 2.0 * sp.pad * ks).max(0.0) } else { full };
        Layout { zones: out, zw, content, paint, joint, islands }
    }
}

fn as_brush<T: Interface>(b: Option<T>) -> Option<ID2D1Brush> {
    b.and_then(|b| b.cast().ok())
}

/// Lineární přechod mezi body (`a` = násobek průhlednosti).
unsafe fn linear(t: &ID2D1DCRenderTarget, c1: Rgba, c2: Rgba, a: f32, from: Vector2, to: Vector2) -> Option<ID2D1Brush> {
    unsafe {
        let stops = [D2D1_GRADIENT_STOP { position: 0.0, color: color(c1, a) }, D2D1_GRADIENT_STOP { position: 1.0, color: color(c2, a) }];
        let coll = t.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP).ok()?;
        as_brush(t.CreateLinearGradientBrush(&D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: from, endPoint: to }, None, &coll).ok())
    }
}

/// Měkká záře: elipsa od barvy (`a`) do průhledna.
unsafe fn radial(t: &ID2D1DCRenderTarget, c: Rgba, a: f32, center: Vector2, rx: f32, ry: f32) {
    unsafe {
        let stops = [D2D1_GRADIENT_STOP { position: 0.0, color: color(c, a) }, D2D1_GRADIENT_STOP { position: 1.0, color: color(c, 0.0) }];
        let Ok(coll) = t.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP) else { return };
        let props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES { center, gradientOriginOffset: Vector2 { X: 0.0, Y: 0.0 }, radiusX: rx, radiusY: ry };
        if let Ok(b) = t.CreateRadialGradientBrush(&props, None, &coll) {
            t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: rx, radiusY: ry }, &b);
        }
    }
}

/// Těla k vykreslení: jedno, u ostrůvků jedno na neprázdnou zónu (stejný tvar, jen užší).
fn bodies(lay: &Layout, body: D2D1_ROUNDED_RECT, bwf: f32, pad: f32) -> Vec<(D2D1_ROUNDED_RECT, f32)> {
    let Some((pad2, _)) = lay.islands else { return vec![(body, pad)] };
    let mut out = Vec::new();
    for (i, w) in lay.zw.iter().enumerate().filter(|(_, w)| **w > 0.0) {
        let iw = w + pad2;
        let left = match i {
            0 => 0.0,
            1 => (bwf - iw) / 2.0,
            _ => bwf - iw,
        };
        let rect = D2D_RECT_F { left: left + body.rect.left.min(0.5), right: left + iw - (bwf - body.rect.right).min(0.5), ..body.rect };
        out.push((D2D1_ROUNDED_RECT { rect, ..body }, left + pad2 / 2.0));
    }
    out
}

/// Kde začíná obsah zóny `i` (bez ostrůvků jako před kolem 25: vlevo od okraje, střed na ose, vpravo k okraji).
fn zone_x(lay: &Layout, i: usize, bwf: f32, pad: f32, isl: &[(D2D1_ROUNDED_RECT, f32)]) -> f32 {
    if lay.islands.is_some() {
        let k = lay.zw[..i].iter().filter(|w| **w > 0.0).count();
        return isl.get(k).map(|b| b.1).unwrap_or(pad);
    }
    match i {
        0 => pad,
        1 => (bwf - lay.zw[1]) / 2.0,
        _ => bwf - pad - lay.zw[2],
    }
}

/// Stín textu bez těla: stejné části černě (baterie bez stínu).
fn shadow(parts: &[Part]) -> Vec<Part> {
    crate::looks::compose::recolor(parts.iter().filter(|p| !matches!(p, Part::Battery { .. })).cloned().collect(), Rgba(0.0, 0.0, 0.0, 1.0))
}

/// Tělo, obsah a akcent hrany vlastního vzhledu. Vrací obdélníky těl (puls a nažhavení se kreslí nad ně).
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn paint(t: &ID2D1DCRenderTarget, r: &Renderer, fm: &Fmts, sp: &Spec, ks: f32, scale: f32, lay: &Layout, body: D2D1_ROUNDED_RECT, bwf: f32, bhf: f32, spin: Option<f32>) -> Vec<D2D1_ROUNDED_RECT> {
    unsafe {
        let p = lay.paint;
        let pad = sp.pad * ks;
        let isl = bodies(lay, body, bwf, pad);
        let lw = p.width * scale;
        // záře pod tělem (okno je o `GLOW_PX` vyšší, záře v něm dobíhá do nuly), kreslí se první, tělo ji nahoře zakryje
        if p.edge == Edge::Glow {
            for (b, _) in &isl {
                let w = b.rect.right - b.rect.left;
                radial(t, p.c1, 0.55, Vector2 { X: b.rect.left + w / 2.0, Y: bhf }, w * 0.55, GLOW_PX * scale);
            }
        }
        // pozadí
        let solid = |c: Rgba| as_brush(brush(t, c, 1.0));
        let fill: Option<ID2D1Brush> = match p.fill {
            FillKind::None => None,
            FillKind::Gradient => {
                let (a, b) = match p.dir {
                    FillDir::Right => (Vector2 { X: 0.0, Y: 0.0 }, Vector2 { X: bwf, Y: 0.0 }),
                    FillDir::Down => (Vector2 { X: 0.0, Y: 0.0 }, Vector2 { X: 0.0, Y: bhf }),
                    FillDir::Diag => (Vector2 { X: 0.0, Y: 0.0 }, Vector2 { X: bwf, Y: bhf }),
                };
                linear(t, p.f1, p.f2, 1.0, a, b)
            }
            _ => solid(p.f1),
        };
        if let Some(b) = &fill {
            for (x, _) in &isl {
                t.FillRoundedRectangle(x, b);
            }
            if sp.ears > 0.0 && lay.islands.is_none() {
                crate::stripdraw::ears(t, r, bwf, sp.ears * ks, p.f1);
            }
            // sklo: jemná světlá hrana, ať je vidět i na tmavé ploše
            if p.fill == FillKind::Glass
                && let Some(e) = brush(t, Rgba(1.0, 1.0, 1.0, 1.0), 0.12)
            {
                for (x, _) in &isl {
                    t.DrawRoundedRectangle(x, &e, 1.0, None);
                }
            }
        }
        if let Some(e) = sp.edge
            && let Some(b) = brush(t, e, 1.0)
        {
            for (x, _) in &isl {
                t.DrawRoundedRectangle(x, &b, 1.0, None);
            }
        }
        // obsah oříznutý na tělo
        t.PushAxisAlignedClip(&D2D_RECT_F { left: 0.0, top: 0.0, right: bwf, bottom: bhf }, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        let mid = bhf / 2.0;
        let cp = composedeco::chip_pad(sp.gap * ks);
        for (i, zone) in lay.zones.iter().enumerate() {
            let mut x = zone_x(lay, i, bwf, pad, &isl);
            for (k, ch) in zone.iter().enumerate() {
                if k > 0 {
                    sep(t, p.sep, x - lay.joint / 2.0, mid, bhf, ks);
                }
                let boxed = ch.chip.is_some() || p.sep == Sep::Pills;
                if boxed {
                    let hh = (bhf * 0.36).max(6.0 * ks).min(mid);
                    let (tone, a) = match ch.chip {
                        Some(c) => (c, 0.24),
                        None => (Rgba(1.0, 1.0, 1.0, 1.0), 0.09),
                    };
                    if let Some(b) = brush(t, tone, a) {
                        let rr = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x, top: mid - hh, right: x + ch.w, bottom: mid + hh }, radiusX: hh, radiusY: hh };
                        t.FillRoundedRectangle(&rr, &b);
                    }
                }
                let ix = x + if boxed { cp } else { 0.0 };
                if p.fill == FillKind::None {
                    draw_parts(t, r, fm, &shadow(ch.parts), sp, ks, ix + ks, ks, bhf + ks, None);
                }
                draw_parts(t, r, fm, ch.parts, sp, ks, ix, 0.0, bhf, spin);
                x += ch.w + lay.joint;
            }
        }
        t.PopAxisAlignedClip();
        // stálý akcent hrany
        for (b, _) in &isl {
            edge(t, &p, b, lw, scale);
        }
        isl.into_iter().map(|x| x.0).collect()
    }
}

/// Oddělovač mezi díly (střed mezery `cx`).
unsafe fn sep(t: &ID2D1DCRenderTarget, s: Sep, cx: f32, mid: f32, bhf: f32, ks: f32) {
    unsafe {
        match s {
            Sep::Dot => {
                if let Some(b) = brush(t, Rgba::TEXT, 0.45) {
                    t.FillEllipse(&D2D1_ELLIPSE { point: Vector2 { X: cx, Y: mid }, radiusX: 1.5 * ks, radiusY: 1.5 * ks }, &b);
                }
            }
            Sep::Bar => {
                if let Some(b) = brush(t, Rgba(1.0, 1.0, 1.0, 1.0), 0.24) {
                    let hh = bhf * 0.22;
                    t.FillRectangle(&D2D_RECT_F { left: cx - 0.5 * ks, top: mid - hh, right: cx + 0.5 * ks, bottom: mid + hh }, &b);
                }
            }
            _ => {}
        }
    }
}

/// Spodní linka / obrys / přechod podle `Paint.edge` na jednom těle.
unsafe fn edge(t: &ID2D1DCRenderTarget, p: &Paint, b: &D2D1_ROUNDED_RECT, lw: f32, scale: f32) {
    unsafe {
        let (l, rt, bot) = (b.rect.left, b.rect.right, b.rect.bottom);
        let w = rt - l;
        // celá šířka = mezi zaobleními rohů, střed = 40 % uprostřed
        let inset = b.radiusX.min(w / 2.0) * 0.8;
        let (x0, x1) = if p.full { (l + inset, rt - inset) } else { (l + w * 0.3, l + w * 0.7) };
        let line = D2D_RECT_F { left: x0, top: bot - lw, right: x1, bottom: bot };
        match p.edge {
            Edge::None => {}
            Edge::Underline => {
                if let Some(br) = brush(t, p.c1, 0.95) {
                    t.FillRectangle(&line, &br);
                }
            }
            Edge::Gradient => {
                if let Some(br) = linear(t, p.c1, p.c2, 1.0, Vector2 { X: x0, Y: 0.0 }, Vector2 { X: x1, Y: 0.0 }) {
                    t.FillRectangle(&line, &br);
                }
            }
            Edge::Outline => {
                if let Some(br) = brush(t, p.c1, 0.9) {
                    let inner = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: l + lw / 2.0, right: rt - lw / 2.0, bottom: bot - lw / 2.0, ..b.rect }, ..*b };
                    t.DrawRoundedRectangle(&inner, &br, lw, None);
                }
            }
            Edge::Glow => {
                // jasné jádro záře: krátká linka ve středu
                if let Some(br) = brush(t, p.c1, 1.0) {
                    let h = lw.min(2.0 * scale);
                    t.FillRectangle(&D2D_RECT_F { top: bot - h, ..line }, &br);
                }
            }
        }
    }
}

/// Šířka nových částí (`Spark`, `Under`).
pub(crate) unsafe fn extra_w(r: &Renderer, fm: &Fmts, p: &Part, ks: f32) -> f32 {
    unsafe {
        match p {
            Part::Spark { .. } => SPARK_W * ks,
            Part::Under { text, .. } => measure(&r.dw, &fm.text, text).max(18.0 * ks),
            _ => 0.0,
        }
    }
}

/// Kreslí `Spark` (lomená čára z historie, svislá osa 0–100 %) a `Under` (text, pod ním tenký pruh podílu).
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn draw_extra(t: &ID2D1DCRenderTarget, r: &Renderer, fm: &Fmts, p: &Part, sp: &Spec, ks: f32, x: f32, y0: f32, y1: f32, w: f32) {
    unsafe {
        let mid = (y0 + y1) / 2.0;
        match p {
            Part::Spark { pts, color: c } if pts.len() >= 2 => {
                let h = (sp.font * ks * 1.1).min(y1 - y0);
                let (top, n) = (mid - h / 2.0, (pts.len() - 1) as f32);
                let at = |i: usize, v: f32| Vector2 { X: x + w * i as f32 / n, Y: top + h * (1.0 - v) };
                let Ok(path) = r.d2d.CreatePathGeometry() else { return };
                let Ok(sink) = path.Open() else { return };
                sink.BeginFigure(at(0, pts[0]), windows::Win32::Graphics::Direct2D::Common::D2D1_FIGURE_BEGIN_HOLLOW);
                for (i, v) in pts.iter().enumerate().skip(1) {
                    sink.AddLine(at(i, *v));
                }
                sink.EndFigure(windows::Win32::Graphics::Direct2D::Common::D2D1_FIGURE_END_OPEN);
                if sink.Close().is_ok()
                    && let Some(b) = brush(t, *c, 1.0)
                {
                    t.DrawGeometry(&path, &b, 1.4 * ks, None);
                    let last = at(pts.len() - 1, pts[pts.len() - 1]);
                    t.FillEllipse(&D2D1_ELLIPSE { point: last, radiusX: 1.8 * ks, radiusY: 1.8 * ks }, &b);
                }
            }
            Part::Under { text, color: c, fill } => {
                let th = 2.0 * ks;
                let bar_y = y1 - (y1 - y0) * 0.2;
                if let Some(b) = brush(t, *c, 1.0) {
                    let lr = D2D_RECT_F { left: x, top: y0, right: x + w + 2.0, bottom: bar_y - th };
                    t.DrawText(&wide(text), &fm.text, &lr, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                }
                let track = D2D_RECT_F { left: x, top: bar_y - th, right: x + w, bottom: bar_y };
                if let Some(b) = brush(t, *c, 0.25) {
                    t.FillRectangle(&track, &b);
                }
                if let Some(b) = brush(t, *c, 1.0) {
                    t.FillRectangle(&D2D_RECT_F { right: x + w * fill.clamp(0.0, 1.0), ..track }, &b);
                }
            }
            _ => {}
        }
    }
}
