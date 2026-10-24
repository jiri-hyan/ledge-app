//! Okno s webem (rychlá akce `kind: "web"`): YouTube Music, ChatGPT, Gmail… v samostatném malém
//! okně s rámem. Zavření okno **zničí** (žádné skrývání, žádný časovač) — paměť se vrátí; přihlášení
//! přežije, protože okno sdílí s šuplíkem data dir WebView2 (cookies) i `drawer::BROWSER_ARGS`, takže
//! běží ve stejném procesu prohlížeče (nový proces = desítky MB navíc).
//!
//! Bezpečnost (stránka je cizí kód):
//! - okno `web-<id>` není v žádné capability (`capabilities/default.json` jmenuje jen `drawer`)
//!   a Tauri ≥ 2.11 odmítne z ne-lokálního původu i vlastní příkazy aplikace bez `remote`
//!   capability (`Webview::on_message`) → stránka nedostane žádné IPC (test `capabilities_*`);
//! - hlavní rámec smí jen na http/https mimo hosty `*.localhost` (tak Tauri na Windows podává
//!   vlastní protokoly: `tauri.localhost` = UI Ledge s plným IPC, `ipc.localhost`); rámce (iframe)
//!   na `*.localhost` se ruší přes WebView2 `FrameNavigationStarting`;
//! - `window.open` / `target=_blank` se otevře ve výchozím prohlížeči, nikdy v Ledge.
//!
//! Pozice a velikost per akce v `%LOCALAPPDATA%\Ledge\web-panels.json` (zápis jen při zavření).

use crate::actionrun::{self, Outcome};
use crate::actions::{Action, ActionKind};
use crate::i18n::tr;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

/// Okna s webem mají label `web-<id akce>` (id je a–z, 0–9, `-`, `_`).
pub const LABEL_PREFIX: &str = "web-";
/// Výchozí velikost (logické px) — „malé okno“ na hudbu / chat.
pub const DEFAULT_W: u32 = 480;
pub const DEFAULT_H: u32 = 720;
pub const MIN_W: u32 = 320;
pub const MAX_W: u32 = 3840;
pub const MIN_H: u32 = 240;
pub const MAX_H: u32 = 2160;
const FILE: &str = "web-panels.json";
/// Okraj od kraje pracovní plochy a mezera pod proužkem (logické px).
const MARGIN: f64 = 12.0;
/// Dvojklik na dlaždici = dvě volání `run_action` naráz → druhé počká a okno jen přenese dopředu.
static OPENING: Mutex<()> = Mutex::new(());

pub fn label(id: &str) -> String {
    format!("{LABEL_PREFIX}{id}")
}

/// Host, pod kterým Tauri na Windows podává vlastní protokoly (`http://tauri.localhost` = UI Ledge,
/// `http://ipc.localhost` = IPC). Holé `localhost` (dev server) je obyčejný web.
fn is_app_host(host: &str) -> bool {
    host.trim_end_matches('.').to_ascii_lowercase().ends_with(".localhost")
}

/// Adresa pro okno s webem: http/https s hostem, bez jména a hesla v adrese, ne `*.localhost`.
pub fn check_url(raw: &str) -> Result<Url, String> {
    let t = raw.trim();
    if t.is_empty() || t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(tr("adresa nesmí být prázdná ani obsahovat mezery.", "the address must not be empty or contain spaces.").into());
    }
    let bad = || tr("okno s webem otevře jen http:// nebo https:// adresu.", "a web window only opens an http:// or https:// address.").to_string();
    let u = Url::parse(t).map_err(|_| bad())?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(bad());
    }
    let Some(host) = u.host_str().filter(|h| !h.is_empty()) else { return Err(bad()) };
    if !u.username().is_empty() || u.password().is_some() {
        return Err(tr("adresa nesmí obsahovat jméno ani heslo.", "the address must not contain a user name or password.").into());
    }
    if is_app_host(host) {
        return Err(tr("adresy *.localhost patří Ledge, okno s webem je neotevře.", "*.localhost addresses belong to Ledge; a web window won't open them.").into());
    }
    Ok(u)
}

/// Smí hlavní rámec okna na tuhle adresu? (odkazy, přesměrování, `location = …`)
pub fn nav_allowed(u: &Url) -> bool {
    matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty() && !is_app_host(h)) && u.username().is_empty() && u.password().is_none()
}

/// Smí vnořený rámec (iframe) na tuhle adresu? Stránky běžně potřebují `about:blank`, `data:`,
/// `blob:`; zakázané jsou jen hosty Ledge (`*.localhost`) a neparsovatelné adresy.
pub fn frame_allowed(raw: &str) -> bool {
    match Url::parse(raw) {
        Ok(u) => !(matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(is_app_host)),
        Err(_) => false,
    }
}

/// Kontrola `width` / `height` z configu (logické px).
pub fn check_size(w: Option<u32>, h: Option<u32>) -> Result<(), String> {
    if w.is_some_and(|w| !(MIN_W..=MAX_W).contains(&w)) {
        return Err(trf!("šířka okna musí být {MIN_W}–{MAX_W} px.", "the window width must be {MIN_W}–{MAX_W} px."));
    }
    if h.is_some_and(|h| !(MIN_H..=MAX_H).contains(&h)) {
        return Err(trf!("výška okna musí být {MIN_H}–{MAX_H} px.", "the window height must be {MIN_H}–{MAX_H} px."));
    }
    Ok(())
}

/// Velikost z configu (logické px), výchozí 480 × 720.
pub fn size(a: &Action) -> (u32, u32) {
    (a.width.unwrap_or(DEFAULT_W).clamp(MIN_W, MAX_W), a.height.unwrap_or(DEFAULT_H).clamp(MIN_H, MAX_H))
}

// ---------- umístění ----------

/// Obdélník ve fyzických px (pracovní plocha monitoru).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
}

impl From<windows::Win32::Foundation::RECT> for Area {
    fn from(r: windows::Win32::Foundation::RECT) -> Self {
        Self { l: r.left, t: r.top, r: r.right, b: r.bottom }
    }
}

/// Uložená poloha okna: vnější pozice + vnitřní velikost (fyzické px) a velikost z configu, pro
/// kterou platí (změna `width`/`height` v Nastavení uloženou polohu zahodí).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Geom {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub cfg: (u32, u32),
}

/// Je uložené okno pořád k chycení? Pruh titulku (horních 32 px) musí aspoň 64 × 16 px ležet na
/// pracovní ploše některého monitoru (odpojený monitor, změněné rozlišení → výchozí umístění).
pub fn visible(g: &Geom, works: &[Area]) -> bool {
    let (l, t, r, b) = (g.x, g.y, g.x.saturating_add(g.w as i32), g.y.saturating_add(32));
    works.iter().any(|a| r.min(a.r) - l.max(a.l) >= 64 && b.min(a.b) - t.max(a.t) >= 16)
}

/// Výchozí umístění na monitoru proužku: velikost × DPI (vejde se do plochy s okrajem), vodorovně
/// u kurzoru (je-li na tom monitoru), jinak uprostřed; nahoře pod proužkem (`top_gap` fyzicky).
pub fn place(logical: (u32, u32), scale: f64, work: Area, cursor_x: Option<i32>, top_gap: i32) -> Geom {
    let m = (MARGIN * scale).round() as i32;
    let max_w = (work.r - work.l - 2 * m).max(1);
    let max_h = (work.b - work.t - top_gap - m).max(1);
    let w = ((logical.0 as f64 * scale).round() as i32).clamp(1, max_w);
    let h = ((logical.1 as f64 * scale).round() as i32).clamp(1, max_h);
    let cx = cursor_x.filter(|x| (work.l..work.r).contains(x)).unwrap_or((work.l + work.r) / 2);
    let x = (cx - w / 2).clamp(work.l + m, (work.r - m - w).max(work.l + m));
    let y = work.t + top_gap;
    Geom { x, y, w: w as u32, h: h as u32, cfg: logical }
}

fn store_path() -> PathBuf {
    actionrun::local_dir().join(FILE)
}

/// Uložené polohy (chybějící / poškozený soubor = žádné).
pub fn load(path: &Path) -> BTreeMap<String, Geom> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// Zapíše polohu akce `id`; nechá jen akce z `keep` (smazané akce se uklidí).
pub fn save(path: &Path, id: &str, g: Geom, keep: &[String]) -> std::io::Result<()> {
    let mut all = load(path);
    all.insert(id.to_string(), g);
    all.retain(|k, _| keep.iter().any(|x| x == k));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&all).unwrap_or_default())
}

// ---------- okno ----------

/// Otevře okno akce, nebo existující přenese dopředu. Volá `run_action` (výslovný klik → okno fokus
/// dostat smí). Druhý klik okno nezavírá: dlaždice znamená „ukaž mi to“ a zavření by bez varování
/// zahodilo rozehranou hudbu nebo rozepsaný mail; zavírá se křížkem okna.
pub fn open(app: &AppHandle, a: &Action, url: &str) -> Result<Outcome, String> {
    let c = crate::state::core();
    if c.dead() {
        return Err(trf!("{}: hra / herní režim — okno se teď neotevře.", "{}: game / game mode — the window won't open now.", a.label));
    }
    let url = check_url(url).map_err(|e| crate::errors::prefixed(&a.label, &e))?;
    let _one = OPENING.lock();
    let lbl = label(&a.id);
    if let Some(w) = app.get_webview_window(&lbl) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(Outcome { message: trf!("{} — okno je vpředu", "{} — window brought to front", a.label), output: None, hide: true });
    }
    let want = size(a);
    let mon = c.monitor();
    let scale = mon.dpi as f64 / 96.0;
    let works: Vec<Area> = crate::screen::monitors().iter().map(|m| Area::from(m.work)).collect();
    let saved = load(&store_path()).get(&a.id).copied().filter(|g| g.cfg == want && visible(g, &works));
    let geom = saved.unwrap_or_else(|| {
        let r = Area::from(mon.rect);
        let cursor = crate::screen::cursor().filter(|p| (r.l..r.r).contains(&p.x) && (r.t..r.b).contains(&p.y)).map(|p| p.x);
        let strip_h = c.strip_logical(&mon).1;
        let work = Area::from(mon.work);
        // proužek je u horní hrany monitoru; pod ním (a pod hlavním panelem nahoře) mezera
        let gap = ((r.t + ((strip_h + MARGIN) * scale).round() as i32) - work.t).max((MARGIN * scale).round() as i32);
        place(want, scale, work, cursor, gap)
    });
    let browser = app.clone();
    let w = WebviewWindowBuilder::new(app, &lbl, WebviewUrl::External(url))
        .title(a.label.trim())
        .visible(false)
        .focused(true)
        .resizable(true)
        .min_inner_size(MIN_W as f64, MIN_H as f64)
        // stejné argumenty + výchozí data dir jako šuplík → jeden proces prohlížeče, sdílené cookies
        .additional_browser_args(crate::drawer::BROWSER_ARGS)
        // soubory se přetahují do stránky (příloha v Gmailu), ne do Tauri
        .disable_drag_drop_handler()
        .on_navigation(nav_allowed)
        .on_new_window(move |u, _| {
            if nav_allowed(&u) {
                let _ = browser.opener().open_url(u.as_str(), None::<&str>);
            }
            NewWindowResponse::Deny
        })
        .build()
        .map_err(|e| crate::errors::any(&a.label, &e))?;
    guard_frames(&w);
    let _ = w.set_position(PhysicalPosition::new(geom.x, geom.y));
    let _ = w.set_size(PhysicalSize::new(geom.w, geom.h));
    let _ = w.show();
    let _ = w.set_focus();
    let (id, win) = (a.id.clone(), w.clone());
    w.on_window_event(move |e| {
        if let tauri::WindowEvent::CloseRequested { .. } = e {
            remember(&id, &win, want);
        }
    });
    Ok(Outcome { message: trf!("{} — otevřeno v okně", "{} — opened in a window", a.label), output: None, hide: true })
}

/// Při zavření zapíše polohu a velikost (minimalizované / maximalizované ne).
fn remember(id: &str, w: &WebviewWindow, cfg: (u32, u32)) {
    if w.is_minimized().unwrap_or(true) || w.is_maximized().unwrap_or(true) {
        return;
    }
    let (Ok(pos), Ok(sz)) = (w.outer_position(), w.inner_size()) else { return };
    let keep: Vec<String> = crate::state::core().cfg.read().actions.iter().filter(|a| a.kind == ActionKind::Web).map(|a| a.id.clone()).collect();
    if let Err(e) = save(&store_path(), id, Geom { x: pos.x, y: pos.y, w: sz.width, h: sz.height, cfg }, &keep) {
        crate::log(&format!("okno s webem {id}: poloha se neuložila ({e})"));
    }
}

/// Vnořené rámce na hosty Ledge (`*.localhost`) zrušit — `on_navigation` hlídá jen hlavní rámec.
fn guard_frames(w: &WebviewWindow) {
    let _ = w.with_webview(|wv| unsafe {
        let Ok(core) = wv.controller().CoreWebView2() else { return };
        let handler = webview2_com::NavigationStartingEventHandler::create(Box::new(|_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut uri = windows::core::PWSTR::null();
            args.Uri(&mut uri)?;
            if !frame_allowed(&webview2_com::take_pwstr(uri)) {
                args.SetCancel(true)?;
            }
            Ok(())
        }));
        let mut token = 0i64;
        if core.add_FrameNavigationStarting(&handler, &mut token).is_err() {
            crate::log("okno s webem: hlídání rámců se nezapojilo");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (adresa, projde?) — jen http(s) s hostem, žádné hosty Ledge, žádné heslo v adrese.
    #[test]
    fn check_url_table() {
        let cases: &[(&str, bool)] = &[
            ("https://music.youtube.com", true),
            ("https://chatgpt.com/", true),
            (" https://mail.google.com/mail/u/0/#inbox ", true),
            ("http://localhost:5173", true),
            ("http://127.0.0.1:8080/x", true),
            ("HTTPS://Example.COM", true),
            ("", false),
            ("mapy.cz", false),
            ("file:///C:/Windows/win.ini", false),
            ("javascript:alert(1)", false),
            ("data:text/html,<b>x</b>", false),
            ("ledge://open", false),
            ("ftp://x.cz", false),
            ("about:blank", false),
            ("https://", false),
            ("https://a b.cz", false),
            ("https://x.cz/\nfoo", false),
            ("http://tauri.localhost/index.html", false),
            ("https://tauri.localhost", false),
            ("http://ipc.localhost/run_action", false),
            ("http://IPC.LOCALHOST./x", false),
            ("https://user:pw@evil.cz", false),
            ("https://google.com@evil.cz", false),
        ];
        for (u, ok) in cases {
            assert_eq!(check_url(u).is_ok(), *ok, "{u:?} → {:?}", check_url(u));
        }
    }

    /// Navigace hlavního rámce: (adresa, povolit?).
    #[test]
    fn nav_table() {
        let cases: &[(&str, bool)] = &[
            ("https://accounts.google.com/signin", true),
            ("http://localhost:3000/", true),
            ("http://tauri.localhost/", false),
            ("http://asset.localhost/C:/x", false),
            ("file:///C:/x", false),
            ("javascript:void(0)", false),
            ("data:text/html,x", false),
            ("blob:https://x.cz/1", false),
            ("about:blank", false),
            ("mailto:a@b.cz", false),
            ("ms-settings:display", false),
            ("https://u:p@x.cz", false),
        ];
        for (u, ok) in cases {
            assert_eq!(nav_allowed(&Url::parse(u).unwrap()), *ok, "{u}");
        }
    }

    /// Vnořené rámce: běžné stránkové věci projdou, hosty Ledge ne.
    #[test]
    fn frame_table() {
        let cases: &[(&str, bool)] = &[
            ("about:blank", true),
            ("about:srcdoc", true),
            ("https://www.youtube.com/embed/x", true),
            ("data:text/html,x", true),
            ("blob:https://x.cz/1", true),
            ("http://localhost:5173", true),
            ("http://tauri.localhost/index.html", false),
            ("https://ipc.localhost/x", false),
            ("http://foo.localhost", false),
            ("", false),
        ];
        for (u, ok) in cases {
            assert_eq!(frame_allowed(u), *ok, "{u}");
        }
    }

    /// (šířka, výška, projde?) a výchozí velikost.
    #[test]
    fn size_table() {
        let cases = [
            (None, None, true),
            (Some(320), Some(240), true),
            (Some(3840), Some(2160), true),
            (Some(319), None, false),
            (Some(3841), None, false),
            (None, Some(239), false),
            (None, Some(2161), false),
            (Some(0), Some(0), false),
        ];
        for (w, h, ok) in cases {
            assert_eq!(check_size(w, h).is_ok(), ok, "{w:?}×{h:?}");
        }
        let a = Action { kind: ActionKind::Web, ..Default::default() };
        assert_eq!(size(&a), (DEFAULT_W, DEFAULT_H));
        assert_eq!(size(&Action { width: Some(800), height: Some(600), ..a }), (800, 600));
    }

    #[test]
    fn place_on_monitor() {
        let work = Area { l: 0, t: 0, r: 1920, b: 1040 };
        // u kurzoru, pod proužkem
        assert_eq!(place((480, 720), 1.0, work, Some(1000), 40), Geom { x: 760, y: 40, w: 480, h: 720, cfg: (480, 720) });
        // kurzor u kraje → okno se přimkne k okraji plochy
        assert_eq!(place((480, 720), 1.0, work, Some(1900), 40).x, 1920 - 12 - 480);
        assert_eq!(place((480, 720), 1.0, work, Some(5), 40).x, 12);
        // kurzor mimo monitor → uprostřed
        assert_eq!(place((480, 720), 1.0, work, Some(-500), 40).x, 960 - 240);
        // DPI 150 % a malá plocha → zmenší se na plochu
        let g = place((1200, 900), 1.5, Area { l: 1920, t: 0, r: 3200, b: 1000 }, None, 60);
        assert_eq!((g.w, g.h, g.y), (1280 - 36, 1000 - 60 - 18, 60));
        assert!(g.x >= 1920 + 18 && g.x as u32 + g.w <= 3200);
    }

    #[test]
    fn saved_geometry_must_stay_reachable() {
        let works = [Area { l: 0, t: 0, r: 1920, b: 1040 }, Area { l: 1920, t: 0, r: 3840, b: 1080 }];
        let g = |x, y| Geom { x, y, w: 480, h: 720, cfg: (480, 720) };
        let cases = [
            (g(100, 100), true),
            (g(3000, 50), true),
            (g(-450, 10), false),  // jen 30 px titulku na ploše
            (g(-400, 10), true),   // 80 px stačí
            (g(100, 1030), false), // titulek pod plochou
            (g(5000, 100), false), // odpojený monitor
            (g(100, -20), false),  // titulek nad plochou (jen 12 px)
        ];
        for (geom, ok) in cases {
            assert_eq!(visible(&geom, &works), ok, "{geom:?}");
        }
    }

    #[test]
    fn store_roundtrip_and_prune() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub").join(FILE);
        assert!(load(&p).is_empty());
        let a = Geom { x: 1, y: 2, w: 300, h: 400, cfg: (480, 720) };
        save(&p, "yt", a, &["yt".into(), "gpt".into()]).unwrap();
        save(&p, "gpt", Geom { x: 5, ..a }, &["yt".into(), "gpt".into()]).unwrap();
        assert_eq!(load(&p).len(), 2);
        // akce „yt“ smazaná → při dalším zápisu zmizí
        save(&p, "gpt", a, &["gpt".into()]).unwrap();
        let all = load(&p);
        assert_eq!(all.keys().collect::<Vec<_>>(), ["gpt"]);
        assert_eq!(all["gpt"], a);
        std::fs::write(&p, "{rozbité").unwrap();
        assert!(load(&p).is_empty());
    }

    /// Stránka v okně s webem nesmí dostat IPC: žádná capability nesmí jmenovat okna `web-*`,
    /// zástupné `*` (jediná výjimka: `note-*` = lístky na plochu, `pins/`, s vlastním UI Ledge)
    /// ani povolovat vzdálené adresy (`remote`). Tauri ≥ 2.11 pak z ne-lokálního
    /// původu odmítne i vlastní příkazy aplikace (`Webview::on_message`, „not allowed by ACL“).
    #[test]
    fn capabilities_never_reach_web_windows() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_none_or(|x| x != "json") {
                continue;
            }
            n += 1;
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            assert!(v.get("remote").is_none(), "{p:?}: remote capability");
            for key in ["windows", "webviews"] {
                for w in v.get(key).and_then(|x| x.as_array()).into_iter().flatten() {
                    let w = w.as_str().unwrap_or("");
                    let pins = w == format!("{}*", crate::pins::LABEL_PREFIX) || w == format!("{}*", crate::desk::LABEL_PREFIX);
                    assert!((pins || !w.contains('*')) && !w.starts_with(LABEL_PREFIX), "{p:?}: {key} {w}");
                }
            }
        }
        assert!(n > 0);
        // label okna s webem se nikdy nepotká s šuplíkem
        assert_ne!(label("drawer"), crate::drawer::LABEL);
        assert!(label("x").starts_with(LABEL_PREFIX));
    }
}
