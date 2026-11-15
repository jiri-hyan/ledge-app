//! Vlákno `ui`: sériově obsluhuje šuplík podle `UiCmd` (animace nikdy na hlavním vlákně).

use super::notes_sync::{SyncMsg, send_sync};
use super::notes_watch::request_notes_reload;
use crate::appearance::{geom, motion, size};
use crate::drawerfit;
use crate::drawer::{self, Opener, View};
use crate::focusreturn;
use crate::hovermode;
use crate::refresh::{maybe_refresh_weather, refresh_git_async, refresh_logos};
use crate::sessionlive;
use crate::snapshot::snapshot;
use crate::screen;
use crate::state::{UiCmd, core};
use crate::stripview::refresh_strip;
use std::sync::mpsc;
use tauri::Emitter;

/// Po příkazu se obnoví proužek, kromě tažení velikosti (živě každý snímek by ho kreslilo zbytečně; puštění
/// pošle `Resize(false)`).
fn refreshes_strip(cmd: &UiCmd) -> bool {
    !matches!(cmd, UiCmd::ResizeLive)
}

/// Zahodit příkaz v mrtvém Ledge? Herní režim zahazuje vše, co ukazuje šuplík (vypnutí jde přes
/// `gamemode::set(false)` dřív než Show). Detekovaná fullscreen hra zahazuje totéž kromě výslovné akce
/// člověka (`Opener::Hotkey`: tray, další spuštění Ledge, klik do widgetu; zkratky hlídá `gamemode::decide`).
fn dropped_in_dead(cmd: &UiCmd, fullscreen: bool, game_mode: bool) -> bool {
    match cmd {
        UiCmd::Show(_, Opener::Hotkey) => game_mode,
        UiCmd::Show(..) | UiCmd::Note | UiCmd::Pin(true) => fullscreen || game_mode,
        _ => false,
    }
}

/// Monitor, na který míří příkaz: Show otevírá na monitoru podle `tracker::choose_monitor` (hover = ten, který
/// nastavila smyčka `hover`; zkratka = pod kurzorem; ctl / upevnění = proužek; otevřený šuplík drží svůj),
/// ostatní příkazy pracují s monitorem šuplíku (`Core::drawer_monitor`). Jen mezi proužky (hlavní + tečky).
fn monitor_for(c: &crate::state::Core, cmd: &UiCmd) -> crate::screen::Monitor {
    let opener = match cmd {
        UiCmd::Show(_, o) => *o,
        UiCmd::Note => Opener::Hotkey,
        _ => return c.drawer_monitor(),
    };
    let main = c.monitor();
    let spec = c.cfg.read().strip.monitor.clone();
    let mut mons = vec![main];
    mons.extend(crate::stripset::secondary_monitors(&spec));
    let targets: Vec<_> = mons.iter().map(super::tracker::Target::of).collect();
    let cursor = screen::cursor().and_then(|p| super::tracker::monitor_at(p.x, p.y, &targets)).map(|i| mons[i].handle);
    let open = c.drawer.view() != View::Hidden;
    let h = super::tracker::choose_monitor(open, *c.opened_on.lock(), opener, cursor, main.handle, &mons.iter().map(|m| m.handle).collect::<Vec<_>>());
    mons.into_iter().find(|m| m.handle == h).unwrap_or(main)
}

// ---------- vlákna ----------

pub(crate) fn spawn_ui_thread(rx: mpsc::Receiver<UiCmd>) {
    std::thread::Builder::new()
        .name("ui".into())
        // L284: `FOREVER` — po stropu pádů by šuplík nešel otevřít a banner (jen v šuplíku) by nikdo neviděl
        .spawn(move || crate::guard::guarded("ui", crate::guard::FOREVER, || {
            let mut last_dpi = 0u32;
            while let Ok(cmd) = rx.recv() {
                let c = core();
                let mon = monitor_for(c, &cmd);
                let (anim, width) = {
                    let cfg = c.cfg.read();
                    c.drawer.set_anchor(cfg.appearance.strip.position);
                    c.drawer.set_expand_ms(cfg.drawer.hover_expand_ms);
                    c.drawer.set_hover_mode(cfg.drawer.hover);
                    c.drawer.set_dock_cards(hovermode::dock_cards(&cfg.drawer.dock_cards));
                    // pohyb jen z `appearance.motion` (styl none i systémové „omezit pohyb“ = okamžitě);
                    // šířka z předvolby stylu nebo `drawer.width`, oříznutá na monitor
                    (motion::anim_ms_now(&cfg.appearance.motion), geom::drawer_width_for(&cfg, mon.rect.right - mon.rect.left, mon.dpi))
                };
                let give_back = matches!(cmd, UiCmd::Close);
                let strip = refreshes_strip(&cmd);
                let shown = matches!(cmd, UiCmd::Show(..) | UiCmd::Note);
                if dropped_in_dead(&cmd, c.dead_on(mon.handle), c.game_mode.load(std::sync::atomic::Ordering::SeqCst)) {
                    if matches!(cmd, UiCmd::Show(..) | UiCmd::Note) && c.drawer.view() == View::Hidden {
                        *c.opened_on.lock() = None; // zahozený Show nesmí nechat starý monitor
                    }
                    continue;
                }
                if matches!(cmd, UiCmd::Show(..) | UiCmd::Note) {
                    *c.opened_on.lock() = Some(mon.handle);
                }
                match cmd {
                    UiCmd::Show(view, opener) => {
                        super::emitgate::note_input(crate::state::now_ms());
                        // střední panel (L147) kreslí karty ze snapshotu: Claude a poznámky obnovit i pro něj
                        if view.has_cards() {
                            sessionlive::request(sessionlive::Wake::Open);
                            request_notes_reload();
                        }
                        if view == View::Full {
                            c.rescan_index_on_open();
                            refresh_git_async();
                            refresh_logos(false);
                            send_sync(SyncMsg::Opened);
                            if c.cfg.read().scripts.in_actions {
                                crate::scripts::refresh_async();
                            }
                            // widgety na otevřené záložce: data starší než interval se obnoví hned
                            crate::widgets::live::wake_opening(&c.cfg.read());
                        }
                        // počasí (jen Full na Osobní) dřív než snapshot — karta hned ukáže „načítám“
                        maybe_refresh_weather(view.has_cards());
                        crate::incidents::maybe_refresh(view.has_cards());
                        crate::limitsfetch::maybe_refresh(view.has_cards());
                        crate::calendar::live::maybe_refresh(view.has_cards());
                        crate::accounts::calsync::maybe_refresh(view.has_cards());
                        crate::accounts::mail::maybe_refresh(view.has_cards());
                        // dok / ostrůvek: hudba živá jen při zobrazení a jen když smí (`media::allowed`, L177); hlasitost
                        // jen v doku a podle `volume::allowed` — každé zvlášť
                        let (music, volume) = {
                            let cfg = c.cfg.read();
                            (hovermode::music_wants_live(cfg.drawer.hover, view, crate::media::allowed(&cfg)), hovermode::volume_wants_live(cfg.drawer.hover, view, crate::volume::allowed(&cfg)))
                        };
                        if music {
                            crate::media::ensure();
                        } else if view.has_cards() || view == View::Island {
                            // vlákna uvolněná po dlouhém schování (L168) se vrátí (Full/Mid i ostrůvek, L177)
                            crate::media::resume();
                        }
                        if volume {
                            crate::volume::ensure();
                        } else if view.has_cards() {
                            crate::volume::resume();
                        }
                        // stav do šuplíku dřív, než vyjede
                        let _ = c.app.emit_to(drawer::LABEL, "state", snapshot());
                        c.drawer.show(view, opener, &mon, width, anim, c.strip_logical(&mon));
                    }
                    // zkratka poznámky v režimu `dock`: dok s rychlou poznámkou a fokusem (výslovná akce člověka);
                    // otevřený Full zůstává Full (stránka „note“ jako dřív)
                    UiCmd::Note if hovermode::note_in_dock(c.cfg.read().drawer.hover, c.drawer.view()) => {
                        {
                            let cfg = c.cfg.read();
                            if crate::media::allowed(&cfg) {
                                crate::media::ensure();
                            }
                            if crate::volume::allowed(&cfg) {
                                crate::volume::ensure();
                            }
                        }
                        let _ = c.app.emit_to(drawer::LABEL, "state", snapshot());
                        c.drawer.show(View::Island, Opener::Hotkey, &mon, width, anim, c.strip_logical(&mon));
                        // frontend (dock) otevře slot poznámky; událost přichází až po `show` = WebView žije a poslouchá
                        let _ = c.app.emit_to(drawer::LABEL, "dock-note", true);
                        c.drawer.take_focus();
                    }
                    UiCmd::Pin(on) => c.drawer.pin(on),
                    UiCmd::Note => {
                        *c.pending_page.lock() = Some("note");
                        request_notes_reload();
                        let _ = c.app.emit_to(drawer::LABEL, "state", snapshot());
                        let _ = c.app.emit_to(drawer::LABEL, "page", "note");
                        c.drawer.show(View::Full, Opener::Hotkey, &mon, width, anim, c.strip_logical(&mon));
                    }
                    UiCmd::Hide | UiCmd::Close => {
                        *c.pending_page.lock() = None;
                        // jen výslovné zavření vrací fokus; spuštění (okno má dostat fokus), blur, myš ne
                        focusreturn::on_close(give_back && c.drawer.view() != View::Hidden);
                        // s čekajícím Claudem se šuplík i náhled po najetí stáhnou do peeku (`hovermode::after_close`)
                        if hovermode::after_close(c.drawer.view(), c.claude.attention(), c.dead_surface()) == View::Peek {
                            c.drawer.show(View::Peek, Opener::Hover, &mon, width, anim, c.strip_logical(&mon));
                            focusreturn::give_back();
                        } else {
                            c.drawer.hide(&mon, anim, c.strip_logical(&mon));
                            *c.opened_on.lock() = None;
                        }
                    }
                    UiCmd::Focus => c.drawer.take_focus(),
                    UiCmd::Release => c.drawer.release_focus(),
                    UiCmd::Size(view, w, h) => c.drawer.report_size(view, w, h, &mon, width),
                    UiCmd::Resize(_) | UiCmd::ResizeLive => {
                        let animate = matches!(cmd, UiCmd::Resize(true));
                        // přepnutí záložky: pevnou výšku záložky zná Rust předem → šířka i výška jedním pohybem
                        let fixed = if animate {
                            let s = c.cfg.read().appearance.size.for_page(&size::page());
                            size::fixed_height(s, drawerfit::limits(mon.rect, mon.work, mon.dpi).max_h)
                        } else {
                            None
                        };
                        c.drawer.resize(&mon, width, if animate { anim } else { 0 }, fixed);
                    }
                    UiCmd::Warm => c.drawer.warm(),
                }
                if strip {
                    refresh_strip();
                }
                // šuplík přešel na monitor s jiným DPI: WM_DPICHANGED okna mohl rozhodit polohu / velikost → dorovnat
                if shown {
                    if last_dpi != 0 && last_dpi != mon.dpi && c.drawer.view() != View::Hidden {
                        c.send_ui(UiCmd::Resize(false));
                    }
                    last_dpi = mon.dpi;
                }
            }
        }))
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_ledge_drops_show_table() {
        let remote = UiCmd::Show(View::Full, Opener::Remote);
        let hover = UiCmd::Show(View::Peek, Opener::Hover);
        let human = UiCmd::Show(View::Full, Opener::Hotkey);
        // (příkaz, fullscreen, herní režim, zahodit)
        for (cmd, fs, gm, want) in [
            (&remote, false, false, false),
            (&remote, true, false, true),
            (&remote, false, true, true),
            (&hover, false, false, false),
            (&hover, true, false, true),
            (&hover, false, true, true),
            (&human, false, false, false),
            (&human, true, false, false),
            (&human, false, true, true),
            (&human, true, true, true),
            (&UiCmd::Note, false, false, false),
            (&UiCmd::Note, true, false, true),
            (&UiCmd::Note, false, true, true),
            (&UiCmd::Pin(true), false, false, false),
            (&UiCmd::Pin(true), true, false, true),
            (&UiCmd::Pin(true), false, true, true),
            (&UiCmd::Pin(false), true, true, false),
            (&UiCmd::Hide, true, true, false),
            (&UiCmd::Close, true, true, false),
        ] {
            assert_eq!(dropped_in_dead(cmd, fs, gm), want, "{fs} {gm}");
        }
    }

    #[test]
    fn strip_refresh_skipped_only_while_dragging() {
        for (cmd, want) in [(UiCmd::ResizeLive, false), (UiCmd::Resize(false), true), (UiCmd::Resize(true), true), (UiCmd::Warm, true), (UiCmd::Hide, true)] {
            assert_eq!(refreshes_strip(&cmd), want);
        }
    }
}
