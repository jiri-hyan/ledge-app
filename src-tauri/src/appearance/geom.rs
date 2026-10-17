//! Čistá geometrie a barvy vzhledu pro Rust: kde u horní hrany leží proužek a šuplík (`anchor_c2`,
//! `span_x`), ořez šířky šuplíku na monitor (`drawer_width`), tvar proužku podle stylu a velikosti
//! (`strip_metrics`) a jeho paleta podle motivu (`palette`, `ink`). Bez WinAPI — tabulkové testy.
//! Volá `stripview::refresh_strip` (model proužku), `strip::draw`, `drawer::place`, `threads::hover`.

use super::{AppearanceConfig, StripPosition, StripSize, StripStyle, Theme};
use crate::strip::Rgba;

/// Odstup šuplíku od boční hrany monitoru u pozice `left` / `right` (logické px).
pub const MARGIN: i32 = 24;
/// Šuplík zabere nejvýš tolik šířky monitoru.
pub const MAX_SHARE: f64 = 0.95;

/// Dvojnásobek x středu „kotvy“ (px) — osa, na které leží proužek, malý i velký šuplík, aby morf
/// proužek ⇄ šuplík zůstal souměrný. Dvojnásobek kvůli lichým šířkám (žádné zaokrouhlení navíc).
/// `full_w` = šířka velkého šuplíku v px: u `left`/`right` k ní přiléhá okraj s odstupem `MARGIN`.
pub fn anchor_c2(left: i32, right: i32, full_w: i32, pos: StripPosition, dpi: u32) -> i32 {
    let mw = (right - left).max(0);
    let full = full_w.clamp(0, mw);
    let margin = MARGIN * dpi as i32 / 96;
    match pos {
        StripPosition::Center => 2 * left + mw,
        StripPosition::Left => 2 * (left + margin).min(right - full) + full,
        StripPosition::Right => 2 * (right - margin - full).max(left) + full,
    }
}

/// Levý okraj prvku šířky `w` (px) vystředěného na kotvě `c2` (zaokrouhlení dolů jako dřív
/// `left + (mw − w) / 2`, i na monitorech se zápornými souřadnicemi).
pub fn span_x(c2: i32, w: i32) -> i32 {
    (c2 - w).div_euclid(2)
}

/// Osa Ledge u horní hrany podle configu (`anchor_c2` se šířkou šuplíku oříznutou na monitor) —
/// jediný výpočet pro proužek (`stripview`) i hover zónu bez proužku (`threads::hover`).
pub fn axis2_for(cfg: &crate::config::Config, left: i32, right: i32, dpi: u32) -> i32 {
    let full = (drawer_width(cfg.drawer.width, right - left, dpi) * dpi as f64 / 96.0).round() as i32;
    anchor_c2(left, right, full, cfg.appearance.strip.position, dpi)
}

/// Šířka velkého šuplíku (logické px): `drawer.width` v rozsahu `DRAWER_WIDTH` a nejvýš `MAX_SHARE`
/// šířky monitoru. `mon_w` = šířka monitoru v px (0 = neznámá → bez stropu monitoru).
pub fn drawer_width(cfg_w: u32, mon_w: i32, dpi: u32) -> f64 {
    let (lo, hi) = super::DRAWER_WIDTH;
    let w = cfg_w.clamp(lo, hi) as f64;
    if mon_w <= 0 {
        return w;
    }
    let cap = (mon_w as f64 * MAX_SHARE / (dpi.max(1) as f64 / 96.0)).floor();
    w.min(cap)
}

/// Tvar proužku v logických px (bez „nažhavení“, které přidá `hot_extra`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripMetrics {
    /// násobek velikosti (písmo, odsazení, tečky)
    pub k: f32,
    pub height: f32,
    pub radius: f32,
    pub hot_extra: f32,
}

pub fn size_factor(s: StripSize) -> f32 {
    match s {
        StripSize::Small => 0.85,
        StripSize::Normal => 1.0,
        StripSize::Large => 1.2,
    }
}

/// Zrcadlo `stripShape` v `src/theme/appearance.ts` (výchozí tvar morfu ve frontendu).
pub fn strip_metrics(style: StripStyle, size: StripSize) -> StripMetrics {
    let k = size_factor(size);
    let (height, radius) = match style {
        StripStyle::Pill => (32.0 * k, 14.0 * k),
        StripStyle::Line => (26.0 * k, 3.0 * k),
        StripStyle::Dot => (22.0 * k, 11.0 * k),
    };
    StripMetrics { k, height, radius, hot_extra: 4.0 * k }
}

/// Barvy těla proužku podle motivu: svislý přechod `top` → `bottom`, okraj (`edge`, `edge_hot`),
/// `light` = tmavý text na světlém těle (přemapuje `ink`) a akcent motivu (přechod podpisové linky,
/// zrcadlo `accent`/`accent-2` v `src/theme/themes.ts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub top: Rgba,
    pub bottom: Rgba,
    pub edge: Rgba,
    pub edge_hot: Rgba,
    pub light: bool,
    pub accent: [Rgba; 3],
}

pub fn palette(theme: Theme) -> Palette {
    let rgb = |r: f32, g: f32, b: f32| Rgba(r, g, b, 1.0);
    let white = |a: f32| Rgba(1.0, 1.0, 1.0, a);
    let (top, bottom, edge, edge_hot, light) = match theme {
        Theme::Dark => (rgb(0.035, 0.040, 0.058), rgb(0.085, 0.094, 0.150), white(0.10), white(0.22), false),
        Theme::Midnight => (rgb(0.020, 0.027, 0.063), rgb(0.051, 0.071, 0.161), white(0.10), white(0.24), false),
        Theme::Graphite => (rgb(0.071, 0.071, 0.078), rgb(0.122, 0.122, 0.133), white(0.11), white(0.22), false),
        Theme::Nord => (rgb(0.180, 0.204, 0.251), rgb(0.208, 0.235, 0.290), white(0.12), white(0.24), false),
        Theme::Light => (rgb(0.973, 0.976, 0.988), rgb(0.910, 0.918, 0.945), Rgba(0.0, 0.0, 0.0, 0.14), Rgba(0.0, 0.0, 0.0, 0.28), true),
        Theme::Contrast => (rgb(0.0, 0.0, 0.0), rgb(0.0, 0.0, 0.0), white(0.6), white(0.95), false),
    };
    let hex = |s: &str| parse_hex(s).unwrap_or(Rgba::DIM);
    let accent = match theme {
        // dnešní linka: fialová → modrá → (mizí do) tyrkysové
        Theme::Dark => [Rgba::VIOLET, Rgba::BLUE, Rgba::TEAL],
        Theme::Midnight => [hex("#818cf8"), hex("#38bdf8"), hex("#38bdf8")],
        Theme::Graphite => [hex("#a1a1aa"), hex("#d4d4d8"), hex("#d4d4d8")],
        Theme::Nord => [hex("#88c0d0"), hex("#81a1c1"), hex("#81a1c1")],
        Theme::Light => [hex("#7c3aed"), hex("#2563eb"), hex("#2563eb")],
        Theme::Contrast => [hex("#ffd400"), hex("#00e5ff"), hex("#00e5ff")],
    };
    Palette { top, bottom, edge, edge_hot, light, accent }
}

/// Barva textu/tečky na těle proužku: na světlém motivu tmavá varianta (kontrast ≥ 4.5:1).
pub fn ink(c: Rgba, light: bool) -> Rgba {
    if !light {
        return c;
    }
    if c == Rgba::TEXT {
        Rgba(0.07, 0.08, 0.11, c.3)
    } else if c == Rgba::DIM {
        Rgba(0.29, 0.31, 0.37, c.3)
    } else {
        Rgba(c.0 * 0.5, c.1 * 0.5, c.2 * 0.5, c.3)
    }
}

/// `#rrggbb` → barva (neprůhledná).
pub fn parse_hex(s: &str) -> Option<Rgba> {
    if !super::is_hex(s) {
        return None;
    }
    let ch = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok().map(|v| v as f32 / 255.0);
    Some(Rgba(ch(1)?, ch(3)?, ch(5)?, 1.0))
}

/// Druhá barva přechodu k vlastnímu akcentu: odstín posunutý o −40° (u výchozí fialové vyjde modrá
/// jako dnes); zrcadlo `accentPair` v TS.
pub fn accent_pair(hex: &str) -> Option<(Rgba, Rgba)> {
    let c = parse_hex(hex)?;
    let (r, g, b) = (c.0, c.1, c.2);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    if max == min {
        return Some((c, c));
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r { (g - b) / d + if g < b { 6.0 } else { 0.0 } } else if max == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 } * 60.0;
    let h = (h + 320.0) % 360.0;
    let a = s * l.min(1.0 - l);
    let f = |n: f32| {
        let k = (n + h / 30.0) % 12.0;
        let v = l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (v * 255.0).round() / 255.0
    };
    Some((c, Rgba(f(0.0), f(8.0), f(4.0), 1.0)))
}

/// Barvy podpisové linky proužku (začátek, střed, dozvuk): prázdné = přechod akcentu (vlastního, jinak
/// motivu — jako linka šuplíku), `accent` = jednobarevně, `#rrggbb` = vlastní barva.
pub fn line_stops(a: &AppearanceConfig, pal: &Palette) -> [Rgba; 3] {
    let own = accent_pair(&a.accent);
    match a.strip.color.as_str() {
        "" => own.map(|(x, y)| [x, y, y]).unwrap_or(pal.accent),
        "accent" => [own.map(|p| p.0).unwrap_or(pal.accent[0]); 3],
        hex => parse_hex(hex).map(|c| [c; 3]).unwrap_or(pal.accent),
    }
}

/// Relativní jas (WCAG) barvy bez průhlednosti.
pub fn lum(c: Rgba) -> f32 {
    let f = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}

/// Kontrastní poměr dvou barev (WCAG, 1–21).
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Barva textu segmentu z volby uživatele (`strip.colors`), čitelná na těle proužku: vrací barvu PŘED
/// `ink` (kreslení ji ještě přemapuje), jejíž `ink` má na obou koncích přechodu kontrast ≥ 4.5:1. Barva,
/// která nestačí, se po krocích míchá k bílé (tmavé motivy) / černé (světlý motiv) — odstín zůstane znát.
pub fn readable(c: Rgba, pal: &Palette) -> Rgba {
    let target = if pal.light { Rgba(0.0, 0.0, 0.0, c.3) } else { Rgba(1.0, 1.0, 1.0, c.3) };
    let ok = |x: Rgba| {
        let k = ink(x, pal.light);
        contrast(k, pal.top) >= 4.5 && contrast(k, pal.bottom) >= 4.5
    };
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        let x = Rgba(c.0 + (target.0 - c.0) * t, c.1 + (target.1 - c.1) * t, c.2 + (target.2 - c.2) * t, c.3);
        if ok(x) {
            return x;
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: (i32, i32) = (0, 1920);

    #[test]
    fn center_matches_old_formula() {
        // dřív: x = left + (mw − w) / 2 — výchozí pozice se nesmí posunout ani o pixel
        for (left, right, w) in [(0, 1920, 1080), (0, 1920, 377), (-1920, 0, 1081), (1920, 4480, 2047), (-2560, -1, 600)] {
            let c2 = anchor_c2(left, right, 1080, StripPosition::Center, 96);
            assert_eq!(span_x(c2, w), left + ((right - left) - w) / 2, "{left}..{right} w={w}");
        }
    }

    #[test]
    fn left_right_with_dpi() {
        // (monitor, dpi, šířka šuplíku px, pozice, očekávané x šuplíku)
        let cases = [
            (FHD, 96, 1080, StripPosition::Left, 24),
            (FHD, 144, 1620, StripPosition::Left, 36),
            (FHD, 96, 1080, StripPosition::Right, 1920 - 24 - 1080),
            (FHD, 192, 1824, StripPosition::Right, 1920 - 48 - 1824),
            ((-1920, 0), 96, 1080, StripPosition::Left, -1920 + 24),
            ((-1920, 0), 120, 1350, StripPosition::Right, -30 - 1350),
            // šuplík širší než monitor minus odstup → přilepí se k druhé hraně, nevyjede ven
            ((0, 1000), 96, 990, StripPosition::Left, 10),
            ((0, 1000), 96, 990, StripPosition::Right, 0),
        ];
        for ((l, r), dpi, full, pos, want) in cases {
            let c2 = anchor_c2(l, r, full, pos, dpi);
            let x = span_x(c2, full);
            assert_eq!(x, want, "{pos:?} {l}..{r} dpi {dpi}");
            assert!(x >= l && x + full <= r, "{pos:?} v monitoru");
        }
    }

    #[test]
    fn strip_and_peek_share_the_axis() {
        // proužek (360 px) i malý šuplík (680) leží na ose velkého šuplíku → morf je souměrný
        for pos in [StripPosition::Center, StripPosition::Left, StripPosition::Right] {
            for dpi in [96, 120, 144, 192] {
                let full = 1080 * dpi as i32 / 96;
                let c2 = anchor_c2(0, 3840, full, pos, dpi);
                let fx = span_x(c2, full);
                for w in [360, 361, 680 * dpi as i32 / 96] {
                    let x = span_x(c2, w);
                    let d = (fx + full / 2) - (x + w / 2);
                    assert!(d.abs() <= 1, "{pos:?} dpi {dpi} w {w}: odchylka středu {d}");
                    assert!(x >= 0 && x + w <= 3840);
                }
            }
        }
    }

    #[test]
    fn drawer_width_clamps() {
        let cases = [
            (1080, 1920, 96, 1080.0),
            (1080, 1366, 96, 1080.0),
            (1800, 1920, 96, 1800.0),
            (2000, 1920, 96, 1824.0),
            (1080, 1920, 192, 912.0), // 1920 px při 200 % = 960 logických px → 95 %
            (500, 1920, 96, 720.0),
            (9000, 0, 96, 2400.0),
            (1200, 2560, 144, 1200.0),
        ];
        for (cfg, mon, dpi, want) in cases {
            assert_eq!(drawer_width(cfg, mon, dpi), want, "{cfg} na {mon} px @ {dpi}");
        }
    }

    #[test]
    fn metrics_default_is_today() {
        let m = strip_metrics(StripStyle::Pill, StripSize::Normal);
        assert_eq!((m.height, m.radius, m.hot_extra, m.k), (crate::strip::HEIGHT, 14.0, 4.0, 1.0));
        assert!(strip_metrics(StripStyle::Dot, StripSize::Small).height < strip_metrics(StripStyle::Line, StripSize::Small).height);
        assert!(strip_metrics(StripStyle::Pill, StripSize::Large).height > m.height);
    }

    #[test]
    fn strip_text_contrast_in_every_theme() {
        // texty proužku (stripview) ≥ 4.5:1 na obou koncích přechodu těla
        let themes = [Theme::Dark, Theme::Midnight, Theme::Graphite, Theme::Nord, Theme::Light, Theme::Contrast];
        for t in themes {
            let p = palette(t);
            for c in [Rgba::TEXT, Rgba::DIM, Rgba::AMBER] {
                for bg in [p.top, p.bottom] {
                    let k = contrast(ink(c, p.light), bg);
                    assert!(k >= 4.5, "{t:?} {c:?}: {k:.2}");
                }
            }
        }
    }

    #[test]
    fn line_stops_resolution() {
        let mut a = AppearanceConfig::default();
        let pal = palette(Theme::Dark);
        assert_eq!(line_stops(&a, &pal), [Rgba::VIOLET, Rgba::BLUE, Rgba::TEAL], "výchozí = dnešní linka");
        a.strip.color = "accent".into();
        assert_eq!(line_stops(&a, &pal), [Rgba::VIOLET; 3], "akcent nenastaven → akcent motivu");
        a.accent = "#ff0000".into();
        let red = Rgba(1.0, 0.0, 0.0, 1.0);
        assert_eq!(line_stops(&a, &pal), [red; 3]);
        a.strip.color = String::new();
        assert_eq!(line_stops(&a, &pal)[0], red);
        a.strip.color = "#0000ff".into();
        assert_eq!(line_stops(&a, &pal), [Rgba(0.0, 0.0, 1.0, 1.0); 3]);
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn accent_pair_matches_frontend() {
        // stejné hodnoty jako accentPair v src/theme/appearance.ts (#ff0000 → #ff00aa, #22c55e → #53c522)
        let hex = |c: Rgba| format!("#{:02x}{:02x}{:02x}", (c.0 * 255.0).round() as u8, (c.1 * 255.0).round() as u8, (c.2 * 255.0).round() as u8);
        for (inp, want) in [("#ff0000", "#ff00aa"), ("#22c55e", "#53c522"), ("#808080", "#808080")] {
            assert_eq!(hex(accent_pair(inp).unwrap().1), want, "{inp}");
        }
    }

    #[test]
    fn theme_accents_mirror_frontend() {
        let ts = include_str!("../../../src/theme/themes.ts");
        let hex = |c: Rgba| format!("#{:02x}{:02x}{:02x}", (c.0 * 255.0).round() as u8, (c.1 * 255.0).round() as u8, (c.2 * 255.0).round() as u8);
        for t in [Theme::Midnight, Theme::Graphite, Theme::Nord, Theme::Light, Theme::Contrast] {
            let p = palette(t);
            let pair = format!("accent: \"{}\", \"accent-2\": \"{}\"", hex(p.accent[0]), hex(p.accent[1]));
            assert!(ts.contains(&pair), "{t:?}: {pair} chybí v themes.ts");
        }
    }
}
