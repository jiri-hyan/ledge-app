//! Vlákno `hover`: poloha myši (100 ms daleko od okraje, 16 ms u něj), fullscreen (500 ms),
//! lištové okno v popředí (100 ms), tlak na hranu (`edge.rs`, `rawpush.rs`) → otevření/zavření šuplíku.
//! Skrytý nebo do okna zamčený kurzor (hra) šuplík neotevře (`pointer::blocked_now`). Změnu zobrazení (monitor,
//! DPI, pracovní plocha) hlásí okno proužku (`state::display_event`); vlákno ji jen zachytí podle počítadla
//! (`display_gen`) a přepočítá podle `pointer::display_plan`. Výčet co 60 s (`MONITOR_TTL`) je jen pojistka.

use crate::appearance::geom;
use crate::config::{self, Trigger};
use crate::drawer::{Opener, View};
use crate::state::{UiCmd, core, log, mark_dirty};
use crate::stripview::refresh_strip;
use super::tracker;
use crate::{edge, rawpush, screen};
use windows::Win32::Foundation::RECT;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub(crate) fn spawn_hover_thread() {
    std::thread::Builder::new()
        .name("hover".into())
        .spawn(|| crate::guard::guarded("hover", crate::guard::CAP, || {
            let mut fswatch = super::fswatch::FsWatch::default();
            let mut mon = core().monitor();
            let mut last_mon = Instant::now();
            let mut seen_gen = crate::state::display_gen();
            let mut last_warm = crate::guard::ago(Duration::from_secs(10));
            let mut fg = screen::ForegroundProbe::default();
            let mut last_fg_check = crate::guard::ago(Duration::from_secs(1));
            let mut latch = edge::BarLatch::default();
            let raw = rawpush::RawPush::start();
            let mut raw_armed = false;
            let mut drawer_arm = crate::shelf::Arm::default();
            let mut tracker = tracker::Tracker::default();
            let mut targets: Vec<tracker::Target> = Vec::with_capacity(4);
            let mut all_mons = screen::monitors();
            let mut others = all_mons.iter().map(|m| m.rect).collect::<Vec<_>>();
            let own_pid = std::process::id();
            let t0 = Instant::now();
            loop {
                let c = core();
                // událost zobrazení z okna proužku (≤ 100 ms odezva) nebo pojistka po `MONITOR_TTL`
                let gen_now = crate::state::display_gen();
                if gen_now != seen_gen || last_mon.elapsed() > crate::state::MONITOR_TTL {
                    if gen_now == seen_gen {
                        crate::state::display_event(); // pojistka: zneplatní cache monitorů
                    }
                    seen_gen = crate::state::display_gen();
                    let before = mon;
                    mon = c.monitor();
                    let now_all = screen::monitors();
                    let plan = super::pointer::display_plan(&before, &mon, &all_mons, &now_all);
                    // jiné rozlišení / DPI / pracovní plocha / monitor: proužek a zóna hned na nové místo
                    if plan.strip {
                        log(&format!("monitor proužku se změnil: {:?} @ {} dpi", (mon.rect.left, mon.rect.top, mon.rect.right, mon.rect.bottom), mon.dpi));
                        refresh_strip();
                        mark_dirty();
                    }
                    if plan.windows {
                        // změna sestavy (připojení, DPI, panel): widgety a lístky mimo domov jdou domů
                        crate::desk::win::on_monitors();
                        // tečky na dalších monitorech (`strip.monitor = all`): vznik / zánik / DPI (`stripset::apply`)
                        if !plan.strip {
                            refresh_strip();
                        }
                    }
                    others = now_all.iter().map(|m| m.rect).collect();
                    all_mons = now_all;
                    last_mon = Instant::now();
                }
                fswatch.tick(&mon, &all_mons, &mut fg, own_pid);
                let srect = *c.strip.rect.lock();
                // cíle hoveru: hlavní proužek (0) + tečky na vedlejších monitorech (`strip.monitor = all`)
                {
                    let cfg = c.cfg.read();
                    targets.clear();
                    targets.push(zone_target(&cfg, &mon, srect));
                    if cfg.strip.monitor == crate::screen::SPEC_ALL {
                        for m in crate::stripset::secondary_monitors(&cfg.strip.monitor) {
                            targets.push(zone_target(&cfg, &m, crate::stripset::rect_of(&m.device_name())));
                        }
                    }
                }
                let cur = screen::cursor();
                let view = c.drawer.view();
                // otevřený šuplík drží svůj monitor, jinak cíl pod kurzorem, jinak hlavní
                let open_idx = (view != View::Hidden).then(|| *c.opened_on.lock()).flatten().and_then(|h| targets.iter().position(|t| t.handle == h));
                let active = cur.map_or(open_idx.unwrap_or(0), |p| tracker::pick_active((p.x, p.y), &targets, open_idx));
                tracker.select(active, targets.len());
                let t = targets[active];
                // mrtvý Ledge (fullscreen hra nebo herní režim) na monitoru aktivního cíle: myš nic, kontrola
                // fullscreenu jede dál
                if c.dead_on(t.handle) {
                    tracker.reset_all();
                    drawer_arm = crate::shelf::Arm::default();
                    crate::shelf::drop::set_armed(None);
                    if raw_armed {
                        raw.arm(false);
                        raw_armed = false;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
                // hlavní zóna a lištové okno (bar režim) patří jen hlavnímu monitoru
                let (zl, zr) = (targets[0].zl, targets[0].zr);
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
                        tracker.reset_push();
                        refresh_strip();
                    }
                }
                let (trigger, edge_ms, dwell_ms, close_ms, keep_warm, hover_ms, push_ms, push_distance, hover_mode) = {
                    let d = &c.cfg.read().drawer;
                    let b = &d.bar_windows;
                    (d.trigger, d.edge_delay_ms, d.dwell_delay_ms, d.close_delay_ms, d.keep_warm_s, d.hover_delay_ms, b.push_ms, b.push_distance, d.hover)
                };
                c.drawer.destroy_if_idle(Duration::from_secs(keep_warm));
                // šuplík schovaný (dok ≥ 60 s, ostatní ≥ 120 s): vlákna hudby a hlasitosti pryč (L145/L168; další zobrazení je vrátí)
                if crate::media::release_due(hover_mode == crate::hovermode::HoverMode::Dock, c.drawer.hidden_secs()) {
                    crate::media::release();
                    crate::volume::release();
                }
                let Some(p) = cur else {
                    tracker.reset_push();
                    crate::shelf::drop::set_armed(None);
                    if raw_armed {
                        raw.arm(false);
                        raw_armed = false;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                };
                let scale = t.scale();
                let in_x = t.in_x(p.x);
                let pm = tracker.cur();
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
                let shelf_zone = (target == Some(crate::shelf::shortcut::Kind::Shelf)).then(|| crate::shelf::zone(t.zl, t.zr, t.zb, t.rect.top, scale));
                let drawer_rect = if view == View::Hidden { RECT::default() } else { c.drawer.rect() };
                let shortcut_zone = (target == Some(crate::shelf::shortcut::Kind::Shortcut)).then_some(drawer_rect);
                let shelf_rect = pm.shelf_arm.step(shelf_zone, p.x, p.y, || screen::primary_button_down() && crate::shelf::drop::ole_drag_likely());
                let shortcut_rect = drawer_arm.step(shortcut_zone, p.x, p.y, || {
                    screen::primary_button_down() && crate::shelf::drop::ole_drag_likely() && fg.read().is_some_and(|f| f.pid != own_pid)
                });
                crate::shelf::drop::set_armed(
                    shelf_rect
                        .map(|r| (r, crate::shelf::shortcut::Kind::Shelf))
                        .or(shortcut_rect.map(|r| (r, crate::shelf::shortcut::Kind::Shortcut))),
                );
                // tlak na hranu (lištová okna): raw myš jen pár px u hrany v zóně, jinak v klidu žádné WM_INPUT
                let bar_push = c.bar_window.load(Ordering::Relaxed) && active == 0;
                let want_raw = bar_push && trigger != Trigger::Off && !view.covers_strip() && in_x && p.y >= t.rect.top && p.y <= t.rect.top + 3 * scale / 96;
                if want_raw != raw_armed {
                    raw.arm(want_raw);
                    raw_armed = want_raw;
                }
                let r = raw.take();
                // tlačítko se čte jen na horním řádku zóny
                let button = in_x && p.y == t.rect.top && screen::primary_button_down();
                let tick = edge::PushTick {
                    now_ms: t0.elapsed().as_millis() as u64,
                    x: p.x,
                    pressing: edge::pressing(in_x, p.y, t.rect.top, button),
                    up: r.up,
                    clicked: r.clicked,
                    relative: r.live && !r.absolute,
                };
                let push_state = pm.push.step(tick, &edge::PushCfg::new(push_ms, push_distance, edge::PUSH_SLACK_PX * scale / 96));
                // předehřátí: myš míří k horní hraně poblíž zóny
                let approaching = view == View::Hidden
                    && trigger != Trigger::Off
                    && p.y < t.rect.top + 140 * scale / 96
                    && p.x >= t.zl - 360 * scale / 96
                    && p.x <= t.zr + 360 * scale / 96;
                if approaching {
                    let at = *pm.approach_since.get_or_insert_with(Instant::now);
                    if at.elapsed() > Duration::from_millis(60) && !c.drawer.is_warm() && last_warm.elapsed() > Duration::from_secs(2) {
                        last_warm = Instant::now();
                        c.send_ui(UiCmd::Warm);
                    }
                } else {
                    pm.approach_since = None;
                }
                let hidden_or_peek = matches!(view, View::Hidden | View::Peek);
                let ctx = tracker::Ctx {
                    view,
                    opener: c.drawer.opener(),
                    trigger,
                    delays: edge::Delays { edge: edge_ms, dwell: dwell_ms, hover: hover_ms },
                    close_ms,
                    hover_mode,
                    push: push_state,
                    bar_push,
                    attention: hidden_or_peek && c.claude.attention(),
                    dropping: shelf_rect.is_some(),
                    // kurzor na oznámení s tlačítkem (Notch): šuplík se neotevře, oznámení počká
                    notch: hidden_or_peek && crate::notify::owns_pointer(p.x, p.y),
                    drawer: drawer_rect,
                };
                // skrytý / zamčený kurzor (hra v okně) šuplík neotevře — kurzor se čte jen u hrany v zóně
                let out = pm.step(&t, (p.x, p.y), Instant::now(), &ctx, super::pointer::blocked_now);
                c.strip.set(|m| m.hot = out.hot && active == 0);
                match out.action {
                    tracker::Action::Open(v) => {
                        // šuplík se otevře na monitoru, kde ho spustil kurzor (`ui` to čte z `opened_on`)
                        *c.opened_on.lock() = Some(t.handle);
                        c.send_ui(UiCmd::Show(v, Opener::Hover));
                    }
                    tracker::Action::Close => {
                        let r = drawer_rect;
                        log(&format!("drawer: zavírám — myš odjela ({},{}) mimo {:?}", p.x, p.y, (r.left, r.top, r.right, r.bottom)));
                        c.send_ui(UiCmd::Hide);
                    }
                    tracker::Action::None => {}
                }
                let near = near_zone(p.x, p.y, (t.zl, t.zr), t.rect.top, scale, view != View::Hidden);
                std::thread::sleep(Duration::from_millis(if near { 16 } else { 100 }));
            }
        }))
        .ok();
}

/// Cíl hoveru pro monitor `m`: zóna podle skutečného obdélníku proužku / tečky `srect` (prázdný = skrytý →
/// pruh na ose Ledge monitoru, `looks::frame::zone`).
fn zone_target(cfg: &config::Config, m: &crate::screen::Monitor, srect: RECT) -> tracker::Target {
    let cx = geom::axis2_for(cfg, m.rect.left, m.rect.right, m.dpi).div_euclid(2);
    let (zl, zr, zb) = crate::looks::frame::zone(srect, cx, m.rect, m.dpi);
    tracker::Target { handle: m.handle, rect: m.rect, dpi: m.dpi, zl, zr, zb }
}

/// Rezerva (logické px) kolem zóny proužku, ve které se smyčka drží na 16 ms (kurzor do zóny dorazí
/// z boku i shora; výška i šířka stejná).
const NEAR_PX: i32 = 80;

/// Je kurzor „blízko“ proužku (rychlý tik 16 ms)? Pásmo = zóna proužku `(zl, zr)` + rezerva do stran
/// a `NEAR_PX` pod horní hranou; otevřený šuplík (`open`) drží rychlý tik vždy. Jinde 100 ms.
fn near_zone(x: i32, y: i32, (zl, zr): (i32, i32), top: i32, scale: i32, open: bool) -> bool {
    let m = NEAR_PX * scale / 96;
    open || (y < top + m && x >= zl - m && x <= zr + m)
}

#[cfg(test)]
mod near_tests {
    use super::near_zone;

    #[test]
    fn near_band_needs_x_and_y() {
        let z = (900, 1020); // zóna proužku, horní hrana 0, 96 dpi → rezerva 80
        for (x, y, open, want, what) in [
            (960, 10, false, true, "nad proužkem"),
            (830, 10, false, true, "v rezervě vlevo"),
            (1100, 79, false, true, "v rezervě vpravo"),
            (100, 10, false, false, "záhlaví okna daleko vlevo"),
            (1800, 0, false, false, "daleko vpravo"),
            (960, 80, false, false, "pod pásmem"),
            (100, 500, true, true, "otevřený šuplík = vždy rychle"),
        ] {
            assert_eq!(near_zone(x, y, z, 0, 96, open), want, "{what}");
        }
        assert!(near_zone(760, 100, z, 0, 192, false), "192 dpi = rezerva 160");
    }
}
