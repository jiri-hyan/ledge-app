//! Integrační testy supervisoru: dítě je tenhle testovací binár spuštěný znovu a ukazující na `child_entry`
//! (bez GUI, bez Tauri; `echo` z `echo.rs`). Vše v dočasné složce — žádné `%APPDATA%`, registr ani `~/.claude`.

use super::child::run_with;
use super::echo::Echo;
use super::policy::{Manifest, MediaAct, Phase, Req, Start, Timing};
use super::sup::{Env, Sup};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Vstup dítěte: bez proměnné `LEDGE_EXT_TEST_ID` je to prázdný test; s ní běží `echo` nad stdin/stdout.
#[test]
fn child_entry() {
    let Ok(id) = std::env::var("LEDGE_EXT_TEST_ID") else { return };
    // libtest nechá na stdout rozepsaný řádek „test … ... “ — protokol musí začít na novém řádku
    {
        use std::io::Write;
        println!();
        let _ = std::io::stdout().flush();
    }
    if std::env::var("LEDGE_EXT_TEST_RAW").as_deref() == Ok("badver") {
        use std::io::Write;
        println!(r#"{{"v":2,"t":"hello","id":"{id}","pid":1}}"#);
        let _ = std::io::stdout().flush();
        std::thread::sleep(Duration::from_secs(30));
        std::process::exit(0);
    }
    let rc = run_with(&id, Box::new(Echo::new()), std::io::stdin(), std::io::stdout());
    std::process::exit(rc);
}

const fn man(id: &'static str, start: Start) -> Manifest {
    Manifest { id, start, idle_quit_min: None, pause_quit_min: None, wants: &["media"] }
}

struct TestEnv {
    dir: PathBuf,
    dead: AtomicBool,
    topics: Mutex<HashMap<String, Value>>,
    reqs: Mutex<Vec<Req>>,
    modes: HashMap<&'static str, &'static str>,
}

impl Env for TestEnv {
    fn cfg_for(&self, id: &str) -> Value {
        let mode = self.modes.get(id).copied().unwrap_or("ok");
        json!({ "log": self.dir.join(format!("{id}.log")), "mode": mode })
    }
    fn lang(&self) -> String {
        "cs".into()
    }
    fn dead(&self) -> bool {
        self.dead.load(Ordering::SeqCst)
    }
    fn topic_data(&self, topic: &str) -> Option<Value> {
        self.topics.lock().get(topic).cloned()
    }
    fn handle_req(&self, _id: &str, req: Req) {
        self.reqs.lock().push(req);
    }
    fn log(&self, _msg: &str) {}
}

struct Rig {
    env: Arc<TestEnv>,
    sup: Arc<Sup>,
    _dir: tempfile::TempDir,
}

fn rig(manifests: Vec<Manifest>, modes: &[(&'static str, &'static str)], timing: Timing) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let env = Arc::new(TestEnv {
        dir: dir.path().to_path_buf(),
        dead: AtomicBool::new(false),
        topics: Mutex::new(HashMap::from([("media".to_string(), json!({"n": 1}))])),
        reqs: Mutex::new(Vec::new()),
        modes: modes.iter().copied().collect(),
    });
    let launcher: super::sup::Launcher = Arc::new(|id: &str| {
        let mut c = Command::new(std::env::current_exe().unwrap());
        c.args(["ext::tests::child_entry", "--exact", "--nocapture", "--test-threads=1"]).env("LEDGE_EXT_TEST_ID", id);
        if id.ends_with("badver") {
            c.env("LEDGE_EXT_TEST_RAW", "badver");
        }
        c
    });
    let sup = Sup::new(env.clone(), launcher, manifests, timing);
    Rig { env, sup, _dir: dir }
}

impl Rig {
    fn phase(&self, id: &str) -> Phase {
        self.sup.status().into_iter().find(|s| s.id == id).unwrap().phase
    }
    fn wait_phase(&self, id: &str, want: Phase) {
        wait(&format!("fáze {id} → {want:?} (je {:?})", self.phase(id)), || self.phase(id) == want);
    }
    fn log(&self, id: &str) -> String {
        std::fs::read_to_string(self.env.dir.join(format!("{id}.log"))).unwrap_or_default()
    }
    fn wait_log(&self, id: &str, needle: &str) {
        wait(&format!("{id}.log obsahuje {needle}"), || self.log(id).contains(needle));
    }
}

fn wait(what: &str, mut cond: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("čekání vypršelo: {what}");
}

fn ids(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

/// Konec Ledge: `shutdown` pošle `quit` a počká, až rozšíření odejde samo (ne tvrdé zabití).
#[test]
fn shutdown_quits_gracefully_and_waits() {
    let r = rig(vec![man("echo-a", Start::Always)], &[], Timing::default());
    r.sup.shutdown(Duration::from_millis(50)); // nic neběží = hned zpět
    r.sup.sync(&ids(&["echo-a"]));
    r.wait_phase("echo-a", Phase::Running);
    r.sup.shutdown(Duration::from_secs(15));
    assert_eq!(r.phase("echo-a"), Phase::Idle, "shutdown se vrátil, až rozšíření odešlo");
    assert!(r.log("echo-a").contains(r#""t":"quit""#), "odešlo řádně přes quit");
}

#[test]
fn lifecycle_handshake_pause_data_req_stop() {
    let r = rig(vec![man("echo-a", Start::Always)], &[], Timing::default());
    r.sup.sync(&ids(&["echo-a"]));
    r.wait_phase("echo-a", Phase::Running);
    // `init` dorazil a po `ready` přišel snapshot tématu `media` (rev 1)
    assert!(r.log("echo-a").contains("init dead=false lang=cs"));
    r.wait_log("echo-a", r#""t":"data""#);
    assert!(r.log("echo-a").contains(r#""rev":1"#) && r.log("echo-a").contains(r#""n":1"#));
    assert!(r.sup.status()[0].pid.is_some() && r.sup.job("echo-a").is_some());

    // hra → `state{pause:true,why:game}`, stav Paused
    r.env.dead.store(true, Ordering::SeqCst);
    r.sup.on_dead_changed();
    r.wait_phase("echo-a", Phase::Paused);
    r.wait_log("echo-a", r#""why":"game""#);
    assert_eq!(r.sup.status()[0].pause_why.as_deref(), Some("game"));
    // ve hře data nejdou, po konci hry jde čerstvý snapshot
    r.env.topics.lock().insert("media".into(), json!({"n": 2}));
    r.sup.notify("media");
    r.env.dead.store(false, Ordering::SeqCst);
    r.sup.on_dead_changed();
    r.wait_phase("echo-a", Phase::Running);
    r.wait_log("echo-a", r#""n":2"#);
    assert!(r.log("echo-a").contains(r#""pause":false"#));

    // rychlé změny se koalescují: poslední hodnota vyhrává, mezistav se nepošle
    r.env.topics.lock().insert("media".into(), json!({"n": 3}));
    r.sup.notify("media");
    r.env.topics.lock().insert("media".into(), json!({"n": 4}));
    r.sup.notify("media");
    r.wait_log("echo-a", r#""n":4"#);
    assert!(!r.log("echo-a").contains(r#""n":3"#), "mezistav n=3 se neměl poslat");

    // požadavky: mimo whitelist se zahodí, whitelist projde do Env
    assert!(r.sup.send_cmd("echo-a", "req", json!({"what": "decide", "do": "allow"})));
    assert!(r.sup.send_cmd("echo-a", "req", json!({"what": "media", "do": "next"})));
    wait("req dorazil", || !r.env.reqs.lock().is_empty());
    assert_eq!(*r.env.reqs.lock(), vec![Req::Media(MediaAct::Next)]);

    // vypnutí v configu = `quit`, slot zmizí
    r.sup.sync(&[]);
    r.wait_phase("echo-a", Phase::Idle);
    r.wait_log("echo-a", r#""t":"quit""#);
    wait("pid zapomenut", || r.sup.status()[0].pid.is_none() && r.sup.job("echo-a").is_none());
    assert!(!r.sup.status()[0].enabled);
}

#[test]
fn crash_goes_to_backoff_and_stop_cancels_it() {
    let r = rig(vec![man("echo-crash", Start::Always)], &[("echo-crash", "crash")], Timing::default());
    r.sup.sync(&ids(&["echo-crash"]));
    r.wait_phase("echo-crash", Phase::Backoff);
    let st = r.sup.status().remove(0);
    assert!(st.error.as_deref().is_some_and(|e| e.contains('3')), "chyba má nést kód konce: {:?}", st.error);
    // odebrání z configu ruší čekání na restart
    r.sup.sync(&[]);
    r.wait_phase("echo-crash", Phase::Idle);
}

#[test]
fn on_demand_hang_is_killed_and_not_restarted() {
    let t = Timing { hello: Duration::from_millis(2000), ready: Duration::from_millis(300), grace: Duration::from_millis(300) };
    let r = rig(vec![man("echo-hang", Start::OnDemand)], &[("echo-hang", "no_ready")], t);
    // OnDemand se samo nespustí
    r.sup.sync(&ids(&["echo-hang"]));
    assert_eq!(r.phase("echo-hang"), Phase::Idle);
    r.sup.wake("echo-hang", "test").unwrap();
    // `init` přišel, `ready` ne → po lhůtě kill, žádný automatický restart, chyba k nahlédnutí
    wait("po zabití Idle s chybou", || {
        let s = r.sup.status().remove(0);
        s.phase == Phase::Idle && s.error.is_some()
    });
    assert!(r.sup.status()[0].error.as_deref().unwrap().contains("včas"));
    assert!(r.log("echo-hang").contains("init dead=false"));
}

#[test]
fn wrong_protocol_version_is_incompatible_and_stays_off() {
    let t = Timing { hello: Duration::from_secs(5), ready: Duration::from_secs(5), grace: Duration::from_millis(300) };
    let r = rig(vec![man("echo-badver", Start::Always)], &[], t);
    r.sup.sync(&ids(&["echo-badver"]));
    r.wait_phase("echo-badver", Phase::Incompatible);
    assert!(r.sup.status()[0].error.as_deref().unwrap().contains("nekompatibilní"));
    // znovu se nezkouší, dokud uživatel rozšíření nevypne
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(r.phase("echo-badver"), Phase::Incompatible);
    r.sup.sync(&[]);
    assert_eq!(r.phase("echo-badver"), Phase::Idle);
}

#[test]
fn wake_is_refused_when_disabled_or_in_game() {
    let r = rig(vec![man("echo-demand", Start::OnDemand)], &[], Timing::default());
    assert!(r.sup.wake("echo-demand", "test").is_err(), "nezapnuté");
    r.sup.sync(&ids(&["echo-demand"]));
    r.env.dead.store(true, Ordering::SeqCst);
    assert!(r.sup.wake("echo-demand", "test").is_err(), "ve hře");
    assert_eq!(r.phase("echo-demand"), Phase::Idle);
    assert!(r.sup.start("neznamy").is_err());
}
