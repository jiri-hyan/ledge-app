use super::*;

#[test]
fn empty_object_gives_defaults() {
    let cfg: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(cfg.drawer.trigger, Trigger::Hover);
    assert!(cfg.modes.personal_enabled);
    assert_eq!(cfg.claude.port, 47800);
    assert_eq!(cfg.claude.enforce_commit, EnforceCommit::Notify);
}

#[test]
fn validation_messages_follow_language() {
    let mut cfg = Config::default();
    cfg.strip.opacity = 2.0;
    assert_eq!(validate(&cfg).unwrap_err(), "strip.opacity musí být 0–1.");
    crate::i18n::with_lang(crate::uiconf::Lang::En, || {
        let e = validate_field(&cfg).unwrap_err();
        assert_eq!((e.field.as_deref(), e.message.as_str()), (Some("strip.opacity"), "strip.opacity must be 0–1."));
    });
    let en: Vec<String> = default_note_kinds_in(crate::uiconf::Lang::En).into_iter().map(|k| k.label).collect();
    assert_eq!(en, ["Idea", "Feedback", "To work on", "Note"]);
    assert_eq!(default_note_kinds(), default_note_kinds_in(crate::uiconf::Lang::Cs));
}

#[test]
fn enforce_commit_old_config_and_values() {
    // starý config bez pole → notify
    let cfg: Config = serde_json::from_str(r#"{"claude":{"enabled":true,"port":47800,"approvalTimeoutS":45}}"#).unwrap();
    assert_eq!(cfg.claude.enforce_commit, EnforceCommit::Notify);
    for (j, e) in [("off", EnforceCommit::Off), ("notify", EnforceCommit::Notify), ("block", EnforceCommit::Block)] {
        let cfg: Config = serde_json::from_str(&format!(r#"{{"claude":{{"enforceCommit":"{j}"}}}}"#)).unwrap();
        assert_eq!(cfg.claude.enforce_commit, e);
    }
    assert!(serde_json::from_str::<Config>(r#"{"claude":{"enforceCommit":"nevim"}}"#).is_err());
    assert!(schema_json().contains("enforceCommit"));
}

#[test]
fn roundtrip_and_atomic_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut cfg = Config::default();
    cfg.projects.push(Project {
        id: "acme-app".into(),
        name: "Acme App".into(),
        enabled: true,
        kind: ProjectKind::Dev,
        path: None,
        remote: Some("acme/app".into()),
        subdir: None,
        run: Some("npm run dev:staging".into()),
        port: Some(5210),
        pass_port: true,
        open_browser: true,
        exe: None,
        app_name: None,
        aumid: None,
        args: vec![],
        accent: Some("#e0a13a".into()),
        monogram: None,
        logo: None,
        links: vec![],
        look: None,
        mute_dirty: false,
    });
    save(&path, &cfg).unwrap();
    let names: Vec<String> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["config.json"], "žádný tmp ani záloha");
    let back = load(&path).unwrap().unwrap();
    assert_eq!(back, cfg);
}

#[test]
fn rejects_duplicate_ids_and_bad_accent() {
    let mut cfg = Config::default();
    let p = Project {
        id: "a".into(),
        name: "A".into(),
        enabled: true,
        kind: ProjectKind::App,
        path: None,
        remote: None,
        subdir: None,
        run: None,
        port: None,
        pass_port: true,
        open_browser: true,
        exe: Some("a.exe".into()),
        app_name: None,
        aumid: None,
        args: vec![],
        accent: None,
        monogram: None,
        logo: None,
        links: vec![],
        look: None,
        mute_dirty: false,
    };
    cfg.projects = vec![p.clone(), p.clone()];
    assert!(validate(&cfg).unwrap_err().contains("dvakrát"));
    let mut q = p.clone();
    q.accent = Some("red".into());
    cfg.projects = vec![q];
    assert!(validate(&cfg).is_err());
}

#[test]
fn missing_file_is_none_and_bom_is_tolerated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    assert!(load(&path).unwrap().is_none());
    std::fs::write(&path, "\u{feff}{}").unwrap();
    assert!(load(&path).unwrap().is_some());
}

#[test]
fn old_config_without_bar_windows_gets_browser_defaults() {
    let cfg: Config = serde_json::from_str(r#"{"drawer":{"trigger":"hover","hoverDelayMs":140}}"#).unwrap();
    let b = &cfg.drawer.bar_windows;
    assert_eq!(b.trigger, BarTrigger::Push);
    assert_eq!(b.push_ms, 300);
    assert_eq!(b.push_distance, 60);
    assert!(b.processes.iter().any(|p| p == "opera.exe"));
    assert!(!b.processes.iter().any(|p| p.eq_ignore_ascii_case("code.exe")));
    // částečný blok: chybějící pole doplní výchozí
    let cfg: Config = serde_json::from_str(r#"{"drawer":{"barWindows":{"pushMs":500}}}"#).unwrap();
    assert_eq!(cfg.drawer.bar_windows.push_ms, 500);
    assert_eq!(cfg.drawer.bar_windows.processes.len(), 7);
    let cfg: Config = serde_json::from_str(r#"{"drawer":{"barWindows":{"trigger":"hover","processes":["Code.exe"]}}}"#).unwrap();
    assert_eq!(cfg.drawer.bar_windows.trigger, BarTrigger::Hover);
    assert_eq!(cfg.drawer.bar_windows.processes, vec!["Code.exe".to_string()]);
}

#[test]
fn sections_defaults_all_on() {
    let d = serde_json::to_value(Sections::default()).unwrap();
    let obj = d.as_object().unwrap();
    assert_eq!(obj.len(), Sections::FIELDS.len());
    // výchozí = podle čistého výchozího rozložení: jen karty, které v něm jsou
    let on: Vec<&str> = obj.iter().filter(|(_, v)| **v == serde_json::Value::Bool(true)).map(|(k, _)| k.as_str()).collect();
    assert_eq!(on, ["battery", "clock", "favorites", "links", "notes", "nowPlaying", "projects", "sound"], "{d}");
    assert_eq!(Sections::default(), crate::layout::derived_sections(&crate::layout::LayoutConfig::default(), &Sections::default()));
    // (JSON drawer.sections, pole → očekávaná hodnota)
    let cases: &[(&str, &[(&str, bool)])] = &[
        // jen 4 pole: uvedená platí, ostatní podle výchozího rozložení (starý config s `layout` / bez něj
        // se stejně odvozuje z rozložení při načtení — `layout::derive`)
        (r#"{"projects":true,"running":false,"claude":true,"notes":true}"#, &[("running", false), ("claude", true), ("repos", false), ("weather", false), ("claudeCosts", false), ("clock", true)]),
        // úplně prázdné a chybějící sekce = výchozí rozložení
        ("{}", &[("projects", true), ("shelf", false), ("claudeSubagents", false), ("links", true)]),
        // nová pole se čtou camelCase
        (r#"{"repos":true,"claudeLimits":true,"claudeGit":false}"#, &[("repos", true), ("claudeLimits", true), ("claudeGit", false), ("claude", false)]),
        // neznámý klíč (pole přidané novější verzí) neshodí načtení
        (r#"{"nowPlaying":false,"clock":false}"#, &[("clock", false), ("links", true)]),
    ];
    for (json, want) in cases {
        let cfg: Config = serde_json::from_str(&format!(r#"{{"drawer":{{"sections":{json}}}}}"#)).unwrap();
        let v = serde_json::to_value(&cfg.drawer.sections).unwrap();
        for (k, on) in *want {
            assert_eq!(v[*k], serde_json::Value::Bool(*on), "{json} → {k}");
        }
    }
    let cfg: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(cfg.drawer.sections, Sections::default());
    let s = schema_json();
    assert!(s.contains("claudeSubagents") && s.contains("weather"));
}

#[test]
fn old_config_without_notes_gets_defaults() {
    let old = r#"{"drawer":{"sections":{"projects":true,"running":false,"claude":true}},
        "hotkeys":{"toggle":"Ctrl+Alt+Space","approve":"Ctrl+Alt+Y","deny":"Ctrl+Alt+D"}}"#;
    let cfg: Config = serde_json::from_str(old).unwrap();
    assert!(cfg.notes.enabled);
    assert_eq!(cfg.notes.root, "");
    assert_eq!(cfg.notes.kinds, default_note_kinds());
    assert_eq!(cfg.hotkeys.note, "Ctrl+Alt+N");
    assert!(cfg.drawer.sections.notes);
    assert!(!cfg.drawer.sections.running);
    let cfg: Config =
        serde_json::from_str(r#"{"notes":{"enabled":false},"hotkeys":{"note":"Ctrl+Alt+M"},"drawer":{"sections":{"notes":false}}}"#).unwrap();
    assert!(!cfg.notes.enabled);
    assert_eq!(cfg.hotkeys.note, "Ctrl+Alt+M");
    assert_eq!(cfg.hotkeys.toggle, "Ctrl+Alt+Space");
    assert!(!cfg.drawer.sections.notes);
    let s = schema_json();
    assert!(s.contains("\"notes\"") && s.contains("\"note\""));
    // bez notes.sync → vypnuto, bez remote (starý config dorovná `onboarding::upgrade_loaded` v `load`)
    assert_eq!(cfg.notes.sync, NotesSyncConfig::default());
    assert!(!cfg.notes.sync.enabled && cfg.notes.sync.interval_min == 5 && cfg.notes.sync.remote.is_empty());
    assert!(s.contains("intervalMin"));
}

#[test]
fn notes_sync_validation() {
    const NOTES: &str = "https://github.com/acme/notes.git";
    let with = |enabled: bool, remote: &str, interval_min: u32| {
        let mut c = Config::default();
        c.notes.sync = NotesSyncConfig { enabled, remote: remote.into(), interval_min };
        validate(&c)
    };
    assert!(with(true, NOTES, 5).is_ok());
    for ok in ["git@github.com:acme/notes.git", "ssh://git@github.com/x/y.git", "file:///C:/tmp/bare", r"C:\tmp\bare.git", "D:/x/notes.git"] {
        assert!(with(true, ok, 5).is_ok(), "{ok}");
    }
    for bad in ["", "http://github.com/x/y.git", "github.com/x/y.git", "-uexec", "https://", "https://x/ y", "relativni/cesta", "ext::sh -c x"] {
        assert!(with(true, bad, 5).is_err(), "{bad}");
    }
    // vypnutý sync s prázdným remote je v pořádku, s nesmyslem ne
    assert!(with(false, "", 5).is_ok());
    assert!(with(false, "http://x/y", 5).is_err());
    assert!(with(true, NOTES, 0).is_err());
    assert!(with(true, NOTES, 121).is_err());
    assert!(with(true, NOTES, 120).is_ok() && with(true, NOTES, 1).is_ok());
    let cfg: Config = serde_json::from_str(r#"{"notes":{"sync":{"enabled":false}}}"#).unwrap();
    assert!(!cfg.notes.sync.enabled && cfg.notes.sync.remote.is_empty() && cfg.notes.sync.interval_min == 5);
}

#[test]
fn hotkeys_off_by_default_even_in_old_config() {
    assert!(!Config::default().hotkeys.enabled);
    assert!(active_hotkeys(&Config::default(), true).is_empty());
    // šéfův config: zkratky vypsané výslovně, pole `enabled` chybí → nic se neregistruje
    let old = r#"{"hotkeys":{"toggle":"Ctrl+Alt+Space","approve":"Ctrl+Alt+Y","deny":"Ctrl+Alt+D","note":"Ctrl+Alt+N"}}"#;
    let cfg = load_str(old).unwrap();
    assert!(!cfg.hotkeys.enabled);
    assert_eq!(cfg.hotkeys.toggle, "Ctrl+Alt+Space");
    assert!(active_hotkeys(&cfg, false).is_empty());
    assert!(active_hotkeys(&cfg, true).is_empty());
    // vypnutí za běhu = odregistrovat všechno, nic nového
    let was = vec![("toggle", "Ctrl+Alt+Space".to_string()), ("note", "Ctrl+Alt+N".to_string())];
    assert_eq!(rebind_plan(&was, &active_hotkeys(&cfg, true)), (vec!["Ctrl+Alt+Space".to_string(), "Ctrl+Alt+N".into()], vec![]));
    // výslovně zapnuté se registrují jako dřív
    let on = load_str(r#"{"hotkeys":{"enabled":true}}"#).unwrap();
    assert_eq!(active_hotkeys(&on, false).len(), 2);
    assert!(schema_json().contains("\"enabled\""));
}

#[test]
fn trigger_off_needs_hotkeys() {
    // šuplík by nešel otevřít vůbec
    let e = load_str(r#"{"drawer":{"trigger":"off"}}"#).unwrap_err();
    assert!(e.contains("myší"), "{e}");
    assert!(load_str(r#"{"drawer":{"trigger":"off"},"hotkeys":{"toggle":"Ctrl+Alt+Space"}}"#).is_err());
    assert!(load_str(r#"{"drawer":{"trigger":"off"},"hotkeys":{"enabled":true,"toggle":" "}}"#).is_err());
    // se zapnutou zkratkou to jde, myší ovládané triggery vždy
    assert!(load_str(r#"{"drawer":{"trigger":"off"},"hotkeys":{"enabled":true}}"#).is_ok());
    assert!(load_str(r#"{"drawer":{"trigger":"edge"}}"#).is_ok());
}

#[test]
fn hotkey_normalize_and_collisions() {
    assert_eq!(normalize_hotkey("Alt+Ctrl+N"), normalize_hotkey(" ctrl + alt + n "));
    assert_eq!(normalize_hotkey("Control+Option+Space"), "alt+ctrl+space");
    assert_ne!(normalize_hotkey("Ctrl+Alt+N"), normalize_hotkey("Ctrl+Shift+N"));
    let mut cfg = Config::default();
    // vypnuté zkratky se nekontrolují (nikdy se neregistrují)
    cfg.hotkeys.note = "alt+ctrl+d".into();
    assert!(validate(&cfg).is_ok());
    cfg.hotkeys = HotkeyConfig { enabled: true, ..HotkeyConfig::default() };
    assert!(validate(&cfg).is_ok());
    cfg.hotkeys.note = "alt+ctrl+d".into(); // = deny
    let e = validate(&cfg).unwrap_err();
    assert!(e.contains("hotkeys.deny") && e.contains("hotkeys.note"), "{e}");
    // s vypnutými poznámkami note nekoliduje (nikdy se neregistruje)
    cfg.notes.enabled = false;
    assert!(validate(&cfg).is_ok());
    cfg.hotkeys.toggle = "Ctrl+Alt+Y".into(); // = approve
    assert!(validate(&cfg).is_err());
    // starý config s kolizí se nenačte (zůstane poslední dobrý) — jen se zapnutými zkratkami
    assert!(load_str(r#"{"hotkeys":{"enabled":true,"toggle":"Ctrl+Alt+N"}}"#).is_err());
    assert!(load_str(r#"{"hotkeys":{"toggle":"Ctrl+Alt+N"}}"#).is_ok());
}

/// Zkratka průchozího režimu lístků: jen s lístkem na ploše, i s vypnutými zkratkami (cesta ven),
/// starý config ji dostane výchozí, kolize se hlásí jen se zapnutými zkratkami.
#[test]
fn pin_passthrough_hotkey() {
    let keys = |v: &[(&'static str, String)]| v.iter().map(|(a, k)| format!("{a}={k}")).collect::<Vec<_>>();
    let old = load_str(r#"{"hotkeys":{"toggle":"Ctrl+Alt+Space","note":"Ctrl+Alt+N"}}"#).unwrap();
    assert_eq!(old.hotkeys.note_passthrough, "Ctrl+Alt+P");
    let cases: &[(bool, bool, bool, &[&str])] = &[
        // (zkratky zapnuté, poznámky zapnuté, lístek na ploše, registrované)
        (false, true, false, &[]),
        (false, true, true, &["notePassthrough=Ctrl+Alt+P"]),
        (false, false, true, &[]),
        (true, true, false, &["toggle=Ctrl+Alt+Space", "note=Ctrl+Alt+N"]),
        (true, true, true, &["toggle=Ctrl+Alt+Space", "note=Ctrl+Alt+N", "notePassthrough=Ctrl+Alt+P"]),
    ];
    for (enabled, notes, open, want) in cases {
        let mut cfg = old.clone();
        cfg.hotkeys.enabled = *enabled;
        cfg.notes.enabled = *notes;
        assert_eq!(keys(&active_hotkeys_pins(&cfg, false, *open)), *want, "{enabled} {notes} {open}");
    }
    // prázdná = žádná zkratka
    let mut cfg = old.clone();
    cfg.hotkeys.note_passthrough = " ".into();
    assert!(active_hotkeys_pins(&cfg, false, true).is_empty());
    // kolize: se zapnutými zkratkami chyba u pole, s vypnutými ne (registruje se sama)
    assert!(load_str(r#"{"hotkeys":{"enabled":true,"notePassthrough":"Alt+Ctrl+N"}}"#).unwrap_err().contains("hotkeys.notePassthrough"));
    assert!(load_str(r#"{"hotkeys":{"notePassthrough":"Ctrl+Alt+N"}}"#).is_ok());
    // vypnuté poznámky = lístky neexistují → žádná kolize
    assert!(load_str(r#"{"notes":{"enabled":false},"hotkeys":{"enabled":true,"notePassthrough":"Ctrl+Alt+Space"}}"#).is_ok());
}

fn load_str(s: &str) -> Result<Config, String> {
    let cfg: Config = serde_json::from_str(s).map_err(|e| e.to_string())?;
    validate(&cfg)?;
    Ok(cfg)
}

#[test]
fn active_set_and_rebind_plan() {
    let mut cfg = Config::default();
    cfg.hotkeys.enabled = true;
    let keys = |v: &[(&'static str, String)]| v.iter().map(|(a, k)| format!("{a}={k}")).collect::<Vec<_>>();
    assert_eq!(keys(&active_hotkeys(&cfg, false)), ["toggle=Ctrl+Alt+Space", "note=Ctrl+Alt+N"]);
    assert_eq!(keys(&active_hotkeys(&cfg, true)), ["toggle=Ctrl+Alt+Space", "approve=Ctrl+Alt+Y", "deny=Ctrl+Alt+D", "note=Ctrl+Alt+N"]);
    cfg.notes.enabled = false;
    assert_eq!(keys(&active_hotkeys(&cfg, false)), ["toggle=Ctrl+Alt+Space"]);

    // beze změny nic
    let a = active_hotkeys(&cfg, false);
    assert_eq!(rebind_plan(&a, &a), (vec![], vec![]));
    // prohození A↔B: nejdřív pryč obě staré, pak obě nové
    let old = vec![("toggle", "Ctrl+Alt+A".to_string()), ("note", "Ctrl+Alt+B".to_string())];
    let new = vec![("toggle", "Ctrl+Alt+B".to_string()), ("note", "Ctrl+Alt+A".to_string())];
    assert_eq!(rebind_plan(&old, &new), (vec!["Ctrl+Alt+A".to_string(), "Ctrl+Alt+B".into()], vec!["Ctrl+Alt+B".to_string(), "Ctrl+Alt+A".into()]));
    // vypnutí poznámek: odregistrovat i note
    let off = vec![("toggle", "Ctrl+Alt+A".to_string())];
    assert_eq!(rebind_plan(&old, &off), (vec!["Ctrl+Alt+A".to_string(), "Ctrl+Alt+B".into()], vec!["Ctrl+Alt+A".to_string()]));
}

#[test]
fn personal_defaults_and_old_config() {
    // výchozí: záložka zapnutá, místo nenastavené, žádné odkazy
    let cfg = Config::default();
    assert!(cfg.modes.personal_enabled);
    assert_eq!(cfg.personal.weather, WeatherPlace { lat: None, lon: None, label: String::new() });
    assert!(cfg.personal.weather.coords().is_none());
    assert!(cfg.personal.links.is_empty());
    // nenastavené místo se zapíše jako null (šéf vidí, co doplnit)
    let v = serde_json::to_value(&cfg).unwrap();
    assert_eq!(v["personal"]["weather"], serde_json::json!({ "lat": null, "lon": null, "label": "" }));
    assert_eq!(v["personal"]["links"], serde_json::json!([]));
    // starý config s výslovným false zůstane vypnutý, bez bloku personal dostane výchozí
    let cfg: Config = serde_json::from_str(r#"{"modes":{"personalEnabled":false}}"#).unwrap();
    assert!(!cfg.modes.personal_enabled);
    assert!(cfg.personal.links.is_empty());
    let cfg: Config = serde_json::from_str(r#"{"modes":{}}"#).unwrap();
    assert!(cfg.modes.personal_enabled);
    // vyplněný blok
    let cfg: Config = serde_json::from_str(
        r#"{"personal":{"weather":{"lat":50.08,"lon":14.42,"label":"Praha"},"links":[{"name":"Mapy","target":"https://mapy.cz"}]}}"#,
    )
    .unwrap();
    assert_eq!(cfg.personal.weather.coords(), Some((50.08, 14.42)));
    assert_eq!(cfg.personal.weather.label, "Praha");
    assert_eq!(cfg.personal.links[0].target, "https://mapy.cz");
    assert!(validate(&cfg).is_ok());
    // jen jedna souřadnice = chyba validace (ne tiché „nenastaveno“); mimo rozsah = chyba
    let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lat":50.0}}}"#).unwrap();
    assert!(cfg.personal.weather.coords().is_none());
    assert!(validate(&cfg).unwrap_err().contains("šířku i délku"));
    let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lon":14.0}}}"#).unwrap();
    assert!(validate(&cfg).is_err());
    let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lat":91,"lon":0}}}"#).unwrap();
    assert!(validate(&cfg).is_err());
    let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lat":0,"lon":-181}}}"#).unwrap();
    assert!(validate(&cfg).is_err());
    assert!(schema_json().contains("\"personal\""));
}

#[test]
fn personal_links_validation() {
    let link = |name: &str, target: &str| PersonalLink { name: name.into(), target: target.into() };
    let mut cfg = Config::default();
    cfg.personal.links = vec![link("Mapy", "https://mapy.cz")];
    assert!(validate(&cfg).is_ok());
    cfg.personal.links = vec![link(" ", "https://mapy.cz")];
    assert!(validate(&cfg).unwrap_err().contains("č. 1 nemá název"));
    cfg.personal.links = vec![link("Mapy", "https://mapy.cz"), link("Fotky", "  ")];
    assert!(validate(&cfg).unwrap_err().contains("„Fotky“ nemá cíl"));
    cfg.personal.links = vec![link("Dlouhý", &format!("https://x.cz/{}", "a".repeat(MAX_LINK_TARGET)))];
    assert!(validate(&cfg).unwrap_err().contains("1024"));
    cfg.personal.links = vec![link("Akorát", &"a".repeat(MAX_LINK_TARGET))];
    assert!(validate(&cfg).is_ok());
    cfg.personal.links = (0..=MAX_LINKS).map(|i| link(&format!("L{i}"), "https://x.cz")).collect();
    assert!(validate(&cfg).unwrap_err().contains("nejvýš 24"));
    cfg.personal.links.pop();
    assert!(validate(&cfg).is_ok());
}

#[test]
fn note_kinds_validation() {
    let mut c = Config::default();
    assert!(validate(&c).is_ok());
    assert!(schema_json().contains("\"kinds\""));
    let bad = |f: &dyn Fn(&mut NoteKind)| {
        let mut k = default_note_kinds();
        f(&mut k[0]);
        k
    };
    for kinds in [
        bad(&|k| k.id = "a b".into()),
        bad(&|k| k.id = "feedback".into()),
        bad(&|k| k.label = " ".into()),
        bad(&|k| k.color = "red".into()),
        bad(&|k| k.icon = "<svg>".into()),
    ] {
        c.notes.kinds = kinds;
        assert!(validate(&c).is_err(), "{:?}", c.notes.kinds[0]);
    }
    c.notes.kinds = bad(&|k| {
        k.color = String::new();
        k.icon = String::new();
    });
    assert!(validate(&c).is_ok());
    // vlastní sada z configu (bez výchozích) projde
    let cfg: Config = serde_json::from_str(r#"{"notes":{"kinds":[{"id":"bug","label":"Chyba"}]}}"#).unwrap();
    assert_eq!(cfg.notes.kinds.len(), 1);
    assert!(validate(&cfg).is_ok());
}

#[test]
fn schema_mentions_projects() {
    assert!(schema_json().contains("projects"));
}

#[test]
fn project_mute_dirty_old_config_and_roundtrip() {
    // starý tvar projektu (před kolem 18) bez `muteDirty` = upozorňovat jako dřív
    let old = r#"{"projects":[{"id":"orbit","name":"Orbit","kind":"dev","remote":"acme/orbit","run":"npm run dev"}]}"#;
    let cfg: Config = serde_json::from_str(old).unwrap();
    assert!(!cfg.projects[0].mute_dirty);
    let back = serde_json::to_value(&cfg.projects[0]).unwrap();
    assert!(back.get("muteDirty").is_none(), "false se nepíše: {back}");
    let cfg: Config = serde_json::from_str(&old.replace(r#""run":"npm run dev""#, r#""run":"npm run dev","muteDirty":true"#)).unwrap();
    assert!(cfg.projects[0].mute_dirty);
    assert_eq!(serde_json::to_value(&cfg.projects[0]).unwrap()["muteDirty"], true);
}

#[test]
fn git_alerts_old_config_and_validation() {
    // starý config bez `gitAlerts` = upozorňovat, nic přečteného; výchozí se nepíše
    let cfg: Config = serde_json::from_str(r#"{"projects":[]}"#).unwrap();
    assert_eq!(cfg.git_alerts, GitAlerts { enabled: true, read: String::new() });
    assert!(serde_json::to_value(&cfg).unwrap().get("gitAlerts").is_none());
    let cfg: Config = serde_json::from_str(r#"{"gitAlerts":{"enabled":false,"read":"k3x9a1"}}"#).unwrap();
    assert!(!cfg.git_alerts.enabled && cfg.git_alerts.read == "k3x9a1");
    assert!(validate(&cfg).is_ok());
    let mut bad = cfg.clone();
    bad.git_alerts.read = "a".repeat(65);
    assert!(validate(&bad).is_err());
    bad.git_alerts.read = "ne platny".into();
    assert!(validate(&bad).is_err());
}
