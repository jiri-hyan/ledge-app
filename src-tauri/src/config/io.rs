//! Umístění configu, načtení, atomické uložení a generované JSON schéma.

use super::recovery::SectionIssue;
use super::{Config, validate};
use crate::i18n::tr;
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
    crate::dirs::or_dot(crate::dirs::appdata()).join("Ledge")
}

/// `%APPDATA%\Notch` — složka starého produktu; čte ji jen migrace (`migrate.rs`).
pub fn legacy_config_dir() -> PathBuf {
    crate::dirs::or_dot(crate::dirs::appdata()).join("Notch")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Config);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

/// Přísná část načtení z už rozparsovaného JSON: typy, upgrady starých configů, validace.
fn build(raw: &serde_json::Value) -> Result<Config, String> {
    let mut cfg = Config::deserialize(raw).map_err(|e| crate::errors::json(tr("Nastavení (config.json)", "Settings (config.json)"), &e))?;
    crate::onboarding::upgrade_loaded(raw, &mut cfg);
    // dřívější `drawer.animationMs` → `appearance.motion` (jedno místo pro pohyb)
    crate::appearance::motion::upgrade_loaded(raw, &mut cfg);
    // nová položka proužku `waiting` i u stávajícího configu (jednou; odebraná zůstane odebraná)
    crate::attention::upgrade_items(&mut cfg.strip);
    // bez `layout`: rozložení ze `drawer.sections` + `appearance.cards`; pak odvozená pole z rozložení
    crate::layout::upgrade_loaded(raw, &mut cfg);
    crate::layout::derive(&mut cfg);
    // `personal.links` → akce (odkaz = akce, která něco otevře); co neprojde, zůstane odkazem
    crate::linkmigrate::migrate(&mut cfg);
    validate(&cfg)?;
    Ok(cfg)
}

fn read_raw(path: &Path) -> Result<Option<serde_json::Value>, String> {
    // s opakováním: přechodný zámek souboru nesmí skončit startem na výchozích a pozdějším přepsáním configu
    let read = crate::fsx::read_retry(path)
        .and_then(|b| String::from_utf8(b).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)));
    let text = match read {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(crate::errors::io(&trf!("Nejde přečíst {}", "Can't read {}", path.display()), &e)),
    };
    let text = text.trim_start_matches('\u{feff}');
    serde_json::from_str(text).map(Some).map_err(|e| crate::errors::json(tr("Nastavení (config.json)", "Settings (config.json)"), &e))
}

/// Načte config. `Ok(None)` = soubor neexistuje. Přísně: jakákoli chyba = `Err` (podle toho `save`
/// pozná, že původní soubor je třeba zazálohovat). Při startu používej `load_lenient`.
pub fn load(path: &Path) -> Result<Option<Config>, String> {
    read_raw(path)?.map(|raw| build(&raw)).transpose()
}

/// Config načtený po sekcích: `cfg` má vadné sekce nahrazené výchozími, `bad` říká které a proč.
pub struct Partial {
    pub cfg: Config,
    pub bad: Vec<SectionIssue>,
}

/// Jako `load`, ale jedna vadná top-level sekce nezahodí celý config: když přísné načtení selže,
/// přidává se sekce po sekci (v pořadí souboru) k prázdnému základu a ta, po které by načtení selhalo,
/// se vynechá (= výchozí). Soubor na disku se nemění; `save` ho před přepsáním zazálohuje, protože
/// přísné `load` pořád selhává. Nejde-li ani to (rozbitý JSON, kořen není objekt), je to `Err` jako dřív.
pub fn load_lenient(path: &Path) -> Result<Option<Partial>, String> {
    read_raw(path)?.map(|raw| lenient_from_value(&raw)).transpose()
}

/// Čistá část `load_lenient` (tabulkové testy bez disku).
pub fn lenient_from_value(raw: &serde_json::Value) -> Result<Partial, String> {
    let first = match build(raw) {
        Ok(cfg) => return Ok(Partial { cfg, bad: Vec::new() }),
        Err(e) => e,
    };
    let Some(obj) = raw.as_object() else { return Err(first) };
    let mut good = serde_json::Map::new();
    let mut bad = Vec::new();
    for (k, v) in obj {
        let mut cand = good.clone();
        cand.insert(k.clone(), v.clone());
        match build(&serde_json::Value::Object(cand.clone())) {
            Ok(_) => good = cand,
            Err(error) => bad.push(SectionIssue { key: k.clone(), error }),
        }
    }
    // vadná sekce = výchozí hodnota té sekce (klíč v souboru zůstane „přítomný“, ať si starší heuristiky
    // — chybí `layout` / `strip.items` — nemyslí, že jde o starý soubor)
    let defaults = serde_json::to_value(Config::default()).ok();
    let mut filled = good.clone();
    for b in &bad {
        if let Some(v) = defaults.as_ref().and_then(|d| d.get(&b.key)) {
            filled.insert(b.key.clone(), v.clone());
        }
    }
    match build(&serde_json::Value::Object(filled)).or_else(|_| build(&serde_json::Value::Object(good))) {
        Ok(cfg) if !bad.is_empty() => Ok(Partial { cfg, bad }),
        _ => Err(first),
    }
}

/// Atomický zápis: tmp soubor + rename, ať watcher ani šéf nikdy neuvidí půlku souboru. Zápisy jsou
/// sériové (Nastavení i průvodce běží na různých vláknech) a každý má vlastní tmp. Nečitelný
/// soubor v cíli (rozbitý JSON, vadná sekce — v paměti jsou pak výchozí hodnoty) se nejdřív zazálohuje
/// (`configio::keep_unreadable`), takže uložení z Nastavení nikdy potichu nesmaže šéfovy projekty;
/// cestu kopie dostane uživatel jako hlášku s tlačítkem Obnovit (`recovery::set_backup`).
pub fn save(path: &Path, cfg: &Config) -> Result<(), String> {
    let bak = save_with(path, cfg, &|from, to| std::fs::copy(from, to).map(|_| ()))?;
    // na disku je teď platný config: zastaralá chyba pryč, varování o rozsazích znovu
    super::recovery::set_applied(cfg);
    if bak.is_some() {
        super::recovery::set_backup(bak);
    }
    Ok(())
}

/// `save` s injektovatelným kopírováním zálohy (test: záloha selže → zápis se odmítne). Vrací cestu
/// zálohy, pokud vznikla.
pub fn save_with(path: &Path, cfg: &Config, copier: &dyn Fn(&Path, &Path) -> std::io::Result<()>) -> Result<Option<PathBuf>, String> {
    static SAVING: parking_lot::Mutex<u64> = parking_lot::Mutex::new(0);
    validate(cfg)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", dir.display()), &e))?;
    }
    // odvozená pole (`drawer.sections`) v souboru vždy odpovídají `layout` (i pro starší Ledge)
    let mut cfg = cfg.clone();
    crate::layout::derive(&mut cfg);
    let text = serde_json::to_string_pretty(&cfg).map_err(|e| crate::errors::json(tr("Nastavení nejde uložit", "Can't save the settings"), &e))?;
    let mut n = SAVING.lock();
    *n += 1;
    // původní (jen částečně čitelný) soubor MUSÍ být v záloze dřív, než se cokoli zapíše; selže-li záloha, končíme
    let bak = crate::configio::keep_unreadable_with(path, copier)?;
    crate::configio::rotate_backups(path);
    let res = crate::fsx::write_atomic(path, (text + "\n").as_bytes());
    res.map_err(|e| crate::errors::io(&trf!("{} nejde zapsat", "Can't write {}", path.display()), &e))?;
    Ok(bak)
}

/// Obnova z kopie `.broken-*`: aktuální soubor zůstane v rotaci (`.1`), kopie se zapíše atomicky a
/// vrátí se její načtení po sekcích (kopie sama zůstává). Soubor, který nejde číst vůbec, se nepřijme.
pub fn restore_backup(path: &Path, backup: &Path) -> Result<Partial, String> {
    let bytes = std::fs::read(backup).map_err(|_| super::recovery::no_backup().to_string())?;
    let raw: serde_json::Value = serde_json::from_slice(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes))
        .map_err(|e| crate::errors::json(tr("Kopie configu", "Config copy"), &e))?;
    let part = lenient_from_value(&raw)?;
    let _guard = RESTORE.lock();
    crate::configio::rotate_backups(path);
    crate::fsx::write_atomic(path, &bytes).map_err(|e| crate::errors::io(&trf!("{} nejde zapsat", "Can't write {}", path.display()), &e))?;
    Ok(part)
}

static RESTORE: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

pub fn write_schema(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join("config.schema.json"), schema_json());
}
