//! Položka skládačky „Claude aktivita“ (kolo 23): fáze × styly × tvary, přednost čekajícího schválení,
//! prázdný stav bez sessions, starý config beze změny a zrcadlo id v `src/strip/compose.ts`.

use super::compose::{self, Compose, Form, Piece, PieceItem, PieceStyle};
use super::facts::ActivityFact;
use super::tests::{facts, parts_sig, sig};
use super::*;
use crate::claude::activity::Phase;

const FORMS: [Form; 6] = [Form::Capsule, Form::Notch, Form::Line, Form::Corner, Form::Drop, Form::Island];
const STYLES: [PieceStyle; 6] = [PieceStyle::Icon, PieceStyle::Badge, PieceStyle::Text, PieceStyle::Ring, PieceStyle::Graph, PieceStyle::Dot];

fn act(phase: Phase, file: &str) -> Facts {
    let mut f = facts(State::Work);
    f.activity = Some(ActivityFact { phase, file: file.into(), project: "app".into() });
    f
}
fn one(shape: Form, style: PieceStyle) -> Compose {
    Compose { shape, left: vec![], center: vec![Piece { item: PieceItem::Activity, style }], right: vec![], ..Compose::default() }
}

#[test]
fn phases_text_color_and_step() {
    // (fáze, soubor, text, podíl: nikdy (fáze je odhad))
    let table: &[(Phase, &str, &str, Option<f32>)] = &[
        (Phase::Reading, "main.rs", "Čtení · main.rs", None),
        (Phase::Editing, "main.rs", "Úpravy · main.rs", None),
        (Phase::Testing, "", "Testy · app", None),
        (Phase::Done, "", "Hotovo · app", None),
        (Phase::Working, "", "Pracuje · app", None),
        (Phase::Error, "lib.rs", "Chyba · lib.rs", None),
    ];
    for (phase, file, text, frac) in table {
        let v = compose::value(PieceItem::Activity, &act(*phase, file)).unwrap();
        assert_eq!((v.text.as_str(), v.frac, v.event, v.num.as_deref()), (*text, *frac, true, None), "{phase:?}");
        let want = match phase {
            Phase::Done => OK,
            Phase::Error => crate::strip::Rgba::RED,
            _ => RUN,
        };
        assert_eq!(v.color, want, "{phase:?}");
    }
    // bez projektu i souboru jen fáze
    let mut f = act(Phase::Editing, "");
    f.activity.as_mut().unwrap().project.clear();
    assert_eq!(compose::value(PieceItem::Activity, &f).unwrap().text, "Úpravy");
}

#[test]
fn waiting_approval_wins_with_badge() {
    let mut f = act(Phase::Editing, "main.rs");
    f.waiting = 2;
    f.waiting_where = "app".into();
    let v = compose::value(PieceItem::Activity, &f).unwrap();
    assert_eq!((v.color, v.num.as_deref(), v.glyph), (WAIT, Some("2"), Some(Glyph::Bell)));
    let badge = parts_sig(&compose::parts(Piece { item: PieceItem::Activity, style: PieceStyle::Badge }, &f));
    assert!(badge.ends_with(" 2]"), "odznak s počtem: {badge}");
    // čeká i bez aktivity (schválení z hooku přišlo dřív než první nástroj)
    let mut f = facts(State::Wait);
    f.activity = None;
    assert!(compose::value(PieceItem::Activity, &f).is_some_and(|v| v.color == WAIT));
}

#[test]
fn empty_without_sessions_every_style_and_shape() {
    // žádná session = žádná aktivita a nic nečeká → položka se nekreslí (prstenec bez dat = nic, ne prázdná dráha)
    let f = facts(State::Idle);
    assert_eq!(f.activity, None);
    assert_eq!(compose::value(PieceItem::Activity, &f), None);
    for style in STYLES {
        assert!(compose::parts(Piece { item: PieceItem::Activity, style }, &f).is_empty(), "{style:?}");
    }
    for shape in FORMS {
        let s = compose::show(&one(shape, PieceStyle::Text), &f);
        if shape == Form::Line {
            assert!(s.line.iter().all(|(_, c)| *c == IDLE), "linka v klidu šedá");
        } else {
            assert!(s.center.is_empty(), "{shape:?}: {}", sig(&s));
        }
    }
}

#[test]
fn phases_by_shapes_and_styles() {
    for phase in [Phase::Reading, Phase::Editing, Phase::Testing, Phase::Working, Phase::Done, Phase::Error] {
        let f = act(phase, "main.rs");
        for shape in FORMS {
            for style in STYLES {
                let s = compose::show(&one(shape, style), &f);
                assert!(!s.hidden, "{phase:?} {shape:?} {style:?}");
                if shape == Form::Line {
                    assert_ne!(s.line[0].1, IDLE, "{phase:?}: úsek linky má barvu fáze");
                    continue;
                }
                let got = parts_sig(&s.center);
                let ok = match style {
                    PieceStyle::Text => got.contains("main.rs"),
                    // fáze je odhad: prstenec ani graf z ní se nekreslí, spadne na ikonu
                    PieceStyle::Ring | PieceStyle::Graph => got.starts_with('[') && !got.contains('◯') && !got.contains('▮'),
                    PieceStyle::Dot => got.starts_with('●'),
                    _ => got.starts_with('['),
                };
                assert!(ok, "{phase:?} {shape:?} {style:?}: {got}");
            }
        }
    }
}

#[test]
fn hidden_by_look_hide_claude() {
    let f = without(act(Phase::Editing, "main.rs"), &[crate::config::StripItem::Claude]);
    assert_eq!(f.activity, None);
}

#[test]
fn old_config_unchanged_and_existing_items_ignore_activity() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.json");
    std::fs::write(&p, r#"{"strip":{"look":"custom","compose":{"center":[{"item":"claude","style":"text"}]}}}"#).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert!(!cfg.strip.compose.has(PieceItem::Activity));
    assert!(!Compose::default().has(PieceItem::Activity), "výchozí skládačka aktivitu nemá");
    assert!(!crate::layout::placed(&crate::config::Config::default().layout, crate::layout::ACTIVITY_CARD), "karta jen z katalogu");
    // stará položka „Stav Clauda“ ukazuje totéž s aktivitou i bez ní
    let plain = facts(State::Work);
    let with = act(Phase::Editing, "main.rs");
    for style in STYLES {
        let pc = Piece { item: PieceItem::Claude, style };
        assert_eq!(parts_sig(&compose::parts(pc, &plain)), parts_sig(&compose::parts(pc, &with)), "{style:?}");
    }
    // nová položka se načte ze zápisu a přežije ořez
    std::fs::write(&p, r#"{"strip":{"look":"custom","compose":{"center":[{"item":"activity","style":"badge"}]}}}"#).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert!(cfg.strip.compose.norm().has(PieceItem::Activity));
}
