//! Choreografie Notche jako čistá funkce: čas od začátku → tvar (šířka, výška, kapka, průhlednost, záře).
//! Bez Win32 — kreslí `notify::draw`. Konec = `None` (vlákno `strip` pak zruší snímkový časovač).

use super::conf::NotchStyle;

/// Otevírání (ms).
pub const OPEN_MS: u64 = 340;
/// Zavírání (ms).
pub const CLOSE_MS: u64 = 260;
/// Mezera mezi proužkem a kapkou (DIP).
pub const DROP_GAP: f32 = 8.0;

/// Cílové rozměry tvaru v DIP podle stylu (`base` = tělo proužku před oznámením).
pub fn target(style: NotchStyle, base: (f32, f32)) -> (f32, f32) {
    let (bw, bh) = base;
    let h = bh.max(26.0);
    match style {
        NotchStyle::Unfold => (bw.max(340.0), 84.0),
        NotchStyle::Sides => (bw.max(320.0), h),
        NotchStyle::Drop => (bw.clamp(240.0, 300.0), 48.0),
        NotchStyle::Ticker => (bw.max(300.0), h),
        NotchStyle::Glow => (bw.max(28.0), h),
    }
}

/// Snímek tvaru.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// šířka tvaru (DIP)
    pub w: f32,
    /// výška tvaru (DIP)
    pub h: f32,
    /// kapka: posun horní hrany bubliny pod tělo proužku (DIP); `None` = tvar roste z proužku
    pub drop_y: Option<f32>,
    /// krytí obsahu (text, ikona, tlačítko) 0–1
    pub alpha: f32,
    /// síla záře kolem tvaru 0–1
    pub glow: f32,
    /// postup během držení 0–1 (tah fajfky, ticker, vlny)
    pub progress: f32,
    /// tvar je plně otevřený a stojí — jen tehdy smí tlačítko reagovat
    pub settled: bool,
}

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - clamp01(t)).powi(3)
}
/// Mírný překmit (pružina) na konci otevření.
fn ease_out_back(t: f32) -> f32 {
    let t = clamp01(t);
    let c1 = 1.2;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
/// Úsek `[a, b]` z 0–1 přemapovaný na 0–1.
fn span(t: f32, a: f32, b: f32) -> f32 {
    clamp01((t - a) / (b - a))
}

/// Tvar v čase `t` ms (od začátku oznámení), `hold` = jak dlouho stojí otevřený. `None` = oznámení skončilo.
pub fn frame(style: NotchStyle, t: u64, hold: u64, base: (f32, f32)) -> Option<Frame> {
    if t >= total_ms(hold) {
        return None;
    }
    let (tw, th) = target(style, base);
    // otevřenost 0–1 (otevírání / držení / zavírání) a lineární čas fáze
    let (open, opening) = if t < OPEN_MS {
        (t as f32 / OPEN_MS as f32, true)
    } else if t < OPEN_MS + hold {
        (1.0, false)
    } else {
        (1.0 - (t - OPEN_MS - hold) as f32 / CLOSE_MS as f32, false)
    };
    let closing = t >= OPEN_MS + hold;
    let progress = if t < OPEN_MS { 0.0 } else { clamp01((t - OPEN_MS) as f32 / hold.max(1) as f32) };
    // obsah naskočí v posledních 40 % otevření a zmizí v prvních 40 % zavření
    let settled = !opening && !closing;
    let alpha = if settled { 1.0 } else { span(open, 0.6, 1.0) };
    let (bw, bh) = base;
    let f = match style {
        // rozbalení: nejdřív do šířky, pak (s mírným překmitem) dolů do kartičky; zpátky naopak
        NotchStyle::Unfold => {
            let (kw, kh) = if opening {
                (ease_out_cubic(span(open, 0.0, 0.55)), ease_out_back(span(open, 0.3, 1.0)))
            } else {
                // zavírání: nejdřív nahoru, pak do úzka
                (ease_out_cubic(span(open, 0.0, 0.6)), ease_out_cubic(span(open, 0.4, 1.0)))
            };
            Frame { w: lerp(bw, tw, kw), h: lerp(bh, th, kh), drop_y: None, alpha, glow: 0.0, progress, settled }
        }
        // do stran: jen šířka, výška zůstává (proužek se „nadechne“ do stran)
        NotchStyle::Sides => {
            let k = if opening { ease_out_back(open) } else { ease_out_cubic(open) };
            Frame { w: lerp(bw, tw, k), h: lerp(bh, th, k), drop_y: None, alpha, glow: 0.0, progress, settled }
        }
        // kapka: bublina vyjede zpod proužku, dopadne s překmitem; při zavření se vrátí nahoru a scvrkne
        NotchStyle::Drop => {
            let fall = if opening { ease_out_back(span(open, 0.0, 0.8)) } else { ease_out_cubic(open) };
            let grow = if opening { ease_out_cubic(span(open, 0.25, 1.0)) } else { open };
            let y = lerp(-th, DROP_GAP, fall);
            Frame { w: lerp(th, tw, grow), h: th, drop_y: Some(y), alpha, glow: 0.0, progress, settled }
        }
        // ticker: rozšíří se jako „do stran“, text projede během držení (`progress`)
        NotchStyle::Ticker => {
            let k = ease_out_cubic(open);
            Frame { w: lerp(bw, tw, k), h: lerp(bh, th, k), drop_y: None, alpha, glow: 0.0, progress, settled }
        }
        // jen záře: tvar zůstává, kolem dýchá záře (3 nádechy za držení)
        NotchStyle::Glow => {
            let breath = 0.55 + 0.45 * (progress * 3.0 * std::f32::consts::TAU).cos();
            let glow = if settled { breath } else { open };
            Frame { w: lerp(bw, tw, ease_out_cubic(open)), h: lerp(bh, th, ease_out_cubic(open)), drop_y: None, alpha, glow, progress, settled }
        }
    };
    Some(f)
}

/// Celková délka oznámení (ms).
pub fn total_ms(hold: u64) -> u64 {
    OPEN_MS + hold + CLOSE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: (f32, f32) = (120.0, 28.0);
    const HOLD: u64 = 4000;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.5
    }

    #[test]
    fn start_hold_end_table() {
        for style in NotchStyle::ALL {
            let (tw, th) = target(style, BASE);
            // začátek: tvar = proužek (u kapky bublina schovaná pod ním), obsah neviditelný
            let f0 = frame(style, 0, HOLD, BASE).unwrap();
            assert_eq!(f0.alpha, 0.0, "{style:?} obsah na začátku");
            assert!(!f0.settled, "{style:?}");
            match f0.drop_y {
                Some(y) => assert!(y < 0.0, "{style:?} kapka začíná pod proužkem"),
                None => assert!(near(f0.w, BASE.0) && near(f0.h, BASE.1), "{style:?} start {f0:?}"),
            }
            // uprostřed držení: cílový tvar, plný obsah, stojí
            let fm = frame(style, OPEN_MS + HOLD / 2, HOLD, BASE).unwrap();
            assert!(near(fm.w, tw) && near(fm.h, th) && fm.alpha == 1.0 && fm.settled, "{style:?} držení {fm:?}");
            assert!(near(fm.progress, 0.5), "{style:?} postup");
            // těsně před koncem: skoro zavřené, obsah pryč
            let fe = frame(style, total_ms(HOLD) - 1, HOLD, BASE).unwrap();
            assert_eq!(fe.alpha, 0.0, "{style:?} obsah na konci");
            assert!(!fe.settled);
            // konec = None
            assert!(frame(style, total_ms(HOLD), HOLD, BASE).is_none(), "{style:?} konec");
            assert!(frame(style, u64::MAX, HOLD, BASE).is_none());
        }
    }

    #[test]
    fn unfold_widens_before_it_drops() {
        // ve třetině otevření je už znatelně širší, ale ještě ne vyšší
        let f = frame(NotchStyle::Unfold, OPEN_MS / 4, HOLD, BASE).unwrap();
        assert!(f.w > BASE.0 + 40.0, "{f:?}");
        assert!(near(f.h, BASE.1), "{f:?}");
        // na konci otevření překmit dolů jen mírný (< 15 %)
        let peak = (0..=OPEN_MS).map(|t| frame(NotchStyle::Unfold, t, HOLD, BASE).unwrap().h).fold(0.0, f32::max);
        assert!((84.0..84.0 * 1.15).contains(&peak), "{peak}");
    }

    #[test]
    fn sides_keeps_height_and_drop_falls_below() {
        for t in (0..total_ms(HOLD)).step_by(37) {
            let s = frame(NotchStyle::Sides, t, HOLD, BASE).unwrap();
            assert!(near(s.h, 28.0), "do stran nemění výšku ({t} ms)");
            let d = frame(NotchStyle::Drop, t, HOLD, BASE).unwrap();
            assert!(d.drop_y.is_some() && near(d.h, 48.0), "kapka má stálou výšku ({t} ms)");
        }
        let d = frame(NotchStyle::Drop, OPEN_MS + 10, HOLD, BASE).unwrap();
        assert!(near(d.drop_y.unwrap(), DROP_GAP));
    }

    #[test]
    fn glow_breathes_only_while_holding() {
        let g: Vec<f32> = (0..30).map(|i| frame(NotchStyle::Glow, OPEN_MS + i * HOLD / 30, HOLD, BASE).unwrap().glow).collect();
        let (lo, hi) = g.iter().fold((1.0f32, 0.0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(lo < 0.25 && hi > 0.95, "nádech {lo}..{hi}");
        assert!(g.iter().all(|x| (0.0..=1.0).contains(x)));
        assert!(frame(NotchStyle::Glow, 0, HOLD, BASE).unwrap().glow == 0.0);
    }

    #[test]
    fn tiny_or_hidden_base_still_opens() {
        // skrytý styl proužku: tělo (0, 0) → tvar vyroste z ničeho a stejně dojde do cíle
        for style in NotchStyle::ALL {
            let f = frame(style, OPEN_MS + 1, HOLD, (0.0, 0.0)).unwrap();
            let (tw, th) = target(style, (0.0, 0.0));
            assert!(near(f.w, tw) && near(f.h, th), "{style:?} {f:?}");
            assert!(tw > 20.0 && th >= 26.0);
        }
    }
}
