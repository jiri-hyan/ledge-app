//! Vlákno `ui`: sériově obsluhuje šuplík podle `UiCmd` (animace nikdy na hlavním vlákně).

use super::notes_sync::{SyncMsg, send_sync};
use super::notes_watch::request_notes_reload;
use crate::appearance::{geom, motion};
use crate::drawer::{self, Opener, View};
use crate::refresh::{maybe_refresh_weather, refresh_git_async, refresh_logos};
use crate::sessionlive;
use crate::snapshot::snapshot;
use crate::state::{UiCmd, core};
use crate::stripview::refresh_strip;
use std::sync::mpsc;
use tauri::Emitter;

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
                    // pohyb jen z `appearance.motion` (styl none = okamžitě); šířka z configu oříznutá na
                    // monitor (nejvýš 95 % šířky)
                    (motion::drawer_anim_ms(&cfg.appearance.motion), geom::drawer_width(cfg.drawer.width, mon.rect.right - mon.rect.left, mon.dpi))
                };
                match cmd {
                    UiCmd::Show(view, opener) => {
                        if view == View::Full {
                            sessionlive::request(sessionlive::Wake::Open);
                            refresh_git_async();
                            refresh_logos(false);
                            request_notes_reload();
                            send_sync(SyncMsg::Opened);
                        }
                        // počasí (jen Full na Osobní) dřív než snapshot — karta hned ukáže „načítám“
                        maybe_refresh_weather(view == View::Full);
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
                    UiCmd::Hide => {
                        *c.pending_page.lock() = None;
                        if c.claude.attention() && c.drawer.view() == View::Full {
                            c.drawer.show(View::Peek, Opener::Hover, &mon, width, anim, c.strip_logical(&mon));
                        } else {
                            c.drawer.hide(&mon, anim, c.strip_logical(&mon));
                        }
                    }
                    UiCmd::Focus => c.drawer.take_focus(),
                    UiCmd::Release => c.drawer.release_focus(),
                    UiCmd::Size(view, w, h) => c.drawer.report_size(view, w, h, &mon, width),
                    UiCmd::Warm => c.drawer.warm(),
                }
                refresh_strip();
            }
        })
        .ok();
}
