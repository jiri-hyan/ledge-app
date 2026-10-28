//! Vlastní zavřený Ledge (`strip.look = "custom"`, kolo 23, „Poskládat si vlastní“): config `strip.compose`
//! = tvar, rozměry a 1–5 položek ve slotech vlevo / uprostřed / vpravo, každá ve svém stylu. Čisté funkce:
//! ořez configu (`Compose::norm`), co ukázat (`show` → `Show { parts = vlevo, center, right }`), rozměry
//! (`spec`) a pod jakým stylem se tvar umístí (`place_look`). Data jen z `Facts` — položka bez zdroje se
//! nekreslí (prstenec limitu bez dat = prázdná dráha). Předvolby „začít z hotového vzhledu“ jsou
//! v Nastavení (`src/strip/compose.ts`, `composeOf`); id tady a tam hlídá `compose_tests::ts_mirror`.

use super::frame::{ISLAND_GAP, Spec};
use super::{Facts, Glyph, IDLE, Look, MUSIC, OK, Part, RUN, Show, State, WAIT, capsule, done_text, state_of, wait_text};
use crate::claude::activity::Phase;
use crate::i18n::tr;
use crate::strip::Rgba;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Kolik položek nejvýš (všechny sloty dohromady).
pub const MAX_PIECES: usize = 5;
/// Šířka (logické px při velikosti 1), u linky se nepoužije (celá hrana).
pub const W_MIN: u16 = 60;
pub const W_MAX: u16 = 620;
/// Výška těla; linka má vlastní rozsah (tloušťka).
pub const H_MIN: u16 = 18;
pub const H_MAX: u16 = 52;
pub const LINE_H_MIN: u16 = 2;
pub const LINE_H_MAX: u16 = 10;

// Tvar vlastního vzhledu (obyčejné komentáře: popisy jdou do schématu MCP, strop `tools/list`).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeShape")]
#[serde(rename_all = "camelCase")]
pub enum Form {
    // zaoblený pás přilepený k horní hraně
    #[default]
    Capsule,
    // černý výřez s oušky
    Notch,
    // tenká linka přes celou hranu (úsek na položku)
    Line,
    // odznak v rohu (strana podle `appearance.strip.position`)
    Corner,
    // měkká kapka visící z hrany (zaoblení až do půlkruhu)
    Drop,
    // plovoucí kapsle kousek pod hranou
    Island,
}

// Položka z knihovny — jen co má skutečný zdroj ve `Facts` (`looks/facts.rs`).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeItem")]
#[serde(rename_all = "camelCase")]
pub enum PieceItem {
    Time,
    Date,
    #[default]
    Claude,
    Limit5h,
    LimitWeek,
    Music,
    Timer,
    Cpu,
    Memory,
    Battery,
    Weather,
    Servers,
    Repos,
    // živá aktivita Clauda: fáze (odhad podle posledního nástroje) + soubor, čekající schválení
    Activity,
}

// Styl položky.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "ComposeStyle")]
#[serde(rename_all = "camelCase")]
pub enum PieceStyle {
    #[default]
    Icon,
    // ikona s číslem
    Badge,
    Text,
    Ring,
    // malý graf = sloupek z aktuální hodnoty (historii Ledge nedrží)
    Graph,
    Dot,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[schemars(rename = "ComposePiece")]
#[serde(default)]
pub struct Piece {
    pub item: PieceItem,
    pub style: PieceStyle,
}

/// Vlastní vzhled zavřeného Ledge (jen pro strip.look custom).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Compose {
    pub shape: Form,
    pub width: u16,
    pub height: u16,
    // zaoblení 0–100 % (100 = úplně kulaté)
    pub round: u8,
    pub left: Vec<Piece>,
    pub center: Vec<Piece>,
    pub right: Vec<Piece>,
    // v klidu skrytý, ukáže se jen při „čeká“ / „hotovo“
    pub hide_idle: bool,
    // střed: víc událostí se střídá po 5 s
    pub rotate: bool,
}

impl Default for Compose {
    /// Výchozí = předvolba „Klasický“ z editoru (`composeOf("classic")`).
    fn default() -> Self {
        Self {
            shape: Form::Capsule,
            width: 300,
            height: 30,
            round: 55,
            left: vec![Piece { item: PieceItem::Claude, style: PieceStyle::Icon }],
            center: vec![Piece { item: PieceItem::Time, style: PieceStyle::Text }],
            right: vec![Piece { item: PieceItem::Limit5h, style: PieceStyle::Ring }],
            hide_idle: false,
            rotate: false,
        }
    }
}

/// Rozsah výšky pro tvar (linka = tloušťka).
pub fn h_range(shape: Form) -> (u16, u16) {
    if shape == Form::Line { (LINE_H_MIN, LINE_H_MAX) } else { (H_MIN, H_MAX) }
}

impl Compose {
    /// Ořez configu: rozměry do rozsahů, každá položka nejvýš jednou, celkem 1–5 (zleva doprava); prázdné =
    /// stav Clauda uprostřed. Starý / ručně psaný config se tak vždy načte a nakreslí.
    pub fn norm(&self) -> Compose {
        let (h0, h1) = h_range(self.shape);
        let mut seen = Vec::new();
        let mut take = |v: &[Piece]| -> Vec<Piece> {
            let mut out = Vec::new();
            for p in v {
                if seen.len() < MAX_PIECES && !seen.contains(&p.item) {
                    seen.push(p.item);
                    out.push(*p);
                }
            }
            out
        };
        let (left, mut center, right) = (take(&self.left), take(&self.center), take(&self.right));
        if left.is_empty() && center.is_empty() && right.is_empty() {
            center.push(Piece { item: PieceItem::Claude, style: PieceStyle::Icon });
        }
        Compose {
            shape: self.shape,
            width: self.width.clamp(W_MIN, W_MAX),
            height: self.height.clamp(h0, h1),
            round: self.round.min(100),
            left,
            center,
            right,
            hide_idle: self.hide_idle,
            rotate: self.rotate,
        }
    }

    pub fn has(&self, item: PieceItem) -> bool {
        self.left.iter().chain(&self.center).chain(&self.right).any(|p| p.item == item)
    }

    /// Pod jakým stylem se tělo umístí (`frame::place`, celá šířka, roh); kreslí se jako `Custom`, linka jako `Line`.
    pub fn place_look(&self) -> Look {
        match self.shape {
            Form::Line => Look::Line,
            Form::Corner => Look::Corner,
            _ => Look::Custom,
        }
    }
}

/// Hodnota položky ze skutečných dat (`None` = zdroj nic nemá → položka se nekreslí).
#[derive(Debug, Clone, PartialEq)]
pub struct Val {
    pub glyph: Option<Glyph>,
    pub color: Rgba,
    pub text: String,
    /// číslo do odznaku
    pub num: Option<String>,
    /// podíl 0–1 pro prstenec / graf
    pub frac: Option<f32>,
    /// událost (střídání ve středu, „neklid“)
    pub event: bool,
}

use crate::util::pct_text as pct;
fn frac(p: f64) -> f32 {
    (p / 100.0).clamp(0.0, 1.0) as f32
}
fn val0(glyph: Option<Glyph>, color: Rgba, text: String) -> Val {
    Val { glyph, color, text, num: None, frac: None, event: false }
}
fn share(label: &str, p: Option<f64>, color: Rgba) -> Option<Val> {
    p.map(|p| Val { num: Some(format!("{:.0}", p.round())), frac: Some(frac(p)), ..val0(None, color, format!("{label} {}", pct(p))) })
}

/// Co položka teď ukazuje.
pub fn value(item: PieceItem, f: &Facts) -> Option<Val> {
    let some_if = |t: &str| (!t.is_empty()).then(|| t.to_string());
    match item {
        PieceItem::Time => some_if(&f.clock).map(|t| val0(Some(Glyph::Clock), Rgba::TEXT, t)),
        PieceItem::Date => some_if(capsule::day_of(&f.date)).map(|t| val0(Some(Glyph::Calendar), Rgba::TEXT, t)),
        PieceItem::Claude => Some(if f.waiting > 0 || f.reminder.is_some() {
            let num = (f.waiting > 0).then(|| f.waiting.to_string());
            Val { num, event: true, ..val0(Some(if f.waiting > 0 { Glyph::Claude } else { Glyph::Bell }), WAIT, wait_text(f)) }
        } else if f.done > 0 {
            Val { num: Some(f.done.to_string()), event: true, ..val0(Some(Glyph::Check), OK, done_text(f)) }
        } else if f.working > 0 {
            let t = if f.working == 1 { tr("Pracuje", "Working").to_string() } else { trf!("Pracuje · {}", "Working · {}", f.working) };
            Val { num: Some(f.working.to_string()), event: true, ..val0(Some(Glyph::Claude), RUN, t) }
        } else {
            val0(Some(Glyph::Claude), Rgba::DIM, tr("Klid", "Idle").into())
        }),
        PieceItem::Limit5h => share("5h", f.limit5h, f.limit5h.map(|p| capsule::ring_color(p, f.accent)).unwrap_or(f.accent)),
        PieceItem::LimitWeek => share(tr("Týden", "Week"), f.limit7d, f.limit7d.map(|p| capsule::ring_color(p, f.accent)).unwrap_or(f.accent)),
        PieceItem::Music => f.music.as_ref().map(|m| Val { frac: m.progress, event: true, ..val0(Some(Glyph::Music), MUSIC, m.text.clone()) }),
        PieceItem::Timer => f.timer.as_ref().map(|t| Val {
            num: t.minutes.map(|m| m.to_string()),
            frac: t.progress,
            event: true,
            ..val0(Some(Glyph::Timer), f.accent, t.text.clone())
        }),
        PieceItem::Cpu => share("CPU", f.cpu, f.cpu.map(|p| if p >= capsule::WARN_PCT { Rgba::AMBER } else { Rgba::TEXT }).unwrap_or(Rgba::TEXT))
            .map(|v| Val { glyph: Some(Glyph::Cpu), ..v }),
        PieceItem::Memory => share("RAM", f.mem, f.mem.map(|p| if p >= capsule::WARN_PCT { Rgba::AMBER } else { Rgba::TEXT }).unwrap_or(Rgba::TEXT))
            .map(|v| Val { glyph: Some(Glyph::Cpu), ..v }),
        PieceItem::Battery => f.battery.map(|b| {
            let c = if b.charging { Rgba::GREEN } else if b.percent <= 20 { Rgba::RED } else { Rgba::TEXT };
            Val { num: Some(b.percent.to_string()), frac: Some(frac(b.percent as f64)), ..val0(None, c, pct(b.percent as f64)) }
        }),
        PieceItem::Weather => f.weather.map(|w| val0(Some(w.glyph), Rgba::TEXT, format!("{}°", w.temp))),
        PieceItem::Servers => {
            let c = if f.server_failed { Rgba::RED } else if f.servers > 0 { OK } else { Rgba::DIM };
            let t = if f.servers > 0 { trf!("Servery {}", "Servers {}", f.servers) } else { String::new() };
            Some(Val { num: (f.servers > 0).then(|| f.servers.to_string()), ..val0(Some(Glyph::Server), c, t) })
        }
        PieceItem::Repos => {
            let c = if f.repos > 0 { Rgba::TEXT } else { Rgba::DIM };
            let t = if f.repos > 0 { trf!("Repa {}", "Repos {}", f.repos) } else { String::new() };
            Some(Val { num: (f.repos > 0).then(|| f.repos.to_string()), ..val0(Some(Glyph::Repo), c, t) })
        }
        PieceItem::Activity => activity_value(f),
    }
}

/// Jméno fáze do proužku (odhad — editor Kostky to u položky říká).
pub fn phase_name(p: Phase) -> &'static str {
    match p {
        Phase::Reading => tr("Čtení", "Reading"),
        Phase::Editing => tr("Úpravy", "Editing"),
        Phase::Testing => tr("Testy", "Tests"),
        Phase::Working => tr("Pracuje", "Working"),
        Phase::Done => tr("Hotovo", "Done"),
        Phase::Error => tr("Chyba", "Error"),
    }
}

/// „Claude aktivita“: čekající schválení / otázka má přednost (odznak = kolik), jinak fáze vedoucí session
/// + soubor (bez souboru projekt). Bez podílu: fáze je odhad, prstenec ani graf z ní se nekreslí. Bez aktivity = nic.
fn activity_value(f: &Facts) -> Option<Val> {
    if f.waiting > 0 {
        return Some(Val { num: Some(f.waiting.to_string()), event: true, ..val0(Some(Glyph::Bell), WAIT, wait_text(f)) });
    }
    let a = f.activity.as_ref()?;
    let what = if a.file.is_empty() { &a.project } else { &a.file };
    let name = phase_name(a.phase);
    let text = if what.is_empty() { name.to_string() } else { format!("{name} · {what}") };
    let (glyph, color) = match a.phase {
        Phase::Done => (Glyph::Check, OK),
        Phase::Error => (Glyph::Bell, Rgba::RED),
        _ => (Glyph::Claude, RUN),
    };
    Some(Val { event: true, ..val0(Some(glyph), color, text) })
}

/// Položka ve zvoleném stylu → díly ke kreslení. Styl, který položka neumí (prstenec bez podílu, ikona bez
/// ikony), spadne na nejbližší, co umí; prázdný text = nic.
pub fn parts(p: Piece, f: &Facts) -> Vec<Part> {
    let limit = matches!(p.item, PieceItem::Limit5h | PieceItem::LimitWeek);
    let Some(v) = value(p.item, f) else {
        // limit bez dat: prstenec / graf = prázdná dráha (žádné vymyšlené %), jinak nic
        return match p.style {
            PieceStyle::Ring if limit => vec![Part::Ring { fill: None, color: IDLE }],
            _ => vec![],
        };
    };
    let txt = |v: &Val| -> Vec<Part> {
        if v.text.is_empty() {
            vec![]
        } else {
            vec![Part::Text { text: v.text.clone(), color: v.color, strong: v.color == WAIT }]
        }
    };
    let icon = |v: &Val, badge: bool| -> Vec<Part> {
        match (p.item, v.glyph) {
            (PieceItem::Battery, _) => {
                let b = f.battery.unwrap_or_default();
                let mut out = vec![Part::Battery { percent: b.percent, charging: b.charging }];
                if badge {
                    out.extend(txt(v));
                }
                out
            }
            (_, Some(g)) => vec![Part::Icon { glyph: g, color: v.color, badge: if badge { v.num.clone() } else { None } }],
            (_, None) => txt(v),
        }
    };
    match p.style {
        PieceStyle::Icon => icon(&v, false),
        PieceStyle::Badge => icon(&v, true),
        PieceStyle::Text => txt(&v),
        PieceStyle::Dot => vec![Part::Dot(if v.event || (v.color != Rgba::DIM && v.color != Rgba::TEXT) { v.color } else { IDLE })],
        PieceStyle::Ring => match v.frac {
            Some(x) => vec![Part::Ring { fill: Some(x), color: v.color }],
            None if p.item == PieceItem::Claude => vec![Part::Ring { fill: None, color: if v.event { v.color } else { IDLE } }],
            None => icon(&v, false),
        },
        PieceStyle::Graph => match v.frac {
            Some(x) => vec![Part::Meter { fill: x, color: v.color }],
            None => icon(&v, false),
        },
    }
}

fn row(v: &[Piece], f: &Facts) -> Vec<Part> {
    v.iter().flat_map(|p| parts(*p, f)).collect()
}

/// Stav → co vlastní vzhled ukáže: `parts` = vlevo, `center`, `right`; linka = úsek na položku.
pub fn show(c: &Compose, f: &Facts) -> Show {
    let c = c.norm();
    let st = state_of(f);
    let mut s = Show { idle: st == State::Idle, ..Show::default() };
    if c.hide_idle && !matches!(st, State::Wait | State::Done) {
        s.hidden = true;
        return s;
    }
    if c.shape == Form::Line {
        s.line = c
            .left
            .iter()
            .chain(&c.center)
            .chain(&c.right)
            .map(|p| (1.0, value(p.item, f).filter(|v| v.event || v.frac.is_some()).map(|v| v.color).unwrap_or(IDLE)))
            .collect();
        return s;
    }
    s.parts = row(&c.left, f);
    s.right = row(&c.right, f);
    if c.rotate && c.center.len() > 1 {
        // střídají se jen události (žádný časovač v klidu); bez události první položka, co něco má
        let ev: Vec<Vec<Part>> = c.center.iter().filter(|p| value(p.item, f).is_some_and(|v| v.event)).map(|p| parts(*p, f)).filter(|v| !v.is_empty()).collect();
        s.center = match ev.len() {
            0 => c.center.iter().map(|p| parts(*p, f)).find(|v| !v.is_empty()).unwrap_or_default(),
            1 => ev[0].clone(),
            _ => {
                s.rotate = ev.clone();
                ev[0].clone()
            }
        };
    } else {
        s.center = row(&c.center, f);
    }
    // prstenec se točí jen při práci Clauda (a se zapnutými animacemi, `StripModel.animate`)
    let ringed = s.parts.iter().chain(&s.center).chain(&s.right).any(|p| matches!(p, Part::Ring { .. }));
    s.spin = ringed && f.working > 0 && f.waiting == 0;
    s
}

/// Rozměry vlastního vzhledu v logických px při velikosti 1 (`appearance.strip.size` je násobí).
pub fn spec(c: &Compose) -> Spec {
    let c = c.norm();
    let h = c.height as f32;
    let k = c.round as f32 / 100.0;
    let base = Spec {
        h,
        r: k * h / 2.0,
        pad: (h * 0.45).clamp(6.0, 20.0),
        gap: (h * 0.28).clamp(4.0, 14.0),
        min_w: c.width as f32,
        font: (h * 0.45).clamp(9.5, 20.0),
        fill: Rgba(0.0, 0.0, 0.0, 1.0),
        edge: None,
        edge_bottom: false,
        ears: 0.0,
        top_gap: 0.0,
        side_gap: 0.0,
        pill: false,
        mono: false,
        display: false,
        dot: (h * 0.27).clamp(5.0, 12.0),
        ring: (h * 0.6).clamp(10.0, 30.0),
        pill_r: 0.0,
    };
    match c.shape {
        Form::Capsule => base,
        Form::Drop => Spec { r: k * h, ..base },
        Form::Notch => Spec { ears: (h * 0.42).clamp(6.0, 18.0), ..base },
        Form::Line => Spec { pad: 0.0, gap: 0.0, min_w: 0.0, fill: Rgba(0.0, 0.0, 0.0, 0.0), ..base },
        Form::Corner => Spec { pill: true, pill_r: (k * h / 2.0).max(0.01), top_gap: 8.0, side_gap: 10.0, edge: Some(Rgba(1.0, 1.0, 1.0, 0.14)), ..base },
        Form::Island => Spec { pill: true, pill_r: (k * h / 2.0).max(0.01), top_gap: ISLAND_GAP, ..base },
    }
}

/// Co vlastní vzhled potřebuje za data navíc (hudba = vlákno `media`, baterie = odběr změn napájení,
/// systém = vzorky CPU / paměti) — jen když tu položku opravdu má.
pub fn needs(c: &Compose) -> (bool, bool, bool) {
    let c = c.norm();
    (c.has(PieceItem::Music), c.has(PieceItem::Battery), c.has(PieceItem::Cpu) || c.has(PieceItem::Memory))
}
