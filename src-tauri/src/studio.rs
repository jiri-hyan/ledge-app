//! Ledge Studio (kolo 24): Nastavení v samostatném okně (`settings.html`, label `studio`) místo šuplíku.
//! Okno vzniká LÍNĚ až na výslovný klik (⚙, paleta, widget, tray, dlaždice) a zavření ho zničí — v klidu
//! nežije nic. Vlastní titulek (bez dekorací Windows), stín, v taskbaru / Alt+Tab, NE topmost; fokus bere
//! jen proto, že ho otevřel uživatel. Velikost a poloha se pamatují mimo config (`studio-window.json`
//! v lokální složce). Výchozí 1360 × 880 logických px oříznuto na 92 % pracovní plochy monitoru Ledge,
//! vycentrované, min 980 × 640. Geometrie je čistá funkce `place` (tabulkový test).
//! Okno smí volat jen příkazy z `capabilities/studio.json` (tabulka `STUDIO_WINDOW` v `ipcacl.rs`).
//! Když okno nejde vytvořit, `studio_open` vrátí chybu a šuplík vykreslí TÝŽ komponent jako stránku.

use crate::state::core;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

pub const LABEL: &str = "studio";
/// Událost do už otevřeného okna: skoč na cíl (id části, stará stránka, kotva `f-<pole>`).
pub const GOTO: &str = "studio-goto";
/// Událost do šuplíku: Studio chce průvodce (`wizard`) nebo úpravy v šuplíku (`edit`).
pub const ASK: &str = "studio-ask";
/// Událost do Studia: okno se chce zavřít (Alt+F4) — uložit rozepsané a zavřít přes `studio_win("close")`.
pub const CLOSE_ASK: &str = "studio-close-ask";
const CLOSE_GRACE: std::time::Duration = std::time::Duration::from_millis(1500);
/// Zvýšené při každé stavbě okna — pojistka zavření nesmí zničit NOVÉ Studio otevřené v mezičase.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Smí pojistka po Alt+F4 okno zničit? Jen když je to pořád ta generace, která o zavření žádala.
pub(crate) fn grace_may_destroy(at_request: u64, now: u64) -> bool {
    at_request == now
}
const FILE: &str = "studio-window.json";
/// Výchozí velikost v logických px.
const DEFAULT_W: f64 = 1360.0;
const DEFAULT_H: f64 = 880.0;
/// Nejmenší velikost v logických px.
pub const MIN_W: f64 = 980.0;
pub const MIN_H: f64 = 640.0;
/// Podíl pracovní plochy, který výchozí okno nepřesáhne.
const FRACTION: f64 = 0.92;

/// Obdélník ve fyzických px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Zapamatovaná geometrie (fyzické px) a jestli bylo okno maximalizované.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Saved {
    pub rect: Option<Rect>,
    #[serde(default)]
    pub max: bool,
}

fn contains(r: &Rect, x: i32, y: i32) -> bool {
    x >= r.x && y >= r.y && x < r.x.saturating_add(r.w) && y < r.y.saturating_add(r.h)
}

/// Kam okno dát. `work` = pracovní plocha monitoru Ledge, `scale` = DPI / 96, `screens` = všechny monitory
/// (zapamatované okno platí, jen když jeho střed leží na některém a není menší než minimum).
pub fn place(work: Rect, scale: f64, screens: &[Rect], saved: Option<Rect>) -> Rect {
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let (min_w, min_h) = ((MIN_W * scale).round() as i32, (MIN_H * scale).round() as i32);
    if let Some(s) = saved
        && s.w >= min_w
        && s.h >= min_h
        && screens.iter().any(|m| contains(m, s.x.saturating_add(s.w / 2), s.y.saturating_add(s.h / 2)))
    {
        // poškozený soubor: nikdy větší než pracovní plocha
        return Rect { w: s.w.min(work.w.max(min_w)), h: s.h.min(work.h.max(min_h)), ..s };
    }
    let fit = |want: f64, room: i32, min: i32| ((want * scale).round() as i32).min((room as f64 * FRACTION) as i32).max(min.min(room)).max(1);
    let w = fit(DEFAULT_W, work.w, min_w);
    let h = fit(DEFAULT_H, work.h, min_h);
    Rect { x: work.x + (work.w - w) / 2, y: work.y + (work.h - h) / 2, w, h }
}

/// Mezera mezi spodkem proužku a horní hranou Studia (fyzické px).
const STRIP_GAP: i32 = 8;

/// Studio nesmí ležet pod proužkem (ten je topmost nahoře na monitoru Ledge): okno na tomhle monitoru (střed
/// v `work`) s horní hranou nad `strip_bottom` se posune pod něj a zkrátí na zbytek plochy. Jiný monitor beze změny.
pub fn below_strip(r: Rect, work: Rect, strip_bottom: i32) -> Rect {
    let top = strip_bottom.saturating_add(STRIP_GAP).max(work.y);
    if r.y >= top || !contains(&work, r.x.saturating_add(r.w / 2), r.y.saturating_add(r.h / 2)) {
        return r;
    }
    let room = work.y.saturating_add(work.h).saturating_sub(top);
    Rect { y: top, h: r.h.min(room).max(1), ..r }
}

/// Cíl skoku z URL / události: jen písmena, číslice, `.`, `-`, `_` (kotvy `f-strip.look`), nejvýš 80 znaků.
pub fn clean_target(t: Option<&str>) -> Option<String> {
    let t = t?.trim().trim_start_matches('#');
    let ok = !t.is_empty() && t.len() <= 80 && t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    ok.then(|| t.to_string())
}

fn store() -> std::path::PathBuf {
    crate::actionrun::local_dir().join(FILE)
}

fn remember(w: &WebviewWindow) {
    let max = w.is_maximized().unwrap_or(false);
    let mut s: Saved = crate::fsx::load_json_or_default(&store());
    if !max && !w.is_minimized().unwrap_or(false)
        && let (Ok(p), Ok(z)) = (w.outer_position(), w.inner_size())
    {
        s.rect = Some(Rect { x: p.x, y: p.y, w: z.width as i32, h: z.height as i32 });
    }
    s.max = max;
    let _ = crate::fsx::save_json(&store(), &s);
}

/// Okno Studia, je-li otevřené (vlastník dialogů, příjemce událostí).
pub fn window() -> Option<WebviewWindow> {
    core().app.get_webview_window(LABEL)
}

/// Otevřít (nebo zaostřit) Studio a skočit na cíl. Chyba = okno nevzniklo (šuplík pak ukáže stránku).
/// `focus` = false (vzdálený `ctl`): okno se ukáže, ale nekrade fokus.
pub fn open(target: Option<String>, focus: bool) -> Result<(), String> {
    /// Od kontroly „okno není“ po `build()` — dvě současná otevření nesmí vyrobit dvě okna.
    static OPENING: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
    let target = clean_target(target.as_deref());
    let _guard = OPENING.lock();
    if let Some(w) = window() {
        return reveal(&w, target, focus);
    }
    let c = core();
    let mon = c.monitor();
    let scale = mon.dpi.max(96) as f64 / 96.0;
    let r = |x: windows::Win32::Foundation::RECT| Rect { x: x.left, y: x.top, w: x.right - x.left, h: x.bottom - x.top };
    let work = if mon.work.right > mon.work.left { r(mon.work) } else { r(mon.rect) };
    let screens: Vec<Rect> = c
        .app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| Rect { x: m.position().x, y: m.position().y, w: m.size().width as i32, h: m.size().height as i32 })
        .collect();
    let saved: Saved = crate::fsx::load_json_or_default(&store());
    let strip_bottom = mon.rect.top.saturating_add((c.strip_logical(&mon).1 * scale).ceil() as i32);
    let at = below_strip(place(work, scale, &screens, saved.rect), work, strip_bottom);
    let url = format!("settings.html{}", target.clone().map(|t| format!("#{t}")).unwrap_or_default());
    let win = WebviewWindowBuilder::new(&c.app, LABEL, WebviewUrl::App(url.into()))
        .title("Ledge Studio")
        .visible(false)
        .decorations(false)
        .shadow(true)
        .resizable(true)
        .maximizable(true)
        .skip_taskbar(false)
        .always_on_top(false)
        .min_inner_size(MIN_W, MIN_H)
        .inner_size(at.w as f64 / scale, at.h as f64 / scale)
        .position(at.x as f64 / scale, at.y as f64 / scale)
        // stejné argumenty + data dir jako šuplík → jeden proces prohlížeče
        .additional_browser_args(crate::drawer::BROWSER_ARGS)
        .disable_drag_drop_handler()
        .build();
    let win = match win {
        // někdo byl rychlejší (jiný proces vstupu): bereme existující okno, žádný fallback
        Err(tauri::Error::WindowLabelAlreadyExists(_)) if window().is_some() => {
            return window().map_or(Ok(()), |w| reveal(&w, target, focus));
        }
        r => r,
    }
    .map_err(|e| crate::errors::any(crate::i18n::tr("Ledge Studio se neotevřelo", "Ledge Studio didn't open"), &e))?;
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let _ = win.set_position(PhysicalPosition::new(at.x, at.y));
    let _ = win.set_size(PhysicalSize::new(at.w as u32, at.h as u32));
    if saved.max {
        let _ = win.maximize();
    }
    let w2 = win.clone();
    win.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            // Alt+F4: nejdřív ať Studio uloží rozepsané (`autosave.ts` `flush`) a zavře se samo (`studio_win("close")`);
            // pojistka — po CLOSE_GRACE se okno zavře i bez odpovědi, nikdy nezůstane viset
            api.prevent_close();
            remember(&w2);
            let _ = w2.emit_to(LABEL, CLOSE_ASK, ());
            let gen_at_request = GENERATION.load(Ordering::SeqCst);
            std::thread::spawn(move || {
                std::thread::sleep(CLOSE_GRACE);
                if let Some(w) = window().filter(|_| grace_may_destroy(gen_at_request, GENERATION.load(Ordering::SeqCst))) {
                    let _ = w.destroy();
                }
            });
        }
    });
    hide_drawer();
    let _ = win.show();
    if focus {
        let _ = win.set_focus();
    }
    Ok(())
}

/// Studio otevírá „místo Ledge“: šuplík je topmost, otevřený by Studio zakryl → zavřít (`Hide` = bez vrácení
/// fokusu, ten dostane Studio).
fn hide_drawer() {
    let c = core();
    if c.drawer.view() != crate::drawer::View::Hidden {
        c.send_ui(crate::state::UiCmd::Hide);
    }
}

fn reveal(w: &WebviewWindow, target: Option<String>, focus: bool) -> Result<(), String> {
    hide_drawer();
    let _ = w.unminimize();
    let _ = w.show();
    if focus {
        let _ = w.set_focus();
    }
    if let Some(t) = target {
        let _ = w.emit_to(LABEL, GOTO, t);
    }
    Ok(())
}

/// Smí vzdálený `ctl` otevřít Studio? Ne v mrtvém Ledge (fullscreen hra / herní režim) — stejně jako šuplík.
pub(crate) fn remote_may_open(dead: bool) -> bool {
    !dead
}

/// Vstup mimo šuplík (tray, widget na ploše, `ctl`): Studio ve vlastním vlákně (tray běží na hlavním
/// vlákně, stavba okna se z něj nesmí zablokovat); když okno nejde vytvořit, `fallback` (šuplík na stránce Nastavení).
pub fn open_or(target: Option<String>, focus: bool, fallback: impl FnOnce() + Send + 'static) {
    std::thread::spawn(move || {
        if let Err(e) = open(target, focus) {
            crate::state::log(&format!("studio: {e} — Nastavení v šuplíku"));
            fallback();
        }
    });
}

/// Otevřít Studio z šuplíku, palety nebo dlaždice (`target` = část, stará stránka, kotva `f-<pole>`).
#[tauri::command(async)]
pub(crate) fn studio_open(target: Option<String>) -> Result<(), String> {
    open(target, true)
}

/// Okenní tlačítka a tažení vlastního titulku (jen okno Studia): `drag` · `minimize` · `toggle_max` · `close`;
/// `wizard` / `edit` = otevřít šuplík s průvodcem / úpravami.
/// Vrací, jestli je okno maximalizované (titulek podle toho změní ikonu).
#[tauri::command]
pub(crate) fn studio_win(window: WebviewWindow, op: String) -> Result<bool, String> {
    if window.label() != LABEL {
        return Err("jen okno Studia".into());
    }
    let r = match op.as_str() {
        "drag" => window.start_dragging(),
        "minimize" => window.minimize(),
        "toggle_max" => {
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize()
            } else {
                window.maximize()
            }
        }
        "close" => {
            remember(&window);
            window.destroy()
        }
        // průvodce prvním spuštěním a úpravy přímo v šuplíku žijí v šuplíku: otevřít ho a říct mu, co chce
        "wizard" | "edit" => {
            let c = core();
            c.send_ui(crate::state::UiCmd::Show(crate::drawer::View::Full, crate::drawer::Opener::Hotkey));
            let _ = c.app.emit_to(crate::drawer::LABEL, ASK, op.as_str());
            Ok(())
        }
        _ => return Err(format!("neznámá operace {op}")),
    };
    r.map_err(|e| e.to_string())?;
    Ok(window.is_maximized().unwrap_or(false))
}

/// Poslat událost i do Studia, je-li otevřené (stav, vzhled, změna configu zvenku). Zavřené = nic.
pub fn forward<S: Serialize + Clone>(event: &str, payload: S) {
    if let Some(w) = window() {
        let _ = w.emit_to(LABEL, event, payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect { x: 0, y: 0, w: 1920, h: 1040 };

    #[test]
    fn place_table() {
        let screens = [Rect { x: 0, y: 0, w: 1920, h: 1080 }, Rect { x: 1920, y: 0, w: 2560, h: 1440 }];
        #[rustfmt::skip]
        let cases: Vec<(Rect, f64, Option<Rect>, Rect)> = vec![
            // FHD 100 %: 1360 × 880 se vejde (92 % = 1766 × 956), vycentrované
            (FHD, 1.0, None, Rect { x: 280, y: 80, w: 1360, h: 880 }),
            // FHD 150 %: 2040 × 1320 → 92 % plochy (1766 × 956), ale ne pod minimum 1470 × 960 → 960 výška
            (FHD, 1.5, None, Rect { x: 77, y: 40, w: 1766, h: 960 }),
            // malý notebook 1366 × 728 (100 %): 92 % = 1256 × 669 ≥ minimum
            (Rect { x: 0, y: 0, w: 1366, h: 728 }, 1.0, None, Rect { x: 55, y: 29, w: 1256, h: 669 }),
            // menší než minimum: celá plocha (nikdy přes ni)
            (Rect { x: 0, y: 0, w: 900, h: 600 }, 1.0, None, Rect { x: 0, y: 0, w: 900, h: 600 }),
            // druhý monitor vpravo
            (Rect { x: 1920, y: 0, w: 2560, h: 1400 }, 1.0, None, Rect { x: 2520, y: 260, w: 1360, h: 880 }),
            // zapamatované a platné (střed na druhém monitoru) → beze změny
            (FHD, 1.0, Some(Rect { x: 2000, y: 100, w: 1200, h: 800 }), Rect { x: 2000, y: 100, w: 1200, h: 800 }),
            // zapamatované mimo všechny monitory → výchozí
            (FHD, 1.0, Some(Rect { x: 9000, y: 100, w: 1200, h: 800 }), Rect { x: 280, y: 80, w: 1360, h: 880 }),
            // zapamatované menší než minimum → výchozí
            (FHD, 1.0, Some(Rect { x: 10, y: 10, w: 500, h: 400 }), Rect { x: 280, y: 80, w: 1360, h: 880 }),
            // rozbité měřítko = 1
            (FHD, f64::NAN, None, Rect { x: 280, y: 80, w: 1360, h: 880 }),
        ];
        for (work, scale, saved, want) in cases {
            assert_eq!(place(work, scale, &screens, saved), want, "{work:?} {scale} {saved:?}");
        }
    }

    #[test]
    fn place_survives_corrupt_saved() {
        let screens = [Rect { x: 0, y: 0, w: 1920, h: 1080 }];
        let huge = Rect { x: 0, y: 0, w: i32::MAX, h: i32::MAX };
        let r = place(FHD, 1.0, &screens, Some(huge));
        assert!(r.w <= FHD.w && r.h <= FHD.h, "{r:?}");
        let edge = Rect { x: i32::MAX - 10, y: i32::MAX - 10, w: i32::MAX, h: i32::MAX };
        assert_eq!(place(FHD, 1.0, &screens, Some(edge)), Rect { x: 280, y: 80, w: 1360, h: 880 });
        // platné, ale větší než plocha → oříznuto
        let big = Rect { x: 0, y: 0, w: 3000, h: 2000 };
        assert_eq!(place(FHD, 1.0, &screens, Some(big)), Rect { x: 0, y: 0, w: 1920, h: 1040 });
    }

    #[test]
    fn grace_only_same_generation() {
        assert!(grace_may_destroy(3, 3));
        assert!(!grace_may_destroy(3, 4));
    }

    #[test]
    fn remote_open_respects_game_mode() {
        assert!(remote_may_open(false));
        assert!(!remote_may_open(true));
    }

    #[test]
    fn clean_target_table() {
        let cases = [
            (Some("f-strip.look"), Some("f-strip.look")),
            (Some("#claude"), Some("claude")),
            (Some("  obsah "), Some("obsah")),
            (Some(""), None),
            (None, None),
            (Some("a b"), None),
            (Some("x\"><script>"), None),
            (Some("../../etc"), None),
        ];
        for (inp, want) in cases {
            assert_eq!(clean_target(inp).as_deref(), want, "{inp:?}");
        }
        assert!(clean_target(Some(&"a".repeat(81))).is_none());
    }

    #[test]
    fn saved_reads_old_and_partial() {
        let s: Saved = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(s, Saved::default());
        let s: Saved = serde_json::from_str(r#"{"rect":{"x":1,"y":2,"w":3,"h":4}}"#).unwrap();
        assert_eq!(s.rect, Some(Rect { x: 1, y: 2, w: 3, h: 4 }));
        assert!(!s.max);
    }

    #[test]
    fn below_strip_table() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1040 };
        #[rustfmt::skip]
        let cases: Vec<(Rect, i32, Rect)> = vec![
            // výchozí poloha (y 80) je pod proužkem 32 px → beze změny
            (Rect { x: 280, y: 80, w: 1360, h: 880 }, 32, Rect { x: 280, y: 80, w: 1360, h: 880 }),
            // uložená poloha nahoře → pod proužek + mezera, výška zkrácená na zbytek plochy
            (Rect { x: 100, y: 0, w: 1400, h: 1040 }, 32, Rect { x: 100, y: 40, w: 1400, h: 1000 }),
            // posunutá jen dolů, když se vejde
            (Rect { x: 100, y: 10, w: 1400, h: 700 }, 32, Rect { x: 100, y: 40, w: 1400, h: 700 }),
            // okno na jiném monitoru (střed mimo work) → beze změny
            (Rect { x: 2000, y: 0, w: 1400, h: 900 }, 32, Rect { x: 2000, y: 0, w: 1400, h: 900 }),
            // proužek vypnutý / panel nahoře: work.y už je pod proužkem
            (Rect { x: 100, y: 0, w: 1400, h: 900 }, -100, Rect { x: 100, y: 0, w: 1400, h: 900 }),
        ];
        for (r, strip, want) in cases {
            assert_eq!(below_strip(r, work, strip), want, "{r:?} strip {strip}");
        }
    }
}
