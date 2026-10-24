//! Vlákno `hover`: poloha myši (100 ms daleko od okraje, 16 ms u něj), fullscreen (500 ms),
//! lištové okno v popředí (100 ms), tlak na hranu (`edge.rs`, `rawpush.rs`) → otevření/zavření šuplíku.
//! Skrytý nebo do okna zamčený kurzor (hra) šuplík neotevře (`pointer::blocked_now`); změna monitoru / DPI
//! proužek hned přepočítá (`pointer::display_changed`).

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
            let mut shelf_arm = crate::shelf::Arm::default();
            let mut drawer_arm = crate::shelf::Arm::default();
            let mut others = screen::monitors().iter().map(|m| m.rect).collect::<Vec<_>>();
            let own_pid = std::process::id();
            let t0 = Instant::now();
            loop {
                let c = core();
                if last_mon.elapsed() > Duration::from_secs(3) {
                    let before = mon;
                    mon = c.monitor_fresh();
                    // jiné rozlišení / DPI / monitor (probuzení, odpojení): proužek a zóna hned na nové místo
                    if super::pointer::display_changed(&before, &mon) {
                        log(&format!("monitor proužku se změnil: {:?} @ {} dpi", (mon.rect.left, mon.rect.top, mon.rect.right, mon.rect.bottom), mon.dpi));
                        refresh_strip();
                        mark_dirty();
                    }
                    let now_mons = screen::monitors().iter().map(|m| m.rect).collect::<Vec<_>>();
                    if now_mons != others {
                        // připojený monitor → widgety na ploše, které stály mimo domov, jdou domů
                        crate::desk::win::on_monitors();
                    }
                    others = now_mons;
                    last_mon = Instant::now();
                }
                if last_fs_check.elapsed() > Duration::from_millis(500) {
                    last_fs_check = Instant::now();
                    let state = screen::fullscreen_state(&mon);
                    let fs = state != screen::FsState::None;
                    // pro lístky: co přesně leží přes monitor (prohlížeč ve F11 ≠ hra); okno v popředí se
                    // čte jen při pokrytí a exe je v cache podle HWND — žádný nový polling
                    let kind = match state {
                        screen::FsState::None => crate::pins::FsKind::None,
                        screen::FsState::Exclusive => crate::pins::FsKind::Exclusive,
                        screen::FsState::Covering => match fg.read() {
                            Some(f) => crate::pins::cover_kind(f.pid == own_pid, f.exe),
                            None => crate::pins::FsKind::CoverOther,
                        },
                    };
                    crate::desk::win::set_fs_kind(kind);
                    let kind_changed = crate::pins::win::set_fs_kind(kind);
                    if c.fullscreen.swap(fs, Ordering::Relaxed) != fs {
                        log(&format!("fullscreen: {fs}"));
                        if fs && c.drawer.view() != View::Hidden {
                            c.send_ui(UiCmd::Hide);
                        }
                        refresh_strip();
                        // lístky „Vždy navrchu“ hře ustoupí (a pak se vrátí)
                        crate::pins::win::sync_dead();
                        crate::desk::win::sync_yield();
                        // konec fullscreenu → emit vlákno hned dožene peek připomínky (`check_reminders`)
                        mark_dirty();
                    } else if kind_changed {
                        // fullscreen trvá, ale změnilo se okno v popředí (prohlížeč ↔ hra)
                        crate::pins::win::sync_dead();
                        crate::desk::win::sync_yield();
                    }
                }
                // mrtvý Ledge (fullscreen hra nebo herní režim): myš nic, kontrola fullscreenu jede dál
                if c.dead() {
                    zone_since = None;
                    shelf_arm = crate::shelf::Arm::default();
                    drawer_arm = crate::shelf::Arm::default();
                    crate::shelf::drop::set_armed(None);
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
                // zóna = proužek v jakémkoli stylu (linka a lišta přes celou šířku, roh, …) + kousek do
                // stran, nebo střed hrany, když je proužek skrytý (neviditelný styl v klidu)
                let (zl, zr, zb) = crate::looks::frame::zone(srect, cx, mon.rect, mon.dpi);
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
                let (trigger, edge_ms, dwell_ms, close_ms, keep_warm, hover_ms, push_ms, push_distance, hover_mode) = {
                    let d = &c.cfg.read().drawer;
                    let b = &d.bar_windows;
                    (d.trigger, d.edge_delay_ms, d.dwell_delay_ms, d.close_delay_ms, d.keep_warm_s, d.hover_delay_ms, b.push_ms, b.push_distance, d.hover)
                };
                c.drawer.destroy_if_idle(Duration::from_secs(keep_warm));
                let Some(p) = screen::cursor() else {
                    push = edge::PushTracker::default();
                    crate::shelf::drop::set_armed(None);
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
                // přihrádka: tah souborů na proužek (šuplík zavřený / peek) → neviditelný cíl přetažení
                // nad zónou (`shelf::drop`); tlačítko a kurzor se čtou jen s kurzorem v zóně
                // zóna se vybírá STAVEM šuplíku (`shelf::shortcut::target_for`): zavřený = přihrádka, otevřený naplno
                // = nový zástupce (kolo 20) — nepřekrývají se. Tah musí začít mimo Ledge (v popředí nesmí být
                // šuplík), jinak by vlastní tah karet v šuplíku mohl cíl přetažení nechtěně zapnout.
                let (shelf_on, shortcut_on) = {
                    let cfg = c.cfg.read();
                    (cfg.shelf.enabled, cfg.smart.drop_shortcut)
                };
                let target = crate::shelf::shortcut::target_for(view == View::Full, shelf_on && !view.covers_strip(), shortcut_on);
                let shelf_zone = (target == Some(crate::shelf::shortcut::Kind::Shelf)).then(|| crate::shelf::zone(zl, zr, zb, mon.rect.top, scale));
                let shortcut_zone = (target == Some(crate::shelf::shortcut::Kind::Shortcut)).then(|| c.drawer.rect());
                let shelf_rect = shelf_arm.step(shelf_zone, p.x, p.y, || screen::primary_button_down() && crate::shelf::drop::ole_drag_likely());
                let shortcut_rect = drawer_arm.step(shortcut_zone, p.x, p.y, || {
                    screen::primary_button_down() && crate::shelf::drop::ole_drag_likely() && fg.read().is_some_and(|f| f.pid != own_pid)
                });
                crate::shelf::drop::set_armed(
                    shelf_rect
                        .map(|r| (r, crate::shelf::shortcut::Kind::Shelf))
                        .or(shortcut_rect.map(|r| (r, crate::shelf::shortcut::Kind::Shortcut))),
                );
                let dropping = shelf_rect.is_some();
                // tlak na hranu (lištová okna): raw myš jen pár px u hrany v zóně, jinak v klidu žádné WM_INPUT
                let bar_push = c.bar_window.load(Ordering::Relaxed);
                let want_raw = bar_push
                    && trigger != Trigger::Off
                    && !view.covers_strip()
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
                        // tah na přihrádku šuplík neotevírá; skrytý / zamčený kurzor (hra v okně) taky ne —
                        // kurzor se čte jen u hrany v zóně
                        let blocked = armed.is_some() && (at_edge || on_strip) && super::pointer::blocked_now();
                        let armed = if dropping || blocked { None } else { armed };
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
                                    // KTERÝ pohled: peek (Claude čeká), celý, nebo náhled podle `drawer.hover`
                                    c.send_ui(UiCmd::Show(crate::hovermode::hover_target(hover_mode, full, peek_hover), Opener::Hover));
                                }
                            }
                            None => zone_since = None,
                        }
                    }
                    // celý šuplík i náhled po najetí: odjetí myši zavírá stejně
                    View::Full | View::Compact | View::Island => {
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
