//! Velikost šuplíku vůči monitoru (čisté funkce): výška nikdy nepřeteče pracovní plochu (bez hlavního
//! panelu, s rezervou dole), Nastavení si může říct o širší šuplík (min(žádost, 90 % monitoru)).
//! Používá `drawer.rs` (`Drawer::logical_size`, `ViewMsg::max_h`); obsah nad limit posouvá frontend.

use windows::Win32::Foundation::RECT;

/// Rezerva nad spodkem pracovní plochy (logické px).
pub const BOTTOM_GAP: f64 = 20.0;
/// Nejvyšší podíl šířky monitoru pro širší šuplík (Nastavení).
pub const WIDE_FRACTION: f64 = 0.9;
/// Nikdy menší než tohle (rozbitý monitor / RDP bez displeje).
const MIN_H: f64 = 240.0;
/// Výchozí šířka, když config žádnou nedá.
const DEFAULT_W: f64 = 1080.0;

/// Limity v logických px: `max_w` = šířka monitoru, `max_h` = od horní hrany monitoru po spodek
/// pracovní plochy minus `BOTTOM_GAP` (šuplík visí z horní hrany i přes případný panel nahoře).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub max_w: f64,
    pub max_h: f64,
}

pub fn limits(mon: RECT, work: RECT, dpi: u32) -> Limits {
    let scale = dpi.max(48) as f64 / 96.0;
    let max_w = ((mon.right - mon.left).max(0) as f64 / scale).max(320.0);
    // prázdná pracovní plocha (výčet selhal) = celý monitor
    let bottom = if work.bottom > work.top { work.bottom.min(mon.bottom) } else { mon.bottom };
    let max_h = ((bottom - mon.top) as f64 / scale - BOTTOM_GAP).max(MIN_H);
    Limits { max_w, max_h }
}

/// Velikost Full: `want_w` > 0 = žádost o širší šuplík (Nastavení), jinak šířka z configu.
/// Širší nikdy užší než běžný a nikdy přes 90 % monitoru; výška oříznutá na `max_h`.
pub fn fit_full(want_w: f64, config_w: f64, h: f64, lim: Limits) -> (f64, f64) {
    let base = if config_w > 0.0 { config_w } else { DEFAULT_W };
    let w = if want_w > 0.0 { want_w.min(lim.max_w * WIDE_FRACTION).max(base) } else { base };
    (w.min(lim.max_w).round(), h.min(lim.max_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT { left, top, right, bottom }
    }

    #[test]
    fn limits_table() {
        // (monitor, pracovní plocha, dpi, max_w, max_h)
        let cases = [
            // 1080p, 100 %, panel dole 48 px
            (r(0, 0, 1920, 1080), r(0, 0, 1920, 1032), 96, 1920.0, 1012.0),
            // 1080p, 125 %: 1536×864 logicky, panel 48 px fyzicky
            (r(0, 0, 1920, 1080), r(0, 0, 1920, 1032), 120, 1536.0, 1032.0 / 1.25 - 20.0),
            // 1080p, 150 %
            (r(0, 0, 1920, 1080), r(0, 0, 1920, 1032), 144, 1280.0, 1032.0 / 1.5 - 20.0),
            // panel nahoře: šuplík visí z horní hrany monitoru → výška po spodek monitoru
            (r(0, 0, 1920, 1080), r(0, 48, 1920, 1080), 96, 1920.0, 1060.0),
            // druhý monitor vpravo, 1440p, 150 %, panel dole
            (r(1920, 0, 4480, 1440), r(1920, 0, 4480, 1392), 144, 1706.666, 1392.0 / 1.5 - 20.0),
            // monitor nad hlavním (záporné souřadnice)
            (r(0, -1080, 1920, 0), r(0, -1080, 1920, -40), 96, 1920.0, 1020.0),
            // prázdná pracovní plocha = celý monitor
            (r(0, 0, 1920, 1080), r(0, 0, 0, 0), 96, 1920.0, 1060.0),
            // nesmyslně malý monitor → minimum
            (r(0, 0, 200, 100), r(0, 0, 200, 100), 96, 320.0, 240.0),
        ];
        for (mon, work, dpi, w, h) in cases {
            let l = limits(mon, work, dpi);
            assert!((l.max_w - w).abs() < 0.01, "{mon:?} {dpi}: w {}", l.max_w);
            assert!((l.max_h - h).abs() < 0.01, "{mon:?} {dpi}: h {}", l.max_h);
        }
    }

    #[test]
    fn fit_full_table() {
        let l1080 = Limits { max_w: 1920.0, max_h: 1012.0 };
        let l150 = Limits { max_w: 1280.0, max_h: 668.0 };
        let tiny = Limits { max_w: 1024.0, max_h: 700.0 };
        // (want_w, config_w, h, limity, čekáno)
        let cases = [
            (0.0, 1080.0, 500.0, l1080, (1080.0, 500.0)),
            (0.0, 1080.0, 1500.0, l1080, (1080.0, 1012.0)),  // přetéká → ořízne
            (1200.0, 1080.0, 900.0, l1080, (1200.0, 900.0)), // Nastavení širší
            (1200.0, 1080.0, 900.0, l150, (1152.0, 668.0)),  // 90 % z 1280
            (1200.0, 1300.0, 400.0, l1080, (1300.0, 400.0)), // nikdy užší než běžný šuplík
            (1200.0, 1080.0, 400.0, tiny, (1024.0, 400.0)),  // malý monitor: max šířka monitoru
            (0.0, 0.0, 300.0, l1080, (1080.0, 300.0)),       // config bez šířky
        ];
        for (want, cw, h, lim, exp) in cases {
            assert_eq!(fit_full(want, cw, h, lim), exp, "want {want} cfg {cw} h {h}");
        }
    }
}
