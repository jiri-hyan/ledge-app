//! Styly zavřeného Ledge (`strip.look`): jedna volba určí tvar, pozici, velikost i CO proužek ukazuje.
//! `classic` = dnešní proužek (tvar `appearance.strip.style` + položky `strip.items`), ostatní styly mají
//! obsah pevně daný stylem a skládá ho tady čistá funkce `show` ze stavu `Facts` (jen reálná data z paměti
//! jádra, sbírá je `stripview::refresh_strip`). Priorita stavů: čeká na tebe > hotovo > časovač > Claude
//! pracuje > hudba > klid (`events`). Pokročilé doladění: `strip.lookHide` = co styl nemá ukazovat
//! (`without`). Tvar a místo u hrany: `frame`. Kreslí `stripdraw`.

pub mod capsule;
pub mod compose;
pub mod composedeco;
#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod activity_tests;
pub mod facts;
pub mod frame;
#[cfg(test)]
mod tests;

use crate::config::StripItem;
use crate::i18n::tr;
use crate::strip::Rgba;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Styl zavřeného Ledge. Seznam id je kontrakt s Nastavením (`src/strip/looks.ts`, test zrcadla).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Look {
    /// dnešní proužek: tvar `appearance.strip.style`, obsah `strip.items`
    #[default]
    Classic,
    /// černá kapsle, v klidu malá, při dění se roztáhne
    Island,
    /// 4 stavové tečky bez textu (Claude, servery, repa, ty)
    Dots,
    /// 3px linka přes celou šířku s barevnými úseky
    Line,
    /// ikony s odznaky čísel
    Icons,
    /// velké hodiny + jedna stavová tečka / ikona
    Clock,
    /// odznak v rohu (levý / pravý podle `appearance.strip.position`)
    Corner,
    /// v klidu nic, objeví se jen při „čeká na tebe“ / „hotovo“
    Ghost,
    /// stavová lišta přes celou šířku (monospace)
    Bar,
    /// černý výřez s oblými „oušky“
    Notch,
    /// text + tenká lišta průběhu (limit 5h / časovač / skladba)
    Progress,
    /// jeden řádek, víc událostí se střídá po 5 s
    Ticker,
    // kolo 22 (obyčejné komentáře: popisy variant jdou do schématu MCP `tools/list`, strop 20 kB):
    // jen baterie a čas
    Minimum,
    // počasí | čas a datum | CPU, paměť, baterie
    Trio,
    // stav Clauda: 1–2 prstence limitů (5h, týden) + čas; prstenec se točí, jen když Claude pracuje
    Rings,
    // kolo 23: vlastní (`strip.compose`: tvar, rozměry, položky) — jen výslovnou volbou
    Custom,
}

/// Tvar kapsle u stylů s tělem (`strip.lookShape`, kolo 22): volba tvaru, ne nový styl. Platí jen pro styly
/// s `Look::shapeable` (ne linka, lišta, tečky, roh, výřez, klasický).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Shape {
    // dnešní tvar stylu (oblé dolní rohy, horní za hranou)
    #[default]
    Round,
    // ostré rohy
    Sharp,
    // prodloužená kapsle (víc místa po stranách)
    Long,
    // „jako Mac“: oblá ze všech stran, kousek pod hranou (Dynamic Island)
    Island,
}

impl Look {
    /// všechny styly v pořadí galerie (testy zrcadel)
    #[allow(dead_code)]
    pub const ALL: [Look; 16] = [
        Look::Classic,
        Look::Island,
        Look::Dots,
        Look::Line,
        Look::Icons,
        Look::Clock,
        Look::Corner,
        Look::Ghost,
        Look::Bar,
        Look::Notch,
        Look::Progress,
        Look::Ticker,
        Look::Minimum,
        Look::Trio,
        Look::Rings,
        Look::Custom,
    ];

    #[allow(dead_code)]
    pub fn id(self) -> &'static str {
        match self {
            Look::Classic => "classic",
            Look::Island => "island",
            Look::Dots => "dots",
            Look::Line => "line",
            Look::Icons => "icons",
            Look::Clock => "clock",
            Look::Corner => "corner",
            Look::Ghost => "ghost",
            Look::Bar => "bar",
            Look::Notch => "notch",
            Look::Progress => "progress",
            Look::Ticker => "ticker",
            Look::Minimum => "minimum",
            Look::Trio => "trio",
            Look::Rings => "rings",
            Look::Custom => "custom",
        }
    }

    /// Lidský název (zrcadlo `cs`/`en` v `src/strip/looks.ts`).
    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            Look::Classic => tr("Klasický", "Classic"),
            Look::Island => tr("Ostrůvek", "Island"),
            Look::Dots => tr("Jen tečky", "Just dots"),
            Look::Line => tr("Tenká linka", "Thin line"),
            Look::Icons => tr("Ikony s odznaky", "Icons with badges"),
            Look::Clock => tr("Hodiny", "Clock"),
            Look::Corner => tr("Rohový odznak", "Corner badge"),
            Look::Ghost => tr("Neviditelný do události", "Hidden until needed"),
            Look::Bar => tr("Stavová lišta", "Status bar"),
            Look::Notch => tr("Výřez", "Notch"),
            Look::Progress => tr("Průběh", "Progress"),
            Look::Ticker => tr("Střídající se řádek", "Rotating line"),
            Look::Minimum => tr("Minimum", "Minimal"),
            Look::Trio => tr("Trojice", "Trio"),
            Look::Rings => tr("Stav Clauda", "Claude status"),
            Look::Custom => tr("Vlastní", "Custom"),
        }
    }

    /// Ukazuje styl hudbu? (vlákno `media` se pak pustí i bez položky `media` v `strip.items`)
    pub fn shows_music(self) -> bool {
        !matches!(self, Look::Classic | Look::Ghost | Look::Minimum | Look::Trio | Look::Rings | Look::Custom)
    }

    /// Ukazuje styl baterii? (`battery::ensure` = odběr změn napájení od Windows, žádný časovač)
    pub fn shows_battery(self) -> bool {
        matches!(self, Look::Minimum | Look::Trio)
    }

    /// Ukazuje styl CPU a paměť? (plánovač `integrations` pak vzorkuje jako u položky proužku „Systém“)
    pub fn shows_system(self) -> bool {
        self == Look::Trio
    }

    /// Jde u stylu zvolit tvar kapsle (`Shape`)?
    pub fn shapeable(self) -> bool {
        matches!(
            self,
            Look::Island | Look::Icons | Look::Clock | Look::Ghost | Look::Progress | Look::Ticker | Look::Minimum | Look::Trio | Look::Rings
        )
    }

    /// Nad taby prohlížeče (lištové okno v popředí) ustoupí na tenkou linku jako klasický proužek?
    /// Linka, tečky, roh a neviditelný styl taby nepřekrývají (nebo jen na okamžik události).
    pub fn retreats(self) -> bool {
        !matches!(self, Look::Line | Look::Dots | Look::Corner | Look::Ghost)
    }

    /// Přes celou šířku monitoru (hover zóna taky).
    pub fn full_width(self) -> bool {
        matches!(self, Look::Line | Look::Bar)
    }
}

/// Jak dlouho po doběhnutí session platí „hotovo“ (ms). Kratší než 15 min klasické položky „N× hotovo“ —
/// „hotovo“ má u stylů přednost před „pracuje“, nesmí ho dlouho zakrývat. Vyprší s minutovým budíkem.
pub const DONE_MS: u64 = 5 * 60 * 1000;
/// Střídání řádků stylu `ticker` (jen při víc než jedné události).
pub const ROTATE_MS: u32 = 5000;

/// Barvy stavů (shodné s náhledem v Nastavení). `WAIT` = i oranžový prstenec „čeká na tebe“ stylu `rings`.
pub const RUN: Rgba = Rgba::BLUE;
pub const WAIT: Rgba = Rgba::AMBER;
pub const OK: Rgba = Rgba::GREEN;
pub const MUSIC: Rgba = Rgba(1.0, 0.81, 0.36, 1.0);
/// tečka bez dění
pub const IDLE: Rgba = Rgba(0.35, 0.38, 0.46, 1.0);
/// text stavové lišty (terminálová zelená, jako náhled v Nastavení)
pub const BAR_TEXT: Rgba = Rgba(0.62, 0.91, 0.66, 1.0);

/// Časovač pro styly (odpočet má přednost před stopkami).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimerFact {
    /// „18 min“, „čas vypršel“, stopky „0:42“
    pub text: String,
    /// zbývající minuty odpočtu (odznak, výřez); `None` = stopky / doběhlý
    pub minutes: Option<u32>,
    /// uplynulý podíl odpočtu 0–1; `None` = stopky / doběhlý (žádná lišta)
    pub progress: Option<f32>,
    pub done: bool,
}

/// Právě hraje (jen při přehrávání).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MusicFact {
    /// „Název · Interpret“ (zkrácené)
    pub text: String,
    /// „1/4 min“ (`mediatime::strip_time`), jen když zdroj dává timeline
    pub time: Option<String>,
    /// pozice / délka 0–1, jen z timeline
    pub progress: Option<f32>,
}

/// Baterie z `battery::view` (jen stroj s baterií; Windows hlásí změny samo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BatteryFact {
    pub percent: u8,
    pub charging: bool,
}

/// Počasí z paměti karty Počasí (`Core.weather`, jen čerstvá data) — proužek nic nestahuje.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherFact {
    /// zaokrouhlené °C
    pub temp: i32,
    pub glyph: Glyph,
}

/// Co se děje — jen paměť jádra (žádné I/O). Prázdné = nevíme / nic.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    /// kolik session čeká na tebe (schválení / otázka)
    pub waiting: usize,
    /// projekt první čekající session ("" = neznámý)
    pub waiting_where: String,
    /// připomínka po termínu (titulek, zkrácený) + kolik jich je
    pub reminder: Option<(String, usize)>,
    pub working: usize,
    pub working_where: String,
    /// session doběhlé za posledních `DONE_MS`
    pub done: usize,
    pub done_where: String,
    pub timer: Option<TimerFact>,
    pub music: Option<MusicFact>,
    /// běžící servery / appky (bez skončených)
    pub servers: usize,
    pub server_failed: bool,
    /// repa s neuloženou / nepushnutou prací
    pub repos: usize,
    /// čerstvé procento limitu 5h (`limits::State::Fresh`)
    pub limit5h: Option<f64>,
    /// čerstvé procento týdenního limitu
    pub limit7d: Option<f64>,
    /// baterie (jen stroj s baterií a zapnutý údaj Baterie)
    pub battery: Option<BatteryFact>,
    /// počasí (jen čerstvá data karty Počasí)
    pub weather: Option<WeatherFact>,
    /// CPU a paměť v % z posledního čerstvého vzorku (`sysmon`)
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
    /// historie CPU / paměti v % ze vzorků proužku (~10 s, nejstarší první; `facts::SysHist`)
    pub cpu_hist: Vec<f32>,
    pub mem_hist: Vec<f32>,
    /// živá aktivita vedoucí session (položka skládačky „Claude aktivita“; `claude/activity.rs`)
    pub activity: Option<facts::ActivityFact>,
    /// „14:32“
    pub clock: String,
    /// „14:32 · ne 18. 10.“
    pub date: String,
    /// název záložky šuplíku („Práce“)
    pub mode: String,
    /// akcent motivu (čitelný na tmavém těle)
    pub accent: Rgba,
}

impl Default for Facts {
    fn default() -> Self {
        Self {
            waiting: 0,
            waiting_where: String::new(),
            reminder: None,
            working: 0,
            working_where: String::new(),
            done: 0,
            done_where: String::new(),
            timer: None,
            music: None,
            servers: 0,
            server_failed: false,
            repos: 0,
            limit5h: None,
            limit7d: None,
            battery: None,
            weather: None,
            cpu: None,
            mem: None,
            cpu_hist: Vec::new(),
            mem_hist: Vec::new(),
            activity: None,
            clock: String::new(),
            date: String::new(),
            mode: String::new(),
            accent: Rgba::VIOLET,
        }
    }
}

/// Pokročilé: zdroje, které styl nemá ukazovat (`strip.lookHide`). `claude` = pracuje / hotovo,
/// `waiting` = čeká na tebe, `running` = servery, `repos`, `timer`, `media`, `limits`, `clock`.
pub fn without(mut f: Facts, hide: &[StripItem]) -> Facts {
    for it in hide {
        match it {
            StripItem::Claude => {
                f.working = 0;
                f.activity = None;
                f.done = 0;
            }
            StripItem::Waiting => f.waiting = 0,
            StripItem::Running => {
                f.servers = 0;
                f.server_failed = false;
            }
            StripItem::Repos => f.repos = 0,
            StripItem::Timer => f.timer = None,
            StripItem::Media => f.music = None,
            StripItem::Limits => {
                f.limit5h = None;
                f.limit7d = None;
            }
            StripItem::System => {
                f.cpu = None;
                f.mem = None;
            }
            StripItem::Clock => {
                f.clock.clear();
                f.date.clear();
            }
            StripItem::Mode => f.mode.clear(),
            // GitHub, Systém, widgety ze skriptů a poštu styly zavřeného Ledge (zatím) nekreslí
            StripItem::Custom | StripItem::Github | StripItem::Widget(_) | StripItem::Mail => {}
        }
    }
    f
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Work,
    Wait,
    Music,
    Timer,
    Done,
}

/// Aktivní události v pořadí priority (čeká > hotovo > časovač > pracuje > hudba).
pub fn events(f: &Facts) -> Vec<State> {
    let mut v = Vec::new();
    if f.waiting > 0 || f.reminder.is_some() {
        v.push(State::Wait);
    }
    if f.done > 0 {
        v.push(State::Done);
    }
    if f.timer.is_some() {
        v.push(State::Timer);
    }
    if f.working > 0 {
        v.push(State::Work);
    }
    if f.music.is_some() {
        v.push(State::Music);
    }
    v
}

/// Hlavní stav (nejvyšší priorita, jinak klid).
pub fn state_of(f: &Facts) -> State {
    events(f).first().copied().unwrap_or(State::Idle)
}

/// Ikona ze systémového písma (Segoe Fluent Icons, záložně Segoe MDL2 Assets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Claude,
    Server,
    Repo,
    Music,
    Timer,
    Bell,
    Check,
    Sun,
    Moon,
    Cloud,
    // kolo 23 (vlastní vzhled): čas, datum, výkon
    Clock,
    Calendar,
    Cpu,
}

impl Glyph {
    /// Znak v písmu ikon (stejné kódy v Segoe Fluent Icons i Segoe MDL2 Assets).
    pub fn code(self) -> char {
        match self {
            Glyph::Claude => '\u{E99A}',
            Glyph::Server => '\u{E968}',
            Glyph::Repo => '\u{E8F1}',
            Glyph::Music => '\u{E8D6}',
            Glyph::Timer => '\u{E916}',
            Glyph::Bell => '\u{EA8F}',
            Glyph::Check => '\u{E73E}',
            Glyph::Sun => '\u{E706}',
            Glyph::Moon => '\u{E708}',
            Glyph::Cloud => '\u{E753}',
            Glyph::Clock => '\u{E121}',
            Glyph::Calendar => '\u{E787}',
            Glyph::Cpu => '\u{E9D9}',
        }
    }

    /// Náhrada běžným písmem, když písmo ikon v systému chybí.
    pub fn fallback(self) -> &'static str {
        match self {
            Glyph::Claude => "✻",
            Glyph::Server => "▣",
            Glyph::Repo => "⑂",
            Glyph::Music => "♪",
            Glyph::Timer => "⏱",
            Glyph::Bell => "!",
            Glyph::Check => "✓",
            Glyph::Sun => "☀",
            Glyph::Moon => "☾",
            Glyph::Cloud => "☁",
            Glyph::Clock => "◷",
            Glyph::Calendar => "▦",
            Glyph::Cpu => "▤",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Text { text: String, color: Rgba, strong: bool },
    Dot(Rgba),
    Icon { glyph: Glyph, color: Rgba, badge: Option<String> },
    /// oddělovač segmentů (styl `bar`, `trio`)
    Sep,
    /// baterie jako tvar (obrys, výplň podle procent, blesk při nabíjení); procenta jsou zvlášť text
    Battery { percent: u8, charging: bool },
    /// prstenec limitu (`rings`): podíl 0–1, `None` = jen dráha bez čísla (limit neznáme)
    Ring { fill: Option<f32>, color: Rgba },
    /// malý graf vlastního vzhledu: vodorovný sloupek z aktuální hodnoty 0–1
    Meter { fill: f32, color: Rgba },
    /// mini graf z historie (hodnoty 0–1, nejstarší první) — vlastní vzhled, styl „průběh“ (kolo 25)
    Spark { pts: Vec<f32>, color: Rgba },
    /// text s tenkým pruhem pod ním (podíl 0–1) — vlastní vzhled, styl „pruh“ (kolo 25)
    Under { text: String, color: Rgba, fill: f32 },
}

/// Díl vlastního vzhledu v řadě `Part`: kolik částí mu patří a barevný podklad (styl „podklad“).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Seg {
    pub n: usize,
    pub chip: Option<Rgba>,
}

/// Co styl právě ukáže (čistá data pro `stripdraw`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Show {
    /// nic není vidět (okno skryté; hover zóna u hrany dál platí)
    pub hidden: bool,
    /// v klidu (ostrůvek je malý)
    pub idle: bool,
    pub parts: Vec<Part>,
    /// zarovnané vpravo (`bar`, hodnota u `progress`)
    pub right: Vec<Part>,
    /// lišta průběhu (podíl 0–1, barva) — jen z reálných dat
    pub progress: Option<(f32, Rgba)>,
    /// úseky linky (váha, barva) — styl `line`
    pub line: Vec<(f32, Rgba)>,
    /// střídané řádky (`ticker`); víc než jeden = střídání po `ROTATE_MS`
    pub rotate: Vec<Vec<Part>>,
    /// prstence se točí (`rings`, Claude pracuje) — jen se zapnutými animacemi (`StripModel.animate`)
    pub spin: bool,
    /// střed vlastního vzhledu (`custom`: `parts` = vlevo, `right` = vpravo; střídá se `rotate`)
    pub center: Vec<Part>,
    /// vlastní vzhled: díly zón vlevo / střed / vpravo (u střídání středu jeden na řádek `rotate`)
    pub segs: [Vec<Seg>; 3],
    /// vlastní vzhled: výzdoba s barvami ze stavu (kolo 25)
    pub paint: Option<composedeco::Paint>,
}

pub(crate) fn text(t: impl Into<String>, color: Rgba) -> Part {
    Part::Text { text: t.into(), color, strong: false }
}
pub(crate) fn strong(t: impl Into<String>, color: Rgba) -> Part {
    Part::Text { text: t.into(), color, strong: true }
}
pub(crate) fn icon(glyph: Glyph, color: Rgba) -> Part {
    Part::Icon { glyph, color, badge: None }
}
fn badge(glyph: Glyph, color: Rgba, n: usize) -> Part {
    Part::Icon { glyph, color, badge: (n > 0).then(|| n.to_string()) }
}
fn at(where_: &str) -> String {
    if where_.is_empty() { String::new() } else { format!(" · {where_}") }
}

/// „Čeká na tebe · ledge +1“, jen připomínka = její titulek.
pub(crate) fn wait_text(f: &Facts) -> String {
    if f.waiting > 0 {
        let more = if f.waiting > 1 { format!(" +{}", f.waiting - 1) } else { String::new() };
        format!("{}{}{more}", tr("Čeká na tebe", "Waiting for you"), at(&f.waiting_where))
    } else {
        match &f.reminder {
            Some((t, n)) if *n > 1 => format!("{t} +{}", n - 1),
            Some((t, _)) => t.clone(),
            None => String::new(),
        }
    }
}
/// tečka čekání (Claude) nebo zvonek (jen připomínka)
pub(crate) fn wait_lead(f: &Facts) -> Part {
    if f.waiting > 0 { Part::Dot(WAIT) } else { icon(Glyph::Bell, WAIT) }
}
pub(crate) fn done_text(f: &Facts) -> String {
    if f.done_where.is_empty() { tr("Hotovo", "Done").into() } else { trf!("Hotovo: {}", "Done: {}", f.done_where) }
}
fn work_text(f: &Facts) -> String {
    match (f.working, f.working_where.as_str()) {
        (1, "") => tr("Claude pracuje", "Claude is working").into(),
        (1, w) => trf!("Claude pracuje · {w}", "Claude is working · {w}"),
        (n, _) => trf!("Claude pracuje · {n}", "Claude is working · {n}"),
    }
}
fn timer_text(f: &Facts) -> String {
    f.timer.as_ref().map(|t| t.text.clone()).unwrap_or_default()
}
fn music_text(f: &Facts) -> String {
    f.music.as_ref().map(|m| m.text.clone()).unwrap_or_default()
}
fn n_working(n: usize) -> String {
    if n <= 1 { tr("pracuje", "working").into() } else { trf!("{n}× pracuje", "{n}× working") }
}

/// Jeden řádek pro událost (ostrůvek, ticker, neviditelný): vedoucí tečka / ikona + text.
fn event_line(st: State, f: &Facts) -> Vec<Part> {
    match st {
        State::Wait => vec![wait_lead(f), strong(wait_text(f), WAIT)],
        State::Done => vec![icon(Glyph::Check, OK), text(done_text(f), Rgba::TEXT)],
        State::Timer => vec![icon(Glyph::Timer, f.accent), text(timer_text(f), Rgba::TEXT)],
        State::Work => vec![Part::Dot(RUN), text(work_text(f), Rgba::TEXT)],
        State::Music => vec![icon(Glyph::Music, MUSIC), text(music_text(f), Rgba::TEXT)],
        State::Idle => vec![],
    }
}

/// Stav → co styl ukáže. `Classic` vrací prázdné (kreslí ho dnešní cesta z `strip.items`).
pub fn show(look: Look, f: &Facts) -> Show {
    let st = state_of(f);
    let idle = st == State::Idle;
    let mut s = Show { idle, ..Show::default() };
    match look {
        // vlastní skládá `compose::show` z `strip.compose` (volá `stripview`)
        Look::Classic | Look::Custom => {}
        Look::Minimum | Look::Trio | Look::Rings => capsule::fill(look, f, &mut s),
        Look::Island => {
            s.parts = if idle { vec![Part::Dot(IDLE)] } else { event_line(st, f) };
        }
        Look::Ghost => {
            if matches!(st, State::Wait | State::Done) {
                s.parts = event_line(st, f);
            } else {
                s.hidden = true;
            }
        }
        Look::Ticker => {
            let ev = events(f);
            if ev.is_empty() {
                s.parts = vec![text(f.date.clone(), Rgba::DIM)];
            } else {
                s.rotate = ev.iter().map(|e| event_line(*e, f)).collect();
                s.parts = s.rotate[0].clone();
                if s.rotate.len() == 1 {
                    s.rotate.clear();
                }
            }
        }
        Look::Dots => {
            let claude = if f.waiting > 0 {
                WAIT
            } else if f.done > 0 {
                OK
            } else if f.working > 0 {
                RUN
            } else {
                IDLE
            };
            let servers = if f.server_failed {
                Rgba::RED
            } else if f.servers > 0 {
                OK
            } else {
                IDLE
            };
            let repos = if f.repos > 0 { WAIT } else { IDLE };
            let you = if f.reminder.is_some() {
                WAIT
            } else if f.timer.is_some() {
                f.accent
            } else if f.music.is_some() {
                MUSIC
            } else {
                IDLE
            };
            s.parts = vec![Part::Dot(claude), Part::Dot(servers), Part::Dot(repos), Part::Dot(you)];
        }
        Look::Line => {
            s.line = match st {
                State::Wait => vec![(1.0, WAIT)],
                State::Done => vec![(1.0, OK)],
                State::Idle => vec![(1.0, IDLE)],
                _ => {
                    let mut v: Vec<(f32, Rgba)> = Vec::new();
                    if let Some(t) = &f.timer {
                        // odpočet: úsek roste s uplynulým časem (reálná data), jinak třetina
                        v.push((t.progress.map(|p| p.clamp(0.05, 1.0) * 3.0).unwrap_or(1.0), f.accent));
                    }
                    if f.working > 0 {
                        v.push((1.0, RUN));
                    }
                    if let Some(m) = &f.music {
                        v.push((m.progress.map(|p| p.clamp(0.05, 1.0) * 3.0).unwrap_or(1.0), MUSIC));
                    }
                    let used: f32 = v.iter().map(|x| x.0).sum();
                    if used < 3.0 {
                        v.push((3.0 - used, IDLE));
                    }
                    v
                }
            };
        }
        Look::Icons => {
            let lead = match st {
                State::Wait => Some(if f.waiting > 0 { badge(Glyph::Bell, WAIT, f.waiting) } else { icon(Glyph::Bell, WAIT) }),
                State::Done => Some(icon(Glyph::Check, OK)),
                State::Timer => Some(Part::Icon {
                    glyph: Glyph::Timer,
                    color: f.accent,
                    badge: f.timer.as_ref().and_then(|t| t.minutes).map(|m| m.to_string()),
                }),
                State::Music => Some(icon(Glyph::Music, MUSIC)),
                _ => None,
            };
            s.parts.extend(lead);
            s.parts.push(badge(Glyph::Claude, if f.working > 0 { RUN } else { Rgba::DIM }, f.working));
            let srv = if f.server_failed { Rgba::RED } else if f.servers > 0 { OK } else { Rgba::DIM };
            s.parts.push(badge(Glyph::Server, srv, f.servers));
            s.parts.push(badge(Glyph::Repo, if f.repos > 0 { Rgba::TEXT } else { Rgba::DIM }, f.repos));
        }
        Look::Clock => {
            let lead = match st {
                State::Idle => None,
                State::Work => Some(Part::Dot(RUN)),
                State::Wait => Some(wait_lead(f)),
                State::Music => Some(icon(Glyph::Music, MUSIC)),
                State::Timer => Some(icon(Glyph::Timer, f.accent)),
                State::Done => Some(Part::Dot(OK)),
            };
            s.parts.extend(lead);
            if !f.clock.is_empty() {
                s.parts.push(strong(f.clock.clone(), Rgba::TEXT));
            }
        }
        Look::Corner => {
            s.parts = match st {
                State::Idle => {
                    let mut v = vec![Part::Dot(IDLE)];
                    if !f.clock.is_empty() {
                        v.push(text(f.clock.clone(), Rgba::TEXT));
                    }
                    v
                }
                State::Work => vec![Part::Dot(RUN), text(n_working(f.working), Rgba::TEXT)],
                State::Wait if f.waiting > 0 => vec![Part::Dot(WAIT), strong(trf!("čeká {}", "{} waiting", f.waiting), WAIT)],
                State::Wait => vec![icon(Glyph::Bell, WAIT), strong(tr("připomínka", "reminder"), WAIT)],
                State::Music => vec![icon(Glyph::Music, MUSIC), text(tr("hraje", "playing"), Rgba::TEXT)],
                State::Timer => vec![icon(Glyph::Timer, f.accent), text(timer_text(f), Rgba::TEXT)],
                State::Done => vec![icon(Glyph::Check, OK), text(tr("hotovo", "done"), Rgba::TEXT)],
            };
        }
        Look::Notch => {
            s.parts = match st {
                State::Idle => vec![],
                State::Work => vec![Part::Dot(RUN), text(f.working.to_string(), Rgba::TEXT)],
                State::Wait if f.waiting > 0 => vec![Part::Dot(WAIT), strong(trf!("{} čeká", "{} waiting", f.waiting), WAIT)],
                State::Wait => vec![icon(Glyph::Bell, WAIT)],
                State::Music => vec![icon(Glyph::Music, MUSIC)],
                State::Timer => {
                    let mut v = vec![icon(Glyph::Timer, f.accent)];
                    if let Some(m) = f.timer.as_ref().and_then(|t| t.minutes) {
                        v.push(text(m.to_string(), Rgba::TEXT));
                    }
                    v
                }
                State::Done => vec![icon(Glyph::Check, OK)],
            };
        }
        Look::Bar => {
            let lead: Vec<Part> = match st {
                State::Idle => vec![text(if f.mode.is_empty() { "ledge".to_string() } else { f.mode.to_lowercase() }, BAR_TEXT)],
                State::Work => vec![text(trf!("claude ▸ {}", "claude ▸ {}", n_working(f.working)), BAR_TEXT)],
                State::Wait if f.waiting > 0 => vec![strong(trf!("claude ▸ ČEKÁ {}", "claude ▸ WAITING {}", f.waiting), WAIT)],
                State::Wait => vec![icon(Glyph::Bell, WAIT), strong(wait_text(f), WAIT)],
                State::Music => vec![icon(Glyph::Music, MUSIC), text(music_text(f).to_lowercase(), BAR_TEXT)],
                State::Timer => vec![icon(Glyph::Timer, f.accent), text(timer_text(f), BAR_TEXT)],
                State::Done => vec![icon(Glyph::Check, OK), text(done_text(f).to_lowercase(), BAR_TEXT)],
            };
            s.parts = lead;
            if f.servers > 0 || f.server_failed {
                s.parts.push(Part::Sep);
                let c = if f.server_failed { Rgba::RED } else { BAR_TEXT };
                s.parts.push(text(format!("srv {}", f.servers), c));
            }
            if f.repos > 0 {
                s.parts.push(Part::Sep);
                s.parts.push(text(format!("repo {}", f.repos), BAR_TEXT));
            }
            let mut right = Vec::new();
            if let Some(p) = f.limit5h {
                right.push(format!("5h {:.0} %", p.floor()));
            }
            if !f.clock.is_empty() {
                right.push(f.clock.clone());
            }
            if !right.is_empty() {
                s.right = vec![text(right.join(" · "), BAR_TEXT)];
            }
        }
        Look::Progress => {
            let limit = f.limit5h.map(|p| ((p / 100.0).clamp(0.0, 1.0) as f32, format!("{:.0} %", p.floor())));
            let (left, right, bar): (Part, Option<String>, Option<(f32, Rgba)>) = match st {
                State::Idle => match &limit {
                    Some((x, t)) => (text(tr("Limit 5h", "5h limit"), Rgba::TEXT), Some(t.clone()), Some((*x, f.accent))),
                    None => (text(f.clock.clone(), Rgba::TEXT), None, None),
                },
                State::Work => (
                    text(work_text(f), Rgba::TEXT),
                    limit.as_ref().map(|(_, t)| format!("5h {t}")),
                    limit.as_ref().map(|(x, _)| (*x, RUN)),
                ),
                State::Wait => (strong(wait_text(f), WAIT), None, None),
                State::Music => {
                    let m = f.music.clone().unwrap_or_default();
                    (text(m.text, Rgba::TEXT), m.time, m.progress.map(|p| (p, MUSIC)))
                }
                State::Timer => {
                    let t = f.timer.clone().unwrap_or_default();
                    (text(tr("Časovač", "Timer"), Rgba::TEXT), Some(t.text), t.progress.map(|p| (p, f.accent)))
                }
                State::Done => (text(tr("Hotovo", "Done"), OK), (!f.done_where.is_empty()).then(|| f.done_where.clone()), None),
            };
            s.parts = vec![left];
            s.right = right.into_iter().map(|t| text(t, Rgba::DIM)).collect();
            s.progress = bar;
        }
    }
    s
}

/// „14:32“ z místního času.
pub fn hm(hour: u16, minute: u16) -> String {
    format!("{hour:02}:{minute:02}")
}
