//! Fullscreen hlídač (dřív blok ve smyčce `hover.rs`): co 500 ms zjistí, co leží přes který monitor
//! (`screen::classify`), a uloží to do `Core.fs_mon` (stav per monitor) a `Core.fullscreen` (= „kdekoli“).
//! Běží ve smyčce vlákna `hover` (žádný vlastní časovač; jedno čtení QUNS a okna v popředí na tik).
//! Zdroj pravdy = okno v popředí + lepkavé pokrytí (`next_fs_map`): monitor s hrou zůstane „pokrytý“, i když
//! fokus přejde jinam (chat na druhém monitoru, vlastní okno Ledge), dokud hra žije a kryje monitor.
//!
//! Dvě vrstvy (šéf, L178): povrchy UI (proužek, šuplík, lístky, widgety, oznámení) jsou mrtvé jen na
//! monitoru s fullscreenem (`Core::dead_surface` / `dead_on`); práce na pozadí a globální zkratky při
//! JAKÉMKOLI fullscreenu (`Core::dead` = `Core.fullscreen`) — hra na A dostane CPU, síť i klávesy.
//! Rozřazení volajících `dead()` je v `docs/codemap/core-detail.md` (Mrtvý Ledge).

use crate::drawer::View;
use crate::pins::FsKind;
use crate::screen::{self, FgWin, FsState, Monitor};
use crate::state::{UiCmd, core, log, mark_dirty};
use crate::stripview::refresh_strip;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::RECT;

/// Stav jednoho monitoru: klíč (`screen::stable_spec`), HMONITOR, obdélník, co leží přes něj, druh pokrytí
/// (prohlížeč ve F11 ≠ hra; pro lístky a widgety) a HWND okna, které ho pokrývá (`cover`, 0 = nic; lepkavé
/// pokrytí).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FsMon {
    pub key: String,
    pub handle: isize,
    pub rect: RECT,
    pub state: FsState,
    pub kind: FsKind,
    pub cover: isize,
}

/// Čistě: další mapa stavů všech monitorů z jednoho čtení (QUNS + okno v popředí) a předchozí mapy.
/// Zdroj pravdy = okno v popředí + **lepkavé pokrytí**: monitor, který byl `Covering`, zůstane, dokud okno
/// (`cover`) žije (`probe` = `screen::window_state`), je viditelné, není minimalizované a stále kryje
/// monitor. Hra na A + fokus na chat na B, nebo klik do vlastního okna Ledge (v popředí `None`), tak A
/// neoživí. Sticky zruší jen okno v popředí, které samo kryje týž monitor (zakrývá hru). `fresh_kind` =
/// druh pokrytí čerstvě zjištěného okna v popředí (u sticky se drží předchozí `kind`). Exkluzivní QUNS bez
/// okna v popředí (vlastní okno) zůstane jen na monitorech, které už byly `Exclusive`; jinak na všech.
pub(crate) fn next_fs_map(mons: &[Monitor], exclusive: bool, fg: Option<FgWin>, prev: &[FsMon], probe: &dyn Fn(isize) -> Option<FgWin>, fresh_kind: FsKind) -> Vec<FsMon> {
    let any_ex_prev = prev.iter().any(|p| p.state == FsState::Exclusive);
    mons.iter()
        .map(|m| {
            let old = prev.iter().find(|p| p.handle == m.handle);
            let (state, kind, cover) = match screen::classify(exclusive, fg, m.handle, &m.rect) {
                FsState::Covering => (FsState::Covering, fresh_kind, fg.map_or(0, |f| f.hwnd)),
                FsState::Exclusive => {
                    if fg.is_none() && any_ex_prev && old.is_none_or(|o| o.state != FsState::Exclusive) {
                        (FsState::None, FsKind::None, 0)
                    } else {
                        (FsState::Exclusive, FsKind::Exclusive, 0)
                    }
                }
                FsState::None => match old {
                    Some(o) if o.state == FsState::Covering && o.cover != 0 && sticky_holds(o.cover, fg, probe, m) => (FsState::Covering, o.kind, o.cover),
                    _ => (FsState::None, FsKind::None, 0),
                },
            };
            FsMon { key: screen::stable_spec(&m.device_name(), &m.rect), handle: m.handle, rect: m.rect, state, kind, cover }
        })
        .collect()
}

/// Drží okno `cover` monitor `m` pořád pokrytý? Ne, když ho zakrývá jiné okno v popředí téhož monitoru, nebo
/// když zmizelo / zmenšilo se / se přesunulo (`probe`).
fn sticky_holds(cover: isize, fg: Option<FgWin>, probe: &dyn Fn(isize) -> Option<FgWin>, m: &Monitor) -> bool {
    if fg.is_some_and(|f| f.hwnd != cover && f.monitor == m.handle && screen::covers(&f.rect, &m.rect)) {
        return false;
    }
    probe(cover).is_some_and(|w| w.eligible && screen::covers(&w.rect, &m.rect))
}

/// Je povrch na monitoru `handle` mrtvý? Herní režim = vždy; jinak podle stavu monitoru. Monitor, který
/// mapa nezná (čerstvě připojený, mezi dvěma tiky), se řídí „kdekoli“ (`any_fs`) — konzervativně.
pub(crate) fn dead_on(game_mode: bool, any_fs: bool, map: &[FsMon], handle: isize) -> bool {
    game_mode || map.iter().find(|m| m.handle == handle).map_or(any_fs, |m| m.state != FsState::None)
}

/// Co leží přes monitor `handle` (pro `pins::pins_yield`); neznámý monitor při fullscreenu bere první známé
/// pokrytí.
pub(crate) fn kind_on(any_fs: bool, map: &[FsMon], handle: isize) -> FsKind {
    match map.iter().find(|m| m.handle == handle) {
        Some(m) => m.kind,
        None if any_fs => map.iter().map(|m| m.kind).find(|k| *k != FsKind::None).unwrap_or(FsKind::None),
        None => FsKind::None,
    }
}

/// HMONITOR monitoru, na kterém leží bod (levý horní roh okna); mimo všechny monitory `None`.
pub(crate) fn handle_at(map: &[FsMon], x: i32, y: i32) -> Option<isize> {
    map.iter().find(|m| x >= m.rect.left && x < m.rect.right && y >= m.rect.top && y < m.rect.bottom).map(|m| m.handle)
}

pub(crate) struct FsWatch {
    last_check: Instant,
}

impl Default for FsWatch {
    fn default() -> Self {
        Self { last_check: crate::guard::ago(Duration::from_secs(5)) }
    }
}

impl FsWatch {
    /// Jeden průchod smyčkou; sám se omezí na 500 ms. `mon` = monitor proužku, `mons` = všechny monitory.
    pub(crate) fn tick(&mut self, mon: &Monitor, mons: &[Monitor], fg: &mut screen::ForegroundProbe, _own_pid: u32) {
        if self.last_check.elapsed() <= Duration::from_millis(500) {
            return;
        }
        self.last_check = Instant::now();
        let c = core();
        let exclusive = screen::exclusive_now();
        let win = screen::foreground_window();
        // druh pokrytí: exe okna v popředí se čte jen při pokrytí a je v cache podle HWND — žádný nový polling
        let covering = !exclusive && win.is_some_and(|w| mons.iter().any(|m| screen::classify(false, Some(w), m.handle, &m.rect) == FsState::Covering));
        let cover = if covering {
            match fg.read() {
                Some(f) => crate::pins::cover_kind(f.exe),
                None => FsKind::CoverOther,
            }
        } else {
            FsKind::None
        };
        let prev = c.fs_mon.lock().clone();
        let map = next_fs_map(mons, exclusive, win, &prev, &screen::window_state, cover);
        let fs = map.iter().any(|m| m.state != FsState::None);
        let surface_was = c.dead_on(mon.handle);
        {
            let mut g = c.fs_mon.lock();
            if *g == map {
                return;
            }
            *g = map;
        }
        let any_changed = c.fullscreen.swap(fs, Ordering::Relaxed) != fs;
        let surface_changed = surface_was != c.dead_on(mon.handle);
        if any_changed {
            log(&format!("fullscreen: {fs}"));
        }
        if any_changed || surface_changed {
            if c.dead_on(mon.handle) && c.drawer.view() != View::Hidden && crate::hovermode::closes(c.drawer.opener(), crate::hovermode::CloseCause::Fullscreen) {
                c.send_ui(UiCmd::Hide);
            }
            refresh_strip();
        }
        // lístky a widgety „Vždy navrchu“ hře na jejím monitoru ustoupí (a pak se vrátí; prohlížeč ve F11 ne)
        crate::pins::win::sync_dead();
        crate::desk::win::sync_yield();
        crate::ext::on_dead_changed();
        if any_changed || surface_changed {
            // konec fullscreenu → emit vlákno hned dožene peek připomínky (`check_reminders`)
            mark_dirty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(l: i32, t: i32, w: i32, h: i32) -> RECT {
        RECT { left: l, top: t, right: l + w, bottom: t + h }
    }

    fn mon(handle: isize, rect: RECT) -> Monitor {
        let mut device = [0u16; 32];
        for (i, c) in format!("DISPLAY{handle}").encode_utf16().enumerate() {
            device[i] = c;
        }
        Monitor { handle, rect, work: rect, primary: handle == 1, dpi: 96, device }
    }

    fn fg(monitor: isize, rect: RECT, eligible: bool) -> Option<FgWin> {
        fgw(monitor * 100, monitor, rect, eligible)
    }

    fn fgw(hwnd: isize, monitor: isize, rect: RECT, eligible: bool) -> Option<FgWin> {
        Some(FgWin { hwnd, monitor, rect, eligible })
    }

    /// Mapa bez historie (jedno čtení) — dřívější `build_map`.
    fn build_map(mons: &[Monitor], exclusive: bool, fg: Option<FgWin>, cover: FsKind) -> Vec<FsMon> {
        next_fs_map(mons, exclusive, fg, &[], &|_| None, cover)
    }

    /// (monitor × druh fullscreenu × QUNS) → (povrch mrtvý na A, na B, pozadí mrtvé = kdekoli, druhy A, B)
    #[test]
    fn surface_and_background_table() {
        let (ra, rb) = (r(0, 0, 1920, 1080), r(1920, 0, 2560, 1440));
        let mons = [mon(1, ra), mon(2, rb)];
        use FsKind::{CoverBrowser, CoverOther, Exclusive};
        let none = FsKind::None;
        let cases = [
            ("nic", false, None, false, false, false, (none, none)),
            ("exkl. na A", true, fg(1, ra, true), true, false, true, (Exclusive, none)),
            ("exkl. na B", true, fg(2, rb, true), false, true, true, (none, Exclusive)),
            ("exkl. bez okna", true, None, true, true, true, (Exclusive, Exclusive)),
            ("borderless na B", false, fg(2, rb, true), false, true, true, (none, CoverOther)),
            ("F11 prohlížeč na A", false, fg(1, ra, true), true, false, true, (CoverBrowser, none)),
            ("maximalizované", false, fg(1, ra, false), false, false, false, (none, none)),
            ("menší okno", false, fg(2, r(1920, 0, 2000, 1000), true), false, false, false, (none, none)),
        ];
        for (name, qun, win, dead_a, dead_b, any, (ka, kb)) in cases {
            let cover = if name.contains("prohlížeč") { CoverBrowser } else { CoverOther };
            let map = build_map(&mons, qun, win, cover);
            let fs = map.iter().any(|m| m.state != FsState::None);
            assert_eq!(fs, any, "{name}: pozadí (kdekoli)");
            assert_eq!(dead_on(false, fs, &map, 1), dead_a, "{name}: povrch A");
            assert_eq!(dead_on(false, fs, &map, 2), dead_b, "{name}: povrch B");
            assert_eq!(kind_on(fs, &map, 1), ka, "{name}: druh A");
            assert_eq!(kind_on(fs, &map, 2), kb, "{name}: druh B");
            // herní režim zabije každý povrch, i bez fullscreenu
            assert!(dead_on(true, fs, &map, 1) && dead_on(true, fs, &map, 2), "{name}: herní režim");
        }
    }

    #[test]
    fn monitor_left_and_above_primary() {
        let (a, left, up) = (r(0, 0, 1920, 1080), r(-1920, 0, 1920, 1080), r(0, -1080, 1920, 1080));
        let mons = [mon(1, a), mon(2, left), mon(3, up)];
        let map = build_map(&mons, false, fg(3, up, true), FsKind::CoverOther);
        let dead: Vec<bool> = [1, 2, 3].iter().map(|h| dead_on(false, true, &map, *h)).collect();
        assert_eq!(dead, [false, false, true]);
        // klíče jsou stabilní a různé
        assert_eq!(map.iter().map(|m| m.key.as_str()).collect::<std::collections::HashSet<_>>().len(), 3);
    }

    #[test]
    fn handle_at_table() {
        let mons = [mon(1, r(0, 0, 1920, 1080)), mon(2, r(-1920, 0, 1920, 1080)), mon(3, r(0, -1080, 1920, 1080))];
        let map = build_map(&mons, false, None, FsKind::None);
        let cases = [((10, 10), Some(1)), ((-5, 10), Some(2)), ((10, -5), Some(3)), ((1920, 0), None), ((0, 0), Some(1)), ((-1920, 0), Some(2))];
        for ((x, y), want) in cases {
            assert_eq!(handle_at(&map, x, y), want, "{x},{y}");
        }
    }

    #[test]
    fn unknown_monitor_follows_anywhere() {
        let mons = [mon(1, r(0, 0, 1920, 1080))];
        let map = build_map(&mons, false, fg(1, r(0, 0, 1920, 1080), true), FsKind::CoverOther);
        // čerstvě připojený monitor 9 mapa nezná: při fullscreenu mrtvý (konzervativně), jinak živý
        assert!(dead_on(false, true, &map, 9));
        assert_eq!(kind_on(true, &map, 9), FsKind::CoverOther);
        assert!(!dead_on(false, false, &[], 9));
        assert_eq!(kind_on(false, &[], 9), FsKind::None);
    }

    #[test]
    fn one_monitor_equals_old_behaviour() {
        // jeden monitor: povrch mrtvý ⇔ kdekoli (dnešní `dead()` bez herního režimu)
        let full = r(0, 0, 1920, 1080);
        let mons = [mon(1, full)];
        for (qun, win) in [(false, None), (true, None), (true, fg(1, full, true)), (false, fg(1, full, true)), (false, fg(1, r(0, 0, 800, 600), true))] {
            let map = build_map(&mons, qun, win, FsKind::CoverOther);
            let any = map.iter().any(|m| m.state != FsState::None);
            assert_eq!(dead_on(false, any, &map, 1), any);
        }
    }

    /// Simulace tiků: (QUNS, okno v popředí, žijící okna pro `probe`) → (povrch A mrtvý, povrch B mrtvý).
    #[test]
    fn sticky_tick_sequences() {
        let (ra, rb) = (r(0, 0, 1920, 1080), r(1920, 0, 2560, 1440));
        let mons = [mon(1, ra), mon(2, rb)];
        let game = fgw(10, 1, ra, true).unwrap(); // hra borderless na A
        let chat = fgw(20, 2, r(2000, 100, 800, 600), false).unwrap(); // okno na B
        let chat_max = fgw(21, 2, rb, false).unwrap(); // maximalizovaný prohlížeč na B
        type Step = (&'static str, Option<FgWin>, Vec<FgWin>, (bool, bool));
        let steps: Vec<Step> = vec![
            ("hra v popředí", Some(game), vec![game], (true, false)),
            ("fokus A→B, hra žije", Some(chat), vec![game, chat], (true, false)),
            ("maximalizovaný prohlížeč na B", Some(chat_max), vec![game, chat_max], (true, false)),
            ("vlastní okno Ledge v popředí (None)", None, vec![game], (true, false)),
            ("zpět do hry", Some(game), vec![game], (true, false)),
            ("hra minimalizována / zavřena (probe None)", Some(chat), vec![chat], (false, false)),
            ("po zavření zůstane živý", None, vec![], (false, false)),
        ];
        let mut prev: Vec<FsMon> = vec![];
        for (name, win, alive, (dead_a, dead_b)) in steps {
            let probe = |h: isize| alive.iter().copied().find(|w| w.hwnd == h);
            prev = next_fs_map(&mons, false, win, &prev, &probe, FsKind::CoverOther);
            let any = prev.iter().any(|m| m.state != FsState::None);
            assert_eq!(dead_on(false, any, &prev, 1), dead_a, "{name}: A");
            assert_eq!(dead_on(false, any, &prev, 2), dead_b, "{name}: B");
        }
        let prev = build_map(&mons, false, Some(game), FsKind::CoverOther);
        // hra se zmenší (nepokrývá monitor) → sticky padne i při žijícím okně
        let small = FgWin { rect: r(0, 0, 800, 600), ..game };
        let map = next_fs_map(&mons, false, Some(chat), &prev, &|_| Some(small), FsKind::CoverOther);
        assert_eq!(map[0].state, FsState::None, "zmenšená hra");
        // jiné okno přes celý monitor A (maximalizované) hru zakryje → sticky padne
        let over = fgw(22, 1, ra, false);
        let map = next_fs_map(&mons, false, over, &prev, &|_| Some(game), FsKind::CoverOther);
        assert_eq!(map[0].state, FsState::None, "hru zakrylo maximalizované okno");
        // sticky drží druh pokrytí (prohlížeč F11 na A + fokus jinde zůstane CoverBrowser)
        let prev = build_map(&mons, false, Some(game), FsKind::CoverBrowser);
        let map = next_fs_map(&mons, false, Some(chat), &prev, &|_| Some(game), FsKind::CoverOther);
        assert_eq!((map[0].state, map[0].kind, map[0].cover), (FsState::Covering, FsKind::CoverBrowser, 10));
        // změna handle monitoru (hotplug) historii zahodí
        let mons2 = [mon(7, ra), mon(2, rb)];
        let map = next_fs_map(&mons2, false, Some(chat), &prev, &|_| Some(game), FsKind::CoverOther);
        assert_eq!(map[0].state, FsState::None, "nový handle");
    }

    #[test]
    fn window_across_two_monitors_covers_both() {
        let (ra, rb) = (r(0, 0, 1920, 1080), r(1920, 0, 1920, 1080));
        let mons = [mon(1, ra), mon(2, rb)];
        let map = build_map(&mons, false, fgw(5, 1, r(0, 0, 3840, 1080), true), FsKind::CoverOther);
        assert!(map.iter().all(|m| m.state == FsState::Covering));
        // okno přes půlku druhého monitoru: jen A
        let map = build_map(&mons, false, fgw(5, 1, r(0, 0, 3000, 1080), true), FsKind::CoverOther);
        assert_eq!(map.iter().map(|m| m.state).collect::<Vec<_>>(), [FsState::Covering, FsState::None]);
    }

    #[test]
    fn exclusive_stays_on_its_monitor_when_own_window_is_foreground() {
        let (ra, rb) = (r(0, 0, 1920, 1080), r(1920, 0, 2560, 1440));
        let mons = [mon(1, ra), mon(2, rb)];
        let prev = build_map(&mons, true, fg(2, rb, true), FsKind::None); // exkluzivní na B
        // klik do Ledge: QUNS dál exkluzivní, v popředí None → jen B (dřív oba)
        let map = next_fs_map(&mons, true, None, &prev, &|_| None, FsKind::None);
        assert_eq!(map.iter().map(|m| m.state).collect::<Vec<_>>(), [FsState::None, FsState::Exclusive]);
        // bez historie = všechny (konzervativně)
        let map = next_fs_map(&mons, true, None, &[], &|_| None, FsKind::None);
        assert!(map.iter().all(|m| m.state == FsState::Exclusive));
    }
}
