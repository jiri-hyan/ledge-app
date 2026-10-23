//! Umístění configu, načtení, atomické uložení a generované JSON schéma.

use super::{Config, validate};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Přepsaná složka configu z prostředí (`LEDGE_CONFIG_DIR`, starší `NOTCH_CONFIG_DIR`) — pro testy a
/// automatizaci. Když je nastavená, migrace ze starého Notche se nespouští.
pub fn config_dir_override() -> Option<PathBuf> {
    ["LEDGE_CONFIG_DIR", "NOTCH_CONFIG_DIR"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.trim().is_empty()))
        .map(PathBuf::from)
}

/// `%APPDATA%\Ledge` (config, schéma, log).
pub fn config_dir() -> PathBuf {
    if let Some(dir) = config_dir_override() {
        return dir;
    }
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    Path::new(&base).join("Ledge")
}

/// `%APPDATA%\Notch` — složka starého produktu; čte ji jen migrace (`migrate.rs`).
pub fn legacy_config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    Path::new(&base).join("Notch")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Config);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

/// Načte config. `Ok(None)` = soubor neexistuje.
pub fn load(path: &Path) -> Result<Option<Config>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(trf!("Nejde přečíst {}: {e}", "Can't read {}: {e}", path.display())),
    };
    let text = text.trim_start_matches('\u{feff}');
    let raw: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("config.json: {e}"))?;
    let mut cfg = Config::deserialize(&raw).map_err(|e| format!("config.json: {e}"))?;
    crate::onboarding::upgrade_loaded(&raw, &mut cfg);
    // dřívější `drawer.animationMs` → `appearance.motion` (jedno místo pro pohyb)
    crate::appearance::motion::upgrade_loaded(&raw, &mut cfg);
    // nová položka proužku `waiting` i u stávajícího configu (jednou; odebraná zůstane odebraná)
    crate::attention::upgrade_items(&mut cfg.strip);
    // bez `layout`: rozložení ze `drawer.sections` + `appearance.cards`; pak odvozená pole z rozložení
    crate::layout::upgrade_loaded(&raw, &mut cfg);
    crate::layout::derive(&mut cfg);
    // `personal.links` → akce (odkaz = akce, která něco otevře); co neprojde, zůstane odkazem
    crate::linkmigrate::migrate(&mut cfg);
    validate(&cfg)?;
    Ok(Some(cfg))
}


/// Atomický zápis: tmp soubor + rename, ať watcher ani šéf nikdy neuvidí půlku souboru. Zápisy jsou
/// sériové (Nastavení i průvodce běží na různých vláknech) a každý má vlastní tmp. Nečitelný
/// soubor v cíli (rozbitý JSON — v paměti jsou pak jen výchozí hodnoty) se nejdřív zazálohuje
/// (`configio::keep_unreadable`), takže uložení z Nastavení nikdy potichu nesmaže šéfovy projekty.
pub fn save(path: &Path, cfg: &Config) -> Result<(), String> {
    static SAVING: parking_lot::Mutex<u64> = parking_lot::Mutex::new(0);
    validate(cfg)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // odvozená pole (`drawer.sections`) v souboru vždy odpovídají `layout` (i pro starší Ledge)
    let mut cfg = cfg.clone();
    crate::layout::derive(&mut cfg);
    let text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    let mut n = SAVING.lock();
    *n += 1;
    crate::configio::keep_unreadable(path)?;
    let tmp = path.with_extension(format!("json.{}-{}.tmp", std::process::id(), *n));
    let res = std::fs::write(&tmp, text + "\n").and_then(|_| std::fs::rename(&tmp, path));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res.map_err(|e| trf!("{} nejde zapsat: {e}", "can't write {}: {e}", path.display()))
}

pub fn write_schema(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join("config.schema.json"), schema_json());
}
