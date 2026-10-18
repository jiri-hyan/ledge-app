//! Jednorázový loopback server pro přesměrování z prohlížeče (RFC 8252 §7.3): `127.0.0.1` na
//! náhodném portu (a stejný port na `[::1]`, když jde — prohlížeč může `localhost` přeložit na IPv6),
//! s termínem a zrušením. Přijme jen požadavek s `code` / `error` a shodným `state` (jinak čeká dál
//! — cizí požadavek přihlášení neukončí). Po první platné odpovědi se server zavře.

use super::oauth::{Callback, parse_callback};
use super::pkce::same;
use crate::i18n::tr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Jak dlouho se čeká na přihlášení v prohlížeči.
pub const WAIT: Duration = Duration::from_secs(5 * 60);

pub struct Listener {
    v4: tiny_http::Server,
    v6: Option<tiny_http::Server>,
    pub port: u16,
}

#[derive(Debug, PartialEq)]
pub enum WaitEnd {
    Code(String),
    /// poskytovatel vrátil `error=…` (zrušeno, nepovoleno)
    Provider(String),
    Timeout,
    Cancelled,
}

pub fn bind() -> Result<Listener, String> {
    let err = || tr("Ledge nemůže otevřít místní port pro přihlášení.", "Ledge can't open a local port for sign-in.").to_string();
    let v4 = tiny_http::Server::http("127.0.0.1:0").map_err(|_| err())?;
    let port = v4.server_addr().to_ip().map(|a| a.port()).ok_or_else(err)?;
    let v6 = tiny_http::Server::http(format!("[::1]:{port}")).ok();
    Ok(Listener { v4, v6, port })
}

fn page(ok: bool) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let (title, text) = if ok {
        (tr("Přihlášeno", "Signed in"), tr("Hotovo — můžeš zavřít tuhle kartu a vrátit se do Ledge.", "Done — you can close this tab and go back to Ledge."))
    } else {
        (tr("Nepřihlášeno", "Not signed in"), tr("Přihlášení se nepovedlo — Ledge ti řekne proč. Kartu můžeš zavřít.", "Sign-in didn't work — Ledge will tell you why. You can close this tab."))
    };
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Ledge — {title}</title></head><body style=\"font:16px system-ui,sans-serif;background:#111;color:#eee;display:grid;place-items:center;height:90vh\"><div><h1 style=\"font-size:20px\">{}</h1><p>{text}</p></div></body></html>",
        if ok { format!("✓ {title}") } else { title.to_string() }
    );
    let h = |k: &str, v: &str| tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).expect("statická hlavička");
    tiny_http::Response::from_string(html)
        .with_header(h("Content-Type", "text/html; charset=utf-8"))
        .with_header(h("Cache-Control", "no-store"))
        .with_header(h("Referrer-Policy", "no-referrer"))
}

impl Listener {
    /// Čeká na přesměrování se `state`. Blokuje — volat mimo async vlákna (`spawn_blocking`).
    pub fn wait(self, state: &str, deadline: Instant, cancel: &AtomicBool) -> WaitEnd {
        let servers: Vec<&tiny_http::Server> = std::iter::once(&self.v4).chain(self.v6.as_ref()).collect();
        loop {
            if cancel.load(Ordering::SeqCst) {
                return WaitEnd::Cancelled;
            }
            if Instant::now() >= deadline {
                return WaitEnd::Timeout;
            }
            for s in &servers {
                let Ok(Some(rq)) = s.recv_timeout(Duration::from_millis(150)) else { continue };
                let end = match parse_callback(rq.url()) {
                    Callback::Code { code, state: st } if same(&st, state) => Some(WaitEnd::Code(code)),
                    Callback::Error { error, state: st } if same(&st, state) => Some(WaitEnd::Provider(error)),
                    _ => None,
                };
                match end {
                    Some(e) => {
                        let _ = rq.respond(page(matches!(e, WaitEnd::Code(_))));
                        return e;
                    }
                    None => {
                        let _ = rq.respond(tiny_http::Response::from_string("").with_status_code(404));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn hit(port: u16, path: &str) -> String {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    #[test]
    fn waits_for_matching_state_then_closes() {
        let l = bind().unwrap();
        let port = l.port;
        assert!(port > 0);
        let t = std::thread::spawn(move || l.wait("st4te", Instant::now() + Duration::from_secs(10), &AtomicBool::new(false)));
        // cizí požadavky přihlášení neukončí
        assert!(hit(port, "/favicon.ico").contains("404"));
        assert!(hit(port, "/?code=x&state=jiny").contains("404"));
        let ok = hit(port, "/?code=abc%2F1&state=st4te");
        assert!(ok.contains("200") && ok.contains("Ledge"));
        assert_eq!(t.join().unwrap(), WaitEnd::Code("abc/1".into()));
    }

    #[test]
    fn provider_error_cancel_and_timeout() {
        let l = bind().unwrap();
        let port = l.port;
        let t = std::thread::spawn(move || l.wait("s", Instant::now() + Duration::from_secs(10), &AtomicBool::new(false)));
        hit(port, "/?error=access_denied&state=s");
        assert_eq!(t.join().unwrap(), WaitEnd::Provider("access_denied".into()));
        let l = bind().unwrap();
        assert_eq!(l.wait("s", Instant::now() + Duration::from_millis(50), &AtomicBool::new(false)), WaitEnd::Timeout);
        let l = bind().unwrap();
        assert_eq!(l.wait("s", Instant::now() + Duration::from_secs(10), &AtomicBool::new(true)), WaitEnd::Cancelled);
    }
}
