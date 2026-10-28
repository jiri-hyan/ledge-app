//! Most pro hooky Claude Code (jen 127.0.0.1).
//!
//! - `POST /permission` ← hook `PermissionRequest` (typ http). Ledge požadavek podrží, dokud šéf
//!   nerozhodne. **Povolit jen explicitně** (klik / zkratka). Timeout, chyba nebo vyřízení jinde
//!   → prázdná 2xx odpověď = Claude Code se zeptá normálně v terminálu. Nikdy auto-allow.
//! - `POST /event` ← hooky SessionStart / UserPromptSubmit / PostToolUse / Stop / SessionEnd /
//!   Notification / SubagentStart / SubagentStop → stav sessions (pracuje / čeká / hotovo) a jejich
//!   subagenti (`subagents.rs`). Při `Stop` navíc hlídač nedokončené práce (`claude.enforceCommit`):
//!   v režimu `block` může odpovědět `{"decision":"block","reason":…}`.
//! - `POST /question` ← hook `PreToolUse` (matcher `AskUserQuestion`): otázku ukáže jako chat a
//!   odpověď z Ledge vrátí Claudovi (`questions.rs`). `PermissionRequest` pro `AskUserQuestion` se
//!   hned pouští (`Pass`) — nikdy povolovací karta s JSONem.
//! - `POST /plan` ← hook `PreToolUse` (matcher `ExitPlanMode`): karta „Plán“ (`planhook.rs`); případný
//!   `PermissionRequest` téhož plánu kartu nezaloží znovu.
//! - `GET /status` → JSON pro Clauda a skripty.
//!
//! Ochrana: požadavky s hlavičkou `Origin` (prohlížeč) se odmítají; Host musí být loopback.
//!
//! Podmoduly: `bridge` (HTTP server), `hygiene` (hlídač nedokončené práce, git), `paths` (cesty a
//! porcelain, čisté), `questions` (otázky Clauda) + `unseen` (pojistka neviděné otázky), `rules` („Vždy povolit“ a důvod zamítnutí, čisté),
//! `preview` + `diff` (náhled změny Edit/Write v kartě), `plan` + `planhook` (review plánu ExitPlanMode). Tady stav
//! session a schvalování. Mapa: `docs/codemap/sessions.md`.

pub mod activity;
mod bridge;
pub mod diff;
mod hygiene;
pub(crate) mod paths;
pub mod plan;
mod planhook;
pub mod preview;
mod questions;
pub mod rules;
pub mod unseen;
#[cfg(test)]
mod activity_tests;
#[cfg(test)]
mod testkit;

pub use bridge::{Ctl, Mcp, serve};
pub use questions::{Ask, QuestionLog, Reply};

use crate::config::EnforceCommit;
use crate::subagents::Subagents;
use hygiene::{BASELINE_ATTEMPTS, Budgets, Track, edited_path, fingerprint, git_baseline};
use paths::absolutize;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    /// Povolit + pravidlo v `updatedPermissions` — zapíše ho Claude Code sám (`rules.rs`)
    AllowAlways(rules::Grant),
    Deny,
    /// Zamítnout s důvodem pro Clauda (`decision.message`), už očištěný `rules::clean_reason`
    DenyWith(String),
    /// plán: povolit + `setMode` pro session (`plan.rs`)
    AllowMode(plan::Mode),
    /// plán: vrátit s připomínkou (`decision.message`), už očištěnou `rules::clean_reason`
    ReturnPlan(String),
    /// nech to na terminálu
    Pass,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub id: u64,
    pub session_id: String,
    pub cwd: String,
    pub tool: String,
    /// lidsky čitelné shrnutí (příkaz, soubor, URL…)
    pub summary: String,
    pub created_at: u64,
    pub expires_at: u64,
    /// návrh „Vždy povolit“ (přesné pravidlo); `None` = tlačítko se neukáže
    pub always: Option<rules::Always>,
    /// náhled změny Edit/Write (`preview.rs`) — sestavený jednou při příchodu žádosti
    pub preview: Option<preview::Preview>,
    /// plán k review (`ExitPlanMode`, `plan.rs`); `Some` = karta plánu místo obyčejného schválení
    pub plan: Option<plan::Plan>,
    /// odkud karta přišla (`/permission` nebo `/plan`) — tvar odpovědi a nabídka akcí plánu
    pub via: plan::Via,
    /// `tool_use_id` z `PreToolUse` (deduplikace karet plánu); `PermissionRequest` ho nemá
    #[serde(skip)]
    pub tool_use_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Working,
    Waiting,
    Idle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub cwd: String,
    pub state: SessionState,
    pub title: Option<String>,
    pub updated_at: u64,
    /// Soubory, které session SAMA upravila (Edit/Write/MultiEdit/NotebookEdit), nejsou zacommitované
    /// a od její poslední úpravy na ně nikdo jiný nesáhl — relativně ke kořeni repa.
    pub uncommitted: Vec<String>,
    /// Commity od startu session (`baseline..HEAD`) na jejích souborech, které nejsou na žádném remote.
    /// Jen když má větev upstream — bez něj je push nesplnitelný požadavek → 0.
    pub unpushed: u32,
    /// Session skončila (`SessionEnd`), ale drží se v seznamu, protože práci nedokončila.
    pub ended: bool,
    /// běžící a nedávno dokončení subagenti (hooky `SubagentStart` / `SubagentStop`)
    pub subagents: Subagents,
    /// `transcript_path` z posledního hooku (kontext session čte `ctxsize`, jen na žádost UI)
    #[serde(skip)]
    pub transcript: Option<String>,
    /// živá aktivita z hooků (fáze = odhad podle posledního nástroje, soubor, poslední zpráva; `activity.rs`)
    pub activity: Option<activity::Activity>,
}

impl Session {
    fn new(id: &str, cwd: &str, state: SessionState, now: u64) -> Self {
        Self { id: id.to_string(), cwd: cwd.to_string(), state, title: None, updated_at: now, uncommitted: vec![], unpushed: 0, ended: false, subagents: Subagents::default(), activity: None, transcript: None }
    }

    /// Nedokončená práce: nezacommitované vlastní soubory nebo nepushnuté vlastní commity.
    pub fn unfinished(&self) -> bool {
        !self.uncommitted.is_empty() || self.unpushed > 0
    }
}

pub type OnChange = Arc<dyn Fn() + Send + Sync>;

struct Inner {
    next_id: u64,
    pending: Vec<(Pending, mpsc::Sender<Decision>)>,
    sessions: HashMap<String, Session>,
    track: HashMap<String, Track>,
    /// otázky Clauda (`questions.rs`); `None` sender = jen ukázaná (`questionHoldS` 0)
    asks: Vec<questions::Held>,
    /// deník posledních otázek Clauda (`questions.rs`, jen paměť)
    question_log: Vec<questions::QuestionLog>,
    /// plány z `PreToolUse`, které Ledge pustil do Claude (`planhook.rs`, deduplikace)
    released: Vec<planhook::Released>,
}

#[derive(Clone)]
pub struct Claude {
    inner: Arc<Mutex<Inner>>,
    on_change: OnChange,
    timeout: Arc<Mutex<Duration>>,
    /// jak dlouho drží otázku Clauda (`claude.questionHoldS`); 0 = jen ukázat
    question_hold: Arc<Mutex<Duration>>,
    /// pojistka otázek (`claude.questionUnseenS`): drženou otázku, kterou UI do té doby neukázalo,
    /// Ledge pustí do Claude; 0 = vypnuto (`questions::step`)
    question_unseen: Arc<Mutex<Duration>>,
    enforce: Arc<Mutex<EnforceCommit>>,
    /// číst poslední zprávu z přepisu při `Stop` (karta „Claude aktivita“ je na záložce; `activity.rs`)
    activity_tail: Arc<std::sync::atomic::AtomicBool>,
    /// testy: práce „na pozadí“ běží hned ve volajícím vlákně (deterministicky)
    inline: bool,
    /// termíny gitu hlídače (testy je prodlouží)
    budgets: Budgets,
}

use crate::util::now_ms;

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Krátké shrnutí toho, co chce Claude udělat.
pub fn summarize(tool: &str, input: &Value) -> String {
    let pick = |k: &str| input.get(k).and_then(Value::as_str).map(str::to_string);
    let text = match tool {
        "Bash" | "PowerShell" => pick("command"),
        "Edit" | "Write" | "Read" | "NotebookEdit" => pick("file_path").or_else(|| pick("notebook_path")),
        "WebFetch" => pick("url"),
        "WebSearch" => pick("query"),
        _ => None,
    }
    .unwrap_or_else(|| {
        let t = input.to_string();
        if t == "null" { String::new() } else { t }
    });
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > 300 { one_line.chars().take(300).collect::<String>() + "…" } else { one_line }
}

/// Nejdelší čekání na schválení: hook `PermissionRequest` má timeout 600 s (`hooksetup::EVENTS`),
/// 50 s rezerva — stejně jako `questions::MAX_HOLD_S` a posuvník v Nastavení (5–550).
pub const MAX_APPROVAL: Duration = Duration::from_secs(550);

impl Claude {
    pub fn new(on_change: OnChange, timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner { next_id: 1, pending: vec![], sessions: HashMap::new(), track: HashMap::new(), asks: vec![], question_log: vec![], released: vec![] })),
            on_change,
            timeout: Arc::new(Mutex::new(timeout.min(MAX_APPROVAL))),
            question_hold: Arc::new(Mutex::new(Duration::from_secs(questions::MAX_HOLD_S))),
            question_unseen: Arc::new(Mutex::new(Duration::ZERO)),
            enforce: Arc::new(Mutex::new(EnforceCommit::default())),
            activity_tail: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            inline: false,
            budgets: Budgets::default(),
        }
    }

    #[cfg(test)]
    fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// Testy: git smí trvat dlouho — výsledek pak nezávisí na vytížení stroje.
    #[cfg(test)]
    fn patient(mut self) -> Self {
        let long = Duration::from_secs(60);
        self.budgets = Budgets { baseline: long, hygiene: long };
        self
    }

    fn background(&self, f: impl FnOnce(&Claude) + Send + 'static) {
        if self.inline {
            f(self);
        } else {
            let me = self.clone();
            std::thread::spawn(move || f(&me));
        }
    }

    pub fn set_enforce(&self, mode: EnforceCommit) {
        *self.enforce.lock() = mode;
    }

    /// Strop `MAX_APPROVAL`: ručně zapsaný `approvalTimeoutS` nad timeoutem hooku by nechal v Ledge
    /// viset kartu „Povolit“, na kterou Claude už nečeká (a obří číslo by přeteklo `expires_at`).
    pub fn set_timeout(&self, t: Duration) {
        *self.timeout.lock() = t.min(MAX_APPROVAL);
    }

    pub fn pending(&self) -> Vec<Pending> {
        self.inner.lock().pending.iter().map(|(p, _)| p.clone()).collect()
    }

    /// Cesta k transcriptu session podle hooku (`transcript_path`); bez hooku `None`.
    pub fn transcript_of(&self, id: &str) -> Option<String> {
        self.inner.lock().sessions.get(id).and_then(|s| s.transcript.clone())
    }

    pub fn sessions(&self) -> Vec<Session> {
        let mut v: Vec<Session> = self.inner.lock().sessions.values().cloned().collect();
        v.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
        v
    }

    /// Rozhodnutí šéfa. `None` = nejstarší čekající.
    pub fn decide(&self, id: Option<u64>, d: Decision) -> bool {
        self.decide_by(id, |_| Some(d))
    }

    /// Klik s doplňkem (`decide(id, allow, extra?)`): „Vždy povolit“ do cíle / důvod zamítnutí. Pravidlo
    /// bere z čekající žádosti (to, co karta ukázala), nikdy z IPC. „Vždy“ jen pro konkrétní `id`.
    /// U plánu rozhoduje `plan::resolve` (režim / připomínka; „Vždy“ ne), jinde režim nedává smysl.
    pub fn decide_with(&self, id: Option<u64>, allow: bool, extra: &rules::Extra) -> bool {
        let plan_only = extra.mode.is_some() || extra.in_claude;
        if (extra.always.is_some() || plan_only) && id.is_none() {
            return false;
        }
        self.decide_by(id, |p| match (&p.plan, plan_only) {
            (Some(_), _) => plan::resolve(allow, extra, p.via),
            (None, true) => None,
            (None, false) => rules::resolve(allow, extra, p.always.as_ref()),
        })
    }

    /// `f` z čekající žádosti udělá rozhodnutí; `None` = nic se nerozhodne, žádost čeká dál.
    fn decide_by(&self, id: Option<u64>, f: impl FnOnce(&Pending) -> Option<Decision>) -> bool {
        let mut g = self.inner.lock();
        let idx = match id {
            Some(id) => g.pending.iter().position(|(p, _)| p.id == id),
            None => if g.pending.is_empty() { None } else { Some(0) },
        };
        let Some(i) = idx else { return false };
        let Some(d) = f(&g.pending[i].0) else { return false };
        let pass = d == Decision::Pass;
        let (p, tx) = g.pending.remove(i);
        let _ = tx.send(d);
        // „Vybrat režim v Claude“ = dál čeká, jen v okně Claude
        if !pass && let Some(sess) = g.sessions.get_mut(&p.session_id) {
            sess.state = SessionState::Working;
            sess.updated_at = now_ms();
        }
        drop(g);
        (self.on_change)();
        true
    }

    /// Obslouží PermissionRequest — blokuje volající vlákno do rozhodnutí/timeoutu.
    pub fn handle_permission(&self, body: &Value) -> Decision {
        let tool = s(body, "tool_name");
        if tool.is_empty() {
            // nečitelný požadavek → žádná prázdná karta v UI, rozhodne terminál
            return Decision::Pass;
        }
        if tool == questions::TOOL {
            // otázka už je v okně Claude (Ledge ji pustil, nebo hook otázek chybí) → tam se odpoví
            let (sid, cwd, now) = (s(body, "session_id"), s(body, "cwd"), now_ms());
            if !sid.is_empty() {
                let mut g = self.inner.lock();
                let sess = g.sessions.entry(sid.clone()).or_insert_with(|| Session::new(&sid, &cwd, SessionState::Waiting, now));
                sess.state = SessionState::Waiting;
                sess.updated_at = now;
                drop(g);
                (self.on_change)();
            }
            return Decision::Pass;
        }
        let input = body.get("tool_input").cloned().unwrap_or(Value::Null);
        let session_id = s(body, "session_id");
        let cwd = s(body, "cwd");
        // čtení souborů jen teď (1×, s limitem a termínem), mimo zámek; snapshot už jen kopíruje
        let preview = preview::for_request(&tool, &input, &cwd);
        let is_plan = tool == plan::TOOL;
        let plan = if is_plan { plan::for_request(&input, &cwd) } else { None };
        let summary = match &plan {
            Some(p) if !p.title.is_empty() => p.title.clone(),
            _ => summarize(&tool, &input),
        };
        let always = if plan.is_some() { None } else { rules::propose(&tool, &input, body.get("permission_suggestions")) };
        let (key, sid) = (plan.as_ref().map(planhook::key), session_id.clone());
        let p = Pending { id: 0, session_id, cwd, tool, summary, created_at: 0, expires_at: 0, always, preview, plan, via: plan::Via::Permission, tool_use_id: None };
        // plán už řeší / řešila karta z `PreToolUse` → žádná druhá karta, rozhodne okno Claude
        self.hold(p, |g, now| is_plan && planhook::duplicate(g, &sid, key, now)).unwrap_or(Decision::Pass)
    }

    /// Vystaví kartu (doplní `id` a časy, session → čeká) a blokuje do rozhodnutí šéfa / termínu
    /// (`Pass`). `reject` pod týmž zámkem: `true` = karta nevznikne → `None`.
    fn hold(&self, mut p: Pending, reject: impl FnOnce(&mut Inner, u64) -> bool) -> Option<Decision> {
        let timeout = *self.timeout.lock();
        let (tx, rx) = mpsc::channel();
        let id = {
            let mut g = self.inner.lock();
            let now = now_ms();
            if reject(&mut g, now) {
                return None;
            }
            let id = g.next_id;
            g.next_id += 1;
            (p.id, p.created_at, p.expires_at) = (id, now, now + timeout.as_millis() as u64);
            let (sid, cwd) = (p.session_id.clone(), p.cwd.clone());
            g.pending.push((p, tx));
            let sess = g.sessions.entry(sid.clone()).or_insert_with(|| Session::new(&sid, &cwd, SessionState::Waiting, now));
            sess.state = SessionState::Waiting;
            sess.updated_at = now;
            id
        };
        (self.on_change)();
        let d = rx.recv_timeout(timeout).unwrap_or(Decision::Pass);
        // timeout → uklidit (decide() už položku odebralo, pokud rozhodl šéf)
        let mut g = self.inner.lock();
        let before = g.pending.len();
        g.pending.retain(|(p, _)| p.id != id);
        let removed = g.pending.len() != before;
        drop(g);
        if removed {
            (self.on_change)();
        }
        Some(d)
    }

    /// Hook události mimo schvalování → stav session. Vrací tělo odpovědi (`Some` jen když `Stop`
    /// v režimu `block` vrací session do práce), jinak `None` = prázdná 200.
    ///
    /// Git (baseline, hygiena) nikdy pod zámkem stavu. Synchronně jen `Stop` v režimu `block`
    /// (odpověď na něm závisí); jinak běží na pozadí a výsledek přijde přes `on_change`.
    pub fn handle_event(&self, body: &Value) -> Option<String> {
        let ev = s(body, "hook_event_name");
        let sid = s(body, "session_id");
        if sid.is_empty() {
            return None;
        }
        let mode = *self.enforce.lock();
        let body_cwd = s(body, "cwd");
        let now = now_ms();
        let mut g = self.inner.lock();
        // Šéf odpověděl v terminálu → čekající žádost téže session je vyřízená jinde.
        if matches!(ev.as_str(), "PostToolUse" | "UserPromptSubmit" | "Stop" | "SessionEnd") {
            let mut i = 0;
            while i < g.pending.len() {
                if g.pending[i].0.session_id == sid {
                    let (_, tx) = g.pending.remove(i);
                    let _ = tx.send(Decision::Pass);
                } else {
                    i += 1;
                }
            }
        }
        questions::on_event(&mut g, &ev, &sid, &s(body, "tool_name"), now);
        let mut baseline_cwd = None;
        let mut check = false;
        let mut tail = None;
        if ev != "SessionEnd" || g.sessions.contains_key(&sid) {
            let Inner { sessions, track, .. } = &mut *g;
            let sess = sessions.entry(sid.clone()).or_insert_with(|| Session::new(&sid, &body_cwd, SessionState::Idle, now));
            let tr = track.entry(sid.clone()).or_default();
            if !body_cwd.is_empty() {
                sess.cwd = body_cwd;
            }
            let tp = s(body, "transcript_path");
            if !tp.is_empty() {
                sess.transcript = Some(tp);
            }
            sess.updated_at = now;
            if let Some(a) = activity::next(sess.activity.as_ref(), &ev, body, now) {
                sess.activity = Some(a);
            }
            if self.tail_wanted(&ev, body) {
                tail = sess.transcript.clone().map(|t| (t, sess.activity.as_ref().map_or(now, |a| a.at)));
            }
            // resume pod stejným session_id → už není ukončená
            sess.ended = ev == "SessionEnd";
            match ev.as_str() {
                "UserPromptSubmit" => {
                    sess.state = SessionState::Working;
                    let title = s(body, "session_title");
                    let prompt = s(body, "prompt");
                    let t = if !title.is_empty() { title } else { prompt };
                    // bez `<system-reminder>` a podobných bloků; samé bloky = titulek zatím žádný
                    if let Some(t) = crate::sessions::clean_preview(&t).filter(|_| sess.title.is_none()) {
                        sess.title = Some(if t.chars().count() > 60 { t.chars().take(60).collect::<String>() + "…" } else { t });
                    }
                }
                "PostToolUse" => {
                    sess.state = SessionState::Working;
                    let input = body.get("tool_input").unwrap_or(&Value::Null);
                    if let Some(p) = edited_path(&s(body, "tool_name"), input) {
                        let abs = absolutize(&p, &sess.cwd);
                        let fp = fingerprint(&abs);
                        tr.record(abs, fp);
                    }
                }
                "PreToolUse" | "PostToolUseFailure" => sess.state = SessionState::Working,
                "SubagentStart" => {
                    sess.state = SessionState::Working;
                    sess.subagents.start(&s(body, "agent_id"), &s(body, "agent_type"), now);
                }
                "SubagentStop" => sess.subagents.stop(&s(body, "agent_id"), &s(body, "agent_type"), &s(body, "last_assistant_message"), now),
                "Stop" => sess.state = SessionState::Idle,
                "SessionEnd" => {
                    sess.state = SessionState::Idle;
                    sess.subagents.clear_running();
                }
                "Notification" if s(body, "notification_type") == "idle_prompt" => sess.state = SessionState::Idle,
                "SessionStart" => sess.state = SessionState::Idle,
                _ => {}
            }
            if mode == EnforceCommit::Off {
                if matches!(ev.as_str(), "Stop" | "SessionEnd") {
                    sess.uncommitted.clear();
                    sess.unpushed = 0;
                }
            } else {
                // baseline při prvním eventu (SessionStart hooknutý není); při selhání znovu, max. 3×
                if ev != "SessionEnd" && !tr.baseline_ok && !tr.baseline_busy && tr.baseline_attempts < BASELINE_ATTEMPTS {
                    tr.baseline_busy = true;
                    tr.baseline_attempts += 1;
                    baseline_cwd = Some(sess.cwd.clone());
                }
                // bez známého repa (a bez baseline v běhu) není co kontrolovat → rozhodne se hned
                check = matches!(ev.as_str(), "Stop" | "SessionEnd") && (tr.repo.is_some() || baseline_cwd.is_some());
            }
        }
        // ukončená session bez nedokončené práce pryč hned (kontrola ji případně odebere i později)
        if ev == "SessionEnd" && !check && g.sessions.get(&sid).is_some_and(|x| !x.unfinished()) {
            g.sessions.remove(&sid);
        }
        // staré nečinné sessions pryč (12 h)
        g.sessions.retain(|_, s| now.saturating_sub(s.updated_at) < 12 * 3600 * 1000);
        let Inner { sessions, track, .. } = &mut *g;
        track.retain(|k, _| sessions.contains_key(k));
        drop(g);
        (self.on_change)();
        if let Some((path, at)) = tail {
            let sid = sid.clone();
            self.background(move |c| {
                let root = crate::sessionlive::claude_dir().map(|d| d.join("projects"));
                c.store_message(&sid, at, activity::read_last_message(Path::new(&path), root.as_deref()));
            });
        }

        let is_stop = ev == "Stop";
        let active = body.get("stop_hook_active").and_then(Value::as_bool).unwrap_or(false);
        let work = move |c: &Claude| {
            if let Some(cwd) = baseline_cwd {
                c.store_baseline(&sid, git_baseline(&cwd, c.budgets.baseline));
            }
            if check { c.run_check(&sid, is_stop, mode, active) } else { None }
        };
        if is_stop && mode == EnforceCommit::Block {
            work(self)
        } else {
            self.background(move |c| {
                work(c);
            });
            None
        }
    }

    pub fn status_json(&self) -> Value {
        let mut sessions = json!(self.sessions());
        activity::redact(&mut sessions);
        json!({ "pending": self.pending(), "questions": self.questions(), "sessions": sessions })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_lifecycle() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.handle_event(&json!({"hook_event_name":"UserPromptSubmit","session_id":"a","cwd":"C:/p","prompt":"oprav   přihlášení\na testy"}));
        assert_eq!(c.sessions()[0].state, SessionState::Working);
        assert_eq!(c.sessions()[0].title.as_deref(), Some("oprav přihlášení a testy"));
        // prompt jen ze systémových bloků titulek nezaloží; další skutečný ano, bez bloků
        c.handle_event(&json!({"hook_event_name":"UserPromptSubmit","session_id":"b","cwd":"C:/p","prompt":"<system-reminder>\nLinked sessions now</system-reminder>"}));
        assert_eq!(c.sessions().iter().find(|s| s.id == "b").unwrap().title, None);
        c.handle_event(&json!({"hook_event_name":"UserPromptSubmit","session_id":"b","cwd":"C:/p","prompt":"<system-reminder>x</system-reminder>\nnasaď build"}));
        assert_eq!(c.sessions().iter().find(|s| s.id == "b").unwrap().title.as_deref(), Some("nasaď build"));
        c.handle_event(&json!({"hook_event_name":"SessionEnd","session_id":"b"}));
        c.handle_event(&json!({"hook_event_name":"Stop","session_id":"a"}));
        assert_eq!(c.sessions()[0].state, SessionState::Idle);
        c.handle_event(&json!({"hook_event_name":"SessionEnd","session_id":"a"}));
        assert!(c.sessions().is_empty());
    }

    /// Ručně zapsaný `approvalTimeoutS` nad timeoutem hooku (600 s) / obří číslo: karta v Ledge
    /// nesmí čekat déle, než Claude čeká na odpověď, a `expires_at` nesmí přetéct.
    #[test]
    fn approval_timeout_is_capped_below_hook_timeout() {
        for secs in [551, 3600, u64::MAX] {
            let c = Claude::new(Arc::new(|| {}), Duration::from_secs(secs));
            assert_eq!(*c.timeout.lock(), MAX_APPROVAL, "new {secs}");
            c.set_timeout(Duration::from_secs(5));
            c.set_timeout(Duration::from_secs(secs));
            assert_eq!(*c.timeout.lock(), MAX_APPROVAL, "set {secs}");
            let c2 = c.clone();
            let h = std::thread::spawn(move || c2.handle_permission(&json!({"session_id": "s", "cwd": "C:/p", "tool_name": "Bash", "tool_input": {"command": "ls"}})));
            let t0 = std::time::Instant::now();
            let p = loop {
                if let Some(p) = c.pending().pop() {
                    break p;
                }
                assert!(t0.elapsed() < Duration::from_secs(5));
                std::thread::sleep(Duration::from_millis(5));
            };
            assert_eq!(p.expires_at - p.created_at, MAX_APPROVAL.as_millis() as u64);
            assert!(c.decide(Some(p.id), Decision::Deny));
            assert_eq!(h.join().unwrap(), Decision::Deny);
        }
    }

    /// Čeká, až `handle_permission` ve vlákně vystaví žádost.
    fn wait_pending(c: &Claude) -> Pending {
        let t0 = std::time::Instant::now();
        loop {
            if let Some(p) = c.pending().pop() {
                return p;
            }
            assert!(t0.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Náhled Write se sestaví při příchodu žádosti ze skutečného souboru (tempdir) a jde se žádostí.
    #[test]
    fn write_request_carries_preview() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("a.txt"), "a\nb\n").unwrap();
        let dir = t.path().to_string_lossy().replace('\\', "/");
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(5));
        let c2 = c.clone();
        let req = json!({"session_id": "s", "cwd": dir, "tool_name": "Write", "tool_input": {"file_path": "a.txt", "content": "a\nB\n"}});
        let h = std::thread::spawn(move || c2.handle_permission(&req));
        let p = wait_pending(&c);
        let pv = p.preview.expect("náhled");
        assert_eq!((pv.added, pv.removed, pv.numbered, pv.file.as_str()), (1, 1, true, "a.txt"));
        assert!(p.plan.is_none());
        assert!(c.decide(Some(p.id), Decision::Deny));
        assert_eq!(h.join().unwrap(), Decision::Deny);
    }

    /// ExitPlanMode: karta plánu (titulek, bez „Vždy povolit“), režim jen u plánu, připomínka = vrácení.
    #[test]
    fn plan_request_and_actions() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(5));
        let run = |c: &Claude, input: Value| {
            let c2 = c.clone();
            let req = json!({"session_id": "s", "cwd": "C:/p", "tool_name": "ExitPlanMode", "tool_input": input,
                "permission_suggestions": [{"type": "addRules", "rules": [{"toolName": "ExitPlanMode"}], "behavior": "allow", "destination": "localSettings"}]});
            std::thread::spawn(move || c2.handle_permission(&req))
        };
        let h = run(&c, json!({"plan": "# Oprava přihlášení\n\n1. testy"}));
        let p = wait_pending(&c);
        assert_eq!(p.summary, "Oprava přihlášení");
        assert!(p.always.is_none(), "u plánu nikdy Vždy povolit");
        assert_eq!(p.plan.as_ref().map(|x| x.title.as_str()), Some("Oprava přihlášení"));
        let mode = rules::Extra { mode: Some(plan::Mode::AcceptEdits), ..Default::default() };
        assert!(!c.decide_with(None, true, &mode), "režim jen s konkrétním id");
        assert!(c.decide_with(Some(p.id), true, &mode));
        assert_eq!(h.join().unwrap(), Decision::AllowMode(plan::Mode::AcceptEdits));

        let h = run(&c, json!({"plan": "# P"}));
        let p = wait_pending(&c);
        assert!(c.decide_with(Some(p.id), false, &rules::Extra { reason: Some("nejdřív testy".into()), ..Default::default() }));
        assert_eq!(h.join().unwrap(), Decision::ReturnPlan("nejdřív testy".into()));

        // bez plánu (prázdný vstup) = obyčejná karta; režim tam nic nerozhodne
        let h = run(&c, json!({}));
        let p = wait_pending(&c);
        assert!(p.plan.is_none());
        assert!(!c.decide_with(Some(p.id), true, &mode));
        assert!(c.decide(None, Decision::Allow), "zkratka dál schválí nejstarší");
        assert_eq!(h.join().unwrap(), Decision::Allow);
    }

    #[test]
    fn subagents_from_hooks() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        c.handle_event(&json!({"hook_event_name":"SubagentStart","session_id":"a","cwd":"C:/p","agent_id":"x1","agent_type":"Explore"}));
        c.handle_event(&json!({"hook_event_name":"SubagentStart","session_id":"a","agent_id":"x2","agent_type":"general-purpose"}));
        let s = &c.sessions()[0];
        assert_eq!(s.state, SessionState::Working);
        assert_eq!(s.subagents.running.iter().map(|r| r.agent_type.as_str()).collect::<Vec<_>>(), ["Explore", "general-purpose"]);
        c.handle_event(&json!({"hook_event_name":"SubagentStop","session_id":"a","agent_id":"x1","agent_type":"Explore","last_assistant_message":"Našel jsem to.","stop_hook_active":false}));
        let s = &c.sessions()[0];
        assert_eq!(s.subagents.running.len(), 1);
        assert_eq!(s.subagents.done[0].message.as_deref(), Some("Našel jsem to."));
        c.handle_event(&json!({"hook_event_name":"Stop","session_id":"a"}));
        assert_eq!(c.sessions()[0].subagents.running.len(), 1, "Stop hlavní session subagenty na pozadí neukončí");
    }

    #[test]
    fn transcript_path_from_hooks() {
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1));
        assert_eq!(c.transcript_of("a"), None);
        c.handle_event(&json!({"hook_event_name":"UserPromptSubmit","session_id":"a","cwd":"C:/p","prompt":"x"}));
        assert_eq!(c.transcript_of("a"), None, "hook bez cesty = nic");
        c.handle_event(&json!({"hook_event_name":"PreToolUse","session_id":"a","transcript_path":"C:/u/.claude/projects/p/a.jsonl"}));
        c.handle_event(&json!({"hook_event_name":"Stop","session_id":"a","transcript_path":""}));
        assert_eq!(c.transcript_of("a").as_deref(), Some("C:/u/.claude/projects/p/a.jsonl"), "prázdná cesta starou nepřepíše");
        assert_eq!(c.transcript_of("b"), None);
    }
}
