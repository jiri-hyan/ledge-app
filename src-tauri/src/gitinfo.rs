//! Běh gitu s termínem (`run`, `capture`, `capture_tail`) a `git status` (větev, neuložené změny, ahead/behind) pro
//! sessions a hygienu, na pozadí s tvrdým stropem.

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub branch: Option<String>,
    pub dirty: u32,
    pub ahead: u32,
    pub behind: u32,
}

pub fn parse_porcelain_v2(out: &str) -> GitInfo {
    let mut g = GitInfo::default();
    for line in out.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            g.branch = if b == "(detached)" { None } else { Some(b.to_string()) };
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    g.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    g.behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') && !line.is_empty() {
            g.dirty += 1;
        }
    }
    g
}

/// Argumenty periodického `git status`: bez volitelných zámků (`--no-optional-locks`) — pozadí nesmí
/// zabrat `index.lock` zrovna, když uživatel v repu commituje.
const STATUS_ARGS: [&str; 5] = ["--no-optional-locks", "status", "--porcelain=v2", "--branch", "--untracked-files=normal"];
/// Tvrdý strop jednoho `git status` na pozadí (síťový disk, zamčený repo): po něm se strom gitu zabije,
/// vlákno nezůstane viset a příští kolo to zkusí znovu.
const STATUS_TIMEOUT: Duration = Duration::from_secs(60);

pub fn status(dir: &Path) -> Option<GitInfo> {
    status_until(dir, Instant::now() + STATUS_TIMEOUT)
}

/// `git status` na pozadí: bez fsmonitoru jako ostatní volání (`git_cmd`) — globální `core.fsmonitor=true`
/// by jinak v každém repu nastartoval daemon, kterého Job Object hned zabije.
fn status_cmd(dir: &Path) -> Command {
    let mut cmd = git_cmd(dir, &STATUS_ARGS);
    cmd.env("GIT_OPTIONAL_LOCKS", "0").env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

fn status_until(dir: &Path, deadline: Instant) -> Option<GitInfo> {
    let out = capture(&mut status_cmd(dir), None, false, deadline).ok()?;
    out.success.then(|| parse_porcelain_v2(&out.stdout))
}

/// `git <args>` v `dir` s tvrdým termínem (hook Claude Code čeká max. 5 s). Po termínu zabije celý
/// strom gitu (Job Object) a vrátí `None`, stejně jako při chybě nebo nenulovém exit kódu.
/// Vždy s `core.fsmonitor=false` — cwd přichází z hooku a fsmonitor by mohl spustit cizí program.
pub fn run(dir: &Path, args: &[&str], deadline: Instant) -> Option<String> {
    run_with_input(dir, args, None, deadline)
}

/// Jako [`run`], `input` jde gitu na stdin (např. pathspecy pro `rev-list --stdin`).
pub fn run_with_input(dir: &Path, args: &[&str], input: Option<&str>, deadline: Instant) -> Option<String> {
    let out = capture(&mut git_cmd(dir, args), input, false, deadline).ok()?;
    out.success.then_some(out.stdout)
}

/// Jako [`run`], navíc proměnné prostředí `env` a vrací i stderr a neúspěšný exit kód
/// (synchronizace poznámek podle stderr rozlišuje offline / nepřihlášeno).
pub fn run_env(dir: &Path, args: &[&str], env: &[(&str, &str)], deadline: Instant) -> Result<Captured, RunError> {
    let mut cmd = git_cmd(dir, args);
    cmd.envs(env.iter().copied());
    capture(&mut cmd, None, true, deadline)
}

/// `git -c core.fsmonitor=false <args>` v `dir`.
fn git_cmd(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.fsmonitor=false"]).args(args).current_dir(dir);
    cmd
}

/// Výsledek [`capture`]: proces doběhl (s jakýmkoli exit kódem).
#[derive(Debug)]
pub struct Captured {
    pub success: bool,
    /// návratový kód procesu (`None` = ukončen signálem / neznámý)
    pub code: Option<i32>,
    pub stdout: String,
    /// prázdné, když volající stderr nechtěl
    pub stderr: String,
}

/// Proč [`capture`] nevrátil výstup.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    /// termín prošel (před spuštěním nebo během běhu — strom procesu je zabitý)
    Timeout,
    /// proces nešel spustit
    Spawn(String),
}

/// Obecné spuštění programu bez okna (CREATE_NO_WINDOW) s tvrdým termínem: po termínu zabije celý
/// strom (Job Object) a vrátí [`RunError::Timeout`]. `cmd` připraví volající (program, argumenty,
/// cwd, env); stdin/stdout/stderr nastaví tahle funkce. `input` jde na stdin, `stderr` = číst i stderr.
/// Jediné místo, které proces opravdu spouští — `run`, `run_with_input`, `run_env` jsou obálky.
pub fn capture(cmd: &mut Command, input: Option<&str>, stderr: bool, deadline: Instant) -> Result<Captured, RunError> {
    capture_tail(cmd, input, stderr, deadline, None)
}

/// Přidá `chunk` do `buf`; s `cap` drží jen posledních ~`cap` bajtů (ořez je líný, aby se nekopírovalo po
/// každém chunku — před čtením výsledku volej [`trim_tail`]). Vrací, jestli se něco usekla.
fn append_tail(buf: &mut Vec<u8>, chunk: &[u8], cap: Option<usize>) -> bool {
    buf.extend_from_slice(chunk);
    match cap {
        Some(cap) if buf.len() > cap.saturating_mul(2).max(cap + 16 * 1024) => {
            buf.drain(..buf.len() - cap);
            true
        }
        _ => false,
    }
}

/// Doořízne `buf` na posledních `cap` bajtů; vrací, jestli se něco usekla.
fn trim_tail(buf: &mut Vec<u8>, cap: Option<usize>) -> bool {
    match cap {
        Some(cap) if buf.len() > cap => {
            buf.drain(..buf.len() - cap);
            true
        }
        _ => false,
    }
}

/// Čte rouru až do konce a průběžně plní sdílený buffer (poslední `cap` bajtů). Díky sdílenému bufferu jde
/// přečtené použít i tehdy, když rouru drží potomek a EOF nepřijde (viz [`capture_tail`]).
fn pump_tail(r: &mut impl std::io::Read, cap: Option<usize>, shared: &std::sync::Mutex<Vec<u8>>) -> bool {
    let mut cut = false;
    let mut chunk = [0u8; 16 * 1024];
    loop {
        match r.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(mut b) = shared.lock() {
                    cut |= append_tail(&mut b, &chunk[..n], cap);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    cut
}

/// (jen testy: produkce čte přes `pump_tail`) Přečte rouru do konce; s `cap` drží v paměti jen posledních `cap` bajtů (ukecaný příkaz rychlé
/// akce nesmí nafouknout Ledge). Vrací (data, bylo useknuto?).
#[cfg(test)]
pub fn read_tail(r: &mut impl std::io::Read, cap: Option<usize>) -> (Vec<u8>, bool) {
    let shared = std::sync::Mutex::new(Vec::new());
    let mut cut = pump_tail(r, cap, &shared);
    let mut buf = shared.into_inner().unwrap_or_default();
    cut |= trim_tail(&mut buf, cap);
    (buf, cut)
}

/// Po exitu hlavního procesu: jak dlouho ještě čekat, až se vyprázdní roury, než se potomek držící rouru
/// (démon, `Start-Process`, `ssh -f`) zabije a použije se to, co přišlo (L240).
const PIPE_GRACE: Duration = Duration::from_millis(300);
/// Po zabití jobu už roury nikdo nedrží, EOF čeká jen na naplánování čtecích vláken (pod zátěží to trvá
/// i sekundy). Dřív se po `PIPE_GRACE` bral rovnou prázdný/částečný buffer → `git status` „bez změn“ (L334).
const DRAIN_AFTER_KILL: Duration = Duration::from_secs(5);

/// Jako [`capture`], ale ze stdout i stderr drží jen posledních `cap` bajtů (viz [`read_tail`]).
/// Exit hlavního procesu má přednost před EOF rour: potomek se zděděnou rourou nezdrží výsledek
/// (po exitu `PIPE_GRACE`, pak se job zabije); termín platí na exit procesu.
pub fn capture_tail(cmd: &mut Command, input: Option<&str>, stderr: bool, deadline: Instant, cap: Option<usize>) -> Result<Captured, RunError> {
    capture_paced(cmd, input, stderr, deadline, cap, Duration::ZERO)
}

/// [`capture_tail`] s umělým zpožděním čtecích vláken (`pump_delay`; produkce `ZERO`) — test zátěže, kdy
/// vlákna nestihnou vyprázdnit rouru v `PIPE_GRACE`.
fn capture_paced(cmd: &mut Command, input: Option<&str>, stderr: bool, deadline: Instant, cap: Option<usize>, pump_delay: Duration) -> Result<Captured, RunError> {
    use std::io::Write;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    if Instant::now() >= deadline {
        return Err(RunError::Timeout);
    }
    cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(if stderr { Stdio::piped() } else { Stdio::null() });
    // KILL_ON_JOB_CLOSE: po návratu (i při úspěchu) nezůstane po procesu nic běžet;
    // do jobu se dostane už při vzniku (pozastavený), takže mu neuteče žádný potomek
    let job = crate::procman::Job::new().ok();
    let mut child = match &job {
        Some(j) => j.spawn(cmd, crate::procman::CREATE_NO_WINDOW).map_err(RunError::Spawn)?,
        None => cmd.creation_flags(crate::procman::CREATE_NO_WINDOW).spawn().map_err(|e| RunError::Spawn(e.to_string()))?,
    };
    let kill = |child: &mut std::process::Child| {
        if let Some(j) = &job {
            j.kill();
        }
        let _ = child.kill();
        let _ = child.wait();
    };
    if let (Some(text), Some(mut sin)) = (input, child.stdin.take()) {
        let text = text.to_string();
        // zápis ve vlákně: velký vstup by jinak mohl uváznout proti plnému stdout
        std::thread::spawn(move || {
            let _ = sin.write_all(text.as_bytes());
        });
    }
    let Some(mut out) = child.stdout.take() else {
        kill(&mut child);
        return Err(RunError::Spawn("bez stdout".into()));
    };
    // každá roura ve vlastním vlákně (plná roura stderr by jinak zablokovala proces i čtení stdout);
    // vlákna plní sdílené buffery a po EOF pošlou `()`
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let out_buf = Arc::new(Mutex::new(Vec::new()));
    let err_buf = Arc::new(Mutex::new(Vec::new()));
    let mut pipes = 1;
    {
        let (buf, tx) = (out_buf.clone(), done_tx.clone());
        std::thread::spawn(move || {
            std::thread::sleep(pump_delay);
            pump_tail(&mut out, cap, &buf);
            let _ = tx.send(());
        });
    }
    if let Some(mut e) = child.stderr.take() {
        pipes += 1;
        let (buf, tx) = (err_buf.clone(), done_tx.clone());
        std::thread::spawn(move || {
            std::thread::sleep(pump_delay);
            pump_tail(&mut e, cap, &buf);
            let _ = tx.send(());
        });
    }
    drop(done_tx);
    // exit procesu: čekání se budí koncem rour (běžný případ), jinak ve zvětšujících se krocích (žádné 2ms točení)
    let mut closed = 0;
    let mut step = Duration::from_millis(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() < deadline => {
                let wait = step.min(deadline.saturating_duration_since(Instant::now()));
                if closed < pipes {
                    if done_rx.recv_timeout(wait).is_ok() {
                        closed += 1;
                    }
                } else {
                    std::thread::sleep(wait.min(Duration::from_millis(5)));
                }
                step = (step * 2).min(Duration::from_millis(50));
            }
            _ => {
                kill(&mut child);
                return Err(RunError::Timeout);
            }
        }
    };
    // po exitu: roury se vyprázdní hned, nebo je drží potomek → po lhůtě ho zabít a vzít, co dorazilo
    let grace_end = Instant::now() + PIPE_GRACE;
    while closed < pipes {
        let left = grace_end.saturating_duration_since(Instant::now());
        if done_rx.recv_timeout(left).is_err() {
            break;
        }
        closed += 1;
    }
    if closed < pipes
        && let Some(j) = &job
    {
        j.kill();
        // držitelé rour jsou po zabití pryč → EOF přijde, jakmile čtecí vlákna doběhnou; nebrat buffer předčasně
        let drain_end = Instant::now() + DRAIN_AFTER_KILL;
        while closed < pipes {
            let left = drain_end.saturating_duration_since(Instant::now());
            if done_rx.recv_timeout(left).is_err() {
                break;
            }
            closed += 1;
        }
    }
    let take = |b: &Mutex<Vec<u8>>| {
        let mut v = b.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
        trim_tail(&mut v, cap);
        v
    };
    Ok(Captured {
        success: status.success(),
        code: status.code(),
        stdout: String::from_utf8_lossy(&take(&out_buf)).into_owned(),
        stderr: String::from_utf8_lossy(&take(&err_buf)).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Velkorysý termín pro testy, které ověřují výsledek gitu, ne rychlost: pod zátěží stroje (fronta, AV,
    /// studený start gitu) by pár sekund dávalo falešné FAIL. Test termínu má vlastní krátký termín.
    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(120)
    }

    /// Vstup dítěte pro `capture_kills_child_at_deadline`: bez `LEDGE_GITINFO_SLEEP_MS` je to prázdný test,
    /// s ní proces spí (testovací binár spuštěný znovu — bez závislosti na `sleep`/`sh`/`ping` v PATH).
    #[test]
    fn sleep_child() {
        let Some(ms) = std::env::var("LEDGE_GITINFO_SLEEP_MS").ok().and_then(|v| v.parse::<u64>().ok()) else { return };
        std::thread::sleep(Duration::from_millis(ms));
    }

    #[test]
    fn run_respects_deadline_and_errors() {
        let t = tempfile::tempdir().unwrap();
        let tmp = t.path();
        // termín už prošel → git se ani nespustí
        assert_eq!(run(tmp, &["--version"], Instant::now()), None);
        let v = run(tmp, &["--version"], soon()).unwrap();
        assert!(v.starts_with("git version"));
        // nenulový exit → None
        assert_eq!(run(tmp, &["nejsem-prikaz"], soon()), None);
        // stdin dojde gitu (hash-object --stdin vrátí sha vstupu)
        let sha = run_with_input(tmp, &["hash-object", "--stdin"], Some("ahoj\n"), soon()).unwrap();
        assert_eq!(sha.trim().len(), 40);
        // run_env: proměnná prostředí dojde gitu, chyba vrátí stderr i neúspěch
        let d = soon();
        let o = run_env(tmp, &["var", "GIT_AUTHOR_IDENT"], &[("GIT_AUTHOR_NAME", "Zkouska"), ("GIT_AUTHOR_EMAIL", "z@x")], d).unwrap();
        assert!(o.success && o.stdout.starts_with("Zkouska <z@x>"), "{o:?}");
        let o = run_env(tmp, &["nejsem-prikaz"], &[], d).unwrap();
        assert!(!o.success && o.stderr.contains("nejsem-prikaz"), "{o:?}");
        assert_eq!(run_env(tmp, &["--version"], &[], Instant::now()).unwrap_err(), RunError::Timeout);
    }

    /// Termín uprostřed běhu: dítě spí 120 s, termín 500 ms → strom zabit, `Timeout`. Strop doby je
    /// velkorysý (60 s): ověřuje, že se nečekalo na dítě, ne rychlost startu procesu.
    #[test]
    fn capture_kills_child_at_deadline() {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args(["gitinfo::tests::sleep_child", "--nocapture", "--test-threads=1"]).env("LEDGE_GITINFO_SLEEP_MS", "120000");
        let t0 = Instant::now();
        assert_eq!(capture(&mut cmd, None, true, Instant::now() + Duration::from_millis(500)).unwrap_err(), RunError::Timeout);
        assert!(t0.elapsed() < Duration::from_secs(60), "čekalo se na dítě: {:?}", t0.elapsed());
    }

    /// L240: potomek drží zděděnou rouru (stderr), hlavní proces už skončil → výsledek přijde po krátké lhůtě
    /// rour (ne po termínu), přečtený výstup se použije a potomek je zabitý.
    #[test]
    fn capture_returns_after_exit_even_if_child_holds_the_pipe() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "echo hotovo & start /B ping -n 30 127.0.0.1 >nul"]);
        let t0 = Instant::now();
        let out = capture(&mut cmd, None, true, Instant::now() + Duration::from_secs(60)).expect("výsledek, ne Timeout");
        assert!(out.success && out.stdout.contains("hotovo"), "{out:?}");
        assert!(t0.elapsed() < Duration::from_secs(20), "čekalo se na rouru potomka: {:?}", t0.elapsed());
    }

    /// L334: čtecí vlákna roury se pod zátěží (1600 testů + clippy) naplánují později než `PIPE_GRACE` po exitu.
    /// Výstup, který už leží v rouře, se nesmí zahodit (dřív: `git status` prošel s prázdným stdout = „čisto“).
    /// Deterministicky: vlákna uměle spí 3× déle než lhůta.
    #[test]
    fn capture_keeps_output_when_readers_are_slower_than_grace() {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "echo hotovo"]);
        let slow = PIPE_GRACE * 3;
        let out = capture_paced(&mut cmd, None, true, Instant::now() + Duration::from_secs(60), None, slow).expect("výsledek");
        assert!(out.success && out.stdout.contains("hotovo"), "{out:?}");
    }

    #[test]
    fn tail_helpers_table() {
        // (vstup, cap) → (výsledek, useknuto)
        let big: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        for (data, cap, want_len, want_cut) in [(&big[..], Some(1000), 1000, true), (&big[..], None, 100_000, false), (&b"ahoj"[..], Some(10), 4, false)] {
            let (buf, cut) = read_tail(&mut &data[..], cap);
            assert_eq!((buf.len(), cut), (want_len, want_cut), "{cap:?}");
            assert!(data.ends_with(&buf));
        }
    }

    #[test]
    fn capture_reports_stderr_exit_and_spawn_errors() {
        let t = tempfile::tempdir().unwrap();
        let tmp = t.path();
        // nenulový exit: výstup se vrátí i se stderr (podle něj se rozhoduje „proč“)
        let mut cmd = Command::new("git");
        cmd.args(["nejsem-prikaz"]).current_dir(tmp);
        let out = capture(&mut cmd, None, true, soon()).unwrap();
        assert!(!out.success);
        assert!(out.stderr.contains("nejsem-prikaz"), "{}", out.stderr);
        // neexistující program → Spawn, ne panika
        let mut cmd = Command::new("ledge-neexistujici-program-xyz");
        assert!(matches!(capture(&mut cmd, None, true, soon()), Err(RunError::Spawn(_))));
        // prošlý termín → Timeout bez spuštění
        let mut cmd = Command::new("git");
        assert_eq!(capture(&mut cmd, None, true, Instant::now()).unwrap_err(), RunError::Timeout);
    }

    #[test]
    fn background_status_takes_no_optional_locks_and_respects_deadline() {
        assert_eq!(STATUS_ARGS[0], "--no-optional-locks", "pozadí nesmí zabrat index.lock");
        let cmd = status_cmd(Path::new("."));
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args[..3], ["-c", "core.fsmonitor=false", STATUS_ARGS[0]], "fsmonitor vypnutý, a to před podpříkazem");
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().to_path_buf();
        // není repo → None (ne panika, ne prázdný GitInfo)
        assert_eq!(status_until(&dir, soon()), None);
        // neexistující složka → None
        assert_eq!(status_until(&dir.join("neni"), soon()), None);
        // repo s jedním nesledovaným souborem
        assert!(run(&dir, &["init", "-q", "-b", "hlavni"], soon()).is_some());
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        let g = status_until(&dir, soon()).expect("repo");
        assert_eq!((g.dirty, g.ahead, g.behind), (1, 0, 0));
        // prošlý termín → git se nespustí
        assert_eq!(status_until(&dir, Instant::now()), None);
    }

    #[test]
    fn porcelain_v2_survives_garbage() {
        let cases: [(&str, GitInfo); 6] = [
            ("", GitInfo::default()),
            ("\n\n\n", GitInfo::default()),
            ("# branch.head (detached)\n", GitInfo::default()),
            ("# branch.ab +x -y\n", GitInfo::default()),
            ("# branch.ab +99999999999 -3\n", GitInfo { behind: 3, ..GitInfo::default() }),
            ("# branch.head větev-č\n2 R. N... 1 1 1 a b R100 x\tstary\nu UU N... 1 1 1 1 a b c f\n", GitInfo { branch: Some("větev-č".into()), dirty: 2, ..GitInfo::default() }),
        ];
        for (input, want) in cases {
            assert_eq!(parse_porcelain_v2(input), want, "{input:?}");
        }
    }

    #[test]
    fn parses_branch_ab_and_dirty() {
        let g = parse_porcelain_v2(
            "# branch.oid abc\n# branch.head atlas-v2\n# branch.upstream origin/atlas-v2\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/x.ts\n? new.txt\n",
        );
        assert_eq!(g, GitInfo { branch: Some("atlas-v2".into()), dirty: 2, ahead: 2, behind: 1 });
    }
}
