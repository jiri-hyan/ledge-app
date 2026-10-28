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
use crate::state::{UiCmd, core};
use crate::stripview::refresh_strip;
use std::sync::mpsc;
use tauri::Emitter;

/// Po příkazu se obnoví proužek, kromě tažení velikosti (živě každý snímek by ho kreslilo zbytečně; puštění
/// pošle `Resize(false)`).
fn refreshes_strip(cmd: &UiCmd) -> bool {
    !matches!(cmd, UiCmd::ResizeLive)
}

// ---------- vlákna ----------

pub(crate) fn spawn_ui_thread(rx: mpsc::Receiver<UiCmd>) {
    std::thread::Builder::new()
        .name("ui".into())
        .spawn(move || {
            while let Ok(cmd) = rx.recv() {
                let c = core();
                let mon = c.monitor();
                let (anim, width) = {
                    let cfg = c.cfg.read();
                    c.drawer.set_anchor(cfg.appearance.strip.position);
                    c.drawer.set_expand_ms(cfg.drawer.hover_expand_ms);
                    // pohyb jen z `appearance.motion` (styl none i systémové „omezit pohyb“ = okamžitě);
                    // šířka z předvolby stylu nebo `drawer.width`, oříznutá na monitor
                    (motion::anim_ms_now(&cfg.appearance.motion), geom::drawer_width_for(&cfg, mon.rect.right - mon.rect.left, mon.dpi))
                };
                let give_back = matches!(cmd, UiCmd::Close);
                let strip = refreshes_strip(&cmd);
                // herní režim: šuplík se neukazuje (vypnutí jde přes `gamemode::set(false)` dřív než Show)
                if matches!(cmd, UiCmd::Show(..) | UiCmd::Note) && c.game_mode.load(std::sync::atomic::Ordering::SeqCst) {
                    continue;
                }
                match cmd {
                    UiCmd::Show(view, opener) => {
                        if view == View::Full {
                            sessionlive::request(sessionlive::Wake::Open);
                            refresh_git_async();
                            refresh_logos(false);
                            request_notes_reload();
                            send_sync(SyncMsg::Opened);
                            if c.cfg.read().scripts.in_actions {
                                crate::scripts::refresh_async();
                            }
                            // widgety na otevřené záložce: data starší než interval se obnoví hned
                            crate::widgets::live::wake_opening(&c.cfg.read());
                        }
                        // počasí (jen Full na Osobní) dřív než snapshot — karta hned ukáže „načítám“
                        maybe_refresh_weather(view == View::Full);
                        crate::incidents::maybe_refresh(view == View::Full);
                        crate::calendar::live::maybe_refresh(view == View::Full);
                        crate::accounts::calsync::maybe_refresh(view == View::Full);
                        crate::accounts::mail::maybe_refresh(view == View::Full);
                        // stav do šuplíku dřív, než vyjede
                        let _ = c.app.emit_to(drawer::LABEL, "state", snapshot());
                        c.drawer.show(view, opener, &mon, width, anim, c.strip_logical(&mon));
                    }
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
                        if hovermode::after_close(c.drawer.view(), c.claude.attention(), c.dead()) == View::Peek {
                            c.drawer.show(View::Peek, Opener::Hover, &mon, width, anim, c.strip_logical(&mon));
                            focusreturn::give_back();
                        } else {
                            c.drawer.hide(&mon, anim, c.strip_logical(&mon));
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
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_refresh_skipped_only_while_dragging() {
        for (cmd, want) in [(UiCmd::ResizeLive, false), (UiCmd::Resize(false), true), (UiCmd::Resize(true), true), (UiCmd::Warm, true), (UiCmd::Hide, true)] {
            assert_eq!(refreshes_strip(&cmd), want);
        }
    }
}
