//! Tabulky: 11 stylů × 6 stavů („stav → co styl ukáže“), priorita a střídání, Pokročilé (`without`),
//! převody faktů, config a zrcadlo seznamu v TS, geometrie a hover zóny každého tvaru.

use super::frame::{self, ISLAND_MS, WidthAnim};
use super::*;
use crate::appearance::StripPosition;
use crate::appearance::geom::{anchor_c2, span_x};
use windows::Win32::Foundation::RECT;

pub(super) const ACC: Rgba = Rgba::VIOLET;

pub(super) fn col(c: Rgba) -> &'static str {
    let table = [
        (RUN, "run"),
        (WAIT, "wait"),
        (OK, "ok"),
        (MUSIC, "music"),
        (IDLE, "idle"),
        (ACC, "acc"),
        (Rgba::DIM, "dim"),
        (Rgba::TEXT, "text"),
        (Rgba::RED, "red"),
        (BAR_TEXT, "bar"),
    ];
    table.iter().find(|(x, _)| *x == c).map(|(_, n)| *n).unwrap_or("?")
}

fn glyph(g: Glyph) -> &'static str {
    match g {
        Glyph::Claude => "claude",
        Glyph::Server => "server",
        Glyph::Repo => "repo",
        Glyph::Music => "music",
        Glyph::Timer => "timer",
        Glyph::Bell => "bell",
        Glyph::Check => "check",
        Glyph::Sun => "sun",
        Glyph::Moon => "moon",
        Glyph::Cloud => "cloud",
        Glyph::Clock => "clock",
        Glyph::Calendar => "cal",
        Glyph::Cpu => "cpu",
    }
}

pub(super) fn parts_sig(v: &[Part]) -> String {
    v.iter()
        .map(|p| match p {
            Part::Text { text, strong: true, .. } => format!("*{text}*"),
            Part::Text { text, .. } => text.clone(),
            Part::Dot(c) => format!("●{}", col(*c)),
            Part::Icon { glyph: g, color, badge: None } => format!("[{}:{}]", glyph(*g), col(*color)),
            Part::Icon { glyph: g, color, badge: Some(b) } => format!("[{}:{} {b}]", glyph(*g), col(*color)),
            Part::Sep => "|".into(),
            Part::Battery { percent, .. } => format!("▭{percent}"),
            Part::Ring { fill, color } => format!("◯{}:{}", fill.map(|x| format!("{:.0}", x * 100.0)).unwrap_or_default(), col(*color)),
            Part::Meter { fill, color } => format!("▮{:.0}:{}", fill * 100.0, col(*color)),
            Part::Spark { pts, color } => format!("~{}:{}", pts.len(), col(*color)),
            Part::Under { text, color, fill } => format!("_{text}_{:.0}:{}", fill * 100.0, col(*color)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Čitelný podpis toho, co styl ukáže (tabulka níž).
pub(super) fn sig(s: &Show) -> String {
    if s.hidden {
        return "∅".into();
    }
    if !s.line.is_empty() {
        return format!("line {}", s.line.iter().map(|(w, c)| format!("{}{w:.1}", col(*c))).collect::<Vec<_>>().join(" "));
    }
    let mut out = parts_sig(&s.parts);
    if !s.center.is_empty() {
        out.push_str(&format!(" ‖ {}", parts_sig(&s.center)));
    }
    if !s.right.is_empty() {
        out.push_str(&format!(" ⇥ {}", parts_sig(&s.right)));
    }
    if let Some((p, c)) = s.progress {
        out.push_str(&format!(" ▭{:.0}%:{}", p * 100.0, col(c)));
    }
    if s.rotate.len() > 1 {
        out.push_str(&format!(" ⟳{}", s.rotate.len()));
    }
    out
}

fn base() -> Facts {
    Facts {
        clock: "14:32".into(),
        date: "14:32 · ne 18. 10.".into(),
        mode: "Práce".into(),
        limit5h: Some(42.4),
        accent: ACC,
        ..Facts::default()
    }
}

pub(super) fn music() -> MusicFact {
    MusicFact { text: "Midnight City · M83".into(), time: Some("1/4 min".into()), progress: Some(0.42) }
}
pub(super) fn timer18() -> TimerFact {
    TimerFact { text: "18 min".into(), minutes: Some(18), progress: Some(0.7), done: false }
}

/// Ukázkový stav (data jako v náhledu `ledge-w7-zavreny.html`).
pub(super) fn facts(st: State) -> Facts {
    let mut f = base();
    match st {
        State::Idle => {}
        State::Work => {
            f.working = 2;
            f.working_where = "ledge".into();
        }
        State::Wait => {
            f.waiting = 1;
            f.waiting_where = "ledge".into();
        }
        State::Music => f.music = Some(music()),
        State::Timer => f.timer = Some(timer18()),
        State::Done => {
            f.done = 1;
            f.done_where = "ledge".into();
        }
    }
    f
}

pub(super) const STATES: [State; 6] = [State::Idle, State::Work, State::Wait, State::Music, State::Timer, State::Done];

#[test]
fn table_11_looks_x_6_states() {
    // (styl, [klid, pracuje, čeká, hudba, časovač, hotovo])
    let r = " ⇥ 5h 42 % · 14:32";
    let table: [(Look, [String; 6]); 11] = [
        (
            Look::Island,
            [
                "●idle".into(),
                "●run Claude pracuje · 2".into(),
                "●wait *Čeká na tebe · ledge*".into(),
                "[music:music] Midnight City · M83".into(),
                "[timer:acc] 18 min".into(),
                "[check:ok] Hotovo: ledge".into(),
            ],
        ),
        (
            Look::Dots,
            [
                "●idle ●idle ●idle ●idle".into(),
                "●run ●idle ●idle ●idle".into(),
                "●wait ●idle ●idle ●idle".into(),
                "●idle ●idle ●idle ●music".into(),
                "●idle ●idle ●idle ●acc".into(),
                "●ok ●idle ●idle ●idle".into(),
            ],
        ),
        (
            Look::Line,
            [
                "line idle1.0".into(),
                "line run1.0 idle2.0".into(),
                "line wait1.0".into(),
                "line music1.3 idle1.7".into(),
                "line acc2.1 idle0.9".into(),
                "line ok1.0".into(),
            ],
        ),
        (
            Look::Icons,
            [
                "[claude:dim] [server:dim] [repo:dim]".into(),
                "[claude:run 2] [server:dim] [repo:dim]".into(),
                "[bell:wait 1] [claude:dim] [server:dim] [repo:dim]".into(),
                "[music:music] [claude:dim] [server:dim] [repo:dim]".into(),
                "[timer:acc 18] [claude:dim] [server:dim] [repo:dim]".into(),
                "[check:ok] [claude:dim] [server:dim] [repo:dim]".into(),
            ],
        ),
        (
            Look::Clock,
            [
                "*14:32*".into(),
                "●run *14:32*".into(),
                "●wait *14:32*".into(),
                "[music:music] *14:32*".into(),
                "[timer:acc] *14:32*".into(),
                "●ok *14:32*".into(),
            ],
        ),
        (
            Look::Corner,
            [
                "●idle 14:32".into(),
                "●run 2× pracuje".into(),
                "●wait *čeká 1*".into(),
                "[music:music] hraje".into(),
                "[timer:acc] 18 min".into(),
                "[check:ok] hotovo".into(),
            ],
        ),
        (
            Look::Ghost,
            ["∅".into(), "∅".into(), "●wait *Čeká na tebe · ledge*".into(), "∅".into(), "∅".into(), "[check:ok] Hotovo: ledge".into()],
        ),
        (
            Look::Bar,
            [
                format!("práce{r}"),
                format!("claude ▸ 2× pracuje{r}"),
                format!("*claude ▸ ČEKÁ 1*{r}"),
                format!("[music:music] midnight city · m83{r}"),
                format!("[timer:acc] 18 min{r}"),
                format!("[check:ok] hotovo: ledge{r}"),
            ],
        ),
        (
            Look::Notch,
            ["".into(), "●run 2".into(), "●wait *1 čeká*".into(), "[music:music]".into(), "[timer:acc] 18".into(), "[check:ok]".into()],
        ),
        (
            Look::Progress,
            [
                "Limit 5h ⇥ 42 % ▭42%:acc".into(),
                "Claude pracuje · 2 ⇥ 5h 42 % ▭42%:run".into(),
                "*Čeká na tebe · ledge*".into(),
                "Midnight City · M83 ⇥ 1/4 min ▭42%:music".into(),
                "Časovač ⇥ 18 min ▭70%:acc".into(),
                "Hotovo ⇥ ledge".into(),
            ],
        ),
        (
            Look::Ticker,
            [
                "14:32 · ne 18. 10.".into(),
                "●run Claude pracuje · 2".into(),
                "●wait *Čeká na tebe · ledge*".into(),
                "[music:music] Midnight City · M83".into(),
                "[timer:acc] 18 min".into(),
                "[check:ok] Hotovo: ledge".into(),
            ],
        ),
    ];
    // classic nic neskládá; minimum, trio a rings (kolo 22) mají vlastní tabulku v `capsule::tests`
    assert_eq!(table.len() + 5, Look::ALL.len(), "každý styl kromě classic, kapslí kola 22 a vlastního (`compose_tests`) má řádek");
    for (look, want) in &table {
        for (st, w) in STATES.iter().zip(want) {
            let f = facts(*st);
            assert_eq!(state_of(&f), *st);
            assert_eq!(sig(&show(*look, &f)), *w, "{look:?} {st:?}");
        }
    }
    // jen neviditelný styl se v klidu schová; ostrůvek ví, že je v klidu (malý)
    for look in Look::ALL {
        assert_eq!(show(look, &facts(State::Idle)).hidden, look == Look::Ghost, "{look:?}");
    }
    assert!(show(Look::Island, &facts(State::Idle)).idle);
    assert!(!show(Look::Island, &facts(State::Work)).idle);
    // klasický styl nic neskládá (kreslí ho `strip.items`)
    assert_eq!(show(Look::Classic, &facts(State::Wait)), Show { idle: false, ..Show::default() });
}

#[test]
fn priority_and_rotation() {
    let mut all = base();
    all.waiting = 2;
    all.waiting_where = "ledge".into();
    all.done = 1;
    all.timer = Some(timer18());
    all.working = 1;
    all.working_where = "ledge".into();
    all.music = Some(music());
    assert_eq!(events(&all), vec![State::Wait, State::Done, State::Timer, State::Work, State::Music]);
    assert_eq!(state_of(&all), State::Wait);
    assert_eq!(sig(&show(Look::Island, &all)), "●wait *Čeká na tebe · ledge +1*");
    // ticker: víc událostí = střídání (časovač jen tehdy), první řádek = nejvyšší priorita
    let t = show(Look::Ticker, &all);
    assert_eq!((t.rotate.len(), parts_sig(&t.parts)), (5, "●wait *Čeká na tebe · ledge +1*".to_string()));
    assert_eq!(parts_sig(&t.rotate[3]), "●run Claude pracuje · ledge");
    // (popis, upravení, kolik řádků se střídá)
    type Case = (&'static str, fn(&mut Facts), usize);
    let cases: [Case; 4] = [
        ("klid", |_| {}, 0),
        ("jedna událost: bez střídání", |f| f.working = 1, 0),
        ("dvě události", |f| {
            f.working = 1;
            f.music = Some(music());
        }, 2),
        ("připomínka + časovač", |f| {
            f.reminder = Some(("Zalít kytky".into(), 1));
            f.timer = Some(timer18());
        }, 2),
    ];
    for (name, set, n) in cases {
        let mut f = base();
        set(&mut f);
        assert_eq!(show(Look::Ticker, &f).rotate.len(), n, "{name}");
    }
    // pořadí: hotovo má přednost před časovačem i prací
    let mut f = base();
    f.done = 1;
    f.working = 3;
    f.timer = Some(timer18());
    assert_eq!(state_of(&f), State::Done);
}

#[test]
fn reminder_counts_as_waiting() {
    let mut f = base();
    f.reminder = Some(("Zalít kytky".into(), 2));
    assert_eq!(state_of(&f), State::Wait);
    let cases = [
        (Look::Island, "[bell:wait] *Zalít kytky +1*"),
        (Look::Ghost, "[bell:wait] *Zalít kytky +1*"),
        (Look::Corner, "[bell:wait] *připomínka*"),
        (Look::Dots, "●idle ●idle ●idle ●wait"),
        (Look::Notch, "[bell:wait]"),
        (Look::Icons, "[bell:wait] [claude:dim] [server:dim] [repo:dim]"),
    ];
    for (look, want) in cases {
        assert_eq!(sig(&show(look, &f)), want, "{look:?}");
    }
}

#[test]
fn only_real_data_no_progress_otherwise() {
    // bez dat limitu: Průběh v klidu ukáže hodiny bez lišty, Claude bez lišty; stopky lištu nemají
    let mut f = base();
    f.limit5h = None;
    assert_eq!(sig(&show(Look::Progress, &f)), "14:32");
    f.working = 1;
    assert_eq!(sig(&show(Look::Progress, &f)), "Claude pracuje");
    let mut f = base();
    f.timer = Some(TimerFact { text: "0:42".into(), minutes: None, progress: None, done: false });
    assert_eq!(sig(&show(Look::Progress, &f)), "Časovač ⇥ 0:42");
    assert_eq!(sig(&show(Look::Icons, &f)), "[timer:acc] [claude:dim] [server:dim] [repo:dim]", "stopky bez odznaku minut");
    assert_eq!(sig(&show(Look::Line, &f)), "line acc1.0 idle2.0", "bez průběhu třetina");
    // skladba bez timeline: bez času i lišty
    let mut f = base();
    f.music = Some(MusicFact { text: "Song".into(), time: None, progress: None });
    assert_eq!(sig(&show(Look::Progress, &f)), "Song");
    // lišta bez limitu: jen hodiny vpravo; bez hodin nic vpravo
    let mut f = base();
    f.limit5h = None;
    assert_eq!(sig(&show(Look::Bar, &f)), "práce ⇥ 14:32");
    f.clock.clear();
    f.mode.clear();
    assert_eq!(sig(&show(Look::Bar, &f)), "ledge");
}

#[test]
fn servers_and_repos() {
    let mut f = base();
    f.servers = 3;
    f.repos = 1;
    assert_eq!(sig(&show(Look::Icons, &f)), "[claude:dim] [server:ok 3] [repo:text 1]");
    assert_eq!(sig(&show(Look::Dots, &f)), "●idle ●ok ●wait ●idle");
    assert_eq!(sig(&show(Look::Bar, &f)), "práce | srv 3 | repo 1 ⇥ 5h 42 % · 14:32");
    f.server_failed = true;
    assert_eq!(sig(&show(Look::Dots, &f)), "●idle ●red ●wait ●idle");
    assert_eq!(sig(&show(Look::Icons, &f)), "[claude:dim] [server:red 3] [repo:text 1]");
    // servery ani repa nejsou událost: klid zůstává klidem (neviditelný styl se neukáže)
    assert_eq!(state_of(&f), State::Idle);
    assert!(show(Look::Ghost, &f).hidden);
}

#[test]
fn advanced_hide_table() {
    let mut all = base();
    all.waiting = 1;
    all.working = 1;
    all.done = 1;
    all.timer = Some(timer18());
    all.music = Some(music());
    all.servers = 2;
    all.repos = 1;
    // (skryté, co z událostí zbude)
    let cases: [(Vec<StripItem>, Vec<State>); 6] = [
        (vec![], vec![State::Wait, State::Done, State::Timer, State::Work, State::Music]),
        (vec![StripItem::Waiting], vec![State::Done, State::Timer, State::Work, State::Music]),
        (vec![StripItem::Claude], vec![State::Wait, State::Timer, State::Music]),
        (vec![StripItem::Timer, StripItem::Media], vec![State::Wait, State::Done, State::Work]),
        (vec![StripItem::Waiting, StripItem::Claude, StripItem::Timer, StripItem::Media], vec![]),
        (vec![StripItem::Custom], vec![State::Wait, State::Done, State::Timer, State::Work, State::Music]),
    ];
    for (hide, want) in cases {
        assert_eq!(events(&without(all.clone(), &hide)), want, "{hide:?}");
    }
    let f = without(all.clone(), &[StripItem::Running, StripItem::Repos, StripItem::Limits, StripItem::Clock]);
    assert_eq!((f.servers, f.repos, f.limit5h, f.clock.as_str(), f.date.as_str()), (0, 0, None, "", ""));
    // všechno skryté → neviditelný styl zůstane schovaný
    let hidden = without(all, &[StripItem::Waiting, StripItem::Claude, StripItem::Timer, StripItem::Media]);
    assert!(show(Look::Ghost, &hidden).hidden);
}

#[test]
fn english_texts() {
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;
    let cases = [
        (Look::Island, State::Work, "●run Claude is working · 2"),
        (Look::Island, State::Wait, "●wait *Waiting for you · ledge*"),
        (Look::Island, State::Done, "[check:ok] Done: ledge"),
        (Look::Corner, State::Wait, "●wait *1 waiting*"),
        (Look::Corner, State::Music, "[music:music] playing"),
        (Look::Notch, State::Wait, "●wait *1 waiting*"),
        (Look::Progress, State::Timer, "Timer ⇥ 18 min ▭70%:acc"),
        (Look::Bar, State::Wait, "*claude ▸ WAITING 1* ⇥ 5h 42 % · 14:32"),
    ];
    for (look, st, want) in cases {
        assert_eq!(with_lang(Lang::En, || sig(&show(look, &facts(st)))), want, "{look:?} {st:?}");
    }
    assert_eq!(with_lang(Lang::En, || Look::Ghost.name()), "Hidden until needed");
}

#[test]
fn fact_conversions() {
    use crate::timer::{Countdown, Stopwatch, TimerState};
    let now = 1_000_000_000;
    // odpočet 25 min, zbývá 17:30 → 18 min, uplynulo 30 %
    let s = TimerState { countdown: Some(Countdown::Running { end: now + 17 * 60_000 + 30_000, total: 25 * 60_000 }), stopwatch: None };
    let t = facts::timer_fact(&s, now).unwrap();
    assert_eq!((t.text.as_str(), t.minutes, t.done), ("18 min", Some(18), false));
    assert!((t.progress.unwrap() - 0.3).abs() < 0.001);
    let s = TimerState { countdown: Some(Countdown::Done { at: now, total: 60_000 }), stopwatch: Some(Stopwatch::Paused { elapsed: 0 }) };
    let t = facts::timer_fact(&s, now).unwrap();
    assert_eq!((t.text.as_str(), t.minutes, t.progress, t.done), ("čas vypršel", None, None, true));
    let s = TimerState { countdown: None, stopwatch: Some(Stopwatch::Running { start: now - 42 * 60_000 }) };
    assert_eq!(facts::timer_fact(&s, now).map(|t| (t.text, t.progress)), Some(("0:42".to_string(), None)));
    assert_eq!(facts::timer_fact(&TimerState::default(), now), None);
    // hudba: jen při přehrávání; průběh z timeline
    let mut v = crate::media::MediaView {
        title: "Midnight City".into(),
        artist: "M83".into(),
        app: String::new(),
        status: "playing",
        can_toggle: true,
        can_prev: true,
        can_next: true,
        cover_key: None,
        timeline: Some(crate::mediatime::Timeline { position_ms: 102_000, duration_ms: 243_000, updated_ms: now, rate: 1.0, can_seek: true, start_ticks: 0 }),
        shuffle: None,
        repeat: None,
    };
    let m = facts::music_fact(Some(&v), now).unwrap();
    assert_eq!((m.text.as_str(), m.time.as_deref()), ("Midnight City · M83", Some("1/4 min")));
    assert!((m.progress.unwrap() - 102.0 / 243.0).abs() < 0.001);
    v.timeline = None;
    assert_eq!(facts::music_fact(Some(&v), now).map(|m| (m.time, m.progress)), Some((None, None)));
    v.status = "paused";
    assert_eq!(facts::music_fact(Some(&v), now), None);
    assert_eq!(facts::music_fact(None, now), None);
    assert_eq!(hm(8, 5), "08:05");
}

#[test]
fn look_ids_serde_and_flags() {
    let mut seen = std::collections::HashSet::new();
    for l in Look::ALL {
        assert_eq!(serde_json::to_string(&l).unwrap(), format!("\"{}\"", l.id()), "{l:?}");
        assert_eq!(serde_json::from_str::<Look>(&format!("\"{}\"", l.id())).unwrap(), l);
        assert!(seen.insert(l.id()));
    }
    assert!(serde_json::from_str::<Look>("\"pill\"").is_err());
    // (styl, hudba, ústup nad taby, celá šířka)
    let flags = [
        (Look::Classic, false, true, false),
        (Look::Line, true, false, true),
        (Look::Bar, true, true, true),
        (Look::Ghost, false, false, false),
        (Look::Corner, true, false, false),
        (Look::Dots, true, false, false),
        (Look::Island, true, true, false),
    ];
    for (l, music, retreat, full) in flags {
        assert_eq!((l.shows_music(), l.retreats(), l.full_width()), (music, retreat, full), "{l:?}");
    }
}

#[test]
fn ts_list_mirrors_rust() {
    // jediný seznam stylů pro Nastavení (`src/strip/looks.ts`) má stejná id i názvy
    let ts = include_str!("../../../src/strip/looks.ts");
    assert_eq!(ts.matches("id: \"").count(), Look::ALL.len(), "počet stylů v looks.ts");
    for l in Look::ALL {
        let cs = l.name();
        let en = crate::i18n::with_lang(crate::uiconf::Lang::En, || l.name());
        let row = format!("id: \"{}\", key: \"appearance.look.{}\", cs: \"{cs}\", en: \"{en}\"", l.id(), l.id());
        assert!(ts.contains(&row), "{row} chybí v looks.ts");
    }
}

#[test]
fn config_old_and_new() {
    // skutečný tvar configu před vlnou 7 (proužek s položkami, barvami, klidem) = classic beze změny
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.json");
    let old = r##"{
      "drawer": { "width": 1300, "trigger": "hover" },
      "appearance": { "style": "terminal", "theme": "terminal", "strip": { "style": "term", "size": "large", "position": "left" } },
      "strip": { "enabled": true, "items": ["mode", "clock", "running", "claude", "waiting", "repos"], "opacity": 0.7,
                 "custom": "🎯", "colors": { "clock": "teal" }, "quiet": ["clock"], "itemsRev": 1 },
      "media": { "variant": "strip" }
    }"##;
    std::fs::write(&p, old).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert_eq!((cfg.strip.look, cfg.strip.look_hide.len()), (Look::Classic, 0));
    assert_eq!(cfg.strip.items.len(), 6);
    assert_eq!(cfg.strip.quiet, vec![StripItem::Clock]);
    assert!(crate::config::validate(&cfg).is_ok());
    // nový tvar: camelCase, zpět stejně
    std::fs::write(&p, r#"{"strip":{"look":"island","lookHide":["media","repos"]}}"#).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert_eq!((cfg.strip.look, cfg.strip.look_hide.as_slice()), (Look::Island, &[StripItem::Media, StripItem::Repos][..]));
    let back = serde_json::to_value(&cfg.strip).unwrap();
    assert_eq!((back["look"].as_str(), back["lookHide"][0].as_str()), (Some("island"), Some("media")));
    // schéma zná všechna id
    let schema = crate::config::schema_json();
    for l in Look::ALL {
        assert!(schema.contains(&format!("\"{}\"", l.id())), "{} ve schématu", l.id());
    }
    assert!(schema.contains("lookHide"));
    // neznámý styl = chyba (jako jiné volby), ne tiché zahození
    assert!(serde_json::from_str::<crate::config::StripConfig>(r#"{"look":"bubble"}"#).is_err());
    // tvar kapsle (kolo 22): chybí = dnešní tvar; zapisuje se camelCase
    assert_eq!(cfg.strip.look_shape, Shape::Round);
    std::fs::write(&p, r#"{"strip":{"look":"trio","lookShape":"island"}}"#).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert_eq!((cfg.strip.look, cfg.strip.look_shape), (Look::Trio, Shape::Island));
    assert_eq!(serde_json::to_value(&cfg.strip).unwrap()["lookShape"].as_str(), Some("island"));
    assert!(schema.contains("lookShape"));
}

#[test]
fn capsule_shapes() {
    let s = Show::default();
    let ks = 1.0;
    let axis2 = anchor_c2(MON.left, MON.right, 1080, StripPosition::Center, 96);
    // (styl, tvar) → (rádius, odsazení, celá pilulka, odstup od hrany)
    let cases = [
        (Look::Trio, Shape::Round, (18.0, 16.0, false, 0.0)),
        (Look::Trio, Shape::Sharp, (4.0, 16.0, false, 0.0)),
        (Look::Trio, Shape::Long, (18.0, 32.0, false, 0.0)),
        (Look::Trio, Shape::Island, (18.0, 16.0, true, frame::ISLAND_GAP)),
        (Look::Island, Shape::Sharp, (4.0, 14.0, false, 0.0)),
        // tvar u stylů bez kapsle nic nemění
        (Look::Notch, Shape::Island, (18.0, 18.0, false, 0.0)),
        (Look::Corner, Shape::Sharp, (11.0, 10.0, true, 8.0)),
        (Look::Line, Shape::Long, (0.0, 0.0, false, 0.0)),
    ];
    for (look, shape, (r, pad, pill, gap)) in cases {
        let sp = frame::shaped(look, frame::spec(look, &s), shape);
        assert_eq!((sp.r, sp.pad, sp.pill, sp.top_gap), (r, pad, pill, gap), "{look:?} {shape:?}");
    }
    // „jako Mac“: tělo 6 px pod hranou, okno o to vyšší, zóna pokryje tělo
    let sp = frame::shaped(Look::Rings, frame::spec(Look::Rings, &s), Shape::Island);
    let p = frame::place(Look::Rings, StripPosition::Center, MON, axis2, 100, 28, &sp, ks);
    assert_eq!((p.oy, p.win.bottom - p.win.top), (6, 34));
    let (_, _, zb) = frame::zone(p.win, axis2 / 2, MON, 96);
    assert!(zb >= 34);
    // prodloužená je širší než dnešní
    let long = frame::shaped(Look::Minimum, frame::spec(Look::Minimum, &s), Shape::Long);
    let round = frame::spec(Look::Minimum, &s);
    assert!(frame::body_width(Look::Minimum, 80.0, &long, ks, MON) > frame::body_width(Look::Minimum, 80.0, &round, ks, MON));
    // které styly tvar mají
    let shapeable: Vec<_> = Look::ALL.into_iter().filter(|l| l.shapeable()).map(|l| l.id()).collect();
    assert_eq!(shapeable, ["island", "icons", "clock", "ghost", "progress", "ticker", "minimum", "trio", "rings"]);
    // kolo 22: baterie a systém jen u svých stylů; hudbu nové kapsle nespouští
    for l in Look::ALL {
        assert_eq!(l.shows_battery(), matches!(l, Look::Minimum | Look::Trio), "{l:?}");
        assert_eq!(l.shows_system(), l == Look::Trio, "{l:?}");
    }
    assert!(!Look::Rings.shows_music() && !Look::Trio.shows_music() && Look::Rings.retreats());
}

#[test]
fn media_thread_wanted_by_music_looks() {
    let mut cfg = crate::config::Config::default();
    assert!(!crate::media::strip_wants(&cfg), "výchozí: bez hudby v proužku");
    cfg.strip.look = Look::Island;
    assert!(crate::media::strip_wants(&cfg));
    cfg.strip.look_hide = vec![StripItem::Media];
    assert!(!crate::media::strip_wants(&cfg), "Pokročilé: hudbu nechci");
    cfg.strip.look_hide.clear();
    cfg.strip.look = Look::Ghost;
    assert!(!crate::media::strip_wants(&cfg));
    cfg.strip.look = Look::Ticker;
    cfg.strip.enabled = false;
    assert!(!crate::media::strip_wants(&cfg));
}

// ---------- geometrie ----------

const MON: RECT = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };

fn inside(r: RECT, m: RECT) -> bool {
    r.left >= m.left && r.right <= m.right && r.top >= m.top && r.bottom <= m.bottom && r.right > r.left && r.bottom > r.top
}

#[test]
fn place_every_look_inside_monitor_and_zone_covers_body() {
    let mons = [(MON, 96u32), (RECT { left: -2560, top: -200, right: 0, bottom: 1240 }, 144), (RECT { left: 1920, top: 0, right: 3840, bottom: 1080 }, 120)];
    for (mon, dpi) in mons {
        let ks = dpi as f32 / 96.0;
        for pos in [StripPosition::Center, StripPosition::Left, StripPosition::Right] {
            let axis2 = anchor_c2(mon.left, mon.right, (1080.0 * ks) as i32, pos, dpi);
            for look in Look::ALL {
                for st in STATES {
                    let s = show(look, &facts(st));
                    let sp = frame::spec(look, &s);
                    let bw = frame::body_width(look, 240.0 * ks, &sp, ks, mon);
                    let bh = (sp.h * ks).round() as i32;
                    let p = frame::place(look, pos, mon, axis2, bw, bh, &sp, ks);
                    assert!(inside(p.win, mon), "{look:?} {pos:?} {dpi} {:?}", (p.win.left, p.win.right));
                    assert_eq!(p.win.top, mon.top, "okno u horní hrany");
                    // tělo leží v okně
                    assert!(p.ox >= 0 && p.ox + p.bw <= p.win.right - p.win.left && p.oy + p.bh <= p.win.bottom - p.win.top);
                    // hover zóna pokryje celé tělo a začíná u horní hrany
                    let (zl, zr, zb) = frame::zone(p.win, axis2.div_euclid(2), mon, dpi);
                    let (bl, br) = (p.win.left + p.ox, p.win.left + p.ox + p.bw);
                    assert!(zl <= bl && zr >= br && zb >= mon.top + p.oy + p.bh, "{look:?} {pos:?}");
                    assert!(zl >= mon.left && zr <= mon.right, "zóna na monitoru");
                }
            }
        }
    }
}

#[test]
fn place_table() {
    let ks = 1.0;
    let axis2 = anchor_c2(MON.left, MON.right, 1080, StripPosition::Center, 96);
    let s = Show::default();
    type PlaceCase = (Look, StripPosition, i32, (i32, i32, i32, i32, i32));
    // (styl, pozice, šířka těla) → (vlevo, vpravo, výška okna, posun těla x, y)
    let cases: [PlaceCase; 7] = [
        (Look::Island, StripPosition::Center, 110, (905, 1015, 26, 0, 0)),
        (Look::Line, StripPosition::Left, 10, (0, 1920, 3, 0, 0)),
        (Look::Bar, StripPosition::Right, 10, (0, 1920, 20, 0, 0)),
        (Look::Corner, StripPosition::Left, 80, (10, 90, 30, 0, 8)),
        (Look::Corner, StripPosition::Right, 80, (1830, 1910, 30, 0, 8)),
        (Look::Corner, StripPosition::Center, 80, (1830, 1910, 30, 0, 8)),
        (Look::Notch, StripPosition::Center, 84, (906, 1014, 28, 12, 0)),
    ];
    for (look, pos, bw, want) in cases {
        let sp = frame::spec(look, &s);
        let bw = frame::body_width(look, bw as f32 - 2.0 * sp.pad, &sp, ks, MON);
        let bh = (sp.h * if look == Look::Line { 1.0 } else { ks }).round() as i32;
        let p = frame::place(look, pos, MON, axis2, bw, bh, &sp, ks);
        assert_eq!((p.win.left, p.win.right, p.win.bottom - p.win.top, p.ox, p.oy), want, "{look:?} {pos:?}");
    }
    // ostrůvek na ose vlevo, u hrany se ořízne do monitoru
    let axis_l = anchor_c2(MON.left, MON.right, 1080, StripPosition::Left, 96);
    let sp = frame::spec(Look::Island, &s);
    let p = frame::place(Look::Island, StripPosition::Left, MON, axis_l, 110, 26, &sp, 1.0);
    assert_eq!(p.win.left, span_x(axis_l, 110));
    let p = frame::place(Look::Island, StripPosition::Left, MON, -400, 110, 26, &sp, 1.0);
    assert_eq!(p.win.left, 0, "ořez na monitor");
    // DPI 150 %: roh 8 → 12 px od hrany, 10 → 15 px od boku
    let sp = frame::spec(Look::Corner, &s);
    let p = frame::place(Look::Corner, StripPosition::Right, MON, axis2, 120, 33, &sp, 1.5);
    assert_eq!((p.win.right, p.oy), (1920 - 15, 12));
}

#[test]
fn body_width_and_spec() {
    let s = Show::default();
    let idle = Show { idle: true, ..Show::default() };
    let sp = frame::spec(Look::Island, &idle);
    assert_eq!(frame::body_width(Look::Island, 7.0, &sp, 1.0, MON), 56, "ostrůvek v klidu malý");
    let sp = frame::spec(Look::Island, &s);
    assert_eq!(frame::body_width(Look::Island, 7.0, &sp, 1.0, MON), 110);
    assert_eq!(frame::body_width(Look::Island, 200.0, &sp, 1.0, MON), 228, "obsah + 2× odsazení");
    assert_eq!(frame::body_width(Look::Island, 200.0, &sp, 2.0, MON), 256, "DPI 200 %");
    assert_eq!(frame::body_width(Look::Line, 0.0, &frame::spec(Look::Line, &s), 1.0, MON), 1920);
    // průběh: s lištou vyšší
    let bar = Show { progress: Some((0.5, OK)), ..Show::default() };
    assert!(frame::spec(Look::Progress, &bar).h > frame::spec(Look::Progress, &s).h);
    // stavová lišta neproporcionální, hodiny větší písmo, roh oblý ze všech stran, výřez s oušky
    assert!(frame::spec(Look::Bar, &s).mono);
    assert!(frame::spec(Look::Clock, &s).display && frame::spec(Look::Clock, &s).font > frame::spec(Look::Island, &s).font);
    assert!(frame::spec(Look::Corner, &s).pill);
    assert!(frame::spec(Look::Notch, &s).ears > 0.0);
    assert_eq!(frame::spec(Look::Dots, &s).fill.3, 0.0, "tečky bez těla");
    assert_eq!(frame::morph_body(Look::Line, 1920, 3, 1.0), (600, 3));
    assert_eq!(frame::morph_body(Look::Island, 110, 26, 1.0), (110, 26));
}

#[test]
fn zone_table() {
    let r = |l, t, rr, b| RECT { left: l, top: t, right: rr, bottom: b };
    // (proužek, osa x, monitor, dpi) → (vlevo, vpravo, spodek)
    let cases = [
        ("klasický uprostřed", r(900, 0, 1020, 32), 960, MON, 96, (876, 1044, 32)),
        ("linka přes celou šířku: ořez, aspoň 4 px", r(0, 0, 1920, 3), 960, MON, 96, (0, 1920, 4)),
        ("stavová lišta", r(0, 0, 1920, 20), 960, MON, 96, (0, 1920, 20)),
        ("roh vpravo: od horní hrany, ořez", r(1830, 0, 1910, 30), 1500, MON, 96, (1806, 1920, 30)),
        ("skrytý (neviditelný v klidu): pruh na ose", RECT::default(), 960, MON, 96, (800, 1120, 4)),
        ("skrytý, osa vlevo u hrany", RECT::default(), 100, MON, 96, (0, 260, 4)),
        ("DPI 150 %", RECT::default(), 960, MON, 144, (720, 1200, 6)),
        ("monitor vlevo se zápornými x", r(-1500, -200, -1300, -170), -1400, RECT { left: -2560, top: -200, right: 0, bottom: 1240 }, 96, (-1524, -1276, -170)),
    ];
    for (name, srect, cx, mon, dpi, want) in cases {
        assert_eq!(frame::zone(srect, cx, mon, dpi), want, "{name}");
    }
}

#[test]
fn island_width_anim() {
    let mut a = WidthAnim::default();
    // první vykreslení: rovnou cíl, bez animace
    assert_eq!(a.step(56.0, true, 0), 56.0);
    assert!(!a.running());
    // změna obsahu: plynule, v půlce mezi, po ISLAND_MS cíl a konec
    assert_eq!(a.step(200.0, true, 1000), 56.0);
    assert!(a.running());
    let mid = a.step(200.0, true, 1000 + ISLAND_MS / 2);
    assert!(mid > 56.0 && mid < 200.0, "{mid}");
    assert_eq!(a.step(200.0, true, 1000 + ISLAND_MS), 200.0);
    assert!(!a.running(), "v klidu žádný časovač");
    // stejný cíl animaci nespustí; „omezit pohyb“ = skok
    assert_eq!(a.step(200.0, true, 5000), 200.0);
    assert!(!a.running());
    assert_eq!(a.step(80.0, false, 6000), 80.0);
    assert!(!a.running());
    // změna během animace pokračuje z aktuální šířky
    a.step(300.0, true, 7000);
    let cur = a.step(300.0, true, 7000 + ISLAND_MS / 2);
    a.step(100.0, true, 7000 + ISLAND_MS / 2);
    let next = a.step(100.0, true, 7000 + ISLAND_MS / 2 + 1);
    assert!((next - cur).abs() < 5.0, "bez skoku: {cur} → {next}");
    a.reset();
    assert_eq!(a.step(150.0, true, 9000), 150.0, "po skrytí rovnou v cílové šířce");
}

#[test]
fn config_080_loads_unchanged() {
    // tvar configu z vydání 0.8.0 (styly zavřeného Ledge, Pokročilé, design styl s rozložením) — nic z kola 22
    // se samo nezapne: tvar kapsle = dnešní, rozložení a šířka beze změny, žádné vzorkování systému navíc
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.json");
    let v080 = r##"{
      "drawer": { "width": 1300, "trigger": "hover", "hover": "full", "hoverDelayMs": 300 },
      "appearance": { "style": "fluent", "theme": "dark", "layout": "panel", "width": "medium", "material": "acrylic",
                      "motion": { "style": "smooth", "speed": 1 }, "strip": { "style": "pill", "size": "normal", "position": "center" } },
      "strip": { "enabled": true, "items": ["clock", "mode", "waiting"], "itemsRev": 2, "look": "island", "lookHide": ["media"] },
      "media": { "variant": "cover" }
    }"##;
    std::fs::write(&p, v080).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert!(crate::config::validate(&cfg).is_ok());
    assert_eq!((cfg.strip.look, cfg.strip.look_shape, cfg.strip.look_hide.as_slice()), (Look::Island, Shape::Round, &[StripItem::Media][..]));
    assert_eq!(cfg.appearance.layout, crate::appearance::look::Layout::Panel);
    assert_eq!(crate::appearance::geom::drawer_width_for(&cfg, 1920, 96), 1080.0);
    assert!(!crate::integrations::wants(&cfg).system);
    // tvar kapsle Round = přesně dnešní rozměry každého stylu
    for look in Look::ALL {
        let s = show(look, &facts(State::Work));
        assert_eq!(frame::shaped(look, frame::spec(look, &s), Shape::Round), frame::spec(look, &s), "{look:?}");
    }
    // skutečný config stávajícího uživatele (fixture) = klasický proužek, klasické rozložení
    std::fs::write(&p, include_str!("../fixtures/config-existing-user.json")).unwrap();
    let cfg = crate::config::load(&p).unwrap().unwrap();
    assert_eq!((cfg.strip.look, cfg.strip.look_shape, cfg.appearance.layout), (Look::Classic, Shape::Round, crate::appearance::look::Layout::Classic));
}
