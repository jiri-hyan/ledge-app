//! Pojistky vlákna `hover` (kolo 22): najetí k hraně NIKDY neotevře šuplík, když hra / přehrávač kurzor
//! skryl nebo zamkl do svého okna (`ClipCursor`) — kurzor ve hře v okně „uvízne“ u horní hrany. Fullscreen
//! a herní režim řeší `Core::dead` (`gamemode`). Druhá pojistka: změna monitoru / DPI / rozlišení =
//! proužek se hned přepočítá (`display_changed`), ne až při další události.
//! Čtení je levné (dvě volání WinAPI) a jen s kurzorem u hrany v zóně proužku.

use crate::screen::Monitor;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    CURSOR_SHOWING, CURSOR_SUPPRESSED, CURSORINFO, GetClipCursor, GetCursorInfo, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// Je kurzor „vidět“ podle `CURSORINFO.flags`? Skrytý (`flags == 0`: hra / přehrávač ho schovaly) vidět není;
/// `CURSOR_SUPPRESSED` (dotyk / pero — systém kurzor jen dočasně nekreslí) je běžná práce, ne skrytí.
pub fn cursor_visible(flags: u32) -> bool {
    flags & (CURSOR_SHOWING.0 | CURSOR_SUPPRESSED.0) != 0
}

/// Smí najetí otevírat? Ne, když kurzor není vidět (`showing` false: skrytý hrou / přehrávačem) nebo je zamčený do menšího obdélníku než celá plocha (`clip` ⊊ `desktop`; prázdný clip = nic).
pub fn pointer_blocked(showing: bool, clip: RECT, desktop: RECT) -> bool {
    if !showing {
        return true;
    }
    let real = clip.right > clip.left && clip.bottom > clip.top;
    real && (clip.left > desktop.left || clip.top > desktop.top || clip.right < desktop.right || clip.bottom < desktop.bottom)
}

/// Změnil se monitor proužku (jiný monitor, rozlišení, poloha, pracovní plocha, DPI)? Pak se proužek přepočítá.
pub fn display_changed(old: &Monitor, new: &Monitor) -> bool {
    old.handle != new.handle || old.rect != new.rect || old.work != new.work || old.dpi != new.dpi
}

/// Aktuální stav kurzoru z Windows (`GetCursorInfo` + `GetClipCursor` + virtuální plocha).
pub fn blocked_now() -> bool {
    unsafe {
        let mut ci = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
        // chyba čtení = nevíme → neblokovat (dnešní chování)
        let showing = GetCursorInfo(&mut ci).is_err() || cursor_visible(ci.flags.0);
        let mut clip = RECT::default();
        if GetClipCursor(&mut clip).is_err() {
            clip = RECT::default();
        }
        let (x, y) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
        let desktop = RECT { left: x, top: y, right: x + GetSystemMetrics(SM_CXVIRTUALSCREEN), bottom: y + GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        let blocked = pointer_blocked(showing, clip, desktop);
        // jeden řádek při přechodu na „zablokováno“ (ne při každém pohybu myši u hrany)
        if blocked != BLOCKED.swap(blocked, Ordering::Relaxed) && blocked {
            crate::log(if showing { "najetí k hraně ignorováno: kurzor je zamčený do okna (ClipCursor)" } else { "najetí k hraně ignorováno: kurzor je skrytý (hra / přehrávač)" });
        }
        blocked
    }
}

/// Naposledy hlášený stav (aby se do logu psal jen přechod, ne každé čtení).
static BLOCKED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use super::*;

    const R: fn(i32, i32, i32, i32) -> RECT = |l, t, r, b| RECT { left: l, top: t, right: r, bottom: b };

    #[test]
    fn pointer_blocked_table() {
        // dva monitory: virtuální plocha od −1920
        let desk = R(-1920, 0, 1920, 1080);
        // (popis, kurzor vidět, clip) → blokováno
        let cases = [
            ("běžná práce: vidět, clip = celá plocha", true, desk, false),
            ("clip ještě nečten (prázdný)", true, RECT::default(), false),
            ("hra skryla kurzor", false, desk, true),
            ("skrytý i bez clipu", false, RECT::default(), true),
            ("hra v okně zamkla kurzor do okna", true, R(100, 0, 1380, 720), true),
            ("zamčeno na jeden monitor (fullscreen okno)", true, R(0, 0, 1920, 1080), true),
            ("clip větší než plocha (nikdy neblokuje)", true, R(-3000, -10, 4000, 2000), false),
        ];
        for (name, showing, clip, want) in cases {
            assert_eq!(pointer_blocked(showing, clip, desk), want, "{name}");
        }
    }

    #[test]
    fn cursor_visible_table() {
        // (popis, flags z GetCursorInfo) → vidět
        let cases = [
            ("běžný kurzor", CURSOR_SHOWING.0, true),
            ("dotyk / pero: systém kurzor potlačil", CURSOR_SUPPRESSED.0, true),
            ("obojí", CURSOR_SHOWING.0 | CURSOR_SUPPRESSED.0, true),
            ("skrytý hrou / přehrávačem", 0, false),
        ];
        for (name, flags, want) in cases {
            assert_eq!(cursor_visible(flags), want, "{name}");
            // a z toho plyne blokování (bez clipu): jen skutečně skrytý kurzor
            assert_eq!(pointer_blocked(cursor_visible(flags), RECT::default(), R(0, 0, 1920, 1080)), !want, "{name}");
        }
    }

    #[test]
    fn display_change_table() {
        let m = Monitor { handle: 1, rect: R(0, 0, 1920, 1080), work: R(0, 0, 1920, 1040), primary: true, dpi: 96 };
        assert!(!display_changed(&m, &m), "stejný monitor = nic");
        let cases = [
            ("DPI 100 → 150 %", Monitor { dpi: 144, ..m }),
            ("rozlišení", Monitor { rect: R(0, 0, 2560, 1440), ..m }),
            ("jiný monitor", Monitor { handle: 2, ..m }),
            ("hlavní panel nahoře / skrytý", Monitor { work: R(0, 40, 1920, 1080), ..m }),
            ("monitor přesunutý vlevo", Monitor { rect: R(-1920, 0, 0, 1080), ..m }),
        ];
        for (name, n) in cases {
            assert!(display_changed(&m, &n), "{name}");
        }
    }
}
