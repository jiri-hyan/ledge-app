//! Co se stane po najetí myší na proužek (`drawer.hover`): rovnou celý šuplík (chování před vlnou 7),
//! dok / ostrůvek (`View::Island`, z docku střední panel `View::Mid`). Tady jsou jen čisté
//! funkce — kam najetí otevře, kam vede zavření s čekajícím Claudem, velikosti malých pohledů a
//! prodleva automatického rozbalení. Okno řídí `drawer.rs`, rozhodnutí volá vlákno `hover`
//! (`threads/hover.rs`), zavření vlákno `ui`. Obsah malých pohledů kreslí frontend (`src/hover/*`) jen
//! z dat, která snapshot už má — nic drahého (útrata, kalendář, git) se kvůli nim nespouští.

use crate::drawer::View;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HoverMode {
    /// rovnou celý šuplík (výchozí, chování před vlnou 7)
    #[default]
    Full,
    /// černá kapsle pod proužkem s jedním nejdůležitějším děním a tlačítky
    Island,
    /// dok: totéž okno jako ostrůvek (`View::Island`), ale frontend kreslí řadu slotů (poznámka, hudba s
    /// hlasitostí, 📌); žádné nové okno ani WebView — liší se jen kresba a výchozí velikost
    /// (uložené `"compact"` z kola ≤ 33 se načte jako `dock`: dok → klik / setrvání otevře střední panel s kartou
    /// Claude — sessiony, schválení, hudba a čas, tedy to, co kompaktní náhled ukazoval; uloží se příště jako `dock`)
    #[serde(alias = "compact")]
    Dock,
}

impl HoverMode {
    /// Pohled, který najetí otevře.
    pub fn view(self) -> View {
        match self {
            HoverMode::Full => View::Full,
            HoverMode::Island | HoverMode::Dock => View::Island,
        }
    }
}

/// Co může okno šuplíku zavřít samo (bez výslovného zavření z UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseCause {
    /// okno ztratilo fokus (`set_on_blur`)
    Blur,
    /// myš odjela z okna (vlákno `hover`)
    MouseLeave,
    /// přechod do fullscreenu (hra, video)
    Fullscreen,
    /// zapnutí herního režimu
    GameMode,
}

/// Zavře `cause` šuplík otevřený `opener`em? Jediná tabulka pro blur / odjetí myši / fullscreen / herní režim
/// (volající: blur v `lib.rs`, `threads/hover.rs`, `gamemode::set`): myší otevřený se zavírá odjetím,
/// zkratkou / klikem otevřený blurem, `Remote` a `Pinned` (📌) ani jedním — ale fullscreen a herní režim
/// zavřou VŽDY (ve hře nesmí zůstat okno nad hrou).
pub fn closes(opener: crate::drawer::Opener, cause: CloseCause) -> bool {
    use crate::drawer::Opener::*;
    match cause {
        CloseCause::Fullscreen | CloseCause::GameMode => true,
        CloseCause::Blur => opener == Hotkey,
        CloseCause::MouseLeave => opener == Hover,
    }
}

/// Dok chce hlasitost živou: je otevřený a hlasitost je povolená (`volume::allowed`). Čistá funkce pro
/// `threads/ui.rs`; mimo dok vlákno Core Audio spustí první Osobní (`personal_live`). Schované zanikají
/// v každém režimu (dok ≥ 60 s, jinak ≥ 120 s, vlákno `hover`) a Full/Mid je vrací (`volume::resume`).
pub fn volume_wants_live(mode: HoverMode, view: View, volume_allowed: bool) -> bool {
    mode == HoverMode::Dock && matches!(view, View::Island | View::Full) && volume_allowed
}

/// Dok i ostrůvek chtějí hudbu živou, když se zobrazí (L177): režim `dock` / `island`, pohled Island / Full a hudba
/// povolená (`media::allowed`, NEZÁVISLE na hlasitosti). Nic neběží ve schovaném šuplíku ani v režimu `full`
/// (tam hudbu spouští Osobní `personal_live`). Schované vlákno `media::release_due` zruší; další zobrazení ho vrátí.
pub fn music_wants_live(mode: HoverMode, view: View, media_allowed: bool) -> bool {
    mode != HoverMode::Full && matches!(view, View::Island | View::Full) && media_allowed
}

/// Zkratka poznámky otevře dok (rychlá poznámka) místo stránky „note“ ve Full: režim `dock` a šuplík
/// není právě celý (z otevřeného Full se jde na stránku „note“ jako dřív).
pub fn note_in_dock(mode: HoverMode, view: View) -> bool {
    mode == HoverMode::Dock && view != View::Full
}

/// Globální šipky pro dok (↑↓ hlasitost, ←→ seek) — fáze B, NEREGISTRUJE se (čeká na živé ověření, že
/// `global-hotkey` přijme klávesu bez modifikátoru). Chtěné jen když: dok je otevřený (`View::Island`),
/// slot hudby je vidět, kurzor je v okně a Ledge není mrtvý (fullscreen / herní režim).
#[allow(dead_code)] // fáze B: zatím jen test, registrace šipek čeká na živé ověření
pub fn dock_keys_wanted(view: View, music_slot: bool, cursor_in: bool, dead: bool) -> bool {
    view == View::Island && music_slot && cursor_in && !dead
}

/// Výchozí `drawer.hoverExpandMs`: za jak dlouho setrvání myši se malý pohled rozbalí do celého šuplíku.
/// 1,2 s: delší než otevření podržením (`dwellDelayMs` 450 ms) — přečíst 3 řádky / jeden řádek ostrůvku
/// stihneš, a kdo zůstane, chce víc. Nad tlačítky a s čekajícím schválením frontend neodpočítává
/// (`src/hover/model.ts` `expandAfter`), takže ⏯ ani Povolit rozbalení „nepředběhne“.
pub const EXPAND_MS: u64 = 1200;
/// Nejkratší / nejdelší prodleva (kratší by malý pohled hned přeskočil, delší nedává smysl).
pub const EXPAND_MIN_MS: u64 = 400;
pub const EXPAND_MAX_MS: u64 = 10_000;

/// Prodleva z configu oříznutá do rozsahu; 0 = rozbalit jen klikem.
pub fn expand_ms(cfg: u64) -> u64 {
    if cfg == 0 { 0 } else { cfg.clamp(EXPAND_MIN_MS, EXPAND_MAX_MS) }
}

/// Kam najetí otevře. `full` = výsledek `edge::hover_open` (false = Claude čeká → peek se schválením ve
/// všech režimech — jedno UI pro schválení), `over_peek` = myš podržená nad peekem (připomínka) →
/// rozbalit rovnou do celého šuplíku jako dřív; jinak pohled podle `drawer.hover`.
pub fn hover_target(mode: HoverMode, full: bool, over_peek: bool) -> View {
    if !full {
        View::Peek
    } else if over_peek {
        View::Full
    } else {
        mode.view()
    }
}

/// Zavření (odjetí myši, blur, spuštění, Esc): s čekajícím Claudem se šuplík i malé pohledy stáhnou do
/// peeku se schválením (ne ve fullscreen / herním režimu); peek sám a ostatní se schovají.
pub fn after_close(view: View, attention: bool, dead: bool) -> View {
    if attention && !dead && view.covers_strip() { View::Peek } else { View::Hidden }
}

/// Pohled podle jména (`/ctl/open {"view": …}`); neznámé = celý šuplík.
pub fn view_by_name(name: Option<&str>) -> View {
    match name {
        Some("peek") => View::Peek,
        Some("island") | Some("dock") => View::Island,
        Some("mid") => View::Mid,
        _ => View::Full,
    }
}

/// Velikosti malých pohledů (logické px) — výchozí, dokud frontend nenahlásí skutečný obsah.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmallSizes {
    peek: (f64, f64),
    island: (f64, f64),
    mid: (f64, f64),
}

/// Výchozí velikost ostrůvku a docku (logické px), dokud frontend nenahlásí skutečný obsah.
pub const ISLAND_SIZE: (f64, f64) = (480.0, 64.0);
pub const DOCK_SIZE: (f64, f64) = (640.0, 64.0);
/// Výchozí velikost středního panelu (šířku určuje `mid_width`, výška se přepíše skutečným obsahem).
pub const MID_SIZE: (f64, f64) = (640.0, 360.0);

impl Default for SmallSizes {
    fn default() -> Self {
        Self { peek: (680.0, 72.0), island: ISLAND_SIZE, mid: MID_SIZE }
    }
}

impl SmallSizes {
    fn slot(&mut self, view: View) -> Option<&mut (f64, f64)> {
        match view {
            View::Peek => Some(&mut self.peek),
            View::Island => Some(&mut self.island),
            View::Mid => Some(&mut self.mid),
            View::Hidden | View::Full => None,
        }
    }

    /// Velikost malého pohledu; `None` = není malý (Full / Hidden).
    pub fn get(&self, view: View) -> Option<(f64, f64)> {
        match view {
            View::Peek => Some(self.peek),
            View::Island => Some(self.island),
            View::Mid => Some(self.mid),
            View::Hidden | View::Full => None,
        }
    }

    /// Změna `drawer.hover`: dok má širší výchozí velikost než ostrůvek. Mění jen velikost, kterou
    /// frontend ještě nepřepsal skutečným obsahem (rovná se výchozí hodnotě druhého režimu).
    pub fn set_mode(&mut self, mode: HoverMode) {
        let (from, to) = if mode == HoverMode::Dock { (ISLAND_SIZE, DOCK_SIZE) } else { (DOCK_SIZE, ISLAND_SIZE) };
        if self.island == from {
            self.island = to;
        }
    }

    /// Hlášení z frontendu; vrací true = změnilo se (o víc než půl px).
    pub fn set(&mut self, view: View, w: f64, h: f64) -> bool {
        let Some(v) = self.slot(view) else { return false };
        let changed = (v.0 - w).abs() > 0.5 || (v.1 - h).abs() > 0.5;
        *v = (w, h);
        changed
    }
}

/// Šířka středního panelu (L147): třetina monitoru (logické px), nejméně `MID_MIN_W`, nejvýš `MID_MAX_W`
/// a nikdy víc než monitor. Počítá se z šířky monitoru při každém zobrazení — nic se neukládá.
pub const MID_MIN_W: f64 = 520.0;
pub const MID_MAX_W: f64 = 760.0;
pub fn mid_width(monitor_w: f64) -> f64 {
    (monitor_w / 3.0).clamp(MID_MIN_W, MID_MAX_W).min(monitor_w.max(1.0)).round()
}

/// Střední panel: šířka podle monitoru (`mid_width`), výška z obsahu oříznutá na strop pracovní plochy.
pub fn fit_mid(h: f64, monitor_w: f64, max_h: f64) -> (f64, f64) {
    (mid_width(monitor_w), h.min(max_h.max(1.0)))
}

/// Kolik karet smí mít střední panel.
pub const DOCK_CARDS_MAX: usize = 3;
/// Výchozí karty středního panelu (`drawer.dockCards`): poznámky (a úkoly v nich), kalendář, Claude.
pub const DOCK_CARDS_DEFAULT: [&str; 3] = ["notes", "calendar", "claude"];

/// `drawer.dockCards` z configu: jen známé vestavěné karty (`layout::CARDS`), bez opakování, nejvýš
/// `DOCK_CARDS_MAX`; po ověření prázdné (nic nezbylo, starý config bez klíče) = výchozí. Čistá funkce.
pub fn dock_cards(cfg: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in cfg {
        if out.len() == DOCK_CARDS_MAX {
            break;
        }
        // sloučená karta (kolo 34: `week` → `calendar` …) se převede na nástupce; vzhled (`design`) dok nenese
        let id = crate::layout::merged_target(id).map_or(id.as_str(), |(new, _)| new).to_string();
        if crate::layout::CARDS.contains(&id.as_str()) && !out.contains(&id) {
            out.push(id);
        }
    }
    if out.is_empty() { DOCK_CARDS_DEFAULT.iter().map(|s| s.to_string()).collect() } else { out }
}

/// Kam vede rozbalení náhledu (klik / setrvání / „▾ celý šuplík“): dok (`drawer.hover = dock`, `View::Island`) →
/// střední panel, ostrůvek a střední panel → celý šuplík; `to_full` = výslovně rovnou celý
/// šuplík (schválení / otázka v docku přebírá ostrůvek, ten střední krok nepotřebuje). Ostatní pohledy: nic.
pub fn expand_target(view: View, dock: bool, to_full: bool) -> Option<View> {
    match view {
        View::Island if dock && !to_full => Some(View::Mid),
        View::Island | View::Mid => Some(View::Full),
        _ => None,
    }
}

/// Malý pohled oříznutý na monitor: šířka nejvýš šířka šuplíku (`width`, už oříznutá na monitor),
/// výška nejvýš strop pracovní plochy (`max_h`). Peek se neořezává (chování před vlnou 7).
pub fn fit_small(view: View, (w, h): (f64, f64), width: f64, max_h: f64) -> (f64, f64) {
    match view {
        View::Island => (w.min(width.max(1.0)), h.min(max_h.max(1.0))),
        View::Mid => fit_mid(h, width, max_h),
        _ => (w, h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_width_table() {
        // (monitor v logických px, šířka panelu): třetina, dolní a horní mez, malý monitor
        for (mon, want) in [(1920.0, 640.0), (1366.0, 520.0), (1280.0, 520.0), (800.0, 520.0), (400.0, 400.0), (2560.0, 760.0), (3840.0, 760.0)] {
            assert_eq!(mid_width(mon), want, "monitor {mon}");
        }
        assert_eq!(fit_mid(900.0, 1920.0, 600.0), (640.0, 600.0), "výška ořezaná na strop");
        assert_eq!(fit_mid(200.0, 1920.0, 600.0), (640.0, 200.0));
        assert_eq!(fit_small(View::Mid, (1.0, 300.0), 1920.0, 600.0), (640.0, 300.0));
    }

    #[test]
    fn dock_cards_table() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let def = v(&DOCK_CARDS_DEFAULT);
        for (input, want) in [
            (v(&[]), def.clone()),                                       // starý config bez klíče
            (v(&["nesmysl", ""]), def.clone()),                           // nic platného → výchozí
            (v(&["claude"]), v(&["claude"])),
            (v(&["timer", "x", "clock"]), v(&["timer", "clock"])),       // neznámé vynechat
            (v(&["claude", "claude", "notes"]), v(&["claude", "notes"])), // bez opakování
            (v(&["claude", "notes", "calendar", "mail"]), v(&["claude", "notes", "calendar"])), // max 3
            (v(&["x", "a", "projects", "running", "repos", "media"]), v(&["projects", "running", "repos"])),
            (v(&["widget:x.js"]), def.clone()),                          // dynamické karty ne
            (v(&["week", "bigclock", "links", "toggles"]), v(&["calendar", "clock", "actions"])), // sloučené karty (kolo 34) → nástupce, max 3
            (v(&["week", "calendar"]), v(&["calendar"])),                // nástupce se nezdvojí
        ] {
            assert_eq!(dock_cards(&input), want, "{input:?}");
        }
        assert!(DOCK_CARDS_DEFAULT.iter().all(|c| crate::layout::CARDS.contains(c)), "výchozí karty existují");
    }

    #[test]
    fn expand_target_table() {
        use View::*;
        // (pohled, dok, rovnou celý) → kam
        for (v, dock, full, want) in [
            (Island, true, false, Some(Mid)),
            (Island, true, true, Some(Full)),
            (Island, false, false, Some(Full)),
            (Mid, true, false, Some(Full)),
            (Full, true, false, None),
            (Peek, true, false, None),
            (Hidden, true, false, None),
        ] {
            assert_eq!(expand_target(v, dock, full), want, "{v:?} dok={dock} full={full}");
        }
        assert!(View::Mid.covers_strip() && View::Mid.has_cards() && View::Full.has_cards() && !View::Island.has_cards());
    }

    #[test]
    fn hover_target_table() {
        use HoverMode::*;
        // (režim, full, nad peekem) → pohled
        let cases = [
            (Full, true, false, View::Full),
            (Island, true, false, View::Island),
            (Dock, true, false, View::Island),
            // Claude čeká → peek se schválením ve všech režimech
            (Full, false, false, View::Peek),
            (Island, false, false, View::Peek),
            (Dock, false, false, View::Peek),
            // podržení nad připomínkou → celý šuplík jako dřív
            (Island, true, true, View::Full),
            (Dock, true, true, View::Full),
            (Full, true, true, View::Full),
        ];
        for (mode, full, over, want) in cases {
            assert_eq!(hover_target(mode, full, over), want, "{mode:?} full={full} over={over}");
        }
    }

    #[test]
    fn after_close_table() {
        // (pohled, Claude čeká, mrtvý) → po zavření
        let cases = [
            (View::Full, true, false, View::Peek),
            (View::Island, true, false, View::Peek),
            (View::Full, false, false, View::Hidden),
            (View::Island, false, false, View::Hidden),
            (View::Peek, true, false, View::Hidden),
            (View::Island, true, true, View::Hidden), // fullscreen / herní režim: nic
            (View::Hidden, true, false, View::Hidden),
        ];
        for (v, att, dead, want) in cases {
            assert_eq!(after_close(v, att, dead), want, "{v:?} attention={att} dead={dead}");
        }
    }

    #[test]
    fn covers_strip_table() {
        let cases = [(View::Hidden, false), (View::Peek, false), (View::Full, true), (View::Island, true), (View::Mid, true)];
        for (v, want) in cases {
            assert_eq!(v.covers_strip(), want, "{v:?}");
        }
    }

    #[test]
    fn expand_ms_table() {
        for (cfg, want) in [(0, 0), (1, 400), (399, 400), (1200, 1200), (10_000, 10_000), (60_000, 10_000)] {
            assert_eq!(expand_ms(cfg), want, "{cfg}");
        }
    }

    #[test]
    fn view_by_name_table() {
        let cases = [(Some("peek"), View::Peek), (Some("island"), View::Island), (Some("dock"), View::Island), (Some("mid"), View::Mid), (Some("full"), View::Full), (Some("x"), View::Full), (None, View::Full)];
        for (n, want) in cases {
            assert_eq!(view_by_name(n), want, "{n:?}");
        }
    }

    #[test]
    fn small_sizes_and_fit() {
        let mut s = SmallSizes::default();
        assert_eq!(s.get(View::Full), None);
        assert_eq!(s.get(View::Hidden), None);
        assert_eq!(s.get(View::Island), Some((480.0, 64.0)));
        assert!(s.set(View::Island, 480.0, 300.0));
        assert!(!s.set(View::Island, 480.2, 300.3), "pod půl px = beze změny");
        assert!(!s.set(View::Full, 1.0, 1.0), "Full se tu neukládá");
        assert_eq!(s.get(View::Island), Some((480.2, 300.3)));
        assert_eq!(s.get(View::Peek), Some((680.0, 72.0)), "jiné pohledy beze změny");
        // ořez: úzký šuplík / nízký monitor; peek se neořezává
        assert_eq!(fit_small(View::Island, (480.0, 900.0), 360.0, 600.0), (360.0, 600.0));
        assert_eq!(fit_small(View::Island, (480.0, 64.0), 1080.0, 600.0), (480.0, 64.0));
        assert_eq!(fit_small(View::Peek, (680.0, 900.0), 360.0, 600.0), (680.0, 900.0));
    }

    /// Skutečný tvar configu z vlny 6 (drawer bez `hover`) — načte se beze změny chování.
    const OLD: &str = r#"{
      "strip": { "enabled": true, "items": ["mode", "clock", "running", "claude"] },
      "drawer": { "trigger": "hover", "hoverDelayMs": 120, "dwellDelayMs": 450, "closeDelayMs": 350, "width": 1080,
                  "glass": "solid", "keepWarmS": 60, "barWindows": { "trigger": "push", "pushMs": 300 } },
      "appearance": { "style": "classic", "theme": "dark", "motion": { "style": "spring" } },
      "projects": []
    }"#;

    #[test]
    fn config_old_and_new_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, OLD).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!((cfg.drawer.hover, cfg.drawer.hover_expand_ms), (HoverMode::Full, EXPAND_MS), "starý config = dnešní chování");
        assert_eq!((cfg.drawer.hover_delay_ms, cfg.drawer.keep_warm_s), (120, 60), "ostatní pole beze ztráty");
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!((json["drawer"]["hover"].as_str(), json["drawer"]["hoverExpandMs"].as_u64()), (Some("full"), Some(EXPAND_MS)));

        let new = OLD.replace(r#""keepWarmS": 60,"#, r#""keepWarmS": 60, "hover": "island", "hoverExpandMs": 0,"#);
        std::fs::write(&path, new).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!((cfg.drawer.hover, cfg.drawer.hover_expand_ms), (HoverMode::Island, 0));
        assert!(crate::config::validate(&cfg).is_ok());

        let dock = OLD.replace(r#""keepWarmS": 60,"#, r#""keepWarmS": 60, "hover": "dock","#);
        std::fs::write(&path, dock).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!((cfg.drawer.hover, cfg.drawer.hover_expand_ms), (HoverMode::Dock, EXPAND_MS));
        assert_eq!(serde_json::to_value(&cfg).unwrap()["drawer"]["hover"].as_str(), Some("dock"));
        assert_eq!(dock_cards(&cfg.drawer.dock_cards), dock_cards(&[]), "starý config bez dockCards = výchozí tři karty");
        assert_eq!(serde_json::to_value(&cfg).unwrap()["drawer"]["dockCards"], serde_json::json!(["notes", "calendar", "claude"]));

        // kolo 34: kompaktní náhled zrušen → starý config s `"hover": "compact"` se načte (neselže) jako dok
        let compact = OLD.replace(r#""keepWarmS": 60,"#, r#""keepWarmS": 60, "hover": "compact", "hoverExpandMs": 2500,"#);
        std::fs::write(&path, compact).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!((cfg.drawer.hover, cfg.drawer.hover_expand_ms), (HoverMode::Dock, 2500));
        assert!(crate::config::validate(&cfg).is_ok());
        assert_eq!(serde_json::to_value(&cfg).unwrap()["drawer"]["hover"].as_str(), Some("dock"));

        let own = OLD.replace(r#""keepWarmS": 60,"#, r#""keepWarmS": 60, "dockCards": ["timer", "nope", "clock"],"#);
        std::fs::write(&path, own).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(dock_cards(&cfg.drawer.dock_cards), vec!["timer".to_string(), "clock".to_string()]);
    }

    #[test]
    fn dock_default_size_follows_mode() {
        let mut s = SmallSizes::default();
        s.set_mode(HoverMode::Dock);
        assert_eq!(s.get(View::Island), Some(DOCK_SIZE));
        s.set_mode(HoverMode::Dock);
        assert_eq!(s.get(View::Island), Some(DOCK_SIZE), "opakovaně beze změny");
        s.set_mode(HoverMode::Island);
        assert_eq!(s.get(View::Island), Some(ISLAND_SIZE));
        // skutečná velikost z frontendu se přepnutím režimu nepřepíše
        s.set(View::Island, 600.0, 120.0);
        s.set_mode(HoverMode::Dock);
        assert_eq!(s.get(View::Island), Some((600.0, 120.0)));
        assert_eq!(s.get(View::Mid), Some(MID_SIZE));
    }

    #[test]
    fn closes_table() {
        use crate::drawer::Opener::*;
        use CloseCause::*;
        // (opener, příčina, zavřít?)
        let cases = [
            (Hover, Blur, false), (Hover, MouseLeave, true), (Hover, Fullscreen, true), (Hover, GameMode, true),
            (Hotkey, Blur, true), (Hotkey, MouseLeave, false), (Hotkey, Fullscreen, true), (Hotkey, GameMode, true),
            (Remote, Blur, false), (Remote, MouseLeave, false), (Remote, Fullscreen, true), (Remote, GameMode, true),
            (Pinned, Blur, false), (Pinned, MouseLeave, false), (Pinned, Fullscreen, true), (Pinned, GameMode, true),
        ];
        for (o, c, want) in cases {
            assert_eq!(closes(o, c), want, "{o:?} {c:?}");
        }
    }

    #[test]
    fn note_in_dock_table() {
        use HoverMode::*;
        for (m, v, want) in [(Dock, View::Hidden, true), (Dock, View::Island, true), (Dock, View::Peek, true), (Dock, View::Full, false), (Island, View::Hidden, false), (Full, View::Hidden, false)] {
            assert_eq!(note_in_dock(m, v), want, "{m:?} {v:?}");
        }
    }

    #[test]
    fn dock_live_and_keys_table() {
        use HoverMode::*;
        // (režim, pohled, hlasitost povolena, živá hlasitost)
        for (m, v, a, want) in [(Dock, View::Island, true, true), (Dock, View::Island, false, false), (Dock, View::Hidden, true, false), (Dock, View::Full, true, true), (Dock, View::Peek, true, false), (Island, View::Island, true, false), (Full, View::Island, true, false)] {
            assert_eq!(volume_wants_live(m, v, a), want, "{m:?} {v:?} {a}");
        }
        // (režim, pohled, hudba povolena, živá hudba): dok i ostrůvek, hudba nezávisle na hlasitosti (L177)
        for (m, v, a, want) in [
            (Dock, View::Island, true, true),
            (Dock, View::Island, false, false),
            (Dock, View::Full, true, true),
            (Dock, View::Hidden, true, false),
            (Dock, View::Peek, true, false),
            (Dock, View::Mid, true, false),
            (Island, View::Island, true, true),
            (Island, View::Island, false, false),
            (Island, View::Full, true, true),
            (Island, View::Hidden, true, false),
            (Island, View::Peek, true, false),
            (Full, View::Island, true, false),
            (Full, View::Full, true, false),
        ] {
            assert_eq!(music_wants_live(m, v, a), want, "{m:?} {v:?} {a}");
        }
        // (pohled, slot hudby, kurzor v okně, mrtvý, šipky)
        for (v, m, c, d, want) in [(View::Island, true, true, false, true), (View::Island, false, true, false, false), (View::Island, true, false, false, false), (View::Island, true, true, true, false), (View::Full, true, true, false, false), (View::Hidden, true, true, false, false)] {
            assert_eq!(dock_keys_wanted(v, m, c, d), want, "{v:?} {m} {c} {d}");
        }
    }

    #[test]
    fn mode_serde_and_view() {
        for (json, mode, view) in [("\"full\"", HoverMode::Full, View::Full), ("\"island\"", HoverMode::Island, View::Island), ("\"dock\"", HoverMode::Dock, View::Island)] {
            let m: HoverMode = serde_json::from_str(json).unwrap();
            assert_eq!((m, m.view()), (mode, view));
            assert_eq!(serde_json::to_string(&m).unwrap(), json);
        }
        // migrace: uložené kompaktní náhledy (kolo ≤ 33) se načtou jako dok (střední panel s kartou Claude), uloží se jako `dock`
        let old: HoverMode = serde_json::from_str("\"compact\"").unwrap();
        assert_eq!((old, old.view()), (HoverMode::Dock, View::Island));
        assert_eq!(serde_json::to_string(&old).unwrap(), "\"dock\"");
        assert!(serde_json::from_str::<HoverMode>("\"expand\"").is_err());
        assert_eq!(HoverMode::default(), HoverMode::Full);
    }
}
