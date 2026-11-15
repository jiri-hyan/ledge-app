//! Testovací rozšíření `echo` (jen `cfg(test)`): ověřuje handshake a smyčku bez okna. Zapisuje každou přijatou
//! zprávu jako řádek do souboru `cfg.log` (dočasná složka testu) a umí na příkaz `cmd` poslat zpět `active`,
//! `req`, `err` nebo spadnout. `cfg.mode`: `ok` (výchozí) · `no_ready` (po `init` mlčí) · `crash` (hned po `ready` kód 3).

use super::child::{Body, Io};
use super::wire::{self, Down, Up};
use std::io::Write;

pub struct Echo;

impl Echo {
    pub fn new() -> Self {
        Echo
    }
}

fn log(io: &Io, line: &str) {
    let Some(path) = io.init.cfg.get("log").and_then(|v| v.as_str()) else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

impl Body for Echo {
    fn wants(&self) -> Vec<String> {
        vec!["media".into()]
    }

    fn run(&mut self, io: Io) -> i32 {
        let mode = io.init.cfg.get("mode").and_then(|v| v.as_str()).unwrap_or("ok").to_string();
        log(&io, &format!("init dead={} lang={}", io.init.dead, io.init.lang));
        match mode.as_str() {
            "no_ready" => {}
            "crash" => {
                io.out.ready();
                return 3;
            }
            _ => {
                io.out.ready();
            }
        }
        while let Ok(m) = io.rx.recv() {
            log(&io, wire::encode_down(&m).trim_end());
            match m {
                Down::Quit => return 0,
                Down::Cmd { name, args } => match name.as_str() {
                    "crash" => return 3,
                    "active" => {
                        io.out.active();
                    }
                    "err" => {
                        io.out.err("echo chyba");
                    }
                    "req" => {
                        let s = |k: &str| args.get(k).and_then(|v| v.as_str()).map(str::to_string);
                        io.out.send(&Up::Req { what: s("what").unwrap_or_default(), act: s("do"), key: s("key") });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        0
    }
}
