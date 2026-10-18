//! Testy historie schránky bez skutečné schránky: historie, náhled, připnuté v tempdiru, config.
//! Filtr (formáty, tajemství, vlastní zápis) má tabulky přímo v `filter.rs`.

use super::*;

fn texts(h: &History) -> Vec<String> {
    h.view().into_iter().map(|v| v.preview).collect()
}

#[test]
fn push_dedup_moves_up_and_trims() {
    let mut h = History::default();
    // (text, čas, strop, očekávané pořadí po kroku)
    let steps: Vec<(&str, i64, usize, Vec<&str>)> = vec![
        ("a", 1, 3, vec!["a"]),
        ("b", 2, 3, vec!["b", "a"]),
        ("c", 3, 3, vec!["c", "b", "a"]),
        ("a", 4, 3, vec!["a", "c", "b"]), // stejný text posune nahoru, nový záznam nevznikne
        ("d", 5, 3, vec!["d", "a", "c"]), // nejstarší vypadne
        ("e", 6, 1, vec!["e"]),           // menší strop ořízne hned
    ];
    for (t, now, max, want) in steps {
        h.push(t, now, max);
        assert_eq!(texts(&h), want, "po {t}");
    }
    assert_eq!(h.len(), 1);
}

#[test]
fn dedup_keeps_id_and_updates_time() {
    let mut h = History::default();
    h.push("x", 1, 10);
    h.push("y", 2, 10);
    let id = h.view().iter().find(|v| v.preview == "x").unwrap().id;
    h.push("x", 9, 10);
    let v = &h.view()[0];
    assert_eq!((v.id, v.at_ms, v.preview.as_str()), (id, 9, "x"));
}

#[test]
fn pinned_survive_trim_and_clear_and_sit_on_top() {
    let mut h = History::default();
    for (i, t) in ["a", "b", "c"].iter().enumerate() {
        h.push(t, i as i64, 3);
    }
    let a = h.view().iter().find(|v| v.preview == "a").unwrap().id;
    assert_eq!(h.pin(a, true), Ok(true));
    assert_eq!(h.pin(a, true), Ok(false), "podruhé nic");
    assert_eq!(texts(&h), ["a", "c", "b"], "připnuté nahoře");
    for t in ["d", "e", "f", "g"] {
        h.push(t, 10, 3);
    }
    assert_eq!(texts(&h), ["a", "g", "f", "e"], "strop počítá jen nepřipnuté");
    assert!(h.clear_unpinned());
    assert_eq!(texts(&h), ["a"]);
    assert!(!h.clear_unpinned(), "nic ke smazání = beze změny");
    assert_eq!(h.remove(a), Some(true));
    assert_eq!(h.remove(a), None);
    assert_eq!(h.pin(a, true), Err(PinErr::Missing));
}

#[test]
fn pin_limit() {
    let mut h = History::default();
    for i in 0..=MAX_PINS {
        h.push(&format!("t{i}"), i as i64, 200);
    }
    let ids: Vec<u64> = h.view().iter().map(|v| v.id).collect();
    for id in &ids[..MAX_PINS] {
        assert_eq!(h.pin(*id, true), Ok(true));
    }
    assert_eq!(h.pin(ids[MAX_PINS], true), Err(PinErr::TooMany));
    assert_eq!(h.pin(ids[0], false), Ok(true));
    assert_eq!(h.pin(ids[MAX_PINS], true), Ok(true));
}

#[test]
fn memory_cap_evicts_oldest_but_keeps_newest() {
    let mut h = History::default();
    let big = "x".repeat(filter::MAX_TEXT_BYTES);
    let n = MAX_TOTAL_BYTES / filter::MAX_TEXT_BYTES + 5;
    for i in 0..n {
        h.push(&format!("{i}{big}"), i as i64, 200);
    }
    let bytes: usize = h.items.iter().map(|e| e.text.len()).sum();
    assert!(bytes <= MAX_TOTAL_BYTES, "{bytes}");
    assert!(h.items[0].text.starts_with(&format!("{}x", n - 1)), "nejnovější zůstává");
}

#[test]
fn preview_one_line_and_cut() {
    let long = "slovo ".repeat(100);
    let cases: Vec<(&str, String)> = vec![
        ("ahoj", "ahoj".into()),
        ("  první\r\n\tdruhý  řádek \n", "první druhý řádek".into()),
        (&long, format!("{}…", &long.trim_end()[..PREVIEW_CHARS])),
    ];
    for (inp, want) in cases {
        assert_eq!(preview(inp), want);
    }
    assert_eq!(preview(&"ž".repeat(500)).chars().count(), PREVIEW_CHARS + 1, "ořez po znacích, ne bajtech");
    let mut h = History::default();
    h.push("a\nb\nc", 1, 5);
    let v = &h.view()[0];
    assert_eq!((v.chars, v.lines), (5, 3));
}

#[test]
fn pins_file_roundtrip_in_tempdir() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sub").join(PINS_FILE);
    let pins = vec![Pin { text: "git status".into(), at_ms: 5 }, Pin { text: "řádek\ns diakritikou".into(), at_ms: 6 }];
    save_pins_to(&file, &pins).unwrap();
    assert_eq!(load_pins(&file), pins);
    let h = History::from_pins(load_pins(&file));
    assert!(h.view().iter().all(|v| v.pinned));
    assert_eq!(h.pins(), pins);
    // prázdné připnuté = soubor pryč
    save_pins_to(&file, &[]).unwrap();
    assert!(!file.exists());
    save_pins_to(&file, &[]).unwrap();
    // poškozený soubor = žádné připnuté, žádná panika
    std::fs::write(&file, "{nesmysl").unwrap();
    assert!(load_pins(&file).is_empty());
    assert!(load_pins(&dir.path().join("neni.json")).is_empty());
}

#[test]
fn from_pins_drops_junk_and_duplicates() {
    let p = |t: &str| Pin { text: t.into(), at_ms: 0 };
    let h = History::from_pins(vec![p("a"), p(""), p("   "), p("a"), p("b")]);
    assert_eq!(texts(&h), ["a", "b"]);
    let many: Vec<Pin> = (0..MAX_PINS + 10).map(|i| p(&format!("p{i}"))).collect();
    assert_eq!(History::from_pins(many).len(), MAX_PINS);
}

#[test]
fn config_defaults_off_and_roundtrip() {
    let cfg: crate::config::Config = serde_json::from_str("{}").unwrap();
    let c = &cfg.clipboard;
    assert!(!c.enabled, "chybějící sekce = vypnuto");
    assert_eq!(c.max, DEFAULT_MAX);
    assert!(c.skip_secrets && c.clear_on_lock && c.save_pins);
    assert!(c.exclude_apps.iter().any(|a| a == "KeePassXC.exe"));
    let cfg: crate::config::Config = serde_json::from_str(r#"{"clipboard":{"enabled":true}}"#).unwrap();
    assert!(cfg.clipboard.enabled);
    assert_eq!(cfg.clipboard.max, DEFAULT_MAX, "chybějící pole = výchozí");
    assert_eq!(cfg.clipboard.exclude_apps.len(), 4);
    let back = serde_json::to_string(&cfg.clipboard).unwrap();
    for k in ["\"enabled\":true", "\"max\":25", "\"skipSecrets\":true", "\"excludeApps\"", "\"clearOnLock\":true", "\"savePins\":true"] {
        assert!(back.contains(k), "{back}");
    }
    let schema = crate::config::schema_json();
    assert!(schema.contains("excludeApps") && schema.contains("clearOnLock"));
}

#[test]
fn validate_table() {
    let ok = ClipboardConfig::default();
    let cases: Vec<(ClipboardConfig, Option<&str>)> = vec![
        (ok.clone(), None),
        (ClipboardConfig { max: 0, ..ok.clone() }, Some("clipboard.max")),
        (ClipboardConfig { max: 1, ..ok.clone() }, None),
        (ClipboardConfig { max: MAX_MAX, ..ok.clone() }, None),
        (ClipboardConfig { max: MAX_MAX + 1, ..ok.clone() }, Some("clipboard.max")),
        (ClipboardConfig { exclude_apps: vec!["a.exe".into(); MAX_EXCLUDE + 1], ..ok.clone() }, Some("clipboard.excludeApps")),
    ];
    for (c, want) in cases {
        assert_eq!(validate(&c).err().and_then(|e| e.field), want.map(String::from), "{c:?}");
    }
    let mut cfg = crate::config::Config::default();
    cfg.clipboard.max = 500;
    assert!(crate::config::validate(&cfg).is_err(), "validace je zapojená v config::validate");
}
