//! Pojistka neviděné otázky Clauda (`claude.questionUnseenS`). Ledge drží otázku (`questions.rs`), ale
//! nikdo ji nemusí vidět: šuplík je zavřený, běží hra přes celou obrazovku (Ledge je „mrtvý“, peek
//! nevyjede), WebView šuplíku se nevytvořilo nebo leží otázka za schválením v peeku. Claude by pak
//! čekal až do `questionHoldS` (výchozí 540 s) a nikdo by o tom nevěděl.
//!
//! UI potvrdí „zobrazeno“ (`question_seen`), jen když je karta otázky opravdu vidět (šuplík otevřený,
//! WebView viditelné, karta v záběru — `src/questions/seen.ts`); jádro to vezme jen při otevřeném
//! šuplíku a živém Ledge (`seen_counts`). Když do `questionUnseenS` potvrzení nepřijde, vlákno hooku
//! otázku pustí do Claude Code (prázdné tělo = Claude se zeptá normálně), v Ledge zůstane jen
//! ukázaná („odpověz v okně Claude“) a deník dostane `releasedUnseen` s lidskou větou.
//! Žádný časovač ani vlákno navíc — čeká vlákno hooku po úsecích (`step`).
//! Mapa: `docs/codemap/sessions.md`.

use super::questions::{self, QuestionLog, push_log};
use super::{Claude, now_ms};
use std::time::Duration;

/// Výchozí pojistka v configu (`claude.questionUnseenS`); 0 = vypnuto.
pub const DEFAULT_S: u64 = 20;
/// Nejkratší rozumná pojistka (UI potřebuje chvíli na vysunutí peeku a vykreslení).
pub const MIN_S: u64 = 5;

/// Co má vlákno hooku udělat teď.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// čekat na odpověď nejvýš tolik ms, pak se zeptat znovu
    Wait(u64),
    /// otázku nikdo neviděl → pustit do Claude (`release_unseen`)
    ReleaseUnseen,
    /// termín držení (`questionHoldS`) vypršel
    Expire,
}

/// Rozhodnutí z času od příchodu otázky (`elapsed`), držení (`hold`), pojistky (`unseen`, 0 = vypnuto)
/// a toho, jestli ji UI už ukázalo. Pojistka platí jen, když je kratší než držení. Nikdy nečeká déle
/// než do konce držení.
pub fn step(elapsed: u64, hold: u64, unseen: u64, seen: bool) -> Step {
    if elapsed >= hold {
        return Step::Expire;
    }
    if unseen > 0 && unseen < hold && !seen {
        return if elapsed >= unseen { Step::ReleaseUnseen } else { Step::Wait(unseen - elapsed) };
    }
    Step::Wait(hold - elapsed)
}

/// Platí potvrzení „zobrazeno“? Jen při otevřeném šuplíku (peek, náhled po najetí nebo plný) a živém
/// Ledge — zavřený šuplík ani fullscreen hra se za „viděno“ nepočítají, i kdyby UI něco poslalo.
pub fn seen_counts(drawer_open: bool, dead: bool) -> bool {
    drawer_open && !dead
}

/// Pojistka z configu: 0 = vypnuto, jinak `MIN_S`…`questions::MAX_HOLD_S`.
pub fn clamp_s(secs: u64) -> u64 {
    if secs == 0 { 0 } else { secs.clamp(MIN_S, questions::MAX_HOLD_S) }
}

/// Lidská věta do deníku: proč Ledge otázku pustil.
pub fn released_detail(unseen_ms: u64) -> String {
    let s = unseen_ms.div_ceil(1000);
    trf!(
        "do {s} s ji v Ledge nikdo neviděl (šuplík zavřený nebo hra přes celou obrazovku) — Claude se zeptá v okně Claude Code",
        "nobody saw it in Ledge within {s} s (drawer closed or a fullscreen game) — Claude asks in the Claude Code window"
    )
}

impl Claude {
    pub fn set_question_unseen(&self, secs: u64) {
        *self.question_unseen.lock() = Duration::from_secs(clamp_s(secs));
    }

    /// UI otázku opravdu ukázalo (`question_seen`; bránu `seen_counts` řeší příkaz). `true` = otázka je
    /// v Ledge (potvrzeno teď nebo dřív), `false` = už tu není. Snapshot se nemění → žádné `on_change`.
    pub fn question_seen(&self, id: u64) -> bool {
        let now = now_ms();
        let mut g = self.inner.lock();
        let Some((a, _)) = g.asks.iter_mut().find(|(a, _)| a.id == id) else { return false };
        if a.seen_at.is_some() {
            return true;
        }
        a.seen_at = Some(now);
        let secs = now.saturating_sub(a.created_at).div_ceil(1000);
        let n = a.questions.len();
        // do deníku: otázka došla až k tobě (bez změny stavu)
        if let Some(e) = g.question_log.iter_mut().find(|x| x.id == id) {
            e.detail = Some(trf!("otázek: {n} · viděno v Ledge po {secs} s", "questions: {n} · seen in Ledge after {secs} s"));
        }
        true
    }

    /// Pojistka: neviděnou drženou otázku pustí do Claude (odešle se prázdné tělo — vlákno hooku skončí,
    /// protože odesílač zmizí) a v Ledge ji nechá jen ukázanou. `false` = mezitím ji někdo viděl,
    /// odpověděl nebo pustil — nic se nemění.
    pub(super) fn release_unseen(&self, id: u64, unseen_ms: u64) -> bool {
        let now = now_ms();
        let mut g = self.inner.lock();
        let Some((a, tx)) = g.asks.iter_mut().find(|(a, tx)| a.id == id && tx.is_some()) else { return false };
        if a.seen_at.is_some() {
            return false;
        }
        *tx = None;
        a.answerable = false;
        a.released_unseen = true;
        a.expires_at = now + questions::SHOW_ONLY_MS;
        let (sid, title) = (a.session_id.clone(), a.questions.first().map(questions::name).unwrap_or_default());
        push_log(&mut g, QuestionLog { id, at: now, session_id: sid, title, outcome: "releasedUnseen".into(), detail: Some(released_detail(unseen_ms)) });
        drop(g);
        (self.on_change)();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::SessionState;
    use serde_json::{Value, json};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    fn ask_body(sid: &str) -> Value {
        json!({"hook_event_name": "PreToolUse", "tool_name": questions::TOOL, "session_id": sid, "cwd": "C:/p", "tool_input": {"questions": [{"question": "Kam?", "header": "Cíl", "options": [{"label": "A"}, {"label": "B"}]}]}})
    }

    fn wait_for(c: &Claude, sid: &str) -> u64 {
        let t0 = Instant::now();
        loop {
            if let Some(a) = c.questions().into_iter().find(|a| a.session_id == sid) {
                return a.id;
            }
            assert!(t0.elapsed() < Duration::from_secs(5), "otázka {sid} se neobjevila");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Pojistka v testu v milisekundách (config bere celé sekundy).
    fn quick(c: &Claude, hold_ms: u64, unseen_ms: u64) {
        *c.question_hold.lock() = Duration::from_millis(hold_ms);
        *c.question_unseen.lock() = Duration::from_millis(unseen_ms);
    }

    #[test]
    fn step_table() {
        use Step::*;
        let cases: &[(&str, u64, u64, u64, bool, Step)] = &[
            ("vypnutá pojistka: čeká do konce držení", 0, 540_000, 0, false, Wait(540_000)),
            ("neviděná: čeká do pojistky", 0, 540_000, 20_000, false, Wait(20_000)),
            ("neviděná: zbytek do pojistky", 15_000, 540_000, 20_000, false, Wait(5_000)),
            ("neviděná po pojistce: pustit", 20_000, 540_000, 20_000, false, ReleaseUnseen),
            ("neviděná dlouho po pojistce: pustit", 300_000, 540_000, 20_000, false, ReleaseUnseen),
            ("viděná: pojistka nehraje, čeká do konce držení", 20_000, 540_000, 20_000, true, Wait(520_000)),
            ("viděná po termínu: vypršelo", 540_000, 540_000, 20_000, true, Expire),
            ("termín má přednost před pojistkou", 540_000, 540_000, 20_000, false, Expire),
            ("pojistka delší než držení: jen držení", 0, 10_000, 20_000, false, Wait(10_000)),
            ("pojistka rovná držení: jen držení", 9_000, 10_000, 10_000, false, Wait(1_000)),
        ];
        for (name, elapsed, hold, unseen, seen, want) in cases {
            assert_eq!(step(*elapsed, *hold, *unseen, *seen), *want, "{name}");
        }
        // nikdy nečeká přes konec držení
        for e in (0..60_000).step_by(997) {
            for (u, seen) in [(0, false), (20_000, false), (20_000, true), (90_000, false)] {
                if let Step::Wait(ms) = step(e, 30_000, u, seen) {
                    assert!(e + ms <= 30_000, "e={e} u={u} seen={seen} ms={ms}");
                }
            }
        }
    }

    #[test]
    fn seen_gate_and_clamp_table() {
        let gate = [((true, false), true), ((false, false), false), ((true, true), false), ((false, true), false)];
        for ((open, dead), want) in gate {
            assert_eq!(seen_counts(open, dead), want, "open={open} dead={dead}");
        }
        for (secs, want) in [(0, 0), (1, MIN_S), (MIN_S, MIN_S), (20, 20), (10_000, questions::MAX_HOLD_S)] {
            assert_eq!(clamp_s(secs), want, "{secs}");
        }
        assert!(released_detail(20_000).starts_with("do 20 s ji v Ledge nikdo neviděl"));
        assert!(crate::i18n::with_lang(crate::uiconf::Lang::En, || released_detail(19_001)).contains("within 20 s"));
    }

    /// Neviděná otázka: po pojistce prázdné tělo (Claude se zeptá sám), v Ledge zůstane jen ukázaná,
    /// session dál čeká, deník `releasedUnseen` s větou; odpověď z Ledge už nejde.
    #[test]
    fn unseen_question_goes_back_to_claude() {
        let changes = Arc::new(AtomicUsize::new(0));
        let ch = changes.clone();
        let c = Claude::new(Arc::new(move || { ch.fetch_add(1, Ordering::Relaxed); }), Duration::from_secs(1));
        quick(&c, 5_000, 150);
        let t0 = Instant::now();
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        let id = wait_for(&c, "s");
        assert_eq!(h.join().unwrap(), None, "prázdné tělo");
        let took = t0.elapsed();
        assert!(took >= Duration::from_millis(150) && took < Duration::from_secs(4), "{took:?}");
        let a = c.questions().pop().expect("v Ledge zůstane jen ukázaná");
        assert!(a.id == id && !a.answerable && a.released_unseen);
        assert!(c.attention(), "proužek / peek dál ví, že Claude čeká");
        assert_eq!(c.sessions()[0].state, SessionState::Waiting);
        assert!(c.answer_question(id, &[questions::Reply { labels: vec!["A".into()], other: None }]).is_err());
        let log = c.question_log();
        assert_eq!((log[0].id, log[0].outcome.as_str()), (id, "releasedUnseen"));
        assert!(log[0].detail.as_deref().unwrap().contains("nikdo neviděl"), "{:?}", log[0].detail);
        assert!(changes.load(Ordering::Relaxed) >= 2, "příchod + puštění");
        // odpověď pak padla v Claude Code: otázka zmizí, deník drží „neviděno“ a připíše konec
        c.handle_event(&json!({"hook_event_name": "PostToolUse", "session_id": "s", "tool_name": questions::TOOL}));
        assert!(c.questions().is_empty());
        let log = c.question_log();
        assert_eq!(log[0].outcome, "releasedUnseen");
        assert!(log[0].detail.as_deref().unwrap().ends_with("odpověď padla v Claude Code"), "{:?}", log[0].detail);
    }

    /// Viděná otázka: pojistka nehraje, Ledge drží dál a odpověď kliknutím projde.
    #[test]
    fn seen_question_keeps_waiting_for_click() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        quick(&c, 5_000, 100);
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        let id = wait_for(&c, "s");
        assert!(c.question_seen(id));
        assert!(c.question_seen(id), "podruhé beze změny");
        assert!(c.question_log()[0].detail.as_deref().unwrap().contains("viděno v Ledge"));
        std::thread::sleep(Duration::from_millis(300));
        let a = c.questions().pop().unwrap();
        assert!(a.answerable && !a.released_unseen, "po pojistce pořád držená");
        c.answer_question(id, &[questions::Reply { labels: vec!["B".into()], other: None }]).unwrap();
        assert!(h.join().unwrap().unwrap().contains(r#""answers":{"Kam?":"B"}"#));
        assert_eq!(c.question_log()[0].outcome, "answered");
        assert!(!c.question_seen(id), "vyřízená otázka už v Ledge není");
        assert!(!c.question_seen(9_999));
    }

    /// Víc session naráz: každá otázka má vlastní vlákno hooku, pojistka i odpověď platí jen pro tu svou.
    #[test]
    fn several_sessions_at_once() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        quick(&c, 5_000, 200);
        let spawn = |sid: &'static str| {
            let c2 = c.clone();
            std::thread::spawn(move || c2.handle_question(&ask_body(sid)))
        };
        let (ha, hb, hc) = (spawn("a"), spawn("b"), spawn("c"));
        let (a, b, cc) = (wait_for(&c, "a"), wait_for(&c, "b"), wait_for(&c, "c"));
        assert_eq!(c.questions().len(), 3);
        // a: viděná a zodpovězená; b: viděná, čeká dál; c: neviděná → pojistka
        assert!(c.question_seen(a) && c.question_seen(b));
        c.answer_question(a, &[questions::Reply { labels: vec!["A".into()], other: None }]).unwrap();
        assert!(ha.join().unwrap().is_some());
        assert_eq!(hc.join().unwrap(), None);
        let qs = c.questions();
        let find = |id| qs.iter().find(|x| x.id == id).cloned();
        assert!(find(b).is_some_and(|x| x.answerable), "b dál drží Ledge");
        assert!(find(cc).is_some_and(|x| !x.answerable && x.released_unseen), "c jen ukázaná");
        // konec tahu session b pustí jen b
        c.handle_event(&json!({"hook_event_name": "Stop", "session_id": "b"}));
        assert_eq!(hb.join().unwrap(), None);
        assert!(c.questions().iter().all(|x| x.session_id == "c"));
        let out: Vec<(u64, String)> = c.question_log().iter().map(|e| (e.id, e.outcome.clone())).collect();
        for (id, want) in [(a, "answered"), (b, "turnEnded"), (cc, "releasedUnseen")] {
            assert!(out.contains(&(id, want.to_string())), "{id} {want}: {out:?}");
        }
    }

    /// Termín hooku vs. držení: hook `/question` má v settings.json pevný timeout, držení i pojistka ho
    /// nikdy nepřetáhnou (jinak by Claude Code spojení zavřel dřív a odpověď z Ledge by se ztratila).
    #[test]
    fn hold_never_outlives_the_hook_timeout() {
        let hook = crate::hooksetup::EVENTS.iter().find(|(_, p, _, _)| *p == "/question").map(|(_, _, t, _)| u64::from(*t)).unwrap();
        assert!(questions::MAX_HOLD_S + 30 <= hook, "rezerva aspoň 30 s: držení {} s, hook {hook} s", questions::MAX_HOLD_S);
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.set_question_hold(10_000);
        c.set_question_unseen(10_000);
        assert!(*c.question_hold.lock() <= Duration::from_secs(questions::MAX_HOLD_S));
        assert!(*c.question_unseen.lock() <= Duration::from_secs(questions::MAX_HOLD_S));
        let cfg = crate::config::ClaudeConfig::default();
        assert!(cfg.question_hold_s <= questions::MAX_HOLD_S && cfg.question_unseen_s < cfg.question_hold_s);
    }

    /// Restart Ledge za držení: proces skončí, spojení hooku spadne → Claude Code to bere jako
    /// neblokující chybu hooku a zeptá se ve svém okně (stejně jako „Ledge neběží“). Nový Ledge o otázce
    /// neví (stav i deník jsou jen v paměti): nic neukáže, odpověď na staré id nepřijme, pozdní
    /// `PostToolUse` / `Stop` té session nic nerozbije a další otázka té session jde normálně.
    #[test]
    fn restart_forgets_held_question() {
        let old = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        quick(&old, 5_000, 0);
        let o2 = old.clone();
        let _held = std::thread::spawn(move || o2.handle_question(&ask_body("s")));
        let id = wait_for(&old, "s");
        // „restart“ = nová instance jádra
        let new = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        quick(&new, 5_000, 0);
        assert!(new.questions().is_empty() && new.question_log().is_empty() && !new.attention());
        assert!(new.answer_question(id, &[questions::Reply { labels: vec!["A".into()], other: None }]).is_err());
        assert!(!new.question_seen(id) && !new.release_question(id));
        new.handle_event(&json!({"hook_event_name": "PostToolUse", "session_id": "s", "tool_name": questions::TOOL}));
        new.handle_event(&json!({"hook_event_name": "Stop", "session_id": "s"}));
        assert!(new.questions().is_empty() && new.question_log().is_empty());
        let n2 = new.clone();
        let h = std::thread::spawn(move || n2.handle_question(&ask_body("s")));
        let fresh = wait_for(&new, "s");
        assert!(new.release_question(fresh));
        assert_eq!(h.join().unwrap(), None);
        old.release_question(id);
    }

    /// Starý config bez klíče = výchozí pojistka; 0 vypne.
    #[test]
    fn config_old_and_new() {
        let old: crate::config::ClaudeConfig = serde_json::from_str(r#"{"enabled":true,"questionHoldS":540}"#).unwrap();
        assert_eq!(old.question_unseen_s, DEFAULT_S);
        let off: crate::config::ClaudeConfig = serde_json::from_str(r#"{"questionUnseenS":0}"#).unwrap();
        assert_eq!(off.question_unseen_s, 0);
    }
}
