//! Návrat fokusu do aplikace, ze které si ho Ledge vzal (rychlá poznámka, paleta, Nastavení).
//!
//! Ledge bere fokus jen výslovně (zkratka / klik → `Drawer::focus`). Těsně předtím si tady zapamatuje
//! okno v popředí (`remember`). Výslovné zavření šuplíku (Esc, ✕, uložení poznámky, zkratka znovu —
//! `UiCmd::Close`) fokus vrátí (`give_back` z `Drawer::hide` / po přepnutí na peek); jakékoli jiné
//! zavření (`UiCmd::Hide`: spuštění akce / projektu / odkazu, které otevírá vlastní okno, blur, myš)
//! zapamatované okno zahodí (`on_close(false)`), aby nové okno fokus dostalo.
//!
//! Rozhodnutí je čisté (`decide`, `keep_or_take`, tabulkové testy); Win32 jen v `probe`/`give_back`.
//! Vrací se JEN, když je Ledge pořád v popředí (uživatel mezitím neklikl jinam) a zapamatované okno
//! pořád existuje, je viditelné a patří stejnému procesu (HWND se po zavření okna může recyklovat).

use parking_lot::Mutex;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SetForegroundWindow};

/// Okno, které mělo fokus, než si ho Ledge vzal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Remembered {
    pub hwnd: isize,
    pub pid: u32,
}

/// Stav systému v okamžiku zavření šuplíku (vše zjištěné Win32, v testech ručně).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Probe {
    /// proces Ledge
    pub ours: u32,
    /// proces okna v popředí (0 = žádné)
    pub fg_pid: u32,
    /// zapamatované okno pořád existuje, je viditelné a není minimalizované
    pub alive: bool,
    /// proces, kterému zapamatovaný HWND patří teď (0 = okno už není)
    pub pid_now: u32,
}

/// Při převzetí fokusu: koho si pamatovat. Je-li v popředí už Ledge (klik do poznámky v šuplíku
/// otevřeném zkratkou), nech původní okno; jinak si zapamatuj to v popředí. Prázdné popředí nic.
pub(crate) fn keep_or_take(current: Option<Remembered>, fg: isize, fg_pid: u32, ours: u32) -> Option<Remembered> {
    if fg == 0 || fg_pid == 0 || fg_pid == ours {
        return current;
    }
    Some(Remembered { hwnd: fg, pid: fg_pid })
}

/// Při výslovném zavření: komu fokus vrátit (`None` = nikomu, nech Windows).
pub(crate) fn decide(prev: Option<Remembered>, p: Probe) -> Option<isize> {
    let prev = prev?;
    if prev.pid == p.ours {
        return None; // nikdy „zpět“ do Ledge (proužek, vlastní dialog)
    }
    if p.fg_pid != p.ours {
        return None; // uživatel mezitím klikl jinam (nebo fokus má nové okno) — nekrást
    }
    if !p.alive || p.pid_now != prev.pid {
        return None; // okno zmizelo / je minimalizované / HWND teď patří jinému procesu
    }
    Some(prev.hwnd)
}

static PREV: Mutex<Option<Remembered>> = Mutex::new(None);

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(h, Some(&mut pid));
    }
    pid
}

/// Těsně před `SetForegroundWindow` na šuplík (`Drawer::focus`).
pub(crate) fn remember() {
    let fg = unsafe { GetForegroundWindow() };
    let fg_pid = if fg.0.is_null() { 0 } else { pid_of(fg) };
    let mut prev = PREV.lock();
    *prev = keep_or_take(*prev, fg.0 as isize, fg_pid, std::process::id());
}

/// Šuplík se zavírá: `give_back` = výslovné zavření (`UiCmd::Close`) → okno nechat pro `give_back()`;
/// jinak (spuštění, které otevírá jiné okno, blur, myš) zapomenout.
pub(crate) fn on_close(give_back: bool) {
    if !give_back {
        *PREV.lock() = None;
    }
}

fn probe(prev: Remembered) -> Probe {
    let h = HWND(prev.hwnd as *mut _);
    unsafe {
        let fg = GetForegroundWindow();
        let exists = IsWindow(Some(h)).as_bool();
        Probe {
            ours: std::process::id(),
            fg_pid: if fg.0.is_null() { 0 } else { pid_of(fg) },
            alive: exists && IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool(),
            pid_now: if exists { pid_of(h) } else { 0 },
        }
    }
}

/// Vrátí fokus zapamatovanému oknu, pokud `decide` dovolí. Volá ho `Drawer::hide` (po přepnutí
/// stavu na Hidden, ještě před schováním okna — Ledge je tak pořád v popředí a Windows to dovolí)
/// a `threads::ui` po přepnutí Full → peek. Zapamatované okno se tím vždy spotřebuje.
pub(crate) fn give_back() {
    let Some(prev) = PREV.lock().take() else { return };
    if let Some(h) = decide(Some(prev), probe(prev)) {
        let ok = unsafe { SetForegroundWindow(HWND(h as *mut _)).as_bool() };
        crate::log(&format!("fokus zpět do předchozího okna: {}", if ok { "ok" } else { "Windows odmítl" }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OURS: u32 = 100;
    const APP: Remembered = Remembered { hwnd: 0x1234, pid: 200 };

    fn probe(fg_pid: u32, alive: bool, pid_now: u32) -> Probe {
        Probe { ours: OURS, fg_pid, alive, pid_now }
    }

    #[test]
    fn decide_table() {
        // (zapamatováno, popředí, okno žije, pid okna teď) → vrátit?
        let cases: &[(&str, Option<Remembered>, Probe, Option<isize>)] = &[
            ("běžný případ: Ledge v popředí, okno žije", Some(APP), probe(OURS, true, 200), Some(0x1234)),
            ("nic zapamatováno", None, probe(OURS, true, 200), None),
            ("uživatel klikl jinam", Some(APP), probe(300, true, 200), None),
            ("fokus už má zapamatované okno samo", Some(APP), probe(200, true, 200), None),
            ("žádné okno v popředí", Some(APP), probe(0, true, 200), None),
            ("okno mezitím zmizelo", Some(APP), probe(OURS, false, 0), None),
            ("okno je minimalizované / skryté", Some(APP), probe(OURS, false, 200), None),
            ("HWND recyklovaný jiným procesem", Some(APP), probe(OURS, true, 999), None),
            ("zapamatované okno je Ledge", Some(Remembered { hwnd: 0x55, pid: OURS }), probe(OURS, true, OURS), None),
        ];
        for (name, prev, p, want) in cases {
            assert_eq!(decide(*prev, *p), *want, "{name}");
        }
    }

    #[test]
    fn keep_or_take_table() {
        let other = Remembered { hwnd: 0x99, pid: 300 };
        // (dosud, fg, pid fg) → nově
        type Case = (&'static str, Option<Remembered>, isize, u32, Option<Remembered>);
        let cases: &[Case] = &[
            ("cizí okno v popředí → zapamatovat", None, 0x1234, 200, Some(APP)),
            ("cizí okno přepíše staré", Some(other), 0x1234, 200, Some(APP)),
            ("v popředí už Ledge → nechat původní", Some(APP), 0x77, OURS, Some(APP)),
            ("v popředí Ledge, nic zapamatováno", None, 0x77, OURS, None),
            ("prázdné popředí → nechat", Some(APP), 0, 0, Some(APP)),
            ("okno bez procesu → nechat", None, 0x1234, 0, None),
        ];
        for (name, cur, fg, fg_pid, want) in cases {
            assert_eq!(keep_or_take(*cur, *fg, *fg_pid, OURS), *want, "{name}");
        }
    }
}
