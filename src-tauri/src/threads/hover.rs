//! Vlákno `hover`: poloha myši (100 ms daleko od okraje, 16 ms u něj), fullscreen (500 ms),
//! lištové okno v popředí (100 ms), tlak na hranu (`edge.rs`, `rawpush.rs`) → otevření/zavření šuplíku.

use crate::appearance::geom;
use crate::config::{self, Trigger};
use crate::drawer::{Opener, View};
use crate::state::{UiCmd, core, log, mark_dirty};
use crate::stripview::refresh_strip;
use crate::{edge, rawpush, screen};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub(crate) fn spawn_hover_thread() {
    std::thread::Builder::new()
        .name("hover".into())
        .spawn(|| {
            let mut zone_since: Option<Instant> = None;
            let mut outside_since: Option<Instant> = None;
            let mut last_fs_check = Instant::now() - Duration::from_secs(5);
            let mut mon = core().monitor();
            let mut last_mon = Instant::now();
            let mut last_warm = Instant::now() - Duration::from_secs(10);
            let mut approach_since: Option<Instant> = None;
            let mut fg = screen::ForegroundProbe::default();
            let mut last_fg_check = Instant::now() - Duration::from_secs(1);
            let mut push = edge::PushTracker::default();
            let mut latch = edge::BarLatch::default();
            let raw = rawpush::RawPush::start();
            let mut raw_armed = false;
            let mut others = screen::monitors().iter().map(|m| m.rect).collect::<Vec<_>>();
            let own_pid = std::process::id();
            let t0 = Instant::now();
            loop {
                let c = core();
                if last_mon.elapsed() > Duration::from_secs(3) {
                    mon = c.monitor_fresh();
                    others = screen::monitors().iter().map(|m| m.rect).collect::<Vec<_>>();
                    last_mon = Instant::now();
                }
                if last_fs_check.elapsed() > Duration::from_millis(500) {
                    last_fs_check = Instant::now();
                    let fs = screen::fullscreen_on(&mon);
                    if c.fullscreen.swap(fs, Ordering::Relaxed) != fs {
                        log(&format!("fullscreen: {fs}"));
                        if fs && c.drawer.view() != View::Hidden {
                            c.send_ui(UiCmd::Hide);
                        }
                        refresh_strip();
                        // konec fullscreenu → emit vlákno hned dožene peek připomínky (`check_reminders`)
                        mark_dirty();
                    }
                }
                if c.fullscreen.load(Ordering::Relaxed) {
                    zone_since = None;
                    push = edge::PushTracker::default();
                    if raw_armed {
                        raw.arm(false);
                        raw_armed = false;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
                let srect = *c.strip.rect.lock();
                // osa Ledge (střed / vlevo / vpravo podle `appearance.strip.position`) — zóna bez proužku
                let cx = geom::axis2_for(&c.cfg.read(), mon.rect.left, mon.rect.right, mon.dpi).div_euclid(2);
                let scale = mon.dpi as i32;
                // zóna = proužek (nebo střed hrany, když je proužek skrytý) + kousek do stran
                let (zl, zr, zb) = if srect.right > srect.left {
                    (srect.left - 24 * scale / 96, srect.right + 24 * scale / 96, srect.bottom)
                } else {
                    (cx - 160 * scale / 96, cx + 160 * scale / 96, mon.rect.top + 4 * scale / 96)
                };
                // lištové okno (prohlížeč) v popředí? Exe je v cache podle HWND, jinak pár levných volání.
                // Monitor nad proužkem (kurzor hranou projde) = tlačit nejde → bar režim ne.
                if last_fg_check.elapsed() > Duration::from_millis(100) {
                    last_fg_check = Instant::now();
                    let sample = {
                        let cfg = c.cfg.read();
                        let b = &cfg.drawer.bar_windows;
                        if b.trigger != config::BarTrigger::Push || edge::edge_passes_through(zl, zr, mon.rect.top, &others) {
                            Some(false)
                        } else {
                            match fg.read() {
                                // okno Ledge (šuplík, nastavení) nebo žádné → drž poslední stav
                                Some(f) if f.pid != own_pid => {
                                    Some(edge::is_bar_window(f.exe, f.maximized, f.monitor == mon.handle, &b.processes))
                                }
                                _ => None,
                            }
                        }
                    };
                    let bar = latch.update(t0.elapsed().as_millis() as u64, sample, edge::BAR_OFF_HOLD_MS);
                    if c.bar_window.swap(bar, Ordering::Relaxed) != bar {
                        push = edge::PushTracker::default();
                        refresh_strip();
                    }
                }
                let (trigger, edge_ms, dwell_ms, close_ms, keep_warm, hover_ms, push_ms, push_distance) = {
                    let d = &c.cfg.read().drawer;
                    let b = &d.bar_windows;
                    (d.trigger, d.edge_delay_ms, d.dwell_delay_ms, d.close_delay_ms, d.keep_warm_s, d.hover_delay_ms, b.push_ms, b.push_distance)
                };
                c.drawer.destroy_if_idle(Duration::from_secs(keep_warm));
                let Some(p) = screen::cursor() else {
                    push = edge::PushTracker::default();
                    if raw_armed {
                        raw.arm(false);
                        raw_armed = false;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                };
                let view = c.drawer.view();
                let in_x = p.x >= zl && p.x <= zr;
                let at_edge = in_x && p.y <= mon.rect.top + 1 && p.y >= mon.rect.top - 2;
                let on_strip = in_x && p.y >= mon.rect.top && p.y < zb;
                // tlak na hranu (lištová okna): raw myš jen pár px u hrany v zóně, jinak v klidu žádné WM_INPUT
                let bar_push = c.bar_window.load(Ordering::Relaxed);
                let want_raw = bar_push
                    && trigger != Trigger::Off
                    && view != View::Full
                    && in_x
                    && p.y >= mon.rect.top
                    && p.y <= mon.rect.top + 3 * scale / 96;
                if want_raw != raw_armed {
                    raw.arm(want_raw);
                    raw_armed = want_raw;
                }
                let r = raw.take();
                // tlačítko se čte jen na horním řádku zóny
                let button = in_x && p.y == mon.rect.top && screen::primary_button_down();
                let tick = edge::PushTick {
                    now_ms: t0.elapsed().as_millis() as u64,
                    x: p.x,
                    pressing: edge::pressing(in_x, p.y, mon.rect.top, button),
                    up: r.up,
                    clicked: r.clicked,
                    relative: r.live && !r.absolute,
                };
                let push_state = push.step(tick, &edge::PushCfg::new(push_ms, push_distance, edge::PUSH_SLACK_PX * scale / 96));
                // předehřátí: myš míří k horní hraně poblíž zóny
                let approaching = view == View::Hidden
                    && trigger != Trigger::Off
                    && p.y < mon.rect.top + 140 * scale / 96
                    && p.x >= zl - 360 * scale / 96
                    && p.x <= zr + 360 * scale / 96;
                if approaching {
                    let t = *approach_since.get_or_insert_with(Instant::now);
                    if t.elapsed() > Duration::from_millis(60) && !c.drawer.is_warm() && last_warm.elapsed() > Duration::from_secs(2) {
                        last_warm = Instant::now();
                        c.send_ui(UiCmd::Warm);
                    }
                } else {
                    approach_since = None;
                }
                match view {
                    View::Hidden | View::Peek => {
                        outside_since = None;
                        let peek_hover = view == View::Peek && {
                            let r = c.drawer.rect();
                            p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom
                        };
                        let delays = edge::Delays { edge: edge_ms, dwell: dwell_ms, hover: hover_ms };
                        let armed = edge::armed(trigger, bar_push, at_edge, on_strip, push_state, &delays);
                        let (armed, full) = edge::hover_open(armed, peek_hover, dwell_ms, c.claude.attention());
                        // při tlačení proužek (linka) „nažhaví“ už během nabíjení
                        let pushing = bar_push && trigger != Trigger::Off && push_state != edge::Push::Idle;
                        let hot = (armed.is_some() || pushing) && !peek_hover;
                        c.strip.set(|m| m.hot = hot);
                        match armed {
                            Some(ms) => {
                                let t = *zone_since.get_or_insert_with(Instant::now);
                                if t.elapsed() >= Duration::from_millis(ms) {
                                    zone_since = None;
                                    push.fire();
                                    c.send_ui(UiCmd::Show(if full { View::Full } else { View::Peek }, Opener::Hover));
                                }
                            }
                            None => zone_since = None,
                        }
                    }
                    View::Full => {
                        zone_since = None;
                        c.strip.set(|m| m.hot = false);
                        if c.drawer.opener() == Opener::Hover {
                            let r = c.drawer.rect();
                            let m = 28 * scale / 96;
                            let inside = p.x >= r.left - m && p.x <= r.right + m && p.y <= r.bottom + m;
                            if inside {
                                outside_since = None;
                            } else {
                                let t = *outside_since.get_or_insert_with(Instant::now);
                                if t.elapsed() >= Duration::from_millis(close_ms) {
                                    outside_since = None;
                                    log(&format!("drawer: zavírám — myš odjela ({},{}) mimo {:?}", p.x, p.y, (r.left, r.top, r.right, r.bottom)));
                                    c.send_ui(UiCmd::Hide);
                                }
                            }
                        }
                    }
                }
                let near = p.y < mon.rect.top + 80 * scale / 96 || view != View::Hidden;
                std::thread::sleep(Duration::from_millis(if near { 16 } else { 100 }));
            }
        })
        .ok();
}
