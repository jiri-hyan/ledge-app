//! Kdo poslouchá na portu — přímo z tabulky TCP Windows (IPv4 i IPv6), bez spouštění netstatu.

use windows::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
};
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listener {
    pub port: u16,
    pub pid: u32,
}

fn table(family: u32) -> Vec<u8> {
    let mut size: u32 = 0;
    unsafe {
        let r = GetExtendedTcpTable(None, &mut size, false, family, TCP_TABLE_OWNER_PID_LISTENER, 0);
        if r != ERROR_INSUFFICIENT_BUFFER.0 && r != NO_ERROR.0 {
            return Vec::new();
        }
        // tabulka se mezi voláními může zvětšit → rezerva
        for _ in 0..3 {
            size += 4096;
            let mut buf = vec![0u8; size as usize];
            let r = GetExtendedTcpTable(
                Some(buf.as_mut_ptr() as *mut _),
                &mut size,
                false,
                family,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            );
            if r == NO_ERROR.0 {
                return buf;
            }
            if r != ERROR_INSUFFICIENT_BUFFER.0 {
                break;
            }
        }
    }
    Vec::new()
}

fn net_port(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}

/// Všechny naslouchající sockety (v4 + v6).
pub fn listeners() -> Vec<Listener> {
    let mut out = Vec::new();
    let v4 = table(AF_INET.0 as u32);
    if v4.len() >= 4 {
        let n = u32::from_ne_bytes(v4[0..4].try_into().unwrap()) as usize;
        let rows = unsafe { v4.as_ptr().add(4) } as *const MIB_TCPROW_OWNER_PID;
        let max = (v4.len() - 4) / std::mem::size_of::<MIB_TCPROW_OWNER_PID>();
        for i in 0..n.min(max) {
            let r = unsafe { std::ptr::read_unaligned(rows.add(i)) };
            out.push(Listener { port: net_port(r.dwLocalPort), pid: r.dwOwningPid });
        }
    }
    let v6 = table(AF_INET6.0 as u32);
    if v6.len() >= 4 {
        let n = u32::from_ne_bytes(v6[0..4].try_into().unwrap()) as usize;
        let rows = unsafe { v6.as_ptr().add(4) } as *const MIB_TCP6ROW_OWNER_PID;
        let max = (v6.len() - 4) / std::mem::size_of::<MIB_TCP6ROW_OWNER_PID>();
        for i in 0..n.min(max) {
            let r = unsafe { std::ptr::read_unaligned(rows.add(i)) };
            out.push(Listener { port: net_port(r.dwLocalPort), pid: r.dwOwningPid });
        }
    }
    out
}

/// PID procesu, který poslouchá na portu (první nalezený), nebo None = port je volný.
pub fn owner(port: u16) -> Option<u32> {
    listeners().into_iter().find(|l| l.port == port).map(|l| l.pid)
}

#[cfg(test)]
pub fn is_free(port: u16) -> bool {
    owner(port).is_none()
}

/// Volný port podle OS (bind na :0).
pub fn ephemeral() -> Option<u16> {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
}

/// Jen testy: spustí `start(port)` na volném portu. Mezi `ephemeral()` (zjištění čísla) a bindem v `start`
/// může port zabrat jiný proces (souběžný build / jiný test, os error 10048) — pak se zkusí nový port, až
/// 10×. `start` musí při chybě nic nenechat běžet (`claude::serve` při selhání bindu nespustí vlákno).
#[cfg(test)]
pub fn with_free_port<T>(mut start: impl FnMut(u16) -> Result<T, String>) -> (u16, T) {
    let mut last = String::new();
    for _ in 0..10 {
        let port = ephemeral().expect("OS nedal volný port");
        match start(port) {
            Ok(v) => return (port, v),
            Err(e) => last = e,
        }
    }
    panic!("po 10 pokusech pořád žádný volný port: {last}")
}

/// Plná cesta k exe procesu (pro „port drží …“).
pub fn process_image(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Krátký popis pro UI: „node.exe (PID 1234)“.
pub fn describe(pid: u32) -> String {
    let name = process_image(pid)
        .and_then(|p| std::path::Path::new(&p).file_name().map(|f| f.to_string_lossy().into_owned()))
        .unwrap_or_else(|| crate::i18n::tr("neznámý proces", "unknown process").into());
    format!("{name} (PID {pid})")
}

/// TCP connect s timeoutem na 127.0.0.1 i [::1] — server může poslouchat jen na jednom z nich.
pub fn probe(port: u16) -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;
    let addrs: [SocketAddr; 2] = [([127, 0, 0, 1], port).into(), ("::1".parse::<std::net::Ipv6Addr>().unwrap(), port).into()];
    addrs.iter().any(|a| TcpStream::connect_timeout(a, Duration::from_millis(250)).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sees_own_listener_with_own_pid() {
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(owner(port), Some(std::process::id()));
        assert!(!is_free(port));
        assert!(probe(port));
        drop(l);
        assert!(is_free(port));
    }

    #[test]
    fn sees_ipv6_only_listener() {
        let Ok(l) = std::net::TcpListener::bind(("::1", 0)) else { return };
        let port = l.local_addr().unwrap().port();
        assert_eq!(owner(port), Some(std::process::id()));
        assert!(probe(port));
    }

    #[test]
    fn with_free_port_retries_when_the_port_is_taken() {
        // první dva porty „zabere někdo jiný“ (skutečný bind na obsazeném portu selže), třetí projde
        let mut taken = Vec::new();
        let mut tries = 0;
        let (port, listener) = with_free_port(|p| {
            tries += 1;
            if tries <= 2 {
                taken.push(std::net::TcpListener::bind(("127.0.0.1", p)).unwrap());
                return std::net::TcpListener::bind(("127.0.0.1", p)).map_err(|e| e.to_string());
            }
            std::net::TcpListener::bind(("127.0.0.1", p)).map_err(|e| e.to_string())
        });
        assert_eq!(tries, 3);
        assert_eq!(listener.local_addr().unwrap().port(), port);
        assert!(taken.iter().all(|t| t.local_addr().unwrap().port() != port));
    }

    #[test]
    #[should_panic(expected = "žádný volný port")]
    fn with_free_port_gives_up() {
        with_free_port::<()>(|_| Err("obsazeno".into()));
    }

    #[test]
    fn describes_own_process() {
        assert!(describe(std::process::id()).contains("PID"));
    }
}
