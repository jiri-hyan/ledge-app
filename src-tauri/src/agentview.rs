//! Napojení na Claude Code Agent View (session na pozadí) — čistá logika bez I/O.
//! Zdroj pravdy: `claude agents --json --all` (pole `kind`, `id`, `state`, `waitingFor`, docs
//! code.claude.com/docs/en/agent-view) a příkazy `claude attach|logs|stop|rm|respawn <id>`,
//! `claude --bg [--name N] "<úkol>"` (ověřeno `--help` v CLI 2.1.259). Poslat zprávu do běžící session
//! z shellu docs neuvádějí → Ledge to nedělá (žádné nedokumentované soubory `~/.claude/jobs`).
//! I/O (spuštění CLI bez okna, terminál pro `attach`) je v `agentrun.rs`, příkazy v `commands/agents.rs`.

use crate::i18n::tr;
use crate::sessions::Launch;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Strop výstupu `claude logs` (konec výstupu — nejnovější řádky).
pub const LOG_MAX: usize = 32 * 1024;
/// Strop textu úkolu pro `claude --bg` (příkazová řádka Windows má 32 767 znaků, `cmd` 8 191).
pub const PROMPT_MAX: usize = 4000;
/// Strop jména session (`--name`).
pub const NAME_MAX: usize = 80;

/// `state` session na pozadí (docs: working | blocked | done | failed | stopped). Neznámá hodnota = `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Working,
    Blocked,
    Done,
    Failed,
    Stopped,
}

pub fn job_state(s: Option<&str>) -> Option<JobState> {
    let s = s?.trim();
    [("working", JobState::Working), ("blocked", JobState::Blocked), ("done", JobState::Done), ("failed", JobState::Failed), ("stopped", JobState::Stopped)]
        .into_iter()
        .find(|(k, _)| s.eq_ignore_ascii_case(k))
        .map(|(_, v)| v)
}

/// `waitingFor` (docs: permission prompt | input needed | sandbox request | worker request | dialog open).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WaitFor {
    Permission,
    Input,
    Sandbox,
    Worker,
    Dialog,
}

pub fn wait_for(s: Option<&str>) -> Option<WaitFor> {
    let s = s?.trim();
    [
        ("permission prompt", WaitFor::Permission),
        ("input needed", WaitFor::Input),
        ("sandbox request", WaitFor::Sandbox),
        ("worker request", WaitFor::Worker),
        ("dialog open", WaitFor::Dialog),
    ]
    .into_iter()
    .find(|(k, _)| s.eq_ignore_ascii_case(k))
    .map(|(_, v)| v)
}

/// Akce nad session na pozadí (jen klikem v kartě).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobAction {
    /// `claude attach <id>` v novém okně terminálu
    Attach,
    /// `claude logs <id>` bez okna, výstup do karty
    Logs,
    /// `claude stop <id>`
    Stop,
    /// `claude rm <id>` (smaže session i její worktree; přepis zůstane pro `claude --resume`)
    Remove,
    /// `claude respawn <id>`
    Respawn,
}

impl JobAction {
    /// Mění stav session → UI se zeptá a Rust bez `confirmed` nic nespustí.
    pub fn needs_confirm(self) -> bool {
        matches!(self, JobAction::Stop | JobAction::Remove | JobAction::Respawn)
    }

    /// Termín běhu CLI (bez okna).
    pub fn wait(self) -> Duration {
        Duration::from_secs(match self {
            JobAction::Logs => 15,
            JobAction::Stop => 20,
            JobAction::Attach | JobAction::Remove | JobAction::Respawn => 30,
        })
    }
}

/// Session na pozadí pro kartu: krátké id, stav úlohy a akce, které dávají smysl.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub state: Option<JobState>,
    pub actions: Vec<JobAction>,
}

/// `None` = bez platného id (nic nenabízet). `proc_alive` = seznam uvádí `pid`/`status` (proces žije).
pub fn job(id: Option<&str>, state: Option<&str>, proc_alive: bool) -> Option<Job> {
    let id = id.filter(|i| valid_job_id(i))?;
    let state = job_state(state);
    Some(Job { id: id.to_string(), state, actions: actions(state, proc_alive) })
}

/// Které akce ukázat. Docs: `respawn` = „running or stopped“, `rm` jen u skončených (zadání),
/// `stop` jen u běžící / rozpracované; `attach` a `logs` vždy (stop help: „resume it later with attach“).
pub fn actions(state: Option<JobState>, proc_alive: bool) -> Vec<JobAction> {
    use JobState::*;
    let mut v = vec![JobAction::Attach, JobAction::Logs];
    if proc_alive || matches!(state, Some(Working | Blocked)) {
        v.push(JobAction::Stop);
    }
    if proc_alive || state == Some(Stopped) {
        v.push(JobAction::Respawn);
    }
    if matches!(state, Some(Done | Failed | Stopped)) {
        v.push(JobAction::Remove);
    }
    v
}

/// Krátké id úlohy (`7c5dcf5d`) bezpečné pro příkazovou řádku: písmena/číslice/`-`/`_`, začíná
/// písmenem nebo číslicí (pomlčka na začátku by byla přepínač).
pub fn valid_job_id(id: &str) -> bool {
    (4..=64).contains(&id.len()) && id.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Argumenty `claude` pro akci bez okna (`Attach` jde do terminálu → `attach_command`).
pub fn cli_args(a: JobAction, id: &str) -> Option<Vec<String>> {
    if !valid_job_id(id) {
        return None;
    }
    let verb = match a {
        JobAction::Attach => return None,
        JobAction::Logs => "logs",
        JobAction::Stop => "stop",
        JobAction::Remove => "rm",
        JobAction::Respawn => "respawn",
    };
    Some(vec![verb.to_string(), id.to_string()])
}

/// `claude attach <id>` v novém okně terminálu (stejná pravidla jako `sessions::resume_command`).
pub fn attach_command(wt: bool, cwd: &str, id: &str) -> Option<Launch> {
    if !valid_job_id(id) || cwd.trim().is_empty() || cwd.contains(['"', '\r', '\n', '%', '^', '&', '|', '<', '>']) {
        return None;
    }
    if wt && !cwd.contains(';') {
        let args = ["-d", cwd, "cmd", "/c", "claude", "attach", id];
        return Some(Launch::Wt(args.iter().map(|s| s.to_string()).collect()));
    }
    Some(Launch::Cmd(format!("/d /c start \"\" /d \"{cwd}\" cmd /c claude attach {id}")))
}

/// Úkol pro `claude --bg`: bez řídicích znaků (kromě nového řádku), ořezaný, ne prázdný, nezačíná
/// pomlčkou (byl by to přepínač), strop `PROMPT_MAX` znaků. Chyba = text pro člověka.
pub fn clean_prompt(s: &str) -> Result<String, &'static str> {
    let s: String = s.replace("\r\n", "\n").chars().map(|c| if c == '\t' { ' ' } else { c }).filter(|c| *c == '\n' || !c.is_control()).collect();
    let s = s.trim();
    if s.is_empty() {
        return Err(tr("Napiš, co má Claude udělat.", "Write what Claude should do."));
    }
    if s.starts_with('-') {
        return Err(tr("Úkol nesmí začínat pomlčkou.", "The task can't start with a dash."));
    }
    if s.chars().count() > PROMPT_MAX {
        return Err(tr("Úkol je moc dlouhý (nejvýš 4000 znaků).", "The task is too long (at most 4000 characters)."));
    }
    Ok(s.to_string())
}

/// Volitelné jméno (`--name`): prázdné = `None` (jméno dá Claude), jeden řádek, strop `NAME_MAX`.
pub fn clean_name(s: Option<&str>) -> Result<Option<String>, &'static str> {
    let s: String = s.unwrap_or_default().chars().filter(|c| !c.is_control()).collect();
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    if s.starts_with('-') {
        return Err(tr("Jméno nesmí začínat pomlčkou.", "The name can't start with a dash."));
    }
    if s.chars().count() > NAME_MAX {
        return Err(tr("Jméno je moc dlouhé (nejvýš 80 znaků).", "The name is too long (at most 80 characters)."));
    }
    Ok(Some(s.to_string()))
}

/// Argumenty `claude --bg [--name N] <úkol>` (pole, žádný shell; úkol je poziční argument podle docs).
pub fn bg_args(prompt: &str, name: Option<&str>) -> Result<Vec<String>, &'static str> {
    let prompt = clean_prompt(prompt)?;
    let mut v = vec!["--bg".to_string()];
    if let Some(n) = clean_name(name)? {
        v.push("--name".into());
        v.push(n);
    }
    v.push(prompt);
    Ok(v)
}

/// Spuštění přes `claude.cmd` (dávka): Rust argumenty pro dávku escapuje, nový řádek ale dávkou
/// neprojde → mezera.
pub fn for_batch(args: &[String]) -> Vec<String> {
    args.iter().map(|a| a.replace('\n', " ")).collect()
}

/// Id z výpisu `claude --bg` („backgrounded · 7c5dcf5d · name“ podle docs); jinak `None`.
pub fn backgrounded_id(out: &str) -> Option<String> {
    let line = out.lines().find(|l| l.trim_start().starts_with("backgrounded"))?;
    let id = line.split('·').nth(1)?.trim();
    valid_job_id(id).then(|| id.to_string())
}

/// Výstup terminálu do textu karty: bez ANSI sekvencí (CSI, OSC) a řídicích znaků, `\r` = nový
/// řádek, nejvýš jeden prázdný řádek za sebou. Na začátku useknutý znak UTF-8 se zahodí.
pub fn clean_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\u{1b}' => match it.next() {
                // CSI: parametry až po koncový bajt 0x40–0x7E
                Some('[') => {
                    for d in it.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&d) {
                            break;
                        }
                    }
                }
                // OSC: až po BEL nebo ESC \
                Some(']') => {
                    while let Some(d) = it.next() {
                        if d == '\u{7}' || (d == '\u{1b}' && it.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\r' => {
                if it.peek() != Some(&'\n') {
                    out.push('\n');
                }
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    let mut lines: Vec<&str> = Vec::new();
    for l in out.lines().map(str::trim_end) {
        if !(l.is_empty() && lines.last().is_some_and(|p| p.is_empty())) {
            lines.push(l);
        }
    }
    lines.join("\n").trim_start_matches('\u{fffd}').trim_matches('\n').to_string()
}

/// Čím se `claude` spustí bez shellu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Program {
    /// `claude.exe` (nativní instalace, nebo binárka npm balíčku vedle `claude.cmd`)
    Exe(PathBuf),
    /// jen `claude.cmd` → dávka (Rust escapuje argumenty pro `cmd`)
    Batch(PathBuf),
}

/// Npm obálka `claude.cmd` jen volá tuhle binárku (`%dp0%\node_modules\@anthropic-ai\claude-code\bin\claude.exe`).
const NPM_EXE: &str = r"node_modules\@anthropic-ai\claude-code\bin\claude.exe";

/// První `claude` na PATH (pořadí složek jako v `cmd`), aliasy Storu (`WindowsApps`) se přeskočí.
pub fn resolve_claude(dirs: &[PathBuf], is_file: &dyn Fn(&Path) -> bool) -> Option<Program> {
    for d in dirs.iter().filter(|d| !d.as_os_str().is_empty() && !d.to_string_lossy().to_ascii_lowercase().contains(r"\microsoft\windowsapps")) {
        let exe = d.join("claude.exe");
        if is_file(&exe) {
            return Some(Program::Exe(exe));
        }
        let cmd = d.join("claude.cmd");
        if is_file(&cmd) {
            let inner = d.join(NPM_EXE);
            return Some(if is_file(&inner) { Program::Exe(inner) } else { Program::Batch(cmd) });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_and_waits_table() {
        let st: &[(Option<&str>, Option<JobState>)] = &[
            (Some("working"), Some(JobState::Working)),
            (Some("BLOCKED"), Some(JobState::Blocked)),
            (Some(" done "), Some(JobState::Done)),
            (Some("failed"), Some(JobState::Failed)),
            (Some("stopped"), Some(JobState::Stopped)),
            (Some("paused"), None),
            (Some(""), None),
            (None, None),
        ];
        for (s, want) in st {
            assert_eq!(job_state(*s), *want, "{s:?}");
        }
        let wf: &[(Option<&str>, Option<WaitFor>)] = &[
            (Some("permission prompt"), Some(WaitFor::Permission)),
            (Some("input needed"), Some(WaitFor::Input)),
            (Some("sandbox request"), Some(WaitFor::Sandbox)),
            (Some("worker request"), Some(WaitFor::Worker)),
            (Some("Dialog Open"), Some(WaitFor::Dialog)),
            (Some("coffee"), None),
            (Some("permission"), None),
            (None, None),
        ];
        for (s, want) in wf {
            assert_eq!(wait_for(*s), *want, "{s:?}");
        }
        assert_eq!(serde_json::to_string(&WaitFor::Permission).unwrap(), "\"permission\"");
        assert_eq!(serde_json::to_string(&JobState::Stopped).unwrap(), "\"stopped\"");
        assert_eq!(serde_json::from_str::<JobAction>("\"remove\"").unwrap(), JobAction::Remove);
        assert!(serde_json::from_str::<JobAction>("\"exec\"").is_err());
    }

    #[test]
    fn actions_table() {
        use JobAction::*;
        use JobState::*;
        let cases: &[(Option<JobState>, bool, &[JobAction])] = &[
            (Some(Working), true, &[Attach, Logs, Stop, Respawn]),
            (Some(Blocked), true, &[Attach, Logs, Stop, Respawn]),
            // proces skončil, úloha pořád rozpracovaná (docs: v seznamu i bez procesu)
            (Some(Working), false, &[Attach, Logs, Stop]),
            (Some(Done), true, &[Attach, Logs, Stop, Respawn, Remove]),
            (Some(Done), false, &[Attach, Logs, Remove]),
            (Some(Failed), false, &[Attach, Logs, Remove]),
            (Some(Stopped), false, &[Attach, Logs, Respawn, Remove]),
            (None, false, &[Attach, Logs]),
            (None, true, &[Attach, Logs, Stop, Respawn]),
        ];
        for (st, alive, want) in cases {
            assert_eq!(actions(*st, *alive), *want, "{st:?} {alive}");
        }
        assert!(Stop.needs_confirm() && Remove.needs_confirm() && Respawn.needs_confirm());
        assert!(!Attach.needs_confirm() && !Logs.needs_confirm());
        assert_eq!(job(Some("7c5dcf5d"), Some("failed"), false), Some(Job { id: "7c5dcf5d".into(), state: Some(Failed), actions: vec![Attach, Logs, Remove] }));
        assert_eq!(job(Some("7c5dcf5d"), Some("neznámý"), false).unwrap().state, None);
        assert_eq!(job(None, Some("done"), false), None);
        assert_eq!(job(Some("-rf"), Some("done"), false), None);
    }

    #[test]
    fn job_id_and_args() {
        for (id, ok) in [("7c5dcf5d", true), ("abcd", true), ("a-b_c9", true), ("abc", false), ("-help", false), ("_x12", false), ("7c5d cf", false), ("7c5d&calc", false), ("x\"y12", false), ("é1234", false), ("", false)] {
            assert_eq!(valid_job_id(id), ok, "{id}");
        }
        assert!(!valid_job_id(&"a".repeat(65)));
        let v = |a: &[&str]| Some(a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(cli_args(JobAction::Logs, "7c5dcf5d"), v(&["logs", "7c5dcf5d"]));
        assert_eq!(cli_args(JobAction::Stop, "7c5dcf5d"), v(&["stop", "7c5dcf5d"]));
        assert_eq!(cli_args(JobAction::Remove, "7c5dcf5d"), v(&["rm", "7c5dcf5d"]));
        assert_eq!(cli_args(JobAction::Respawn, "7c5dcf5d"), v(&["respawn", "7c5dcf5d"]));
        assert_eq!(cli_args(JobAction::Attach, "7c5dcf5d"), None);
        assert_eq!(cli_args(JobAction::Stop, "x & calc"), None);
    }

    #[test]
    fn attach_command_table() {
        let id = "7c5dcf5d";
        assert_eq!(attach_command(true, r"C:\p", id), Some(Launch::Wt(["-d", r"C:\p", "cmd", "/c", "claude", "attach", id].iter().map(|s| s.to_string()).collect())));
        assert_eq!(attach_command(false, r"C:\a b", id), Some(Launch::Cmd(format!("/d /c start \"\" /d \"C:\\a b\" cmd /c claude attach {id}"))));
        assert!(matches!(attach_command(true, r"C:\a;b", id), Some(Launch::Cmd(_))));
        for (cwd, i) in [(r"C:\p", "x&y1"), (r"C:\p", "-all"), (r#"C:\p" & calc"#, id), (r"C:\%P%", id), (" ", id)] {
            assert_eq!(attach_command(true, cwd, i), None, "{cwd} {i}");
        }
    }

    #[test]
    fn bg_args_table() {
        let s = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(bg_args("  oprav testy  ", None), Ok(s(&["--bg", "oprav testy"])));
        assert_eq!(bg_args("oprav testy", Some("  ")), Ok(s(&["--bg", "oprav testy"])));
        assert_eq!(bg_args("oprav \"testy\" & calc %PATH%", Some("flaky fix")), Ok(s(&["--bg", "--name", "flaky fix", "oprav \"testy\" & calc %PATH%"])), "metaznaky jdou jako data");
        assert_eq!(bg_args("řádek 1\r\nřádek 2\u{7}\t!", None), Ok(s(&["--bg", "řádek 1\nřádek 2 !"])));
        assert!(bg_args("   ", None).is_err());
        assert!(bg_args("--dangerously-skip-permissions", None).is_err());
        assert!(bg_args("ok", Some("-x")).is_err());
        assert!(bg_args(&"x".repeat(PROMPT_MAX + 1), None).is_err());
        assert!(bg_args(&"ž".repeat(PROMPT_MAX), None).is_ok(), "strop ve znacích, ne bajtech");
        assert!(bg_args("ok", Some(&"n".repeat(NAME_MAX + 1))).is_err());
        assert_eq!(clean_name(Some("a\nb")), Ok(Some("ab".into())));
        assert_eq!(for_batch(&s(&["--bg", "a\nb"])), s(&["--bg", "a b"]));
    }

    #[test]
    fn backgrounded_and_output() {
        let out = "backgrounded · 7c5dcf5d · flaky-test-fix\n  claude agents             list sessions\n  claude attach 7c5dcf5d    open in this terminal\n";
        assert_eq!(backgrounded_id(out).as_deref(), Some("7c5dcf5d"));
        assert_eq!(backgrounded_id("něco jiného"), None);
        assert_eq!(backgrounded_id("backgrounded · x&y · n"), None);
        let cases: &[(&[u8], &str)] = &[
            (b"\x1b[1;32mok\x1b[0m hotovo", "ok hotovo"),
            (b"\x1b]0;titulek\x07text", "text"),
            (b"\x1b]8;;https://x\x1b\\odkaz\x1b]8;;\x1b\\", "odkaz"),
            (b"a\r\nb\rc", "a\nb\nc"),
            (b"a\n\n\n\nb\n\n", "a\n\nb"),
            (b"x\x08\x00y\tz", "xy\tz"),
            (b"\x1b[?25l\x1b[2K", ""),
            ("\u{17e}lu\u{165}ou\u{10d}k\u{fd}".as_bytes(), "žluťoučký"),
        ];
        for (inp, want) in cases {
            assert_eq!(clean_output(inp), *want, "{inp:?}");
        }
        // useknutý začátek vícebajtového znaku (strop logů) se zahodí
        let z = "žx".as_bytes();
        assert_eq!(clean_output(&z[1..]), "x");
    }

    #[test]
    fn resolve_claude_table() {
        let d = |s: &str| PathBuf::from(s);
        let files = [r"C:\npm\claude.cmd", r"C:\npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe", r"C:\only\claude.cmd", r"C:\nat\claude.exe", r"C:\Users\x\AppData\Local\Microsoft\WindowsApps\claude.exe"];
        let is_file = |p: &Path| files.iter().any(|f| Path::new(f) == p);
        assert_eq!(resolve_claude(&[d(r"C:\nic"), d(r"C:\npm")], &is_file), Some(Program::Exe(d(r"C:\npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe"))));
        assert_eq!(resolve_claude(&[d(r"C:\only"), d(r"C:\nat")], &is_file), Some(Program::Batch(d(r"C:\only\claude.cmd"))));
        assert_eq!(resolve_claude(&[d(r"C:\Users\x\AppData\Local\Microsoft\WindowsApps"), d(r"C:\nat")], &is_file), Some(Program::Exe(d(r"C:\nat\claude.exe"))));
        assert_eq!(resolve_claude(&[d(""), d(r"C:\nic")], &is_file), None);
    }
}
