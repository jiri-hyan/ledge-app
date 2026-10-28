//! Karta „Rychlé dlaždice Windows“ (kolo 23, id `winquick`; jen v katalogu karet): čistá logika bez Windows API —
//! config `winquick`, výběr a pořadí dlaždic, příkazy terminálu, výběr nejnovějšího snímku, motiv Windows přes
//! `ThemeStore` (testy ho nahrazují atrapou, nesahají na registr). Win API je v `winquickwin.rs`, příkazy v
//! `commands/winquick.rs`. Každá akce jen na výslovný klik; nic neběží v klidu.

use crate::configerr::{FieldError, fe};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Dlaždice, které karta umí (pořadí = výchozí podle užitku).
pub const TILE_IDS: [&str; 9] = ["mic", "out", "term", "dir", "theme", "wifi", "bt", "set", "shot"];
pub const MAX_FOLDERS: usize = 8;
const MAX_PATH_LEN: usize = 1024;

fn default_tiles() -> Vec<String> {
    TILE_IDS.iter().map(|s| s.to_string()).collect()
}

/// Sekce `winquick` v configu. Karta je opt-in; tohle jsou její volby.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WinQuickConfig {
    /// Které dlaždice karta ukazuje a v jakém pořadí (`mic`, `out`, `term`, `dir`, `theme`, `wifi`, `bt`, `set`, `shot`).
    /// Neznámé a opakované se při uložení zahodí. Výchozí všechny.
    pub tiles: Vec<String>,
    /// Povolit přepínání výstupu zvuku jedním klikem (neoficiální rozhraní Windows `IPolicyConfig`). Výchozí false:
    /// dlaždice jen otevře Nastavení zvuku.
    pub output_switch: bool,
    /// Připnuté složky dlaždice „Složky“ (absolutní cesty, nejvýš 8). Stažené, Plocha a Dokumenty jsou vždy.
    pub folders: Vec<String>,
}

impl Default for WinQuickConfig {
    fn default() -> Self {
        Self { tiles: default_tiles(), output_switch: false, folders: Vec::new() }
    }
}

/// Jen známé dlaždice, bez duplicit, v zadaném pořadí.
pub fn normalize_tiles(tiles: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tiles {
        if TILE_IDS.contains(&t.as_str()) && !out.contains(t) {
            out.push(t.clone());
        }
    }
    out
}

/// Cesta připnuté složky: absolutní, rozumně dlouhá, bez řídicích znaků.
pub fn folder_ok(p: &str) -> bool {
    !p.is_empty() && p.len() <= MAX_PATH_LEN && !p.chars().any(|c| c.is_control()) && Path::new(p).is_absolute()
}

pub fn validate(c: &WinQuickConfig) -> Result<(), FieldError> {
    if c.folders.len() > MAX_FOLDERS {
        return Err(fe("winquick.folders", trf!("winquick.folders: nejvýš {MAX_FOLDERS} složek.", "winquick.folders: at most {MAX_FOLDERS} folders.")));
    }
    if let Some(i) = c.folders.iter().position(|f| !folder_ok(f)) {
        return Err(fe(format!("winquick.folders[{i}]"), crate::i18n::tr("winquick.folders: cesta musí být absolutní.", "winquick.folders: the path must be absolute.")));
    }
    Ok(())
}

// ---------- terminál ve složce ----------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TermKind {
    Wt,
    PowerShell,
    Explorer,
}

pub fn parse_term(s: &str) -> Option<TermKind> {
    match s {
        "wt" => Some(TermKind::Wt),
        "powershell" => Some(TermKind::PowerShell),
        "explorer" => Some(TermKind::Explorer),
        _ => None,
    }
}

/// Brána příkazů: karta Rychlé dlaždice Windows je na nějaké záložce.
pub fn card_allowed(layout: &crate::layout::LayoutConfig) -> bool {
    crate::layout::placed(layout, crate::layout::WINQUICK_CARD)
}

/// Dlaždice (`theme`, …) je v seznamu zapnutých.
pub fn tile_enabled(wq: &WinQuickConfig, tile: &str) -> bool {
    wq.tiles.iter().any(|t| t == tile)
}

/// Dlaždice výstupu v Rychlých dlaždicích (`wq_output_set`) smí přepínat jen s volbou `outputSwitch`.
pub fn output_set_allowed(wq: &WinQuickConfig) -> bool {
    wq.output_switch
}

/// Jak spustit: program, argumenty, pracovní složka. Terminál je viditelný záměrně (výslovný klik).
#[derive(Debug, PartialEq)]
pub struct Launch {
    pub exe: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

pub fn launch_for(kind: TermKind, dir: &Path) -> Launch {
    let d = dir.to_string_lossy().to_string();
    match kind {
        // `;` odděluje v `wt` příkazy → v cestě jako `\;`
        TermKind::Wt => Launch { exe: "wt.exe".into(), args: vec!["-d".into(), d.replace(';', "\\;")], cwd: None },
        TermKind::PowerShell => Launch { exe: "powershell.exe".into(), args: vec!["-NoExit".into()], cwd: Some(dir.to_path_buf()) },
        TermKind::Explorer => Launch { exe: "explorer.exe".into(), args: vec![d], cwd: None },
    }
}

// ---------- poslední snímek obrazovky ----------

const IMAGE_EXT: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "gif", "webp"];

pub fn is_image(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, e)| IMAGE_EXT.contains(&e.to_ascii_lowercase().as_str()))
}

/// Nejnovější obrázek (podle mtime, při shodě podle jména) a počet obrázků ve složce.
pub fn newest_image(entries: &[(String, u64)]) -> (Option<usize>, usize) {
    let imgs: Vec<usize> = (0..entries.len()).filter(|&i| is_image(&entries[i].0)).collect();
    let best = imgs.iter().copied().max_by(|&a, &b| entries[a].1.cmp(&entries[b].1).then_with(|| entries[a].0.cmp(&entries[b].0)));
    (best, imgs.len())
}

/// Odkaz pro chat Claude Code: `@cesta`; cesta s mezerou jde do uvozovek.
pub fn claude_ref(path: &str) -> String {
    if path.contains(char::is_whitespace) { format!("@\"{path}\"") } else { format!("@{path}") }
}

// ---------- motiv Windows ----------

/// Úložiště motivu (registr `Themes\Personalize`); testy ho nahrazují atrapou.
pub trait ThemeStore {
    /// `AppsUseLightTheme`; `None` = hodnota chybí.
    fn apps_light(&self) -> Option<u32>;
    /// Zapíše obě hodnoty (`AppsUseLightTheme`, `SystemUsesLightTheme`) a rozešle změnu.
    fn set(&mut self, light: bool) -> Result<(), String>;
}

/// Aktuální motiv: `true` = světlý. Chybějící hodnota = světlý (výchozí stav Windows).
pub fn is_light(store: &dyn ThemeStore) -> bool {
    store.apps_light().unwrap_or(1) != 0
}

/// Přepne a vrátí nový stav (`true` = světlý).
pub fn toggle_theme(store: &mut dyn ThemeStore) -> Result<bool, String> {
    let to_light = !is_light(store);
    store.set(to_light)?;
    Ok(to_light)
}

// ---------- rádia (Wi-Fi / Bluetooth) ----------

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RadioKind {
    Wifi,
    Bluetooth,
}

pub fn parse_radio(s: &str) -> Option<RadioKind> {
    match s {
        "wifi" => Some(RadioKind::Wifi),
        "bt" => Some(RadioKind::Bluetooth),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn command_gates_table() {
        let layout = |cards: &[&str]| -> crate::layout::LayoutConfig {
            let cs: Vec<String> = cards.iter().map(|c| format!(r#"{{"id":"{c}"}}"#)).collect();
            serde_json::from_str(&format!(r#"{{"pages":[{{"id":"prace","name":"","cards":[{}]}}]}}"#, cs.join(","))).unwrap()
        };
        assert!(card_allowed(&layout(&[crate::layout::WINQUICK_CARD])));
        assert!(!card_allowed(&layout(&["notes"])));
        assert!(!card_allowed(&layout(&[])));
        let mut wq = WinQuickConfig::default();
        assert!(tile_enabled(&wq, "theme"));
        wq.tiles = s(&["wifi"]);
        assert!(!tile_enabled(&wq, "theme"));
        assert!(!tile_enabled(&wq, ""));
        assert!(!output_set_allowed(&WinQuickConfig::default()), "výchozí vypnuto");
        wq.output_switch = true;
        assert!(output_set_allowed(&wq));
    }

    #[test]
    fn wt_escapes_semicolon() {
        let l = launch_for(TermKind::Wt, Path::new(r"C:\Work;b"));
        assert_eq!(l.args, s(&["-d", r"C:\Work\;b"]));
        assert_eq!(launch_for(TermKind::Explorer, Path::new(r"C:;b")).args, s(&[r"C:;b"]), "jen wt");
    }

    #[test]
    fn old_config_without_section_keeps_defaults() {
        let c: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert_eq!(c.winquick, WinQuickConfig::default());
        assert!(!c.winquick.output_switch, "dlaždice výstupu (wq_output_set) je výchozí vypnutá");
        assert_eq!(c.winquick.tiles.len(), TILE_IDS.len());
    }

    #[test]
    fn tiles_table() {
        let cases: &[(&[&str], &[&str])] = &[
            (&["out", "mic"], &["out", "mic"]),
            (&["mic", "mic", "x", "shot"], &["mic", "shot"]),
            (&["", "MIC"], &[]),
            (&[], &[]),
        ];
        for (inp, want) in cases {
            assert_eq!(normalize_tiles(&s(inp)), s(want), "{inp:?}");
        }
    }

    #[test]
    fn folders_validation() {
        let ok = WinQuickConfig { folders: s(&[r"C:\Work\app"]), ..Default::default() };
        assert!(validate(&ok).is_ok());
        for bad in ["", "relativni", r"..\app", "C:\\a\nb"] {
            let c = WinQuickConfig { folders: s(&[bad]), ..Default::default() };
            assert!(validate(&c).is_err(), "{bad:?}");
        }
        let many = WinQuickConfig { folders: (0..9).map(|i| format!(r"C:\app{i}")).collect(), ..Default::default() };
        assert!(validate(&many).is_err());
    }

    #[test]
    fn launch_table() {
        let d = Path::new(r"C:\Work\app v2");
        assert_eq!(launch_for(TermKind::Wt, d), Launch { exe: "wt.exe".into(), args: s(&["-d", r"C:\Work\app v2"]), cwd: None });
        let ps = launch_for(TermKind::PowerShell, d);
        assert_eq!((ps.exe.as_str(), ps.cwd.as_deref()), ("powershell.exe", Some(d)));
        assert_eq!(launch_for(TermKind::Explorer, d).args, s(&[r"C:\Work\app v2"]));
        assert_eq!(parse_term("wt"), Some(TermKind::Wt));
        assert_eq!(parse_term("cmd /c calc"), None);
    }

    #[test]
    fn newest_image_table() {
        let e = |v: &[(&str, u64)]| v.iter().map(|(n, t)| (n.to_string(), *t)).collect::<Vec<_>>();
        assert_eq!(newest_image(&[]), (None, 0));
        assert_eq!(newest_image(&e(&[("a.txt", 9)])), (None, 0));
        let l = e(&[("a.png", 5), ("b.TXT", 99), ("c.JPG", 7), ("d.png", 7)]);
        assert_eq!(newest_image(&l), (Some(3), 3), "shoda času = podle jména");
    }

    #[test]
    fn claude_ref_table() {
        assert_eq!(claude_ref(r"C:\Snimky\a.png"), r"@C:\Snimky\a.png");
        assert_eq!(claude_ref(r"C:\Snimky\Snimek 1.png"), "@\"C:\\Snimky\\Snimek 1.png\"");
    }

    struct Fake {
        light: Option<u32>,
        fail: bool,
        writes: Vec<bool>,
    }
    impl ThemeStore for Fake {
        fn apps_light(&self) -> Option<u32> {
            self.light
        }
        fn set(&mut self, light: bool) -> Result<(), String> {
            if self.fail {
                return Err("zamčeno".into());
            }
            self.writes.push(light);
            self.light = Some(light as u32);
            Ok(())
        }
    }

    #[test]
    fn theme_toggle_with_fake_store() {
        let mut f = Fake { light: Some(1), fail: false, writes: vec![] };
        assert_eq!(toggle_theme(&mut f), Ok(false));
        assert_eq!(toggle_theme(&mut f), Ok(true));
        assert_eq!(f.writes, vec![false, true]);
        let mut missing = Fake { light: None, fail: false, writes: vec![] };
        assert_eq!(toggle_theme(&mut missing), Ok(false), "bez hodnoty = světlý → přepne na tmavý");
        let mut bad = Fake { light: Some(0), fail: true, writes: vec![] };
        assert!(toggle_theme(&mut bad).is_err());
    }

    #[test]
    fn radio_parse() {
        assert_eq!(parse_radio("wifi"), Some(RadioKind::Wifi));
        assert_eq!(parse_radio("bt"), Some(RadioKind::Bluetooth));
        assert_eq!(parse_radio("nfc"), None);
    }
}
