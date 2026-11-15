//! Klasický proužek (`strip.look = classic`, styl `pill`/`line`/`dot` a tvary stylů vlny 6): tělo, tečky stavů,
//! segmenty s textem a podpisová linka. Kreslí ho vlákno `strip` (`strip.rs`) do vrstveného okna; styl `dot`
//! (tečky stavů) je i vzhled proužku na vedlejších monitorech (`stripset.rs`).

use crate::appearance::StripStyle;
use crate::appearance::geom::{self, ink};
use crate::strip::{Renderer, Rgba, Segment, StripModel, color, measure, pulse_now, text_format, wide};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F, D2D1_GRADIENT_STOP};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_EXTEND_MODE_CLAMP, D2D1_GAMMA_2_2, D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES,
    D2D1_ROUNDED_RECT, ID2D1DCRenderTarget,
};
use windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL;
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::PCWSTR;

fn grad(t: &ID2D1DCRenderTarget, stops: &[(f32, D2D1_COLOR_F)], from: (f32, f32), to: (f32, f32))
    -> Option<windows::Win32::Graphics::Direct2D::ID2D1LinearGradientBrush> {
    unsafe {
        let st: Vec<D2D1_GRADIENT_STOP> = stops.iter().map(|(p, c)| D2D1_GRADIENT_STOP { position: *p, color: *c }).collect();
        let coll = t.CreateGradientStopCollection(&st, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP).ok()?;
        t.CreateLinearGradientBrush(
            &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                startPoint: windows_numerics::Vector2 { X: from.0, Y: from.1 },
                endPoint: windows_numerics::Vector2 { X: to.0, Y: to.1 },
            },
            None,
            &coll,
        )
        .ok()
    }
}

/// Vykreslí proužek a umístí ho. Vrací jeho obdélník na obrazovce (i se stíny) a velikost těla (px).
pub(crate) unsafe fn draw(r: &Renderer, hwnd: HWND, m: &StripModel) -> Option<(RECT, (i32, i32))> {
    unsafe {
        let mt = m.metrics;
        let deco = crate::appearance::stripdeco::strip_deco(m.style);
        let pal = m.palette;
        let pulse = pulse_now(m);
        let dots_only = m.style == StripStyle::Dot;
        let scale = m.dpi as f32 / 96.0;
        let h_logical = if m.line_only { 3.0 } else if m.hot { mt.height + mt.hot_extra } else { mt.height };
        let ks = scale * mt.k;
        let font = 12.5 * ks;
        let fmt = text_format(&r.dw, font, false, deco.font.family()).ok()?;
        let fmt_strong = text_format(&r.dw, font, true, deco.font.family()).ok()?;
        for f in [&fmt, &fmt_strong] {
            let _ = f.SetParagraphAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
        }
        let pad = if dots_only { 9.0 * ks } else { 16.0 * ks };
        let gap = if dots_only { 4.0 * ks } else { 12.0 * ks };
        let dot = 7.0 * ks;
        let dot_gap = 6.0 * ks;
        let pill_pad = 8.0 * ks;
        // styl `dot`: jen tečky stavů (segmenty bez tečky se vynechají); bez stavu jedna neutrální
        let dot_segs: Vec<Segment>;
        let segments: &[Segment] = if dots_only {
            dot_segs = {
                let v: Vec<Segment> = m.segments.iter().filter(|s| s.dot.is_some()).cloned().collect();
                if v.is_empty() {
                    vec![Segment { text: String::new(), color: Rgba::DIM, dot: Some(Rgba::DIM), strong: false, pill: false }]
                } else {
                    v
                }
            };
            &dot_segs
        } else {
            &m.segments
        };

        // šířka obsahu
        let mut widths = Vec::new();
        let mut content = 0.0;
        if !m.line_only {
            for (i, s) in segments.iter().enumerate() {
                if dots_only {
                    widths.push((0.0, dot * 1.6));
                    content += dot * 1.6 + if i + 1 < segments.len() { gap } else { 0.0 };
                    continue;
                }
                let tw = measure(&r.dw, if s.strong { &fmt_strong } else { &fmt }, &s.text);
                let mut w = tw + if s.dot.is_some() { dot + dot_gap } else { 0.0 };
                if s.pill {
                    w += pill_pad * 2.0;
                }
                widths.push((tw, w));
                content += w;
                if i + 1 < segments.len() {
                    content += gap * 2.0 + 1.0;
                }
            }
        }
        let min_w = if dots_only { 44.0 * ks } else { 120.0 * ks };
        let width = if m.line_only { (180.0 * scale).round() } else { (content + pad * 2.0).ceil().max(min_w) };
        let height = (h_logical * scale).round();
        let (bw, bh) = (width as i32, height as i32);
        // stíny a záře stylu leží kolem těla: okno je o ně větší, tělo zůstává na ose
        let (ex_l, ex_r, ex_b) = if m.line_only { (0.0, 0.0, 0.0) } else { deco.extents() };
        let (ex_l, ex_r, ex_b) = ((ex_l * scale).ceil(), (ex_r * scale).ceil(), (ex_b * scale).ceil());
        let (w, h) = (bw + (ex_l + ex_r) as i32, bh + ex_b as i32);
        let mon = m.monitor;
        let axis2 = m.axis2.unwrap_or(2 * mon.left + (mon.right - mon.left));
        let x = (geom::span_x(axis2, bw) - ex_l as i32).clamp(mon.left, (mon.right - w).max(mon.left));
        let y = mon.top;

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
        let dib = CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let old = SelectObject(mem, HGDIOBJ(dib.0));

        let rect = RECT { left: 0, top: 0, right: w, bottom: h };
        if r.target.BindDC(mem, &rect).is_ok() {
            let t = &r.target;
            t.BeginDraw();
            t.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            t.SetTransform(&windows_numerics::Matrix3x2::translation(ex_l, 0.0));
            let radius = if m.line_only { 2.0 * scale } else { mt.radius * scale };
            // horní rohy „za okrajem“: obdélník začíná nad obrazovkou
            let body = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: 0.5, top: -radius, right: width - 0.5, bottom: height - 0.5 },
                radiusX: radius,
                radiusY: radius,
            };
            if m.line_only {
                // bez události (proužek ustoupil nad taby prohlížeče) tlumená neutrální linka,
                // při tlačení na hranu se rozsvítí
                let (col, a) = match (pulse, m.ambient) {
                    (Some((p, l)), _) => (ink(p, pal.light), 0.45 + 0.55 * l),
                    (None, Some(a)) => (ink(a, pal.light), 1.0),
                    (None, None) => (ink(Rgba::DIM, pal.light), if m.hot { 0.9 } else { 0.35 }),
                };
                if let Ok(b) = t.CreateSolidColorBrush(&color(col, a), None) {
                    t.FillRoundedRectangle(&body, &b);
                }
            } else {
                // tělo: svislý přechod podle motivu, dole o chlup světlejší (světlý motiv: při
                // nažhavení tmavší)
                // tvrdý stín (blok) by průsvitným tělem prosvítal → tělo je u něj vždy plné
                let o = if m.hot { 0.96 } else if deco.hard_shadow > 0.0 { 1.0 } else { m.opacity };
                let lift = match (m.hot, pal.light) {
                    (false, _) => 0.0,
                    (true, false) => 0.035,
                    (true, true) => -0.03,
                };
                let tone = |c: Rgba| D2D1_COLOR_F { r: c.0 + lift, g: c.1 + lift, b: c.2 + lift, a: o };
                // stíny stylu pod tělem: tvrdý posunutý (brutal) / měkký ve vrstvách (kapka, štítek)
                if deco.hard_shadow > 0.0 {
                    let d = deco.hard_shadow * scale;
                    let sh = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: body.rect.left + d, top: body.rect.top + d, right: body.rect.right + d, bottom: body.rect.bottom + d }, ..body };
                    if let Ok(b) = t.CreateSolidColorBrush(&color(ink(Rgba::TEXT, pal.light), o), None) {
                        t.FillRoundedRectangle(&sh, &b);
                    }
                }
                if deco.soft_shadow > 0.0 {
                    let s = deco.soft_shadow * scale;
                    for i in 1..=3 {
                        let e = s * i as f32 / 3.0;
                        let sh = D2D1_ROUNDED_RECT {
                            rect: D2D_RECT_F { left: body.rect.left - e * 0.6, top: body.rect.top, right: body.rect.right + e * 0.6, bottom: body.rect.bottom + e },
                            radiusX: body.radiusX + e,
                            radiusY: body.radiusY + e,
                        };
                        if let Ok(b) = t.CreateSolidColorBrush(&color(Rgba(0.1, 0.12, 0.2, 1.0), 0.06 * o), None) {
                            t.FillRoundedRectangle(&sh, &b);
                        }
                    }
                }
                if let Some(b) = grad(t, &[(0.0, tone(pal.top)), (1.0, tone(pal.bottom))], (0.0, 0.0), (0.0, height)) {
                    t.FillRoundedRectangle(&body, &b);
                }
                if deco.edge_w > 0.0 {
                    let edge = match (deco.edge_accent, m.hot) {
                        (true, hot) => Rgba(m.line[0].0, m.line[0].1, m.line[0].2, if hot { 1.0 } else { 0.7 }),
                        (false, true) => pal.edge_hot,
                        (false, false) => pal.edge,
                    };
                    if let Ok(b) = t.CreateSolidColorBrush(&color(edge, 1.0), None) {
                        t.DrawRoundedRectangle(&body, &b, deco.edge_w * scale, None);
                    }
                }
                // segmenty
                let mut cx = pad;
                let mid = height / 2.0;
                for (i, s) in segments.iter().enumerate() {
                    let (tw, sw) = widths[i];
                    let seg_start = cx;
                    if dots_only {
                        let dc = ink(s.dot.unwrap_or(Rgba::DIM), pal.light);
                        let center = windows_numerics::Vector2 { X: cx + sw / 2.0, Y: mid };
                        if let Ok(b) = t.CreateSolidColorBrush(&color(dc, 0.25), None) {
                            t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: dot, radiusY: dot }, &b);
                        }
                        if let Ok(b) = t.CreateSolidColorBrush(&color(dc, 1.0), None) {
                            t.FillEllipse(&D2D1_ELLIPSE { point: center, radiusX: dot * 0.6, radiusY: dot * 0.6 }, &b);
                        }
                        cx = seg_start + sw + gap;
                        continue;
                    }
                    if s.pill {
                        let tint = ink(s.dot.unwrap_or(s.color), pal.light);
                        if let Ok(b) = t.CreateSolidColorBrush(&color(tint, 0.16), None) {
                            let ph = 22.0 * scale;
                            t.FillRoundedRectangle(
                                &D2D1_ROUNDED_RECT {
                                    rect: D2D_RECT_F { left: seg_start, top: mid - ph / 2.0, right: seg_start + sw, bottom: mid + ph / 2.0 },
                                    radiusX: ph / 2.0,
                                    radiusY: ph / 2.0,
                                },
                                &b,
                            );
                        }
                        cx += pill_pad;
                    }
                    if let Some(dc) = s.dot.map(|d| ink(d, pal.light)) {
                        // jemná záře pod tečkou
                        if let Ok(b) = t.CreateSolidColorBrush(&color(dc, 0.25), None) {
                            t.FillEllipse(
                                &D2D1_ELLIPSE {
                                    point: windows_numerics::Vector2 { X: cx + dot / 2.0, Y: mid },
                                    radiusX: dot,
                                    radiusY: dot,
                                },
                                &b,
                            );
                        }
                        if let Ok(b) = t.CreateSolidColorBrush(&color(dc, 1.0), None) {
                            t.FillEllipse(
                                &D2D1_ELLIPSE {
                                    point: windows_numerics::Vector2 { X: cx + dot / 2.0, Y: mid },
                                    radiusX: dot / 2.0,
                                    radiusY: dot / 2.0,
                                },
                                &b,
                            );
                        }
                        cx += dot + dot_gap;
                    }
                    if let Ok(b) = t.CreateSolidColorBrush(&color(ink(s.color, pal.light), 1.0), None) {
                        let txt = wide(&s.text);
                        let lr = D2D_RECT_F { left: cx, top: 0.0, right: cx + tw + 2.0, bottom: height };
                        let f = if s.strong { &fmt_strong } else { &fmt };
                        t.DrawText(&txt, f, &lr, &b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                    }
                    cx = seg_start + sw;
                    if i + 1 < segments.len() {
                        cx += gap;
                        // oddělovač: barva okraje motivu (bílá na tmavém, černá na světlém)
                        let sep = Rgba(pal.edge.0, pal.edge.1, pal.edge.2, 0.12);
                        if let Ok(b) = t.CreateSolidColorBrush(&color(sep, 1.0), None) {
                            t.FillRectangle(&D2D_RECT_F { left: cx, top: mid - 6.0 * ks, right: cx + 1.0, bottom: mid + 6.0 * ks }, &b);
                        }
                        cx += 1.0 + gap;
                    }
                }
                // podpisová linka dole: v klidu tlumený fialovo-modrý přechod (nebo barva z configu),
                // při události plná barva; styl `line` = linka skoro přes celou šířku
                let share = match (deco.line_share, m.style, m.ambient.is_some() || m.hot) {
                    (Some(s), _, true) => s.max(0.78),
                    (Some(s), _, false) => s,
                    (None, StripStyle::Line, _) => 0.94,
                    (None, _, true) => 0.78,
                    (None, _, false) => 0.42,
                };
                // puls: linka se v rytmu roztáhne k plné šířce
                let share = match pulse {
                    Some((_, l)) => share + (0.94 - share) * l,
                    None => share,
                };
                let lw = width * share;
                let left = (width - lw) / 2.0;
                let lh = deco.line_h * scale;
                let line = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F { left, top: height - lh, right: left + lw, bottom: height },
                    radiusX: scale.min(lh / 2.0),
                    radiusY: scale.min(lh / 2.0),
                };
                let a = if m.hot { 1.0 } else { 0.55 };
                let stops: Vec<(f32, D2D1_COLOR_F)> = match pulse.map(|(c, l)| (c, 0.5 + 0.5 * l)).or(m.ambient.map(|c| (c, 1.0))) {
                    // událost (nebo puls): plná barva uprostřed, při pulsu jas v rytmu
                    Some((c, pa)) => {
                        let c = ink(c, pal.light);
                        vec![(0.0, color(c, 0.0)), (0.5, color(c, pa)), (1.0, color(c, 0.0))]
                    }
                    None => {
                        let [l0, l1, l2] = m.line;
                        vec![(0.0, color(l0, 0.0)), (0.35, color(l0, a)), (0.65, color(l1, a)), (1.0, color(l2, 0.0))]
                    }
                };
                // neonová záře: tatáž linka ve dvou širších průsvitných vrstvách (statická, kreslí se jen při změně)
                if deco.glow > 0.0 {
                    for (spread, k) in [(deco.glow * scale, 0.12), (deco.glow * scale * 0.5, 0.25)] {
                        let faint: Vec<(f32, D2D1_COLOR_F)> = stops.iter().map(|(p, c)| (*p, D2D1_COLOR_F { a: c.a * k, ..*c })).collect();
                        let halo = D2D1_ROUNDED_RECT {
                            rect: D2D_RECT_F { left: left - spread, top: height - lh - spread, right: left + lw + spread, bottom: height + spread },
                            radiusX: spread + lh,
                            radiusY: spread + lh,
                        };
                        if let Some(b) = grad(t, &faint, (left - spread, 0.0), (left + lw + spread, 0.0)) {
                            t.FillRoundedRectangle(&halo, &b);
                        }
                    }
                }
                if let Some(b) = grad(t, &stops, (left, 0.0), (left + lw, 0.0)) {
                    t.FillRoundedRectangle(&line, &b);
                }
            }
            // tečka zmeškaného oznámení (`notify`), jen na proužku s obsahem
            if !m.line_only {
                crate::notify::draw::missed_dot(t, m.missed, width, height, scale);
            }
            t.SetTransform(&windows_numerics::Matrix3x2::identity());
            let _ = t.EndDraw(None, None);
        }
        let _ = &r.d2d;

        let pos = POINT { x, y };
        let size = SIZE { cx: w, cy: h };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(hwnd, Some(screen), Some(&pos), Some(&size), Some(mem), Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);

        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(dib.0));
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        let _ = PCWSTR::null();
        Some((RECT { left: x, top: y, right: x + w, bottom: y + h }, (bw, bh)))
    }
}
