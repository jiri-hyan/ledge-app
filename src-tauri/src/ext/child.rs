//! Strana rozšíření: `ledge.exe --ext <id>` (volá `main.rs` před Tauri). Kostra NDJSON po stdin/stdout:
//! `hello` → čekání na `init` → `Body::run` (okno, scéna — E2) → `bye`. Konkrétní rozšíření implementuje
//! `Body` a registruje se v `body_for`. stdout patří jen protokolu (logy na stderr, Ledge je sbírá do
//! `fail_tail`); EOF na stdin = Ledge zemřel → synteticky `Down::Quit`.

use super::wire::{self, Decoded, Down, Up};
use parking_lot::Mutex;
use serde_json::Value;
use std::io::{BufReader, Read, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::Arc;
use std::time::Duration;

/// Jak dlouho čekáme na `init` po `hello`.
const INIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Délka běhu `--diag`.
const DIAG_SECS: Duration = Duration::from_secs(60);

/// Co Ledge poslal v `init`.
#[derive(Debug, Clone)]
pub struct InitData {
    pub cfg: Value,
    pub lang: String,
    pub dead: bool,
    #[allow(dead_code)] // revize témat z `init` (zatím vždy `{}`), čte je E2
    pub rev: Value,
}

/// Zapisovač zpráv ext → Ledge (sdílitelný mezi vlákny, řádek se zapíše celý a hned splachne).
#[derive(Clone)]
pub struct Out(Arc<Mutex<Box<dyn Write + Send>>>);

impl Out {
    pub fn new(w: impl Write + Send + 'static) -> Self {
        Out(Arc::new(Mutex::new(Box::new(w))))
    }
    /// `false` = Ledge už nečte (rozbitá roura).
    pub fn send(&self, m: &Up) -> bool {
        let line = wire::encode_up(m);
        let mut w = self.0.lock();
        w.write_all(line.as_bytes()).and_then(|()| w.flush()).is_ok()
    }
    /// Okno vzniklo a vykresluje; teprve teď Ledge počítá rozšíření za běžící.
    pub fn ready(&self) -> bool {
        self.send(&Up::Ready)
    }
    /// Uživatelská akce v rozšíření (vynuluje nečinnost).
    pub fn active(&self) -> bool {
        self.send(&Up::Active)
    }
    pub fn err(&self, msg: impl Into<String>) -> bool {
        self.send(&Up::Err { msg: msg.into() })
    }
}

/// Vstupy pro tělo rozšíření.
pub struct Io {
    #[allow(dead_code)] // id rozšíření pro tělo (E2)
    pub id: String,
    pub init: InitData,
    /// zprávy Ledge → ext po `init`; `Down::Quit` přijde i při EOF stdin (Ledge zemřel)
    pub rx: Receiver<Down>,
    pub out: Out,
}

/// Konkrétní rozšíření (E2: `wall`). `run` běží na hlavním vlákně procesu (okno potřebuje vlastní smyčku
/// zpráv) a vrací návratový kód; `ready` pošle samo, až bude co ukázat.
pub trait Body {
    /// Témata dat, o která stojí (Ledge je ořízne podle manifestu).
    fn wants(&self) -> Vec<String>;
    fn run(&mut self, io: Io) -> i32;
}

/// Registr těl: `wall` (živá tapeta, E2). Další rozšíření = větev sem + řádek v `ext::REGISTRY`.
pub fn body_for(id: &str) -> Option<Box<dyn Body>> {
    match id {
        "wall" => Some(Box::new(super::wall::Wall)),
        _ => None,
    }
}

/// Jedno čtecí vlákno: řádky → `Down`. Neznámé typy a nesmysly se zahodí, chyba verze / příliš dlouhý řádek
/// a EOF končí `Down::Quit`.
fn spawn_reader(input: impl Read + Send + 'static, tx: Sender<Down>) {
    std::thread::spawn(move || {
        let mut r = BufReader::new(input);
        let mut buf = Vec::new();
        loop {
            match wire::read_line(&mut r, &mut buf) {
                Ok(Some(Ok(line))) => match wire::decode_down(&line) {
                    Ok(Decoded::Msg(m)) => {
                        if tx.send(m).is_err() {
                            return;
                        }
                    }
                    Ok(Decoded::Unknown(t)) => eprintln!("ext: neznámá zpráva '{t}' zahozena"),
                    Err(e @ wire::WireError::Version(_)) => {
                        eprintln!("ext: {e}");
                        break;
                    }
                    Err(e) => eprintln!("ext: {e}"),
                },
                Ok(Some(Err(e))) => {
                    eprintln!("ext: {e}");
                    break;
                }
                Ok(None) | Err(_) => break,
            }
        }
        let _ = tx.send(Down::Quit);
    });
}

/// Handshake + běh těla nad zadanými proudy (v produkci stdin/stdout, v testech roury).
pub fn run_with(id: &str, mut body: Box<dyn Body>, input: impl Read + Send + 'static, output: impl Write + Send + 'static) -> i32 {
    let out = Out::new(output);
    if !out.send(&Up::Hello { id: id.to_string(), pid: std::process::id(), wants: body.wants() }) {
        return 1;
    }
    let (tx, rx) = channel();
    spawn_reader(input, tx);
    let init = loop {
        match rx.recv_timeout(INIT_TIMEOUT) {
            Ok(Down::Init { cfg, lang, dead, rev }) => break InitData { cfg, lang, dead, rev },
            Ok(Down::Quit) => return 0,
            Ok(_) => {} // před `init` nic jiného neposlouchat
            Err(RecvTimeoutError::Timeout) => {
                eprintln!("ext: init nepřišel do {} s", INIT_TIMEOUT.as_secs());
                return 2;
            }
            Err(RecvTimeoutError::Disconnected) => return 0,
        }
    };
    let rc = body.run(Io { id: id.to_string(), init, rx, out: out.clone() });
    if rc == 0 {
        out.send(&Up::Bye);
    }
    rc
}

/// Pojmenovaný mutex `Local\ledge-ext-<id>`: druhá instance téhož rozšíření skončí. `true` = jsme první.
#[cfg(windows)]
fn single_instance(id: &str) -> bool {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;
    let name: Vec<u16> = format!("Local\\ledge-ext-{id}").encode_utf16().chain(Some(0)).collect();
    unsafe {
        // handle se záměrně neuvolňuje — drží ho proces do konce
        match CreateMutexW(None, false, PCWSTR(name.as_ptr())) {
            Ok(_) => GetLastError() != ERROR_ALREADY_EXISTS,
            Err(_) => true, // bez mutexu raději běžet; supervisor stejně drží max 1 dítě na id
        }
    }
}

/// `ledge.exe --ext <id>` — vstup z `main.rs`. Vrací návratový kód procesu.
pub fn main() -> i32 {
    // stejná bariéra jako `run()`: automatizace bez oken
    if ["LEDGE_NO_GUI", "NOTCH_NO_GUI"].iter().any(|k| std::env::var(k).as_deref() == Ok("1")) {
        return 0;
    }
    let Some(id) = std::env::args().nth(2) else {
        eprintln!("použití: ledge --ext <id>");
        return 64;
    };
    let Some(body) = body_for(&id) else {
        eprintln!("rozšíření '{id}' tahle verze Ledge nezná");
        return 64;
    };
    if !single_instance(&id) {
        return 0;
    }
    if std::env::args().any(|a| a == "--diag") {
        return run_diag(body);
    }
    run_with(&id, body, std::io::stdin(), std::io::stdout())
}

/// `ledge.exe --ext <id> --diag`: bez Ledge (ruční spuštění), 60 s, `cfg.diag = true`, výstup protokolu se zahodí.
/// Pro šéfovy zkoušky (Win+D, restart Průzkumníka) — tělo si log píše samo (`wall/diag.rs`).
fn run_diag(mut body: Box<dyn Body>) -> i32 {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        std::thread::sleep(DIAG_SECS);
        let _ = tx.send(Down::Quit);
    });
    let init = InitData { cfg: serde_json::json!({ "diag": true }), lang: "cs".into(), dead: false, rev: Value::Null };
    eprintln!("ext --diag: {} s, log viz codemap ext-detail.md", DIAG_SECS.as_secs());
    body.run(Io { id: "diag".into(), init, rx, out: Out::new(std::io::sink()) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::echo::Echo;
    use std::io::Cursor;

    /// Zapisovač do sdílené paměti.
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Sink {
        fn lines(&self) -> Vec<String> {
            String::from_utf8_lossy(&self.0.lock()).lines().map(str::to_string).collect()
        }
    }

    fn init_line() -> String {
        wire::encode_down(&Down::Init { cfg: serde_json::json!({}), lang: "cs".into(), dead: false, rev: serde_json::json!({}) })
    }

    #[test]
    fn handshake_hello_then_body_then_bye_on_quit() {
        let sink = Sink::default();
        let input = format!("{}{}", init_line(), wire::encode_down(&Down::Quit));
        let rc = run_with("echo-t1", Box::new(Echo::new()), Cursor::new(input.into_bytes()), sink.clone());
        assert_eq!(rc, 0);
        let lines = sink.lines();
        let j = |l: &str| serde_json::from_str::<Value>(l).unwrap();
        assert_eq!(j(&lines[0]), serde_json::json!({"v": 1, "t": "hello", "id": "echo-t1", "pid": std::process::id(), "wants": ["media"]}));
        assert_eq!(j(&lines[1]), serde_json::json!({"v": 1, "t": "ready"}));
        assert_eq!(j(lines.last().unwrap()), serde_json::json!({"v": 1, "t": "bye"}));
    }

    #[test]
    fn eof_without_init_exits_cleanly_and_garbage_before_init_is_ignored() {
        let sink = Sink::default();
        let rc = run_with("echo-t2", Box::new(Echo::new()), Cursor::new(Vec::new()), sink.clone());
        assert_eq!(rc, 0);
        assert_eq!(sink.lines().len(), 1, "jen hello — tělo se nespustilo");
        // nesmysl a neznámý typ před init nevadí
        let sink = Sink::default();
        let input = format!("nejson\n{{\"v\":1,\"t\":\"z-budoucna\"}}\n{}{}", init_line(), wire::encode_down(&Down::Quit));
        assert_eq!(run_with("echo-t2", Box::new(Echo::new()), Cursor::new(input.into_bytes()), sink.clone()), 0);
        assert!(sink.lines().iter().any(|l| l.contains(r#""t":"ready""#)));
    }

    #[test]
    fn wrong_version_closes_channel_and_exits() {
        let sink = Sink::default();
        let rc = run_with("echo-t3", Box::new(Echo::new()), Cursor::new(b"{\"v\":9,\"t\":\"init\"}\n".to_vec()), sink.clone());
        assert_eq!(rc, 0, "verze mimo kontrakt = reader končí, tělo se nespustí");
        assert_eq!(sink.lines().len(), 1);
    }
}
