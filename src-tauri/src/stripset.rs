//! Notch na každém monitoru (`strip.monitor`: `primary` | `all` | `dev:<zařízení>;…`, kolo 34, L162, L178).
//!
//! Plný proužek (`strip.rs` + `Core.strip`) je jen na JEDNOM monitoru: u `primary` / `all` na hlavním, u
//! `dev:…` na zvoleném. Volba `all` navíc přidá na každý další monitor **tečku**: vedlejší proužek
//! (`Strip::start_on(true)`, styl `dot` ze `stripclassic.rs`, jen tečky stavů + puls). Je to malé nativní okno
//! (žádný WebView2): vlastní vlákno `strip2` + vlastní D2D factory (`SINGLE_THREADED`, nesdílí se) a DIB na pár
//! desítek × ~30 px. Paměť jedné tečky: odhad +1–3 MB private (D2D factory + software render target + zásobník
//! vlákna; DWrite factory je `SHARED`, tedy sdílená v procesu), neměřeno — přijetí je `scripts/perf.ps1` na dvou
//! monitorech. V klidu žádný časovač (puls a změna šířky jedou jen po dobu události, stejně jako hlavní proužek).
//!
//! Životní cyklus: `stripview::refresh_strip` po každé změně obsahu zavolá `apply` (vznik / zánik / nový model);
//! změnu sestavy monitorů hlásí `display_event` (wndproc hlavního okna) → vlákno `hover` → `refresh_strip`.
//! Otevření šuplíku z tečky na vedlejším monitoru zatím NEEXISTUJE: tečka je click-through indikátor a hover
//! zóna i šuplík zůstávají na monitoru plného proužku (předělá balík `hovermon`, `threads/tracker.rs`).
//!
//! Stabilní identita: `strip.monitor` se ukládá jako `dev:<zařízení>;<š>x<v>@<x>,<y>` (`screen::stable_spec`).
//! Starý číselný index se při načtení configu přepíše (`migrate_config`) a uloží (`settings::read_disk`), takže
//! po restartu a přeskládání monitorů notch neskočí jinam.

use crate::appearance::StripStyle;
use crate::appearance::geom::StripMetrics;
use crate::config::Config;
use crate::screen::{self, Ident, Monitor, SPEC_ALL};
use crate::strip::{Strip, StripModel};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use windows::Win32::Foundation::RECT;

/// Indexy monitorů s vedlejším proužkem (tečkou): jen u `all`, všechny kromě monitoru plného proužku.
pub fn secondaries(spec: &str, all: &[Ident]) -> Vec<usize> {
    if spec != SPEC_ALL {
        return Vec::new();
    }
    let main = screen::resolve(spec, all, None).idx;
    (0..all.len()).filter(|&i| i != main).collect()
}

/// Starý číselný index → stabilní spec monitoru, který ten index dnes označuje; index mimo sestavu = `primary`
/// (stejný výsledek, jaký dával `resolve`). `None` = nic k přepsání (není číslo, nebo zrovna žádný monitor — RDP).
pub fn migrate_spec(spec: &str, all: &[Ident]) -> Option<String> {
    let i: usize = spec.trim().parse().ok()?;
    if all.is_empty() {
        return None;
    }
    Some(all.get(i).map_or_else(|| "primary".to_string(), |m| screen::stable_spec(m.device, &m.rect)))
}

fn migrate_with(cfg: &mut Config, all: &[Ident]) -> bool {
    let Some(new) = migrate_spec(&cfg.strip.monitor, all) else { return false };
    cfg.strip.monitor = new;
    true
}

/// Při načtení configu: číselný `strip.monitor` → stabilní spec. Výčet monitorů jen u čísla (jinak žádné volání OS).
/// `true` = config se změnil a patří zpět na disk.
pub fn migrate_config(cfg: &mut Config) -> bool {
    if cfg.strip.monitor.trim().parse::<usize>().is_err() {
        return false;
    }
    let all = screen::monitors();
    let old = cfg.strip.monitor.clone();
    let changed = screen::with_idents(&all, |ids| migrate_with(cfg, ids));
    if changed {
        crate::log(&format!("strip.monitor „{old}“ → „{}“ (stabilní identita monitoru)", cfg.strip.monitor));
    }
    changed
}

/// Co se vedlejšímu proužku liší od hlavního (počítá `refresh_strip`, stejné pro všechny monitory).
#[derive(Clone, Copy)]
pub struct DotView {
    pub visible: bool,
    pub line_only: bool,
    pub metrics: StripMetrics,
}

/// Model tečky pro monitor `mon`: barvy, stavové segmenty (kreslí se jen ty s tečkou), ambient a puls z hlavního
/// modelu; styl `dot`, osa a DPI podle monitoru. Bez oznámení, přihrádky, stylu zavřeného Ledge a živé hrany.
pub fn secondary_model(main: &StripModel, mon: &Monitor, axis2: i32, v: &DotView) -> StripModel {
    StripModel {
        visible: v.visible,
        line_only: v.line_only,
        monitor: mon.rect,
        dpi: mon.dpi,
        style: StripStyle::Dot,
        metrics: v.metrics,
        axis2: Some(axis2),
        segments: main.segments.clone(),
        ambient: main.ambient,
        opacity: main.opacity,
        palette: main.palette,
        line: main.line,
        pulse: main.pulse,
        ..StripModel::default()
    }
}

/// Co zrušit a co založit, když běží proužky `have` a mají být `want` (klíč = název zařízení, bez ohledu na velikost písmen).
pub fn plan(have: &[String], want: &[String]) -> (Vec<String>, Vec<String>) {
    let has = |l: &[String], k: &str| l.iter().any(|x| x.eq_ignore_ascii_case(k));
    let stop = have.iter().filter(|k| !has(want, k)).cloned().collect();
    let start = want.iter().filter(|k| !has(have, k)).cloned().collect();
    (stop, start)
}

static SET: Mutex<Vec<(String, Arc<Strip>)>> = Mutex::new(Vec::new());

/// Srovná běžící vedlejší proužky s `targets` (monitor + osa) a pošle jim model z hlavního `main`.
/// `Strip::set` překreslí jen při skutečné změně; prázdné `targets` zruší všechny.
pub fn apply(main: &StripModel, targets: &[(Monitor, i32)], v: &DotView) {
    let mut set = SET.lock();
    let have: Vec<String> = set.iter().map(|(k, _)| k.clone()).collect();
    let want: Vec<String> = targets.iter().map(|(m, _)| m.device_name()).collect();
    if have.is_empty() && want.is_empty() {
        return;
    }
    let (stop, start) = plan(&have, &want);
    set.retain(|(k, s)| {
        let gone = stop.contains(k);
        if gone {
            s.stop();
        }
        !gone
    });
    for k in start {
        set.push((k, Strip::start_on(true)));
    }
    for (m, axis2) in targets {
        let key = m.device_name();
        if let Some((_, s)) = set.iter().find(|(k, _)| k.eq_ignore_ascii_case(&key)) {
            let model = secondary_model(main, m, *axis2, v);
            s.set(|x| *x = model);
        }
    }
}

/// Monitory s vedlejším proužkem pro `spec` z cache podle počítadla změn zobrazení (`state::display_gen`) a
/// pojistné lhůty `MONITOR_TTL`; mimo `all` prázdné a bez volání OS (`refresh_strip` běží často).
pub fn secondary_monitors(spec: &str) -> Vec<Monitor> {
    static CACHE: Mutex<Option<(u64, Instant, Vec<Monitor>)>> = Mutex::new(None);
    if spec != SPEC_ALL {
        *CACHE.lock() = None;
        return Vec::new();
    }
    let gen_now = crate::state::display_gen();
    let mut c = CACHE.lock();
    if let Some((g, at, v)) = c.as_ref()
        && *g == gen_now
        && at.elapsed() < crate::state::MONITOR_TTL
    {
        return v.clone();
    }
    let all = screen::monitors();
    let v: Vec<Monitor> = screen::with_idents(&all, |ids| secondaries(spec, ids)).into_iter().filter_map(|i| all.get(i).copied()).collect();
    *c = Some((gen_now, Instant::now(), v.clone()));
    v
}

/// Obdélník tečky na monitoru `device` (hover zóna, `threads/hover.rs`); prázdný = tečka skrytá / ještě nevznikla.
pub fn rect_of(device: &str) -> RECT {
    SET.lock().iter().find(|(k, _)| k.eq_ignore_ascii_case(device)).map_or(RECT::default(), |(_, s)| *s.rect.lock())
}

/// Monitor pro volbu v Nastavení (`strip.monitor`): `id` je stabilní spec, který se uloží do configu.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    /// číslo displeje z Windows (`\.\DISPLAY2` → 2), jinak pořadí
    pub n: usize,
    pub w: i32,
    pub h: i32,
    pub primary: bool,
}

/// Číslo displeje z názvu zařízení (`\.\DISPLAY2` → 2); bez čísla `fallback`.
pub fn display_no(device: &str, fallback: usize) -> usize {
    let digits = device.len() - device.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    device[device.len() - digits..].parse().unwrap_or(fallback)
}

pub fn infos(all: &[Ident]) -> Vec<MonitorInfo> {
    all.iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            id: screen::stable_spec(m.device, &m.rect),
            n: display_no(m.device, i + 1),
            w: m.rect.right - m.rect.left,
            h: m.rect.bottom - m.rect.top,
            primary: m.primary,
        })
        .collect()
}

/// IPC pro Nastavení: připojené monitory (výběr `strip.monitor`: Hlavní / Všechny / konkrétní).
#[tauri::command]
pub(crate) fn monitors_list() -> Vec<MonitorInfo> {
    let all = screen::monitors();
    screen::with_idents(&all, infos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::RECT;

    fn r(l: i32, t: i32, w: i32, h: i32) -> RECT {
        RECT { left: l, top: t, right: l + w, bottom: t + h }
    }

    const D1: &str = r"\.\DISPLAY1";
    const D2: &str = r"\.\DISPLAY2";
    const ST1: &str = r"dev:\.\DISPLAY1;1920x1080@0,0";
    const ST2: &str = r"dev:\.\DISPLAY2;2560x1440@1920,0";

    fn two() -> [Ident<'static>; 2] {
        [Ident { device: D1, rect: r(0, 0, 1920, 1080), primary: true }, Ident { device: D2, rect: r(1920, 0, 2560, 1440), primary: false }]
    }

    #[test]
    fn secondaries_table() {
        let all = two();
        // tři monitory, hlavní uprostřed seznamu, jeden vlevo od něj (záporné x)
        let three = [
            Ident { device: "A", rect: r(-1920, 0, 1920, 1080), primary: false },
            Ident { device: "B", rect: r(0, 0, 1920, 1080), primary: true },
            Ident { device: "C", rect: r(1920, 0, 1920, 1080), primary: false },
        ];
        let none: [Ident; 0] = [];
        let cases: Vec<(&str, &[Ident], Vec<usize>)> = vec![
            ("all", &all, vec![1]),
            ("all", &three, vec![0, 2]),
            ("primary", &all, vec![]),
            (ST2, &all, vec![]),
            ("1", &all, vec![]),
            ("all", &all[..1], vec![]), // jediný monitor: žádná tečka
            ("all", &none, vec![]),     // RDP bez monitoru
        ];
        for (spec, ids, want) in cases {
            assert_eq!(secondaries(spec, ids), want, "{spec}");
        }
    }

    /// Starý config s indexem → stabilní spec; po přeskládání monitorů drží zařízení, ne číslo.
    #[test]
    fn old_config_index_becomes_stable_spec() {
        let mut cfg: Config = serde_json::from_str(r#"{"strip":{"monitor":"1"}}"#).unwrap();
        let all = two();
        assert!(migrate_with(&mut cfg, &all));
        assert_eq!(cfg.strip.monitor, ST2);
        // uložený a znovu načtený config už nic nepřepisuje
        let mut again: Config = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(again.strip.monitor, ST2);
        assert!(!migrate_with(&mut again, &all));
        // enumerace přeskládaná (DISPLAY2 je teď první): spec pořád míří na DISPLAY2
        let sw = [Ident { device: D2, rect: all[1].rect, primary: false }, Ident { device: D1, rect: all[0].rect, primary: true }];
        assert_eq!(screen::resolve(&cfg.strip.monitor, &sw, None).idx, 0);
        // config bez klíče = výchozí `primary`, nic se nemění
        let mut def: Config = serde_json::from_str("{}").unwrap();
        assert!(!migrate_with(&mut def, &all));
        assert_eq!(def.strip.monitor, "primary");
    }

    #[test]
    fn migrate_spec_table() {
        let all = two();
        let cases: Vec<(&str, &[Ident], Option<&str>)> = vec![
            ("0", &all, Some(ST1)),
            ("1", &all, Some(ST2)),
            (" 1 ", &all, Some(ST2)),
            ("3", &all, Some("primary")), // index mimo sestavu
            ("1", &[], None),             // žádný monitor: zkusí se příště
            ("primary", &all, None),
            ("all", &all, None),
            (ST2, &all, None),
            ("", &all, None),
        ];
        for (spec, ids, want) in cases {
            assert_eq!(migrate_spec(spec, ids).as_deref(), want, "{spec:?}");
        }
    }

    #[test]
    fn plan_table() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let cases = [
            (s(&[]), s(&[]), (s(&[]), s(&[]))),
            (s(&[]), s(&["A"]), (s(&[]), s(&["A"]))),
            (s(&["A"]), s(&["a"]), (s(&[]), s(&[]))),         // velikost písmen nevadí
            (s(&["A", "B"]), s(&["B"]), (s(&["A"]), s(&[]))), // odpojen
            (s(&["A"]), s(&[]), (s(&["A"]), s(&[]))),         // volba už není `all`
            (s(&["A"]), s(&["B", "C"]), (s(&["A"]), s(&["B", "C"]))),
        ];
        for (have, want, out) in cases {
            assert_eq!(plan(&have, &want), out, "{have:?} → {want:?}");
        }
    }

    #[test]
    fn secondary_model_is_a_quiet_dot() {
        use crate::strip::{Rgba, Segment};
        let main = StripModel {
            segments: vec![Segment { text: "Claude čeká".into(), color: Rgba::TEXT, dot: Some(Rgba::AMBER), strong: true, pill: true }],
            ambient: Some(Rgba::AMBER),
            hot: true,
            missed: Some(Rgba::RED),
            axis2: Some(1),
            ..StripModel::default()
        };
        let mon = Monitor { handle: 1, rect: r(1920, 0, 2560, 1440), work: r(1920, 0, 2560, 1400), primary: false, dpi: 144, device: [0; 32] };
        let v = DotView { visible: true, line_only: false, metrics: main.metrics };
        let m = secondary_model(&main, &mon, 7000, &v);
        assert_eq!((m.style, m.dpi, m.axis2, m.monitor.left), (StripStyle::Dot, 144, Some(7000), 1920));
        assert_eq!(m.ambient, Some(Rgba::AMBER), "stav (barva události) se přenáší");
        assert_eq!(m.segments, main.segments, "kreslí se jen segmenty s tečkou (stripclassic, styl dot)");
        assert!(m.show.is_none() && m.notice.is_none() && m.missed.is_none() && !m.hot, "žádný styl zavřeného Ledge, oznámení ani nažhavení");
        assert_eq!(m.shelf, crate::shelf::Hint::None);
        assert!(m.visible && !m.line_only);
        let off = secondary_model(&main, &mon, 7000, &DotView { visible: false, line_only: true, ..v });
        assert!(!off.visible && off.line_only);
    }

    #[test]
    fn infos_and_display_no() {
        let all = two();
        let i = infos(&all);
        assert_eq!((i[1].n, i[1].w, i[1].h, i[1].primary), (2, 2560, 1440, false));
        assert_eq!(i[0].id, ST1);
        assert_eq!(display_no(r"\.\DISPLAY12", 1), 12);
        assert_eq!(display_no("Virtual", 3), 3);
        assert_eq!(display_no("", 4), 4);
    }
}
