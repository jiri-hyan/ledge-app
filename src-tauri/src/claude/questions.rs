//! Otázky Clauda (`AskUserQuestion`) v Ledge: hook `PreToolUse` s matcherem `AskUserQuestion` →
//! `POST /question`. Ledge otázku ukáže jako chat (peek, karta Claude) a podrží požadavek, dokud šéf
//! neodpoví (`answer_question`), nepustí ji do okna Claude (`release_question`), nevyprší
//! `claude.questionHoldS` nebo neskončí turn. Odpověď = `permissionDecision: allow` + `updatedInput`
//! (původní vstup beze změny + `answers`), jinak prázdné tělo = Claude se zeptá normálně.
//! Čisté: `parse`, `build_answers`, `join_values`, `answer_body`. Mapa: `docs/codemap/sessions.md`.

use super::{Claude, Inner, Session, SessionState, now_ms, s};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::mpsc;
use std::time::Duration;

pub const TOOL: &str = "AskUserQuestion";
/// Nejdelší držení otázky: hook má timeout 600 s (`hooksetup::EVENTS`), 50 s rezerva.
pub const MAX_HOLD_S: u64 = 550;
/// Jak dlouho Ledge ukazuje otázku, kterou rovnou pustil do Claude (`questionHoldS` = 0).
const SHOW_ONLY_MS: u64 = 10 * 60 * 1000;
const MAX_QUESTIONS: usize = 4;
const MAX_OPTIONS: usize = 8;
const MAX_OTHER_CHARS: usize = 4000;
/// Více vybraných možností v jedné hodnotě `answers`. Ověřeno proti schématu Claude Code 2.1.259
/// (`sdk-tools.d.ts` + zdroj nástroje): `answers` = otázka → řetězec, „multi-select answers are
/// comma-separated“; pole CLI spojí stejně `", "` a výsledek porovná s popisky možností.
pub const MULTI_SEP: &str = ", ";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opt {
    pub label: String,
    pub description: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    /// přesný text od Clauda — klíč v `answers`
    pub question: String,
    pub header: String,
    pub options: Vec<Opt>,
    pub multi_select: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    pub id: u64,
    pub session_id: String,
    pub cwd: String,
    pub questions: Vec<Question>,
    pub created_at: u64,
    /// held: do kdy Ledge drží odpověď; jen ukázaná: do kdy ji Ledge ukazuje
    pub expires_at: u64,
    /// false = `questionHoldS` 0 — Ledge jen ukazuje, odpovídá se v okně Claude
    pub answerable: bool,
}

/// Záznam o otázce Clauda v deníku (posledních `MAX_LOG` událostí pro diagnostiku v Nastavení).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionLog {
    pub id: u64,
    pub at: u64,
    pub session_id: String,
    pub title: String,
    /// `holding` · `shown` · `answered` · `released` · `expired` · `turnEnded` · `skipped`
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

const MAX_LOG: usize = 20;

fn push_log(g: &mut Inner, entry: QuestionLog) {
    if let Some(existing) = g.question_log.iter_mut().find(|x| x.id != 0 && x.id == entry.id) {
        existing.at = entry.at;
        existing.outcome = entry.outcome;
        if entry.detail.is_some() {
            existing.detail = entry.detail;
        }
        return;
    }
    g.question_log.insert(0, entry);
    g.question_log.truncate(MAX_LOG);
}

/// Odpověď z UI na jednu otázku (pořadí jako `Ask.questions`): vybrané možnosti + vlastní text.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Reply {
    pub labels: Vec<String>,
    pub other: Option<String>,
}

pub(super) enum Outcome {
    Answer(Map<String, Value>),
    Release,
}

pub(super) type Held = (Ask, Option<mpsc::Sender<Outcome>>);

// ---------- čistá logika ----------

fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).filter(|t| !t.trim().is_empty()).map(str::to_string)
}

/// `tool_input` → otázky. Cokoli nečitelného (0 nebo > 4 otázek, chybí text, duplicitní text nebo
/// možnost) = `None` → Ledge nerozhoduje a otázka jde do okna Claude.
pub fn parse(input: &Value) -> Option<Vec<Question>> {
    let arr = input.get("questions")?.as_array()?;
    if arr.is_empty() || arr.len() > MAX_QUESTIONS {
        return None;
    }
    let mut out: Vec<Question> = Vec::with_capacity(arr.len());
    for q in arr {
        let question = text(q, "question")?;
        if out.iter().any(|x| x.question == question) {
            return None;
        }
        let raw = q.get("options").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        if raw.len() > MAX_OPTIONS {
            return None;
        }
        let mut options: Vec<Opt> = Vec::with_capacity(raw.len());
        for o in raw {
            let label = text(o, "label")?;
            if options.iter().any(|x| x.label == label) {
                return None;
            }
            options.push(Opt { label, description: text(o, "description").unwrap_or_default(), preview: text(o, "preview") });
        }
        out.push(Question {
            question,
            header: text(q, "header").unwrap_or_default(),
            options,
            multi_select: q.get("multiSelect").and_then(Value::as_bool).unwrap_or(false),
        });
    }
    Some(out)
}

pub fn join_values(vals: &[&str]) -> String {
    vals.join(MULTI_SEP)
}

fn name(q: &Question) -> String {
    let t = if q.header.is_empty() { &q.question } else { &q.header };
    if t.chars().count() > 40 { t.chars().take(40).collect::<String>() + "…" } else { t.clone() }
}

/// Odpovědi z UI → mapa `answers` (text otázky → hodnota). Možnosti v pořadí od Clauda, vlastní
/// text („Jiné“) na konec. Každá otázka musí mít odpověď; jednovýběr nejvýš jednu.
pub fn build_answers(qs: &[Question], replies: &[Reply]) -> Result<Map<String, Value>, String> {
    if qs.len() != replies.len() {
        return Err(crate::i18n::tr("Odpověď nesedí na otázky — zkus to znovu.", "The answer doesn't fit the questions — try again.").into());
    }
    let mut map = Map::new();
    for (q, r) in qs.iter().zip(replies) {
        if let Some(l) = r.labels.iter().find(|l| !q.options.iter().any(|o| &o.label == *l)) {
            return Err(trf!("Neznámá možnost „{l}“.", "Unknown option “{l}”."));
        }
        let mut vals: Vec<&str> = q.options.iter().filter(|o| r.labels.contains(&o.label)).map(|o| o.label.as_str()).collect();
        if let Some(t) = r.other.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            if t.chars().count() > MAX_OTHER_CHARS {
                return Err(trf!("Vlastní odpověď je delší než {MAX_OTHER_CHARS} znaků.", "Your own answer is longer than {MAX_OTHER_CHARS} characters."));
            }
            vals.push(t);
        }
        if vals.is_empty() {
            return Err(trf!("Chybí odpověď na „{}“.", "The answer to “{}” is missing.", name(q)));
        }
        if !q.multi_select && vals.len() > 1 {
            return Err(trf!("Na „{}“ jde jen jedna odpověď.", "“{}” takes only one answer.", name(q)));
        }
        map.insert(q.question.clone(), Value::String(join_values(&vals)));
    }
    Ok(map)
}

/// Tělo odpovědi hooku: povolit + `updatedInput` = původní vstup (vč. `questions`) + `answers`.
/// Samotné `allow` nestačí (dokumentace hooků) — vstup se vždy posílá celý.
pub fn answer_body(input: &Value, answers: Map<String, Value>) -> String {
    let mut updated = input.as_object().cloned().unwrap_or_default();
    updated.insert("answers".into(), Value::Object(answers));
    json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow", "updatedInput": updated}}).to_string()
}

fn wait_on(sessions: &mut std::collections::HashMap<String, Session>, sid: &str, cwd: &str, now: u64) {
    let sess = sessions.entry(sid.to_string()).or_insert_with(|| Session::new(sid, cwd, SessionState::Waiting, now));
    sess.state = SessionState::Waiting;
    sess.updated_at = now;
}

/// Hook události pro otázky (volá `Claude::handle_event` pod zámkem): konec turnu pustí všechny
/// otázky session do Claude, `PostToolUse` téhle otázky uklidí jen ukázané (odpověď padla v Claude).
pub(super) fn on_event(g: &mut Inner, ev: &str, sid: &str, tool: &str, now: u64) {
    let turn_over = matches!(ev, "UserPromptSubmit" | "Stop" | "SessionEnd");
    let answered_there = ev == "PostToolUse" && tool == TOOL;
    let mut ended_ids = Vec::new();
    g.asks.retain(|(a, tx)| {
        let gone = a.session_id == sid && (turn_over || (answered_there && tx.is_none()));
        if gone {
            if let Some(tx) = tx {
                let _ = tx.send(Outcome::Release);
            }
            ended_ids.push((a.id, a.session_id.clone(), a.questions.first().map(name).unwrap_or_default(), if answered_there { "answered" } else { "turnEnded" }, end_detail(ev)));
        }
        !gone && (tx.is_some() || a.expires_at > now)
    });
    for (id, s_id, title, out, detail) in ended_ids {
        push_log(g, QuestionLog { id, at: now, session_id: s_id, title, outcome: out.into(), detail: Some(detail.into()) });
    }
}

/// Lidská věta, proč otázka skončila mimo Ledge (do deníku v Nastavení, ne do logu).
fn end_detail(ev: &str) -> &'static str {
    match ev {
        "PostToolUse" => crate::i18n::tr("odpověď padla v Claude Code", "answered in Claude Code"),
        "UserPromptSubmit" => crate::i18n::tr("nový pokyn v Claude Code", "new prompt in Claude Code"),
        "SessionEnd" => crate::i18n::tr("session skončila", "the session ended"),
        _ => crate::i18n::tr("Claude ukončil tah", "Claude ended the turn"),
    }
}

impl Claude {
    pub fn set_question_hold(&self, secs: u64) {
        *self.question_hold.lock() = Duration::from_secs(secs.min(MAX_HOLD_S));
    }

    pub fn questions(&self) -> Vec<Ask> {
        let now = now_ms();
        self.inner.lock().asks.iter().filter(|(a, _)| a.expires_at > now).map(|(a, _)| a.clone()).collect()
    }

    pub fn question_log(&self) -> Vec<QuestionLog> {
        self.inner.lock().question_log.clone()
    }

    /// Čeká na šéfa: schválení nebo otázka (peek, jantarový proužek).
    pub fn attention(&self) -> bool {
        let now = now_ms();
        let g = self.inner.lock();
        !g.pending.is_empty() || g.asks.iter().any(|(a, _)| a.expires_at > now)
    }

    /// `POST /question` (PreToolUse). Blokuje do odpovědi / puštění / termínu. `None` = prázdná 200.
    pub fn handle_question(&self, body: &Value) -> Option<String> {
        if s(body, "tool_name") != TOOL {
            return None;
        }
        let sid = s(body, "session_id");
        let Some(input) = body.get("tool_input") else {
            let mut g = self.inner.lock();
            push_log(&mut g, QuestionLog { id: 0, at: now_ms(), session_id: sid, title: TOOL.into(), outcome: "skipped".into(), detail: Some(crate::i18n::tr("hook neposlal otázku", "the hook sent no question").into()) });
            return None;
        };
        let Some(questions) = parse(input) else {
            let mut g = self.inner.lock();
            push_log(&mut g, QuestionLog { id: 0, at: now_ms(), session_id: sid, title: TOOL.into(), outcome: "skipped".into(), detail: Some(crate::i18n::tr("otázku Ledge neumí ukázat (víc než 4 otázky, víc než 8 možností nebo stejné možnosti) — odpověz v Claude Code", "Ledge can't show this question (more than 4 questions, more than 8 options or duplicates) — answer in Claude Code").into()) });
            return None;
        };
        if sid.is_empty() {
            return None;
        }
        let hold = *self.question_hold.lock();
        let answerable = !hold.is_zero();
        let (tx, rx) = mpsc::channel();
        let id = {
            let mut g = self.inner.lock();
            let id = g.next_id;
            g.next_id += 1;
            let now = now_ms();
            let cwd = s(body, "cwd");
            let expires_at = now + if answerable { hold.as_millis() as u64 } else { SHOW_ONLY_MS };
            let first_title = questions.first().map(name).unwrap_or_else(|| TOOL.into());
            let q_count = questions.len();
            wait_on(&mut g.sessions, &sid, &cwd, now);
            g.asks.push((Ask { id, session_id: sid.clone(), cwd, questions, created_at: now, expires_at, answerable }, answerable.then_some(tx)));
            push_log(&mut g, QuestionLog {
                id,
                at: now,
                session_id: sid,
                title: first_title,
                outcome: if answerable { "holding" } else { "shown" }.into(),
                detail: Some(trf!("otázek: {q_count}", "questions: {q_count}")),
            });
            id
        };
        (self.on_change)();
        if !answerable {
            return None;
        }
        let out = match rx.recv_timeout(hold) {
            Ok(o) => Some(o),
            Err(_) => {
                // termín: uklidit pod zámkem; odpověď, která to stihla těsně před ním, platí
                let mut g = self.inner.lock();
                g.asks.retain(|(a, _)| a.id != id);
                push_log(&mut g, QuestionLog { id, at: now_ms(), session_id: String::new(), title: String::new(), outcome: "expired".into(), detail: None });
                drop(g);
                (self.on_change)();
                rx.try_recv().ok()
            }
        };
        match out {
            Some(Outcome::Answer(map)) => Some(answer_body(input, map)),
            _ => None,
        }
    }

    /// Klik šéfa v Ledge: odpovědi → Claude pokračuje. Chyba = česky pro `say()`.
    pub fn answer_question(&self, id: u64, replies: &[Reply]) -> Result<(), String> {
        let mut g = self.inner.lock();
        let i = g.asks.iter().position(|(a, tx)| a.id == id && tx.is_some()).ok_or_else(|| crate::i18n::tr("Otázka už v Ledge není — odpověz v okně Claude.", "The question is no longer in Ledge — answer in the Claude window."))?;
        let map = build_answers(&g.asks[i].0.questions, replies)?;
        let (a, tx) = g.asks.remove(i);
        if let Some(tx) = tx {
            let _ = tx.send(Outcome::Answer(map));
        }
        let now = now_ms();
        if let Some(sess) = g.sessions.get_mut(&a.session_id) {
            sess.state = SessionState::Working;
            sess.updated_at = now;
        }
        push_log(&mut g, QuestionLog { id: a.id, at: now, session_id: a.session_id, title: a.questions.first().map(name).unwrap_or_default(), outcome: "answered".into(), detail: None });
        drop(g);
        (self.on_change)();
        Ok(())
    }

    /// „Odpovědět v Claude“: otázka se ukáže v okně Claude (session dál čeká na šéfa).
    pub fn release_question(&self, id: u64) -> bool {
        let mut g = self.inner.lock();
        let Some(i) = g.asks.iter().position(|(a, _)| a.id == id) else { return false };
        let (a, tx) = g.asks.remove(i);
        if let Some(tx) = tx {
            let _ = tx.send(Outcome::Release);
        }
        push_log(&mut g, QuestionLog { id: a.id, at: now_ms(), session_id: a.session_id, title: a.questions.first().map(name).unwrap_or_default(), outcome: "released".into(), detail: None });
        drop(g);
        (self.on_change)();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn opt(l: &str) -> Value {
        json!({"label": l, "description": format!("popis {l}")})
    }
    fn q(text: &str, multi: bool, opts: &[&str]) -> Value {
        json!({"question": text, "header": "Hlava", "multiSelect": multi, "options": opts.iter().map(|l| opt(l)).collect::<Vec<_>>()})
    }
    fn input(qs: Vec<Value>) -> Value {
        json!({ "questions": qs })
    }
    fn reply(labels: &[&str], other: Option<&str>) -> Reply {
        Reply { labels: labels.iter().map(|x| x.to_string()).collect(), other: other.map(str::to_string) }
    }

    #[test]
    fn parse_table() {
        let four = (1..=4).map(|i| q(&format!("Q{i}?"), false, &["A", "B"])).collect::<Vec<_>>();
        let five = (1..=5).map(|i| q(&format!("Q{i}?"), false, &["A", "B"])).collect::<Vec<_>>();
        let cases: Vec<(&str, Value, Option<usize>)> = vec![
            ("jedna", input(vec![q("Kam?", false, &["A", "B"])]), Some(1)),
            ("čtyři", input(four), Some(4)),
            ("pět = nečitelné", input(five), None),
            ("žádná", input(vec![]), None),
            ("chybí questions", json!({}), None),
            ("questions není pole", json!({"questions": "x"}), None),
            ("chybí text otázky", input(vec![json!({"options": [opt("A")]})]), None),
            ("prázdný text", input(vec![q("  ", false, &["A"])]), None),
            ("duplicitní otázka", input(vec![q("Kam?", false, &["A"]), q("Kam?", false, &["B"])]), None),
            ("duplicitní možnost", input(vec![q("Kam?", false, &["A", "A"])]), None),
            ("možnost bez labelu", input(vec![json!({"question": "Kam?", "options": [{"description": "x"}]})]), None),
            ("bez možností = jen Jiné", input(vec![json!({"question": "Kam?"})]), Some(1)),
        ];
        for (name, v, want) in cases {
            assert_eq!(parse(&v).map(|x| x.len()), want, "{name}");
        }
        let p = parse(&input(vec![json!({"question": "Co?", "options": [{"label": "A", "preview": "<b>"}, {"label": "B"}], "multiSelect": true})])).unwrap();
        assert_eq!(p[0], Question {
            question: "Co?".into(),
            header: String::new(),
            options: vec![
                Opt { label: "A".into(), description: String::new(), preview: Some("<b>".into()) },
                Opt { label: "B".into(), description: String::new(), preview: None },
            ],
            multi_select: true,
        });
    }

    #[test]
    fn answers_table() {
        let qs = parse(&input(vec![q("Kam?", false, &["Praha", "Brno"]), q("Co?", true, &["A", "B", "C"])])).unwrap();
        let ok = |r: Vec<Reply>| build_answers(&qs, &r).map(Value::Object);
        // jednovýběr + vícevýběr (pořadí možností od Clauda, ne pořadí kliků)
        assert_eq!(ok(vec![reply(&["Brno"], None), reply(&["C", "A"], None)]), Ok(json!({"Kam?": "Brno", "Co?": "A, C"})));
        // Jiné: vlastní text (ořezaný); u vícevýběru na konec
        assert_eq!(ok(vec![reply(&[], Some("  Ostrava ")), reply(&["B"], Some("D"))]), Ok(json!({"Kam?": "Ostrava", "Co?": "B, D"})));
        // prázdné Jiné se nepočítá
        assert!(ok(vec![reply(&[], Some("   ")), reply(&["A"], None)]).unwrap_err().starts_with("Chybí odpověď na „Hlava“"));
        // jednovýběr: dvě odpovědi ne
        assert!(ok(vec![reply(&["Praha", "Brno"], None), reply(&["A"], None)]).unwrap_err().contains("jen jedna"));
        assert!(ok(vec![reply(&["Praha"], Some("x")), reply(&["A"], None)]).unwrap_err().contains("jen jedna"));
        // neznámá možnost, špatný počet odpovědí
        assert_eq!(ok(vec![reply(&["Plzeň"], None), reply(&["A"], None)]), Err("Neznámá možnost „Plzeň“.".into()));
        assert!(ok(vec![reply(&["Praha"], None)]).is_err());
        let long = "x".repeat(MAX_OTHER_CHARS + 1);
        assert!(ok(vec![reply(&[], Some(&long)), reply(&["A"], None)]).is_err());
    }

    #[test]
    fn body_keeps_original_input() {
        let inp = input(vec![q("Kam?", false, &["Praha", "Brno"])]);
        let mut m = Map::new();
        m.insert("Kam?".into(), json!("Brno"));
        let v: Value = serde_json::from_str(&answer_body(&inp, m)).unwrap();
        assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "allow");
        assert_eq!(v["hookSpecificOutput"]["updatedInput"]["questions"], inp["questions"]);
        assert_eq!(v["hookSpecificOutput"]["updatedInput"]["answers"], json!({"Kam?": "Brno"}));
    }

    fn ask_body(sid: &str) -> Value {
        json!({"hook_event_name": "PreToolUse", "tool_name": TOOL, "session_id": sid, "cwd": "C:/p", "tool_input": input(vec![q("Kam?", false, &["A", "B"])])})
    }

    fn wait_for_ask(c: &Claude) -> Ask {
        let t0 = std::time::Instant::now();
        loop {
            if let Some(a) = c.questions().into_iter().next() {
                return a;
            }
            assert!(t0.elapsed() < Duration::from_secs(5), "otázka se neobjevila");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn hold_answer_release_and_turn_end() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.set_question_hold(5);
        // odpověď z UI → tělo s answers, session zpět v práci
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        let a = wait_for_ask(&c);
        assert!(a.answerable);
        assert_eq!(c.sessions()[0].state, SessionState::Waiting);
        assert!(c.answer_question(a.id, &[reply(&["Z"], None)]).is_err(), "neznámá možnost otázku nezahodí");
        c.answer_question(a.id, &[reply(&["B"], None)]).unwrap();
        let body = h.join().unwrap().expect("odpověď");
        assert!(body.contains(r#""answers":{"Kam?":"B"}"#), "{body}");
        assert_eq!(c.sessions()[0].state, SessionState::Working);
        assert!(c.questions().is_empty() && !c.attention());
        // „Odpovědět v Claude“ → prázdné tělo, session dál čeká
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        let a = wait_for_ask(&c);
        assert!(c.release_question(a.id));
        assert_eq!(h.join().unwrap(), None);
        assert_eq!(c.sessions()[0].state, SessionState::Waiting);
        assert!(c.answer_question(a.id, &[reply(&["A"], None)]).is_err());
        // konec turnu (Esc v Claude, nový prompt) pustí otázku
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        wait_for_ask(&c);
        c.handle_event(&json!({"hook_event_name": "UserPromptSubmit", "session_id": "s", "prompt": "jinak"}));
        assert_eq!(h.join().unwrap(), None);
        assert!(c.questions().is_empty());
    }

    /// Vícevýběr naživo přes hook: 2+ zaškrtnuté možnosti dojdou Claudovi VŠECHNY, jedním řetězcem
    /// spojeným „, “ — přesně tvar, který Claude Code 2.1.x čeká (`answers`: otázka → řetězec,
    /// „multi-select answers are comma-separated“; pole by si sám spojil stejně). Víc otázek v jednom
    /// volání, „Jiné…“ víceřádkové a s unicode.
    /// Deník otázek (zadání týden 43, bod 7a): každá otázka jeden řádek, nejnovější nahoře, stav se
    /// přepisuje podle id (čeká → odpovězeno / předáno / konec tahu), nečitelné se zapíšou jako přeskočené.
    #[test]
    fn question_log_records_outcomes() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.set_question_hold(5);
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        let a = wait_for_ask(&c);
        assert_eq!(c.question_log()[0].outcome, "holding");
        c.answer_question(a.id, &[reply(&["B"], None)]).unwrap();
        h.join().unwrap();
        let log = c.question_log();
        assert_eq!((log.len(), log[0].id, log[0].outcome.as_str(), log[0].title.as_str()), (1, a.id, "answered", "Hlava"), "titulek = štítek otázky");
        // konec tahu
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&ask_body("s")));
        let b = wait_for_ask(&c);
        c.handle_event(&json!({"hook_event_name": "Stop", "session_id": "s"}));
        h.join().unwrap();
        let log = c.question_log();
        assert_eq!((log.len(), log[0].id, log[0].outcome.as_str()), (2, b.id, "turnEnded"));
        // nečitelná otázka = přeskočeno (id 0, neslučuje se)
        let bad = json!({"tool_name": TOOL, "session_id": "s", "tool_input": {"questions": []}});
        assert_eq!(c.handle_question(&bad), None);
        assert_eq!(c.handle_question(&bad), None);
        let log = c.question_log();
        assert_eq!((log.len(), log[0].outcome.as_str(), log[1].outcome.as_str()), (4, "skipped", "skipped"));
        // strop deníku
        for _ in 0..30 {
            c.handle_question(&bad);
        }
        assert_eq!(c.question_log().len(), MAX_LOG);
    }

    #[test]
    fn multi_select_sends_every_checked_option() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.set_question_hold(5);
        let inp = input(vec![q("Co spustit?", true, &["Testy", "Lint", "Build"]), q("Kam?", false, &["Praha", "Brno"]), q("Proč?", true, &["A", "B"])]);
        let body = json!({"tool_name": TOOL, "session_id": "s", "cwd": "C:/p", "tool_input": inp});
        let c2 = c.clone();
        let h = std::thread::spawn(move || c2.handle_question(&body));
        let a = wait_for_ask(&c);
        c.answer_question(a.id, &[reply(&["Build", "Testy"], None), reply(&[], Some("Ostrava\nčást Poruba 🙂")), reply(&["A", "B"], Some("  ještě tohle  "))]).unwrap();
        let out: Value = serde_json::from_str(&h.join().unwrap().expect("odpověď")).unwrap();
        let upd = &out["hookSpecificOutput"]["updatedInput"];
        assert_eq!(upd["answers"], json!({"Co spustit?": "Testy, Build", "Kam?": "Ostrava\nčást Poruba 🙂", "Proč?": "A, B, ještě tohle"}));
        assert_eq!(upd["questions"], inp["questions"], "otázky beze změny");
    }

    #[test]
    fn timeout_and_garbage_pass_to_claude() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        *c.question_hold.lock() = Duration::from_millis(150);
        let t0 = std::time::Instant::now();
        assert_eq!(c.handle_question(&ask_body("s")), None);
        assert!(t0.elapsed() >= Duration::from_millis(150));
        assert!(c.questions().is_empty());
        let cases = [
            json!({"tool_name": "Bash", "session_id": "s", "tool_input": {"command": "ls"}}),
            json!({"tool_name": TOOL, "session_id": "s", "tool_input": {"questions": []}}),
            json!({"tool_name": TOOL, "session_id": "", "tool_input": input(vec![q("Kam?", false, &["A"])])}),
            Value::Null,
        ];
        for b in cases {
            assert_eq!(c.handle_question(&b), None, "{b}");
        }
        assert!(c.questions().is_empty());
    }

    #[test]
    fn show_only_when_hold_is_zero() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.set_question_hold(0);
        assert_eq!(c.handle_question(&ask_body("s")), None, "pustí hned");
        let a = c.questions().pop().unwrap();
        assert!(!a.answerable && c.attention());
        assert!(c.answer_question(a.id, &[reply(&["A"], None)]).is_err(), "jen ukázaná se odpovědět nedá");
        // jiný nástroj otázku nechá, odpověď v Claude (PostToolUse AskUserQuestion) ji uklidí
        c.handle_event(&json!({"hook_event_name": "PostToolUse", "session_id": "s", "tool_name": "Bash"}));
        assert_eq!(c.questions().len(), 1);
        c.handle_event(&json!({"hook_event_name": "PostToolUse", "session_id": "s", "tool_name": TOOL}));
        assert!(c.questions().is_empty());
        c.set_question_hold(10_000);
        assert_eq!(*c.question_hold.lock(), Duration::from_secs(MAX_HOLD_S), "strop");
    }

    #[test]
    fn config_old_and_new() {
        let old: crate::config::ClaudeConfig = serde_json::from_str(r#"{"enabled":true,"port":47800,"approvalTimeoutS":45}"#).unwrap();
        assert_eq!(old.question_hold_s, 540, "starý config = otázky drží Ledge");
        let off: crate::config::ClaudeConfig = serde_json::from_str(r#"{"questionHoldS":0}"#).unwrap();
        assert_eq!(off.question_hold_s, 0);
    }

    #[test]
    fn permission_for_ask_user_question_passes_at_once() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(30));
        let t0 = std::time::Instant::now();
        let d = c.handle_permission(&json!({"session_id": "s", "cwd": "C:/p", "tool_name": TOOL, "tool_input": input(vec![q("Kam?", false, &["A"])])}));
        assert_eq!(d, super::super::Decision::Pass);
        assert!(t0.elapsed() < Duration::from_secs(2));
        assert!(c.pending().is_empty(), "žádná povolovací karta s JSONem");
        assert_eq!(c.sessions()[0].state, SessionState::Waiting, "otázka čeká v okně Claude");
    }
}
