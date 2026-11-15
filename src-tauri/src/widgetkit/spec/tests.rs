use super::*;

const COCKPIT: &str = r#"{ "kit": 1, "title": "Claude kokpit", "icon": "◐",
  "rows": [
    { "bar": "limits.five.pct", "label": "5 h", "tone": {"over": [[95,"red"],[80,"amber"]]} },
    { "text": "Čeká: {waiting.count}", "tone": {"over": [[1,"red"]]}, "open": "drawer:claude" },
    { "text": "Dnes {costs.today|usd}", "copy": "{costs.today}" } ],
  "buttons": [ { "label": "Build", "action": "build-ledge" } ], "neznamy": [1] }"#;

fn ok(s: &str) -> Spec {
    parse(s).unwrap_or_else(|e| panic!("{s} → {e:?}"))
}

#[test]
fn full_example_parses_and_sorts_thresholds() {
    let s = ok(COCKPIT);
    assert_eq!((s.title.as_str(), s.icon.as_deref(), s.rows.len(), s.buttons.len()), ("Claude kokpit", Some("◐"), 3, 1));
    assert!(s.notes.is_empty(), "{:?}", s.notes);
    assert_eq!(s.rows[0].tone.as_ref().unwrap().over, vec![(80.0, KitTone::Amber), (95.0, KitTone::Red)]);
    assert_eq!(s.rows[1].act.open.as_deref(), Some("drawer:claude"));
    assert_eq!(s.buttons[0].act.action.as_deref(), Some("build-ledge"));
    // tvar pro UI = KitSpec (akce ploše na řádku, bez poznámek)
    let j = serde_json::to_value(&s).unwrap();
    assert_eq!(j["kit"], 1);
    assert_eq!(j["rows"][1]["open"], "drawer:claude");
    assert_eq!(j["rows"][0]["tone"]["over"][1], serde_json::json!([95.0, "red"]));
    assert!(j.get("notes").is_none() && j["rows"][0].get("copy").is_none());
}

#[test]
fn fatal_errors_table() {
    let big = format!(r#"{{"kit":1,"title":"x","rows":[{{"text":"{}"}}]}}"#, "a".repeat(MAX_BYTES));
    let cases: [(&str, &str); 9] = [
        ("", "JSON"),
        ("{", "JSON"),
        ("[1]", "objekt"),
        (r#"{"title":"x","rows":[]}"#, "kit"),
        (r#"{"kit":2,"title":"x","rows":[{"text":"a"}]}"#, "kit"),
        (r#"{"kit":1,"rows":[{"text":"a"}]}"#, "title"),
        (r#"{"kit":1,"title":"x"}"#, "rows"),
        (r#"{"kit":1,"title":"x","rows":[]}"#, "nic"),
        (big.as_str(), "KB"),
    ];
    for (input, want) in cases {
        let e = parse(input).unwrap_err();
        assert!(e.iter().any(|n| n.contains(want)), "{input:.40} → {e:?}");
    }
}

#[test]
fn bom_and_unknown_keys_are_tolerated() {
    let s = ok("\u{feff}{\"kit\":1,\"title\":\"T\",\"extra\":{\"a\":1},\"rows\":[{\"text\":\"a\",\"foo\":true}]}");
    assert_eq!(s.rows.len(), 1);
    assert!(s.notes.is_empty());
}

#[test]
fn caps_rows_buttons_and_text() {
    let rows = (0..15).map(|i| format!(r#"{{"text":"r{i}"}}"#)).collect::<Vec<_>>().join(",");
    let btns = (0..9).map(|i| format!(r##"{{"label":"b{i}","url":"https://x.cz/{i}"}}"##)).collect::<Vec<_>>().join(",");
    let long = "ž".repeat(200);
    let s = ok(&format!(r#"{{"kit":1,"title":"{long}","rows":[{rows},{{"text":"{long}"}}],"buttons":[{btns}]}}"#));
    assert_eq!((s.rows.len(), s.buttons.len()), (MAX_ROWS, MAX_BUTTONS));
    assert_eq!(s.title.chars().count(), MAX_TITLE);
    assert!(s.notes.iter().any(|n| n.contains("řádků")) && s.notes.iter().any(|n| n.contains("tlačítek")));
    let t = ok(&format!(r#"{{"kit":1,"title":"x","rows":[{{"text":"{long}"}}]}}"#));
    assert_eq!(t.rows[0].text.as_ref().unwrap().chars().count(), MAX_TEXT);
    assert_eq!(t.notes.len(), 1);
}

#[test]
fn action_whitelist_table() {
    let long_copy = "c".repeat(MAX_COPY + 1);
    let cases: Vec<(String, bool)> = vec![
        (r#""open":"C:\\Fotky""#.into(), true),
        (r#""open":"drawer:claude""#.into(), true),
        (r#""open":"drawer:widget:a.kit.json""#.into(), true),
        (r#""open":"drawer:""#.into(), false),
        (r#""open":"drawer:a b""#.into(), false),
        (r#""open":"""#.into(), false),
        (r#""url":"https://x.cz/a""#.into(), true),
        (r#""url":"http://x.cz""#.into(), true),
        (r#""url":"javascript:alert(1)""#.into(), false),
        (r#""url":"file:///C:/x""#.into(), false),
        (r#""url":"https://x.cz/a b""#.into(), false),
        (r#""copy":"{costs.today}""#.into(), true),
        (r#""copy":"{costs.today""#.into(), false),
        (format!(r#""copy":"{long_copy}""#), false),
        (r#""action":"build-ledge""#.into(), true),
        (r#""action":"Build Ledge""#.into(), false),
        (r#""action":"../x""#.into(), false),
        (r#""action":5"#.into(), false),
    ];
    for (act, good) in cases {
        let s = ok(&format!(r#"{{"kit":1,"title":"x","rows":[{{"text":"a"}}],"buttons":[{{"label":"B",{act}}}]}}"#));
        assert_eq!(s.buttons.len(), usize::from(good), "{act:.60} → {:?}", s.notes);
        assert_eq!(s.notes.is_empty(), good, "{act:.60}");
    }
}

#[test]
fn one_action_per_row_first_wins() {
    let s = ok(r#"{"kit":1,"title":"x","rows":[{"text":"a","copy":"1","url":"https://x.cz","open":"C:\\x"}]}"#);
    let a = &s.rows[0].act;
    assert_eq!((a.open.as_deref(), a.url.as_deref(), a.copy.as_deref()), (Some("C:\\x"), None, None));
    assert_eq!(s.notes.len(), 1);
}

#[test]
fn template_syntax_table() {
    for good in ["", "bez zdroje", "{a}", "{limits.five.pct}", "Dnes {costs.today|usd} a {clock.hm}", "{a_1.b|pct}"] {
        assert!(template_ok(good).is_ok(), "{good}");
    }
    for bad in ["{", "}", "{}", "{a", "a}", "{{a}}", "{a b}", "{a|}", "{|a}", "{a|b|c}", "{a-b}", &format!("{{{}}}", "a".repeat(65))] {
        assert!(template_ok(bad).is_err(), "{bad}");
    }
}

#[test]
fn bad_rows_are_dropped_with_notes() {
    let s = ok(r#"{"kit":1,"title":"x","rows":[
        5, {}, {"label":"jen popisek"}, {"text":"{a"}, {"bar":"a b"}, {"bar":"x.y","text":3},
        {"text":"ok","tone":{"over":[[1,"fialova"],["x","red"],[2],[3,"red"]]}}, {"text":"b","tone":"red"}]}"#);
    assert_eq!(s.rows.len(), 3, "{:?}", s.rows);
    assert_eq!(s.rows[0].bar.as_deref(), Some("x.y"));
    assert_eq!(s.rows[1].tone.as_ref().unwrap().over, vec![(3.0, KitTone::Red)]);
    assert!(s.rows[2].tone.is_none());
    assert!(s.notes.len() >= 8, "{:?}", s.notes);
}

#[test]
fn only_buttons_is_enough_and_too_many_thresholds_are_cut() {
    let s = ok(r#"{"kit":1,"title":"x","rows":[],"buttons":[{"label":"A","open":"drawer:claude"}]}"#);
    assert!((s.rows.len(), s.buttons.len()) == (0, 1));
    let t = ok(r#"{"kit":1,"title":"x","rows":[{"text":"a","tone":{"over":[[1,"ok"],[2,"ok"],[3,"ok"],[4,"ok"],[5,"red"]]}}]}"#);
    assert_eq!(t.rows[0].tone.as_ref().unwrap().over.len(), MAX_TONES);
}

#[test]
fn never_panics_on_garbage() {
    for g in ["null", "true", "\"s\"", "{\"kit\":1,\"title\":[1],\"rows\":{}}", "{\"kit\":1,\"title\":\"x\",\"rows\":[[[]]],\"buttons\":7}", "\u{feff}", "{\"kit\":1.5}"] {
        let _ = parse(g);
    }
}
