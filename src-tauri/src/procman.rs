//! Správce spuštěných projektů (dev servery a aplikace).
//!
//! Převzato z dřívějšího spouštěče v Electronu (`runner.ts`, `proc.ts`) a opraveno:
//! - každý dev server žije v **Job Objectu s KILL_ON_JOB_CLOSE** → Stop zabije celý strom
//!   (cmd → npm → node → esbuild…) a když Ledge skončí, nezůstane žádný sirotek držící port;
//! - čekání na naběhnutí **sleduje i konec procesu** → pád je hlášen hned, ne po timeoutu;
//! - URL z výstupu se hledá po **řádcích bez ANSI**, ne v náhodně useknutých chuncích;
//! - obsazený port se nepovažuje za „náš server“ — zjistí se, kdo ho drží, a přidělí se jiný;
//! - nic se nespouští s viditelnou konzolí (CREATE_NO_WINDOW);
//! - chybějící exe/složka = srozumitelná chyba, ne pád.

use crate::ports;
use parking_lot::Mutex;
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read};
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    TerminateJobObject,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows::Win32::System::Threading::{
    CREATE_SUSPENDED, OpenProcess, OpenThread, PROCESS_QUERY_LIMITED_INFORMATION, ResumeThread, THREAD_SUSPEND_RESUME,
};
use windows::core::PCWSTR;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const LOG_LINES: usize = 400;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)https?://(?:localhost|127\.0\.0\.1|\[::1\]):(\d{2,5})").unwrap());
static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*\x07").unwrap());

// ---------- Job Object ----------

/// Job Object s KILL_ON_JOB_CLOSE — sdílí ho i `gitinfo::run` (zabití celého stromu gitu).
pub(crate) struct Job(HANDLE);
unsafe impl Send for Job {}
unsafe impl Sync for Job {}

impl Job {
    pub(crate) fn new() -> Result<Self, String> {
        unsafe {
            let h = CreateJobObjectW(None, PCWSTR::null()).map_err(|e| e.to_string())?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .map_err(|e| e.to_string())?;
            Ok(Job(h))
        }
    }
    fn assign(&self, child: &Child) -> Result<(), String> {
        unsafe {
            AssignProcessToJobObject(self.0, HANDLE(child.as_raw_handle())).map_err(|e| e.to_string())
        }
    }
    /// Spustí proces rovnou v jobu: vznikne pozastavený, přiřadí se a teprve pak se rozběhne.
    /// Přiřazení až po běžném `spawn()` je závod — cmd.exe stihne pustit node dřív, než je
    /// v jobu, a vnuk pak žije mimo job (Stop ho nezabije, port drží sirotek).
    /// `flags` = ostatní příznaky (CREATE_NO_WINDOW…); `creation_flags` je přepisuje, ne sčítá.
    pub(crate) fn spawn(&self, cmd: &mut Command, flags: u32) -> Result<Child, String> {
        let mut child = cmd
            .creation_flags(flags | CREATE_SUSPENDED.0)
            .spawn()
            .map_err(|e| trf!("Nepodařilo se spustit: {e}", "Couldn't start: {e}"))?;
        let res = self.assign(&child).map_err(|e| format!("Job Object: {e}")).and_then(|()| resume_threads(child.id()));
        if let Err(e) = res {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        Ok(child)
    }
    fn contains(&self, pid: u32) -> bool {
        unsafe {
            let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
            let mut res = windows::core::BOOL(0);
            let ok = IsProcessInJob(p, Some(self.0), &mut res).is_ok() && res.as_bool();
            let _ = CloseHandle(p);
            ok
        }
    }
    pub(crate) fn kill(&self) {
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0); // KILL_ON_JOB_CLOSE → zbytek stromu umře
        }
    }
}

/// Rozběhne pozastavený proces (std nevydá handle hlavního vlákna → najdeme ho přes ToolHelp).
fn resume_threads(pid: u32) -> Result<(), String> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| format!("ToolHelp: {e}"))?;
        let mut te = THREADENTRY32 { dwSize: std::mem::size_of::<THREADENTRY32>() as u32, ..Default::default() };
        let mut resumed = 0;
        let mut more = Thread32First(snap, &mut te).is_ok();
        while more {
            if te.th32OwnerProcessID == pid
                && let Ok(t) = OpenThread(THREAD_SUSPEND_RESUME, false, te.th32ThreadID)
            {
                if ResumeThread(t) != u32::MAX {
                    resumed += 1;
                }
                let _ = CloseHandle(t);
            }
            more = Thread32Next(snap, &mut te).is_ok();
        }
        let _ = CloseHandle(snap);
        if resumed == 0 {
            return Err(trf!("Nepodařilo se rozběhnout proces {pid}.", "Couldn't get process {pid} running."));
        }
        Ok(())
    }
}

// ---------- veřejné typy ----------

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// app: běží přípravný příkaz (build)
    Preparing,
    /// dev: server startuje
    Starting,
    /// dev: server odpovídá
    Ready,
    /// app: aplikace běží
    Running,
    /// skončil sám (po naběhnutí)
    Exited,
    /// nenaběhl / spadl při startu / chyba
    Failed,
}

impl Phase {
    pub fn is_live(self) -> bool {
        matches!(self, Phase::Preparing | Phase::Starting | Phase::Ready | Phase::Running)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcView {
    pub id: String,
    pub phase: Phase,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub url: Option<String>,
    /// unix ms
    pub started_at: u64,
    /// unix ms, kdy naběhl (dev) / spustil se (app)
    pub ready_at: Option<u64>,
    pub exit_code: Option<i32>,
    /// lidská zpráva (proč selhal, proč jiný port…)
    pub message: Option<String>,
    pub last_line: Option<String>,
    /// počet řádků, které vypadají jako chyba
    pub errors: u32,
    /// odkud to běží (složka) a co (příkaz po doplnění portu / exe) — dlaždice, pilulka, Běží
    pub cwd: Option<String>,
    pub cmd: Option<String>,
    /// poslední řádky výstupu při selhání (`fail_tail`), jinak prázdné — snapshot zůstává malý
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tail: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ProcEvent {
    Changed,
    /// dev server naběhl (konzument potřebuje jen URL; kdo naběhl, ukáže snapshot)
    Ready { url: String, open_browser: bool },
    /// spadl / nenaběhl — `message` = lidská zpráva (toast, když šuplík není otevřený)
    Failed { id: String, message: String },
}

/// Kolik posledních řádků logu ukáže selhání na dlaždici.
pub const TAIL_LINES: usize = 4;

/// Poslední `n` řádků výstupu bez řádků Ledge (`› příkaz`) — co ukázat u selhání.
pub fn fail_tail<'a>(log: impl DoubleEndedIterator<Item = &'a String>, n: usize) -> Vec<String> {
    let mut v: Vec<String> = log.rev().filter(|l| !l.starts_with('›') && !l.trim().is_empty()).take(n).cloned().collect();
    v.reverse();
    v
}

pub type Notify = Arc<dyn Fn(ProcEvent) + Send + Sync>;

pub struct DevSpec {
    pub id: String,
    pub cwd: PathBuf,
    pub run: String,
    pub port: Option<u16>,
    pub pass_port: bool,
    pub open_browser: bool,
    pub timeout: Duration,
}

pub struct AppSpec {
    pub id: String,
    pub cwd: Option<PathBuf>,
    pub prepare: Option<String>,
    pub exe: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum LaunchOutcome {
    Started,
    AlreadyRunning { url: Option<String> },
}

struct Entry {
    generation: u64,
    view: ProcView,
    log: VecDeque<String>,
    job: Option<Job>,
}

#[derive(Clone)]
pub struct ProcMan {
    inner: Arc<Mutex<Inner>>,
    notify: Notify,
}

struct Inner {
    next_gen: u64,
    entries: HashMap<String, Entry>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// `npm run x` potřebuje `-- --port N`; pnpm/yarn/bun předají `--port N` rovnou
/// (u pnpm by se `--` předalo skriptu doslova a Vite by port ignoroval).
pub fn with_port(run: &str, port: u16, pass_port: bool) -> String {
    if !pass_port {
        return run.to_string();
    }
    let first = run.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
    match first.as_str() {
        "npm" | "npm.cmd" => format!("{run} -- --port {port}"),
        "pnpm" | "pnpm.cmd" | "yarn" | "yarn.cmd" | "bun" => format!("{run} --port {port}"),
        _ => run.to_string(),
    }
}

pub fn clean_line(raw: &str) -> String {
    ANSI_RE.replace_all(raw, "").trim_end().to_string()
}

pub fn url_port(line: &str) -> Option<u16> {
    URL_RE.captures(line).and_then(|c| c[1].parse().ok())
}

fn looks_like_error(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("error") && !l.contains("0 error") || l.contains("failed to") || l.contains("eaddrinuse")
}

impl ProcMan {
    pub fn new(notify: Notify) -> Self {
        Self { inner: Arc::new(Mutex::new(Inner { next_gen: 1, entries: HashMap::new() })), notify }
    }

    pub fn snapshot(&self) -> Vec<ProcView> {
        let g = self.inner.lock();
        let mut v: Vec<ProcView> = g.entries.values().map(|e| e.view.clone()).collect();
        v.sort_by_key(|p| p.started_at);
        v
    }

    pub fn log(&self, id: &str) -> Vec<String> {
        self.inner.lock().entries.get(id).map(|e| e.log.iter().cloned().collect()).unwrap_or_default()
    }

    /// Který náš projekt drží daný PID (proces v jeho jobu).
    pub fn project_for_pid(&self, pid: u32) -> Option<String> {
        let g = self.inner.lock();
        g.entries.iter().find(|(_, e)| e.job.as_ref().is_some_and(|j| j.contains(pid))).map(|(k, _)| k.clone())
    }

    /// Zastaví projekt (celý strom procesů) a zapomene ho.
    pub fn stop(&self, id: &str) {
        let entry = self.inner.lock().entries.remove(id);
        if let Some(j) = entry.as_ref().and_then(|e| e.job.as_ref()) {
            j.kill();
        }
        (self.notify)(ProcEvent::Changed);
    }

    /// Smaže skončený/selhaný záznam z přehledu.
    pub fn dismiss(&self, id: &str) {
        let mut g = self.inner.lock();
        if g.entries.get(id).is_some_and(|e| !e.view.phase.is_live()) {
            g.entries.remove(id);
        }
        drop(g);
        (self.notify)(ProcEvent::Changed);
    }

    pub fn stop_all(&self) {
        let all: Vec<Entry> = self.inner.lock().entries.drain().map(|(_, e)| e).collect();
        for e in all {
            if let Some(j) = &e.job {
                j.kill();
            }
        }
    }

    fn begin(&self, id: &str, phase: Phase) -> Result<Option<LaunchOutcome>, u64> {
        let mut g = self.inner.lock();
        if let Some(e) = g.entries.get(id)
            && e.view.phase.is_live()
        {
            return Ok(Some(LaunchOutcome::AlreadyRunning { url: e.view.url.clone() }));
        }
        let generation = g.next_gen;
        g.next_gen += 1;
        g.entries.insert(
            id.to_string(),
            Entry {
                generation,
                view: ProcView {
                    id: id.to_string(),
                    phase,
                    pid: None,
                    port: None,
                    url: None,
                    started_at: now_ms(),
                    ready_at: None,
                    exit_code: None,
                    message: None,
                    last_line: None,
                    errors: 0,
                    cwd: None,
                    cmd: None,
                    tail: Vec::new(),
                },
                log: VecDeque::new(),
                job: None,
            },
        );
        Err(generation)
    }

    /// Upraví záznam, jen pokud patří stále stejnému spuštění (generation).
    fn update<R>(&self, id: &str, generation: u64, f: impl FnOnce(&mut Entry) -> R) -> Option<R> {
        let mut g = self.inner.lock();
        let e = g.entries.get_mut(id)?;
        if e.generation != generation {
            return None;
        }
        Some(f(e))
    }

    fn fail(&self, id: &str, generation: u64, msg: String) {
        let hit = self.update(id, generation, |e| {
            e.view.phase = Phase::Failed;
            e.view.message = Some(msg.clone());
            e.view.tail = fail_tail(e.log.iter(), TAIL_LINES);
            if let Some(j) = e.job.take() {
                j.kill();
            }
        });
        if hit.is_some() {
            (self.notify)(ProcEvent::Failed { id: id.to_string(), message: msg });
            (self.notify)(ProcEvent::Changed);
        }
    }

    fn spawn_shell(&self, id: &str, generation: u64, cwd: &Path, cmdline: &str, port: Option<u16>) -> Result<Child, String> {
        let comspec = std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into());
        let mut cmd = Command::new(comspec);
        cmd.raw_arg(format!("/d /s /c \"{cmdline}\""))
            .current_dir(cwd)
            .env("FORCE_COLOR", "0")
            .env("NO_COLOR", "1")
            .env("BROWSER", "none")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(p) = port {
            cmd.env("PORT", p.to_string());
        }
        let job = Job::new()?;
        let mut child = job.spawn(&mut cmd, CREATE_NO_WINDOW)?;
        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        self.update(id, generation, |e| {
            e.job = Some(job);
            e.view.pid = Some(pid);
            e.log.push_back(format!("› {cmdline}"));
        });
        if let Some(s) = stdout {
            self.pump(id, generation, s);
        }
        if let Some(s) = stderr {
            self.pump(id, generation, s);
        }
        Ok(child)
    }

    fn pump(&self, id: &str, generation: u64, stream: impl Read + Send + 'static) {
        let me = self.clone();
        let id = id.to_string();
        std::thread::Builder::new()
            .name(format!("log-{id}"))
            .spawn(move || {
                let mut reader = BufReader::new(stream);
                let mut buf = Vec::with_capacity(512);
                loop {
                    buf.clear();
                    match reader.read_until(b'\n', &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let text = String::from_utf8_lossy(&buf);
                    // progress bary přepisují řádek přes \r — bereme poslední stav
                    let raw = text.rsplit('\r').find(|s| !s.trim().is_empty()).unwrap_or("");
                    let line = clean_line(raw);
                    if line.is_empty() {
                        continue;
                    }
                    let alive = me
                        .update(&id, generation, |e| {
                            if e.view.url.is_none()
                                && let Some(p) = url_port(&line)
                            {
                                e.view.port = Some(p);
                            }
                            if looks_like_error(&line) {
                                e.view.errors += 1;
                            }
                            e.view.last_line = Some(line.clone());
                            e.log.push_back(line);
                            while e.log.len() > LOG_LINES {
                                e.log.pop_front();
                            }
                        })
                        .is_some();
                    if !alive {
                        break;
                    }
                    (me.notify)(ProcEvent::Changed);
                }
            })
            .ok();
    }

    pub fn launch_dev(&self, spec: DevSpec) -> Result<LaunchOutcome, String> {
        if !spec.cwd.is_dir() {
            return Err(trf!("Složka projektu neexistuje: {}", "The project folder doesn't exist: {}", spec.cwd.display()));
        }
        let generation = match self.begin(&spec.id, Phase::Starting) {
            Ok(Some(o)) => return Ok(o),
            Ok(None) => unreachable!(),
            Err(g) => g,
        };
        // port: preferovaný, pokud je volný; jinak (drží ho cokoli jiného) volný od OS
        let mut note = None;
        let port = match spec.port {
            Some(p) => match ports::owner(p) {
                None => p,
                Some(pid) => {
                    let q = ports::ephemeral().unwrap_or(0);
                    let who = match self.project_for_pid(pid) {
                        Some(proj) => trf!("projekt {proj}", "project {proj}"),
                        None => ports::describe(pid),
                    };
                    note = Some(trf!("Port {p} drží {who} → použit {q}", "Port {p} is held by {who} → using {q}"));
                    q
                }
            },
            None => ports::ephemeral().unwrap_or(0),
        };
        let cmdline = with_port(&spec.run, port, spec.pass_port);
        self.update(&spec.id, generation, |e| {
            e.view.cwd = Some(spec.cwd.to_string_lossy().into_owned());
            e.view.cmd = Some(cmdline.clone());
            e.view.port = Some(port);
            e.view.message = note.clone();
            if let Some(n) = &note {
                e.log.push_back(n.clone());
            }
        });
        (self.notify)(ProcEvent::Changed);

        let mut child = match self.spawn_shell(&spec.id, generation, &spec.cwd, &cmdline, Some(port)) {
            Ok(c) => c,
            Err(e) => {
                self.fail(&spec.id, generation, e.clone());
                return Err(e);
            }
        };

        let me = self.clone();
        let id = spec.id.clone();
        let open_browser = spec.open_browser;
        let timeout = spec.timeout;
        std::thread::Builder::new()
            .name(format!("watch-{id}"))
            .spawn(move || {
                let t0 = Instant::now();
                // 1) čekání na naběhnutí — zároveň hlídá konec procesu
                loop {
                    if let Ok(Some(st)) = child.try_wait() {
                        let code = st.code();
                        me.update(&id, generation, |e| e.view.exit_code = code);
                        let tail = me.log(&id).into_iter().rev().find(|l| !l.starts_with('›'));
                        let msg = match tail {
                            Some(t) => trf!("Server skončil dřív, než naběhl (kód {}): {t}", "The server exited before it was up (code {}): {t}", code.unwrap_or(-1)),
                            None => trf!("Server skončil dřív, než naběhl (kód {}).", "The server exited before it was up (code {}).", code.unwrap_or(-1)),
                        };
                        me.fail(&id, generation, msg);
                        return;
                    }
                    let Some(target) = me.update(&id, generation, |e| e.view.port).flatten() else { return };
                    if ports::probe(target) {
                        let url = format!("http://localhost:{target}/");
                        me.update(&id, generation, |e| {
                            e.view.phase = Phase::Ready;
                            e.view.url = Some(url.clone());
                            e.view.ready_at = Some(now_ms());
                        });
                        (me.notify)(ProcEvent::Ready { url, open_browser });
                        (me.notify)(ProcEvent::Changed);
                        break;
                    }
                    if t0.elapsed() > timeout {
                        me.fail(&id, generation, trf!("Server nenaběhl do {} s.", "The server wasn't up within {} s.", timeout.as_secs()));
                        let _ = child.wait();
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
                // 2) běží — blokující čekání na konec, nula CPU
                let code = child.wait().ok().and_then(|s| s.code());
                let msg = trf!("Server skončil (kód {}).", "The server exited (code {}).", code.unwrap_or(-1));
                let hit = me.update(&id, generation, |e| {
                    e.view.phase = Phase::Exited;
                    e.view.exit_code = code;
                    e.view.message = Some(msg.clone());
                    e.view.tail = fail_tail(e.log.iter(), TAIL_LINES);
                    e.job = None;
                });
                if hit.is_some() {
                    if code != Some(0) {
                        (me.notify)(ProcEvent::Failed { id: id.clone(), message: msg });
                    }
                    (me.notify)(ProcEvent::Changed);
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(LaunchOutcome::Started)
    }

    pub fn launch_app(&self, spec: AppSpec) -> Result<LaunchOutcome, String> {
        if spec.prepare.is_none() && !spec.exe.is_file() {
            return Err(trf!("Aplikace nenalezena: {}", "App not found: {}", spec.exe.display()));
        }
        let first = if spec.prepare.is_some() { Phase::Preparing } else { Phase::Running };
        let generation = match self.begin(&spec.id, first) {
            Ok(Some(o)) => return Ok(o),
            Ok(None) => unreachable!(),
            Err(g) => g,
        };
        self.update(&spec.id, generation, |e| {
            e.view.cwd = spec.cwd.as_ref().map(|d| d.to_string_lossy().into_owned());
            e.view.cmd = Some(match &spec.prepare {
                Some(prep) => format!("{prep} → {}", spec.exe.display()),
                None => spec.exe.to_string_lossy().into_owned(),
            });
        });
        (self.notify)(ProcEvent::Changed);
        let me = self.clone();
        let id = spec.id.clone();
        std::thread::Builder::new()
            .name(format!("app-{id}"))
            .spawn(move || {
                if let (Some(prep), Some(cwd)) = (&spec.prepare, &spec.cwd) {
                    let child = me.spawn_shell(&id, generation, cwd, prep, None);
                    let code = match child {
                        Ok(mut c) => c.wait().ok().and_then(|s| s.code()),
                        Err(e) => {
                            me.fail(&id, generation, e);
                            return;
                        }
                    };
                    if code != Some(0) {
                        me.fail(&id, generation, trf!("Příprava selhala (kód {}).", "Preparation failed (code {}).", code.unwrap_or(-1)));
                        return;
                    }
                    if !spec.exe.is_file() {
                        me.fail(&id, generation, trf!("Po přípravě chybí {}", "{} is missing after preparation", spec.exe.display()));
                        return;
                    }
                }
                let dir = spec.exe.parent().map(Path::to_path_buf).or(spec.cwd.clone());
                let mut cmd = Command::new(&spec.exe);
                cmd.args(&spec.args);
                if let Some(d) = dir {
                    cmd.current_dir(d);
                }
                // aplikace NEJDE do jobu: má přežít restart Ledge
                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        me.fail(&id, generation, trf!("Spuštění selhalo: {e}", "Launch failed: {e}"));
                        return;
                    }
                };
                let pid = child.id();
                let t0 = Instant::now();
                me.update(&id, generation, |e| {
                    e.view.phase = Phase::Running;
                    e.view.pid = Some(pid);
                    e.view.ready_at = Some(now_ms());
                    e.job = None;
                });
                (me.notify)(ProcEvent::Changed);
                let code = child.wait().ok().and_then(|s| s.code());
                if t0.elapsed() < Duration::from_secs(3) && code.is_some_and(|c| c != 0) {
                    me.fail(&id, generation, trf!("Aplikace hned skončila (kód {}).", "The app exited right away (code {}).", code.unwrap_or(-1)));
                    return;
                }
                // zavřená aplikace není chyba → zmizí z přehledu
                let mut g = me.inner.lock();
                if g.entries.get(&id).is_some_and(|e| e.generation == generation) {
                    g.entries.remove(&id);
                }
                drop(g);
                (me.notify)(ProcEvent::Changed);
            })
            .map_err(|e| e.to_string())?;
        Ok(LaunchOutcome::Started)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> (ProcMan, Arc<Mutex<Vec<ProcEvent>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev = events.clone();
        (ProcMan::new(Arc::new(move |e| ev.lock().push(e))), events)
    }

    fn wait_for(pm: &ProcMan, id: &str, want: Phase, secs: u64) -> ProcView {
        let t0 = Instant::now();
        loop {
            if let Some(v) = pm.snapshot().into_iter().find(|v| v.id == id)
                && v.phase == want
            {
                return v;
            }
            assert!(t0.elapsed() < Duration::from_secs(secs), "nedočkal jsem se {want:?}: {:?}", pm.snapshot());
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn node_server_script(dir: &Path) -> PathBuf {
        let p = dir.join("srv.js");
        std::fs::write(
            &p,
            r#"const http=require('http');const port=+process.env.PORT;
setTimeout(()=>http.createServer((q,r)=>r.end('ok')).listen(port,'127.0.0.1',()=>console.log(`\x1b[32m  ➜  Local:   http://localhost:${port}/\x1b[0m`)),300);"#,
        )
        .unwrap();
        p
    }

    #[test]
    fn port_flag_per_package_manager() {
        assert_eq!(with_port("npm run dev:staging", 5210, true), "npm run dev:staging -- --port 5210");
        assert_eq!(with_port("pnpm dev", 3000, true), "pnpm dev --port 3000");
        assert_eq!(with_port("node server.js", 3001, true), "node server.js");
        assert_eq!(with_port("npm run dev", 1, false), "npm run dev");
    }

    #[test]
    fn url_found_after_ansi_strip() {
        let l = clean_line("\x1b[32m  ➜  Local:   \x1b[1mhttp://localhost:\x1b[22m5210\x1b[0m/");
        assert_eq!(url_port(&l), Some(5210));
        assert_eq!(url_port("Network: http://192.168.1.5:5173/"), None);
    }

    #[test]
    fn dev_server_becomes_ready_and_stop_kills_tree() {
        let dir = tempfile::tempdir().unwrap();
        let srv = node_server_script(dir.path());
        let (pm, events) = quiet();
        let out = pm
            .launch_dev(DevSpec {
                id: "t".into(),
                cwd: dir.path().into(),
                run: format!("node \"{}\"", srv.display()),
                port: None,
                pass_port: true,
                open_browser: true,
                timeout: Duration::from_secs(20),
            })
            .unwrap();
        assert!(matches!(out, LaunchOutcome::Started));
        let v = wait_for(&pm, "t", Phase::Ready, 20);
        let port = v.port.unwrap();
        assert!(ports::probe(port));
        // fáze Ready se zapíše dřív, než watcher odešle událost → na ni se čeká, ne se jen kouká
        let t0 = Instant::now();
        while !events.lock().iter().any(|e| matches!(e, ProcEvent::Ready { .. })) {
            assert!(t0.elapsed() < Duration::from_secs(5), "událost Ready nepřišla");
            std::thread::sleep(Duration::from_millis(20));
        }
        // druhé spuštění téhož = už běží, nic nového
        assert!(matches!(
            pm.launch_dev(DevSpec {
                id: "t".into(),
                cwd: dir.path().into(),
                run: "node x".into(),
                port: None,
                pass_port: true,
                open_browser: false,
                timeout: Duration::from_secs(1),
            })
            .unwrap(),
            LaunchOutcome::AlreadyRunning { .. }
        ));
        // node je vnuk (cmd → node) — PID na portu musí patřit našemu jobu
        let pid = ports::owner(port).unwrap();
        assert_eq!(pm.project_for_pid(pid).as_deref(), Some("t"));
        pm.stop("t");
        let t0 = Instant::now();
        while ports::probe(port) && t0.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!ports::probe(port), "port pořád drží sirotek");
    }

    #[test]
    fn crash_during_start_is_reported_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let (pm, _) = quiet();
        let t0 = Instant::now();
        pm.launch_dev(DevSpec {
            id: "c".into(),
            cwd: dir.path().into(),
            run: "node -e \"console.error('Error: boom'); process.exit(3)\"".into(),
            port: None,
            pass_port: false,
            open_browser: false,
            timeout: Duration::from_secs(30),
        })
        .unwrap();
        let v = wait_for(&pm, "c", Phase::Failed, 10);
        assert!(t0.elapsed() < Duration::from_secs(10), "pád musí být hlášen hned, ne po timeoutu");
        assert_eq!(v.exit_code, Some(3));
        assert!(v.message.unwrap().contains("boom"));
        assert_eq!(v.errors, 1);
        // stav selhání na dlaždici: poslední řádky výstupu bez řádku Ledge, odkud a co se spouštělo
        assert_eq!(v.tail, ["Error: boom"]);
        assert_eq!(v.cwd.as_deref(), Some(dir.path().to_string_lossy().as_ref()));
        assert!(v.cmd.unwrap().starts_with("node -e"));
    }

    #[test]
    fn fail_tail_table() {
        let log = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<VecDeque<String>>();
        let cases: [(&[&str], usize, &[&str]); 5] = [
            (&[], 4, &[]),
            (&["› npm run dev"], 4, &[]),
            (&["› npm run dev", "a", "b"], 4, &["a", "b"]),
            (&["a", "b", "c", "d", "e", "f"], 4, &["c", "d", "e", "f"]),
            (&["a", "  ", "› x", "b"], 2, &["a", "b"]),
        ];
        for (input, n, want) in cases {
            assert_eq!(fail_tail(log(input).iter(), n), want, "{input:?}");
        }
    }

    #[test]
    fn busy_preferred_port_is_not_mistaken_for_our_server() {
        let blocker = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let busy = blocker.local_addr().unwrap().port();
        let dir = tempfile::tempdir().unwrap();
        let srv = node_server_script(dir.path());
        let (pm, _) = quiet();
        pm.launch_dev(DevSpec {
            id: "b".into(),
            cwd: dir.path().into(),
            run: format!("node \"{}\"", srv.display()),
            port: Some(busy),
            pass_port: false,
            open_browser: false,
            timeout: Duration::from_secs(20),
        })
        .unwrap();
        let v = wait_for(&pm, "b", Phase::Ready, 20);
        assert_ne!(v.port, Some(busy));
        assert!(v.message.unwrap().contains(&format!("Port {busy} drží")));
        pm.stop("b");
    }

    #[test]
    fn missing_dir_and_missing_exe_are_errors_not_panics() {
        let (pm, _) = quiet();
        assert!(pm
            .launch_dev(DevSpec {
                id: "x".into(),
                cwd: "C:/neexistuje/nikde".into(),
                run: "npm run dev".into(),
                port: None,
                pass_port: true,
                open_browser: false,
                timeout: Duration::from_secs(1),
            })
            .is_err());
        assert!(pm
            .launch_app(AppSpec {
                id: "y".into(),
                cwd: None,
                prepare: None,
                exe: "C:/neexistuje/app.exe".into(),
                args: vec![],
            })
            .is_err());
        assert!(pm.snapshot().is_empty());
    }

    #[test]
    fn timeout_kills_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        let (pm, events) = quiet();
        pm.launch_dev(DevSpec {
            id: "s".into(),
            cwd: dir.path().into(),
            run: "node -e \"setInterval(()=>{},1000)\"".into(),
            port: None,
            pass_port: false,
            open_browser: false,
            timeout: Duration::from_secs(2),
        })
        .unwrap();
        let v = wait_for(&pm, "s", Phase::Failed, 10);
        assert!(v.message.unwrap().contains("nenaběhl"));
        // událost nese id a zprávu (toast), ne jen „něco selhalo“
        let t0 = Instant::now();
        while !events.lock().iter().any(|e| matches!(e, ProcEvent::Failed { id, message } if id == "s" && message.contains("nenaběhl"))) {
            assert!(t0.elapsed() < Duration::from_secs(5), "událost Failed nepřišla");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
