//! Widgety na plochu („Na plochu“): karta Ledge jako malé okno bez rámu na ploše / druhém monitoru,
//! žije i se zavřeným šuplíkem. Tady jsou čisté věci (tabulkové testy): druhy widgetů (`Kind`), stav
//! widgetu a soubor `desk.json` v `%LOCALAPPDATA%\Ledge` (poloha platí jen pro tento PC), profily
//! rozmístění, volby za běhu (`Patch`), viditelnost ve hře (`shown`, `on_top`), umístění po startu a po
//! odpojení monitoru (`plan_place` — volá `pins::restore` / `pins::monitor_of`), přichycení k okrajům
//! (`snap`) a config `desk` (výchozí chování). Okna: `win.rs`, data pro okna: `live.rs`, příkazy
//! `commands/desk.rs`, mapa `docs/codemap/desk.md`.
//!
//! Geometrie jako u lístků: pozice = vnější levý horní roh ve FYZICKÝCH px (tím je dán i monitor),
//! velikost = vnitřní v LOGICKÝCH px. `x`/`y` je „domov“ — místo, které zvolil uživatel. Když jeho monitor
//! chybí, okno stojí jinde (`win.rs` `DISPLACED`) a domov se nepřepíše; po připojení se vrátí.

pub(crate) mod live;
pub(crate) mod save;
pub(crate) mod win;

use crate::pins::{self, Mon, Place};
use crate::webpanel::Area;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Okna widgetů mají label `desk-<id>`.
pub const LABEL_PREFIX: &str = "desk-";
pub const FILE: &str = "desk.json";
/// Strop počtu widgetů. Každý widget je samostatné okno: sdílí s šuplíkem proces prohlížeče i (díky
/// `--renderer-process-limit=1`) renderer, ale má vlastní stránku, paměť JS a kompozici okna. Šest pokryje
/// všech pět druhů + jeden skript navíc a drží paměť v rozumných mezích (`docs/codemap/desk.md` → Okna).
pub const MAX_WIDGETS: usize = 6;
pub const MAX_PROFILES: usize = 8;
pub const NAME_MAX: usize = 40;
pub const MIN_W: f64 = 160.0;
pub const MIN_H: f64 = 48.0;
pub const MAX_W: f64 = 1600.0;
pub const MAX_H: f64 = 1400.0;
/// Výška lišty widgetu (logické px) — za ni se chytá.
pub const BAR_H: f64 = 24.0;
pub const OPACITY_MIN: u8 = 30;
/// Přichycení k okraji pracovní plochy / jiného widgetu, když je blíž než tohle (fyzické px na 96 dpi).
pub const SNAP_PX: f64 = 14.0;

/// Druh widgetu = co ukazuje. Text v souboru: `limits` · `waiting` · `dev` · `agenda` · `widget:<soubor>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// limity Clauda 5 h / týden
    Limits,
    /// schválení a otázky Clauda, které čekají na člověka
    Waiting,
    /// dev servery spuštěné Ledge
    Dev,
    /// kalendář dne
    Agenda,
    /// kolo 22: pás týdne s dnešními událostmi (karta Týden)
    Week,
    /// kolo 22: velké hodiny a datum (karta Velké hodiny)
    Clock,
    /// kolo 22: útrata a tokeny Claude dnes a za týden (karta Claude dnes)
    Usage,
    /// widget ze skriptu (`widgets/`), id `widget:<soubor>`
    Script(String),
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim() {
            "limits" => Some(Kind::Limits),
            "waiting" => Some(Kind::Waiting),
            "dev" => Some(Kind::Dev),
            "agenda" => Some(Kind::Agenda),
            "week" => Some(Kind::Week),
            "clock" => Some(Kind::Clock),
            "usage" => Some(Kind::Usage),
            id => crate::widgets::plan::file_of(id).map(|_| Kind::Script(id.to_string())),
        }
    }

    pub fn key(&self) -> String {
        match self {
            Kind::Limits => "limits".into(),
            Kind::Waiting => "waiting".into(),
            Kind::Dev => "dev".into(),
            Kind::Agenda => "agenda".into(),
            Kind::Week => "week".into(),
            Kind::Clock => "clock".into(),
            Kind::Usage => "usage".into(),
            Kind::Script(id) => id.clone(),
        }
    }

    /// Výchozí velikost (plná, kompaktní) v logických px.
    pub fn default_size(&self) -> ((f64, f64), (f64, f64)) {
        match self {
            Kind::Limits => ((320.0, 118.0), (230.0, 72.0)),
            Kind::Waiting => ((320.0, 190.0), (230.0, 72.0)),
            Kind::Dev => ((330.0, 190.0), (240.0, 96.0)),
            Kind::Agenda => ((310.0, 240.0), (250.0, 120.0)),
            Kind::Week => ((330.0, 150.0), (260.0, 96.0)),
            Kind::Clock => ((260.0, 150.0), (200.0, 96.0)),
            Kind::Usage => ((330.0, 230.0), (240.0, 110.0)),
            Kind::Script(_) => ((300.0, 210.0), (240.0, 110.0)),
        }
    }
}

/// Vrstva vůči ostatním oknům.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    /// navrchu přes všechna okna (ve hře ustoupí)
    Top,
    /// normální okno — schová se za aktivní okno
    #[default]
    Normal,
    /// jen na ploše — pod všemi okny
    Desktop,
}

/// Config `desk`: výchozí chování nových widgetů a společné volby. Seznam widgetů a jejich poloha jsou
/// v `desk.json` (jen tento PC), ne v configu — config se synchronizuje mezi PC.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct DeskConfig {
    /// Vrstva nového widgetu: `top` · `normal` (výchozí) · `desktop` (jen na ploše, pod okny).
    pub layer: Layer,
    /// Nový widget se schová ve hře přes celou obrazovku a v herním režimu. Výchozí true.
    pub hide_in_game: bool,
    /// Neprůhlednost nového widgetu v % (30–100). Výchozí 100.
    pub opacity: u8,
    /// Přichytit widget k okraji plochy a k jiným widgetům po přesunu. Výchozí true.
    pub snap: bool,
}

impl Default for DeskConfig {
    fn default() -> Self {
        Self { layer: Layer::Normal, hide_in_game: true, opacity: 100, snap: true }
    }
}

impl DeskConfig {
    /// Tolerantní čtení: pole se špatným typem / hodnotou = výchozí, průhlednost do 30–100. Rozbitá sekce
    /// `desk` (ruční úprava, starší / novější Ledge) tak nikdy neshodí načtení celého configu.
    pub fn from_value(v: &serde_json::Value) -> Self {
        let d = Self::default();
        let field = |k: &str| v.get(k);
        Self {
            layer: field("layer").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or(d.layer),
            hide_in_game: field("hideInGame").and_then(serde_json::Value::as_bool).unwrap_or(d.hide_in_game),
            opacity: field("opacity").and_then(serde_json::Value::as_f64).filter(|o| o.is_finite()).map_or(d.opacity, |o| o.clamp(OPACITY_MIN as f64, 100.0).round() as u8),
            snap: field("snap").and_then(serde_json::Value::as_bool).unwrap_or(d.snap),
        }
    }
}

impl<'de> Deserialize<'de> for DeskConfig {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(Self::from_value(&v))
    }
}

/// Jeden widget na ploše (řádek v `desk.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DeskWidget {
    /// stálé id (label okna `desk-<id>`)
    pub id: String,
    /// druh jako text (`Kind::key`)
    pub kind: String,
    /// domov: vnější pozice (fyzické px); `None` = ještě neumístěný
    pub x: Option<i32>,
    pub y: Option<i32>,
    /// plná velikost (logické px)
    pub w: f64,
    pub h: f64,
    /// kompaktní velikost (logické px)
    pub cw: f64,
    pub ch: f64,
    pub compact: bool,
    pub layer: Layer,
    /// zamknout: nejde posunout ani měnit velikost
    pub locked: bool,
    /// průchozí: klik projde na okno pod widgetem (jen ke čtení)
    pub through: bool,
    /// neprůhlednost 30–100 %
    pub opacity: u8,
    /// schovat ve hře přes celou obrazovku / v herním režimu
    pub hide_in_game: bool,
}

impl Default for DeskWidget {
    fn default() -> Self {
        let ((w, h), (cw, ch)) = Kind::Limits.default_size();
        Self { id: String::new(), kind: String::new(), x: None, y: None, w, h, cw, ch, compact: false, layer: Layer::Normal, locked: false, through: false, opacity: 100, hide_in_game: true }
    }
}

fn size(v: f64, lo: f64, hi: f64, def: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { def }
}

impl DeskWidget {
    pub fn new(id: &str, kind: &Kind, d: &DeskConfig) -> Self {
        let ((w, h), (cw, ch)) = kind.default_size();
        Self { id: id.into(), kind: kind.key(), w, h, cw, ch, layer: d.layer, opacity: d.opacity, hide_in_game: d.hide_in_game, ..Self::default() }
            .sanitized()
    }

    pub fn kind(&self) -> Option<Kind> {
        Kind::parse(&self.kind)
    }

    pub fn label(&self) -> String {
        label_for(&self.id)
    }

    /// Velikost, kterou okno teď má mít (logické px).
    pub fn size_now(&self) -> (f64, f64) {
        if self.compact { (self.cw, self.ch) } else { (self.w, self.h) }
    }

    pub fn sanitized(mut self) -> Self {
        let ((dw, dh), (dcw, dch)) = self.kind().map_or(Kind::Limits.default_size(), |k| k.default_size());
        self.w = size(self.w, MIN_W, MAX_W, dw);
        self.h = size(self.h, MIN_H, MAX_H, dh);
        self.cw = size(self.cw, MIN_W, MAX_W, dcw);
        self.ch = size(self.ch, MIN_H, MAX_H, dch);
        self.opacity = self.opacity.clamp(OPACITY_MIN, 100);
        if self.x.is_none() || self.y.is_none() {
            (self.x, self.y) = (None, None);
        }
        self
    }

    /// Lístek se stejnou geometrií — `pins::restore` umí obnovu na monitor i s DPI, znovupoužijeme ji.
    fn as_pin(&self) -> pins::Pin {
        let (w, h) = self.size_now();
        pins::Pin { path: String::new(), x: self.x, y: self.y, w, h, prefs: pins::PinPrefs { collapsed: false, ..Default::default() }, ..Default::default() }
    }
}

pub fn label_for(id: &str) -> String {
    format!("{LABEL_PREFIX}{id}")
}

/// Id z labelu okna (`desk-<id>` → `<id>`).
pub fn id_of(label: &str) -> Option<&str> {
    label.strip_prefix(LABEL_PREFIX).filter(|s| valid_id(s))
}

/// Id do labelu okna: jen `[a-z0-9]`, 1–16 znaků.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 16 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Nové id: krátké podle druhu + číslo, které ještě není.
pub fn new_id(kind: &Kind, taken: &[&str]) -> String {
    let base = match kind {
        Kind::Limits => "lim",
        Kind::Waiting => "wait",
        Kind::Dev => "dev",
        Kind::Agenda => "cal",
        Kind::Week => "week",
        Kind::Clock => "clock",
        Kind::Usage => "use",
        Kind::Script(_) => "wg",
    };
    (1..).map(|n| format!("{base}{n}")).find(|id| !taken.contains(&id.as_str())).expect("nekonečná řada")
}

/// Změna voleb z UI (chybějící pole = beze změny).
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Patch {
    pub layer: Option<Layer>,
    pub locked: Option<bool>,
    pub through: Option<bool>,
    pub opacity: Option<u8>,
    pub compact: Option<bool>,
    pub hide_in_game: Option<bool>,
}

impl Patch {
    pub fn apply(&self, w: &DeskWidget) -> DeskWidget {
        DeskWidget {
            layer: self.layer.unwrap_or(w.layer),
            locked: self.locked.unwrap_or(w.locked),
            through: self.through.unwrap_or(w.through),
            opacity: self.opacity.unwrap_or(w.opacity),
            compact: self.compact.unwrap_or(w.compact),
            hide_in_game: self.hide_in_game.unwrap_or(w.hide_in_game),
            ..w.clone()
        }
        .sanitized()
    }
}

/// Pojmenované rozmístění („Práce“, „Hraní“): které widgety a kde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub widgets: Vec<DeskWidget>,
}

/// Celý soubor `desk.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DeskFile {
    pub widgets: Vec<DeskWidget>,
    pub profiles: Vec<Profile>,
    /// naposledy obnovený / uložený profil (jen popisek v Nastavení)
    pub active: Option<String>,
}

/// Proč widget nejde přidat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddErr {
    Full,
    Unknown,
}

/// Platné widgety: známý druh, platné id, bez duplicit (id i druhu), nejvýš `MAX_WIDGETS`.
fn clean_widgets(v: Vec<DeskWidget>) -> Vec<DeskWidget> {
    let mut out: Vec<DeskWidget> = Vec::new();
    for w in v {
        let ok = w.kind().is_some() && valid_id(&w.id) && !out.iter().any(|o| o.id == w.id || o.kind == w.kind);
        if ok && out.len() < MAX_WIDGETS {
            out.push(w.sanitized());
        }
    }
    out
}

/// Stejné jméno profilu bez ohledu na velikost písmen (i s diakritikou: „PRÁCE“ = „Práce“).
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

pub fn clean_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(NAME_MAX).collect()
}

impl DeskFile {
    pub fn by_id(&self, id: &str) -> Option<&DeskWidget> {
        self.widgets.iter().find(|w| w.id == id)
    }

    pub fn by_id_mut(&mut self, id: &str) -> Option<&mut DeskWidget> {
        self.widgets.iter_mut().find(|w| w.id == id)
    }

    pub fn by_kind(&self, kind: &Kind) -> Option<&DeskWidget> {
        let k = kind.key();
        self.widgets.iter().find(|w| w.kind == k)
    }

    /// Připnout druh: existující widget téhož druhu se jen vrátí (`false`), jinak nový (`true`).
    pub fn add(&mut self, kind: &Kind, d: &DeskConfig) -> Result<(String, bool), AddErr> {
        if let Some(w) = self.by_kind(kind) {
            return Ok((w.id.clone(), false));
        }
        if self.widgets.len() >= MAX_WIDGETS {
            return Err(AddErr::Full);
        }
        let id = {
            let taken: Vec<&str> = self.widgets.iter().map(|w| w.id.as_str()).chain(self.profiles.iter().flat_map(|p| p.widgets.iter().map(|w| w.id.as_str()))).collect();
            new_id(kind, &taken)
        };
        self.widgets.push(DeskWidget::new(&id, kind, d));
        Ok((id, true))
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.widgets.len();
        self.widgets.retain(|w| w.id != id);
        before != self.widgets.len()
    }

    /// Uložit dnešní rozmístění pod jménem (stejné jméno = přepsat). `Err` = plno / prázdné jméno.
    pub fn save_profile(&mut self, name: &str) -> Result<String, AddErr> {
        let name = clean_name(name);
        if name.is_empty() {
            return Err(AddErr::Unknown);
        }
        let snap = Profile { name: name.clone(), widgets: self.widgets.clone() };
        match self.profiles.iter().position(|p| same_name(&p.name, &name)) {
            Some(i) => self.profiles[i] = snap,
            None if self.profiles.len() >= MAX_PROFILES => return Err(AddErr::Full),
            None => self.profiles.push(snap),
        }
        self.active = Some(name.clone());
        Ok(name)
    }

    /// Obnovit profil: widgety = jeho widgety. Vrací rozdíl pro okna (`None` = profil neexistuje).
    pub fn apply_profile(&mut self, name: &str) -> Option<Diff> {
        let p = self.profiles.iter().find(|p| same_name(&p.name, name))?.clone();
        let diff = diff(&self.widgets, &p.widgets);
        self.widgets = p.widgets;
        self.active = Some(p.name);
        Some(diff)
    }

    pub fn delete_profile(&mut self, name: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| !same_name(&p.name, name));
        if self.active.as_deref().is_some_and(|a| same_name(a, name)) {
            self.active = None;
        }
        before != self.profiles.len()
    }
}

/// Co udělat s okny při přechodu ze `cur` na `target` (id widgetů).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    /// zavřít (v cíli nejsou)
    pub close: Vec<String>,
    /// přesunout / přenastavit (jsou v obou)
    pub keep: Vec<String>,
    /// otevřít (jsou jen v cíli)
    pub open: Vec<String>,
}

pub fn diff(cur: &[DeskWidget], target: &[DeskWidget]) -> Diff {
    let has = |v: &[DeskWidget], id: &str| v.iter().any(|w| w.id == id);
    Diff {
        close: cur.iter().filter(|w| !has(target, &w.id)).map(|w| w.id.clone()).collect(),
        keep: target.iter().filter(|w| has(cur, &w.id)).map(|w| w.id.clone()).collect(),
        open: target.iter().filter(|w| !has(cur, &w.id)).map(|w| w.id.clone()).collect(),
    }
}

/// Stav widgetů (chybějící / poškozený soubor = žádné; neznámé druhy, duplicity a nesmysly pryč).
pub fn load(path: &Path) -> DeskFile {
    let raw: DeskFile = crate::fsx::load_json_or_default(path);
    parse_clean(raw)
}

fn parse_clean(raw: DeskFile) -> DeskFile {
    let mut profiles: Vec<Profile> = Vec::new();
    for p in raw.profiles {
        let name = clean_name(&p.name);
        if name.is_empty() || profiles.iter().any(|o| same_name(&o.name, &name)) || profiles.len() >= MAX_PROFILES {
            continue;
        }
        profiles.push(Profile { name, widgets: clean_widgets(p.widgets) });
    }
    let active = raw.active.map(|a| clean_name(&a)).filter(|a| profiles.iter().any(|p| same_name(&p.name, a)));
    DeskFile { widgets: clean_widgets(raw.widgets), profiles, active }
}

/// Zápis celého souboru (tmp + přejmenování — nikdy půlka souboru).
pub fn save(path: &Path, f: &DeskFile) -> std::io::Result<()> {
    crate::fsx::save_json(path, f)
}

// ---------- viditelnost ----------

/// Je widget vidět? Schovaný všemi (paleta / Nastavení) ne; ve hře (`yielding`) jen s vypnutým
/// „schovat ve hře“.
pub fn shown(w: &DeskWidget, yielding: bool, hidden_all: bool) -> bool {
    !hidden_all && !(yielding && w.hide_in_game)
}

/// Má být navrchu? Jen vrstva Navrchu a ne ve hře (hře vždy ustoupí, i když zůstane vidět).
pub fn on_top(w: &DeskWidget, yielding: bool) -> bool {
    w.layer == Layer::Top && !yielding
}

/// Jde widget posunout / měnit velikost?
pub fn movable(w: &DeskWidget) -> bool {
    !w.locked && !w.through
}

// ---------- geometrie ----------

/// Kam okno dát po startu / obnově profilu: domov na monitoru, kde leží (`pins::restore`); domov mimo
/// všechny monitory (odpojený) → na monitor proužku a `true` = je „mimo domov“ (domov se nepřepíše).
pub fn plan_place(w: &DeskWidget, mons: &[Mon], fallback: &Mon, top_gap: i32, index: usize) -> (Place, bool) {
    let p = w.as_pin();
    let place = pins::restore(&p, mons, fallback, top_gap, index);
    (place, w.x.is_some() && !home_on_screen(w, mons, fallback.scale))
}

/// Leží lišta domova na některém monitoru (dost na to, aby šla chytit)?
pub fn home_on_screen(w: &DeskWidget, mons: &[Mon], scale: f64) -> bool {
    let (Some(x), Some(y)) = (w.x, w.y) else { return false };
    let width = (w.size_now().0 * scale).round() as i32;
    pins::monitor_of(x, y, width, (BAR_H * scale).round() as i32, mons).is_some()
}

/// Přichycení: hrana okna blíž než `dist` k hraně pracovní plochy nebo k hraně jiného widgetu → přilepit.
/// `r` = (x, y, w, h) fyzicky, `others` = obdélníky ostatních widgetů (l, t, r, b). Vrací novou pozici.
pub fn snap(r: (i32, i32, i32, i32), work: Area, others: &[Area], dist: i32) -> (i32, i32) {
    let (x, y, w, h) = r;
    let pick = |pos: i32, size: i32, edges: &[i32]| -> i32 {
        let mut best: Option<(i32, i32)> = None;
        for &e in edges {
            for cand in [e, e - size] {
                let d = (pos - cand).abs();
                if d <= dist && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, cand));
                }
            }
        }
        best.map_or(pos, |(_, c)| c)
    };
    let mut xs = vec![work.l, work.r];
    let mut ys = vec![work.t, work.b];
    for o in others {
        // jen soused, který se v druhé ose aspoň trochu překrývá (jinak by se lepilo přes půl plochy)
        if o.t < y + h && o.b > y {
            xs.extend([o.l, o.r]);
        }
        if o.l < x + w && o.r > x {
            ys.extend([o.t, o.b]);
        }
    }
    (pick(x, w, &xs), pick(y, h, &ys))
}

#[cfg(test)]
mod tests;
