//! Živá hrana vlastního zavřeného Ledge (kolo 28, `strip.compose.deco`): barevné sady, délka linky, stav
//! Clauda → efekt (`EdgeLive`), fáze animací (přelévání, dech, kometa, naplnění, čárky), kdy běží snímky a jakou
//! rezervu potřebuje okno pro svit pod tělem. Čisté funkce: čas (`ms`) dodá vlákno proužku (`stripfx::prepare`)
//! jen během animace, jinak se kreslí statický snímek. Kreslí `stripfx`. Id hlídá `ts_mirror` (`src/strip/edgefx.ts`).

use super::composedeco::{Edge, EdgeLen, Paint, light};
use super::{Facts, IDLE, OK, RUN, State, WAIT, state_of};
use crate::appearance::geom::parse_hex;
use crate::strip::Rgba;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Dosah záře pod tělem (logické px) při síle 2 — o tolik je okno vyšší.
pub const GLOW_PX: f32 = 8.0;
/// Dosah rozsvícení celé římsy (logické px) při síle 2.
pub const LIGHT_PX: f32 = 14.0;
/// Jednorázové naplnění „hotovo“ (ms).
pub const SWEEP_MS: u64 = 1200;
/// Jak dlouho hraje „Vyzkoušet“ z editoru (ms).
pub const TRY_MS: u64 = 4000;
/// Od kolika % limitu (5h nebo týden) je stav „limit“.
pub const LIMIT_AT: f64 = 80.0;

/// Stav Clauda pro živou hranu (pořadí priority: čeká > hotovo > limit > pracuje > klid).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LiveState {
    #[default]
    Idle,
    Work,
    Wait,
    Done,
    Limit,
}

impl LiveState {
    pub const ALL: [LiveState; 5] = [LiveState::Idle, LiveState::Work, LiveState::Wait, LiveState::Done, LiveState::Limit];
    pub fn id(self) -> &'static str {
        match self {
            LiveState::Idle => "idle",
            LiveState::Work => "work",
            LiveState::Wait => "wait",
            LiveState::Done => "done",
            LiveState::Limit => "limit",
        }
    }
    pub fn from_id(s: &str) -> Option<LiveState> {
        Self::ALL.into_iter().find(|x| x.id() == s)
    }
}

/// Živá hrana podle stavu Clauda (`strip.compose.deco.live`). `none` u stavu = jen stálá hrana (`deco.edge`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[schemars(rename = "ComposeLive")]
#[serde(default, rename_all = "camelCase")]
pub struct EdgeLive {
    // zapnuto (výchozí vypnuto = jen stálá hrana)
    pub on: bool,
    pub work: Edge,
    pub wait: Edge,
    pub done: Edge,
    // limit 5h nebo týdne od 80 %
    pub limit: Edge,
}

impl Default for EdgeLive {
    fn default() -> Self {
        Self { on: false, work: Edge::Flow, wait: Edge::Breath, done: Edge::Sweep, limit: Edge::Dashes }
    }
}

/// Co je potřeba ke kreslení efektu (část `Paint`): nastavení z configu, stav a čas snímku.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fx {
    /// rychlost 1–5, síla 1–3
    pub speed: u8,
    pub power: u8,
    /// hýbat se i v klidu (stojí CPU)
    pub always: bool,
    pub live: EdgeLive,
    pub state: LiveState,
    /// poměr limitu 0–1 (jen čerstvá data), jinak `None`
    pub frac: Option<f32>,
    /// Claude pracuje nebo čeká — jen tehdy se efekty hýbou (bez `always`)
    pub busy: bool,
    pub accent: Rgba,
    /// čas snímku (ms vlákna proužku) jen během animace; `None` = statický snímek
    pub ms: Option<u64>,
    /// průběh jednorázového naplnění 0–1 (1 = hotovo / bez animací)
    pub sweep: f32,
}

impl Fx {
    pub fn new(accent: Rgba) -> Fx {
        Fx { speed: 3, power: 2, always: false, live: EdgeLive::default(), state: LiveState::Idle, frac: None, busy: false, accent, ms: None, sweep: 1.0 }
    }
}

/// Stav Clauda z faktů: (stav, poměr limitu, pracuje / čeká). Limit = vyšší z čerstvých 5h / týden.
pub fn live_state(f: &Facts) -> (LiveState, Option<f32>, bool) {
    let lim = [f.limit5h, f.limit7d].into_iter().flatten().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |a| a.max(x))));
    let frac = lim.map(|x| (x / 100.0).clamp(0.0, 1.0) as f32);
    let st = match state_of(f) {
        State::Wait => LiveState::Wait,
        State::Done => LiveState::Done,
        _ if lim.is_some_and(|x| x >= LIMIT_AT) => LiveState::Limit,
        _ if f.working > 0 => LiveState::Work,
        _ => LiveState::Idle,
    };
    (st, frac, f.working > 0 || st == LiveState::Wait)
}

/// Barva Clauda pro stav (režim barvy `claude` u „Vyzkoušet“).
pub fn state_color(st: LiveState) -> Rgba {
    match st {
        LiveState::Work => RUN,
        LiveState::Wait => WAIT,
        LiveState::Done => OK,
        _ => IDLE,
    }
}

/// Barevné sady (lidské názvy v editoru): id → 3 barvy podél hrany.
pub const SETS: [(&str, [&str; 3]); 5] = [
    ("aurora", ["#34d399", "#22d3ee", "#a78bfa"]),
    ("sunset", ["#ffd166", "#f97316", "#e11d48"]),
    ("ocean", ["#38bdf8", "#2dd4bf", "#6366f1"]),
    ("embers", ["#fbbf24", "#f97316", "#dc2626"]),
    ("clay", ["#d97757", "#f0b38f", "#b85a3a"]),
];

pub fn set_colors(id: &str) -> Option<[Rgba; 3]> {
    let (_, h) = SETS.iter().find(|(k, _)| *k == id)?;
    Some([parse_hex(h[0])?, parse_hex(h[1])?, parse_hex(h[2])?])
}

/// Barvy hrany (už oříznuté `Deco::norm`): sada = 3 pevné; `accent` / `claude` / hex = první, druhá
/// (prázdná = světlejší první, jako před kolem 28) a volitelná třetí.
pub fn colors(color: &str, color2: &str, color3: &str, accent: Rgba, claude: Rgba) -> (Rgba, Rgba, Option<Rgba>) {
    if let Some([a, b, c]) = set_colors(color) {
        return (a, b, Some(c));
    }
    let c1 = match color {
        "claude" => claude,
        s => parse_hex(s).unwrap_or(accent),
    };
    (c1, parse_hex(color2).unwrap_or_else(|| light(c1)), parse_hex(color3))
}

/// Délka linky jako díl šířky těla (vystředěná); `None` = celá šířka mezi zaobleními.
pub fn span(len: EdgeLen, pct: u8) -> Option<f32> {
    match len {
        EdgeLen::Center => Some(0.4),
        EdgeLen::Wide => Some(0.8),
        EdgeLen::Full => None,
        EdgeLen::Custom if pct >= 100 => None,
        EdgeLen::Custom => Some(pct.max(10) as f32 / 100.0),
    }
}

/// Kde linka začíná a končí na těle `l..r` se zaoblením `radius`. Střed 40 % beze změny (30–70 %);
/// delší linka nevyjede přes zaoblené rohy.
pub fn span_x(span: Option<f32>, l: f32, r: f32, radius: f32) -> (f32, f32) {
    let w = r - l;
    let inset = radius.min(w / 2.0) * 0.8;
    match span {
        None => (l + inset, r - inset),
        Some(f) if f <= 0.4 => (l + w * (0.5 - f / 2.0), l + w * (0.5 + f / 2.0)),
        Some(f) => ((l + w * (0.5 - f / 2.0)).max(l + inset), (l + w * (0.5 + f / 2.0)).min(r - inset)),
    }
}

/// Efekt pro stav (`none` = stálá hrana).
pub fn state_edge(l: &EdgeLive, st: LiveState) -> Edge {
    match st {
        LiveState::Idle => Edge::None,
        LiveState::Work => l.work,
        LiveState::Wait => l.wait,
        LiveState::Done => l.done,
        LiveState::Limit => l.limit,
    }
}

/// Hrana, která se teď kreslí: živá hrana podle stavu (zapnutá, nebo „Vyzkoušet“ = `tried`), jinak stálá.
/// Čeká = jantar, hotovo = zelená, limit = jantar / od 95 % červená; pracuje = barvy hrany.
pub fn apply(p: &Paint, tried: Option<LiveState>) -> Paint {
    let st = match tried {
        Some(s) => s,
        None if p.fx.live.on => p.fx.state,
        None => return *p,
    };
    let e = state_edge(&p.fx.live, st);
    if e == Edge::None {
        return *p;
    }
    let mut q = *p;
    (q.edge, q.fx.state) = (e, st);
    let tone = match st {
        LiveState::Wait => Some(WAIT),
        LiveState::Done => Some(OK),
        LiveState::Limit if p.fx.frac.unwrap_or(0.0) >= 0.95 => Some(crate::strip::Rgba::RED),
        LiveState::Limit => Some(WAIT),
        _ => None,
    };
    if let Some(c) = tone {
        (q.c1, q.c2, q.c3) = (c, light(c), None);
    }
    if tried == Some(LiveState::Limit) && q.fx.frac.is_none() {
        // ukázka bez skutečných dat limitu: čárky naplněné ze 4/5 (jen „Vyzkoušet“)
        q.fx.frac = Some(0.8);
    }
    q
}

/// Hýbe se efekt v čase? (Naplnění má vlastní hodiny, ostatní statické.)
pub fn moving(e: Edge) -> bool {
    matches!(e, Edge::Flow | Edge::Dashes | Edge::Comet | Edge::Breath)
}

/// Potřebuje proužek snímky (~30 fps)? Jen běžící naplnění, „Vyzkoušet“, nebo pohyblivý efekt / přelévaná
/// výplň, když Claude pracuje či čeká (nebo `always`). Bez animací (`animate` = false) jen „Vyzkoušet“ (dohraje).
pub fn frames(p: &Paint, tried: bool, sweeping: bool, animate: bool) -> bool {
    if tried {
        return true;
    }
    animate && (sweeping || ((moving(p.edge) || p.fill_flow) && (p.fx.busy || p.fx.always)))
}

/// Násobek síly (1–3); síla 2 = vzhled před kolem 28.
pub fn power_k(power: u8) -> f32 {
    match power {
        0 | 1 => 0.7,
        2 => 1.0,
        _ => 1.4,
    }
}

/// Dosah svitu pod tělem (logické px) pro efekt; 0 = jen linka.
pub fn halo_px(e: Edge, power: u8) -> f32 {
    match e {
        Edge::Glow => GLOW_PX * power_k(power),
        Edge::Light => LIGHT_PX * power_k(power),
        Edge::Breath | Edge::Flow | Edge::Comet | Edge::Sweep => GLOW_PX,
        _ => 0.0,
    }
}

/// Rezerva okna pro svit (px vodorovně na každou stranu, px dole): největší ze stálé hrany a živých efektů
/// (okno nemění velikost se stavem). Záře je o 10 % širší než tělo (`rx = 0.55 × šířka`), tak i do stran.
pub fn reserve(p: &Paint, bw: f32, scale: f32) -> (i32, i32) {
    let l = &p.fx.live;
    let live = [l.work, l.wait, l.done, l.limit];
    let all = std::iter::once(p.edge).chain(if l.on { live.to_vec() } else { Vec::new() });
    let y = all.map(|e| halo_px(e, p.fx.power)).fold(0.0, f32::max);
    if y <= 0.0 {
        return (0, 0);
    }
    ((bw * 0.06).ceil() as i32 + 1, (y * scale).ceil() as i32)
}

/// Rozsah snímků ve stejném tempu: perioda (ms) pro rychlost 1–5.
fn period(base: u64, speed: u8) -> u64 {
    let k = [1.8, 1.35, 1.0, 0.72, 0.5][speed.clamp(1, 5) as usize - 1];
    (base as f32 * k) as u64
}

/// Posun přelévání 0–1 (statický snímek = 0).
pub fn flow_phase(ms: Option<u64>, speed: u8) -> f32 {
    ms.map_or(0.0, |t| {
        let p = period(3200, speed);
        (t % p) as f32 / p as f32
    })
}

/// Jas dechu 0,3–1 (statický = 0,85).
pub fn breath(ms: Option<u64>, speed: u8) -> f32 {
    ms.map_or(0.85, |t| {
        let p = period(3400, speed);
        let x = (t % p) as f32 / p as f32;
        0.3 + 0.7 * (0.5 - 0.5 * (x * std::f32::consts::TAU).cos())
    })
}

/// Kometa: poloha 0–1 a směr (±1) — tam a zpět (statický = uprostřed, doprava).
pub fn comet(ms: Option<u64>, speed: u8) -> (f32, f32) {
    ms.map_or((0.5, 1.0), |t| {
        let p = period(2200, speed);
        let x = (t % (2 * p)) as f32 / p as f32;
        if x < 1.0 { (x, 1.0) } else { (2.0 - x, -1.0) }
    })
}

/// Průběh naplnění po `elapsed` ms (zpomaluje ke konci), 0–1.
pub fn sweep(elapsed: u64) -> f32 {
    let x = (elapsed as f32 / SWEEP_MS as f32).clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

/// Kolik čárek na linku délky `len_px` (krok ~7 px × měřítko), 4–48.
pub fn dash_n(len_px: f32, scale: f32) -> usize {
    ((len_px / (7.0 * scale.max(0.5))).round() as usize).clamp(4, 48)
}

/// Krytí čárek: s limitem se plní poměrem (`frac`, nenaplněné slabě), bez něj všechny; při animaci
/// běží po čárkách světlá vlna (`phase` 0–1).
pub fn dashes(n: usize, frac: Option<f32>, phase: Option<f32>) -> Vec<f32> {
    let lit = frac.map_or(n, |f| (f.clamp(0.0, 1.0) * n as f32).round() as usize);
    (0..n)
        .map(|i| {
            if i >= lit {
                return 0.16;
            }
            match phase {
                None => 0.9,
                Some(ph) => {
                    let d = ((i as f32 + 0.5) / n as f32 - ph).rem_euclid(1.0);
                    0.45 + 0.55 * (1.0 - (d * 4.0).min(1.0))
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::looks::composedeco::{Deco, Fill, Sep, paint};

    fn facts(waiting: usize, working: usize, done: usize, l5: Option<f64>) -> Facts {
        Facts { waiting, working, done, limit5h: l5, accent: Rgba::VIOLET, ..Facts::default() }
    }

    #[test]
    fn live_state_priority_table() {
        // (čeká, pracuje, hotovo, limit 5h, čekaný stav, pracuje / čeká)
        for (w, k, d, l, want, busy) in [
            (0, 0, 0, None, LiveState::Idle, false),
            (0, 1, 0, None, LiveState::Work, true),
            (1, 1, 0, Some(90.0), LiveState::Wait, true),
            (0, 1, 1, None, LiveState::Done, true),
            (0, 1, 0, Some(85.0), LiveState::Limit, true),
            (0, 0, 0, Some(80.0), LiveState::Limit, false),
            (0, 0, 0, Some(79.9), LiveState::Idle, false),
        ] {
            let (st, frac, b) = live_state(&facts(w, k, d, l));
            assert_eq!((st, b), (want, busy), "{w} {k} {d} {l:?}");
            assert_eq!(frac, l.map(|x| (x / 100.0) as f32));
        }
        // týdenní limit taky; bez dat žádný poměr (nic nevymýšlí)
        let f = Facts { limit7d: Some(92.0), ..facts(0, 0, 0, Some(10.0)) };
        assert_eq!(live_state(&f).0, LiveState::Limit);
        assert_eq!(live_state(&facts(0, 0, 0, None)).1, None);
        for s in LiveState::ALL {
            assert_eq!(LiveState::from_id(s.id()), Some(s));
        }
        assert_eq!(LiveState::from_id("povolit"), None);
    }

    #[test]
    fn colors_and_span_table() {
        let a = Rgba::VIOLET;
        assert_eq!(colors("accent", "", "", a, RUN), (a, light(a), None));
        assert_eq!(colors("claude", "", "", a, WAIT).0, WAIT);
        let (c1, c2, c3) = colors("#ff0000", "#00ff00", "#0000ff", a, RUN);
        assert_eq!((c1, c2, c3), (Rgba(1.0, 0.0, 0.0, 1.0), Rgba(0.0, 1.0, 0.0, 1.0), Some(Rgba(0.0, 0.0, 1.0, 1.0))));
        for (id, _) in SETS {
            let (x, _, z) = colors(id, "", "", a, RUN);
            assert!(z.is_some() && x != a, "{id}");
        }
        // (délka, %, čekané x0, x1) na těle 0–100 se zaoblením 10 (okraj 8)
        for (len, pct, want) in [
            (EdgeLen::Center, 0, (30.0, 70.0)),
            (EdgeLen::Wide, 0, (10.0, 90.0)),
            (EdgeLen::Full, 0, (8.0, 92.0)),
            (EdgeLen::Custom, 60, (20.0, 80.0)),
            (EdgeLen::Custom, 100, (8.0, 92.0)),
            (EdgeLen::Custom, 98, (8.0, 92.0)),
        ] {
            let (x0, x1) = span_x(span(len, pct), 0.0, 100.0, 10.0);
            assert!((x0 - want.0).abs() < 0.01 && (x1 - want.1).abs() < 0.01, "{len:?} {pct}: {x0} {x1}");
        }
    }

    #[test]
    fn apply_live_table() {
        let mut d = Deco { edge: Edge::Underline, ..Deco::default() };
        let pf = |d: &Deco, w, k, dn, l| paint(d, &Fill::default(), Sep::None, &facts(w, k, dn, l));
        // vypnutá živá hrana = stálá
        assert_eq!(apply(&pf(&d, 0, 1, 0, None), None).edge, Edge::Underline);
        d.live.on = true;
        let f = |w, k, dn, l| pf(&d, w, k, dn, l);
        // (stav, čekaný efekt, první barva)
        for (p, want, c) in [
            (f(0, 0, 0, None), Edge::Underline, Rgba::VIOLET),
            (f(0, 1, 0, None), Edge::Flow, Rgba::VIOLET),
            (f(1, 0, 0, None), Edge::Breath, WAIT),
            (f(0, 0, 1, None), Edge::Sweep, OK),
            (f(0, 0, 0, Some(85.0)), Edge::Dashes, WAIT),
            (f(0, 0, 0, Some(97.0)), Edge::Dashes, Rgba::RED),
        ] {
            let q = apply(&p, None);
            assert_eq!((q.edge, q.c1), (want, c), "{:?}", p.fx.state);
        }
        // stav vypnutý = stálá hrana; „Vyzkoušet“ i s vypnutou živou hranou
        d.live.wait = Edge::None;
        assert_eq!(apply(&pf(&d, 1, 0, 0, None), None).edge, Edge::Underline);
        d.live.on = false;
        let q = apply(&pf(&d, 0, 0, 0, None), Some(LiveState::Limit));
        assert_eq!((q.edge, q.fx.frac), (Edge::Dashes, Some(0.8)));
        assert_eq!(apply(&pf(&d, 0, 0, 0, None), Some(LiveState::Done)).edge, Edge::Sweep);
    }

    #[test]
    fn frames_only_when_active() {
        let d = |edge, always| Deco { edge, always, ..Deco::default() };
        let p = |deco: &Deco, k| paint(deco, &Fill::default(), Sep::None, &facts(0, k, 0, None));
        // (efekt, stále, pracuje, „Vyzkoušet“, naplnění běží, animace, čekané snímky)
        for (e, al, k, tr, sw, an, want) in [
            (Edge::None, false, 0, false, false, true, false),
            (Edge::Glow, false, 1, false, false, true, false),
            (Edge::Flow, false, 0, false, false, true, false),
            (Edge::Flow, false, 1, false, false, true, true),
            (Edge::Flow, false, 1, false, false, false, false),
            (Edge::Flow, true, 0, false, false, true, true),
            (Edge::Light, true, 0, false, false, true, false),
            (Edge::None, false, 0, true, false, false, true),
            (Edge::Sweep, false, 0, false, true, true, true),
            (Edge::Sweep, false, 0, false, false, true, false),
        ] {
            assert_eq!(frames(&p(&d(e, al), k), tr, sw, an), want, "{e:?} {al} {k} {tr} {sw} {an}");
        }
        // přelévaná výplň se hýbe jako efekt
        let mut q = p(&d(Edge::None, false), 1);
        q.fill_flow = true;
        assert!(frames(&q, false, false, true));
    }

    #[test]
    fn reserve_table() {
        let p = |edge, power, on| paint(&Deco { edge, power, live: EdgeLive { on, ..EdgeLive::default() }, ..Deco::default() }, &Fill::default(), Sep::None, &facts(0, 0, 0, None));
        // (efekt, síla, živá hrana, čekaná rezerva při 100 px, 1×)
        for (e, pw, on, want) in [
            (Edge::None, 2, false, (0, 0)),
            (Edge::Underline, 2, false, (0, 0)),
            (Edge::Glow, 2, false, (7, 8)),
            (Edge::Glow, 3, false, (7, 12)),
            (Edge::Light, 2, false, (7, 14)),
            (Edge::Underline, 2, true, (7, 8)),
        ] {
            assert_eq!(reserve(&p(e, pw, on), 100.0, 1.0), want, "{e:?} {pw} {on}");
        }
    }

    #[test]
    fn phases_table() {
        assert_eq!(flow_phase(None, 3), 0.0);
        assert!((flow_phase(Some(1600), 3) - 0.5).abs() < 1e-3);
        assert!((flow_phase(Some(800), 5) - 0.5).abs() < 1e-3);
        assert_eq!(breath(None, 3), 0.85);
        assert!((breath(Some(0), 3) - 0.3).abs() < 1e-3);
        assert!((breath(Some(1700), 3) - 1.0).abs() < 1e-3);
        assert_eq!(comet(None, 3), (0.5, 1.0));
        assert_eq!(comet(Some(1100), 3), (0.5, 1.0));
        assert_eq!(comet(Some(3300), 3), (0.5, -1.0));
        // (ms, průběh naplnění)
        for (ms, want) in [(0, 0.0), (SWEEP_MS, 1.0), (SWEEP_MS * 5, 1.0)] {
            assert_eq!(sweep(ms), want);
        }
        assert!(sweep(SWEEP_MS / 2) > 0.5);
        assert_eq!(dash_n(1.0, 1.0), 4);
        assert_eq!(dash_n(140.0, 1.0), 20);
        assert_eq!(dash_n(10_000.0, 1.0), 48);
        // limit 50 % z 10 čárek: 5 svítí, zbytek slabě; bez limitu všechny
        let v = dashes(10, Some(0.5), None);
        assert_eq!((v.iter().filter(|a| **a > 0.5).count(), v[9]), (5, 0.16));
        assert!(dashes(10, None, None).iter().all(|a| *a == 0.9));
        assert!(dashes(10, None, Some(0.3)).iter().all(|a| (0.45..=1.0).contains(a)));
    }

    #[test]
    fn ts_mirror() {
        let ts = include_str!("../../../src/strip/edgefx.ts");
        let ids = |v: Vec<serde_json::Value>| v.iter().map(|x| format!("\"{}\"", x.as_str().unwrap())).collect::<Vec<_>>().join(", ");
        let j = |x: &[Edge]| ids(x.iter().map(|x| serde_json::to_value(x).unwrap()).collect());
        let edges = [
            Edge::None, Edge::Underline, Edge::Outline, Edge::Glow, Edge::Gradient, Edge::Flow, Edge::Dashes, Edge::Comet, Edge::Light, Edge::Breath,
            Edge::OutlineGradient, Edge::Sweep,
        ];
        let lens = ids([EdgeLen::Center, EdgeLen::Wide, EdgeLen::Full, EdgeLen::Custom].iter().map(|x| serde_json::to_value(x).unwrap()).collect());
        let states = LiveState::ALL.iter().map(|s| format!("\"{}\"", s.id())).collect::<Vec<_>>().join(", ");
        let sets = SETS.iter().map(|(k, _)| format!("\"{k}\"")).collect::<Vec<_>>().join(", ");
        for (name, list) in [("COMPOSE_EDGES", j(&edges)), ("EDGE_LENS", lens), ("LIVE_STATES", states), ("EDGE_SETS", sets)] {
            assert!(ts.contains(&format!("{name} = [{list}]")), "{name}: {list}");
        }
        for (k, h) in SETS {
            assert!(ts.contains(&format!("{k}: [\"{}\", \"{}\", \"{}\"]", h[0], h[1], h[2])), "sada {k}");
        }
        let l = EdgeLive::default();
        let want = "DEFAULT_LIVE: ComposeLive = { on: false, work: \"flow\", wait: \"breath\", done: \"sweep\", limit: \"dashes\" }";
        assert!(ts.contains(want) && l == EdgeLive { on: false, work: Edge::Flow, wait: Edge::Breath, done: Edge::Sweep, limit: Edge::Dashes }, "{want}");
        assert!(ts.contains(&format!("TRY_MS = {TRY_MS};")));
    }
}
