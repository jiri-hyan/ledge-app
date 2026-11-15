//! Supervisor rozšíření v hlavním procesu: spouští `ledge.exe --ext <id>` v Job Objectu (`procman::Job`),
//! vede handshake `hello` / `init` / `ready`, posílá pauzu a data, hlídá termíny (`policy::Life`) a pády
//! (`policy::after_exit`). Rozhodování je v `policy.rs` (čistě); tady jsou jen vlákna a roury.
//!
//! Vlákna existují jen, dokud rozšíření běží nebo čeká na restart: na slot jeden „supervise“ (jediná smyčka
//! `recv_timeout(nejbližší termín)` — žádný periodický tik) a na běh čtečka stdout, čtečka stderr, zapisovač
//! stdin a čekač na konec procesu. Bez zapnutého rozšíření neběží nic.

use super::policy::{
    self, Act, After, Coalescer, Due, ExitKind, Facts, Life, Manifest, Offer, Phase, Req, Start, SyncAct, Timing,
};
use super::wire::{self, Decoded, Down, Up, WireError};
use crate::procman::{CREATE_NO_WINDOW, Job};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::io::{BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Co supervisor potřebuje zvenku (v aplikaci čte `Core`, v testech je to maketa).
pub trait Env: Send + Sync + 'static {
    /// nastavení rozšíření (`%LOCALAPPDATA%\Ledge\ext\<id>.json`), `{}` když není
    fn cfg_for(&self, id: &str) -> Value;
    fn lang(&self) -> String;
    /// fullscreen hra / herní režim
    fn dead(&self) -> bool;
    /// aktuální data tématu (jen paměť); `None` = téma neznáme
    fn topic_data(&self, topic: &str) -> Option<Value>;
    /// povolený požadavek z rozšíření (už prošel whitelistem)
    fn handle_req(&self, id: &str, req: Req);
    fn log(&self, msg: &str);
}

/// Jak se vyrobí příkaz pro rozšíření (v aplikaci `current_exe --ext <id>`).
pub type Launcher = Arc<dyn Fn(&str) -> Command + Send + Sync>;

/// Co o rozšíření ukazuje Nastavení.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtStatus {
    pub id: String,
    pub enabled: bool,
    pub phase: Phase,
    pub on_demand: bool,
    pub pid: Option<u32>,
    /// poslední chyba (`err` od rozšíření, kód konce, nekompatibilní verze)
    pub error: Option<String>,
    /// posledních pár řádků stderr po pádu
    pub tail: Vec<String>,
    /// proč je v pauze (`game`)
    pub pause_why: Option<String>,
}

enum Ev {
    Up(Up),
    /// nesmysl na stdout (log knihovny, hluk) — zahodit
    Garbage,
    Fatal(WireError),
    Exit(Option<i32>),
}

enum Cmd {
    /// slušně ukončit (`quit`) a skončit
    Stop,
    Pause(Option<&'static str>),
    /// téma se změnilo — natáhnout čerstvá data
    Topic(String),
    /// libovolná zpráva dolů (`cmd`)
    #[allow(dead_code)] // posílá `send_cmd` (zatím jen testy, použije E1 / E5)
    Down(Down),
}

enum Msg {
    /// událost z instance (číslo instance chrání před zpožděnými zprávami starého běhu)
    Ev(u64, Ev),
    Cmd(Cmd),
}

#[derive(Default)]
struct Slot {
    phase: Option<Phase>,
    gn: u64,
    tx: Option<Sender<Msg>>,
    job: Option<Arc<Job>>,
    pid: Option<u32>,
    error: Option<String>,
    tail: Vec<String>,
    topics: Vec<String>,
    crashes: Vec<Instant>,
    pause_why: Option<&'static str>,
}

impl Slot {
    fn phase(&self) -> Phase {
        self.phase.unwrap_or(Phase::Idle)
    }
}

pub struct Sup {
    env: Arc<dyn Env>,
    launcher: Launcher,
    manifests: Vec<Manifest>,
    timing: Timing,
    slots: Mutex<HashMap<String, Slot>>,
    enabled: Mutex<Vec<String>>,
}

struct Outcome {
    kind: ExitKind,
    ready_at: Option<Instant>,
    incompat: bool,
    stopped: bool,
    error: Option<String>,
    tail: Vec<String>,
}

const TAIL_KEEP: usize = 40;

impl Sup {
    pub fn new(env: Arc<dyn Env>, launcher: Launcher, manifests: Vec<Manifest>, timing: Timing) -> Arc<Self> {
        Arc::new(Sup { env, launcher, manifests, timing, slots: Mutex::new(HashMap::new()), enabled: Mutex::new(Vec::new()) })
    }

    fn manifest(&self, id: &str) -> Option<Manifest> {
        self.manifests.iter().find(|m| m.id == id).copied()
    }

    fn update<R>(&self, id: &str, gn: u64, f: impl FnOnce(&mut Slot) -> R) -> Option<R> {
        let mut g = self.slots.lock();
        let s = g.get_mut(id).filter(|s| s.gn == gn)?;
        Some(f(s))
    }

    fn pause_now(&self) -> Option<&'static str> {
        policy::pause_reason(Facts { dead: self.env.dead() })
    }

    // ---------- veřejné API ----------

    /// Srovná běh s configem: spustí povolená `Always` rozšíření, zastaví odebraná (`policy::plan_sync`).
    pub fn sync(self: &Arc<Self>, enabled: &[String]) {
        *self.enabled.lock() = enabled.to_vec();
        let slots: Vec<(String, Phase)> = self.slots.lock().iter().map(|(k, s)| (k.clone(), s.phase())).collect();
        for act in policy::plan_sync(enabled, &self.manifests, &slots, self.env.dead()) {
            match act {
                SyncAct::Start(id) => {
                    let _ = self.start(&id);
                }
                SyncAct::Stop(id) => self.stop(&id),
            }
        }
    }

    /// Spustí rozšíření (nic, když už běží). Smazá předchozí selhání.
    pub fn start(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let man = self.manifest(id).ok_or_else(|| format!("neznámé rozšíření {id}"))?;
        let (tx, rx) = channel();
        let gn = {
            let mut g = self.slots.lock();
            let s = g.entry(id.to_string()).or_default();
            if s.phase().is_active() {
                return Ok(());
            }
            s.gn += 1;
            *s = Slot { gn: s.gn, phase: Some(Phase::Starting), tx: Some(tx.clone()), ..Slot::default() };
            s.gn
        };
        let sup = self.clone();
        let id = id.to_string();
        std::thread::Builder::new()
            .name(format!("ext-{id}"))
            .spawn(move || sup.supervise(&id, gn, man, tx, rx))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Slušně ukončí (`quit`, po lhůtě kill) a zapomene slot, i selhaný.
    pub fn stop(&self, id: &str) {
        let mut g = self.slots.lock();
        if let Some(s) = g.get_mut(id) {
            match &s.tx {
                Some(tx) if s.phase().is_active() => {
                    let _ = tx.send(Msg::Cmd(Cmd::Stop));
                }
                _ => {
                    g.remove(id);
                }
            }
        }
    }

    /// Konec Ledge: všem aktivním pošle `quit` (config zůstane, jen běh) a počká nejvýš `wait`, než odejdou
    /// samy (profil WebView2 se zavře řádně); co nestihne, zabije až Job při zavření handlu (L238).
    pub fn shutdown(&self, wait: Duration) {
        let active: Vec<String> = self.slots.lock().iter().filter(|(_, s)| s.phase().is_active()).map(|(k, _)| k.clone()).collect();
        if active.is_empty() {
            return;
        }
        // bez toho by `Always` rozšíření po `quit` hned znovu naběhlo (Ledge končí, nic se nespouští)
        self.enabled.lock().clear();
        for id in &active {
            self.stop(id);
        }
        let end = Instant::now() + wait;
        while Instant::now() < end && self.slots.lock().values().any(|s| s.phase().is_active()) {
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// `OnDemand` probuzení (klik, událost). Ve hře odmítne, jako `ctl open`.
    pub fn wake(self: &Arc<Self>, id: &str, reason: &str) -> Result<(), String> {
        if !self.enabled.lock().iter().any(|e| e == id) {
            return Err(format!("rozšíření {id} není zapnuté"));
        }
        if self.env.dead() {
            return Err("ve hře se rozšíření nespouští".into());
        }
        self.env.log(&format!("ext {id}: wake ({reason})"));
        if matches!(self.slots.lock().get(id).map(Slot::phase), Some(Phase::Failed | Phase::Incompatible)) {
            self.slots.lock().remove(id);
        }
        self.start(id)
    }

    /// Hra / herní režim začal nebo skončil (`hover.rs`, `gamemode::set`): pauza běžících, po hře znovu spustit
    /// `Always` rozšíření, která mezitím skončila (`pause_quit_min`).
    pub fn on_dead_changed(self: &Arc<Self>) {
        let why = self.pause_now();
        for s in self.slots.lock().values() {
            if let (Some(tx), true) = (&s.tx, s.phase().is_active()) {
                let _ = tx.send(Msg::Cmd(Cmd::Pause(why)));
            }
        }
        if why.is_none() {
            let enabled = self.enabled.lock().clone();
            self.sync(&enabled);
        }
    }

    /// Téma se změnilo (např. `media`): rozšíření, která ho odebírají, si natáhnou data. Bez odběratele nic.
    pub fn notify(&self, topic: &str) {
        for s in self.slots.lock().values() {
            if let (Some(tx), true) = (&s.tx, s.topics.iter().any(|t| t == topic)) {
                let _ = tx.send(Msg::Cmd(Cmd::Topic(topic.to_string())));
            }
        }
    }

    /// Pošle rozšíření `cmd` (obal alba, změna pozadí tapety…). `false` = neběží.
    pub fn send_cmd(&self, id: &str, name: &str, args: Value) -> bool {
        let g = self.slots.lock();
        let Some(s) = g.get(id).filter(|s| s.phase().is_active()) else { return false };
        s.tx.as_ref().is_some_and(|tx| tx.send(Msg::Cmd(Cmd::Down(Down::Cmd { name: name.into(), args }))).is_ok())
    }

    pub fn status(&self) -> Vec<ExtStatus> {
        let enabled = self.enabled.lock().clone();
        let g = self.slots.lock();
        self.manifests
            .iter()
            .map(|m| {
                let s = g.get(m.id);
                ExtStatus {
                    id: m.id.to_string(),
                    enabled: enabled.iter().any(|e| e == m.id),
                    phase: s.map_or(Phase::Idle, Slot::phase),
                    on_demand: m.start == Start::OnDemand,
                    pid: s.and_then(|s| s.pid),
                    error: s.and_then(|s| s.error.clone()),
                    tail: s.map(|s| s.tail.clone()).unwrap_or_default(),
                    pause_why: s.and_then(|s| s.pause_why).map(str::to_string),
                }
            })
            .collect()
    }

    /// Job běžícího rozšíření (pro měření `QueryInformationJobObject`), `None` když neběží.
    pub fn job(&self, id: &str) -> Option<Arc<Job>> {
        self.slots.lock().get(id).and_then(|s| s.job.clone())
    }

    // ---------- vlákno slotu ----------

    fn supervise(self: Arc<Self>, id: &str, gn: u64, man: Manifest, tx: Sender<Msg>, rx: Receiver<Msg>) {
        let mut pause = self.pause_now();
        let mut inst: u64 = 0;
        let mut stopped = false;
        loop {
            inst += 1;
            let out = self.run_once(id, gn, &man, inst, &tx, &rx, &mut pause);
            stopped |= out.stopped;
            let now = Instant::now();
            let crashes = self
                .update(id, gn, |s| {
                    s.job = None;
                    s.pid = None;
                    s.topics.clear();
                    s.pause_why = None;
                    if out.kind == ExitKind::Crash && !out.incompat && !out.stopped {
                        policy::record_crash(&mut s.crashes, out.ready_at, now)
                    } else {
                        s.crashes.len().min(u8::MAX as usize) as u8
                    }
                })
                .unwrap_or(0);
            if (out.kind == ExitKind::Crash || out.incompat) && !out.stopped {
                self.update(id, gn, |s| {
                    s.error = out.error.clone().or(s.error.take());
                    s.tail = out.tail.clone();
                });
            }
            let after = if out.incompat || out.stopped { After::Idle } else { policy::after_exit(man.start, out.kind, crashes) };
            match after {
                _ if out.incompat => {
                    self.update(id, gn, |s| s.phase = Some(Phase::Incompatible));
                    return;
                }
                After::Failed => {
                    self.env.log(&format!("ext {id}: selhalo ({crashes} pádů)"));
                    self.update(id, gn, |s| s.phase = Some(Phase::Failed));
                    return;
                }
                After::Idle => {
                    self.update(id, gn, |s| s.phase = Some(Phase::Idle));
                    // Stop těsně před opětovným zapnutím v configu: dožít a spustit znovu
                    if stopped && self.enabled.lock().iter().any(|e| e == id) && man.start == Start::Always && self.pause_now().is_none() {
                        self.slots.lock().remove(id);
                        let _ = self.start(id);
                    } else if stopped {
                        let mut g = self.slots.lock();
                        if g.get(id).is_some_and(|s| s.gn == gn) && !self.enabled.lock().iter().any(|e| e == id) {
                            g.remove(id);
                        }
                    }
                    return;
                }
                After::Restart(d) => {
                    self.env.log(&format!("ext {id}: restart za {} s", d.as_secs()));
                    self.update(id, gn, |s| s.phase = Some(Phase::Backoff));
                    let until = Instant::now() + d;
                    loop {
                        match rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
                            Ok(Msg::Cmd(Cmd::Stop)) => {
                                self.update(id, gn, |s| s.phase = Some(Phase::Idle));
                                self.slots.lock().remove(id);
                                return;
                            }
                            Ok(Msg::Cmd(Cmd::Pause(w))) => pause = w,
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                    if self.pause_now().is_some() {
                        // hra: spustí se po jejím konci (`on_dead_changed` → `sync`)
                        self.update(id, gn, |s| s.phase = Some(Phase::Idle));
                        return;
                    }
                }
            }
        }
    }

    /// Jeden běh procesu od spuštění do konce.
    #[allow(clippy::too_many_arguments)]
    fn run_once(&self, id: &str, gn: u64, man: &Manifest, inst: u64, tx: &Sender<Msg>, rx: &Receiver<Msg>, pause: &mut Option<&'static str>) -> Outcome {
        let mut out = Outcome { kind: ExitKind::Crash, ready_at: None, incompat: false, stopped: false, error: None, tail: Vec::new() };
        let job = match Job::new() {
            Ok(j) => Arc::new(j),
            Err(e) => {
                out.error = Some(e);
                return out;
            }
        };
        let mut cmd = (self.launcher)(id);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = match job.spawn(&mut cmd, CREATE_NO_WINDOW) {
            Ok(c) => c,
            Err(e) => {
                out.error = Some(e);
                return out;
            }
        };
        let pid = child.id();
        let (stdin, stdout, stderr) = (child.stdin.take(), child.stdout.take(), child.stderr.take());
        self.update(id, gn, |s| {
            s.phase = Some(Phase::Starting);
            s.job = Some(job.clone());
            s.pid = Some(pid);
            s.error = None;
        });

        // čtečka stdout → události
        if let Some(stdout) = stdout {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(stdout);
                let mut buf = Vec::new();
                loop {
                    let ev = match wire::read_line(&mut r, &mut buf) {
                        Ok(Some(Ok(line))) => match wire::decode_up(&line) {
                            Ok(Decoded::Msg(m)) => Ev::Up(m),
                            Ok(Decoded::Unknown(_)) => Ev::Garbage,
                            Err(e @ WireError::Version(_)) => Ev::Fatal(e),
                            Err(_) => Ev::Garbage,
                        },
                        Ok(Some(Err(e))) => Ev::Fatal(e),
                        Ok(None) | Err(_) => return,
                    };
                    let fatal = matches!(ev, Ev::Fatal(_));
                    if tx.send(Msg::Ev(inst, ev)).is_err() || fatal {
                        return;
                    }
                }
            });
        }
        // čtečka stderr → ocas pro „selhalo“
        let tail: Arc<Mutex<VecDeque<String>>> = Arc::default();
        if let Some(stderr) = stderr {
            let tail = tail.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(stderr);
                let mut buf = Vec::new();
                while crate::procman::read_line_capped(&mut r, &mut buf, 4096).is_ok_and(|n| n > 0) {
                    let line = crate::procman::clean_line(&String::from_utf8_lossy(&buf));
                    buf.clear();
                    if !line.is_empty() {
                        let mut t = tail.lock();
                        if t.len() >= TAIL_KEEP {
                            t.pop_front();
                        }
                        t.push_back(line);
                    }
                }
            });
        }
        // zapisovač stdin (zablokovaný / mrtvý ext nesmí zastavit smyčku termínů)
        let (wtx, wrx) = channel::<String>();
        if let Some(mut stdin) = stdin {
            std::thread::spawn(move || {
                while let Ok(line) = wrx.recv() {
                    if stdin.write_all(line.as_bytes()).and_then(|()| stdin.flush()).is_err() {
                        return;
                    }
                }
            });
        }
        // čekač na konec procesu (EOF stdout nestačí — WebView2 dítě může držet roury)
        {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let code = child.wait().ok().and_then(|s| s.code());
                let _ = tx.send(Msg::Ev(inst, Ev::Exit(code)));
            });
        }

        let send = |d: &Down| {
            let _ = wtx.send(wire::encode_down(d));
        };
        let mut life = Life::with_timing(Instant::now(), self.timing);
        let mut coal = Coalescer::default();
        let mut pending: HashMap<String, Value> = HashMap::new();
        let mut revs: HashMap<String, u64> = HashMap::new();
        let mut topics: Vec<String> = Vec::new();
        let (mut said_bye, mut asked_quit, mut killed_late) = (false, false, false);
        let mut ext_err: Option<String> = None;

        let offer = |t: &str, coal: &mut Coalescer, pending: &mut HashMap<String, Value>, revs: &mut HashMap<String, u64>, now: Instant| {
            let Some(v) = self.env.topic_data(t) else { return };
            let fp = crate::desk::live::fnv(&v.to_string());
            match coal.offer(t, fp, now) {
                Offer::Send => {
                    let r = revs.entry(t.to_string()).or_default();
                    *r += 1;
                    send(&Down::Data { topic: t.to_string(), rev: *r, d: v });
                }
                Offer::Same => {}
                Offer::Defer(_) => {
                    pending.insert(t.to_string(), v);
                }
            }
        };

        let exit_code = loop {
            let now = Instant::now();
            let due = life.next_due(man);
            let wake = [due.map(|d| d.0), coal.next_due()].into_iter().flatten().min();
            let msg = match wake {
                Some(t) => rx.recv_timeout(t.saturating_duration_since(now)),
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            let now = Instant::now();
            match msg {
                Err(RecvTimeoutError::Disconnected) => {
                    job.kill();
                    break None;
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Some((t, d)) = due.filter(|(t, _)| now >= *t) {
                        let _ = t;
                        self.env.log(&format!("ext {id}: termín {d:?}"));
                        match policy::act_for(d) {
                            Act::Quit => {
                                send(&Down::Quit);
                                life.quitting(now);
                                asked_quit = true;
                            }
                            Act::Kill => {
                                killed_late = true;
                                if matches!(d, Due::HelloLate | Due::ReadyLate) {
                                    ext_err = Some(format!("rozšíření nenaběhlo včas ({d:?})"));
                                }
                                job.kill();
                            }
                        }
                    }
                    for t in coal.take_due(now) {
                        if let (Some(v), None) = (pending.remove(&t), *pause) {
                            let r = revs.entry(t.clone()).or_default();
                            *r += 1;
                            send(&Down::Data { topic: t, rev: *r, d: v });
                        }
                    }
                }
                Ok(Msg::Ev(i, _)) if i != inst => {}
                Ok(Msg::Ev(_, Ev::Exit(code))) => break code,
                Ok(Msg::Ev(_, Ev::Garbage)) => {}
                Ok(Msg::Ev(_, Ev::Fatal(e))) => {
                    self.env.log(&format!("ext {id}: {e}"));
                    if matches!(e, WireError::Version(_)) {
                        out.incompat = true;
                        out.error = Some("nekompatibilní verze rozšíření (aktualizuj Ledge)".into());
                        send(&Down::Quit);
                        life.quitting(now);
                        asked_quit = true;
                    } else {
                        ext_err = Some(e.to_string());
                        job.kill();
                    }
                }
                Ok(Msg::Ev(_, Ev::Up(m))) => match m {
                    Up::Hello { id: hid, wants, .. } => {
                        if life.stage != policy::Stage::Spawned {
                            continue;
                        }
                        if hid != id {
                            ext_err = Some(format!("hello s cizím id {hid}"));
                            job.kill();
                            continue;
                        }
                        topics = policy::granted_topics(man, &wants);
                        let t = topics.clone();
                        self.update(id, gn, |s| s.topics = t);
                        life.greeted(now);
                        send(&Down::Init { cfg: self.env.cfg_for(id), lang: self.env.lang(), dead: pause.is_some(), rev: json!({}) });
                        if let Some(why) = *pause {
                            send(&Down::State { pause: true, why: Some(why.to_string()) });
                            life.set_pause(true, now);
                        }
                    }
                    Up::Ready => {
                        if life.stage == policy::Stage::Greeted {
                            life.ready(now);
                            out.ready_at = Some(now);
                            let paused = *pause;
                            self.update(id, gn, |s| {
                                s.phase = Some(if paused.is_some() { Phase::Paused } else { Phase::Running });
                                s.pause_why = paused;
                            });
                            if pause.is_none() {
                                for t in &topics {
                                    offer(t, &mut coal, &mut pending, &mut revs, now);
                                }
                            }
                        }
                    }
                    Up::Active => life.active(now),
                    Up::Bye => said_bye = true,
                    Up::Err { msg } => {
                        let msg: String = msg.chars().take(500).collect();
                        self.update(id, gn, |s| s.error = Some(msg.clone()));
                        ext_err = Some(msg);
                    }
                    Up::Req { what, act, key } => match policy::parse_req(&what, act.as_deref(), key.as_deref()) {
                        Some(r) => self.env.handle_req(id, r),
                        None => self.env.log(&format!("ext {id}: požadavek '{what}' zahozen (mimo whitelist)")),
                    },
                },
                Ok(Msg::Cmd(c)) => match c {
                    Cmd::Stop => {
                        out.stopped = true;
                        if life.stage != policy::Stage::Quitting {
                            send(&Down::Quit);
                            life.quitting(now);
                            asked_quit = true;
                        }
                    }
                    Cmd::Pause(why) => {
                        if *pause == why {
                            continue;
                        }
                        *pause = why;
                        // před `init` si pauzu přečte `init.dead` a hned po něm `state`
                        if matches!(life.stage, policy::Stage::Ready | policy::Stage::Greeted) {
                            life.set_pause(why.is_some(), now);
                            send(&Down::State { pause: why.is_some(), why: why.map(str::to_string) });
                            coal.reset();
                            pending.clear();
                            if life.stage == policy::Stage::Ready {
                                self.update(id, gn, |s| {
                                    s.phase = Some(if why.is_some() { Phase::Paused } else { Phase::Running });
                                    s.pause_why = why;
                                });
                                if why.is_none() {
                                    // po pauze jeden čerstvý snapshot všech témat
                                    for t in &topics {
                                        offer(t, &mut coal, &mut pending, &mut revs, now);
                                    }
                                }
                            }
                        }
                    }
                    Cmd::Topic(t) => {
                        if life.stage == policy::Stage::Ready && pause.is_none() && topics.contains(&t) {
                            offer(&t, &mut coal, &mut pending, &mut revs, now);
                        }
                    }
                    Cmd::Down(d) => {
                        if matches!(life.stage, policy::Stage::Ready) {
                            send(&d);
                        }
                    }
                },
            }
        };
        job.kill(); // zbytek stromu (WebView2) a uvolnění rour
        let tail_lines = crate::procman::fail_tail(tail.lock().iter(), crate::procman::TAIL_LINES);
        out.kind = policy::exit_kind(exit_code == Some(0), said_bye, asked_quit, killed_late);
        out.tail = tail_lines;
        if out.kind == ExitKind::Crash && out.error.is_none() {
            out.error = ext_err.or_else(|| Some(match exit_code {
                Some(c) => trf!("Rozšíření skončilo s kódem {c}.", "The extension exited with code {c}."),
                None => crate::i18n::tr("Rozšíření skončilo neočekávaně.", "The extension ended unexpectedly.").to_string(),
            }));
        }
        out
    }
}
