//! Tvar a místo stylu zavřeného Ledge u horní hrany (čistá geometrie v px): rozměry stylu (`spec`),
//! šířka těla, okno vrstveného proužku a poloha těla v něm (`place`), hover zóna (`zone`), výchozí
//! tvar morfu šuplíku (`morph_body`) a plynulá změna šířky ostrůvku (`WidthAnim`). Kreslí `stripdraw`.

use super::{Look, Shape, Show};
use crate::appearance::StripPosition;
use crate::appearance::geom::span_x;
use crate::strip::Rgba;
use windows::Win32::Foundation::RECT;

/// Rozměry a výplň stylu v logických px při velikosti 1 (`appearance.strip.size` je násobí).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub h: f32,
    /// rádius dolních rohů (horní jsou „za hranou“); u `pill` všech čtyř
    pub r: f32,
    pub pad: f32,
    pub gap: f32,
    pub min_w: f32,
    /// velikost písma textu
    pub font: f32,
    /// výplň těla; alfa 0 = bez těla (tečky)
    pub fill: Rgba,
    /// okraj těla
    pub edge: Option<Rgba>,
    /// okraj jen dole (stavová lišta)
    pub edge_bottom: bool,
    /// „ouška“ výřezu po stranách (px)
    pub ears: f32,
    /// odstup od horní hrany (roh)
    pub top_gap: f32,
    /// odstup od boční hrany (roh)
    pub side_gap: f32,
    /// tělo zaoblené ze všech stran (odznak v rohu)
    pub pill: bool,
    pub mono: bool,
    /// větší písmo hodin (Segoe UI Variable Display)
    pub display: bool,
    /// průměr stavové tečky
    pub dot: f32,
    /// průměr prstence (0 = jako `dot`; vlastní vzhled má tečky i prstence)
    pub ring: f32,
    /// rádius těla `pill` (0 = půl výšky; vlastní vzhled podle zaoblení)
    pub pill_r: f32,
}

const BLACK: Rgba = Rgba(0.0, 0.0, 0.0, 1.0);

pub fn spec(look: Look, show: &Show) -> Spec {
    let base = Spec {
        h: 26.0,
        r: 13.0,
        pad: 14.0,
        gap: 8.0,
        min_w: 0.0,
        font: 12.5,
        fill: BLACK,
        edge: None,
        edge_bottom: false,
        ears: 0.0,
        top_gap: 0.0,
        side_gap: 0.0,
        pill: false,
        mono: false,
        display: false,
        dot: 7.0,
        ring: 0.0,
        pill_r: 0.0,
    };
    match look {
        // klasický proužek kreslí `strip::draw`; tady jen rozumné rozměry pro hover zónu a morf
        Look::Classic => Spec { h: 32.0, ..base },
        Look::Island => Spec { min_w: if show.idle { 56.0 } else { 110.0 }, ..base },
        Look::Dots => Spec { h: 20.0, r: 0.0, pad: 6.0, gap: 7.0, fill: Rgba(0.0, 0.0, 0.0, 0.0), dot: 9.0, ..base },
        Look::Line => Spec { h: 3.0, r: 0.0, pad: 0.0, gap: 0.0, fill: Rgba(0.0, 0.0, 0.0, 0.0), ..base },
        Look::Icons => Spec {
            r: 12.0,
            pad: 10.0,
            gap: 12.0,
            font: 15.0,
            fill: Rgba(0.059, 0.067, 0.094, 0.82),
            edge: Some(Rgba(1.0, 1.0, 1.0, 0.12)),
            ..base
        },
        Look::Clock => Spec { h: 28.0, r: 10.0, pad: 12.0, font: 16.0, fill: Rgba(0.0, 0.0, 0.0, 0.55), display: true, ..base },
        Look::Corner => Spec {
            h: 22.0,
            r: 11.0,
            pad: 10.0,
            gap: 6.0,
            font: 11.5,
            fill: Rgba(0.067, 0.067, 0.067, 1.0),
            edge: Some(Rgba(1.0, 1.0, 1.0, 0.14)),
            top_gap: 8.0,
            side_gap: 10.0,
            pill: true,
            ..base
        },
        Look::Ghost => Spec { r: 14.0, ..base },
        Look::Bar => Spec {
            h: 20.0,
            r: 0.0,
            pad: 10.0,
            gap: 8.0,
            font: 11.0,
            fill: Rgba(0.043, 0.051, 0.063, 1.0),
            edge: Some(Rgba(0.133, 0.133, 0.133, 1.0)),
            edge_bottom: true,
            mono: true,
            ..base
        },
        Look::Notch => Spec { h: 28.0, r: 18.0, pad: 18.0, gap: 6.0, min_w: 84.0, ears: 12.0, ..base },
        Look::Progress => Spec {
            h: if show.progress.is_some() { 30.0 } else { 24.0 },
            r: 10.0,
            pad: 12.0,
            gap: 10.0,
            min_w: 180.0,
            font: 11.0,
            fill: Rgba(0.039, 0.047, 0.071, 0.85),
            ..base
        },
        Look::Ticker => Spec { h: 24.0, r: 12.0, min_w: 200.0, font: 12.0, fill: Rgba(0.0, 0.0, 0.0, 0.7), ..base },
        // kolo 22: malá kapsle (baterie + čas), trojice ~360 × 36, stav Clauda s prstenci (`dot` = průměr prstence)
        Look::Minimum => Spec { h: 24.0, r: 12.0, pad: 12.0, gap: 6.0, min_w: 64.0, font: 12.0, ..base },
        Look::Trio => Spec { h: 36.0, r: 18.0, pad: 16.0, gap: 8.0, min_w: 200.0, font: 12.5, ..base },
        Look::Rings => Spec { h: 28.0, r: 14.0, pad: 12.0, gap: 6.0, min_w: 72.0, font: 13.0, dot: 16.0, ..base },
        // vlastní: skutečné rozměry z `strip.compose` (`compose::spec`); tady jen náhrada bez configu
        Look::Custom => base,
    }
}

/// Odstup kapsle „jako Mac“ od horní hrany (logické px).
pub const ISLAND_GAP: f32 = 6.0;

/// Tvar kapsle (`strip.lookShape`) na rozměry stylu; styly bez `shapeable` beze změny.
pub fn shaped(look: Look, sp: Spec, shape: Shape) -> Spec {
    if !look.shapeable() {
        return sp;
    }
    match shape {
        Shape::Round => sp,
        Shape::Sharp => Spec { r: sp.r.min(4.0), ..sp },
        Shape::Long => Spec { pad: sp.pad * 2.0, min_w: sp.min_w.max(200.0), ..sp },
        Shape::Island => Spec { pill: true, top_gap: ISLAND_GAP, ..sp },
    }
}

/// Šířka těla (px): přes celou šířku monitoru, nebo obsah + odsazení, aspoň `min_w`.
pub fn body_width(look: Look, content_px: f32, sp: &Spec, ks: f32, mon: RECT) -> i32 {
    if look.full_width() {
        return (mon.right - mon.left).max(1);
    }
    (content_px + 2.0 * sp.pad * ks).max(sp.min_w * ks).ceil().max(1.0) as i32
}

/// Okno proužku na obrazovce a tělo v něm (px).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub win: RECT,
    /// levý / horní okraj těla v okně
    pub ox: i32,
    pub oy: i32,
    pub bw: i32,
    pub bh: i32,
}

/// Kam okno patří: celá šířka (linka, lišta), roh podle `position` (střed = pravý roh), výřez s oušky
/// na ose, ostatní vystředěné na ose Ledge (`axis2` = `geom::anchor_c2`). Vždy uvnitř monitoru.
#[allow(clippy::too_many_arguments)]
pub fn place(look: Look, pos: StripPosition, mon: RECT, axis2: i32, bw: i32, bh: i32, sp: &Spec, ks: f32) -> Placed {
    let top = mon.top;
    if look.full_width() {
        return Placed { win: RECT { left: mon.left, top, right: mon.right, bottom: top + bh }, ox: 0, oy: 0, bw: mon.right - mon.left, bh };
    }
    let fit = |x: i32, w: i32| x.clamp(mon.left, (mon.right - w).max(mon.left));
    if look == Look::Corner {
        let tg = (sp.top_gap * ks).round() as i32;
        let side = (sp.side_gap * ks).round() as i32;
        let x = match pos {
            StripPosition::Left => mon.left + side,
            StripPosition::Center | StripPosition::Right => mon.right - side - bw,
        };
        let x = fit(x, bw);
        return Placed { win: RECT { left: x, top, right: x + bw, bottom: top + tg + bh }, ox: 0, oy: tg, bw, bh };
    }
    let e = (sp.ears * ks).ceil() as i32;
    let w = bw + 2 * e;
    let x = fit(span_x(axis2, bw) - e, w);
    // kapsle „jako Mac“ (`Shape::Island`) visí kousek pod hranou; ostatní tvary u hrany (`top_gap` 0)
    let tg = (sp.top_gap * ks).round() as i32;
    Placed { win: RECT { left: x, top, right: x + w, bottom: top + tg + bh }, ox: e, oy: tg, bw, bh }
}

/// Hover zóna u horní hrany (px): obdélník proužku (`srect`, i s okolím 24 px) oříznutý na monitor,
/// aspoň 4 px vysoká (linka má 3 px); skrytý proužek (neviditelný styl v klidu, vypnutý proužek) =
/// pruh 320 × 4 px na ose Ledge (`cx`). Vrací (vlevo, vpravo, spodek).
pub fn zone(srect: RECT, cx: i32, mon: RECT, dpi: u32) -> (i32, i32, i32) {
    let s = dpi as i32;
    let min_b = mon.top + 4 * s / 96;
    let (l, r, b) = if srect.right > srect.left {
        (srect.left - 24 * s / 96, srect.right + 24 * s / 96, srect.bottom.max(min_b))
    } else {
        (cx - 160 * s / 96, cx + 160 * s / 96, min_b)
    };
    (l.max(mon.left), r.min(mon.right), b)
}

/// Výchozí tvar morfu šuplíku (px těla): u stylů přes celou šířku jen kus kolem osy (šuplík není
/// širší než monitor a morf je středový).
pub fn morph_body(look: Look, bw: i32, bh: i32, scale: f32) -> (i32, i32) {
    if look.full_width() { (bw.min((600.0 * scale) as i32), bh) } else { (bw, bh) }
}

/// Délka plynulé změny šířky ostrůvku (ms).
pub const ISLAND_MS: u64 = 240;

/// Plynulá změna šířky ostrůvku: jen při změně obsahu (nový cíl), v klidu nic; bez animací
/// („omezit pohyb“, `motion.style: none`) rovnou na cíl.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WidthAnim {
    from: f32,
    to: f32,
    cur: f32,
    t0: Option<u64>,
}

impl WidthAnim {
    /// Šířka k vykreslení v čase `now` (ms).
    pub fn step(&mut self, target: f32, animate: bool, now: u64) -> f32 {
        if (target - self.to).abs() > 0.5 {
            if animate && self.cur > 0.0 {
                self.from = self.cur;
                self.t0 = Some(now);
            } else {
                self.t0 = None;
            }
            self.to = target;
        }
        match self.t0 {
            Some(t0) if now.saturating_sub(t0) < ISLAND_MS => {
                let x = now.saturating_sub(t0) as f32 / ISLAND_MS as f32;
                let k = 1.0 - (1.0 - x).powi(3);
                self.cur = self.from + (self.to - self.from) * k;
            }
            _ => {
                self.t0 = None;
                self.cur = self.to;
            }
        }
        self.cur
    }

    pub fn running(&self) -> bool {
        self.t0.is_some()
    }

    /// Proužek zmizel: příště se ukáže rovnou v cílové šířce.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
