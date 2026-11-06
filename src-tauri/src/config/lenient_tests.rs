//! Načtení po sekcích, zálohy `.broken-*`, obnova a rozsahy `claude.*` (kolo 29). Vše v dočasných
//! složkách, nikdy v `%APPDATA%\Ledge`.

use super::recovery::{self, KEEP_BROKEN};
use super::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const V02: &str = include_str!("../fixtures/config-versions/v0.2-reconstructed.json");
const V09: &str = include_str!("../fixtures/config-versions/v0.9-reconstructed.json");
const EXISTING: &str = include_str!("../fixtures/config-existing-user.json");

fn broken(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.file_name().unwrap().to_string_lossy().contains(".broken-")).collect();
    v.sort();
    v
}

fn ids(cfg: &Config) -> Vec<String> {
    cfg.projects.iter().map(|p| p.id.clone()).collect()
}

/// Config z každé éry (rekonstruované z tvaru struktur ve verzích, bez skutečných jmen) se načte
/// přísně, bez vadných sekcí, nic z uživatelských dat nezmizí a uložení → načtení je stabilní.
#[test]
fn versioned_fixtures_load_without_loss() {
    let default_with_projects = {
        let c = Config {
            projects: serde_json::from_value(json!([{ "id": "app", "name": "App", "kind": "dev", "run": "npm run dev", "remote": "example/app" }])).unwrap(),
            ..Config::default()
        };
        serde_json::to_string_pretty(&c).unwrap()
    };
    let cases: [(&str, &str, &[&str]); 4] = [
        ("v0.2", V02, &["app", "app-v2"]),
        ("v0.9", V09, &["app"]),
        ("existující uživatel", EXISTING, &["acme-app", "acme-studio"]),
        ("aktuální výchozí + projekt", &default_with_projects, &["app"]),
    ];
    for (what, text, want_ids) in cases {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, text).unwrap();
        let strict = load(&path).unwrap_or_else(|e| panic!("{what}: {e}")).unwrap();
        let part = load_lenient(&path).unwrap().unwrap();
        assert!(part.bad.is_empty(), "{what}");
        assert_eq!(part.cfg, strict, "{what}");
        assert_eq!(ids(&strict), want_ids, "{what}");
        // uložení → načtení = totéž (nic se při round-tripu neztratí ani nezmění)
        save(&path, &strict).unwrap();
        assert_eq!(load(&path).unwrap().unwrap(), strict, "{what}: round-trip");
        assert!(broken(dir.path()).is_empty(), "{what}: platný config se nezálohuje jako rozbitý");
    }
    // konkrétní data ze starých verzí
    let v: Value = serde_json::to_value(lenient_from_value(&serde_json::from_str(V02).unwrap()).unwrap().cfg).unwrap();
    assert_eq!(v.pointer("/personal/weather/label"), Some(&json!("Praha")));
    assert_eq!(v.pointer("/hotkeys/toggle"), Some(&json!("Ctrl+Alt+L")));
    assert_eq!(v.pointer("/update/repo"), Some(&json!("example/app")));
    let c02 = lenient_from_value(&serde_json::from_str(V02).unwrap()).unwrap().cfg;
    assert!(c02.actions.iter().any(|a| a.target.contains("example.com/mapa")) || c02.personal.links.iter().any(|l| l.target.contains("example.com/mapa")), "odkaz z v0.2 nesmí zmizet");
    let c09 = lenient_from_value(&serde_json::from_str(V09).unwrap()).unwrap().cfg;
    assert!(c09.actions.iter().any(|a| a.id == "mapy") && c09.update.interval_h == 12 && !c09.autostart);
}

/// Jedna vadná sekce → ostatní zůstanou, vadná je výchozí a ví se která.
#[test]
fn one_bad_section_keeps_the_rest() {
    let good: Value = serde_json::from_str(V09).unwrap();
    let cases: [(&str, &str, Value); 4] = [
        ("hodnota mimo rozsah", "strip", json!({ "opacity": 7 })),
        ("špatný typ", "hotkeys", json!({ "enabled": "ano" })),
        ("duplicitní id", "projects", json!([{ "id": "a", "name": "A", "kind": "dev", "run": "x" }, { "id": "a", "name": "B", "kind": "dev", "run": "x" }])),
        ("neplatný remote poznámek", "notes", json!({ "sync": { "enabled": true, "remote": "--upload-pack=x" } })),
    ];
    for (what, key, bad_val) in cases {
        let mut raw = good.clone();
        raw[key] = bad_val;
        assert!(build_err(&raw), "{what}: přísné načtení musí selhat");
        let part = lenient_from_value(&raw).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(part.bad.iter().map(|b| b.key.as_str()).collect::<Vec<_>>(), [key], "{what}");
        assert!(!part.bad[0].error.is_empty());
        let def = serde_json::to_value(Config::default()).unwrap();
        let got = serde_json::to_value(&part.cfg).unwrap();
        assert_eq!(got.get(key), def.get(key), "{what}: vadná sekce = výchozí");
        // ostatní sekce zůstaly
        for other in ["update", "actions", "claude", "gitAlerts"] {
            assert_eq!(got.get(other), serde_json::to_value(lenient_from_value(&good).unwrap().cfg).unwrap().get(other), "{what}: {other}");
        }
        if key != "projects" {
            assert_eq!(ids(&part.cfg), ["app"], "{what}");
        }
        assert!(recovery::sections_message(&part.bad).unwrap().contains(key), "{what}");
    }
    // úplně rozbité: bez záchrany
    assert!(lenient_from_value(&json!([1, 2])).is_err());
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.json");
    std::fs::write(&p, r#"{"projects":[{"id":"moje""#).unwrap();
    assert!(load_lenient(&p).is_err());
}

fn build_err(raw: &Value) -> bool {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("c.json");
    std::fs::write(&p, raw.to_string()).unwrap();
    load(&p).is_err()
}

/// Vadná sekce → zápis z Nastavení: původní soubor je v záloze (bajt po bajtu) PŘED přepsáním, na disku
/// je pak platný config s nedotčenými ostatními sekcemi a výchozí vadnou sekcí; druhý zápis už nezálohuje.
#[test]
fn bad_section_then_settings_save_keeps_original_in_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut raw: Value = serde_json::from_str(V09).unwrap();
    raw["strip"] = json!({ "opacity": 7 });
    let original = serde_json::to_string_pretty(&raw).unwrap();
    std::fs::write(&path, &original).unwrap();
    let part = load_lenient(&path).unwrap().unwrap();
    assert_eq!(part.bad.len(), 1);
    // Ledge nic nezapsal jen proto, že načetl
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    assert!(broken(dir.path()).is_empty());
    // uložení z Nastavení (v paměti je částečný config)
    let bak = save_with(&path, &part.cfg, &|a, b| std::fs::copy(a, b).map(|_| ())).unwrap().expect("záloha musí vzniknout");
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), original, "záloha = původní bajty");
    assert_eq!(broken(dir.path()), [bak]);
    let now = load(&path).unwrap().unwrap();
    assert_eq!(now, part.cfg);
    assert_eq!(ids(&now), ["app"], "ostatní sekce přežily");
    assert_eq!(serde_json::to_value(&now.strip).unwrap(), serde_json::to_value(Config::default().strip).unwrap(), "vadná sekce = výchozí");
    // znovu: disk je platný, žádná další záloha
    assert!(save_with(&path, &now, &|a, b| std::fs::copy(a, b).map(|_| ())).unwrap().is_none());
    assert_eq!(broken(dir.path()).len(), 1);
}

/// „Bez zálohy zápis odmítne“ platí i pro částečné načtení: selže-li kopie, soubor zůstane beze změny.
#[test]
fn save_is_refused_when_the_backup_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut raw: Value = serde_json::from_str(V09).unwrap();
    raw["strip"] = json!({ "opacity": 7 });
    let original = raw.to_string();
    std::fs::write(&path, &original).unwrap();
    let part = load_lenient(&path).unwrap().unwrap();
    let fail = |_: &Path, _: &Path| Err(std::io::Error::other("disk plný"));
    let err = save_with(&path, &part.cfg, &fail).unwrap_err();
    assert!(err.contains("nejde zazálohovat") || err.contains("can't be backed up"), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original, "původní soubor nedotčen");
    assert!(broken(dir.path()).is_empty());
    assert!(!dir.path().join("config.json.1").exists(), "ani rotace záloh se nespustila");
}

/// Limit kopií: po vytvoření nové se nejstarší mažou, nová i nejnovější zůstanou.
#[test]
fn broken_copies_are_capped_and_newest_survives() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    for i in 1..=8 {
        std::fs::write(dir.path().join(format!("config.json.broken-{i}")), format!("stará {i}")).unwrap();
    }
    std::fs::write(&path, "{ rozbité").unwrap();
    let new = crate::configio::keep_unreadable(&path).unwrap().unwrap();
    let left = broken(dir.path());
    assert_eq!(left.len(), KEEP_BROKEN);
    assert!(left.contains(&new), "nová kopie zůstala");
    assert!(dir.path().join("config.json.broken-8").exists(), "nejnovější stará zůstala");
    assert!(!dir.path().join("config.json.broken-1").exists() && !dir.path().join("config.json.broken-4").exists());
    assert_eq!(std::fs::read_to_string(&new).unwrap(), "{ rozbité");
    // selhání nové kopie nesmí smazat nic
    std::fs::write(&path, "{ znovu rozbité").unwrap();
    let before = broken(dir.path());
    assert!(crate::configio::keep_unreadable_with(&path, &|_, _| Err(std::io::Error::other("x"))).is_err());
    assert_eq!(broken(dir.path()), before);
}

/// Obnova: soubor se vrátí z kopie, současný zůstane v `.1`, kopie zůstává.
#[test]
fn restore_brings_the_original_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut raw: Value = serde_json::from_str(V09).unwrap();
    raw["strip"] = json!({ "opacity": 7 });
    let original = serde_json::to_string_pretty(&raw).unwrap();
    std::fs::write(&path, &original).unwrap();
    let part = load_lenient(&path).unwrap().unwrap();
    let bak = save_with(&path, &part.cfg, &|a, b| std::fs::copy(a, b).map(|_| ())).unwrap().unwrap();
    let saved = std::fs::read(&path).unwrap();
    let back = restore_backup(&path, &bak).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    assert_eq!(back.bad.len(), 1, "vadná sekce se ukáže znovu");
    assert_eq!(std::fs::read(dir.path().join("config.json.1")).unwrap(), saved, "současný soubor v rotaci");
    assert!(bak.exists());
    // chybějící / nečitelná kopie = chyba, soubor beze změny
    assert!(restore_backup(&path, &dir.path().join("config.json.broken-1")).is_err());
    std::fs::write(dir.path().join("config.json.broken-2"), "{ nic").unwrap();
    assert!(restore_backup(&path, &dir.path().join("config.json.broken-2")).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

/// Starý config s hodnotami `claude.*` mimo rozsah se načte i uloží; varování jsou jen informace.
#[test]
fn out_of_range_claude_values_still_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, r#"{"claude":{"approvalTimeoutS":1,"questionHoldS":9999,"questionUnseenS":2,"port":0}}"#).unwrap();
    let cfg = load(&path).unwrap().unwrap();
    let w = recovery::range_warnings(&cfg);
    assert_eq!(w.iter().map(|w| w.path).collect::<Vec<_>>(), ["claude.approvalTimeoutS", "claude.questionHoldS", "claude.questionUnseenS", "claude.port"]);
    save(&path, &cfg).unwrap();
    assert_eq!(load(&path).unwrap().unwrap(), cfg, "hodnoty se při uložení nemění");
    assert!(broken(dir.path()).is_empty());
}
