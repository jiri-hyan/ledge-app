//! Rychlé odkazy záložky Osobní (`personal.links`, starý tvar) → rychlé akce (`actions`). Odkaz je od
//! vlny 6 jen akce, která něco otevře (web, složka, soubor; aplikace bez argumentů) — v Nastavení je
//! jeden seznam („Projekty, akce a odkazy“) a karta Odkazy ukazuje akce typu web / složka / soubor.
//!
//! Převod běží při každém načtení configu ([`crate::config::load`]) a při uložení z Nastavení
//! (`save_config` — import staré zálohy). Je **idempotentní** a **beze ztráty**:
//! - odkaz, jehož cíl už má akce viditelná na záložce Osobní, se jen odebere (je tam);
//! - jinak vznikne akce `showOn: personal` (odkazy byly na Osobní) s typem odhadnutým bez disku;
//! - odkaz, který by jako akce neprošel validací (relativní cesta, plný seznam akcí…), ZŮSTANE
//!   v `personal.links` — karta Odkazy ho ukáže dál a Nastavení ho nabídne k úpravě.
//!
//! Zápis na disk dělá až příští uložení (`config::save` je jediný zapisovač); do té doby se převod
//! při načtení zopakuje se stejným výsledkem (stejná id).

use crate::actions::{self, Action, ActionKind, MAX_ACTIONS, MAX_LABEL, ShowOn};
use crate::config::{Config, PersonalLink};
use crate::personal;
use std::path::Path;

/// Typ akce z cíle odkazu bez disku: web, `.exe`, dokument z allowlistu, jinak složka.
pub fn guess_kind(target: &str) -> ActionKind {
    let t = target.trim();
    if matches!(personal::classify(t, &[], &|_| false, &|_| false), personal::LinkTarget::Web(_)) {
        return ActionKind::Url;
    }
    let ext = Path::new(t.trim_end_matches(['.', ' '])).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "exe" => ActionKind::App,
        e if personal::FILE_ALLOW.contains(&e) => ActionKind::File,
        _ => ActionKind::Folder,
    }
}

/// Id z názvu: a–z, 0–9 a `-` (bez diakritiky), jednoznačné vůči `taken`.
fn unique_id(taken: &[String], label: &str) -> String {
    let base: String = crate::onboarding::repos::slug(label).chars().take(56).collect();
    let base = match base.trim_end_matches('-') {
        "" | "projekt" => "odkaz".to_string(),
        b => b.to_string(),
    };
    (1..)
        .map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") })
        .find(|id| !taken.iter().any(|t| t == id))
        .expect("nekonečná řada")
}

/// Akce z odkazu (ještě neověřená).
fn to_action(l: &PersonalLink, taken: &[String]) -> Action {
    let target = l.target.trim().to_string();
    let name = if l.name.trim().is_empty() { target.as_str() } else { l.name.trim() };
    let label: String = name.chars().take(MAX_LABEL).collect();
    let label = label.trim_end().to_string();
    Action { id: unique_id(taken, &label), label, kind: guess_kind(&target), target, show_on: ShowOn::Personal, ..Action::default() }
}

/// Je cíl už mezi akcemi viditelnými na záložce Osobní? (porovnání bez velikosti písmen a mezer okolo)
fn already_there(list: &[Action], target: &str) -> bool {
    let t = target.trim();
    list.iter().any(|a| a.show_on != ShowOn::Work && a.target.trim().eq_ignore_ascii_case(t))
}

/// Převede `personal.links` na akce. Vrací počet odkazů, které z `personal.links` zmizely (převedené
/// nebo už existující). Co převést nejde, zůstane v `personal.links` v původním pořadí.
pub fn migrate(cfg: &mut Config) -> usize {
    if cfg.personal.links.is_empty() {
        return 0;
    }
    let links = std::mem::take(&mut cfg.personal.links);
    let mut gone = 0;
    for l in links {
        if already_there(&cfg.actions, &l.target) {
            gone += 1;
            continue;
        }
        let taken: Vec<String> = cfg.actions.iter().map(|a| a.id.clone()).collect();
        let a = to_action(&l, &taken);
        if cfg.actions.len() < MAX_ACTIONS && actions::validate_one(&a).is_ok() {
            cfg.actions.push(a);
            gone += 1;
        } else {
            cfg.personal.links.push(l);
        }
    }
    gone
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn link(name: &str, target: &str) -> PersonalLink {
        PersonalLink { name: name.into(), target: target.into() }
    }

    #[test]
    fn kind_from_target_without_disk() {
        let cases = [
            ("https://mapy.cz", ActionKind::Url),
            ("HTTP://x.cz/a?b=1", ActionKind::Url),
            (r"C:\Users\x\Dokumenty", ActionKind::Folder),
            (r"C:\Users\x\nakup.txt", ActionKind::File),
            (r"C:\Users\x\Výpis.PDF", ActionKind::File),
            (r"C:\Hry\hra.exe", ActionKind::App),
            ("notepad.exe", ActionKind::App),
            (r"C:\x\skript.bat", ActionKind::Folder), // neznámá přípona → ověří až spuštění (jako dřív karta)
            ("  https://mezery.cz  ", ActionKind::Url),
        ];
        for (t, want) in cases {
            assert_eq!(guess_kind(t), want, "{t}");
        }
    }

    /// Skutečný tvar configu z doby před vlnou 6: odkazy v `personal.links`, pár akcí, bez `layout`.
    const OLD: &str = r#"{
      "strip": { "enabled": true, "items": ["mode", "clock", "running", "claude"] },
      "drawer": { "trigger": "hover", "hoverDelayMs": 120, "sections": { "links": true, "weather": true } },
      "modes": { "personalEnabled": true },
      "personal": {
        "weather": { "lat": 50.08, "lon": 14.42, "label": "Praha" },
        "links": [
          { "name": "Mapy", "target": "https://mapy.cz" },
          { "name": "Fotky", "target": "C:\\Users\\jiri\\Pictures" },
          { "name": "Nákup", "target": "C:\\Users\\jiri\\nakup.txt" },
          { "name": "Kalkulačka", "target": "calc.exe" },
          { "name": "Gmail", "target": "https://mail.google.com" },
          { "name": "Relativní", "target": "Dokumenty\\x" }
        ]
      },
      "actions": [
        { "id": "gmail", "label": "Pošta", "kind": "url", "target": "https://MAIL.google.com" },
        { "id": "mapy", "label": "Mapy práce", "kind": "url", "target": "https://mapy.cz", "showOn": "work" }
      ],
      "projects": []
    }"#;

    #[test]
    fn old_config_loads_with_links_as_actions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, OLD).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        let got: Vec<(String, String, ActionKind, ShowOn, String)> =
            cfg.actions.iter().map(|a| (a.id.clone(), a.label.clone(), a.kind, a.show_on, a.target.clone())).collect();
        let s = |x: &str| x.to_string();
        assert_eq!(
            got,
            vec![
                (s("gmail"), s("Pošta"), ActionKind::Url, ShowOn::Both, s("https://MAIL.google.com")),
                (s("mapy"), s("Mapy práce"), ActionKind::Url, ShowOn::Work, s("https://mapy.cz")),
                // „Mapy“ na Práci nestačí → nová akce s jiným id; Gmail už na Osobní je → nevznikne znovu
                (s("mapy-2"), s("Mapy"), ActionKind::Url, ShowOn::Personal, s("https://mapy.cz")),
                (s("fotky"), s("Fotky"), ActionKind::Folder, ShowOn::Personal, s(r"C:\Users\jiri\Pictures")),
                (s("nakup"), s("Nákup"), ActionKind::File, ShowOn::Personal, s(r"C:\Users\jiri\nakup.txt")),
                (s("kalkulacka"), s("Kalkulačka"), ActionKind::App, ShowOn::Personal, s("calc.exe")),
            ]
        );
        // relativní cesta jako akce neprojde → zůstala odkazem (nic se neztratí)
        assert_eq!(cfg.personal.links, vec![link("Relativní", r"Dokumenty\x")]);
        // ostatní beze změny
        assert_eq!(cfg.personal.weather.label, "Praha");
        assert!(crate::config::validate(&cfg).is_ok());
    }

    #[test]
    fn idempotent_and_stable() {
        let raw: serde_json::Value = serde_json::from_str(OLD).unwrap();
        let mut a: Config = serde_json::from_value(raw).unwrap();
        assert_eq!(migrate(&mut a), 5);
        let once = a.clone();
        assert_eq!(migrate(&mut a), 0);
        assert_eq!(a, once);
        // uložený a znovu načtený config: nic nového, stejná id
        let mut b: Config = serde_json::from_value(serde_json::to_value(&once).unwrap()).unwrap();
        assert_eq!(migrate(&mut b), 0);
        assert_eq!(b, once);
        // soubor ještě v původním tvaru (neuloženo) → při každém načtení stejný výsledek
        let mut c: Config = serde_json::from_str(OLD).unwrap();
        migrate(&mut c);
        assert_eq!(c, once);
    }

    #[test]
    fn nothing_to_do_and_limits() {
        let mut cfg = Config::default();
        assert_eq!(migrate(&mut cfg), 0);
        assert_eq!(cfg, Config::default());
        // plný seznam akcí: odkazy zůstanou odkazy
        cfg.actions = (0..MAX_ACTIONS).map(|i| Action { id: format!("a{i}"), label: format!("A{i}"), target: format!("https://a{i}.cz"), ..Action::default() }).collect();
        cfg.personal.links = vec![link("Mapy", "https://mapy.cz")];
        assert_eq!(migrate(&mut cfg), 0);
        assert_eq!((cfg.actions.len(), cfg.personal.links.len()), (MAX_ACTIONS, 1));
        // dlouhý název se zkrátí na popisek akce, prázdný název = cíl; id bez diakritiky
        let mut cfg = Config::default();
        cfg.personal.links = vec![link(&"Ž".repeat(200), "https://dlouhy.cz"), link("  ", "https://bez-nazvu.cz")];
        assert_eq!(migrate(&mut cfg), 2);
        assert_eq!(cfg.actions[0].label.chars().count(), MAX_LABEL);
        assert!(cfg.actions[0].id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        assert_eq!((cfg.actions[1].label.as_str(), cfg.actions[1].id.as_str()), ("https://bez-nazvu.cz", "https-bez-nazvu-cz"));
        assert!(actions::validate(&cfg.actions).is_ok());
        let v = serde_json::to_value(&cfg).unwrap();
        assert_eq!(v["personal"]["links"], json!([]));
        assert_eq!(v["actions"][0]["showOn"], json!("personal"));
    }
}
