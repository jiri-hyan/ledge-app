//! Review plánu přes zdokumentovanou cestu: hook `PreToolUse` s matcherem `ExitPlanMode` → `POST /plan`.
//!
//! Docs hooků (code.claude.com/docs/en/hooks, „ExitPlanMode“ + „PreToolUse decision control“):
//! - Claude Code do `tool_input` vloží `plan` (Markdown ze souboru) a `planFilePath`; vstup nese `tool_use_id`.
//! - `permissionDecision: "allow"` u ExitPlanMode **musí** nést `updatedInput` (samotné allow nestačí);
//!   `updatedInput` nahrazuje celý vstup → posíláme ho beze změny. Allow + `updatedInput` projde i v plan
//!   mode bez dialogu (docs permissions → „Hook approvals in plan mode“).
//! - `"deny"` + `permissionDecisionReason` = důvod vidí Claude → „Vrátit s připomínkou“.
//! - Režim po schválení (`acceptEdits`) `PreToolUse` nastavit neumí → karta nabídne „Vybrat režim v Claude“
//!   (`Pass`). Prázdná 2xx / nedostupný most / termín = žádné rozhodnutí → Claude se zeptá sám (docs: HTTP
//!   hook s chybou spojení je non-blocking error, termín → „normal permission flow“).
//!
//! Deduplikace (jeden plán = jedna karta): druhá kopie téhož `PreToolUse` (session + `tool_use_id`, jinak
//! obsah) kartu nezaloží; `PermissionRequest` plánu, který čeká na kartě z `PreToolUse` nebo který Ledge
//! před chvílí pustil do Claude (`Released`, `RELEASED_MS`), taky ne (`duplicate`). Allow / deny
//! z `PreToolUse` dialog v Claude přeskočí, takže `PermissionRequest` po nich nepřijde.
//! Mapa: `docs/codemap/sessions.md` → Schvalování.

use super::{Claude, Decision, Inner, Pending, plan, s, summarize};
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};

/// Jak dlouho si Ledge pamatuje plán puštěný do Claude: `PermissionRequest` po něm přijde hned (ms).
pub const RELEASED_MS: u64 = 2 * 60 * 1000;
/// Strop paměti puštěných plánů (víc session naráz).
pub const RELEASED_MAX: usize = 32;

/// Plán z `PreToolUse`, o kterém Ledge nerozhodl (termín, „Vybrat režim v Claude“, konec turnu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Released {
    pub session_id: String,
    pub key: u64,
    pub at: u64,
}

/// Klíč obsahu plánu — týž plán z obou hooků dá týž klíč. Jen v paměti procesu.
pub fn key(p: &plan::Plan) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    p.markdown.trim().hash(&mut h);
    h.finish()
}

/// `tool_use_id` vstupu `PreToolUse` (neprázdný).
pub fn tool_use_id(body: &Value) -> Option<String> {
    body.get("tool_use_id").and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

fn is_hook_plan(p: &Pending, sid: &str) -> bool {
    p.via == plan::Via::PreToolUse && p.session_id == sid && p.plan.is_some()
}

/// Druhá kopie téže karty z `PreToolUse`: stejná session a stejný `tool_use_id` (když ho mají oba),
/// jinak stejný obsah plánu.
pub(super) fn same_card<'a>(mut pending: impl Iterator<Item = &'a Pending>, sid: &str, tuid: Option<&str>, k: u64) -> bool {
    pending.any(|p| {
        is_hook_plan(p, sid)
            && match (p.tool_use_id.as_deref(), tuid) {
                (Some(a), Some(b)) => a == b,
                _ => p.plan.as_ref().map(key) == Some(k),
            }
    })
}

/// `PermissionRequest` plánu (`k` = klíč jeho plánu, `None` = vstup plán nenese), který už řeší karta
/// z `PreToolUse` (čeká) nebo který Ledge před < `RELEASED_MS` pustil do Claude. Nalezený `Released` se
/// spotřebuje — další (přepracovaný) plán téže session už kartu dostane.
pub(super) fn duplicate(g: &mut Inner, sid: &str, k: Option<u64>, now: u64) -> bool {
    if g.pending.iter().any(|(p, _)| is_hook_plan(p, sid)) {
        return true;
    }
    take_released(&mut g.released, sid, k, now)
}

pub(super) fn take_released(released: &mut Vec<Released>, sid: &str, k: Option<u64>, now: u64) -> bool {
    released.retain(|r| now.saturating_sub(r.at) < RELEASED_MS);
    match released.iter().position(|r| r.session_id == sid && k.is_none_or(|k| k == r.key)) {
        Some(i) => {
            released.remove(i);
            true
        }
        None => false,
    }
}

pub(super) fn remember(released: &mut Vec<Released>, r: Released) {
    released.retain(|x| r.at.saturating_sub(x.at) < RELEASED_MS && !(x.session_id == r.session_id && x.key == r.key));
    released.push(r);
    if released.len() > RELEASED_MAX {
        released.remove(0);
    }
}

/// Tělo odpovědi `POST /plan`. `None` = prázdná 200 → Claude se zeptá sám (`Pass`; `AllowMode` /
/// `AllowAlways` sem nepatří — `plan::resolve` je u `PreToolUse` nevydá).
pub fn body(d: &Decision, input: &Value) -> Option<String> {
    let out = |v: Value| Some(json!({ "hookSpecificOutput": v }).to_string());
    match d {
        Decision::Allow => out(json!({"hookEventName": "PreToolUse", "permissionDecision": "allow", "updatedInput": input})),
        Decision::Deny => out(json!({"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "Zamítnuto v Ledge."})),
        Decision::DenyWith(r) => out(json!({"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": format!("Zamítnuto v Ledge: {r}")})),
        Decision::ReturnPlan(r) => out(json!({"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": plan::return_message(r)})),
        Decision::AllowMode(_) | Decision::AllowAlways(_) | Decision::Pass => None,
    }
}

impl Claude {
    /// `POST /plan` (PreToolUse, ExitPlanMode). Blokuje do rozhodnutí / termínu. `None` = prázdná 200.
    pub fn handle_plan(&self, body_in: &Value) -> Option<String> {
        if s(body_in, "tool_name") != plan::TOOL {
            return None;
        }
        let sid = s(body_in, "session_id");
        let input = body_in.get("tool_input").filter(|v| v.is_object())?;
        if sid.is_empty() {
            return None;
        }
        let cwd = s(body_in, "cwd");
        // bez plánu není co ukázat → Claude se zeptá sám (případný PermissionRequest = obyčejná karta)
        let plan = plan::for_request(input, &cwd)?;
        let k = key(&plan);
        let tuid = tool_use_id(body_in);
        let summary = if plan.title.is_empty() { summarize(plan::TOOL, input) } else { plan.title.clone() };
        let p = Pending {
            id: 0,
            session_id: sid.clone(),
            cwd,
            tool: plan::TOOL.into(),
            summary,
            created_at: 0,
            expires_at: 0,
            always: None,
            preview: None,
            plan: Some(plan),
            via: plan::Via::PreToolUse,
            tool_use_id: tuid.clone(),
        };
        let d = self.hold(p, |g, _| same_card(g.pending.iter().map(|(p, _)| p), &sid, tuid.as_deref(), k))?;
        if d == Decision::Pass {
            remember(&mut self.inner.lock().released, Released { session_id: sid, key: k, at: super::now_ms() });
        }
        body(&d, input)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SessionState, rules};
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    const PLAN: &str = "# Oprava přihlášení\n\n1. testy";

    /// Vstup přesně podle docs (PreToolUse input + ExitPlanMode `plan` / `planFilePath`).
    fn req(sid: &str, tuid: &str, plan: &str) -> Value {
        json!({"session_id": sid, "transcript_path": "C:/t.jsonl", "cwd": "C:/p", "permission_mode": "plan", "hook_event_name": "PreToolUse",
            "tool_name": "ExitPlanMode", "tool_input": {"plan": plan, "planFilePath": "C:/Users/x/.claude/plans/a.md"}, "tool_use_id": tuid})
    }

    fn perm(sid: &str, input: Value) -> Value {
        json!({"session_id": sid, "cwd": "C:/p", "hook_event_name": "PermissionRequest", "tool_name": "ExitPlanMode", "tool_input": input})
    }

    fn wait_pending(c: &Claude, n: usize) -> Vec<Pending> {
        let t0 = std::time::Instant::now();
        loop {
            let p = c.pending();
            if p.len() >= n {
                return p;
            }
            assert!(t0.elapsed() < Duration::from_secs(5), "karta nepřišla");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn hso(b: &str) -> Value {
        serde_json::from_str::<Value>(b).unwrap()["hookSpecificOutput"].clone()
    }

    /// Tvar odpovědi `PreToolUse` podle docs: allow vždy s celým `updatedInput`, deny s důvodem pro Clauda.
    #[test]
    fn body_table() {
        let input = json!({"plan": PLAN, "planFilePath": "C:/a.md", "neznámé": 1});
        let cases: Vec<(&str, Decision, Option<Value>)> = vec![
            ("schválit", Decision::Allow, Some(json!({"hookEventName": "PreToolUse", "permissionDecision": "allow", "updatedInput": input.clone()}))),
            ("zamítnout", Decision::Deny, Some(json!({"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "Zamítnuto v Ledge."}))),
            (
                "vrátit s připomínkou",
                Decision::ReturnPlan("nejdřív testy \"x\"".into()),
                Some(json!({"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "Plán vrácen z Ledge s připomínkou: nejdřív testy \"x\""})),
            ),
            ("důvod", Decision::DenyWith("ne".into()), Some(json!({"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "Zamítnuto v Ledge: ne"}))),
            ("v Claude / termín", Decision::Pass, None),
            ("režim přes PreToolUse nejde", Decision::AllowMode(plan::Mode::AcceptEdits), None),
        ];
        for (what, d, want) in cases {
            assert_eq!(body(&d, &input).map(|b| hso(&b)), want, "{what}");
        }
    }

    #[test]
    fn parse_input_table() {
        assert_eq!(tool_use_id(&json!({"tool_use_id": "toolu_01ABC"})).as_deref(), Some("toolu_01ABC"));
        assert_eq!(tool_use_id(&json!({"tool_use_id": "  "})), None);
        assert_eq!(tool_use_id(&json!({})), None);
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        // cizí nástroj, chybí session, vstup není objekt, prázdný plán bez souboru → nic, žádná karta
        for b in [
            json!({"session_id": "s", "tool_name": "Bash", "tool_input": {"command": "ls"}}),
            json!({"tool_name": "ExitPlanMode", "tool_input": {"plan": PLAN}}),
            json!({"session_id": "s", "tool_name": "ExitPlanMode", "tool_input": "x"}),
            json!({"session_id": "s", "cwd": "C:/nikde-neni", "tool_name": "ExitPlanMode", "tool_input": {"plan": " ", "planFilePath": "C:/nikde-neni/a.md"}}),
        ] {
            assert_eq!(c.handle_plan(&b), None, "{b}");
        }
        assert!(c.pending().is_empty());
        // klíč: obsah (okolní mezery nevadí), jiný plán = jiný klíč
        let p = |t: &str| plan::parse(t, None).unwrap();
        assert_eq!(key(&p(PLAN)), key(&p(&format!("\n{PLAN}\n"))));
        assert_ne!(key(&p(PLAN)), key(&p("# Jiný")));
    }

    /// Karta z `PreToolUse`: schválit → allow + celý vstup; vrátit → deny + důvod; režim nejde; zkratka jde.
    #[test]
    fn hook_card_actions() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(5));
        let run = |c: &Claude, r: Value| {
            let c2 = c.clone();
            std::thread::spawn(move || c2.handle_plan(&r))
        };
        let h = run(&c, req("s", "toolu_1", PLAN));
        let p = wait_pending(&c, 1).remove(0);
        assert_eq!((p.via, p.summary.as_str(), p.tool.as_str()), (plan::Via::PreToolUse, "Oprava přihlášení", "ExitPlanMode"));
        assert!(p.always.is_none());
        assert_eq!(c.sessions()[0].state, SessionState::Waiting);
        let mode = rules::Extra { mode: Some(plan::Mode::AcceptEdits), ..Default::default() };
        assert!(!c.decide_with(Some(p.id), true, &mode), "acceptEdits přes PreToolUse nejde");
        assert!(c.decide_with(Some(p.id), true, &rules::Extra::default()));
        let out = hso(&h.join().unwrap().expect("tělo"));
        assert_eq!(out["permissionDecision"], "allow");
        assert_eq!(out["updatedInput"], req("s", "toolu_1", PLAN)["tool_input"], "updatedInput = vstup beze změny");

        let h = run(&c, req("s", "toolu_2", PLAN));
        let p = wait_pending(&c, 1).remove(0);
        assert!(c.decide_with(Some(p.id), false, &rules::Extra { reason: Some("nejdřív testy".into()), ..Default::default() }));
        assert_eq!(hso(&h.join().unwrap().unwrap())["permissionDecisionReason"], "Plán vrácen z Ledge s připomínkou: nejdřív testy");

        let h = run(&c, req("s", "toolu_3", PLAN));
        wait_pending(&c, 1);
        assert!(c.decide(None, Decision::Deny), "zkratka dál zamítne nejstarší");
        assert_eq!(hso(&h.join().unwrap().unwrap())["permissionDecision"], "deny");
    }

    /// Jeden plán = jedna karta: druhá kopie `PreToolUse`, `PermissionRequest` při čekající kartě i po puštění.
    #[test]
    fn one_plan_one_card() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(5));
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_plan(&req("s", "toolu_1", PLAN)));
        let p = wait_pending(&c, 1).remove(0);
        // druhá kopie téhož PreToolUse (stejné tool_use_id) i téhož obsahu bez id → hned prázdno
        assert_eq!(c.handle_plan(&req("s", "toolu_1", PLAN)), None);
        let mut no_id = req("s", "", PLAN);
        no_id.as_object_mut().unwrap().remove("tool_use_id");
        assert_eq!(c.handle_plan(&no_id), None);
        // PermissionRequest téže session při čekající kartě → Pass hned, bez karty
        let t0 = std::time::Instant::now();
        assert_eq!(c.handle_permission(&perm("s", json!({"plan": PLAN}))), Decision::Pass);
        assert!(t0.elapsed() < Duration::from_secs(2));
        assert_eq!(c.pending().len(), 1);
        // „Vybrat režim v Claude“ → prázdno; session dál čeká (v okně Claude)
        let inc = rules::Extra { in_claude: true, ..Default::default() };
        assert!(!c.decide_with(None, true, &inc), "jen s konkrétním id");
        assert!(c.decide_with(Some(p.id), true, &inc));
        assert_eq!(h.join().unwrap(), None);
        assert_eq!(c.sessions()[0].state, SessionState::Waiting);
        // PermissionRequest, který po puštění přijde (s plánem i bez), kartu nezaloží — jednou
        assert_eq!(c.handle_permission(&perm("s", json!({}))), Decision::Pass);
        assert!(c.pending().is_empty());
        // jiná session / další plán: karta jako dřív (Released se spotřeboval)
        let c3 = c.clone();
        let h = std::thread::spawn(move || c3.handle_permission(&perm("s", json!({"plan": "# Nový plán"}))));
        let p = wait_pending(&c, 1).remove(0);
        assert_eq!((p.via, p.summary.as_str()), (plan::Via::Permission, "Nový plán"));
        assert!(c.decide(Some(p.id), Decision::Allow));
        assert_eq!(h.join().unwrap(), Decision::Allow);
    }

    /// (popis, paměť, sid, klíč, teď, duplikát?, zbylo)
    type ReleasedCase<'a> = (&'a str, Vec<Released>, &'a str, Option<u64>, u64, bool, usize);

    #[test]
    fn released_memory_table() {
        let r = |sid: &str, k: u64, at: u64| Released { session_id: sid.into(), key: k, at };
        let cases: Vec<ReleasedCase> = vec![
            ("stejný plán", vec![r("s", 1, 0)], "s", Some(1), 10, true, 0),
            ("bez plánu ve vstupu", vec![r("s", 1, 0)], "s", None, 10, true, 0),
            ("jiný plán", vec![r("s", 1, 0)], "s", Some(2), 10, false, 1),
            ("jiná session", vec![r("a", 1, 0)], "s", Some(1), 10, false, 1),
            ("starý", vec![r("s", 1, 0)], "s", Some(1), RELEASED_MS, false, 0),
        ];
        for (what, mut mem, sid, k, now, want, left) in cases {
            assert_eq!(take_released(&mut mem, sid, k, now), want, "{what}");
            assert_eq!(mem.len(), left, "{what}");
        }
        let mut mem = vec![];
        for i in 0..(RELEASED_MAX as u64 + 5) {
            remember(&mut mem, r("s", i, 1000));
        }
        assert_eq!(mem.len(), RELEASED_MAX, "strop");
        remember(&mut mem, r("s", RELEASED_MAX as u64 + 4, 1000));
        assert_eq!(mem.iter().filter(|x| x.key == RELEASED_MAX as u64 + 4).count(), 1, "bez duplicit");
        remember(&mut mem, r("s", 999, 1000 + RELEASED_MS));
        assert_eq!(mem.len(), 1, "staré pryč");
    }

    /// Termín: prázdno (Claude se zeptá sám), karta pryč, plán zapamatovaný pro deduplikaci.
    #[test]
    fn timeout_passes_and_remembers() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_millis(200));
        assert_eq!(c.handle_plan(&req("s", "toolu_1", PLAN)), None);
        assert!(c.pending().is_empty());
        assert_eq!(c.handle_permission(&perm("s", json!({"plan": PLAN}))), Decision::Pass);
    }
}
