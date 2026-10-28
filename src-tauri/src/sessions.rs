//! Všechny Claude Code session na PC pro kartu Claude — čistá logika bez I/O.
//! Zdroje: `claude agents --json --all` (dokumentované, hlavní seznam), registr
//! `~/.claude/sessions/<pid>.json` (nedokumentovaný → jen obohacení: `entrypoint`, čerstvější `status`)
//! a hook stav `claude::Session` (čeká na schválení, hlídač commitů, subagenti). Klíč = `sessionId`.
//! I/O (spawn, watcher) je v `sessionlive.rs`, Win32 (přepnutí okna, resume) ve `winfocus.rs`.

use crate::i18n::tr;
use crate::claude::{Session, SessionState};
use crate::subagents::Subagents;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Strop spawnu `claude agents` (spawn trvá ~1 s CPU).
pub const SPAWN_GAP_MS: u64 = 5000;
/// Hook session, kterou seznam procesů po tolika ms od její poslední události nezná, už neběží.
const GONE_GRACE_MS: u64 = 10_000;

/// Položka z `claude agents --json`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Agent {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub kind: Option<String>,
    pub started_at: Option<u64>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub state: Option<String>,
    pub waiting_for: Option<String>,
    /// krátké id session na pozadí (`attach`/`logs`/`stop`…), jen u `kind: background`
    pub job_id: Option<String>,
}

/// Soubor registru `~/.claude/sessions/<pid>.json` (formát se může změnit — vše volitelné).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reg {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub kind: Option<String>,
    pub entrypoint: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub started_at: Option<u64>,
    /// `statusUpdatedAt` (jinak `updatedAt`): od kdy platí `status`
    pub status_at: Option<u64>,
}

fn st(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| v.get(*k).and_then(Value::as_str)).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn num(v: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|k| v.get(*k).and_then(|x| x.as_u64().or_else(|| x.as_str()?.parse().ok())))
}

/// `None` = výstup není JSON pole (chyba CLI, jiná verze) → volající nic netvrdí.
pub fn parse_agents(out: &str) -> Option<Vec<Agent>> {
    let t = out.trim();
    // případné řádky před JSON (varování CLI) přeskočit
    let t = t.get(t.find('[')?..=t.rfind(']')?)?;
    let arr: Vec<Value> = serde_json::from_str(t).ok()?;
    Some(
        arr.iter()
            .filter_map(|v| {
                Some(Agent {
                    session_id: st(v, &["sessionId", "session_id", "id"])?,
                    pid: num(v, &["pid"]).and_then(|p| u32::try_from(p).ok()).unwrap_or(0),
                    cwd: st(v, &["cwd"]).unwrap_or_default(),
                    kind: st(v, &["kind"]),
                    started_at: num(v, &["startedAt"]),
                    name: st(v, &["name", "title"]),
                    status: st(v, &["status"]),
                    state: st(v, &["state"]),
                    waiting_for: st(v, &["waitingFor"]),
                    job_id: st(v, &["id"]),
                })
            })
            .collect(),
    )
}

pub fn parse_registry(text: &str) -> Option<Reg> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    Some(Reg {
        session_id: st(&v, &["sessionId"])?,
        pid: num(&v, &["pid"]).and_then(|p| u32::try_from(p).ok())?,
        cwd: st(&v, &["cwd"]).unwrap_or_default(),
        kind: st(&v, &["kind"]),
        entrypoint: st(&v, &["entrypoint"]),
        name: st(&v, &["name"]),
        status: st(&v, &["status"]),
        started_at: num(&v, &["startedAt"]),
        status_at: num(&v, &["statusUpdatedAt", "updatedAt"]),
    })
}

/// Jen `<pid>.json` — vedle leží i `<pid>.<hash>.key` (tajné tokeny, nikdy nečíst).
pub fn is_registry_file(name: &str) -> bool {
    name.strip_suffix(".json").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Stav pro UI; pořadí = řazení v kartě.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Waiting,
    Working,
    Idle,
    Ended,
}

/// `status` z CLI/registru (busy/idle/waiting…), `state` z docs (working|blocked|done|failed|stopped).
pub fn live_status(status: Option<&str>, state: Option<&str>, waiting_for: Option<&str>) -> Status {
    let is = |v: Option<&str>, set: &[&str]| v.is_some_and(|v| set.iter().any(|s| v.eq_ignore_ascii_case(s)));
    // `done` = turn doběhl a session čeká na další prompt; s živým procesem (`status`) je to nečinná, ne konec
    let done_alive = is(state, &["done"]) && status.is_some();
    if (is(state, &["done", "failed", "stopped", "completed"]) && !done_alive) || is(status, &["done", "failed", "stopped", "completed", "exited"]) {
        Status::Ended
    } else if waiting_for.is_some() || is(state, &["blocked"]) || is(status, &["waiting", "blocked", "needs_input"]) {
        Status::Waiting
    } else if is(state, &["working"]) || is(status, &["busy", "working", "running"]) {
        Status::Working
    } else {
        Status::Idle
    }
}

/// Co udělá klik na session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAction {
    /// běžící session v desktopové appce Claude → přepnout do jejího okna (záložku vybrat neumíme)
    SwitchDesktop { pid: u32 },
    /// běžící CLI session → okno terminálu / editoru, ve kterém běží
    SwitchTerminal { pid: u32 },
    /// skončená session → `claude --resume` v novém terminálu
    Resume,
    Unavailable(&'static str),
}

/// `alive`: `Some(true)` = proces běží, `Some(false)` = skončila, `None` = nevíme.
pub fn decide_open(entrypoint: Option<&str>, kind: Option<&str>, alive: Option<bool>, pid: Option<u32>) -> OpenAction {
    let background = kind.is_some_and(|k| k.eq_ignore_ascii_case("background"));
    match (alive, pid) {
        // na pozadí: okno nemá a `--resume` by vedle běžící úlohy založil druhou → jen „Připojit“ (agentview)
        _ if background => OpenAction::Unavailable(tr("Session běží na pozadí bez okna — připoj se tlačítkem Připojit.", "The session runs in the background without a window — use Attach.")),
        (Some(true), Some(pid)) if pid > 0 && entrypoint == Some("claude-desktop") => OpenAction::SwitchDesktop { pid },
        (Some(true), Some(pid)) if pid > 0 => OpenAction::SwitchTerminal { pid },
        (Some(true), _) => OpenAction::Unavailable(tr("Nevím, ve kterém procesu session běží.", "I don't know which process the session runs in.")),
        (Some(false), _) => OpenAction::Resume,
        (None, _) => OpenAction::Unavailable(tr("Session zatím není v seznamu procesů — zkus to za chvíli.", "The session isn't in the process list yet — try again in a moment.")),
    }
}

pub fn open_hint(a: &OpenAction, name: &str) -> String {
    match a {
        OpenAction::SwitchDesktop { .. } => trf!("Přepne do Claude — session: {name}", "Switches to Claude — session: {name}"),
        OpenAction::SwitchTerminal { .. } => tr("Přepne do okna, ve kterém session běží", "Switches to the window the session runs in").into(),
        OpenAction::Resume => tr("Otevře session znovu v terminálu (claude --resume)", "Reopens the session in a terminal (claude --resume)").into(),
        OpenAction::Unavailable(why) => (*why).into(),
    }
}

/// Za kolik ms smí další spawn `claude agents` (strop 1× za `SPAWN_GAP_MS`).
pub fn spawn_wait(last_spawn_ms: Option<u64>, now_ms: u64) -> u64 {
    last_spawn_ms.map_or(0, |l| (l + SPAWN_GAP_MS).saturating_sub(now_ms))
}

/// Řádek karty Claude. `state`/`title`/`ended` drží kompatibilitu s hlavičkou a peekem v `App.tsx`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub name: String,
    /// titulek z hooků (první prompt), jen když se liší od `name`
    pub title: Option<String>,
    pub cwd: String,
    /// název projektu (peek) a jeho id (logo v kartě)
    pub project: Option<String>,
    pub project_id: Option<String>,
    pub status: Status,
    /// zastaralý tvar `status` pro hlavičku: working | waiting | idle
    pub state: SessionState,
    /// od kdy je ve stavu (ms), `None` = nevíme
    pub since: Option<u64>,
    pub updated_at: u64,
    pub entrypoint: Option<String>,
    pub pid: Option<u32>,
    pub alive: Option<bool>,
    /// session posílá Ledge hooky
    pub hooked: bool,
    pub uncommitted: Vec<String>,
    pub unpushed: u32,
    pub ended: bool,
    pub subagents: Subagents,
    pub can_open: bool,
    pub open_hint: String,
    #[serde(skip)]
    pub kind: Option<String>,
    /// `kind: background` (Agent View)
    pub background: bool,
    /// session na pozadí s krátkým id: stav úlohy a akce, které Ledge nabídne (`agentview`)
    pub job: Option<crate::agentview::Job>,
    /// na co čeká (`waitingFor`), jen když je stav „čeká“
    pub waiting_for: Option<crate::agentview::WaitFor>,
    /// živá aktivita z hooků (karta „Claude aktivita“, `claude/activity.rs`); `None` = hooky nic neposlaly
    pub activity: Option<crate::claude::activity::Activity>,
}

impl Row {
    pub fn open_action(&self) -> OpenAction {
        decide_open(self.entrypoint.as_deref(), self.kind.as_deref(), self.alive, self.pid)
    }
}

/// Živý seznam: `agents` = `None`, když ho zatím nemáme (spawn selhal / ještě neproběhl).
pub struct Live<'a> {
    pub agents: Option<&'a [Agent]>,
    pub fetched_at: u64,
    pub registry: &'a [Reg],
}

/// Systémové bloky, které Claude Code vkládá do textu promptu / přepisu (hook `UserPromptSubmit` je pošle
/// v `prompt`) — do náhledu v kartě nepatří.
const SYSTEM_TAGS: &[&str] = &[
    "system-reminder",
    "command-name",
    "command-message",
    "command-args",
    "command-contents",
    "local-command-stdout",
    "local-command-stderr",
    "local-command-caveat",
    "task-notification",
    "user-prompt-submit-hook",
    "bash-input",
    "bash-stdout",
    "bash-stderr",
];

/// Text pro náhled (podtitul / jméno session): bez bloků `SYSTEM_TAGS` i s obsahem (i v HTML escapované
/// podobě `&lt;tag&gt;`), neuzavřený blok = zbytek pryč, osamocená uzavírací značka pryč, mezery sloučené.
/// `None` = nic nezbylo.
pub fn clean_preview(s: &str) -> Option<String> {
    let mut s = s.to_string();
    for tag in SYSTEM_TAGS {
        if s.contains("&lt;") {
            s = s.replace(&format!("&lt;/{tag}&gt;"), &format!("</{tag}>")).replace(&format!("&lt;{tag}&gt;"), &format!("<{tag}>")).replace(&format!("&lt;{tag} "), &format!("<{tag} "));
        }
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(i) = rest.find('<') {
        let after = &rest[i + 1..];
        let (closing, name) = match after.strip_prefix('/') {
            Some(n) => (true, n),
            None => (false, after),
        };
        let tag = SYSTEM_TAGS.iter().find(|t| name.strip_prefix(**t).is_some_and(|r| r.starts_with(['>', ' ', '\n', '\r', '\t', '/'])));
        let Some(tag) = tag else {
            out.push_str(&rest[..=i]);
            rest = after;
            continue;
        };
        out.push_str(&rest[..i]);
        out.push(' ');
        rest = if closing {
            after.find('>').map_or("", |j| &after[j + 1..])
        } else {
            let close = format!("</{tag}>");
            after.find(&close).map_or("", |j| &after[j + close.len()..])
        };
    }
    out.push_str(rest);
    let t = out.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty()).then_some(t)
}

fn last_segment(cwd: &str) -> String {
    cwd.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or("session").to_string()
}

/// `dirs` = (id projektu, složka), `name_of` = id → název.
pub fn merge(live: &Live, hooked: Vec<Session>, dirs: &[(String, String)], name_of: &dyn Fn(&str) -> Option<String>, now: u64) -> Vec<Row> {
    let mut hooks: HashMap<String, Session> = hooked.into_iter().map(|s| (s.id.clone(), s)).collect();
    let reg: HashMap<&str, &Reg> = live.registry.iter().map(|r| (r.session_id.as_str(), r)).collect();
    let mut rows = Vec::new();
    let mut push = |id: &str, cwd: String, base: Base, h: Option<Session>| {
        let r = reg.get(id).copied();
        let waiting = h.as_ref().is_some_and(|h| h.state == SessionState::Waiting);
        let status = if waiting && base.status != Status::Ended { Status::Waiting } else { base.status };
        let unfinished = h.as_ref().is_some_and(Session::unfinished);
        // skončená bez rozdělané práce nikoho nezajímá — kromě úlohy na pozadí (je v seznamu, dokud ji nikdo neodebere)
        if status == Status::Ended && !unfinished && base.job.is_none() {
            return;
        }
        // náhledy textu bez systémových bloků Claude Code (`<system-reminder>`…) — prázdné = žádný text
        let title = h.as_ref().and_then(|h| h.title.as_deref()).and_then(clean_preview);
        let name = base.name.as_deref().and_then(clean_preview).or_else(|| r.and_then(|r| r.name.as_deref()).and_then(clean_preview)).or_else(|| title.clone()).unwrap_or_else(|| last_segment(&cwd));
        let project_id = crate::notes::project_for_cwd(dirs, &cwd).map(str::to_string).or_else(|| {
            // session ve worktree (`atlas-v2`) patří projektu hlavního checkoutu
            let main = crate::resolve::main_checkout(std::path::Path::new(&cwd))?;
            crate::notes::project_for_cwd(dirs, &main.to_string_lossy()).map(str::to_string)
        });
        let updated_at = [h.as_ref().map(|h| h.updated_at), r.and_then(|r| r.status_at), base.started_at].into_iter().flatten().max().unwrap_or(now);
        let alive = if status == Status::Ended { Some(false) } else { base.alive };
        let pid = base.pid.or(r.map(|r| r.pid));
        let entrypoint = r.and_then(|r| r.entrypoint.clone());
        let kind = base.kind.or_else(|| r.and_then(|r| r.kind.clone()));
        let background = kind.as_deref().is_some_and(|k| k.eq_ignore_ascii_case("background"));
        let waiting_for = base.waiting_for.filter(|_| status == Status::Waiting);
        let action = decide_open(entrypoint.as_deref(), kind.as_deref(), alive, pid);
        let (uncommitted, unpushed, subagents, activity) = h.map(|h| (h.uncommitted, h.unpushed, h.subagents, h.activity)).unwrap_or_default();
        rows.push(Row {
            id: id.to_string(),
            title: title.filter(|t| *t != name),
            project: project_id.as_deref().and_then(name_of),
            project_id,
            state: match status {
                Status::Waiting => SessionState::Waiting,
                Status::Working => SessionState::Working,
                _ => SessionState::Idle,
            },
            since: base.since.or(r.and_then(|r| r.status_at)),
            hooked: base.hooked,
            ended: status == Status::Ended,
            can_open: !matches!(action, OpenAction::Unavailable(_)),
            open_hint: open_hint(&action, &name),
            name,
            cwd,
            status,
            updated_at,
            entrypoint,
            pid,
            alive,
            uncommitted,
            unpushed,
            subagents,
            kind,
            background,
            job: base.job,
            waiting_for,
            activity,
        });
    };
    for a in live.agents.unwrap_or_default() {
        let h = hooks.remove(&a.session_id);
        let r = reg.get(a.session_id.as_str()).copied();
        let status = agent_status(a, r, live.fetched_at);
        let background = a.kind.as_deref().is_some_and(|k| k.eq_ignore_ascii_case("background"));
        // pid/status jen dokud proces žije (docs); úloha na pozadí je v seznamu i bez procesu
        let proc_alive = a.pid > 0 || a.status.is_some();
        let base = Base {
            status,
            name: a.name.clone(),
            started_at: a.started_at,
            since: r.and_then(|r| r.status_at).or(a.started_at),
            alive: Some(!background || proc_alive),
            pid: (a.pid > 0).then_some(a.pid),
            kind: a.kind.clone(),
            hooked: h.is_some(),
            job: if background { crate::agentview::job(a.job_id.as_deref(), a.state.as_deref(), proc_alive) } else { None },
            waiting_for: crate::agentview::wait_for(a.waiting_for.as_deref()),
        };
        let cwd = if a.cwd.is_empty() { h.as_ref().map(|h| h.cwd.clone()).unwrap_or_default() } else { a.cwd.clone() };
        push(&a.session_id, cwd, base, h);
    }
    // session známé jen z hooků (Ledge je vidí dřív, než je zná seznam procesů, nebo už skončily)
    let mut rest: Vec<Session> = hooks.into_values().collect();
    rest.sort_by(|a, b| a.id.cmp(&b.id));
    for h in rest {
        let gone = h.ended || (live.agents.is_some() && live.fetched_at > h.updated_at + GONE_GRACE_MS);
        let r = reg.get(h.id.as_str()).copied();
        let status = if gone {
            Status::Ended
        } else {
            match h.state {
                SessionState::Waiting => Status::Waiting,
                SessionState::Working => Status::Working,
                SessionState::Idle => Status::Idle,
            }
        };
        let base = Base {
            status,
            name: None,
            started_at: None,
            since: Some(h.updated_at),
            // soubor registru existuje = proces běží (pokud nespadl bez úklidu)
            alive: if gone { Some(false) } else { r.map(|_| true) },
            pid: None,
            kind: None,
            hooked: true,
            job: None,
            waiting_for: None,
        };
        let (id, cwd) = (h.id.clone(), h.cwd.clone());
        push(&id, cwd, base, Some(h));
    }
    rows.sort_by(|a, b| a.status.cmp(&b.status).then(b.updated_at.cmp(&a.updated_at)));
    rows
}

/// Co o session ví živý seznam (agent nebo hook) — vstup pro `merge::push`.
struct Base {
    status: Status,
    name: Option<String>,
    started_at: Option<u64>,
    since: Option<u64>,
    alive: Option<bool>,
    pid: Option<u32>,
    kind: Option<String>,
    hooked: bool,
    job: Option<crate::agentview::Job>,
    waiting_for: Option<crate::agentview::WaitFor>,
}

/// Stav položky živého seznamu; registr je čerstvější, když se změnil po posledním spawnu.
pub fn agent_status(a: &Agent, r: Option<&Reg>, fetched_at: u64) -> Status {
    match r.filter(|r| r.status.is_some() && r.status_at.is_some_and(|t| t > fetched_at)) {
        Some(r) => live_status(r.status.as_deref(), None, None),
        None => live_status(a.status.as_deref(), a.state.as_deref(), a.waiting_for.as_deref()),
    }
}

/// Session, které podle živého seznamu (Agent View, `status: waiting` / `waitingFor` / `blocked`) čekají
/// na člověka — i bez hooků Ledge. Pro čítač proužku (`attention::waiting_count_all`), který je sloučí
/// se session hlášenými hookem (stejné `sessionId` = jedna).
pub fn waiting_ids<'a>(live: &Live<'a>) -> Vec<&'a str> {
    let reg: HashMap<&str, &Reg> = live.registry.iter().map(|r| (r.session_id.as_str(), r)).collect();
    live.agents
        .unwrap_or_default()
        .iter()
        .filter(|a| agent_status(a, reg.get(a.session_id.as_str()).copied(), live.fetched_at) == Status::Waiting)
        .map(|a| a.session_id.as_str())
        .collect()
}

/// Bezpečné id session pro příkazovou řádku (UUID a podobné).
pub fn valid_session_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Jak spustit `claude --resume <id>` v novém okně terminálu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// `wt` s argumenty (Windows Terminal)
    Wt(Vec<String>),
    /// `cmd` se surovou příkazovou řádkou (`start` otevře nové okno konzole)
    Cmd(String),
}

/// `None` = id nebo složka nejsou bezpečné pro příkazovou řádku.
pub fn resume_command(wt: bool, cwd: &str, id: &str) -> Option<Launch> {
    if !valid_session_id(id) || cwd.trim().is_empty() || cwd.contains(['"', '\r', '\n', '%', '^', '&', '|', '<', '>']) {
        return None;
    }
    // `;` odděluje příkazy wt → taková složka jen přes cmd
    if wt && !cwd.contains(';') {
        let args = ["-d", cwd, "cmd", "/c", "claude", "--resume", id];
        return Some(Launch::Wt(args.iter().map(|s| s.to_string()).collect()));
    }
    Some(Launch::Cmd(format!("/d /c start \"\" /d \"{cwd}\" cmd /c claude --resume {id}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENTS: &str = r#"[
      {"pid":12148,"cwd":"C:\\Users\\x\\Desktop\\atlas-v2","kind":"interactive","startedAt":1000,"sessionId":"s-desk","name":"Review commitů","status":"idle"},
      {"pid":360,"cwd":"C:\\Users\\x\\Desktop\\ledge","kind":"interactive","startedAt":2000,"sessionId":"s-cli","name":"","status":"busy"},
      {"id":"s-bg","state":"blocked","waitingFor":"permission prompt","kind":"background","cwd":"C:\\jinde"},
      {"pid":1,"status":"busy"}
    ]"#;

    #[test]
    fn parse_agents_defensive() {
        let a = parse_agents(&format!("varování CLI\n{AGENTS}\n")).unwrap();
        assert_eq!(a.len(), 3, "položka bez id se zahodí");
        assert_eq!(a[0], Agent { pid: 12148, session_id: "s-desk".into(), cwd: r"C:\Users\x\Desktop\atlas-v2".into(), kind: Some("interactive".into()), started_at: Some(1000), name: Some("Review commitů".into()), status: Some("idle".into()), state: None, waiting_for: None, job_id: None });
        assert_eq!(a[1].name, None, "prázdné jméno = žádné");
        assert_eq!((a[2].session_id.as_str(), a[2].pid, a[2].waiting_for.as_deref()), ("s-bg", 0, Some("permission prompt")));
        for bad in ["", "nejsem json", "{\"a\":1}", "[1,2"] {
            assert_eq!(parse_agents(bad), None, "{bad}");
        }
        assert_eq!(parse_agents("[]"), Some(vec![]));
    }

    #[test]
    fn parse_registry_and_names() {
        let r = parse_registry(r#"{"pid":12148,"sessionId":"s1","cwd":"C:\\p","kind":"interactive","entrypoint":"claude-desktop","hostSessionId":"local_x","name":"N","status":"busy","updatedAt":5,"statusUpdatedAt":4,"novePole":[1]}"#).unwrap();
        assert_eq!((r.entrypoint.as_deref(), r.status_at, r.pid), (Some("claude-desktop"), Some(4), 12148));
        assert_eq!(parse_registry(r#"{"pid":1,"sessionId":"s","updatedAt":9}"#).unwrap().status_at, Some(9));
        assert_eq!(parse_registry(r#"{"sessionId":"s"}"#), None, "bez pid");
        assert_eq!(parse_registry("{"), None);
        let names = [("12148.json", true), ("1.json", true), ("12148.55a4.key", false), (".json", false), ("abc.json", false), ("12148.json.tmp", false)];
        for (n, want) in names {
            assert_eq!(is_registry_file(n), want, "{n}");
        }
    }

    #[test]
    fn live_status_table() {
        use Status::*;
        type Case<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>, Status);
        let cases: &[Case] = &[
            (Some("busy"), None, None, Working),
            (Some("idle"), None, None, Idle),
            (Some("waiting"), None, None, Waiting),
            (Some("idle"), None, Some("input needed"), Waiting),
            (None, Some("working"), None, Working),
            (None, Some("blocked"), None, Waiting),
            (None, Some("done"), None, Ended),
            // done s živým procesem = čeká na další prompt (nečinná), ne konec
            (Some("idle"), Some("done"), None, Idle),
            (Some("busy"), Some("done"), None, Working),
            (Some("busy"), Some("failed"), None, Ended),
            (Some("STOPPED"), None, None, Ended),
            (Some("neznámý"), None, None, Idle),
            (None, None, None, Idle),
        ];
        for (status, state, wf, want) in cases {
            assert_eq!(live_status(*status, *state, *wf), *want, "{status:?} {state:?} {wf:?}");
        }
    }

    #[test]
    fn decide_open_table() {
        use OpenAction::*;
        let desk = Some("claude-desktop");
        type Case<'a> = (Option<&'a str>, Option<&'a str>, Option<bool>, Option<u32>, OpenAction);
        let cases: &[Case] = &[
            (desk, Some("interactive"), Some(true), Some(5), SwitchDesktop { pid: 5 }),
            (Some("cli"), Some("interactive"), Some(true), Some(5), SwitchTerminal { pid: 5 }),
            (None, None, Some(true), Some(5), SwitchTerminal { pid: 5 }),
            (desk, Some("background"), Some(true), Some(5), Unavailable("Session běží na pozadí bez okna — připoj se tlačítkem Připojit.")),
            // na pozadí ani skončená neresumuje (vedle úlohy by vznikla druhá session)
            (None, Some("background"), Some(false), None, Unavailable("Session běží na pozadí bez okna — připoj se tlačítkem Připojit.")),
            (desk, None, Some(true), None, Unavailable("Nevím, ve kterém procesu session běží.")),
            (desk, None, Some(true), Some(0), Unavailable("Nevím, ve kterém procesu session běží.")),
            // skončená → resume, i desktopová (deep link do appky není dokumentovaný)
            (desk, None, Some(false), Some(5), Resume),
            (None, None, Some(false), None, Resume),
            // běží-li, nikdy resume (prokládání zpráv)
            (None, None, None, None, Unavailable("Session zatím není v seznamu procesů — zkus to za chvíli.")),
        ];
        for (e, k, alive, pid, want) in cases {
            assert_eq!(decide_open(*e, *k, *alive, *pid), *want, "{e:?} {k:?} {alive:?} {pid:?}");
        }
        assert_eq!(open_hint(&SwitchDesktop { pid: 1 }, "Review"), "Přepne do Claude — session: Review");
        crate::i18n::with_lang(crate::uiconf::Lang::En, || {
            assert_eq!(open_hint(&SwitchDesktop { pid: 1 }, "Review"), "Switches to Claude — session: Review");
            assert_eq!(open_hint(&decide_open(None, Some("background"), Some(true), Some(7)), "x"), "The session runs in the background without a window — use Attach.");
        });
    }

    #[test]
    fn spawn_throttle() {
        assert_eq!(spawn_wait(None, 100), 0);
        assert_eq!(spawn_wait(Some(1000), 1000), SPAWN_GAP_MS);
        assert_eq!(spawn_wait(Some(1000), 4000), 2000);
        assert_eq!(spawn_wait(Some(1000), 6000), 0);
        assert_eq!(spawn_wait(Some(1000), 99_000), 0);
    }

    #[test]
    fn resume_command_table() {
        let id = "6679cf49-ee94-4091-9d7b-9841d38e4802";
        assert_eq!(
            resume_command(true, r"C:\Users\x\proj", id),
            Some(Launch::Wt(["-d", r"C:\Users\x\proj", "cmd", "/c", "claude", "--resume", id].iter().map(|s| s.to_string()).collect()))
        );
        assert_eq!(resume_command(false, r"C:\a b", id), Some(Launch::Cmd(format!("/d /c start \"\" /d \"C:\\a b\" cmd /c claude --resume {id}"))));
        assert!(matches!(resume_command(true, r"C:\a;b", id), Some(Launch::Cmd(_))), "středník → cmd");
        for (cwd, sid) in [(r"C:\p", "x & calc"), (r"C:\p", "kratke"), (r#"C:\p" & calc"#, id), (r"C:\%PATH%", id), ("  ", id), (r"C:\a|b", id)] {
            assert_eq!(resume_command(true, cwd, sid), None, "{cwd} {sid}");
        }
    }

    #[test]
    fn clean_preview_table() {
        let cases: &[(&str, Option<&str>)] = &[
            ("oprav testy", Some("oprav testy")),
            ("<system-reminder>\nLinked sessions now (s…)\n</system-reminder>\noprav testy", Some("oprav testy")),
            ("oprav <system-reminder>x</system-reminder> testy <system-reminder>y</system-reminder>", Some("oprav testy")),
            ("&lt;system-reminder&gt; Linked sessions &lt;/system-reminder&gt; ahoj", Some("ahoj")),
            ("<command-name>/clear</command-name>\n<command-message>clear</command-message>\n<command-args></command-args>", None),
            ("<local-command-stdout>hotovo</local-command-stdout>", None),
            ("<task-notification>\n<task-id>a1</task-id>\n</task-notification>", None),
            ("<system-reminder>\r\nx</system-reminder>\r\nok", Some("ok")),
            ("<system-reminder data=\"1\">x</system-reminder>zbytek", Some("zbytek")),
            // neuzavřený blok (oříznutý text) = zbytek je systémový
            ("úkol <system-reminder> Linked sessions now (s…", Some("úkol")),
            ("</system-reminder> po", Some("po")),
            ("<system-reminder>jen systém</system-reminder>", None),
            // jiné značky a nerovnosti z textu uživatele zůstávají
            ("oprav <div> a a < b &lt;x&gt;", Some("oprav <div> a a < b &lt;x&gt;")),
            ("<system-reminders>není náš</system-reminders>", Some("<system-reminders>není náš</system-reminders>")),
            ("   ", None),
        ];
        for (inp, want) in cases {
            assert_eq!(clean_preview(inp).as_deref(), *want, "{inp:?}");
        }
        // řádek karty: titulek z hooku i jméno ze seznamu jdou přes filtr
        let agents = parse_agents(r#"[{"pid":3,"sessionId":"s1","cwd":"C:/x/WorkSpace","status":"busy","name":"<system-reminder>a</system-reminder>"}]"#).unwrap();
        let mut h = hook("s1", "", SessionState::Working, 1);
        h.title = Some("<system-reminder> Linked sessions now (s…".into());
        let rows = merge(&Live { agents: Some(&agents), fetched_at: 0, registry: &[] }, vec![h], &[], &|_| None, 2);
        assert_eq!((rows[0].name.as_str(), rows[0].title.as_deref()), ("WorkSpace", None));
    }

    #[test]
    fn agent_view_rows() {
        use crate::agentview::{JobAction::*, JobState, WaitFor};
        let json = r#"[
          {"id":"7c5dcf5d","sessionId":"u-failed","kind":"background","state":"failed","cwd":"C:/p","startedAt":1},
          {"id":"aa11bb22","sessionId":"u-done","kind":"background","state":"done","pid":5,"status":"idle","cwd":"C:/p","startedAt":2},
          {"id":"bad id!","sessionId":"u-badid","kind":"background","state":"working","pid":6,"status":"busy","cwd":"C:/p","startedAt":3},
          {"pid":7,"sessionId":"u-input","kind":"interactive","status":"waiting","waitingFor":"input needed","cwd":"C:/q","startedAt":4},
          {"pid":8,"sessionId":"u-new","kind":"interactive","status":"waiting","waitingFor":"něco nového","cwd":"C:/q","startedAt":5},
          {"pid":9,"sessionId":"u-stale","kind":"interactive","status":"waiting","waitingFor":"permission prompt","cwd":"C:/q","startedAt":6},
          {"id":"cc33dd44","sessionId":"u-blocked","kind":"background","state":"blocked","waitingFor":"sandbox request","cwd":"C:/p","startedAt":7}
        ]"#;
        let agents = parse_agents(json).unwrap();
        assert_eq!((agents[0].job_id.as_deref(), agents[3].job_id.as_deref()), (Some("7c5dcf5d"), None));
        // registr po posledním spawnu: u-stale už pracuje (odpověď padla v terminálu)
        let registry = vec![parse_registry(r#"{"pid":9,"sessionId":"u-stale","status":"busy","statusUpdatedAt":200}"#).unwrap()];
        let live = Live { agents: Some(&agents), fetched_at: 100, registry: &registry };
        let rows = merge(&live, vec![], &[], &|_| None, 300);
        let by = |id: &str| rows.iter().find(|r| r.id == id).unwrap_or_else(|| panic!("{id}"));
        // skončená úloha na pozadí zůstává (jde odebrat), bez otevření
        let f = by("u-failed");
        assert_eq!((f.status, f.background, f.can_open), (Status::Ended, true, false));
        assert_eq!(f.job.as_ref().map(|j| (j.id.as_str(), j.state, j.actions.clone())), Some(("7c5dcf5d", Some(JobState::Failed), vec![Attach, Logs, Remove])));
        // done s živým procesem = nečinná, jde zastavit, restartovat i odebrat
        let d = by("u-done");
        assert_eq!((d.status, d.job.as_ref().unwrap().actions.clone()), (Status::Idle, vec![Attach, Logs, Stop, Respawn, Remove]));
        // neplatné id → žádné akce, řádek zůstává
        assert_eq!((by("u-badid").job.is_none(), by("u-badid").background, by("u-badid").status), (true, true, Status::Working));
        // interaktivní čekající: přeložitelný důvod; neznámá hodnota = čeká bez důvodu
        assert_eq!((by("u-input").status, by("u-input").waiting_for, by("u-input").job.is_none()), (Status::Waiting, Some(WaitFor::Input), true));
        assert_eq!((by("u-new").status, by("u-new").waiting_for), (Status::Waiting, None));
        // čerstvější registr přebije starý waitingFor
        assert_eq!((by("u-stale").status, by("u-stale").waiting_for), (Status::Working, None));
        let b = by("u-blocked");
        assert_eq!((b.status, b.waiting_for, b.job.as_ref().unwrap().actions.clone()), (Status::Waiting, Some(WaitFor::Sandbox), vec![Attach, Logs, Stop]));
        let json_row = serde_json::to_value(b).unwrap();
        assert_eq!((json_row["background"].as_bool(), json_row["waitingFor"].as_str(), json_row["job"]["state"].as_str()), (Some(true), Some("sandbox"), Some("blocked")));
        assert_eq!(json_row["job"]["actions"], serde_json::json!(["attach", "logs", "stop"]));

        // čítač proužku: čekající podle Agent View, ne ty, které registr mezitím posunul
        let mut w = waiting_ids(&live);
        w.sort_unstable();
        assert_eq!(w, ["u-blocked", "u-input", "u-new"]);
        assert!(waiting_ids(&Live { agents: None, fetched_at: 0, registry: &[] }).is_empty());
    }

    fn hook(id: &str, cwd: &str, state: SessionState, updated_at: u64) -> Session {
        Session { id: id.into(), cwd: cwd.into(), state, title: None, updated_at, uncommitted: vec![], unpushed: 0, ended: false, subagents: Subagents::default(), transcript: None, activity: None }
    }

    #[test]
    fn merge_sources() {
        let agents = parse_agents(AGENTS).unwrap();
        let registry = vec![
            parse_registry(r#"{"pid":12148,"sessionId":"s-desk","entrypoint":"claude-desktop","status":"busy","statusUpdatedAt":21000}"#).unwrap(),
            parse_registry(r#"{"pid":777,"sessionId":"h-new","entrypoint":"cli"}"#).unwrap(),
        ];
        let dirs = vec![("atlas".to_string(), r"C:\Users\x\Desktop\atlas-v2".to_string())];
        let name_of = |id: &str| (id == "atlas").then(|| "Atlas".to_string());
        // hooky: s-cli čeká na schválení; h-new zná jen hook (+ registr); h-gone neběží a nemá práci;
        // h-dirty skončila s nezacommitovanou prací
        let mut cli = hook("s-cli", "", SessionState::Waiting, 3000);
        cli.title = Some("oprav testy".into());
        let mut dirty = hook("h-dirty", r"C:\Users\x\Desktop\atlas-v2\sub", SessionState::Idle, 100);
        dirty.uncommitted = vec!["a.rs".into()];
        dirty.ended = true;
        let hooked = vec![cli, hook("h-new", r"C:\nove", SessionState::Working, 15_000), hook("h-gone", r"C:\x", SessionState::Idle, 100), dirty];
        let live = Live { agents: Some(&agents), fetched_at: 20_000, registry: &registry };
        let rows = merge(&live, hooked, &dirs, &name_of, 30_000);
        let got: Vec<(&str, Status)> = rows.iter().map(|r| (r.id.as_str(), r.status)).collect();
        assert_eq!(got, [("s-bg", Status::Waiting), ("s-cli", Status::Waiting), ("s-desk", Status::Working), ("h-new", Status::Working), ("h-dirty", Status::Ended)]);
        let by = |id: &str| rows.iter().find(|r| r.id == id).unwrap();
        // desktop: jméno z agents, stav z čerstvějšího registru, projekt z cwd, klik = přepnout do appky
        let d = by("s-desk");
        assert_eq!((d.name.as_str(), d.project.as_deref(), d.project_id.as_deref()), ("Review commitů", Some("Atlas"), Some("atlas")));
        assert_eq!((d.entrypoint.as_deref(), d.since, d.can_open, d.hooked), (Some("claude-desktop"), Some(21_000), true, false));
        assert_eq!(d.open_action(), OpenAction::SwitchDesktop { pid: 12148 });
        // CLI: bez jména → titulek z hooků; čekání na schválení z hooku přebíjí „busy“
        let c = by("s-cli");
        assert_eq!((c.name.as_str(), c.title.as_deref(), c.state, c.hooked), ("oprav testy", None, SessionState::Waiting, true));
        assert_eq!(c.open_action(), OpenAction::SwitchTerminal { pid: 360 });
        // na pozadí: nejde otevřít, poctivě proč
        assert!(!by("s-bg").can_open && by("s-bg").open_hint.contains("pozadí"));
        // jen z hooku + registr: pid a entrypoint z registru, jméno = konec cwd
        let n = by("h-new");
        assert_eq!((n.name.as_str(), n.pid, n.alive, n.entrypoint.as_deref()), ("nove", Some(777), Some(true), Some("cli")));
        // skončená s prací: resume, projekt i z podsložky
        let e = by("h-dirty");
        assert_eq!((e.ended, e.alive, e.project.as_deref(), e.open_action()), (true, Some(false), Some("Atlas"), OpenAction::Resume));
        assert_eq!(e.uncommitted, ["a.rs"]);

        // bez živého seznamu: hook session zůstává s neznámým během, nic se nevymýšlí
        let rows = merge(&Live { agents: None, fetched_at: 0, registry: &[] }, vec![hook("h", r"C:\x", SessionState::Idle, 1)], &dirs, &name_of, 2);
        assert_eq!((rows[0].status, rows[0].alive, rows[0].can_open), (Status::Idle, None, false));
    }
}
