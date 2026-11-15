//! Hlídač dvou instancí přes hranici verzí. Zámek jedné instance (`single_instance` v `lib.rs`) je podle
//! identifikátoru, takže Ledge se starším identifikátorem a nové se navzájem nevidí a prály by se o port mostu
//! (výchozí 47800, `claude/bridge.rs`). Hlídač proto při startu zjistí, kdo port drží: proces **se stejným názvem
//! exe** (jiný PID) = jiná instance Ledge. Nikdy nezabíjí cizí proces. Rozhodnutí je čistá funkce [`decide`].
//!
//! - nikdo / my sami / cizí program (ne Ledge) → pokračovat (cizí obsazený port ohlásí `bridge_error`)
//! - Ledge → čekat ≤ 15 s po 250 ms (po aktualizaci instalátor starý proces právě zavírá), pak skončit jako druhé
//!   spuštění. Předání příkazu „otevři šuplík“ starší instanci nejde: `/ctl/*` chce klíč hooků.
//! - Start STARÉ verze při běžící nové ošetřit nelze (stará hlídač nemá): její most se nespustí (port je obsazený)
//!   a obě okna běží — proto instalátor starou před instalací zavírá.

use std::path::Path;

pub const WAIT_MS: u64 = 15_000;
const POLL_MS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Holder {
    pub pid: u32,
    /// proces má stejný název exe jako my
    pub same_image: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Proceed,
    Wait,
    Yield,
}

pub fn decide(holder: Option<Holder>, own_pid: u32, waited_ms: u64, max_ms: u64) -> Verdict {
    match holder {
        None => Verdict::Proceed,
        Some(h) if h.pid == own_pid || !h.same_image => Verdict::Proceed,
        Some(_) if waited_ms < max_ms => Verdict::Wait,
        Some(_) => Verdict::Yield,
    }
}

/// Stejný název exe (bez ohledu na složku a velikost písmen)? Neznámý název = ne.
pub fn same_image(own: Option<&str>, theirs: Option<&str>) -> bool {
    let name = |p: &str| Path::new(p).file_name().map(|f| f.to_string_lossy().to_lowercase());
    match (own.and_then(name), theirs.and_then(name)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Zkoumá port; vrací verdikt a řádek do logu (jen když se čekalo nebo se končí).
pub fn await_predecessor(port: u16) -> (Verdict, Option<String>) {
    let own_pid = std::process::id();
    let own = std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned());
    run(own_pid, WAIT_MS, &|| {
        crate::ports::owner(port).map(|pid| Holder { pid, same_image: same_image(own.as_deref(), crate::ports::process_image(pid).as_deref()) })
    }, &|ms| std::thread::sleep(std::time::Duration::from_millis(ms)), port)
}

/// Smyčka nad podvrženým zdrojem držitele (testovatelná).
fn run(own_pid: u32, max_ms: u64, holder: &dyn Fn() -> Option<Holder>, sleep: &dyn Fn(u64), port: u16) -> (Verdict, Option<String>) {
    let mut waited = 0;
    loop {
        match decide(holder(), own_pid, waited, max_ms) {
            Verdict::Wait => {
                sleep(POLL_MS);
                waited += POLL_MS;
            }
            Verdict::Proceed => {
                let line = (waited > 0).then(|| format!("twoinst: port {port} se uvolnil po {waited} ms, start pokračuje"));
                return (Verdict::Proceed, line);
            }
            Verdict::Yield => {
                return (Verdict::Yield, Some(format!("twoinst: port {port} drží jiná instance Ledge ani po {waited} ms — tohle spuštění končí")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn h(pid: u32, same: bool) -> Option<Holder> {
        Some(Holder { pid, same_image: same })
    }

    #[test]
    fn decide_table() {
        for (holder, waited, want) in [
            (None, 0, Verdict::Proceed),
            (h(7, true), 0, Verdict::Wait),
            (h(7, true), 14_999, Verdict::Wait),
            (h(7, true), 15_000, Verdict::Yield),
            (h(1, true), 99_999, Verdict::Proceed), // my sami
            (h(7, false), 0, Verdict::Proceed),     // cizí program
            (h(7, false), 99_999, Verdict::Proceed),
        ] {
            assert_eq!(decide(holder, 1, waited, WAIT_MS), want, "{holder:?} {waited}");
        }
    }

    #[test]
    fn same_image_table() {
        for (a, b, want) in [
            (Some("C:\\A\\ledge.exe"), Some("D:\\x\\Ledge.EXE"), true),
            (Some("C:\\A\\ledge.exe"), Some("C:\\A\\node.exe"), false),
            (None, Some("ledge.exe"), false),
            (Some("ledge.exe"), None, false),
        ] {
            assert_eq!(same_image(a, b), want, "{a:?} {b:?}");
        }
    }

    #[test]
    fn predecessor_leaves_in_time() {
        let n = Cell::new(0);
        let slept = Cell::new(0);
        let (v, line) = run(1, WAIT_MS, &|| { n.set(n.get() + 1); if n.get() <= 4 { h(7, true) } else { None } }, &|ms| slept.set(slept.get() + ms), 47800);
        assert_eq!(v, Verdict::Proceed);
        assert_eq!(slept.get(), 4 * POLL_MS);
        assert!(line.unwrap().contains("uvolnil"));
    }

    #[test]
    fn predecessor_stays_so_we_yield() {
        let slept = Cell::new(0);
        let (v, line) = run(1, WAIT_MS, &|| h(7, true), &|ms| slept.set(slept.get() + ms), 47800);
        assert_eq!((v, slept.get()), (Verdict::Yield, WAIT_MS));
        assert!(line.unwrap().contains("končí"));
    }

    #[test]
    fn free_port_costs_nothing() {
        let slept = Cell::new(0);
        let (v, line) = run(1, WAIT_MS, &|| None, &|ms| slept.set(slept.get() + ms), 47800);
        assert_eq!((v, slept.get(), line), (Verdict::Proceed, 0, None));
    }
}
