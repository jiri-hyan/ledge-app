//! Průvodce prvním spuštěním a čistý start nového uživatele.
//!
//! - **Nový uživatel** (config neexistuje a Notch nebylo co migrovat): [`fresh_config`] — žádné projekty,
//!   sync poznámek vypnutý, `onboarding.done: false` → šuplík při prvním otevření ukáže průvodce.
//! - **Existující config**: `onboarding` chybí → `done: true` (průvodce se neukáže, jde spustit ručně);
//!   [`upgrade_loaded`] dorovná starý config bez `notes.sync.remote` podle pracovní kopie poznámek
//!   a dřívější repo vydání `update.repo` převede na veřejné ([`legacy_update_repo`]).
//! - Podmoduly kroků: `repos` (sken rep), `apps` (Start menu), `claude` (detekce), `dialog` (výběr
//!   složky), `themes` (motivy ze schématu), `apply` (zápis výběru do configu). Příkazy: `commands::onboarding`.

pub mod apply;
pub mod apps;
pub mod claude;
pub mod dialog;
pub mod repos;
pub mod themes;

use crate::config::{Config, NotesSyncConfig, UpdateConfig, valid_notes_remote};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OnboardingConfig {
    /// Průvodce prvním spuštěním proběhl (nebo ho uživatel přeskočil). `false` = ukáže se při
    /// otevření šuplíku. Config bez tohoto pole (založený dřív) = `true`.
    pub done: bool,
}

impl Default for OnboardingConfig {
    fn default() -> Self {
        Self { done: true }
    }
}

/// Config pro úplně nového uživatele: žádné cizí projekty ani repa, průvodce čeká.
pub fn fresh_config() -> Config {
    Config { onboarding: OnboardingConfig { done: false }, ..Default::default() }
}

/// Úpravy po načtení existujícího souboru (`config::load`): jen doplňují to, co dřívější výchozí
/// hodnoty dávaly implicitně. Nic se nezapisuje — soubor se změní až při uložení z Nastavení.
pub fn upgrade_loaded(raw: &Value, cfg: &mut Config) {
    let root = cfg.notes.root.clone();
    let origin = || crate::notes::resolve_root(&root).ok().and_then(|r| crate::resolve::read_origin(&r));
    legacy_notes_sync(raw.pointer("/notes/sync"), &mut cfg.notes.sync, origin);
    missing_kinds_in_lang(raw, cfg);
    legacy_update_repo(&mut cfg.update);
}

/// Repo, ze kterého se dřív stahovaly aktualizace. Vydání jsou teď ve veřejném repu (výchozí `update.repo`).
const LEGACY_UPDATE_REPO: &str = "jiri-hyan/ledge";

/// `update.repo` = dřívější repo vydání (bez ohledu na velikost písmen a mezery okolo) → výchozí
/// veřejné repo. Jiné repo se nemění. Jen v paměti — soubor se změní až při uložení z Nastavení. Čistá funkce.
pub fn legacy_update_repo(update: &mut UpdateConfig) {
    if update.repo.trim().eq_ignore_ascii_case(LEGACY_UPDATE_REPO) {
        update.repo = UpdateConfig::default().repo;
    }
}

/// Chybí-li v souboru `notes.kinds`, výchozí kategorie se berou za běhu — v jazyce configu (`ui.lang`),
/// ne vždy česky. Uvedené kategorie (i výchozí české) se nemění. Čistá funkce.
pub fn missing_kinds_in_lang(raw: &Value, cfg: &mut Config) {
    if raw.pointer("/notes/kinds").is_none() {
        cfg.notes.kinds = crate::config::default_note_kinds_in(cfg.ui.lang);
    }
}

/// Dřív chybějící `notes.sync` (nebo jeho `remote`) znamenalo sync zapnutý na autorovo repo. Teď se
/// remote vezme z `origin` pracovní kopie poznámek — kde sync běžel, běží dál se stejným repem; kde
/// kopie není, zůstane vypnutý (nové výchozí). Výslovně uvedený `remote` se nikdy nemění. Čistá funkce.
pub fn legacy_notes_sync(sync: Option<&Value>, out: &mut NotesSyncConfig, origin: impl FnOnce() -> Option<String>) {
    if sync.and_then(|s| s.get("remote")).is_some() {
        return;
    }
    let enabled_explicit = sync.and_then(|s| s.get("enabled")).is_some();
    match origin().filter(|o| valid_notes_remote(o)) {
        Some(o) => {
            out.remote = o;
            if !enabled_explicit {
                out.enabled = true;
            }
        }
        // zapnutý sync bez remote by config shodil validací — radši vypnout (stav uvidí Nastavení)
        None => out.enabled = false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, ProjectKind};

    const AUTHOR_CFG: &str = include_str!("../fixtures/config-existing-user.json");

    #[test]
    fn new_user_config_is_clean() {
        let cfg = fresh_config();
        assert!(cfg.projects.is_empty());
        assert!(!cfg.onboarding.done);
        assert!(!cfg.notes.sync.enabled && cfg.notes.sync.remote.is_empty());
        assert_eq!(cfg.update.repo, "jiri-hyan/ledge-app");
        config::validate(&cfg).unwrap();
        let json = serde_json::to_string(&cfg).unwrap().to_lowercase();
        for author in ["acme", "someone", "notes.git"] {
            assert!(!json.contains(author), "{author} v čistém configu");
        }
        // uloží se a načte beze změny (průvodce čeká i po restartu)
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        config::save(&path, &cfg).unwrap();
        assert_eq!(config::load(&path).unwrap().unwrap(), cfg);
    }

    #[test]
    fn missing_kinds_follow_config_language() {
        let labels = |c: &Config| c.notes.kinds.iter().map(|k| k.label.clone()).collect::<Vec<_>>();
        let load = |text: &str| {
            let raw: Value = serde_json::from_str(text).unwrap();
            let mut cfg = Config::deserialize(&raw).unwrap();
            missing_kinds_in_lang(&raw, &mut cfg);
            cfg
        };
        assert_eq!(labels(&load(r#"{"ui":{"lang":"en"}}"#))[0], "Idea", "chybí kinds → jazyk configu");
        assert_eq!(labels(&load(r#"{"ui":{"lang":"cs"}}"#))[0], "Nápad");
        let given = r#"{"ui":{"lang":"en"},"notes":{"kinds":[{"id":"napad","label":"Nápad"}]}}"#;
        assert_eq!(labels(&load(given)), ["Nápad"], "uvedené kategorie se nemění");
    }

    #[test]
    fn existing_config_keeps_behaviour() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, AUTHOR_CFG).unwrap();
        let cfg = config::load(&path).unwrap().unwrap();
        assert!(cfg.onboarding.done, "existující config průvodce neukáže");
        assert_eq!(cfg.ui.lang, crate::uiconf::Lang::Cs);
        assert!(cfg.notes.sync.enabled && cfg.notes.sync.remote == "https://github.com/acme/notes.git");
        assert_eq!(cfg.projects.len(), 2);
        assert_eq!(cfg.projects[0].kind, ProjectKind::Dev);
        // round-trip: uložení a nové načtení = totéž chování
        config::save(&path, &cfg).unwrap();
        assert_eq!(config::load(&path).unwrap().unwrap(), cfg);
    }

    #[test]
    fn legacy_update_repo_table() {
        let run = |raw: &str| {
            let v: Value = serde_json::from_str(raw).unwrap();
            let mut cfg = Config::deserialize(&v).unwrap();
            legacy_update_repo(&mut cfg.update);
            cfg.update.repo
        };
        #[rustfmt::skip]
        let cases: &[(&str, &str)] = &[
            // dřívější repo vydání → veřejné
            (r#"{"update":{"repo":"jiri-hyan/ledge"}}"#,       "jiri-hyan/ledge-app"),
            (r#"{"update":{"repo":"  Jiri-Hyan/LEDGE \t"}}"#,  "jiri-hyan/ledge-app"),
            // chybějící blok update / repo = výchozí
            (r#"{}"#,                                           "jiri-hyan/ledge-app"),
            (r#"{"update":{"intervalH":12}}"#,                  "jiri-hyan/ledge-app"),
            // jiné repo zůstane beze změny (i podobné jméno)
            (r#"{"update":{"repo":"jiri-hyan/ledge-app"}}"#,   "jiri-hyan/ledge-app"),
            (r#"{"update":{"repo":"acme/ledge"}}"#,             "acme/ledge"),
            (r#"{"update":{"repo":"jiri-hyan/ledge-fork"}}"#,  "jiri-hyan/ledge-fork"),
            (r#"{"update":{"repo":"jiri-hyan/ledgex"}}"#,      "jiri-hyan/ledgex"),
        ];
        for (raw, want) in cases {
            assert_eq!(run(raw), *want, "{raw}");
        }
    }

    #[test]
    fn load_moves_legacy_update_repo_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let notes = dir.path().join("notes");
        let raw = serde_json::json!({ "notes": { "root": notes.to_string_lossy() }, "update": { "repo": " JIRI-HYAN/ledge " } });
        std::fs::write(&path, raw.to_string()).unwrap();
        let cfg = config::load(&path).unwrap().unwrap();
        assert_eq!(cfg.update.repo, "jiri-hyan/ledge-app");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw.to_string(), "soubor se při načtení nepřepisuje");
        config::save(&path, &cfg).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("\"jiri-hyan/ledge-app\""));
    }

    #[test]
    fn legacy_notes_sync_table() {
        let origin = "https://github.com/acme/notes.git";
        let run = |raw: &str, origin: Option<&str>| {
            let v: Value = serde_json::from_str(raw).unwrap();
            let mut cfg = Config::deserialize(&v).unwrap();
            legacy_notes_sync(v.pointer("/notes/sync"), &mut cfg.notes.sync, || origin.map(String::from));
            (cfg.notes.sync.enabled, cfg.notes.sync.remote)
        };
        #[rustfmt::skip]
        let cases: Vec<(&str, Option<&str>, (bool, &str))> = vec![
            // starý config bez notes.sync, kořen je pracovní kopie → sync běží dál se stejným repem
            (r#"{}"#,                                              Some(origin), (true, origin)),
            (r#"{"notes":{"enabled":true}}"#,                      Some(origin), (true, origin)),
            // kopie poznámek není → vypnuto (nové výchozí)
            (r#"{}"#,                                              None,         (false, "")),
            // výslovně vypnutý, bez remote → zůstane vypnutý, remote se doplní
            (r#"{"notes":{"sync":{"enabled":false}}}"#,            Some(origin), (false, origin)),
            // zapnutý bez remote a bez kopie → vypnout (jinak nevalidní config)
            (r#"{"notes":{"sync":{"enabled":true}}}"#,             None,         (false, "")),
            // výslovný remote se nikdy nemění
            (r#"{"notes":{"sync":{"enabled":true,"remote":"git@github.com:x/y.git"}}}"#, Some(origin), (true, "git@github.com:x/y.git")),
            (r#"{"notes":{"sync":{"enabled":false,"remote":""}}}"#, Some(origin), (false, "")),
            // nesmyslný origin se nepřebírá
            (r#"{}"#,                                              Some("http://x/y"), (false, "")),
        ];
        for (raw, o, (en, remote)) in cases {
            assert_eq!(run(raw, o), (en, remote.to_string()), "{raw} / {o:?}");
        }
    }

    #[test]
    fn load_reads_origin_of_notes_working_copy() {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(notes.join(".git")).unwrap();
        std::fs::write(notes.join(".git").join("config"), "[remote \"origin\"]\n\turl = https://github.com/acme/notes.git\n").unwrap();
        let path = dir.path().join("config.json");
        let raw = serde_json::json!({ "notes": { "root": notes.to_string_lossy() } });
        std::fs::write(&path, raw.to_string()).unwrap();
        let cfg = config::load(&path).unwrap().unwrap();
        assert!(cfg.notes.sync.enabled);
        assert_eq!(cfg.notes.sync.remote, "https://github.com/acme/notes.git");
    }
}
