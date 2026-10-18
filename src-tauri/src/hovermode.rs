//! Co se stane po najetí myší na proužek (`drawer.hover`): rovnou celý šuplík (chování před vlnou 7),
//! kompaktní náhled (`View::Compact`), nebo roztažený ostrůvek (`View::Island`). Tady jsou jen čisté
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
    /// malý panel pod proužkem: čekající schválení, session Clauda, právě hraje, čas a limit 5h
    Compact,
    /// černá kapsle pod proužkem s jedním nejdůležitějším děním a tlačítky
    Island,
}

impl HoverMode {
    /// Pohled, který najetí otevře.
    pub fn view(self) -> View {
        match self {
            HoverMode::Full => View::Full,
            HoverMode::Compact => View::Compact,
            HoverMode::Island => View::Island,
        }
    }
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
        Some("compact") => View::Compact,
        Some("island") => View::Island,
        _ => View::Full,
    }
}

/// Velikosti malých pohledů (logické px) — výchozí, dokud frontend nenahlásí skutečný obsah.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmallSizes {
    peek: (f64, f64),
    compact: (f64, f64),
    island: (f64, f64),
}

impl Default for SmallSizes {
    fn default() -> Self {
        Self { peek: (680.0, 72.0), compact: (420.0, 220.0), island: (480.0, 64.0) }
    }
}

impl SmallSizes {
    fn slot(&mut self, view: View) -> Option<&mut (f64, f64)> {
        match view {
            View::Peek => Some(&mut self.peek),
            View::Compact => Some(&mut self.compact),
            View::Island => Some(&mut self.island),
            View::Hidden | View::Full => None,
        }
    }

    /// Velikost malého pohledu; `None` = není malý (Full / Hidden).
    pub fn get(&self, view: View) -> Option<(f64, f64)> {
        match view {
            View::Peek => Some(self.peek),
            View::Compact => Some(self.compact),
            View::Island => Some(self.island),
            View::Hidden | View::Full => None,
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

/// Malý pohled oříznutý na monitor: šířka nejvýš šířka šuplíku (`width`, už oříznutá na monitor),
/// výška nejvýš strop pracovní plochy (`max_h`). Peek se neořezává (chování před vlnou 7).
pub fn fit_small(view: View, (w, h): (f64, f64), width: f64, max_h: f64) -> (f64, f64) {
    match view {
        View::Compact | View::Island => (w.min(width.max(1.0)), h.min(max_h.max(1.0))),
        _ => (w, h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_target_table() {
        use HoverMode::*;
        // (režim, full, nad peekem) → pohled
        let cases = [
            (Full, true, false, View::Full),
            (Compact, true, false, View::Compact),
            (Island, true, false, View::Island),
            // Claude čeká → peek se schválením ve všech režimech
            (Full, false, false, View::Peek),
            (Compact, false, false, View::Peek),
            (Island, false, false, View::Peek),
            // podržení nad připomínkou → celý šuplík jako dřív
            (Compact, true, true, View::Full),
            (Island, true, true, View::Full),
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
            (View::Compact, true, false, View::Peek),
            (View::Island, true, false, View::Peek),
            (View::Full, false, false, View::Hidden),
            (View::Compact, false, false, View::Hidden),
            (View::Island, false, false, View::Hidden),
            (View::Peek, true, false, View::Hidden),
            (View::Compact, true, true, View::Hidden), // fullscreen / herní režim: nic
            (View::Hidden, true, false, View::Hidden),
        ];
        for (v, att, dead, want) in cases {
            assert_eq!(after_close(v, att, dead), want, "{v:?} attention={att} dead={dead}");
        }
    }

    #[test]
    fn covers_strip_table() {
        let cases = [(View::Hidden, false), (View::Peek, false), (View::Full, true), (View::Compact, true), (View::Island, true)];
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
        let cases = [(Some("peek"), View::Peek), (Some("compact"), View::Compact), (Some("island"), View::Island), (Some("full"), View::Full), (Some("x"), View::Full), (None, View::Full)];
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
        assert!(s.set(View::Compact, 420.0, 300.0));
        assert!(!s.set(View::Compact, 420.2, 300.3), "pod půl px = beze změny");
        assert!(!s.set(View::Full, 1.0, 1.0), "Full se tu neukládá");
        assert_eq!(s.get(View::Compact), Some((420.2, 300.3)));
        assert_eq!(s.get(View::Peek), Some((680.0, 72.0)), "jiné pohledy beze změny");
        // ořez: úzký šuplík / nízký monitor; peek se neořezává
        assert_eq!(fit_small(View::Compact, (420.0, 900.0), 360.0, 600.0), (360.0, 600.0));
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
    }

    #[test]
    fn mode_serde_and_view() {
        for (json, mode, view) in [("\"full\"", HoverMode::Full, View::Full), ("\"compact\"", HoverMode::Compact, View::Compact), ("\"island\"", HoverMode::Island, View::Island)] {
            let m: HoverMode = serde_json::from_str(json).unwrap();
            assert_eq!((m, m.view()), (mode, view));
            assert_eq!(serde_json::to_string(&m).unwrap(), json);
        }
        assert!(serde_json::from_str::<HoverMode>("\"expand\"").is_err());
        assert_eq!(HoverMode::default(), HoverMode::Full);
    }
}
