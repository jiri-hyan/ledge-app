//! Vlastní zavřený Ledge (`compose`, kolo 23): ořez configu, položky × styly, předvolby × stavy, rozměry
//! a místo každého tvaru, config z 0.9.0 beze změny a zrcadlo id v `src/strip/compose.ts`.

use super::compose::{self, Compose, Form, PieceItem, Piece, PieceStyle};
use super::tests::{STATES, facts, parts_sig, sig};
use super::*;
use crate::appearance::StripPosition;
use crate::appearance::geom::anchor_c2;
use windows::Win32::Foundation::RECT;

fn pc(item: PieceItem, style: PieceStyle) -> Piece {
    Piece { item, style }
}
fn cmp(shape: Form, left: &[Piece], center: &[Piece], right: &[Piece]) -> Compose {
    Compose { shape, left: left.to_vec(), center: center.to_vec(), right: right.to_vec(), ..Compose::default() }
}

/// (vstup, šířka, výška, zaoblení, počty vlevo / střed / vpravo)
type NormRow = (Compose, u16, u16, u8, (usize, usize, usize));

#[test]
fn norm_clamps_and_limits() {
    use PieceItem::*;
    let i = PieceStyle::Icon;
    // (vstup, šířka, výška, zaoblení, počty vlevo/střed/vpravo)
    let table: Vec<NormRow> = vec![
        (Compose::default(), 300, 30, 55, (1, 1, 1)),
        (Compose { width: 5, height: 200, round: 250, ..Compose::default() }, 60, 52, 100, (1, 1, 1)),
        (Compose { width: 9000, height: 0, ..Compose::default() }, 620, 18, 55, (1, 1, 1)),
        // linka: tloušťka 2–10
        (Compose { shape: Form::Line, height: 30, ..Compose::default() }, 300, 10, 55, (1, 1, 1)),
        (Compose { shape: Form::Line, height: 0, ..Compose::default() }, 300, 2, 55, (1, 1, 1)),
        // nejvýš 5, zleva doprava; každá položka jednou
        (cmp(Form::Capsule, &[pc(Time, i), pc(Date, i), pc(Cpu, i)], &[pc(Claude, i), pc(Time, i), pc(Music, i)], &[pc(Timer, i), pc(Repos, i)]), 300, 30, 55, (3, 2, 0)),
        // prázdné = stav Clauda uprostřed (1–5 položek)
        (cmp(Form::Capsule, &[], &[], &[]), 300, 30, 55, (0, 1, 0)),
    ];
    for (c, w, h, r, n) in table {
        let x = c.norm();
        assert_eq!((x.width, x.height, x.round, (x.left.len(), x.center.len(), x.right.len())), (w, h, r, n), "{c:?}");
        let total = x.left.len() + x.center.len() + x.right.len();
        assert!((1..=compose::MAX_PIECES).contains(&total));
        assert_eq!(x.norm(), x, "ořez je idempotentní");
    }
    assert_eq!(cmp(Form::Capsule, &[], &[], &[]).norm().center, vec![pc(Claude, PieceStyle::Icon)]);
}

#[test]
fn items_x_styles_real_data_only() {
    use PieceItem::*;
    use PieceStyle::*;
    let work = facts(State::Work);
    let mut full = facts(State::Work);
    full.limit7d = Some(86.0);
    full.cpu = Some(23.0);
    full.mem = Some(61.0);
    full.battery = Some(BatteryFact { percent: 82, charging: false });
    full.weather = Some(WeatherFact { temp: 21, glyph: Glyph::Sun });
    full.servers = 3;
    // (položka, styl, fakta, podpis)
    let table: Vec<(PieceItem, PieceStyle, &Facts, &str)> = vec![
        (Time, Text, &work, "14:32"),
        (Time, Icon, &work, "[clock:text]"),
        (Date, Text, &work, "ne 18. 10."),
        (Claude, Icon, &work, "[claude:run]"),
        (Claude, Badge, &work, "[claude:run 2]"),
        (Claude, Text, &work, "Pracuje · 2"),
        (Claude, Dot, &work, "●run"),
        (Claude, Ring, &work, "◯:run"),
        (Limit5h, Ring, &work, "◯42:acc"),
        (Limit5h, Text, &work, "5h 42 %"),
        (Limit5h, Graph, &work, "▮42:acc"),
        (Limit5h, Badge, &work, "5h 42 %"),
        (LimitWeek, Ring, &full, "◯86:wait"),
        // bez dat: prstenec limitu = prázdná dráha, ostatní nic (žádné vymyšlené %)
        (LimitWeek, Ring, &work, "◯:idle"),
        (LimitWeek, Text, &work, ""),
        (Cpu, Text, &work, ""),
        (Cpu, Text, &full, "CPU 23 %"),
        (Cpu, Graph, &full, "▮23:text"),
        (Memory, Badge, &full, "[cpu:text 61]"),
        (Battery, Icon, &full, "▭82"),
        (Battery, Badge, &full, "▭82 82 %"),
        (Battery, Icon, &work, ""),
        (Weather, Text, &full, "21°"),
        (Weather, Icon, &full, "[sun:text]"),
        (Weather, Ring, &full, "[sun:text]"),
        (Music, Icon, &work, ""),
        (Timer, Badge, &work, ""),
        (Servers, Badge, &full, "[server:ok 3]"),
        (Servers, Icon, &work, "[server:dim]"),
        (Servers, Text, &work, ""),
        (Repos, Badge, &work, "[repo:dim]"),
    ];
    for (item, style, f, want) in table {
        assert_eq!(parts_sig(&compose::parts(pc(item, style), f)), want, "{item:?} {style:?}");
    }
    let m = facts(State::Music);
    assert_eq!(parts_sig(&compose::parts(pc(Music, Graph), &m)), "▮42:music");
    let t = facts(State::Timer);
    assert_eq!(parts_sig(&compose::parts(pc(Timer, Badge), &t)), "[timer:acc 18]");
    let w = facts(State::Wait);
    assert_eq!(parts_sig(&compose::parts(pc(Claude, Text), &w)), "*Čeká na tebe · ledge*");
}

#[test]
fn compose_x_6_states() {
    use PieceItem::*;
    use PieceStyle::*;
    let classic = Compose::default();
    let ghost = Compose { hide_idle: true, ..cmp(Form::Island, &[], &[pc(Claude, Text)], &[]) };
    let ticker = Compose { rotate: true, ..cmp(Form::Island, &[], &[pc(Claude, Text), pc(Music, Text), pc(Timer, Text)], &[]) };
    let line = cmp(Form::Line, &[pc(Claude, Dot)], &[pc(Limit5h, Dot)], &[pc(Music, Dot)]);
    // (compose, [klid, pracuje, čeká, hudba, časovač, hotovo])
    let table: Vec<(&Compose, [&str; 6])> = vec![
        (
            &classic,
            [
                "[claude:dim] ‖ 14:32 ⇥ ◯42:acc",
                "[claude:run] ‖ 14:32 ⇥ ◯42:acc",
                "[claude:wait] ‖ 14:32 ⇥ ◯42:acc",
                "[claude:dim] ‖ 14:32 ⇥ ◯42:acc",
                "[claude:dim] ‖ 14:32 ⇥ ◯42:acc",
                "[check:ok] ‖ 14:32 ⇥ ◯42:acc",
            ],
        ),
        (&ghost, ["∅", "∅", " ‖ *Čeká na tebe · ledge*", "∅", "∅", " ‖ Hotovo: ledge"]),
        (&ticker, [" ‖ Klid", " ‖ Pracuje · 2", " ‖ *Čeká na tebe · ledge*", " ‖ Midnight City · M83", " ‖ 18 min", " ‖ Hotovo: ledge"]),
        (
            &line,
            ["line idle1.0 acc1.0 idle1.0", "line run1.0 acc1.0 idle1.0", "line wait1.0 acc1.0 idle1.0", "line idle1.0 acc1.0 music1.0", "line idle1.0 acc1.0 idle1.0", "line ok1.0 acc1.0 idle1.0"],
        ),
    ];
    for (c, want) in &table {
        for (st, w) in STATES.iter().zip(want) {
            assert_eq!(sig(&compose::show(c, &facts(*st))), *w, "{:?} {st:?}", c.shape);
        }
    }
    // střídání jen u víc událostí (v klidu žádný časovač)
    let mut f = facts(State::Work);
    f.music = Some(super::tests::music());
    let s = compose::show(&ticker, &f);
    assert_eq!((s.rotate.len(), sig(&s)), (2, " ‖ Pracuje · 2 ⟳2".to_string()));
    assert!(compose::show(&ticker, &facts(State::Idle)).rotate.is_empty());
    // prstenec se točí jen při práci Clauda
    assert!(compose::show(&classic, &facts(State::Work)).spin);
    for st in [State::Idle, State::Wait, State::Music, State::Done] {
        assert!(!compose::show(&classic, &facts(st)).spin, "{st:?}");
    }
    // klid ví, že je v klidu
    assert!(compose::show(&classic, &facts(State::Idle)).idle);
}

#[test]
fn spec_and_place_every_form() {
    let mon = RECT { left: -1920, top: 0, right: 0, bottom: 1080 };
    let axis2 = anchor_c2(mon.left, mon.right, 1080, StripPosition::Center, 96);
    // (tvar, umístí se jako, pill, ouška > 0, odstup shora, rádius)
    let table = [
        (Form::Capsule, Look::Custom, false, false, 0.0, 8.25),
        (Form::Drop, Look::Custom, false, false, 0.0, 16.5),
        (Form::Notch, Look::Custom, false, true, 0.0, 8.25),
        (Form::Line, Look::Line, false, false, 0.0, 2.75),
        (Form::Corner, Look::Corner, true, false, 8.0, 8.25),
        (Form::Island, Look::Custom, true, false, frame::ISLAND_GAP, 8.25),
    ];
    for (shape, at, pill, ears, tg, r) in table {
        let c = Compose { shape, ..Compose::default() };
        let sp = compose::spec(&c);
        assert_eq!((c.place_look(), sp.pill, sp.ears > 0.0, sp.top_gap, sp.r), (at, pill, ears, tg, r), "{shape:?}");
        let h = if shape == Form::Line { 10.0 } else { 30.0 };
        assert_eq!(sp.h, h, "{shape:?} výška z configu (ořez)");
        for pos in [StripPosition::Left, StripPosition::Center, StripPosition::Right] {
            let bw = frame::body_width(at, 40.0, &sp, 1.0, mon);
            let p = frame::place(at, pos, mon, axis2, bw, sp.h as i32, &sp, 1.0);
            assert!(p.win.left >= mon.left && p.win.right <= mon.right && p.win.top == mon.top, "{shape:?} {pos:?}");
            if shape == Form::Line {
                assert_eq!(bw, 1920);
            } else {
                assert_eq!(bw, 300, "{shape:?}: šířka = strip.compose.width");
            }
            // hover zóna kryje skutečný obdélník
            let (l, rr, b) = frame::zone(p.win, axis2 / 2, mon, 96);
            assert!(l <= p.win.left && rr >= p.win.right && b >= p.win.bottom);
        }
    }
    // zaoblení 0 = ostré, prstenec a tečka mají vlastní průměr
    let sp = compose::spec(&Compose { round: 0, height: 40, ..Compose::default() });
    assert_eq!((sp.r, sp.ring, sp.min_w), (0.0, 24.0, 300.0));
    // ostatní styly prstenec ani rádius `pill` nemění (0 = dnešní chování)
    for look in Look::ALL {
        let s = frame::spec(look, &Show::default());
        assert_eq!((s.ring, s.pill_r), (0.0, 0.0), "{look:?}");
    }
}

#[test]
fn needs_only_with_item() {
    use PieceItem::*;
    let i = PieceStyle::Icon;
    let table = [
        (Compose::default(), (false, false, false)),
        (cmp(Form::Capsule, &[pc(Music, i)], &[], &[]), (true, false, false)),
        (cmp(Form::Capsule, &[pc(Battery, i)], &[pc(Cpu, i)], &[]), (false, true, true)),
        (cmp(Form::Capsule, &[], &[pc(Memory, i)], &[]), (false, false, true)),
    ];
    for (c, want) in table {
        assert_eq!(compose::needs(&c), want, "{c:?}");
    }
    let mut cfg = crate::config::Config::default();
    cfg.strip.compose = cmp(Form::Capsule, &[pc(Cpu, i)], &[pc(Music, i)], &[]);
    // položky v compose nic nespustí, dokud vlastní vzhled není zapnutý
    assert!(!crate::integrations::wants(&cfg).system);
    assert!(!crate::media::strip_wants(&cfg));
    cfg.strip.look = Look::Custom;
    assert!(crate::integrations::wants(&cfg).system);
    assert!(crate::media::strip_wants(&cfg));
    assert!(!Look::Custom.shows_music() && !Look::Custom.shapeable() && Look::Custom.retreats() && !Look::Custom.full_width());
}

#[test]
fn config_090_loads_unchanged_custom_only_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.json");
    // tvar configu z 0.9.0 (kapsle kola 22, tvar kapsle) — bez `compose`
    let v090 = r##"{
      "appearance": { "style": "classic", "theme": "dark", "strip": { "style": "pill", "size": "normal", "position": "center" } },
      "strip": { "enabled": true, "items": ["clock", "mode", "waiting"], "itemsRev": 2, "look": "rings", "lookShape": "island", "lookHide": [] }
    }"##;
    std::fs::write(&p, v090).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert!(crate::config::validate(&cfg).is_ok());
    assert_eq!((cfg.strip.look, cfg.strip.look_shape), (Look::Rings, Shape::Island));
    assert_eq!(cfg.strip.compose, Compose::default());
    // výchozí config: klasický, nic vlastního se samo nezapne
    assert_eq!(crate::config::Config::default().strip.look, Look::Classic);
    // compose bez look custom vzhled nemění
    std::fs::write(&p, r#"{"strip":{"look":"island","compose":{"shape":"drop","width":200}}}"#).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert_eq!((cfg.strip.look, cfg.strip.compose.shape, cfg.strip.compose.height), (Look::Island, Form::Drop, 30));
    // zapnutý výslovně; mimo rozsah se načte a ořízne až při použití
    let json = r#"{"strip":{"look":"custom","compose":{"shape":"island","width":9999,"height":1,"round":200,
      "left":[{"item":"limit5h","style":"ring"}],"center":[{"item":"time","style":"text"}],"right":[{"item":"limitWeek","style":"graph"}],"hideIdle":true}}}"#;
    std::fs::write(&p, json).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert!(crate::config::validate(&cfg).is_ok());
    let n = cfg.strip.compose.norm();
    assert_eq!((cfg.strip.look, n.width, n.height, n.round, n.hide_idle), (Look::Custom, compose::W_MAX, compose::H_MIN, 100, true));
    let back = serde_json::to_value(&cfg.strip).unwrap();
    assert_eq!((back["look"].as_str(), back["compose"]["right"][0]["item"].as_str(), back["compose"]["hideIdle"].as_bool()), (Some("custom"), Some("limitWeek"), Some(true)));
    // neznámá položka = chyba jako jiné volby (ne tiché zahození)
    assert!(serde_json::from_str::<crate::config::StripConfig>(r#"{"compose":{"left":[{"item":"mic"}]}}"#).is_err());
}

#[test]
fn ts_mirror() {
    // seznamy pro editor v Nastavení (`src/strip/compose.ts`) mají stejná id a rozsahy
    let ts = include_str!("../../../src/strip/compose.ts");
    let ids = |v: Vec<String>| v.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ");
    let ser = |x: serde_json::Value| x.as_str().unwrap().to_string();
    let forms = [Form::Capsule, Form::Notch, Form::Line, Form::Corner, Form::Drop, Form::Island];
    let items = [
        PieceItem::Time, PieceItem::Date, PieceItem::Claude, PieceItem::Limit5h, PieceItem::LimitWeek, PieceItem::Music, PieceItem::Timer,
        PieceItem::Cpu, PieceItem::Memory, PieceItem::Battery, PieceItem::Weather, PieceItem::Servers, PieceItem::Repos,
        PieceItem::Activity,
    ];
    let styles = [PieceStyle::Icon, PieceStyle::Badge, PieceStyle::Text, PieceStyle::Ring, PieceStyle::Graph, PieceStyle::Dot];
    let f = ids(forms.iter().map(|x| ser(serde_json::to_value(x).unwrap())).collect());
    let i = ids(items.iter().map(|x| ser(serde_json::to_value(x).unwrap())).collect());
    let s = ids(styles.iter().map(|x| ser(serde_json::to_value(x).unwrap())).collect());
    assert!(ts.contains(&format!("COMPOSE_SHAPES = [{f}]")), "tvary: {f}");
    assert!(ts.contains(&format!("COMPOSE_ITEMS = [{i}]")), "položky: {i}");
    assert!(ts.contains(&format!("ITEM_STYLES = [{s}]")), "styly: {s}");
    for (k, v) in [
        ("MAX_PIECES", compose::MAX_PIECES as u16),
        ("W_MIN", compose::W_MIN),
        ("W_MAX", compose::W_MAX),
        ("H_MIN", compose::H_MIN),
        ("H_MAX", compose::H_MAX),
        ("LINE_H_MIN", compose::LINE_H_MIN),
        ("LINE_H_MAX", compose::LINE_H_MAX),
    ] {
        assert!(ts.contains(&format!("export const {k} = {v};")), "{k} = {v}");
    }
    // výchozí compose = předvolba „Klasický“ v editoru
    assert!(ts.contains(r#"classic: mk("capsule", 300, 30, 55, ["claude:icon"], ["time:text"], ["limit5h:ring"])"#));
}
