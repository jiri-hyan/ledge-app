//! Rozhodnutí „otevřít šuplík myší?“ — čistá logika bez WinAPI volání, aby šla pokrýt tabulkou případů.
//!
//! Hover vlákno (`lib.rs`) změří polohu, tlačítko, raw pohyb a okno v popředí a sem pošle jen čísla
//! a příznaky. „Lištové“ okno (maximalizovaný prohlížeč s taby nahoře, viz `config::BarWindowsConfig`)
//! mění pravidla: najetí ani stání na hraně neplatí, otevírá jen skutečný TLAK myši nahoru
//! (raw input, `rawpush.rs`); kde tlak měřit nejde (tablet, vzdálená plocha), souvislé stání na hraně.

use crate::config::Trigger;
use windows::Win32::Foundation::RECT;

/// Tolerance pohybu v ose x během tlačení (logické px) — tlačení, ne projíždění po tabech.
pub const PUSH_SLACK_PX: i32 = 4;
/// V jakém okně (ms) se sčítá tlak — pomalé „sunutí“ po hraně se neposčítá do otevření.
pub const PUSH_WINDOW_MS: u64 = 600;
/// Pohyb nahoru v prvních ms na hraně se nepočítá: setrvačnost rychlého švihu k tabům
/// (kurzor se zastaví na hraně, ruka ještě dojíždí) není úmyslný tlak.
pub const PUSH_SETTLE_MS: u64 = 80;
/// Kolik ms musí být lištové okno trvale pryč, než proužek zase vyroste (proti bliknutí).
pub const BAR_OFF_HOLD_MS: u64 = 300;
/// Spodní meze hodnot z configu (příliš malé by otvíraly šuplík omylem).
pub const MIN_PUSH_MS: u64 = 100;
pub const MIN_PUSH_DISTANCE: u32 = 10;

/// Prodlevy (ms) z `DrawerConfig` — v hover smyčce se čtou jednou za tik, bez klonování configu.
#[derive(Debug, Clone, Copy)]
pub struct Delays {
    pub edge: u64,
    pub dwell: u64,
    pub hover: u64,
}

/// Parametry tlačení (z `drawer.barWindows`, už ořezané na rozumné minimum).
#[derive(Debug, Clone, Copy)]
pub struct PushCfg {
    /// záložní pravidlo (bez relativního raw inputu): jak dlouho stát na hraně
    pub ms: u64,
    /// kolik raw counts pohybu nahoru je „zatlačení“
    pub distance: u32,
    /// tolerance v x (fyzické px)
    pub slack_px: i32,
}

impl PushCfg {
    pub fn new(push_ms: u64, push_distance: u32, slack_px: i32) -> Self {
        Self { ms: push_ms.max(MIN_PUSH_MS), distance: push_distance.max(MIN_PUSH_DISTANCE), slack_px }
    }
}

/// Je okno v popředí „lištové“? `exe` = cesta nebo jen název exe; porovnává se název, bez ohledu na velikost.
pub fn is_bar_window(exe: &str, maximized: bool, on_strip_monitor: bool, processes: &[String]) -> bool {
    if !maximized || !on_strip_monitor {
        return false;
    }
    let name = exe.rsplit(['\\', '/']).next().unwrap_or("").trim();
    !name.is_empty() && processes.iter().any(|p| p.trim().eq_ignore_ascii_case(name))
}

/// Navazuje nad monitorem proužku (v x zóny proužku) jiný monitor? Pak kurzor hranou projde
/// a tlačit nejde — bar režim se pro tenhle monitor nepoužije. `zl..=zr` = zóna v px.
pub fn edge_passes_through(zl: i32, zr: i32, top: i32, monitors: &[RECT]) -> bool {
    monitors.iter().any(|m| m.top < top && m.bottom >= top && m.left <= zr && m.right > zl)
}

/// Tlačí kurzor právě teď na hranu? Přesně na horním řádku monitoru, v zóně proužku, bez tlačítka.
pub fn pressing(in_x: bool, y: i32, monitor_top: i32, button_down: bool) -> bool {
    in_x && y == monitor_top && !button_down
}

/// Jeden tik hover vlákna pro `PushTracker`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PushTick {
    pub now_ms: u64,
    pub x: i32,
    /// viz `pressing`
    pub pressing: bool,
    /// raw counts pohybu nahoru od minulého tiku
    pub up: u32,
    /// mezi tiky proběhl klik
    pub clicked: bool,
    /// tlak jde měřit (raw input běží a zařízení je relativní); jinak záložní časové pravidlo
    pub relative: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Push {
    /// netlačí (nebo jen stojí na hraně)
    Idle,
    /// tlak se sčítá → proužek „nažhaví“
    Charging,
    /// dotlačeno → otevřít
    Ready,
}

const RING: usize = 64;

/// Měří tlak na horní hranu. Pohyb v x nad toleranci (od místa, kde tlačení začalo) nebo klik ho
/// začne měřit znovu, opuštění hrany ho zruší. Po otevření (`fire`) se znovu nenabije, dokud myš
/// hranu neopustí — zavřený šuplík (Esc) se tak hned neotevře zase. Bez alokací (kruhový buffer).
#[derive(Debug)]
pub struct PushTracker {
    since_ms: Option<u64>,
    anchor_x: i32,
    spent: bool,
    /// (čas, counts) pohybu nahoru, jen nenulové tiky
    ring: [(u64, u32); RING],
    head: usize,
}

impl Default for PushTracker {
    fn default() -> Self {
        Self { since_ms: None, anchor_x: 0, spent: false, ring: [(0, 0); RING], head: 0 }
    }
}

impl PushTracker {
    fn restart(&mut self, now_ms: u64, x: i32) {
        self.since_ms = Some(now_ms);
        self.anchor_x = x;
        self.ring = [(0, 0); RING];
    }

    pub fn step(&mut self, t: PushTick, cfg: &PushCfg) -> Push {
        if !t.pressing {
            *self = Self::default();
            return Push::Idle;
        }
        if self.spent {
            return Push::Idle;
        }
        let fresh = match self.since_ms {
            Some(_) if t.clicked => true,
            Some(_) => (t.x - self.anchor_x).abs() > cfg.slack_px,
            None => true,
        };
        if fresh {
            // pohyb, který kurzor na hranu (nebo na nové místo) teprve dovezl, se nepočítá
            self.restart(t.now_ms, t.x);
            return if t.relative { Push::Idle } else { Push::Charging };
        }
        let since = self.since_ms.unwrap_or(t.now_ms);
        let held = t.now_ms.saturating_sub(since);
        if !t.relative {
            return if held >= cfg.ms { Push::Ready } else { Push::Charging };
        }
        if t.up > 0 && held >= PUSH_SETTLE_MS {
            self.ring[self.head] = (t.now_ms, t.up);
            self.head = (self.head + 1) % RING;
        }
        let sum: u32 = self
            .ring
            .iter()
            .filter(|&&(at, c)| c > 0 && t.now_ms.saturating_sub(at) <= PUSH_WINDOW_MS)
            .map(|&(_, c)| c)
            .sum();
        if sum >= cfg.distance {
            Push::Ready
        } else if sum > 0 {
            Push::Charging
        } else {
            Push::Idle
        }
    }

    /// Šuplík se otevřel → další otevření až po opuštění hrany.
    pub fn fire(&mut self) {
        let fresh = Self::default();
        *self = Self { spent: true, ..fresh };
    }
}

/// Hystereze stavu „lištové okno v popředí“: zapne se hned, vypne až po `hold_ms` trvalého false.
/// `None` (popředí je okno Ledge, nebo žádné) = drž poslední stav — otevření/zavření šuplíku
/// tak proužek nerozbliká a po Esc se šuplík neotevře znovu najetím.
#[derive(Debug, Default)]
pub struct BarLatch {
    on: bool,
    off_since: Option<u64>,
}

impl BarLatch {
    pub fn update(&mut self, now_ms: u64, sample: Option<bool>, hold_ms: u64) -> bool {
        match sample {
            None => self.off_since = None,
            Some(true) => {
                self.on = true;
                self.off_since = None;
            }
            Some(false) if self.on => {
                let since = *self.off_since.get_or_insert(now_ms);
                if now_ms.saturating_sub(since) >= hold_ms {
                    self.on = false;
                    self.off_since = None;
                }
            }
            Some(false) => {}
        }
        self.on
    }
}

/// Za kolik ms od vstupu do zóny otevřít (`None` = neotevírat).
/// `bar_push` = v popředí je lištové okno s triggerem "push"; pak rozhoduje jen `push`
/// (měří `PushTracker`) a výsledek je `Some(0)` = hned, jakmile je dotlačeno.
pub fn armed(trigger: Trigger, bar_push: bool, at_edge: bool, on_strip: bool, push: Push, d: &Delays) -> Option<u64> {
    if trigger == Trigger::Off {
        return None;
    }
    if bar_push {
        return (push == Push::Ready).then_some(0);
    }
    match trigger {
        Trigger::Off => None,
        Trigger::Edge => at_edge.then_some(d.edge),
        Trigger::Dwell => {
            if at_edge { Some(d.edge) } else if on_strip { Some(d.dwell) } else { None }
        }
        Trigger::Hover => {
            if at_edge { Some(d.edge.min(d.hover)) } else if on_strip { Some(d.hover) } else { None }
        }
    }
}

/// Co udělá najetí, když je šuplík skrytý nebo v peeku. Čeká-li Claude (otázka / schválení), zůstane
/// se u malého peeku: najetí na proužek ukáže peek a podržení nad peekem ho NErozbalí na celý šuplík.
/// Vrací (za kolik ms otevřít, otevřít celý šuplík?).
pub fn hover_open(armed: Option<u64>, peek_hover: bool, dwell_ms: u64, claude_waiting: bool) -> (Option<u64>, bool) {
    match (peek_hover, claude_waiting) {
        (true, true) => (None, false),
        (true, false) => (Some(dwell_ms), true),
        (false, waiting) => (armed, !waiting),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_open_table() {
        // (armed, peek_hover, waiting) → (ms, full)
        assert_eq!(hover_open(Some(140), false, 400, false), (Some(140), true)); // běžné najetí → celý
        assert_eq!(hover_open(Some(140), false, 400, true), (Some(140), false)); // Claude čeká → jen peek
        assert_eq!(hover_open(None, false, 400, true), (None, false));
        assert_eq!(hover_open(None, true, 400, false), (Some(400), true)); // podržení nad připomínkou → rozbalit
        assert_eq!(hover_open(Some(140), true, 400, true), (None, false)); // nad otázkou zůstat malý
    }

    const D: Delays = Delays { edge: 120, dwell: 450, hover: 140 };
    const P: PushCfg = PushCfg { ms: 300, distance: 60, slack_px: PUSH_SLACK_PX };

    fn browsers() -> Vec<String> {
        crate::config::BarWindowsConfig::default().processes
    }

    #[test]
    fn bar_window_detection_table() {
        let list = browsers();
        // (exe, maximalizované, na monitoru proužku, čekáno)
        let cases: &[(&str, bool, bool, bool)] = &[
            (r"C:\Users\x\AppData\Local\Programs\Opera GX\opera.exe", true, true, true),
            ("chrome.exe", true, true, true),
            (r"C:\Program Files\Mozilla Firefox\FIREFOX.EXE", true, true, true), // velikost písmen
            ("MsEdge.exe", true, true, true),
            ("chrome.exe", false, true, false), // nemaximalizované okno
            ("chrome.exe", true, false, false), // jiný monitor
            (r"C:\Program Files\Microsoft VS Code\Code.exe", true, true, false), // není ve výchozím seznamu
            ("", true, true, false),            // proces se nepodařilo zjistit
            ("notepad.exe", true, true, false), // neznámý proces
            ("chrome.exe.bak", true, true, false),
        ];
        for &(exe, max, mon, want) in cases {
            assert_eq!(is_bar_window(exe, max, mon, &list), want, "{exe} max={max} mon={mon}");
        }
        assert!(is_bar_window("Code.exe", true, true, &[" code.exe ".into()]));
    }

    #[test]
    fn monitor_above_strip_zone_table() {
        let r = |left, top, right, bottom| RECT { left, top, right, bottom };
        let main = r(0, 0, 2560, 1440);
        let (zl, zr) = (1100, 1460);
        // (ostatní monitory, čekáno)
        let cases: Vec<(Vec<RECT>, bool)> = vec![
            (vec![main], false),                                // jen jeden monitor
            (vec![main, r(2560, 0, 4480, 1080)], false),        // vedle, ne nad
            (vec![main, r(640, -1080, 2560, 0)], true),         // přímo nad zónou
            (vec![main, r(0, -1080, 1100, 0)], false),          // nad, ale končí před zónou (right exkluzivní)
            (vec![main, r(0, -1080, 1101, 0)], true),           // přesah 1 px do zóny
            (vec![main, r(1461, -1080, 3381, 0)], false),       // nad, ale až za zónou
            (vec![main, r(640, -2160, 2560, -1080)], false),    // vysoko, nenavazuje
            (vec![main, r(0, 1440, 2560, 2880)], false),        // pod
        ];
        for (mons, want) in cases {
            assert_eq!(edge_passes_through(zl, zr, 0, &mons), want, "{mons:?}");
        }
    }

    #[test]
    fn pressing_needs_exact_top_row_zone_and_no_button() {
        assert!(pressing(true, 0, 0, false));
        assert!(pressing(true, -1080, -1080, false)); // monitor nad hlavním
        assert!(!pressing(true, 1, 0, false)); // o řádek níž = najetí, ne tlačení
        assert!(!pressing(false, 0, 0, false)); // mimo zónu proužku
        assert!(!pressing(true, 0, 0, true)); // tlačítko dole (tažení tabu, výběr)
    }

    /// Tik: (ms, x, na hraně?, counts nahoru, klik?)
    type T = (u64, i32, bool, u32, bool);

    /// Projede sekvenci tiků a vrátí výsledek posledního.
    fn run(relative: bool, ticks: &[T]) -> Push {
        let mut tr = PushTracker::default();
        let mut last = Push::Idle;
        for &(now_ms, x, pressing, up, clicked) in ticks {
            last = tr.step(PushTick { now_ms, x, pressing, up, clicked, relative }, &P);
        }
        last
    }

    #[test]
    fn raw_push_table() {
        use Push::*;
        let cases: &[(&str, &[T], Push)] = &[
            ("stání na tabu bez tlaku (klik + tooltip)", &[(0, 500, true, 0, false), (400, 500, true, 0, false), (900, 500, true, 0, false)], Idle),
            ("souvislý tlak nahoru", &[(0, 500, true, 0, false), (100, 500, true, 20, false), (200, 500, true, 25, false), (300, 500, true, 20, false)], Ready),
            ("rozjetý tlak", &[(0, 500, true, 0, false), (100, 500, true, 20, false)], Charging),
            ("švih k tabům: dojezd na hraně se nepočítá", &[(0, 500, true, 200, false), (16, 500, true, 150, false), (64, 500, true, 90, false), (300, 500, true, 0, false)], Idle),
            ("okno vypršelo: tlak po kouskách s pauzou", &[(0, 500, true, 0, false), (100, 500, true, 40, false), (800, 500, true, 30, false)], Charging),
            ("klik mezi tiky vynuluje", &[(0, 500, true, 0, false), (100, 500, true, 50, false), (200, 500, true, 0, true), (300, 500, true, 30, false)], Charging),
            ("drift v x nad toleranci vynuluje", &[(0, 500, true, 0, false), (100, 500, true, 50, false), (200, 510, true, 30, false)], Idle),
            ("chvění v toleranci nevadí", &[(0, 500, true, 0, false), (100, 503, true, 30, false), (200, 497, true, 30, false)], Ready),
            ("opuštění hrany vynuluje", &[(0, 500, true, 0, false), (100, 500, true, 50, false), (150, 500, false, 0, false), (200, 500, true, 0, false), (300, 500, true, 30, false)], Charging),
            ("mimo hranu nic", &[(0, 500, false, 500, false)], Idle),
        ];
        for (name, ticks, want) in cases {
            assert_eq!(run(true, ticks), *want, "{name}");
        }
    }

    #[test]
    fn time_fallback_table() {
        use Push::*;
        // absolutní zařízení / bez raw inputu: rozhoduje souvislé stání
        assert_eq!(run(false, &[(0, 500, true, 0, false), (320, 500, true, 0, false)]), Ready);
        assert_eq!(run(false, &[(0, 500, true, 0, false), (150, 500, true, 0, false)]), Charging); // krátké držení
        assert_eq!(run(false, &[(0, 500, true, 0, false), (200, 520, true, 0, false), (400, 520, true, 0, false)]), Charging); // projíždění
        assert_eq!(run(false, &[(0, 500, true, 0, false), (200, 500, true, 0, true), (400, 500, true, 0, false)]), Charging); // klik
        assert_eq!(run(false, &[(0, 500, false, 0, false)]), Idle);
    }

    #[test]
    fn tracker_does_not_rearm_until_edge_is_left() {
        let mut tr = PushTracker::default();
        let tick = |now_ms, pressing, up| PushTick { now_ms, x: 500, pressing, up, clicked: false, relative: true };
        assert_eq!(tr.step(tick(0, true, 0), &P), Push::Idle);
        assert_eq!(tr.step(tick(100, true, 70), &P), Push::Ready);
        tr.fire();
        assert_eq!(tr.step(tick(200, true, 70), &P), Push::Idle);
        assert_eq!(tr.step(tick(300, false, 0), &P), Push::Idle);
        assert_eq!(tr.step(tick(400, true, 0), &P), Push::Idle);
        assert_eq!(tr.step(tick(500, true, 70), &P), Push::Ready);
    }

    #[test]
    fn push_cfg_clamps_too_small_values() {
        let c = PushCfg::new(0, 0, 4);
        assert_eq!((c.ms, c.distance), (MIN_PUSH_MS, MIN_PUSH_DISTANCE));
        let c = PushCfg::new(500, 80, 4);
        assert_eq!((c.ms, c.distance), (500, 80));
    }

    #[test]
    fn bar_latch_table() {
        let mut l = BarLatch::default();
        assert!(!l.update(0, Some(false), 300));
        assert!(l.update(10, Some(true), 300)); // zapne hned
        assert!(l.update(100, Some(false), 300)); // krátký výpadek drží
        assert!(l.update(200, None, 300)); // popředí = Ledge → drž
        assert!(l.update(300, Some(false), 300)); // odpočet začíná znovu
        assert!(l.update(550, Some(false), 300));
        assert!(!l.update(600, Some(false), 300)); // 300 ms trvale false → vypne
        assert!(!l.update(700, None, 300)); // drží i vypnuto
        assert!(l.update(800, Some(true), 300));
    }

    #[test]
    fn armed_table() {
        use Push::*;
        use Trigger::*;
        // (trigger, lištové okno, at_edge, on_strip, tlak, čekáno)
        type Case = (Trigger, bool, bool, bool, Push, Option<u64>);
        let cases: &[Case] = &[
            // mimo lištové okno beze změny
            (Hover, false, false, true, Idle, Some(140)),
            (Hover, false, true, true, Idle, Some(120)),
            (Hover, false, false, false, Idle, None),
            (Edge, false, false, true, Idle, None),
            (Edge, false, true, true, Idle, Some(120)),
            (Dwell, false, false, true, Idle, Some(450)),
            (Off, false, true, true, Ready, None),
            // lištové okno: najetí na proužek ani stání na hraně nic
            (Hover, true, false, true, Idle, None),
            (Hover, true, true, true, Idle, None),
            (Hover, true, true, true, Charging, None),
            // dotlačeno → hned
            (Hover, true, true, true, Ready, Some(0)),
            (Dwell, true, true, true, Ready, Some(0)),
            // Off vypíná i tlačení
            (Off, true, true, true, Ready, None),
        ];
        for &(trig, bar, edge, strip, push, want) in cases {
            assert_eq!(armed(trig, bar, edge, strip, push, &D), want, "{trig:?} bar={bar} edge={edge} strip={strip} push={push:?}");
        }
    }
}
