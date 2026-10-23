//! Záloha configu z Nastavení: export do souboru, import s náhledem po sekcích (co se změní a co je
//! rizikové), výchozí hodnoty. Import NIKDY nezapisuje — vrátí ověřený config, UI z něj převezme
//! vybrané sekce do rozpracované verze a ukládá se běžně přes `save_config` (validace před zápisem).
//! Pojistka každého zápisu (`config::save`): nečitelný cíl se před přepsáním zkopíruje vedle
//! (`keep_unreadable` → `config.json.broken-<čas>`).

use crate::config::{self, Config};
use crate::i18n::tr;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

/// Sekce, jejichž import může spouštět příkazy nebo posílat data jinam — UI je ve výchozím stavu
/// nezaškrtne a ukáže proč.
const RISKY: [(&str, &str, &str); 4] = [
    ("projects", "obsahuje příkazy a programy, které se spustí kliknutím na projekt", "contains commands and programs that run when you click a project"),
    ("actions", "obsahuje akce, které můžou spouštět příkazy nebo programy", "contains actions that can run commands or programs"),
    ("notes", "mění, kam se synchronizují poznámky (git repo)", "changes where notes are synced to (git repo)"),
    ("update", "mění, odkud se stahují nové verze Ledge", "changes where new Ledge versions are downloaded from"),
];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionDiff {
    /// klíč nejvyšší úrovně configu (`drawer`, `projects`…)
    pub key: String,
    pub changed: bool,
    /// proč je převzetí rizikové (prázdné = není)
    pub risk: String,
    /// krátce co se liší: změněné podklíče, nebo počet položek seznamu
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub cfg: Value,
    pub sections: Vec<SectionDiff>,
}

/// Rozdíl dvou configů po sekcích nejvyšší úrovně (bez `$schema`): sekce importu, pak ty, které import nemá.
pub fn section_diff(cur: &Value, imp: &Value) -> Vec<SectionDiff> {
    let empty = serde_json::Map::new();
    let (c, i) = (cur.as_object().unwrap_or(&empty), imp.as_object().unwrap_or(&empty));
    let mut keys: Vec<&String> = i.keys().collect();
    keys.extend(c.keys().filter(|k| !i.contains_key(*k)));
    keys.into_iter()
        .filter(|k| k.as_str() != "$schema")
        .map(|k| {
            let (a, b) = (c.get(k).unwrap_or(&Value::Null), i.get(k).unwrap_or(&Value::Null));
            SectionDiff {
                key: k.clone(),
                changed: a != b,
                risk: RISKY.iter().find(|(r, ..)| r == k).map(|(_, cs, en)| tr(cs, en).to_string()).unwrap_or_default(),
                detail: if a == b { String::new() } else { detail(a, b) },
            }
        })
        .collect()
}

fn detail(a: &Value, b: &Value) -> String {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => trf!("{} → {} položek", "{} → {} items", x.len(), y.len()),
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&str> = y.keys().chain(x.keys()).map(String::as_str).filter(|k| x.get(*k) != y.get(*k)).collect();
            keys.sort_unstable();
            keys.dedup();
            let more = keys.len().saturating_sub(4);
            let mut s = keys.into_iter().take(4).collect::<Vec<_>>().join(", ");
            if more > 0 {
                s.push_str(&trf!(" a {more} další", " and {more} more"));
            }
            s
        }
        _ => format!("{} → {}", short(a), short(b)),
    }
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 40 { format!("{}…", s.chars().take(40).collect::<String>()) } else { s }
}

/// Načte a ověří soubor k importu (stejná validace jako config). Neplatný = chyba, nic se nemění.
pub fn read_import(path: &Path) -> Result<Config, String> {
    match config::load(path) {
        Ok(Some(c)) => Ok(c),
        Ok(None) => Err(trf!("Soubor {} neexistuje.", "File {} doesn't exist.", path.display())),
        Err(e) => Err(trf!("Soubor nejde převzít: {e}", "Can't take over the file: {e}")),
    }
}

/// Náhled importu proti aktuálnímu configu.
pub fn preview(current: &Config, path: &Path) -> Result<ImportPreview, String> {
    let imported = read_import(path)?;
    let cur = serde_json::to_value(current).map_err(|e| crate::errors::json(tr("Náhled importu", "Import preview"), &e))?;
    let cfg = serde_json::to_value(&imported).map_err(|e| crate::errors::json(tr("Náhled importu", "Import preview"), &e))?;
    let sections = section_diff(&cur, &cfg);
    Ok(ImportPreview { cfg, sections })
}

/// Export: ověřený config jako čitelný JSON (atomicky, `config::save`). Nikdy nepřepíše živý config.
pub fn export(cfg: &Config, path: &Path) -> Result<(), String> {
    let live = config::config_path();
    if same_file(path, &live) {
        return Err(tr("Zálohu ulož jinam než do živého config.json.", "Save the backup somewhere other than the live config.json.").into());
    }
    config::save(path, cfg)
}

/// Před přepsáním: cíl, který jde přečíst jen s chybou (rozbitý JSON, neplatná hodnota — Ledge pak
/// běží na výchozích hodnotách), zkopíruje vedle jako `<jméno>.broken-<unix s>` a vrátí cestu kopie.
/// Neexistující nebo platný cíl = `None`. Když kopie nejde, zápis se odmítne (nic se neztratí).
pub fn keep_unreadable(path: &Path) -> Result<Option<std::path::PathBuf>, String> {
    if !path.exists() || config::load(path).is_ok() {
        return Ok(None);
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "config.json".into());
    let bak = (1..)
        .map(|i| path.with_file_name(if i == 1 { format!("{name}.broken-{secs}") } else { format!("{name}.broken-{secs}-{i}") }))
        .find(|p| !p.exists())
        .expect("nekonečná řada");
    std::fs::copy(path, &bak).map_err(|e| {
        crate::errors::io(&trf!("{} nejde zazálohovat, nepřepisuji ho", "{} can't be backed up, not overwriting it", path.display()), &e)
    })?;
    Ok(Some(bak))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn section_diff_table() {
        let cur = json!({ "$schema": "a", "drawer": { "trigger": "hover", "width": 1080 }, "projects": [1, 2], "autostart": true, "roots": ["C:\\x"] });
        let imp = json!({ "$schema": "b", "drawer": { "trigger": "edge", "width": 1080 }, "projects": [1, 2, 3], "autostart": true, "update": { "repo": "x/y" } });
        let d = section_diff(&cur, &imp);
        let get = |k: &str| d.iter().find(|s| s.key == k).unwrap().clone();
        assert!(d.iter().all(|s| s.key != "$schema"));
        assert_eq!(get("drawer"), SectionDiff { key: "drawer".into(), changed: true, risk: String::new(), detail: "trigger".into() });
        assert_eq!(get("projects").detail, "2 → 3 položek");
        assert!(!get("projects").risk.is_empty());
        assert!(!get("autostart").changed && get("autostart").detail.is_empty());
        // sekce jen v jednom ze souborů
        assert!(get("update").changed && !get("update").risk.is_empty());
        assert!(get("roots").changed);
    }

    #[test]
    fn detail_shortens() {
        let a = json!({ "a": 1, "b": 1, "c": 1, "d": 1, "e": 1, "f": 1 });
        let b = json!({ "a": 2, "b": 2, "c": 2, "d": 2, "e": 2, "f": 2 });
        assert_eq!(detail(&a, &b), "a, b, c, d a 2 další");
        assert_eq!(detail(&json!(true), &json!(false)), "true → false");
        assert!(detail(&json!("x".repeat(80)), &json!(1)).contains('…'));
    }

    #[test]
    fn import_validates_before_anything() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, r#"{"drawer":{"trigger":"off"}}"#).unwrap();
        assert!(read_import(&bad).unwrap_err().contains("nejde převzít"));
        let broken = dir.path().join("broken.json");
        std::fs::write(&broken, "{ nejde").unwrap();
        assert!(read_import(&broken).is_err());
        assert!(read_import(&dir.path().join("chybi.json")).unwrap_err().contains("neexistuje"));
        // starý soubor bez nových polí = výchozí hodnoty
        let old = dir.path().join("old.json");
        std::fs::write(&old, r#"{"autostart":false}"#).unwrap();
        let p = preview(&Config::default(), &old).unwrap();
        assert_eq!(p.cfg["autostart"], json!(false));
        assert!(p.sections.iter().any(|s| s.key == "autostart" && s.changed));
        assert!(p.sections.iter().any(|s| s.key == "hotkeys" && !s.changed));
    }

    /// Rozbitý config na disku (v paměti jsou výchozí hodnoty) + uložení z Nastavení / průvodce =
    /// původní obsah musí zůstat ve zálohovací kopii, ne zmizet.
    #[test]
    fn save_over_unreadable_config_keeps_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cases = [
            ("prázdný", ""),
            ("rozbitý JSON", r#"{"projects":[{"id":"moje","name":"Moje""#),
            ("špatný typ", r#"{"autostart":"ano","projects":[{"id":"moje","name":"Moje","kind":"web"}]}"#),
            ("neplatná hodnota", r#"{"strip":{"opacity":7},"projects":[{"id":"moje","name":"Moje","kind":"web"}]}"#),
            ("záporné číslo", r#"{"claude":{"port":-1}}"#),
        ];
        for (what, text) in cases {
            for e in std::fs::read_dir(dir.path()).unwrap() {
                std::fs::remove_file(e.unwrap().path()).unwrap();
            }
            std::fs::write(&path, text).unwrap();
            assert!(config::load(&path).is_err(), "{what}");
            config::save(&path, &Config::default()).unwrap();
            assert_eq!(config::load(&path).unwrap().unwrap(), Config::default(), "{what}");
            let baks: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().path()).filter(|p| p != &path).collect();
            assert_eq!(baks.len(), 1, "{what}: {baks:?}");
            assert!(baks[0].file_name().unwrap().to_string_lossy().starts_with("config.json.broken-"), "{what}");
            assert_eq!(std::fs::read_to_string(&baks[0]).unwrap(), text, "{what}");
            // platný config se už nezálohuje
            config::save(&path, &Config::default()).unwrap();
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2, "{what}");
        }
    }

    #[test]
    fn save_into_read_only_file_fails_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        config::save(&path, &Config::default()).unwrap();
        let mut perm = std::fs::metadata(&path).unwrap().permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&path, perm.clone()).unwrap();
        let cfg = Config { autostart: false, ..Config::default() };
        let err = config::save(&path, &cfg).unwrap_err();
        assert!(err.contains("nejde zapsat"), "{err}");
        assert_eq!(config::load(&path).unwrap().unwrap(), Config::default(), "původní zůstal");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "žádný tmp po chybě");
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        std::fs::set_permissions(&path, perm).unwrap();
    }

    /// Souběžné zápisy (Nastavení + průvodce na jiných vláknech): soubor je vždy celý a platný.
    #[test]
    fn concurrent_saves_never_tear_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = std::sync::Arc::new(dir.path().join("config.json"));
        let hs: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for j in 0..10 {
                        let cfg = Config { roots: vec![format!("C:/r{i}-{j}"); 50], ..Config::default() };
                        config::save(&path, &cfg).unwrap();
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap();
        }
        assert!(config::load(&path).unwrap().is_some());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "žádné zbytky tmp");
    }

    #[test]
    fn export_roundtrip_and_refuses_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("zaloha.json");
        let mut cfg = Config { autostart: false, ..Config::default() };
        export(&cfg, &out).unwrap();
        assert_eq!(read_import(&out).unwrap(), cfg);
        cfg.strip.opacity = 3.0;
        assert!(export(&cfg, &dir.path().join("x.json")).is_err());
        assert!(!dir.path().join("x.json").exists());
    }
}
