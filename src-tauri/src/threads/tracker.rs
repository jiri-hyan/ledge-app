//! Čistý stav hover logiky per monitor (kolo 34, L178): cíl (monitor + zóna proužku / tečky), jeden krok
//! `PerMonitor::step` (dwell otevření, odjetí zavírá) a výběr aktivního cíle podle kurzoru. Žádné OS volání ani
//! čas z hodin — `now` přichází zvenku, lazy OS dotazy (`blocked`) jako closure. Smyčka `hover.rs` drží `Tracker`
//! a vykonává `Out`; jeden cíl = přesně dnešní chování (hlídá `legacy_equivalence`).
//!
//! Aktivní cíl: otevřený šuplík drží svůj monitor (`open`), jinak cíl pod kurzorem, jinak hlavní (index 0).
//! Změna aktivního cíle = reset stavu opuštěného (dwell uprostřed přechodu mezi monitory se zahodí).

use crate::config::Trigger;
use crate::drawer::{Opener, View};
use crate::edge::{self, Delays, Push, PushTracker};
use crate::hovermode::{self, HoverMode};
use crate::screen::Monitor;
use crate::shelf::Arm;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::RECT;

/// Monitor se zónou hoveru: hlavní proužek (index 0) nebo tečka na vedlejším monitoru.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub handle: isize,
    /// celý monitor (souřadnice virtuální plochy, i záporné)
    pub rect: RECT,
    pub dpi: u32,
    /// zóna u horní hrany: vlevo, vpravo, spodek (`looks::frame::zone`)
    pub zl: i32,
    pub zr: i32,
    pub zb: i32,
}

impl Target {
    /// Cíl bez zóny (jen geometrie monitoru): výběr monitoru pro zkratku.
    pub fn of(m: &Monitor) -> Target {
        Target { handle: m.handle, rect: m.rect, dpi: m.dpi, zl: m.rect.left, zr: m.rect.right, zb: m.rect.top }
    }
    pub fn scale(&self) -> i32 {
        self.dpi as i32
    }
    pub fn in_x(&self, x: i32) -> bool {
        x >= self.zl && x <= self.zr
    }
    pub fn at_edge(&self, x: i32, y: i32) -> bool {
        self.in_x(x) && y <= self.rect.top + 1 && y >= self.rect.top - 2
    }
    pub fn on_strip(&self, x: i32, y: i32) -> bool {
        self.in_x(x) && y >= self.rect.top && y < self.zb
    }
}

/// Index monitoru, který obsahuje bod (půlotevřeně: levý/horní okraj ano, pravý/dolní ne); mimo všechny `None`.
pub fn monitor_at(x: i32, y: i32, targets: &[Target]) -> Option<usize> {
    targets.iter().position(|t| x >= t.rect.left && x < t.rect.right && y >= t.rect.top && y < t.rect.bottom)
}

/// Který cíl hlídat: otevřený šuplík drží svůj monitor (`open`), jinak cíl pod kurzorem, jinak hlavní (0).
pub fn pick_active(p: (i32, i32), targets: &[Target], open: Option<usize>) -> usize {
    open.filter(|&i| i < targets.len()).or_else(|| monitor_at(p.0, p.1, targets)).unwrap_or(0)
}

/// Na kterém monitoru se šuplík otevře (`handle`): otevřený drží svůj; `Hover` věří monitoru, který nastavila
/// hover smyčka těsně před `send_ui` (`pending`); výslovná akce člověka (zkratka) = monitor pod kurzorem, jen
/// mezi `choices` (hlavní + tečky); jinak (ctl, MCP, kurzor mimo) hlavní.
pub fn choose_monitor(open: bool, opened_on: Option<isize>, opener: Opener, cursor: Option<isize>, main: isize, choices: &[isize]) -> isize {
    let known = |h: isize| choices.contains(&h);
    if open && let Some(h) = opened_on.filter(|&h| known(h)) {
        return h;
    }
    match opener {
        Opener::Hover => opened_on.filter(|&h| known(h)).unwrap_or(main),
        Opener::Hotkey => cursor.filter(|&h| known(h)).unwrap_or(main),
        Opener::Remote | Opener::Pinned => main,
    }
}

/// Vstupy jednoho kroku, které nepatří cíli (konfigurace, stav šuplíku, okolí).
pub struct Ctx {
    pub view: View,
    pub opener: Opener,
    pub trigger: Trigger,
    pub delays: Delays,
    pub close_ms: u64,
    pub hover_mode: HoverMode,
    pub push: Push,
    /// lištové okno v popředí + tlak (jen hlavní cíl)
    pub bar_push: bool,
    /// Claude čeká (`claude.attention`)
    pub attention: bool,
    /// tah souborů na přihrádku
    pub dropping: bool,
    /// kurzor na oznámení s tlačítkem (Notch)
    pub notch: bool,
    /// obdélník otevřeného šuplíku / peeku
    pub drawer: RECT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    /// otevřít pohled (`hover_target`)
    Open(View),
    /// myš odjela
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Out {
    /// proužek „nažhavit“
    pub hot: bool,
    pub action: Action,
}

/// Stav jednoho monitoru. `push` a `shelf` plní smyčka (raw input, tlačítko), `step` je čte a `push.fire()` volá sám.
#[derive(Debug, Default)]
pub struct PerMonitor {
    pub zone_since: Option<Instant>,
    pub outside_since: Option<Instant>,
    pub approach_since: Option<Instant>,
    pub push: PushTracker,
    pub shelf_arm: Arm,
}

fn inside(r: &RECT, x: i32, y: i32, m: i32) -> bool {
    x >= r.left - m && x <= r.right + m && y <= r.bottom + m
}

fn since(now: Instant, t: Instant) -> Duration {
    now.saturating_duration_since(t)
}

impl PerMonitor {
    /// Jeden tik. `blocked` = skrytý / zamčený kurzor (hra), ptá se jen u hrany v zóně a jen když je co blokovat.
    pub fn step(&mut self, t: &Target, p: (i32, i32), now: Instant, c: &Ctx, blocked: impl FnOnce() -> bool) -> Out {
        let (x, y) = p;
        match c.view {
            View::Hidden | View::Peek => {
                self.outside_since = None;
                let peek_hover = c.view == View::Peek && x >= c.drawer.left && x <= c.drawer.right && y >= c.drawer.top && y <= c.drawer.bottom;
                let (at_edge, on_strip) = (t.at_edge(x, y), t.on_strip(x, y));
                let armed = edge::armed(c.trigger, c.bar_push, at_edge, on_strip, c.push, &c.delays);
                let (armed, full) = edge::hover_open(armed, peek_hover, c.delays.dwell, c.attention);
                let blocked = armed.is_some() && (at_edge || on_strip) && blocked();
                let armed = if c.dropping || blocked || c.notch { None } else { armed };
                let pushing = c.bar_push && c.trigger != Trigger::Off && c.push != Push::Idle;
                let hot = (armed.is_some() || pushing) && !peek_hover;
                let mut action = Action::None;
                match armed {
                    Some(ms) => {
                        let s = *self.zone_since.get_or_insert(now);
                        if since(now, s) >= Duration::from_millis(ms) {
                            self.zone_since = None;
                            self.push.fire();
                            action = Action::Open(hovermode::hover_target(c.hover_mode, full, peek_hover));
                        }
                    }
                    None => self.zone_since = None,
                }
                Out { hot, action }
            }
            View::Full | View::Island | View::Mid => {
                self.zone_since = None;
                let mut action = Action::None;
                if hovermode::closes(c.opener, hovermode::CloseCause::MouseLeave) {
                    if inside(&c.drawer, x, y, 28 * t.scale() / 96) {
                        self.outside_since = None;
                    } else {
                        let s = *self.outside_since.get_or_insert(now);
                        if since(now, s) >= Duration::from_millis(c.close_ms) {
                            self.outside_since = None;
                            action = Action::Close;
                        }
                    }
                }
                Out { hot: false, action }
            }
        }
    }
}

/// Stavy všech cílů + který je aktivní.
#[derive(Debug, Default)]
pub struct Tracker {
    per: Vec<PerMonitor>,
    active: usize,
}

impl Tracker {
    /// Srovná počet stavů s počtem cílů a přepne aktivní cíl na `want`; opuštěný dostane čistý stav.
    /// `true` = aktivní cíl se změnil (nebo se změnila sestava).
    pub fn select(&mut self, want: usize, n: usize) -> bool {
        let mut changed = false;
        if self.per.len() != n {
            self.per.clear();
            self.per.resize_with(n, PerMonitor::default);
            changed = true;
        }
        if want != self.active {
            if let Some(old) = self.per.get_mut(self.active) {
                *old = PerMonitor::default();
            }
            self.active = want;
            changed = true;
        }
        changed
    }

    pub fn cur(&mut self) -> &mut PerMonitor {
        let i = self.active.min(self.per.len().saturating_sub(1));
        if self.per.is_empty() {
            self.per.push(PerMonitor::default());
        }
        &mut self.per[i]
    }

    /// Tlak na hranu do základu (změna lištového okna).
    pub fn reset_push(&mut self) {
        for p in &mut self.per {
            p.push = PushTracker::default();
        }
    }

    /// Všechny stavy do základu (mrtvý Ledge, kurzor nečitelný).
    pub fn reset_all(&mut self) {
        for p in &mut self.per {
            *p = PerMonitor::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(l: i32, t: i32, rr: i32, b: i32) -> RECT {
        RECT { left: l, top: t, right: rr, bottom: b }
    }
    fn tgt(handle: isize, rect: RECT) -> Target {
        // zóna 120 px kolem středu, spodek 30 px pod horní hranou
        let cx = (rect.left + rect.right) / 2;
        Target { handle, rect, dpi: 96, zl: cx - 60, zr: cx + 60, zb: rect.top + 30 }
    }
    fn ctx(view: View) -> Ctx {
        Ctx {
            view,
            opener: Opener::Hover,
            trigger: Trigger::Dwell,
            delays: Delays { edge: 100, dwell: 200, hover: 300 },
            close_ms: 400,
            hover_mode: HoverMode::Full,
            push: Push::Idle,
            bar_push: false,
            attention: false,
            dropping: false,
            notch: false,
            drawer: r(800, 0, 1100, 500),
        }
    }
    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn monitor_at_table() {
        // hlavní 0..1920 × 0..1080, vlevo (záporné x) a nad primárním (záporné y)
        let ts = [tgt(1, r(0, 0, 1920, 1080)), tgt(2, r(-1280, 0, 0, 1024)), tgt(3, r(0, -1080, 1920, 0))];
        for (p, want, what) in [
            ((10, 10), Some(0), "hlavní vlevo nahoře"),
            ((1919, 1079), Some(0), "poslední pixel hlavního"),
            ((1920, 10), None, "vpravo mimo"),
            ((-1, 10), Some(1), "levý monitor záporné x"),
            ((-1280, 0), Some(1), "levý okraj levého"),
            ((-1281, 0), None, "mimo vlevo"),
            ((100, -1), Some(2), "nad primárním"),
            ((100, -1080), Some(2), "horní okraj horního"),
            ((100, -1081), None, "mimo nahoře"),
            ((0, 0), Some(0), "roh patří hlavnímu (půlotevřeně)"),
        ] {
            assert_eq!(monitor_at(p.0, p.1, &ts), want, "{what}");
        }
        assert_eq!(monitor_at(0, 0, &[]), None);
    }

    #[test]
    fn pick_active_table() {
        let ts = [tgt(1, r(0, 0, 1920, 1080)), tgt(2, r(1920, 0, 3840, 1080))];
        for (p, open, want, what) in [
            ((2000, 5), None, 1, "kurzor na druhém"),
            ((10, 5), None, 0, "kurzor na hlavním"),
            ((5000, 5), None, 0, "mimo všechny = hlavní"),
            ((2000, 5), Some(0), 0, "otevřený šuplík drží svůj monitor"),
            ((10, 5), Some(1), 1, "otevřený na druhém, kurzor na hlavním"),
            ((10, 5), Some(7), 0, "neplatný index otevřeného"),
        ] {
            assert_eq!(pick_active(p, &ts, open), want, "{what}");
        }
    }

    #[test]
    fn choose_monitor_table() {
        let all = [1, 2, 3];
        for (open, opened, opener, cur, want, what) in [
            (true, Some(2), Opener::Hotkey, Some(3), 2, "otevřený drží svůj"),
            (false, Some(3), Opener::Hover, Some(1), 3, "hover věří nastavenému monitoru"),
            (false, None, Opener::Hover, Some(3), 1, "hover bez nastavení = hlavní"),
            (false, Some(9), Opener::Hover, None, 1, "neznámý monitor = hlavní"),
            (false, Some(3), Opener::Hotkey, Some(2), 2, "zkratka = pod kurzorem, ne starý stav"),
            (false, None, Opener::Hotkey, Some(9), 1, "kurzor na monitoru bez cíle = hlavní"),
            (false, None, Opener::Hotkey, None, 1, "kurzor nečitelný = hlavní"),
            (false, Some(3), Opener::Remote, Some(2), 1, "ctl = hlavní"),
            (false, None, Opener::Pinned, Some(2), 1, "upevněno = hlavní"),
            (true, Some(9), Opener::Hover, Some(2), 1, "otevřený na zmizelém monitoru = hlavní"),
        ] {
            assert_eq!(choose_monitor(open, opened, opener, cur, 1, &all), want, "{what}");
        }
    }

    #[test]
    fn tracker_resets_left_monitor_mid_dwell() {
        let base = Instant::now();
        let ts = [tgt(1, r(0, 0, 1920, 1080)), tgt(2, r(1920, 0, 3840, 1080))];
        let mut tr = Tracker::default();
        let c = ctx(View::Hidden);
        tr.select(0, 2);
        // kurzor v zóně hlavního: dwell běží
        let o = tr.cur().step(&ts[0], (960, 5), at(base, 0), &c, || false);
        assert!(o.hot && o.action == Action::None);
        assert!(tr.cur().zone_since.is_some());
        // přejede na druhý monitor v půlce dwellu → reset hlavního
        assert!(tr.select(1, 2));
        let o = tr.cur().step(&ts[1], (2880, 5), at(base, 150), &c, || false);
        assert!(o.hot && o.action == Action::None, "dwell druhého začíná od nuly");
        // zpět na hlavní: starý čas se nepamatuje, 200 ms od návratu
        assert!(tr.select(0, 2));
        let o = tr.cur().step(&ts[0], (960, 5), at(base, 250), &c, || false);
        assert_eq!(o.action, Action::None);
        let o = tr.cur().step(&ts[0], (960, 5), at(base, 460), &c, || false);
        assert_eq!(o.action, Action::Open(View::Full));
    }

    #[test]
    fn step_open_close_sequence() {
        let base = Instant::now();
        let t = tgt(1, r(0, 0, 1920, 1080));
        let mut pm = PerMonitor::default();
        let hid = ctx(View::Hidden);
        // mimo zónu: nic
        assert_eq!(pm.step(&t, (100, 5), at(base, 0), &hid, || false), Out { hot: false, action: Action::None });
        // v zóně: nažhaví, po dwell otevře
        assert_eq!(pm.step(&t, (960, 5), at(base, 10), &hid, || false).action, Action::None);
        assert_eq!(pm.step(&t, (960, 5), at(base, 209), &hid, || false).action, Action::None);
        assert_eq!(pm.step(&t, (960, 5), at(base, 210), &hid, || false).action, Action::Open(View::Full));
        // odjetí z otevřeného: po close_ms zavře, návrat dovnitř počítadlo ruší
        let open = ctx(View::Full);
        assert_eq!(pm.step(&t, (900, 300), at(base, 300), &open, || false).action, Action::None, "uvnitř");
        assert_eq!(pm.step(&t, (50, 700), at(base, 400), &open, || false).action, Action::None, "venku začíná");
        assert_eq!(pm.step(&t, (900, 300), at(base, 600), &open, || false).action, Action::None, "návrat ruší");
        assert_eq!(pm.step(&t, (50, 700), at(base, 700), &open, || false).action, Action::None);
        assert_eq!(pm.step(&t, (50, 700), at(base, 1100), &open, || false).action, Action::Close);
        // šuplík otevřený klávesou se myší nezavírá
        let mut hk = ctx(View::Full);
        hk.opener = Opener::Hotkey;
        assert_eq!(pm.step(&t, (50, 700), at(base, 2000), &hk, || false).action, Action::None);
        assert_eq!(pm.step(&t, (50, 700), at(base, 5000), &hk, || false).action, Action::None);
    }

    #[test]
    fn step_blocked_drop_notch_and_peek() {
        let base = Instant::now();
        let t = tgt(1, r(0, 0, 1920, 1080));
        let mut pm = PerMonitor::default();
        let c = ctx(View::Hidden);
        // zamčený kurzor (hra): neotevře
        assert!(!pm.step(&t, (960, 5), at(base, 0), &c, || true).hot);
        assert_eq!(pm.step(&t, (960, 5), at(base, 500), &c, || true).action, Action::None);
        // `blocked` se nevolá mimo zónu
        let mut asked = false;
        pm.step(&t, (100, 500), at(base, 600), &c, || {
            asked = true;
            false
        });
        assert!(!asked);
        // přihrádka a oznámení blokují
        let mut d = ctx(View::Hidden);
        d.dropping = true;
        assert_eq!(pm.step(&t, (960, 5), at(base, 700), &d, || false).action, Action::None);
        assert_eq!(pm.step(&t, (960, 5), at(base, 2000), &d, || false).action, Action::None);
        let mut n = ctx(View::Hidden);
        n.notch = true;
        assert_eq!(pm.step(&t, (960, 5), at(base, 3000), &n, || false).action, Action::None);
        // peek s čekajícím Claudem: najetí do peeku neotevře; bez něj otevře po dwell Full (nad peekem)
        let mut pk = ctx(View::Peek);
        pk.attention = true;
        assert_eq!(pm.step(&t, (900, 100), at(base, 4000), &pk, || false), Out { hot: false, action: Action::None });
        pk.attention = false;
        assert_eq!(pm.step(&t, (900, 100), at(base, 5000), &pk, || false).action, Action::None);
        assert_eq!(pm.step(&t, (900, 100), at(base, 5200), &pk, || false).action, Action::Open(View::Full));
    }

    #[test]
    fn push_trigger_uses_ready_only() {
        let base = Instant::now();
        let t = tgt(1, r(0, 0, 1920, 1080));
        let mut pm = PerMonitor::default();
        let mut c = ctx(View::Hidden);
        c.bar_push = true;
        c.push = Push::Charging;
        let o = pm.step(&t, (960, 0), at(base, 0), &c, || false);
        assert!(o.hot && o.action == Action::None, "nabíjení nažhaví, neotevře");
        c.push = Push::Ready;
        assert_eq!(pm.step(&t, (960, 0), at(base, 10), &c, || false).action, Action::Open(View::Full));
    }

    /// Dnešní vložená logika `hover.rs` (před kolem 34, fáze 1) — referenční kopie pro test ekvivalence.
    #[allow(clippy::too_many_arguments)]
    fn legacy(
        zone_since: &mut Option<Instant>,
        outside_since: &mut Option<Instant>,
        push: &mut PushTracker,
        t: &Target,
        p: (i32, i32),
        now: Instant,
        c: &Ctx,
        blocked_v: bool,
    ) -> Out {
        let (zl, zr, zb) = (t.zl, t.zr, t.zb);
        let in_x = p.0 >= zl && p.0 <= zr;
        let at_edge = in_x && p.1 <= t.rect.top + 1 && p.1 >= t.rect.top - 2;
        let on_strip = in_x && p.1 >= t.rect.top && p.1 < zb;
        let mut out = Out { hot: false, action: Action::None };
        match c.view {
            View::Hidden | View::Peek => {
                *outside_since = None;
                let peek_hover = c.view == View::Peek && p.0 >= c.drawer.left && p.0 <= c.drawer.right && p.1 >= c.drawer.top && p.1 <= c.drawer.bottom;
                let armed = edge::armed(c.trigger, c.bar_push, at_edge, on_strip, c.push, &c.delays);
                let (armed, full) = edge::hover_open(armed, peek_hover, c.delays.dwell, c.attention);
                let blocked = armed.is_some() && (at_edge || on_strip) && blocked_v;
                let armed = if c.dropping || blocked || c.notch { None } else { armed };
                let pushing = c.bar_push && c.trigger != Trigger::Off && c.push != Push::Idle;
                out.hot = (armed.is_some() || pushing) && !peek_hover;
                match armed {
                    Some(ms) => {
                        let s = *zone_since.get_or_insert(now);
                        if now.duration_since(s) >= Duration::from_millis(ms) {
                            *zone_since = None;
                            push.fire();
                            out.action = Action::Open(hovermode::hover_target(c.hover_mode, full, peek_hover));
                        }
                    }
                    None => *zone_since = None,
                }
            }
            View::Full | View::Island | View::Mid => {
                *zone_since = None;
                if hovermode::closes(c.opener, hovermode::CloseCause::MouseLeave) {
                    let r = c.drawer;
                    let m = 28 * t.scale() / 96;
                    let inside = p.0 >= r.left - m && p.0 <= r.right + m && p.1 <= r.bottom + m;
                    if inside {
                        *outside_since = None;
                    } else {
                        let s = *outside_since.get_or_insert(now);
                        if now.duration_since(s) >= Duration::from_millis(c.close_ms) {
                            *outside_since = None;
                            out.action = Action::Close;
                        }
                    }
                }
            }
        }
        out
    }

    /// Dlouhá deterministická sekvence tiků (16 ms) přes všechny pohledy, spouštěče a polohy: nový `step` musí
    /// dávat tytéž `Out` jako dnešní vložená logika, a to i se zasekávanou přihrádkou, tlakem a blokem.
    #[test]
    fn legacy_equivalence() {
        let base = Instant::now();
        let t = tgt(1, r(0, 0, 1920, 1080));
        let points = [(960, 0), (960, 1), (960, 5), (960, 29), (960, 31), (100, 5), (300, 900), (900, 300), (1130, 200), (1131, 530), (50, 700), (905, 3)];
        let views = [View::Hidden, View::Peek, View::Full, View::Island, View::Mid];
        let triggers = [Trigger::Hover, Trigger::Edge, Trigger::Dwell, Trigger::Off];
        let pushes = [Push::Idle, Push::Charging, Push::Ready];
        let mut seed = 0x2545F491u32;
        let mut rnd = move |n: usize| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as usize % n
        };
        let mut pm = PerMonitor::default();
        let (mut zs, mut os, mut lp) = (None, None, PushTracker::default());
        let mut ms = 0u64;
        let mut c = ctx(View::Hidden);
        let mut p = points[0];
        let mut opens = 0;
        let mut closes = 0;
        for i in 0..20_000 {
            ms += 16;
            // stav se mění zřídka, ať se dwell stihne dokončit; kurzor občas
            if i % 120 == 0 {
                c.view = views[rnd(views.len())];
                c.trigger = triggers[rnd(triggers.len())];
                c.opener = [Opener::Hover, Opener::Hotkey, Opener::Remote][rnd(3)];
                c.hover_mode = [HoverMode::Full, HoverMode::Island, HoverMode::Dock][rnd(3)];
                c.bar_push = rnd(4) == 0;
                c.attention = rnd(3) == 0;
                c.dropping = rnd(8) == 0;
                c.notch = rnd(8) == 0;
                c.push = pushes[rnd(3)];
            }
            if i % 45 == 0 {
                p = points[rnd(points.len())];
            }
            let blocked = rnd(10) == 0;
            let now = at(base, ms);
            let want = legacy(&mut zs, &mut os, &mut lp, &t, p, now, &c, blocked);
            let got = pm.step(&t, p, now, &c, || blocked);
            assert_eq!(got, want, "tik {i}: {:?} {:?} {:?}", c.view, p, c.trigger);
            assert_eq!(pm.zone_since, zs, "tik {i}: zone_since");
            assert_eq!(pm.outside_since, os, "tik {i}: outside_since");
            match got.action {
                Action::Open(_) => opens += 1,
                Action::Close => closes += 1,
                Action::None => {}
            }
        }
        assert!(opens > 20 && closes > 5, "sekvence musí pokrýt otevření ({opens}) i zavření ({closes})");
    }
}
