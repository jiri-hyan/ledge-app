//! Spuštění rychlé akce: kam vede (`resolve` — znovu přes `personal::classify`), skrytý příkaz
//! s termínem (`gitinfo::capture_tail`: bez okna, jen konec výstupu, Job Object, zabití stromu), příkaz ve viditelném
//! terminálu, aplikace s argumenty. Uložené souhlasy s příkazy (`Consents`, otisky v
//! `%LOCALAPPDATA%\Ledge\actions-consent.json` — mimo config, takže je nepřinese sync ani import)
//! a které akce právě běží (`Running`). Volá `commands/actions.rs`.

use crate::i18n::tr;
use crate::actions::{self, Action, ActionKind, Shell};
use crate::gitinfo::{self, RunError};
use crate::personal::{self, LinkTarget};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::BTreeSet;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const DETACHED_PROCESS: u32 = 0x0000_0008;
/// Kolik řádků / znaků výstupu jde do hlášky (celý konec je v logu).
const TAIL_LINES: usize = 12;
const TAIL_CHARS: usize = 1500;
const MAX_CONSENTS: usize = 256;
/// Kolik z konce stdout / stderr skrytého příkazu se drží v paměti (do hlášky jde stejně jen konec).
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// `%LOCALAPPDATA%\Ledge` (cache ikon, souhlasy); s `LEDGE_CONFIG_DIR` (testy) `<ten>\local`.
pub fn local_dir() -> PathBuf {
    if let Some(dir) = crate::config::config_dir_override() {
        return dir.join("local");
    }
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    Path::new(&base).join("Ledge")
}

// ---------- kam akce vede ----------

/// Co se otevře / spustí — výsledek `resolve` nad ověřeným cílem.
#[derive(Debug, PartialEq)]
pub enum Launch {
    Url(String),
    /// složka, dokument, `.exe` bez argumentů → ShellExecute (opener)
    Path(PathBuf),
    /// `.exe` s argumenty — přímo, bez shellu
    Exe(PathBuf),
    Command,
}

/// Ověřený cíl (`personal::classify`) proti typu akce: web jen http(s), soubor jen z allowlistu,
/// `.exe` jen jako aplikace. Příkaz se neklasifikuje (spouští se shellem).
pub fn resolve(a: &Action, checked: Option<LinkTarget>) -> Result<Launch, String> {
    let checked = match (a.kind, checked) {
        (ActionKind::Command, _) => return Ok(Launch::Command),
        (_, Some(c)) => c,
        (_, None) => return Err(tr("cíl nejde ověřit", "can't verify the target").into()),
    };
    let wrong = |what: &str| Err(trf!("cíl je {what}, ale akce je typu „{}“ — oprav typ v Nastavení", "the target is {what}, but the action's type is “{}” — fix the type in Settings", kind_name(a.kind)));
    match (a.kind, checked) {
        (_, LinkTarget::Invalid(why)) => Err(why),
        (ActionKind::Url, LinkTarget::Web(u)) => Ok(Launch::Url(u)),
        (ActionKind::Folder, LinkTarget::Folder(p)) | (ActionKind::File, LinkTarget::File(p)) => Ok(Launch::Path(p)),
        (ActionKind::App, LinkTarget::App(p)) => Ok(if a.args.is_empty() { Launch::Path(p) } else { Launch::Exe(p) }),
        (_, LinkTarget::Web(_)) => wrong(kind_name(ActionKind::Url)),
        (_, LinkTarget::Folder(_)) => wrong(kind_name(ActionKind::Folder)),
        (_, LinkTarget::File(_)) => wrong(kind_name(ActionKind::File)),
        (_, LinkTarget::App(_)) => wrong(kind_name(ActionKind::App)),
    }
}

pub fn kind_name(k: ActionKind) -> &'static str {
    match k {
        ActionKind::Url => tr("web", "website"),
        ActionKind::File => tr("soubor", "file"),
        ActionKind::Folder => tr("složka", "folder"),
        ActionKind::App => tr("aplikace", "app"),
        ActionKind::Command => tr("příkaz", "command"),
    }
}

/// Ověří cíl na disku (stat — mimo UI/emit vlákno). Příkaz → `None`.
pub fn check_fs(a: &Action) -> Option<LinkTarget> {
    (a.kind != ActionKind::Command).then(|| personal::classify_fs(&a.target))
}

/// Pracovní složka: `cwd` z akce, jinak domovská. Musí existovat.
pub fn work_dir(a: &Action) -> Result<PathBuf, String> {
    let dir = match a.cwd.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => PathBuf::from(personal::normalize(c)?),
        None => std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\")),
    };
    if dir.is_dir() { Ok(dir) } else { Err(trf!("pracovní složka {} neexistuje", "working folder {} doesn't exist", dir.display())) }
}

// ---------- příkazy ----------

/// Program a jeho argumenty pro příkaz akce. `cmd`: celá řádka jako surový argument
/// (`/d /s /c "…"`, UTF-8 výstup přes `chcp 65001`), PowerShell: `-NoProfile -Command`.
#[derive(Debug, PartialEq)]
pub struct ShellCmd {
    pub program: &'static str,
    /// `cmd`: surový řetězec (`raw_arg`); PowerShell: jednotlivé argumenty
    pub raw: Option<String>,
    pub args: Vec<String>,
}

pub fn shell_cmd(a: &Action, terminal: bool) -> ShellCmd {
    let line = actions::command_line(a);
    match (a.shell, terminal) {
        (Shell::Cmd, false) => ShellCmd { program: "cmd", raw: Some(format!("/d /s /c \"chcp 65001>nul & {line}\"")), args: vec![] },
        (Shell::Cmd, true) => ShellCmd { program: "cmd", raw: Some(format!("/d /s /k \"{line}\"")), args: vec![] },
        (Shell::Powershell, false) => ShellCmd {
            program: "powershell",
            raw: None,
            args: ["-NoProfile", "-NonInteractive", "-Command"].map(String::from).into_iter().chain([format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {line}")]).collect(),
        },
        (Shell::Powershell, true) => ShellCmd {
            program: "powershell",
            raw: None,
            args: ["-NoProfile", "-NoExit", "-Command"].map(String::from).into_iter().chain([line]).collect(),
        },
    }
}

fn build(sc: &ShellCmd, dir: &Path) -> Command {
    let mut cmd = Command::new(sc.program);
    if let Some(raw) = &sc.raw {
        cmd.raw_arg(raw);
    }
    cmd.args(&sc.args).current_dir(dir);
    cmd
}

/// Konec výstupu pro hlášku: posledních `TAIL_LINES` neprázdných řádků stdout+stderr, nejvýš
/// `TAIL_CHARS` znaků (useknuto zepředu).
pub fn tail(stdout: &str, stderr: &str) -> String {
    let lines: Vec<&str> = stdout.lines().chain(stderr.lines()).map(str::trim_end).filter(|l| !l.trim().is_empty()).collect();
    let text = lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n");
    let n = text.chars().count();
    if n <= TAIL_CHARS { text } else { format!("…{}", text.chars().skip(n - TAIL_CHARS).collect::<String>()) }
}

/// Výsledek spuštění pro UI: `hide` = zavřít šuplík (otevřelo se něco jiného).
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub message: String,
    pub output: Option<String>,
    pub hide: bool,
}

/// Skrytý příkaz: bez okna, s termínem `timeoutS`, výstup do hlášky. Nenulový exit = chyba.
pub fn run_hidden(a: &Action) -> Result<Outcome, String> {
    let dir = work_dir(a)?;
    let mut cmd = build(&shell_cmd(a, false), &dir);
    capture(&mut cmd, &a.label, a.timeout_s, true)
}

/// Spustí `cmd` bez okna s termínem `timeout_s` (pak zabije strom), drží jen konec výstupu.
/// `show` = výstup do `Outcome.output` (jinak jen hláška; při chybě se konec ukáže vždy).
/// Sdílí ho skrytý příkaz akce i skript (`scripts`).
pub fn capture(cmd: &mut Command, label: &str, timeout_s: u32, show: bool) -> Result<Outcome, String> {
    let deadline = Instant::now() + Duration::from_secs(timeout_s.into());
    match gitinfo::capture_tail(cmd, None, true, deadline, Some(MAX_OUTPUT_BYTES)) {
        Ok(out) => {
            let t = tail(&out.stdout, &out.stderr);
            if out.success {
                Ok(Outcome { message: trf!("{} — hotovo", "{} — done", label), output: (show && !t.is_empty()).then_some(t), hide: false })
            } else {
                Err(if t.is_empty() { trf!("{} skončil chybou.", "{} ended with an error.", label) } else { trf!("{} skončil chybou:\n{t}", "{} ended with an error:\n{t}", label) })
            }
        }
        Err(RunError::Timeout) => Err(trf!(
            "{} nedoběhl do {} s — ukončen. Delší příkazy pouštěj v terminálu.",
            "{} didn't finish within {} s — stopped. Run longer commands in a terminal.",
            label,
            timeout_s
        )),
        Err(RunError::Spawn(e)) => Err(trf!("{} nejde spustit: {e}", "can't run {}: {e}", label)),
    }
}

/// Spustí `cmd` v novém viditelném okně konzole (výslovný klik člověka).
pub fn spawn_console(cmd: &mut Command, label: &str) -> Result<Outcome, String> {
    cmd.creation_flags(CREATE_NEW_CONSOLE).spawn().map_err(|e| trf!("Terminál nejde otevřít: {e}", "Can't open a terminal: {e}"))?;
    Ok(Outcome { message: trf!("{} — běží v terminálu", "{} — running in a terminal", label), output: None, hide: true })
}

/// Příkaz ve viditelném terminálu (nové okno konzole; Windows Terminal, je-li výchozí).
pub fn run_terminal(a: &Action) -> Result<Outcome, String> {
    let dir = work_dir(a)?;
    spawn_console(&mut build(&shell_cmd(a, true), &dir), &a.label)
}

/// `.exe` s argumenty, přímo (bez shellu), odpojené.
pub fn run_exe(a: &Action, exe: &Path) -> Result<Outcome, String> {
    let dir = work_dir(a)?;
    Command::new(exe)
        .args(&a.args)
        .current_dir(dir)
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| trf!("{} nejde spustit: {e}", "can't run {}: {e}", a.label))?;
    Ok(Outcome { message: trf!("{} — spuštěno", "{} — started", a.label), output: None, hide: true })
}

// ---------- souhlasy ----------

/// Otisky (`actions::fingerprint`) příkazů, které člověk v Nastavení výslovně povolil.
pub struct Consents {
    path: PathBuf,
    set: Option<BTreeSet<String>>,
}

impl Consents {
    pub const fn new() -> Self {
        Consents { path: PathBuf::new(), set: None }
    }

    /// Souhlasy v jiném souboru (testy — nikdy `%LOCALAPPDATA%`).
    #[cfg(test)]
    pub fn at(path: PathBuf) -> Self {
        Consents { path, set: None }
    }

    fn file(&self) -> PathBuf {
        if self.path.as_os_str().is_empty() { local_dir().join("actions-consent.json") } else { self.path.clone() }
    }

    fn loaded(&mut self) -> &mut BTreeSet<String> {
        let path = self.file();
        self.set.get_or_insert_with(|| {
            std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok()).unwrap_or_default().into_iter().collect()
        })
    }

    pub fn has(&mut self, a: &Action) -> bool {
        self.has_fp(&actions::fingerprint(a))
    }

    /// Jen z paměti (snapshot): ještě nenačtené = bez souhlasu (ptát se).
    pub fn has_cached(&self, a: &Action) -> bool {
        self.has_cached_fp(&actions::fingerprint(a))
    }

    /// Uloží souhlas s přesně touhle akcí (atomicky přes dočasný soubor).
    pub fn grant(&mut self, a: &Action) -> Result<(), String> {
        self.grant_fp(actions::fingerprint(a))
    }

    /// Souhlas podle otisku (akce i skript — `scripts::run::fingerprint` má vlastní prefix).
    pub fn has_fp(&mut self, fp: &str) -> bool {
        self.loaded().contains(fp)
    }

    pub fn has_cached_fp(&self, fp: &str) -> bool {
        self.set.as_ref().is_some_and(|s| s.contains(fp))
    }

    pub fn grant_fp(&mut self, fp: String) -> Result<(), String> {
        let path = self.file();
        let set = self.loaded();
        if !set.insert(fp.clone()) {
            return Ok(());
        }
        // plný seznam: vyhodit jiný souhlas, nikdy ten právě udělený
        while set.len() > MAX_CONSENTS {
            let Some(old) = set.iter().find(|f| **f != fp).cloned() else { break };
            set.remove(&old);
        }
        let text = serde_json::to_string_pretty(&set.iter().collect::<Vec<_>>()).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| trf!("Souhlas nejde uložit: {e}", "Can't save the consent: {e}"))?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| trf!("Souhlas nejde uložit: {e}", "Can't save the consent: {e}"))
    }
}

pub static CONSENTS: Mutex<Consents> = Mutex::new(Consents::new());

// ---------- běžící ----------

/// Akce, které právě běží (skrytý příkaz) — dlaždice ukáže „běží“, druhý klik se odmítne.
pub static RUNNING: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Drží id v `RUNNING`; drop ho uvolní a probudí emit.
pub struct RunGuard(String);

impl RunGuard {
    pub fn take(id: &str) -> Option<RunGuard> {
        RUNNING.lock().insert(id.to_string()).then(|| RunGuard(id.to_string()))
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.lock().remove(&self.0);
        crate::state::mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(kind: ActionKind, target: &str) -> Action {
        Action { id: "a".into(), label: "A".into(), kind, target: target.into(), ..Default::default() }
    }

    /// (typ akce, ověřený cíl, očekávaný výsledek) — typ musí sedět s tím, co cíl doopravdy je.
    #[test]
    fn resolve_table() {
        let p = |s: &str| PathBuf::from(s);
        let ok = |l: Launch| Ok::<Launch, ()>(l);
        let cases: Vec<(Action, Option<LinkTarget>, Result<Launch, ()>)> = vec![
            (act(ActionKind::Url, "https://x.cz"), Some(LinkTarget::Web("https://x.cz".into())), ok(Launch::Url("https://x.cz".into()))),
            (act(ActionKind::Folder, r"C:\f"), Some(LinkTarget::Folder(p(r"C:\f"))), ok(Launch::Path(p(r"C:\f")))),
            (act(ActionKind::File, r"C:\a.txt"), Some(LinkTarget::File(p(r"C:\a.txt"))), ok(Launch::Path(p(r"C:\a.txt")))),
            (act(ActionKind::App, r"C:\a.exe"), Some(LinkTarget::App(p(r"C:\a.exe"))), ok(Launch::Path(p(r"C:\a.exe")))),
            (Action { args: vec!["-x".into()], ..act(ActionKind::App, r"C:\a.exe") }, Some(LinkTarget::App(p(r"C:\a.exe"))), ok(Launch::Exe(p(r"C:\a.exe")))),
            (act(ActionKind::Command, "dir"), None, ok(Launch::Command)),
            // typ nesedí s cílem
            (act(ActionKind::File, r"C:\a.exe"), Some(LinkTarget::App(p(r"C:\a.exe"))), Err(())),
            (act(ActionKind::Folder, r"C:\a.txt"), Some(LinkTarget::File(p(r"C:\a.txt"))), Err(())),
            (act(ActionKind::File, r"C:\x.bat"), Some(LinkTarget::Invalid("typ .bat".into())), Err(())),
            (act(ActionKind::Url, "https://x.cz"), None, Err(())),
        ];
        for (a, checked, want) in cases {
            assert_eq!(resolve(&a, checked).map_err(|_| ()), want, "{a:?}");
        }
        let e = resolve(&act(ActionKind::File, r"C:\a.exe"), Some(LinkTarget::App(p(r"C:\a.exe")))).unwrap_err();
        assert!(e.contains("aplikace") && e.contains("soubor"), "{e}");
    }

    #[test]
    fn shell_cmd_shapes() {
        let a = Action { args: vec!["--x".into(), "a b".into()], ..act(ActionKind::Command, "npm run build") };
        assert_eq!(shell_cmd(&a, false).raw.as_deref(), Some(r#"/d /s /c "chcp 65001>nul & npm run build --x "a b"""#));
        assert_eq!(shell_cmd(&a, true).raw.as_deref(), Some(r#"/d /s /k "npm run build --x "a b"""#));
        let ps = Action { shell: Shell::Powershell, ..a };
        let h = shell_cmd(&ps, false);
        assert_eq!((h.program, h.raw.is_none(), h.args[2].as_str()), ("powershell", true, "-Command"));
        assert!(h.args[3].ends_with("npm run build --x 'a b'"), "{}", h.args[3]);
        assert!(shell_cmd(&ps, true).args.contains(&"-NoExit".to_string()));
    }

    #[test]
    fn tail_keeps_last_lines_and_chars() {
        assert_eq!(tail("a\n\nb\n", "c\n"), "a\nb\nc");
        let many: String = (0..30).map(|i| format!("r{i}\n")).collect();
        let t = tail(&many, "");
        assert_eq!(t.lines().count(), TAIL_LINES);
        assert!(t.ends_with("r29"));
        let long = "x".repeat(TAIL_CHARS + 50);
        let t = tail(&long, "");
        assert!(t.starts_with('…') && t.chars().count() == TAIL_CHARS + 1);
    }

    #[test]
    fn consents_persist_per_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("c.json");
        let a = act(ActionKind::Command, "dir");
        let mut c = Consents { path: path.clone(), set: None };
        assert!(!c.has(&a));
        c.grant(&a).unwrap();
        assert!(c.has(&a));
        // nový stav ze souboru (restart Ledge)
        let mut again = Consents { path, set: None };
        assert!(again.has(&a));
        assert!(!again.has(&Action { target: "del /q *".into(), ..a.clone() }), "jiný příkaz souhlas nemá");
        assert!(again.has(&Action { label: "Přejmenováno".into(), ..a }));
    }

    #[test]
    fn hidden_command_runs_with_output_and_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = Some(dir.path().to_string_lossy().into_owned());
        let ok = Action { cwd: cwd.clone(), ..act(ActionKind::Command, "echo ahoj") };
        let out = run_hidden(&ok).unwrap();
        assert_eq!(out.output.as_deref(), Some("ahoj"));
        let fail = Action { cwd: cwd.clone(), ..act(ActionKind::Command, "exit 3") };
        assert!(run_hidden(&fail).unwrap_err().contains("chybou"));
        let slow = Action { cwd, timeout_s: 1, ..act(ActionKind::Command, "ping -n 5 127.0.0.1") };
        assert!(run_hidden(&slow).unwrap_err().contains("nedoběhl"));
        let gone = Action { cwd: Some(r"C:\neexistuje-ledge-test".into()), ..act(ActionKind::Command, "echo x") };
        assert!(run_hidden(&gone).unwrap_err().contains("neexistuje"));
    }

    /// Argumenty dojdou do programu doslova (skutečný cmd / PowerShell, bez okna): koncová `\`
    /// v cestě s mezerou, `$`, apostrof, `#`.
    #[test]
    fn hidden_command_passes_args_literally() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = Some(dir.path().to_string_lossy().into_owned());
        let cmd = Action { cwd: cwd.clone(), args: vec![r"C:\Moje složka\".into(), "dál".into()], ..act(ActionKind::Command, "git rev-parse --sq-quote") };
        assert_eq!(run_hidden(&cmd).unwrap().output.as_deref(), Some(r" 'C:\Moje složka\' 'dál'"));
        let args: Vec<String> = ["a b$HOME", "it's", "#1", "x,y"].map(String::from).into();
        let ps = Action { cwd, shell: Shell::Powershell, args: args.clone(), ..act(ActionKind::Command, "Write-Output") };
        assert_eq!(run_hidden(&ps).unwrap().output.as_deref(), Some(args.join("\n").as_str()));
    }

    /// Ukecaný příkaz: v paměti zůstane jen konec výstupu, ne všechno, co vypsal.
    #[test]
    fn read_tail_keeps_only_the_end() {
        let data: Vec<u8> = (0..200_000u32).flat_map(|i| format!("{i}\n").into_bytes()).collect();
        let (buf, cut) = gitinfo::read_tail(&mut data.as_slice(), Some(1000));
        assert!(cut && buf.len() <= 1000 && data.ends_with(&buf));
        assert!(String::from_utf8_lossy(&buf).ends_with("199999\n"));
        let (all, cut) = gitinfo::read_tail(&mut data.as_slice(), None);
        assert!(!cut && all == data);
        let (small, cut) = gitinfo::read_tail(&mut &b"ahoj"[..], Some(1000));
        assert!(!cut && small == b"ahoj");
    }

    #[test]
    fn consent_survives_full_store() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = Consents { path: dir.path().join("c.json"), set: None };
        for i in 0..MAX_CONSENTS {
            c.grant(&act(ActionKind::Command, &format!("echo {i}"))).unwrap();
        }
        // nový souhlas, jehož otisk se řadí před všechny uložené (dřív ho úklid hned vyhodil)
        let min = c.loaded().iter().next().cloned().unwrap();
        let first = (0..).map(|j| act(ActionKind::Command, &format!("echo z{j}"))).find(|a| actions::fingerprint(a) < min).unwrap();
        c.grant(&first).unwrap();
        assert!(c.has(&first), "právě udělený souhlas platí");
        let mut again = Consents { path: dir.path().join("c.json"), set: None };
        assert!(again.has(&first));
        assert_eq!(again.loaded().len(), MAX_CONSENTS);
    }

    #[test]
    fn run_guard_blocks_second_run() {
        let g = RunGuard::take("guard-test").unwrap();
        assert!(RunGuard::take("guard-test").is_none());
        drop(g);
        assert!(RunGuard::take("guard-test").is_some());
    }
}
