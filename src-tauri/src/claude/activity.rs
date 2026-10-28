//! Živá aktivita session (kolo 23): fáze, poslední soubor a poslední zpráva Clauda — jen z reálných zdrojů.
//!
//! - Fáze = ODHAD podle posledního nástroje z hooku `PostToolUse` (Read/Grep/Glob → čtení, Edit/Write → úpravy,
//!   Bash/PowerShell se spuštěním testů / buildu → testy, jiný nástroj → pracuje), `Stop` → hotovo.
//!   Chyba se zobrazí jen při hooku `PostToolUseFailure`, který Ledge instaluje (nebo když
//!   `PostToolUse` výjimečně nese `exit_code` / `is_error`); jinak se nic nevymýšlí. UI fázi označí
//!   „odhad podle posledního nástroje“.
//! - Soubor = `tool_input.file_path` / `notebook_path` posledního nástroje, který ho má.
//! - Poslední zpráva = `last_assistant_message` z hooku `Stop`, jinak konec přepisu (`transcript_path`,
//!   nejvýš `TAIL_MAX` od konce, jen při `Stop` a jen když je karta „Claude aktivita“ na některé záložce).
//!
//! Žádné vlákno ani časovač: vše se počítá jen při příchodu hooku. Mapa: `docs/codemap/sessions.md`.

use super::{Claude, Session, SessionState};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::Ordering;

/// Kolik bajtů konce přepisu se nejvýš čte (hledá se poslední text Clauda).
pub const TAIL_MAX: u64 = 64 * 1024;
/// Délka zprávy v kartě (znaky), delší se zkrátí s „…“.
pub const MSG_MAX: usize = 220;

/// Fáze tahu (odhad z posledního nástroje).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Reading,
    Editing,
    Testing,
    /// jiný nástroj (web, subagent, obyčejný příkaz…) nebo začátek tahu
    Working,
    Done,
    Error,
}

/// Co session naposledy dělala (`Session.activity`, posílá se i v `sessions::Row`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub phase: Phase,
    /// poslední nástroj z hooku ("" = začátek tahu / konec)
    pub tool: String,
    /// poslední soubor z `tool_input` (celá cesta, jak ji poslal Claude)
    pub file: Option<String>,
    /// poslední text Clauda (zkrácený, bez systémových bloků)
    pub message: Option<String>,
    /// kdy (ms) se fáze naposledy změnila (další hook téže fáze ho nemění)
    pub at: u64,
}

/// Spouští příkaz testy / build / kontrolu? Jen první slovo úseku (`&&`, `;`, `|`) je spouštěč a některé další
/// slovo je úloha — „cat tests/a.txt“ testy nejsou.
pub fn is_check_command(cmd: &str) -> bool {
    const RUNNERS: &[&str] = &[
        "cargo", "npm", "pnpm", "yarn", "bun", "npx", "node", "deno", "python", "python3", "py", "go", "dotnet", "make", "gradle", "gradlew", "mvn", "uv",
    ];
    const SELF: &[&str] = &["pytest", "vitest", "jest", "tsc", "ctest", "mocha", "playwright"];
    const TASKS: &[&str] = &["test", "tests", "build", "check", "clippy", "lint", "nextest", "pytest", "vitest", "jest", "tsc", "typecheck", "compile", "verify"];
    let lower = cmd.to_lowercase();
    lower.split(['&', ';', '|', '\n']).any(|seg| {
        let mut words = seg.split_whitespace().filter(|w| !w.contains('=')).map(|w| w.trim_matches(['"', '\'', '(', ')']));
        let Some(first) = words.next() else { return false };
        let first = first.rsplit(['/', '\\']).next().unwrap_or(first).trim_end_matches(".exe").trim_end_matches(".cmd");
        if SELF.contains(&first) {
            return true;
        }
        RUNNERS.contains(&first)
            && words.any(|w| {
                // `test:unit`, `scripts/test.mjs`, `-m pytest`
                let base = w.rsplit(['/', '\\']).next().unwrap_or(w);
                let stem = base.split([':', '.']).next().unwrap_or(base);
                TASKS.contains(&stem)
            })
    })
}

/// Nenulový výsledek příkazu z `tool_response` (jen pole, která hook opravdu nese; chybí-li = `false`).
pub fn failed(response: &Value) -> bool {
    let code = ["exit_code", "exitCode", "returnCode"].iter().find_map(|k| response.get(*k).and_then(Value::as_i64));
    code.is_some_and(|c| c != 0) || response.get("is_error").and_then(Value::as_bool).unwrap_or(false)
}

/// Fáze z jednoho nástroje (`PostToolUse`).
pub fn phase_of(tool: &str, input: &Value, response: &Value) -> Phase {
    if failed(response) {
        return Phase::Error;
    }
    match tool {
        "Read" | "Grep" | "Glob" | "LS" | "NotebookRead" => Phase::Reading,
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => Phase::Editing,
        "Bash" | "PowerShell" if input.get("command").and_then(Value::as_str).is_some_and(is_check_command) => Phase::Testing,
        _ => Phase::Working,
    }
}

/// Soubor z `tool_input` (`file_path`, u notebooků `notebook_path`); prázdný = `None`.
pub fn file_of(input: &Value) -> Option<String> {
    ["file_path", "notebook_path"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// Jméno souboru z cesty („src/main.rs“ → „main.rs“).
pub fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or(path)
}

/// Zpráva do karty: bez systémových bloků, jeden řádek, nejvýš `MSG_MAX` znaků. Prázdná = `None`.
pub fn clean_message(text: &str) -> Option<String> {
    let t = crate::sessions::clean_preview(text)?;
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        return None;
    }
    Some(if t.chars().count() > MSG_MAX { t.chars().take(MSG_MAX).collect::<String>() + "…" } else { t })
}

/// Text jedné řádky přepisu, je-li to odpověď Clauda (hlavní vlákno, ne subagent) s textem.
fn assistant_text(line: &[u8]) -> Option<String> {
    if memchr::memmem::find(line, b"\"assistant\"").is_none() || memchr::memmem::find(line, b"\"isSidechain\":true").is_some() {
        return None;
    }
    let v: Value = serde_json::from_slice(line).ok()?;
    if v.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    let content = v.get("message")?.get("content")?;
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => return None,
    };
    clean_message(&text)
}

/// Poslední text Clauda v konci přepisu. `from_start` = výřez začíná začátkem souboru (jinak je první řádek
/// useknutý a přeskočí se). Bez textové odpovědi = `None`.
pub fn last_message(tail: &[u8], from_start: bool) -> Option<String> {
    let lines: Vec<&[u8]> = tail.split(|b| *b == b'\n').collect();
    for (i, line) in lines.iter().enumerate().rev() {
        if i == 0 && !from_start {
            break;
        }
        if let Some(t) = assistant_text(line) {
            return Some(t);
        }
    }
    None
}

/// Přečte poslední zprávu z přepisu (jen `*.jsonl` pod `projects_root`, nejvýš `TAIL_MAX` od konce). Čte disk.
pub fn read_last_message(path: &Path, projects_root: Option<&Path>) -> Option<String> {
    if !crate::ctxsize::allowed(path, projects_root) {
        return None;
    }
    let (buf, whole) = crate::ctxsize::read_tail(path, TAIL_MAX).ok()?;
    last_message(&buf, whole)
}

/// Aktivita po jedné hook události (`None` = událost aktivitu nemění). Čistá funkce.
pub fn next(prev: Option<&Activity>, ev: &str, body: &Value, now: u64) -> Option<Activity> {
    let mut a = next_raw(prev, ev, body, now)?;
    if let Some(p) = prev.filter(|p| p.phase == a.phase) {
        a.at = p.at;
    }
    Some(a)
}

fn next_raw(prev: Option<&Activity>, ev: &str, body: &Value, now: u64) -> Option<Activity> {
    let keep_msg = || prev.and_then(|a| a.message.clone());
    let tool = || body.get("tool_name").and_then(Value::as_str).unwrap_or("").to_string();
    let null = Value::Null;
    match ev {
        "UserPromptSubmit" => Some(Activity { phase: Phase::Working, tool: String::new(), file: None, message: keep_msg(), at: now }),
        "PostToolUse" | "PostToolUseFailure" => {
            let input = body.get("tool_input").unwrap_or(&null);
            let response = body.get("tool_response").unwrap_or(&null);
            let phase = if ev == "PostToolUseFailure" { Phase::Error } else { phase_of(&tool(), input, response) };
            let file = file_of(input).or_else(|| prev.and_then(|a| a.file.clone()));
            Some(Activity { phase, tool: tool(), file, message: keep_msg(), at: now })
        }
        "Stop" => {
            let msg = body.get("last_assistant_message").and_then(Value::as_str).and_then(clean_message).or_else(keep_msg);
            Some(Activity { phase: Phase::Done, tool: String::new(), file: prev.and_then(|a| a.file.clone()), message: msg, at: now })
        }
        _ => None,
    }
}

/// Poslal hook `Stop` text Clauda sám (`last_assistant_message`)? Pak se přepis nečte.
pub fn hook_has_message(body: &Value) -> bool {
    body.get("last_assistant_message").and_then(Value::as_str).and_then(clean_message).is_some()
}

/// Session, jejíž aktivitu ukáže zavřený Ledge: nejčerstvější změna fáze mezi neukončenými sessions;
/// „hotovo“ jen do `done_ms` od konce tahu, rozpracovaná fáze jen u session, která pracuje.
pub fn lead<'a>(sessions: impl IntoIterator<Item = &'a Session>, now: u64, done_ms: u64) -> Option<&'a Session> {
    sessions
        .into_iter()
        .filter(|s| !s.ended)
        .filter(|s| match &s.activity {
            Some(a) if a.phase == Phase::Done => now.saturating_sub(a.at) < done_ms,
            Some(_) => s.state == SessionState::Working,
            None => false,
        })
        .max_by_key(|s| s.activity.as_ref().map_or(0, |a| a.at))
}

impl Claude {
    /// Číst poslední zprávu z přepisu při `Stop` (jen když je karta „Claude aktivita“ na některé záložce).
    pub fn set_activity_tail(&self, on: bool) {
        self.activity_tail.store(on, Ordering::Relaxed);
    }

    /// Číst konec přepisu? Jen při `Stop` bez `last_assistant_message` v hooku a se zapnutým čtením.
    pub(super) fn tail_wanted(&self, ev: &str, body: &Value) -> bool {
        ev == "Stop" && self.activity_tail.load(Ordering::Relaxed) && !hook_has_message(body)
    }

    /// Uloží zprávu přečtenou z přepisu, pokud session mezitím nezačala další tah.
    pub(super) fn store_message(&self, sid: &str, at: u64, msg: Option<String>) {
        let Some(msg) = msg else { return };
        let mut g = self.inner.lock();
        let changed = match g.sessions.get_mut(sid).and_then(|s| s.activity.as_mut()) {
            Some(a) if a.at == at && a.message.as_deref() != Some(msg.as_str()) => {
                a.message = Some(msg);
                true
            }
            _ => false,
        };
        drop(g);
        if changed {
            (self.on_change)();
        }
    }
}

/// Veřejný `/status` (bez autorizace) nenese text zprávy ani plnou cestu: u `activity.file` zůstane jen název.
pub fn redact(sessions: &mut Value) {
    let Some(list) = sessions.as_array_mut() else { return };
    for s in list {
        let Some(a) = s.get_mut("activity").and_then(Value::as_object_mut) else { continue };
        a.remove("message");
        if let Some(f) = a.get("file").and_then(Value::as_str).map(|f| base_name(f).to_string()) {
            a.insert("file".into(), Value::String(f));
        }
    }
}
