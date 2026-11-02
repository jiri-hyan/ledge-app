//! Výzdoba vlastního zavřeného Ledge (kolo 25, `strip.compose.deco` / `fill` / `sep`): stálý akcent hrany,
//! pozadí těla a oddělení dílů. Čisté funkce: ořez (`Deco::norm` …), barvy ze stavu (`paint` → `Paint` pro
//! `stripcompose`), šířky zón s oddělovači (`joint`, `zone_w`, `content_w`) a co se vejde (`fit` — díly se
//! schovávají od konce zóny, nikdy se neořízne text). Výchozí hodnoty = dnešní vzhled (černé tělo, bez linky,
//! díly jen s mezerou). Id hlídá `compose_tests::ts_mirror`.

use super::{Facts, IDLE, OK, RUN, State, WAIT, state_of};
use crate::appearance::geom::parse_hex;
use crate::strip::Rgba;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Stálý akcent těla (obyčejné komentáře: popisy jdou do schématu MCP).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeEdge")]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    #[default]
    None,
    // stálá spodní linka
    Underline,
    // obrys těla
    Outline,
    // měkká záře pod tělem
    Glow,
    // spodní linka přechodem dvou barev
    Gradient,
    // kolo 28 (`edgefx`): přechod 2–3 barev, který teče podél hrany
    Flow,
    // čárky; ve stavu limitu naplněné poměrem limitu
    Dashes,
    // světelný bod běží po hraně
    Comet,
    // celá římsa se rozsvítí: měkký svit pod celým proužkem
    Light,
    // pomalý puls jasu
    Breath,
    // obrys těla přechodem barev
    OutlineGradient,
    // jednorázové naplnění zleva doprava (hotovo)
    Sweep,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeEdgeLen")]
#[serde(rename_all = "camelCase")]
pub enum EdgeLen {
    // střed 40 %
    #[default]
    Center,
    // široká 80 %
    Wide,
    Full,
    // vlastní % (`pct`)
    Custom,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeFillKind")]
#[serde(rename_all = "camelCase")]
pub enum FillKind {
    #[default]
    Black,
    // poloprůhledné tmavé sklo
    Glass,
    // tmavý odstín akcentu
    Tint,
    Gradient,
    // bez těla, jen obsah se stínem
    None,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeFillDir")]
#[serde(rename_all = "camelCase")]
pub enum FillDir {
    #[default]
    Right,
    Down,
    Diag,
}

// Jak se díly od sebe oddělují.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeSep")]
#[serde(rename_all = "camelCase")]
pub enum Sep {
    #[default]
    None,
    // větší mezera
    Gap,
    // tečka ·
    Dot,
    // svislá čárka
    Bar,
    // každý díl ve vlastní kapsli
    Pills,
    // zóny vlevo / střed / vpravo jako tři ostrůvky
    Islands,
}

/// Akcent hrany vlastního vzhledu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(rename = "ComposeDeco")]
#[serde(default, rename_all = "camelCase")]
pub struct Deco {
    pub edge: Edge,
    pub len: EdgeLen,
    // accent | claude (barva podle stavu Clauda) | sada (aurora, sunset, ocean, embers, clay) | #rrggbb
    pub color: String,
    // druhá barva přechodu (#rrggbb, prázdné = světlejší první)
    pub color2: String,
    // tloušťka 1–3 px
    pub width: u8,
    // třetí barva přechodu (#rrggbb, prázdné = jen dvě)
    pub color3: String,
    // délka linky v % u `len: custom` (10–100)
    pub pct: u8,
    // rychlost animace 1–5
    pub speed: u8,
    // síla svitu 1–3
    pub power: u8,
    // hýbat se i v klidu (stojí CPU); jinak jen když Claude pracuje / čeká
    pub always: bool,
    // živá hrana podle stavu Clauda
    pub live: super::edgefx::EdgeLive,
}

impl Default for Deco {
    fn default() -> Self {
        Self {
            edge: Edge::None,
            len: EdgeLen::Center,
            color: "accent".into(),
            color2: String::new(),
            width: 2,
            color3: String::new(),
            pct: 40,
            speed: 3,
            power: 2,
            always: false,
            live: super::edgefx::EdgeLive::default(),
        }
    }
}

/// Pozadí těla.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(rename = "ComposeFill")]
#[serde(default, rename_all = "camelCase")]
pub struct Fill {
    pub kind: FillKind,
    // barvy přechodu (#rrggbb; prázdné = tmavý akcent → černá)
    pub from: String,
    pub to: String,
    pub dir: FillDir,
    // prostřední barva přechodu (#rrggbb, prázdné = jen dvě)
    pub via: String,
    // přechod se pomalu přelévá (jen když se hýbe hrana: Claude pracuje / čeká, nebo `deco.always`)
    pub flow: bool,
}

/// Barva hrany: `accent`, `claude`, `#rrggbb`; jiné = `accent`.
pub fn norm_edge_color(s: &str) -> String {
    let s = s.trim().to_ascii_lowercase();
    if s == "claude" || parse_hex(&s).is_some() || super::edgefx::set_colors(&s).is_some() { s } else { "accent".into() }
}
/// Barva dílu: `""` (podle hodnoty, dnešní), `accent`, `#rrggbb`; jiné = `""`.
pub fn norm_piece_color(s: &str) -> String {
    let s = s.trim().to_ascii_lowercase();
    if s == "accent" || parse_hex(&s).is_some() { s } else { String::new() }
}
fn norm_hex(s: &str) -> String {
    let s = s.trim().to_ascii_lowercase();
    if parse_hex(&s).is_some() { s } else { String::new() }
}

impl Deco {
    pub fn norm(&self) -> Deco {
        Deco {
            edge: self.edge,
            len: self.len,
            color: norm_edge_color(&self.color),
            color2: norm_hex(&self.color2),
            width: self.width.clamp(1, 3),
            color3: norm_hex(&self.color3),
            pct: self.pct.clamp(10, 100),
            speed: self.speed.clamp(1, 5),
            power: self.power.clamp(1, 3),
            always: self.always,
            live: self.live,
        }
    }
}
impl Fill {
    pub fn norm(&self) -> Fill {
        Fill { kind: self.kind, from: norm_hex(&self.from), to: norm_hex(&self.to), dir: self.dir, via: norm_hex(&self.via), flow: self.flow }
    }
}

/// Výzdoba s barvami ze stavu (pro kreslení; část `Show`, ať změna stavu Clauda překreslí).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    pub edge: Edge,
    /// délka linky jako díl šířky (vystředěná); `None` = celá šířka (`edgefx::span`)
    pub span: Option<f32>,
    pub c1: Rgba,
    pub c2: Rgba,
    /// třetí barva (sada, vlastní `color3`)
    pub c3: Option<Rgba>,
    /// tloušťka linky (logické px)
    pub width: f32,
    pub fill: FillKind,
    pub f1: Rgba,
    pub f2: Rgba,
    pub dir: FillDir,
    pub sep: Sep,
    /// prostřední barva přechodu pozadí a jeho přelévání
    pub f3: Option<Rgba>,
    pub fill_flow: bool,
    /// živá hrana: síla, rychlost, stav Clauda, čas snímku (`edgefx`)
    pub fx: super::edgefx::Fx,
}

impl Paint {
    /// Dnešní vzhled: černé tělo, bez linky, díly jen s mezerou (kreslí se přesně jako před kolem 25).
    #[cfg(test)]
    pub fn plain(&self) -> bool {
        self.edge == Edge::None && self.fill == FillKind::Black && self.sep == Sep::None
    }
}

/// Barva Clauda podle stavu: čeká = jantar, hotovo = zelená, pracuje = modrá, klid = šedá.
pub fn claude_color(f: &Facts) -> Rgba {
    match state_of(f) {
        State::Wait => WAIT,
        State::Done => OK,
        State::Work => RUN,
        _ => IDLE,
    }
}

/// Tmavý odstín barvy (pozadí „tint“): 22 % barvy na černé.
pub fn dark(c: Rgba) -> Rgba {
    Rgba(c.0 * 0.22, c.1 * 0.22, c.2 * 0.22, 1.0)
}
/// Světlejší odstín (druhá barva přechodu linky, když chybí).
pub fn light(c: Rgba) -> Rgba {
    let m = |x: f32| x + (1.0 - x) * 0.55;
    Rgba(m(c.0), m(c.1), m(c.2), c.3)
}

/// Sklo: tmavá modrošedá na 62 % (vrstvené okno ukáže plochu pod ním).
pub const GLASS: Rgba = Rgba(0.09, 0.10, 0.13, 0.62);

/// Výzdoba → barvy pro kreslení (čistá funkce; `Facts.accent` = akcent motivu).
pub fn paint(d: &Deco, fill: &Fill, sep: Sep, f: &Facts) -> Paint {
    let fill = fill.norm();
    let (state, frac, busy) = super::edgefx::live_state(f);
    let grad = fill.kind == FillKind::Gradient;
    let (f1, f2) = match fill.kind {
        FillKind::Black => (Rgba(0.0, 0.0, 0.0, 1.0), Rgba(0.0, 0.0, 0.0, 1.0)),
        FillKind::Glass => (GLASS, GLASS),
        FillKind::Tint => (dark(f.accent), dark(f.accent)),
        FillKind::Gradient => (parse_hex(&fill.from).unwrap_or_else(|| dark(f.accent)), parse_hex(&fill.to).unwrap_or(Rgba(0.0, 0.0, 0.0, 1.0))),
        FillKind::None => (Rgba(0.0, 0.0, 0.0, 0.0), Rgba(0.0, 0.0, 0.0, 0.0)),
    };
    let fx = super::edgefx::Fx { state, frac, busy, ..super::edgefx::Fx::new(f.accent) };
    let (f3, fill_flow) = (parse_hex(&fill.via).filter(|_| grad), fill.flow && grad);
    let mut p = Paint { edge: Edge::None, span: None, c1: f.accent, c2: f.accent, c3: None, width: 2.0, fill: fill.kind, f1, f2, dir: fill.dir, sep, f3, fill_flow, fx };
    restyle(&mut p, d, claude_color(f));
    p
}

/// Hrana z `deco` do barev pro kreslení (`claude` = barva podle stavu); i pro „Vyzkoušet“ s rozpracovaným `deco`.
pub fn restyle(p: &mut Paint, d: &Deco, claude: Rgba) {
    let d = d.norm();
    (p.c1, p.c2, p.c3) = super::edgefx::colors(&d.color, &d.color2, &d.color3, p.fx.accent, claude);
    p.edge = d.edge;
    p.span = super::edgefx::span(d.len, d.pct);
    p.width = d.width as f32;
    (p.fx.speed, p.fx.power, p.fx.always, p.fx.live) = (d.speed, d.power, d.always, d.live);
}

/// Mezera mezi dvěma díly zóny (px) podle oddělení; `gap` = dnešní mezera mezi díly, `ks` = měřítko.
pub fn joint(sep: Sep, gap: f32, ks: f32) -> f32 {
    match sep {
        Sep::None | Sep::Islands => gap,
        Sep::Gap => gap * 2.5,
        Sep::Dot => gap * 2.0 + 3.0 * ks,
        Sep::Bar => gap * 2.0 + 1.0 * ks,
        Sep::Pills => gap * 0.6,
    }
}

/// Vnitřní okraj kapsle dílu (vlevo i vpravo) u `pills` a stylu „podklad“.
pub fn chip_pad(gap: f32) -> f32 {
    gap * 0.75
}

/// Šířka zóny z šířek dílů a mezery mezi nimi.
pub fn zone_w(ws: &[f32], joint: f32) -> f32 {
    ws.iter().sum::<f32>() + joint * ws.len().saturating_sub(1) as f32
}

/// Šířka obsahu ze šířek zón (vlevo, střed, vpravo): střed na ose těla, boky stejně široké (širší z nich) —
/// stejně jako před kolem 25. U ostrůvků má každá zóna vlastní tělo (`pad2` = oba okraje) a mezi nimi `igap`.
pub fn content_w(z: [f32; 3], gap: f32, islands: Option<(f32, f32)>) -> f32 {
    let (z, gap) = match islands {
        Some((pad2, igap)) => (z.map(|w| if w > 0.0 { w + pad2 } else { 0.0 }), igap),
        None => (z, gap),
    };
    let [l, c, r] = z;
    if c > 0.0 {
        let side = l.max(r);
        c + if side > 0.0 { 2.0 * (side + gap) } else { 0.0 }
    } else {
        l + r + if l > 0.0 && r > 0.0 { gap } else { 0.0 }
    }
}

/// Kolik dílů z každé zóny se vejde do `avail` (px obsahu). Schovává se od konce zóny (nejnižší priorita);
/// nejdřív z širšího boku (při shodě vpravo), střed až nakonec; aspoň jeden díl zůstane vždy.
pub fn fit(zones: [&[f32]; 3], joint: f32, gap: f32, islands: Option<(f32, f32)>, avail: f32) -> [usize; 3] {
    let mut n = zones.map(|z| z.len());
    let width = |n: &[usize; 3]| content_w([0, 1, 2].map(|i| zone_w(&zones[i][..n[i]], joint)), gap, islands);
    while width(&n) > avail && n.iter().sum::<usize>() > 1 {
        let side = |i: usize| zone_w(&zones[i][..n[i]], joint);
        let pick = if n[0] > 0 && (n[2] == 0 || side(0) > side(2)) {
            0
        } else if n[2] > 0 {
            2
        } else {
            1
        };
        n[pick] -= 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_norm_table() {
        for (raw, edge, piece) in [
            ("accent", "accent", "accent"),
            ("Claude", "claude", ""),
            ("#FF8800", "#ff8800", "#ff8800"),
            ("", "accent", ""),
            ("modrá", "accent", ""),
            ("#12345", "accent", ""),
        ] {
            assert_eq!(norm_edge_color(raw), edge, "{raw}");
            assert_eq!(norm_piece_color(raw), piece, "{raw}");
        }
        let d = Deco { width: 9, color2: "x".into(), ..Deco::default() }.norm();
        assert_eq!((d.width, d.color2.as_str()), (3, ""));
        assert_eq!(Deco { width: 0, ..Deco::default() }.norm().width, 1);
    }

    #[test]
    fn paint_from_state_and_defaults() {
        let f = |waiting, working, done| Facts { waiting, working, done, accent: Rgba::VIOLET, ..Facts::default() };
        let claude = Deco { edge: Edge::Underline, color: "claude".into(), ..Deco::default() };
        for (fx, want) in [(f(1, 0, 0), WAIT), (f(0, 0, 1), OK), (f(0, 1, 0), RUN), (f(0, 0, 0), IDLE)] {
            assert_eq!(paint(&claude, &Fill::default(), Sep::None, &fx).c1, want);
        }
        // výchozí = dnešek
        let p = paint(&Deco::default(), &Fill::default(), Sep::None, &f(0, 0, 0));
        assert!(p.plain());
        assert_eq!((p.c1, p.f1), (Rgba::VIOLET, Rgba(0.0, 0.0, 0.0, 1.0)));
        // vlastní hex, druhá barva chybí = světlejší první; tint = tmavý akcent
        let p = paint(&Deco { color: "#ff0000".into(), ..Deco::default() }, &Fill { kind: FillKind::Tint, ..Fill::default() }, Sep::Dot, &f(0, 0, 0));
        assert_eq!(p.c1, Rgba(1.0, 0.0, 0.0, 1.0));
        assert_eq!(p.c2, light(p.c1));
        assert_eq!(p.f1, dark(Rgba::VIOLET));
        assert!(!p.plain());
        let g = paint(&Deco::default(), &Fill { kind: FillKind::Gradient, from: "#000080".into(), dir: FillDir::Diag, ..Fill::default() }, Sep::None, &f(0, 0, 0));
        assert_eq!((g.f1.2 > 0.4, g.f2), (true, Rgba(0.0, 0.0, 0.0, 1.0)));
        assert_eq!(paint(&Deco::default(), &Fill { kind: FillKind::None, ..Fill::default() }, Sep::None, &f(0, 0, 0)).f1.3, 0.0);
    }

    #[test]
    fn joint_and_content_table() {
        // bez oddělení = dnešní mezera; ostatní širší, kapsle těsnější (mají vlastní okraj)
        assert_eq!(joint(Sep::None, 8.0, 1.0), 8.0);
        assert!(joint(Sep::Gap, 8.0, 1.0) > joint(Sep::Dot, 8.0, 1.0));
        assert!(joint(Sep::Pills, 8.0, 1.0) < 8.0);
        assert_eq!(zone_w(&[10.0, 20.0, 30.0], 5.0), 70.0);
        assert_eq!(zone_w(&[], 5.0), 0.0);
        // (zóny, ostrůvky, čekaná šířka)
        for (z, isl, want) in [
            ([10.0, 40.0, 20.0], None, 40.0 + 2.0 * (20.0 + 8.0)),
            ([10.0, 0.0, 20.0], None, 38.0),
            ([10.0, 0.0, 0.0], None, 10.0),
            ([0.0, 40.0, 0.0], None, 40.0),
            ([10.0, 40.0, 20.0], Some((12.0, 6.0)), 52.0 + 2.0 * (32.0 + 6.0)),
            ([10.0, 0.0, 20.0], Some((12.0, 6.0)), 22.0 + 32.0 + 6.0),
        ] {
            assert_eq!(content_w(z, 8.0, isl), want, "{z:?} {isl:?}");
        }
    }

    #[test]
    fn old_config_keeps_look() {
        // config z kola 25 (bez klíčů kola 28): záře, střed 40 %, barva podle Clauda, 1 px — stejná barva i tvar, bez pohybu
        let d: Deco = serde_json::from_str(r#"{"edge":"glow","len":"center","color":"claude","width":1}"#).unwrap();
        let f = Facts { working: 1, accent: Rgba::VIOLET, ..Facts::default() };
        let p = paint(&d, &Fill::default(), Sep::None, &f);
        assert_eq!((p.edge, p.span, p.c1, p.c2, p.c3, p.width), (Edge::Glow, Some(0.4), RUN, light(RUN), None, 1.0));
        assert_eq!((p.fx.power, p.fx.live.on, p.fx.always, p.fill_flow, p.f3), (2, false, false, false, None));
        assert!(!super::super::edgefx::frames(&p, false, false, true));
        assert_eq!(super::super::edgefx::apply(&p, None), p);
        let fl: Fill = serde_json::from_str(r##"{"kind":"gradient","from":"#112233","to":"#000000"}"##).unwrap();
        assert_eq!((fl.via.as_str(), fl.flow), ("", false));
        // nové klíče: ořez rozsahů, sada barev, neznámá sada = akcent
        let n = Deco { pct: 0, speed: 9, power: 0, color: "Aurora".into(), color3: "#ABCDEF".into(), ..Deco::default() }.norm();
        assert_eq!((n.pct, n.speed, n.power, n.color.as_str(), n.color3.as_str()), (10, 5, 1, "aurora", "#abcdef"));
        assert_eq!(norm_edge_color("duha"), "accent");
        // přelévaná výplň jen u přechodu
        let g = paint(&Deco::default(), &Fill { kind: FillKind::Glass, flow: true, via: "#ffffff".into(), ..Fill::default() }, Sep::None, &f);
        assert_eq!((g.fill_flow, g.f3), (false, None));
    }

    #[test]
    fn fit_hides_from_zone_end() {
        let (l, c, r) = ([30.0, 30.0, 30.0], [50.0, 50.0], [30.0, 30.0]);
        let z: [&[f32]; 3] = [&l, &c, &r];
        // vejde se všechno
        assert_eq!(fit(z, 4.0, 4.0, None, 10_000.0), [3, 2, 2]);
        // (dostupná šířka, čekané počty): nejdřív širší bok, střed naposled, aspoň jeden díl
        let full = content_w([98.0, 104.0, 64.0], 4.0, None);
        for (avail, want) in [(full, [3, 2, 2]), (full - 1.0, [2, 2, 2]), (104.0 + 2.0 * 68.0, [2, 2, 2]), (104.0 + 2.0 * 67.0, [1, 2, 1]), (104.0, [0, 2, 0]), (50.0, [0, 1, 0]), (1.0, [0, 1, 0])] {
            assert_eq!(fit(z, 4.0, 4.0, None, avail), want, "{avail}");
        }
        // jen boky: ubírá se z širšího, při shodě vpravo
        let z2: [&[f32]; 3] = [&[20.0, 20.0], &[], &[20.0, 20.0]];
        assert_eq!(fit(z2, 4.0, 4.0, None, 70.0), [2, 0, 1]);
    }
}
