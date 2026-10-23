//! Zápis výběru z průvodce do configu: projekty (bez duplicit podle složky / remote / exe, unikátní
//! id), jazyk, odkazy (vždy do `actions` — odkaz je akce typu web, `linkmigrate.rs`), motiv
//! (`appearance.theme`, jen když sekce existuje) a příznak `onboarding.done`. Čistá funkce [`apply`];
//! uložení a aplikaci dělá příkaz `onboarding_apply`.

use crate::actions::{MAX_ACTIONS, MAX_LABEL};
use crate::config::{Config, Project, validate};
use crate::i18n::tr;
use crate::onboarding::repos::{norm, slug};
use crate::uiconf::Lang;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Patch {
    pub lang: Option<Lang>,
    pub projects: Vec<Project>,
    pub links: Vec<NewLink>,
    pub theme: Option<String>,
    pub done: Option<bool>,
    /// Detekce průvodce: Claude Code je nainstalovaný. Spolu s `done` (první dokončení) přidá na Práci kartu Claude.
    pub claude: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    /// názvy přidaných projektů
    pub added: Vec<String>,
    /// názvy přeskočených projektů (už v Ledge)
    pub skipped: Vec<String>,
    pub links_added: usize,
    /// kam šly odkazy: vždy `actions` (dřív i `personal.links`; UI pole čte dál)
    pub links_to: Option<&'static str>,
    /// motiv: uložen / config sekci `appearance` nemá (`None` = motiv se neměnil)
    pub theme_saved: Option<bool>,
}

/// Tentýž projekt: stejná složka, stejný remote, nebo stejné absolutní exe.
pub fn same_project(a: &Project, b: &Project) -> bool {
    let path = |p: &Project| p.path.as_deref().map(norm);
    let remote = |p: &Project| p.remote.as_deref().and_then(crate::resolve::parse_remote);
    let exe = |p: &Project| p.exe.as_deref().filter(|e| std::path::Path::new(e).is_absolute()).map(norm);
    let eq = |x: Option<String>, y: Option<String>| x.as_deref().is_some_and(|s| !s.is_empty()) && x == y;
    eq(path(a), path(b)) || (remote(a).is_some() && remote(a) == remote(b)) || eq(exe(a), exe(b))
}

/// `base`, jinak `base-2`, `base-3`… podle id, která už existují.
pub fn unique_id(taken: &[String], base: &str) -> String {
    let base = if base.trim().is_empty() { "projekt".to_string() } else { base.to_string() };
    (1..)
        .map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") })
        .find(|id| !taken.iter().any(|t| t == id))
        .expect("nekonečná řada")
}

fn web_url(u: &str) -> bool {
    let u = u.trim();
    (u.starts_with("https://") || u.starts_with("http://")) && u.len() > 8 && !u.chars().any(char::is_whitespace)
}

pub fn apply(cfg: &Config, patch: &Patch) -> Result<(Config, Applied), String> {
    let mut c = cfg.clone();
    let mut out = Applied::default();
    for p in &patch.projects {
        if c.projects.iter().any(|e| same_project(e, p)) {
            out.skipped.push(p.name.clone());
            continue;
        }
        let taken: Vec<String> = c.projects.iter().map(|e| e.id.clone()).collect();
        let mut p = p.clone();
        p.id = unique_id(&taken, &slug(if p.id.is_empty() { &p.name } else { &p.id }));
        out.added.push(p.name.clone());
        c.projects.push(p);
    }
    if let Some(l) = patch.lang {
        c.ui.lang = l;
        // čerstvý config (průvodce poprvé, kategorie nikdo neupravil): výchozí kategorie poznámek v jazyce
        // průvodce; existující config se nemění
        if !cfg.onboarding.done && c.notes.kinds == crate::config::default_note_kinds() {
            c.notes.kinds = crate::config::default_note_kinds_in(l);
        }
    }
    if let Some(d) = patch.done {
        // první dokončení průvodce (config byl čerstvý): čisté rozložení dostane karty podle detekce — Claude,
        // když ho našel, a Repa, když jsou mezi projekty git složky. Opakované spuštění průvodce rozložení nemění.
        if d && !cfg.onboarding.done {
            crate::layout::with_detected(&mut c.layout, patch.claude, c.projects.iter().any(|p| p.path.is_some()));
        }
        c.onboarding.done = d;
    }
    let mut v = serde_json::to_value(&c).map_err(|e| crate::errors::json(tr("Výběr nejde uložit", "Can't save the selection"), &e))?;
    if !patch.links.is_empty() {
        add_links(&mut v, &patch.links, &mut out)?;
    }
    if let Some(theme) = &patch.theme {
        match v.get_mut("appearance").and_then(Value::as_object_mut) {
            Some(a) => {
                a.insert("theme".into(), json!(theme));
            }
            None => out.theme_saved = Some(false),
        }
    }
    let next = Config::deserialize(&v).map_err(|e| crate::errors::json(tr("Výběr nejde uložit", "Can't save the selection"), &e))?;
    validate(&next)?;
    if let Some(theme) = &patch.theme
        && out.theme_saved.is_none()
    {
        let back = serde_json::to_value(&next).map_err(|e| crate::errors::json(tr("Výběr nejde uložit", "Can't save the selection"), &e))?;
        out.theme_saved = Some(back.pointer("/appearance/theme") == Some(&json!(theme)));
    }
    Ok((next, out))
}

fn add_links(v: &mut Value, links: &[NewLink], out: &mut Applied) -> Result<(), String> {
    if let Some(bad) = links.iter().find(|l| !web_url(&l.url) || l.label.trim().is_empty()) {
        return Err(trf!("Odkaz „{}“ chce název a adresu http(s)://…", "Link “{}” needs a name and an http(s):// address…", bad.label));
    }
    let obj = v.as_object_mut().ok_or("config není objekt")?;
    let actions = obj.entry("actions").or_insert_with(|| json!([])).as_array_mut().ok_or("config: actions není seznam")?;
    out.links_to = Some("actions");
    for l in links {
        // plný seznam akcí by shodil validaci celého kroku
        if actions.len() >= MAX_ACTIONS {
            break;
        }
        if actions.iter().any(|a| a.get("target").and_then(Value::as_str) == Some(l.url.trim())) {
            continue;
        }
        let taken: Vec<String> = actions.iter().filter_map(|a| a.get("id").and_then(Value::as_str).map(String::from)).collect();
        // id akce má nejvýš 64 znaků (`actions::validate`); rezerva na `-2…`
        let base: String = slug(&l.label).chars().take(56).collect();
        let id = unique_id(&taken, base.trim_end_matches('-'));
        let label: String = l.label.trim().chars().take(MAX_LABEL).collect();
        actions.push(json!({ "id": id, "label": label.trim_end(), "kind": "url", "target": l.url.trim() }));
        out.links_added += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProjectKind;
    use crate::onboarding::repos::{Facts, suggest};

    fn dev(name: &str, path: &str, remote: Option<&str>) -> Project {
        let f = Facts { package: Some(json!({ "scripts": { "dev": "vite" } })), ..Default::default() };
        suggest(&f, name, path, remote).2.unwrap()
    }

    #[test]
    fn projects_merge_without_duplicates() {
        let mut cfg = crate::onboarding::fresh_config();
        cfg.projects.push(dev("Web", r"C:\r\web", Some("https://github.com/acme/web.git")));
        let patch = Patch {
            projects: vec![
                dev("Web", r"c:/r/web/", None),                     // stejná složka
                dev("Web kopie", r"D:\jinde\web", Some("acme/web")), // stejný remote
                dev("Web", r"C:\r\web2", None),                      // jiné → id web-2
                dev("Účty", r"C:\r\ucty", None),
            ],
            done: Some(true),
            lang: Some(Lang::En),
            ..Default::default()
        };
        let (next, out) = apply(&cfg, &patch).unwrap();
        assert_eq!(out.added, ["Web", "Účty"]);
        assert_eq!(out.skipped, ["Web", "Web kopie"]);
        let ids: Vec<&str> = next.projects.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["web", "web-2", "ucty"]);
        assert!(next.onboarding.done && next.ui.lang == Lang::En);
        // podruhé totéž = nic nového
        let (again, out2) = apply(&next, &patch).unwrap();
        assert_eq!((again.projects.len(), out2.added.len()), (3, 0));
    }

    #[test]
    fn wizard_language_sets_default_kinds_only_on_fresh_config() {
        let labels = |c: &Config| c.notes.kinds.iter().map(|k| k.label.clone()).collect::<Vec<_>>();
        let en = Patch { lang: Some(Lang::En), done: Some(true), ..Default::default() };
        // čerstvý config z průvodce v angličtině → anglické popisky, id stejná
        let (next, _) = apply(&crate::onboarding::fresh_config(), &en).unwrap();
        assert_eq!(labels(&next), ["Idea", "Feedback", "To work on", "Note"]);
        assert_eq!(next.notes.kinds.iter().map(|k| k.id.as_str()).collect::<Vec<_>>(), ["napad", "feedback", "ukol", "info"]);
        // existující config (průvodce už proběhl) se nemění
        let (old, _) = apply(&Config::default(), &en).unwrap();
        assert_eq!(labels(&old), ["Nápad", "Feedback", "K zapracování", "Poznámka"]);
        // upravené kategorie ani v čerstvém configu nepřepisuje
        let mut mine = crate::onboarding::fresh_config();
        mine.notes.kinds[0].label = "Moje".into();
        assert_eq!(apply(&mine, &en).unwrap().0.notes.kinds[0].label, "Moje");
    }

    #[test]
    fn apps_by_absolute_exe() {
        let app = |name: &str, exe: &str| Project { kind: ProjectKind::App, exe: Some(exe.into()), app_name: Some(name.into()), path: None, remote: None, run: None, ..dev(name, "", None) };
        let mut cfg = Config::default();
        cfg.projects.push(app("Code", r"C:\P\Code.exe"));
        let patch = Patch { projects: vec![app("VS Code", r"c:\p\code.exe"), app("Figma", r"C:\P\Figma.exe")], ..Default::default() };
        let (next, out) = apply(&cfg, &patch).unwrap();
        assert_eq!((out.added.as_slice(), next.projects.len()), (&["Figma".to_string()][..], 2));
        // relativní exe (electron) duplicitu nedělá
        let e = |n: &str| Project { exe: Some("node_modules/electron/dist/electron.exe".into()), ..app(n, "") };
        assert!(!same_project(&e("A"), &e("B")));
    }

    #[test]
    fn links_go_to_actions() {
        let links = vec![NewLink { label: "Gmail".into(), url: "https://mail.google.com".into() }, NewLink { label: "Gmail".into(), url: "https://mail.google.com".into() }];
        let (next, out) = apply(&Config::default(), &Patch { links: links.clone(), ..Default::default() }).unwrap();
        // odkaz = akce typu web; do `personal.links` už nic nejde
        assert_eq!((out.links_to, out.links_added), (Some("actions"), 1));
        assert!(next.personal.links.is_empty());
        assert_eq!((next.actions[0].label.as_str(), next.actions[0].target.as_str()), ("Gmail", "https://mail.google.com"));
        // sekce `actions` v JSON chybí (ručně psaný config) → založí se
        let mut v = json!({ "personal": { "links": [] } });
        let mut o = Applied::default();
        add_links(&mut v, &links[..1], &mut o).unwrap();
        assert_eq!(v["actions"][0]["target"], json!("https://mail.google.com"));
        assert_eq!(v["personal"]["links"], json!([]));
        for bad in ["javascript:alert(1)", "file:///C:/x", "https://", "mail.google.com"] {
            let p = Patch { links: vec![NewLink { label: "x".into(), url: bad.into() }], ..Default::default() };
            assert!(apply(&Config::default(), &p).is_err(), "{bad}");
        }
    }

    #[test]
    fn actions_shape_when_section_exists() {
        let mut v = json!({ "actions": [{ "id": "gmail", "label": "Gmail", "kind": "url", "target": "https://x.cz" }] });
        let mut out = Applied::default();
        add_links(&mut v, &[NewLink { label: "Gmail".into(), url: "https://mail.google.com".into() }, NewLink { label: "X".into(), url: "https://x.cz".into() }], &mut out).unwrap();
        assert_eq!(v["actions"][1], json!({ "id": "gmail-2", "label": "Gmail", "kind": "url", "target": "https://mail.google.com" }));
        assert_eq!((out.links_added, out.links_to), (1, Some("actions")));
    }

    /// Plné `actions` (64) nebo dlouhý název odkazu nesmí shodit celý krok průvodce validací.
    #[test]
    fn links_respect_action_limits() {
        let actions = (0..MAX_ACTIONS).map(|i| crate::actions::Action { id: format!("a{i}"), label: format!("A{i}"), target: format!("https://a{i}.cz"), ..Default::default() }).collect();
        let mut cfg = Config { actions, ..Config::default() };
        let long = NewLink { label: "Ž".repeat(200), url: "https://dlouhy.cz".into() };
        let p = Patch { links: vec![long.clone()], done: Some(true), ..Default::default() };
        let (next, out) = apply(&cfg, &p).unwrap();
        assert_eq!((next.actions.len(), out.links_added, next.onboarding.done), (MAX_ACTIONS, 0, true));
        cfg.actions.truncate(3);
        let (next, out) = apply(&cfg, &p).unwrap();
        assert_eq!((next.actions.len(), out.links_added), (4, 1));
        assert_eq!(next.actions[3].label.chars().count(), MAX_LABEL);
    }

    #[test]
    fn theme_only_when_appearance_exists() {
        let (_, out) = apply(&Config::default(), &Patch { theme: Some("dark".into()), ..Default::default() }).unwrap();
        let has = serde_json::to_value(Config::default()).unwrap().get("appearance").is_some();
        assert!(out.theme_saved.is_some());
        if !has {
            assert_eq!(out.theme_saved, Some(false));
        }
        assert_eq!(unique_id(&["a".into(), "a-2".into()], "a"), "a-3");
    }

    #[test]
    fn finishing_adds_cards_by_detection() {
        use crate::layout::{WORK, placed};
        let fresh = crate::onboarding::fresh_config();
        let work_ids = |c: &Config| c.layout.pages.iter().find(|p| p.id == WORK).unwrap().cards.iter().map(|k| k.id.clone()).collect::<Vec<_>>();
        let git = dev("Web", r"C:\r\web", None);
        let app = Project { path: None, exe: Some(r"C:\Apps\a.exe".into()), kind: crate::config::ProjectKind::App, ..dev("Aplikace", r"C:\x", None) };
        // (popis, Claude nalezen, projekty, Práce po dokončení)
        let cases: Vec<(&str, bool, Vec<Project>, Vec<&str>)> = vec![
            ("nic nenalezeno", false, vec![], vec!["projects", "notes"]),
            ("jen Claude", true, vec![], vec!["projects", "claude", "notes"]),
            ("jen git projekt", false, vec![git.clone()], vec!["projects", "repos", "notes"]),
            ("aplikace nejsou repo", false, vec![app], vec!["projects", "notes"]),
            ("Claude i repo", true, vec![git], vec!["projects", "claude", "repos", "notes"]),
        ];
        for (name, claude, projects, want) in cases {
            let (next, _) = apply(&fresh, &Patch { done: Some(true), claude, projects, ..Default::default() }).unwrap();
            assert_eq!(work_ids(&next), want, "{name}");
            assert!(next.onboarding.done, "{name}");
            assert_eq!(placed(&next.layout, "claude"), claude, "{name}");
        }
        // hotový průvodce spuštěný znovu rozložení nemění
        let mut again = crate::onboarding::fresh_config();
        again.onboarding.done = true;
        let (next, _) = apply(&again, &Patch { done: Some(true), claude: true, ..Default::default() }).unwrap();
        assert_eq!(next.layout, again.layout, "hotový průvodce znovu: rozložení se nemění");
        // krok bez `done` (průběžné ukládání) rozložení nemění ani s detekcí
        let (next, _) = apply(&fresh, &Patch { claude: true, ..Default::default() }).unwrap();
        assert_eq!(next.layout, fresh.layout);
    }
}
