//! Odolnost proti pádům (L225–L227): panic hook do `ledge.log`, dohled nad stálými vlákny
//! (restart se stropem, příznak „služba spadla“ pro UI), rotace logu, hlášení selhání startu a
//! `ago` — bezpečné „před N s“ nad `Instant` (krátký uptime). Žádný časovač ani polling: dohled je
//! jen `catch_unwind` kolem těla vlákna, spí se jen po pádu (backoff).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Log se při této velikosti přesune na `ledge.log.1` (starý `.1` se přepíše).
pub(crate) const LOG_MAX: u64 = 1 << 20;
/// Kolik pádů v okně ještě vlákno dostane restart; víc = vzdát a ukázat „služba spadla“.
pub(crate) const CAP: u32 = 5;
/// Vlákno, bez kterého UI nemá jak říct pravdu (`emit`) nebo se vůbec ukázat (`ui`, L284): restartuje
/// se vždy, jen s dlouhým backoffem.
pub(crate) const FOREVER: u32 = u32::MAX;
const WINDOW: Duration = Duration::from_secs(600);
const BASE_DELAY: Duration = Duration::from_millis(250);
const MAX_DELAY: Duration = Duration::from_secs(30);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Restart(Duration),
    GiveUp,
}

/// Politika restartu (čistá): zapíše pád `now` do `history`, zapomene pády starší než `window`
/// a rozhodne. Do `cap` pádů v okně = restart s backoffem 250 ms · 2^(n−1) (strop 30 s), jinak vzdát.
pub(crate) fn decide(history: &mut Vec<Instant>, now: Instant, cap: u32, window: Duration) -> Decision {
    history.retain(|&t| now.saturating_duration_since(t) < window);
    history.push(now);
    let n = history.len() as u32;
    if n > cap {
        return Decision::GiveUp;
    }
    let delay = BASE_DELAY.saturating_mul(1u32 << (n - 1).min(10));
    Decision::Restart(delay.min(MAX_DELAY))
}

/// `now − d` bez paniky: `Instant` je na Windows blízko bootu, takže `Instant::now() - 60 s` krátce po
/// startu podteče. Podtečení vrátí `now` (= „právě teď“, nejbližší reprezentovatelné). Čistá.
pub(crate) fn ago_from(now: Instant, d: Duration) -> Instant {
    now.checked_sub(d).unwrap_or(now)
}

pub(crate) fn ago(d: Duration) -> Instant {
    ago_from(Instant::now(), d)
}

// ---------- příznak „služba spadla“ ----------

static ANY_DOWN: AtomicBool = AtomicBool::new(false);
static DOWN: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

fn set_down(name: &'static str) {
    let mut d = DOWN.lock().unwrap_or_else(|e| e.into_inner());
    if !d.contains(&name) {
        d.push(name);
    }
    ANY_DOWN.store(true, Ordering::Relaxed);
}

/// Názvy spadlých služeb (`/status.qa.down`); bez pádu bez zámku.
pub(crate) fn down_names() -> Vec<&'static str> {
    if !ANY_DOWN.load(Ordering::Relaxed) {
        return Vec::new();
    }
    DOWN.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Věta pro banner chyb v UI (`config::recovery::shown_error`), dokud nějaká služba stojí; jinak `None`
/// (bez zámku a bez alokace).
pub(crate) fn down_message() -> Option<String> {
    if !ANY_DOWN.load(Ordering::Relaxed) {
        return None;
    }
    let d = DOWN.lock().unwrap_or_else(|e| e.into_inner());
    let names = d.join(", ");
    Some(trf!("Služba „{names}“ spadla a neběží — data nemusí být aktuální. Restartuj Ledge (podrobnosti v ledge.log).", "The “{names}” service crashed and is not running — data may be stale. Restart Ledge (details in ledge.log)."))
}

// ---------- dohled ----------

/// Pustí `body` pod `catch_unwind`; po pádu ho (po backoffu) spustí znovu, nejvýš `cap`× za 10 min.
/// Pak se vzdá, označí službu jako spadlou (banner v UI) a vrátí se. Normální návrat těla = konec vlákna.
/// Tělo si stav staví samo (nové volání = čistý start); sdílené věci (přijímač kanálu) drží vně a půjčuje.
pub(crate) fn guarded(name: &'static str, cap: u32, body: impl FnMut()) {
    supervise(name, cap, body, std::thread::sleep, crate::state::mark_dirty, crate::state::log);
}

fn supervise(name: &'static str, cap: u32, mut body: impl FnMut(), sleep: impl Fn(Duration), on_down: impl Fn(), log: impl Fn(&str)) {
    let mut history = Vec::new();
    loop {
        if catch_unwind(AssertUnwindSafe(&mut body)).is_ok() {
            return;
        }
        match decide(&mut history, Instant::now(), cap, WINDOW) {
            Decision::Restart(d) => {
                log(&format!("vlákno {name} spadlo, restart za {} ms ({}/{})", d.as_millis(), history.len(), cap));
                sleep(d);
            }
            Decision::GiveUp => {
                log(&format!("vlákno {name} spadlo {} × za {} s — vzdávám restart", history.len(), WINDOW.as_secs()));
                set_down(name);
                on_down();
                return;
            }
        }
    }
}

// ---------- panic hook, log, start ----------

/// Panic hook: místo, zpráva a stack do `ledge.log` (release nemá konzoli, stderr nikdo nevidí), pak
/// původní hook. Volat co nejdřív v `run`.
pub(crate) fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let t = std::thread::current();
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let bt = std::backtrace::Backtrace::force_capture();
        crate::state::log(&format!("PANIKA ve vlákně '{}' {place}: {msg}\n{bt}", t.name().unwrap_or("?")));
        prev(info);
    }));
}

/// Je log větší než strop (čistá)? Pak se před dalším zápisem přesune na `.1`.
pub(crate) fn needs_rotation(len: u64) -> bool {
    len > LOG_MAX
}

/// Rotace `ledge.log` → `ledge.log.1` (přepíše starou kopii; příčina pádu tak zůstane aspoň v jednom ze dvou souborů).
pub(crate) fn rotate_log(path: &std::path::Path) {
    if std::fs::metadata(path).is_ok_and(|m| needs_rotation(m.len())) {
        let mut old = path.as_os_str().to_owned();
        old.push(".1");
        let _ = std::fs::rename(path, old);
    }
}

/// Start Tauri selhal (typicky chybí WebView2): záznam do logu, okno se zprávou, konec s kódem 1.
pub(crate) fn start_failed(err: &str) -> ! {
    crate::state::log(&format!("start selhal: {err}"));
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::HSTRING;
    let text = trf!("Ledge se nepodařilo spustit.\n\n{err}\n\nZkontroluj, že je nainstalovaný Microsoft WebView2 Runtime. Podrobnosti jsou v ledge.log.", "Ledge could not start.\n\n{err}\n\nCheck that the Microsoft WebView2 Runtime is installed. Details are in ledge.log.");
    unsafe {
        MessageBoxW(None, &HSTRING::from(text), &HSTRING::from("Ledge"), MB_OK | MB_ICONERROR);
    }
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn at(base: Instant, s: u64) -> Instant {
        base + Duration::from_secs(s)
    }

    #[test]
    fn politika_restartu_tabulka() {
        let b = Instant::now();
        let w = Duration::from_secs(600);
        // (časy pádů v s, cap, očekávané rozhodnutí posledního pádu)
        let rows: [(&[u64], u32, Decision); 7] = [
            (&[0], 5, Decision::Restart(Duration::from_millis(250))),
            (&[0, 1], 5, Decision::Restart(Duration::from_millis(500))),
            (&[0, 1, 2, 3, 4], 5, Decision::Restart(Duration::from_millis(4000))),
            (&[0, 1, 2, 3, 4, 5], 5, Decision::GiveUp),
            // staré pády vypadnou z okna → znovu první pád
            (&[0, 1, 2, 3, 4, 700], 5, Decision::Restart(Duration::from_millis(250))),
            // backoff má strop 30 s
            (&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], u32::MAX, Decision::Restart(Duration::from_secs(30))),
            (&[0], 0, Decision::GiveUp),
        ];
        for (times, cap, want) in rows {
            let mut h = Vec::new();
            let mut got = None;
            for &t in times {
                got = Some(decide(&mut h, at(b, t), cap, w));
            }
            assert_eq!(got, Some(want), "pády {times:?}, cap {cap}");
        }
    }

    #[test]
    fn guarded_po_panice_zavola_telo_znovu() {
        let calls = Cell::new(0);
        let slept = RefCell::new(Vec::new());
        supervise(
            "test-ok",
            CAP,
            || {
                calls.set(calls.get() + 1);
                if calls.get() < 3 {
                    panic!("test: zamýšlená panika");
                }
            },
            |d| slept.borrow_mut().push(d),
            || panic!("nemá se vzdát"),
            |_| {},
        );
        assert_eq!(calls.get(), 3);
        assert_eq!(*slept.borrow(), vec![Duration::from_millis(250), Duration::from_millis(500)]);
    }

    #[test]
    fn guarded_po_stropu_vzda_a_oznaci_sluzbu() {
        let calls = Cell::new(0);
        let down = Cell::new(false);
        supervise(
            "test-down",
            2,
            || {
                calls.set(calls.get() + 1);
                panic!("test: zamýšlená panika")
            },
            |_| {},
            || down.set(true),
            |_| {},
        );
        assert_eq!(calls.get(), 3); // 1. běh + 2 restarty, třetí pád už je nad stropem
        assert!(down.get());
        set_down("test-down");
        assert!(down_message().is_some_and(|m| m.contains("test-down")));
    }

    #[test]
    fn ago_nepodtece() {
        let now = Instant::now();
        assert_eq!(ago_from(now, Duration::from_secs(0)), now);
        // dostatečně velké „před“ podteče na každém stroji → vrátí now, nespadne
        assert_eq!(ago_from(now, Duration::from_secs(u64::MAX / 4)), now);
        let later = now + Duration::from_secs(120);
        assert_eq!(ago_from(later, Duration::from_secs(60)), now + Duration::from_secs(60));
    }

    #[test]
    fn rotace_logu_az_nad_stropem() {
        assert!(!needs_rotation(LOG_MAX));
        assert!(needs_rotation(LOG_MAX + 1));
        let dir = std::env::temp_dir().join(format!("ledge-guard-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("ledge.log");
        std::fs::write(dir.join("ledge.log.1"), b"stara").unwrap();
        std::fs::write(&p, vec![b'x'; LOG_MAX as usize + 1]).unwrap();
        rotate_log(&p);
        assert!(!p.exists());
        assert_eq!(std::fs::metadata(dir.join("ledge.log.1")).unwrap().len(), LOG_MAX + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
