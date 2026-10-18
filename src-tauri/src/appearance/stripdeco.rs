//! Ozdoby proužku podle stylu (`appearance.strip.style`) jako čistá data pro `strip::draw`: šířka
//! okraje, barva okraje z akcentu (neon), tvrdý / měkký stín, záře linky, písmo a tvar podpisové
//! linky. Výška a rádius jsou v `geom::strip_metrics`. Ozdoby jsou statické — proužek se kreslí jen
//! při změně obsahu (žádný časovač navíc).

use super::StripStyle;

/// Písmo textu na proužku (rodina pro DirectWrite: `family`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripFont {
    Sans,
    Mono,
    Serif,
    Grotesk,
}

impl StripFont {
    /// Systémové písmo Windows; chybí-li, DirectWrite vezme náhradní.
    pub fn family(self) -> &'static str {
        match self {
            StripFont::Sans => "Segoe UI Variable Text",
            StripFont::Mono => "Cascadia Mono",
            StripFont::Serif => "Georgia",
            StripFont::Grotesk => "Bahnschrift",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripDeco {
    /// tloušťka okraje (logické px)
    pub edge_w: f32,
    /// okraj v barvě akcentu (neonová linka) místo okraje motivu
    pub edge_accent: bool,
    /// tvrdý stín posunutý doprava dolů o tolik px (0 = žádný)
    pub hard_shadow: f32,
    /// měkký stín pod tělem, dosah v px (0 = žádný)
    pub soft_shadow: f32,
    /// záře kolem podpisové linky (dosah v px, 0 = žádná)
    pub glow: f32,
    pub font: StripFont,
    /// podíl šířky podpisové linky v klidu (`None` = dnešní chování podle stylu a stavu)
    pub line_share: Option<f32>,
    /// výška podpisové linky (logické px)
    pub line_h: f32,
}

impl StripDeco {
    /// Kolik px kolem těla zabírají stíny a záře: (vlevo, vpravo, dole). Okno proužku je o tolik
    /// větší, tělo zůstává na ose (posun doleva o `left`).
    pub fn extents(&self) -> (f32, f32, f32) {
        let soft = self.soft_shadow;
        (soft, soft.max(self.hard_shadow), soft.max(self.hard_shadow).max(self.glow))
    }
}

pub fn strip_deco(style: StripStyle) -> StripDeco {
    let base = StripDeco { edge_w: 1.0, edge_accent: false, hard_shadow: 0.0, soft_shadow: 0.0, glow: 0.0, font: StripFont::Sans, line_share: None, line_h: 2.0 };
    match style {
        StripStyle::Pill | StripStyle::Line | StripStyle::Dot => base,
        StripStyle::Term => StripDeco { font: StripFont::Mono, line_share: Some(1.0), line_h: 1.0, ..base },
        StripStyle::Tag => StripDeco { font: StripFont::Serif, line_share: Some(0.18), soft_shadow: 3.0, ..base },
        StripStyle::Glow => StripDeco { edge_accent: true, glow: 5.0, line_share: Some(0.7), ..base },
        StripStyle::Block => StripDeco { edge_w: 2.0, hard_shadow: 3.0, font: StripFont::Grotesk, line_share: Some(1.0), line_h: 3.0, ..base },
        StripStyle::Drop => StripDeco { edge_w: 0.0, soft_shadow: 6.0, line_share: Some(0.3), ..base },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deco_table() {
        // dnešní styly beze změny kreslení
        for s in [StripStyle::Pill, StripStyle::Line, StripStyle::Dot] {
            let d = strip_deco(s);
            assert_eq!((d.edge_w, d.line_share, d.extents(), d.font), (1.0, None, (0.0, 0.0, 0.0), StripFont::Sans), "{s:?}");
        }
        // (styl, písmo, má stín/záři mimo tělo)
        let cases = [
            (StripStyle::Term, StripFont::Mono, false),
            (StripStyle::Tag, StripFont::Serif, true),
            (StripStyle::Glow, StripFont::Sans, true),
            (StripStyle::Block, StripFont::Grotesk, true),
            (StripStyle::Drop, StripFont::Sans, true),
        ];
        for (s, font, outside) in cases {
            let d = strip_deco(s);
            assert_eq!(d.font, font, "{s:?}");
            let (l, r, b) = d.extents();
            assert_eq!(l + r + b > 0.0, outside, "{s:?}");
            assert!(d.line_share.is_some_and(|x| (0.0..=1.0).contains(&x)), "{s:?}");
        }
        // tvrdý stín jen doprava dolů: vlevo nic
        assert_eq!(strip_deco(StripStyle::Block).extents(), (0.0, 3.0, 3.0));
    }
}
