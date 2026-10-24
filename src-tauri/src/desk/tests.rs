use super::*;

const FHD: Area = Area { l: 0, t: 0, r: 1920, b: 1040 };
const RIGHT: Area = Area { l: 1920, t: 0, r: 4480, b: 1400 };

fn mons() -> Vec<Mon> {
    vec![Mon { work: FHD, scale: 1.0 }, Mon { work: RIGHT, scale: 1.5 }]
}

fn w(id: &str, kind: &str) -> DeskWidget {
    DeskWidget { id: id.into(), kind: kind.into(), ..DeskWidget::default() }
}

#[test]
fn kinds_parse_and_key() {
    let cases: &[(&str, Option<Kind>)] = &[
        ("limits", Some(Kind::Limits)),
        (" waiting ", Some(Kind::Waiting)),
        ("dev", Some(Kind::Dev)),
        ("agenda", Some(Kind::Agenda)),
        ("widget:docker.ps1", Some(Kind::Script("widget:docker.ps1".into()))),
        ("widget:", None),
        ("widget:..\\x.ps1", None),
        ("claude", None),
        ("", None),
    ];
    for (s, want) in cases {
        assert_eq!(Kind::parse(s), *want, "{s:?}");
        if let Some(k) = want {
            assert_eq!(Kind::parse(&k.key()).as_ref(), Some(k), "tam a zpět {s:?}");
        }
    }
}

#[test]
fn ids_and_labels() {
    assert!(valid_id("lim1") && valid_id("wg12"));
    for bad in ["", "Lim1", "a-b", "a b", "x".repeat(17).as_str()] {
        assert!(!valid_id(bad), "{bad:?}");
    }
    assert_eq!(label_for("dev1"), "desk-dev1");
    assert_eq!(id_of("desk-dev1"), Some("dev1"));
    assert_eq!(id_of("note-abc"), None);
    assert_eq!(id_of("desk-"), None);
    assert_eq!(new_id(&Kind::Dev, &[]), "dev1");
    assert_eq!(new_id(&Kind::Dev, &["dev1", "dev2"]), "dev3");
    assert_eq!(new_id(&Kind::Script("widget:x".into()), &["dev1"]), "wg1");
    // nikdy se nepotká s lístkem, oknem webu ani šuplíkem
    let l = label_for("lim1");
    assert!(!l.starts_with(crate::pins::LABEL_PREFIX) && !l.starts_with(crate::webpanel::LABEL_PREFIX) && l != crate::drawer::LABEL);
}

#[test]
fn config_default_and_old_config() {
    let d = DeskConfig::default();
    assert_eq!((d.layer, d.hide_in_game, d.opacity, d.snap), (Layer::Normal, true, 100, true));
    // starý config bez `desk` = výchozí chování, nic se nemění
    let old: crate::config::Config = serde_json::from_str(r#"{"drawer":{"width":900}}"#).unwrap();
    assert_eq!(old.desk, d);
    // částečný klíč doplní zbytek
    let part: crate::config::Config = serde_json::from_str(r#"{"desk":{"layer":"desktop","hideInGame":false}}"#).unwrap();
    assert_eq!(part.desk, DeskConfig { layer: Layer::Desktop, hide_in_game: false, ..d.clone() });
    let out = serde_json::to_value(&part.desk).unwrap();
    assert_eq!(out, serde_json::json!({"layer":"desktop","hideInGame":false,"opacity":100,"snap":true}));
    // rozbitá sekce nikdy neshodí celý config: špatné typy = výchozí, průhlednost do mezí
    let cases = [
        (r#"{"desk":null}"#, d.clone()),
        (r#"{"desk":[]}"#, d.clone()),
        (r#"{"desk":"x"}"#, d.clone()),
        (r#"{"desk":{"layer":"pod","hideInGame":"ano","opacity":-1,"snap":3}}"#, DeskConfig { opacity: OPACITY_MIN, ..d.clone() }),
        (r#"{"desk":{"opacity":1e308}}"#, d.clone()),
        (r#"{"desk":{"opacity":64.6,"layer":"top"}}"#, DeskConfig { opacity: 65, layer: Layer::Top, ..d.clone() }),
    ];
    for (json, want) in cases {
        let c: crate::config::Config = serde_json::from_str(json).unwrap_or_else(|e| panic!("{json}: {e}"));
        assert_eq!(c.desk, want, "{json}");
    }
}

#[test]
fn file_missing_broken_and_old_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join(FILE);
    assert_eq!(load(&path), DeskFile::default());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // neznámý druh, špatné id, duplicita id i druhu, nesmyslné velikosti, jen x bez y, neznámé pole
    std::fs::write(
        &path,
        r#"{"widgets":[
            {"id":"lim1","kind":"limits","x":10,"y":20,"w":5,"h":99999,"opacity":3,"layer":"top"},
            {"id":"lim2","kind":"limits"},
            {"id":"lim1","kind":"dev"},
            {"id":"BAD","kind":"agenda"},
            {"id":"x1","kind":"pocasi"},
            {"id":"dev1","kind":"dev","x":7}
          ],
          "profiles":[{"name":"  Práce  ","widgets":[{"id":"cal1","kind":"agenda"}]},{"name":"práce"},{"name":"   "}],
          "active":"Práce","neznamy":1}"#,
    )
    .unwrap();
    let f = load(&path);
    assert_eq!(f.widgets.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(), ["lim1", "dev1"]);
    let a = &f.widgets[0];
    assert_eq!((a.x, a.y, a.w, a.h, a.opacity, a.layer), (Some(10), Some(20), MIN_W, MAX_H, OPACITY_MIN, Layer::Top));
    // chybějící volby = výchozí: schovat ve hře zapnuto
    assert!(a.hide_in_game && !a.locked && !a.through && !a.compact);
    assert_eq!((f.widgets[1].x, f.widgets[1].y), (None, None), "jen x bez y = neumístěný");
    assert_eq!(f.profiles.len(), 1, "duplicitní a prázdné jméno pryč");
    assert_eq!(f.profiles[0].name, "Práce");
    assert_eq!(f.active.as_deref(), Some("Práce"));
    // zápis a zpět beze ztráty
    save(&path, &f).unwrap();
    assert_eq!(load(&path), f);
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"hideInGame\": true") && raw.contains("\"layer\": \"top\""), "{raw}");
    std::fs::write(&path, "{rozbité").unwrap();
    assert_eq!(load(&path), DeskFile::default());
    // víc widgetů než strop → jen prvních MAX_WIDGETS
    let many: Vec<DeskWidget> = (1..=MAX_WIDGETS + 3).map(|i| w(&format!("wg{i}"), &format!("widget:s{i}.ps1"))).collect();
    let f = parse_clean(DeskFile { widgets: many, ..Default::default() });
    assert_eq!(f.widgets.len(), MAX_WIDGETS);
}

#[test]
fn add_existing_full_and_remove() {
    let d = DeskConfig { layer: Layer::Desktop, opacity: 70, ..DeskConfig::default() };
    let mut f = DeskFile::default();
    let (id, new) = f.add(&Kind::Limits, &d).unwrap();
    assert!(new && id == "lim1");
    let lim = f.by_id(&id).unwrap();
    assert_eq!((lim.layer, lim.opacity, lim.hide_in_game, lim.x), (Layer::Desktop, 70, true, None), "výchozí z configu");
    // stejný druh podruhé = tentýž widget
    assert_eq!(f.add(&Kind::Limits, &d), Ok(("lim1".into(), false)));
    for i in 0..MAX_WIDGETS - 1 {
        f.add(&Kind::Script(format!("widget:s{i}.cmd")), &d).unwrap();
    }
    assert_eq!(f.widgets.len(), MAX_WIDGETS);
    assert_eq!(f.add(&Kind::Dev, &d), Err(AddErr::Full));
    // i plný vrátí existující
    assert_eq!(f.add(&Kind::Limits, &d), Ok(("lim1".into(), false)));
    assert!(f.remove("lim1") && !f.remove("lim1"));
    assert!(f.add(&Kind::Dev, &d).unwrap().1);
}

#[test]
fn patch_table() {
    let base = DeskWidget::new("lim1", &Kind::Limits, &DeskConfig::default());
    type Check<'a> = Box<dyn Fn(&DeskWidget) -> bool + 'a>;
    let cases: &[(&str, &str, Check)] = &[
        ("prázdný", "{}", Box::new(|x| *x == base)),
        ("zámek", r#"{"locked":true}"#, Box::new(|x| x.locked && !x.through)),
        ("průchozí + navrchu", r#"{"through":true,"layer":"top"}"#, Box::new(|x| x.through && x.layer == Layer::Top)),
        ("průhlednost pod mez", r#"{"opacity":0}"#, Box::new(|x| x.opacity == OPACITY_MIN)),
        ("průhlednost nad mez", r#"{"opacity":250}"#, Box::new(|x| x.opacity == 100)),
        ("kompaktní", r#"{"compact":true}"#, Box::new(|x| x.compact && x.size_now() == (x.cw, x.ch))),
        ("hra", r#"{"hideInGame":false}"#, Box::new(|x| !x.hide_in_game)),
    ];
    for (name, json, ok) in cases {
        let p: Patch = serde_json::from_str(json).unwrap();
        let out = p.apply(&base);
        assert!(ok(&out), "{name}: {out:?}");
        assert_eq!((out.id.as_str(), out.x, out.w), ("lim1", base.x, base.w), "{name}: geometrie a id beze změny");
    }
}

#[test]
fn profiles_save_apply_delete() {
    let d = DeskConfig::default();
    let mut f = DeskFile::default();
    f.add(&Kind::Limits, &d).unwrap();
    f.add(&Kind::Dev, &d).unwrap();
    f.by_id_mut("lim1").unwrap().x = Some(100);
    f.by_id_mut("lim1").unwrap().y = Some(50);
    assert_eq!(f.save_profile("  Práce "), Ok("Práce".into()));
    assert_eq!(f.save_profile("   "), Err(AddErr::Unknown));
    // jiné rozmístění: limity jinde, dev pryč, agenda nová
    f.by_id_mut("lim1").unwrap().x = Some(900);
    f.remove("dev1");
    f.add(&Kind::Agenda, &d).unwrap();
    f.save_profile("Hraní").unwrap();
    assert_eq!(f.profiles.len(), 2);
    let diff = f.apply_profile("práce").unwrap();
    assert_eq!(diff, Diff { close: vec!["cal1".into()], keep: vec!["lim1".into()], open: vec!["dev1".into()] });
    assert_eq!(f.by_id("lim1").unwrap().x, Some(100), "poloha z profilu");
    assert_eq!(f.active.as_deref(), Some("Práce"));
    assert!(f.apply_profile("Neexistuje").is_none());
    // přepsat stejné jméno (bez ohledu na velikost písmen) = žádný nový profil
    f.save_profile("PRÁCE").unwrap();
    assert_eq!(f.profiles.len(), 2);
    assert!(f.delete_profile("PRÁCE") && f.active.is_none());
    for i in 0..MAX_PROFILES {
        let _ = f.save_profile(&format!("P{i}"));
    }
    assert_eq!(f.profiles.len(), MAX_PROFILES);
    assert_eq!(f.save_profile("Další"), Err(AddErr::Full));
    // nové id nekoliduje s widgetem, který je jen v profilu
    let mut g = DeskFile { profiles: vec![Profile { name: "A".into(), widgets: vec![w("dev1", "dev")] }], ..Default::default() };
    assert_eq!(g.add(&Kind::Dev, &d).unwrap().0, "dev2");
}

#[test]
fn visibility_in_game_table() {
    // (vrstva, schovat ve hře, hra, schovat vše) → (vidět, navrchu)
    let cases = [
        (Layer::Top, true, false, false, true, true),
        (Layer::Top, true, true, false, false, false),
        (Layer::Top, false, true, false, true, false),
        (Layer::Normal, true, false, false, true, false),
        (Layer::Normal, false, true, false, true, false),
        (Layer::Desktop, true, true, false, false, false),
        (Layer::Desktop, false, false, false, true, false),
        (Layer::Top, false, false, true, false, true),
        (Layer::Normal, false, true, true, false, false),
    ];
    for (layer, hide, game, all, want_shown, want_top) in cases {
        let x = DeskWidget { layer, hide_in_game: hide, ..w("lim1", "limits") };
        assert_eq!(shown(&x, game, all), want_shown, "{layer:?} schovat {hide} hra {game} vše {all}");
        assert_eq!(on_top(&x, game), want_top, "{layer:?} hra {game}");
    }
    assert!(movable(&w("a1", "dev")));
    assert!(!movable(&DeskWidget { locked: true, ..w("a1", "dev") }));
    assert!(!movable(&DeskWidget { through: true, ..w("a1", "dev") }));
}

#[test]
fn place_home_and_disconnected_monitor() {
    let ms = mons();
    let fb = ms[0];
    let at = |x: i32, y: i32| DeskWidget { x: Some(x), y: Some(y), ..w("lim1", "limits") };
    let fresh = w("lim1", "limits");
    let new_place = pins::place_new(&fresh.as_pin(), &fb, 40, 1);
    // (widget, monitory, kam, mimo domov?)
    type Case<'a> = (DeskWidget, Vec<Mon>, Place, bool, &'a str);
    let cases: Vec<Case> = vec![
        (at(100, 100), ms.clone(), Place { x: 100, y: 100, w: 320, h: 118 }, false, "domov na prvním monitoru"),
        (at(2000, 50), ms.clone(), Place { x: 2000, y: 50, w: 480, h: 177 }, false, "druhý monitor 150 %: logická velikost × DPI"),
        (at(2000, 50), vec![ms[0]], new_place, true, "druhý monitor odpojený → monitor proužku, domov zůstane"),
        (fresh.clone(), ms.clone(), new_place, false, "neumístěný = nový, není mimo domov"),
        (DeskWidget { compact: true, ..at(100, 100) }, ms.clone(), Place { x: 100, y: 100, w: 230, h: 72 }, false, "kompaktní velikost"),
    ];
    for (x, m, want, displaced, why) in cases {
        assert_eq!(plan_place(&x, &m, &fb, 40, 1), (want, displaced), "{why}");
    }
    // po připojení monitoru je domov zase na obrazovce
    assert!(home_on_screen(&at(2000, 50), &ms, 1.0));
    assert!(!home_on_screen(&at(2000, 50), &ms[..1], 1.0));
    assert!(!home_on_screen(&fresh, &ms, 1.0));
}

#[test]
fn snap_table() {
    let others = [Area { l: 500, t: 100, r: 800, b: 300 }];
    // (x, y, w, h) → kam
    type Rect = (i32, i32, i32, i32);
    let cases: &[(Rect, (i32, i32), &str)] = &[
        ((8, 9, 200, 100), (0, 0), "levý horní roh plochy"),
        ((30, 30, 200, 100), (30, 30), "daleko od okraje = beze změny"),
        ((1715, 935, 200, 100), (1720, 940), "pravý dolní roh: hrana okna k hraně plochy"),
        ((290, 150, 200, 100), (300, 150), "pravá hrana k levé hraně souseda"),
        ((810, 120, 200, 100), (800, 120), "levá hrana k pravé hraně souseda"),
        ((290, 600, 200, 100), (290, 600), "soused v jiné výšce se nepočítá"),
    ];
    for (r, want, why) in cases {
        assert_eq!(snap(*r, FHD, &others, 14), *want, "{why}");
    }
}
