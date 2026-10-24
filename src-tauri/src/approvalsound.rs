//! Zvuk u schvalování a otázky Clauda (`smart.approvalSound`, výchozí VYPNUTO): jedno tiché systémové
//! pípnutí (`MessageBeep` s „Hvězdičkou“ — hlasitost a zvuk určuje Windows, Ledge žádný soubor ani
//! přehrávač nepřidává), když přijde NOVÉ schválení nebo otázka (`attention::on_change`, stejná událost jako
//! puls proužku). Ztlumené, když by zvuk jen rušil: Windows nepřijímá oznámení (hra, prezentace, Nerušit —
//! QUNS), Ledge je mrtvý (fullscreen hra), šuplík je otevřený naplno (člověk se dívá), nebo je v popředí
//! terminál / okno Claude (člověk to vidí sám). Mez frekvence: nejméně `minGapS` sekund mezi dvěma zvuky.
//!
//! Rozhodnutí je čistá funkce `decide` (tabulkový test); systémová volání jen tehdy, když je volba zapnutá.
//! Žádné vlákno, žádný časovač — vše z události.

use parking_lot::Mutex;
use std::time::Instant;

/// Programy, jejichž okno v popředí zvuk ztlumí: terminály a okno Claude (člověk Clauda vidí).
const QUIET_APPS: &[&str] = &[
    "windowsterminal.exe",
    "wt.exe",
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "conhost.exe",
    "openconsole.exe",
    "wezterm-gui.exe",
    "alacritty.exe",
    "mintty.exe",
    "claude.exe",
];

/// Je to terminál nebo okno Claude? (název exe bez ohledu na velikost písmen; prázdný = neznámý = ne)
pub(crate) fn quiet_app(exe: &str) -> bool {
    let e = exe.trim().to_ascii_lowercase();
    !e.is_empty() && QUIET_APPS.contains(&e.as_str())
}

/// Vstupy rozhodnutí.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Gate {
    pub(crate) enabled: bool,
    /// Ledge je mrtvý (fullscreen hra / herní režim)
    pub(crate) dead: bool,
    /// Windows oznámení právě nepřijímá (QUNS ≠ ACCEPTS_NOTIFICATIONS)
    pub(crate) os_quiet: bool,
    /// šuplík je otevřený naplno
    pub(crate) drawer_full: bool,
    /// v popředí terminál nebo okno Claude
    pub(crate) app_quiet: bool,
    /// ms od posledního zvuku; `None` = ještě žádný
    pub(crate) since_last_ms: Option<u64>,
    /// nejmenší odstup (ms)
    pub(crate) min_gap_ms: u64,
}

/// Zahrát teď? Vypnuto / mrtvý / tichý systém / šuplík / terminál / příliš brzy po minulém = ne.
pub(crate) fn decide(g: Gate) -> bool {
    g.enabled && !g.dead && !g.os_quiet && !g.drawer_full && !g.app_quiet && g.since_last_ms.is_none_or(|t| t >= g.min_gap_ms)
}

static LAST: Mutex<Option<Instant>> = Mutex::new(None);

/// Přišlo nové schválení / otázka Clauda (volá `attention::on_change`). Zjistí brány až když je volba
/// zapnutá, pak případně pípne.
pub(crate) fn on_needs_input() {
    use crate::drawer::View;
    let c = crate::state::core();
    let (enabled, gap_s) = {
        let cfg = c.cfg.read();
        (cfg.smart.approval_sound.enabled, cfg.smart.approval_sound.min_gap_s)
    };
    if !enabled {
        return;
    }
    let dead = c.dead();
    let mut gate = Gate { enabled, dead, drawer_full: c.drawer.view() == View::Full, min_gap_ms: gap_s as u64 * 1000, ..Gate::default() };
    gate.since_last_ms = LAST.lock().map(|t| t.elapsed().as_millis() as u64);
    // systémová volání jen když na nich rozhodnutí ještě závisí
    if !dead && !gate.drawer_full && gate.since_last_ms.is_none_or(|t| t >= gate.min_gap_ms) {
        gate.os_quiet = !crate::toast::accepts_notifications();
        if !gate.os_quiet {
            let mut fg = crate::screen::ForegroundProbe::default();
            gate.app_quiet = fg.read().is_some_and(|f| quiet_app(f.exe));
        }
    }
    if decide(gate) {
        *LAST.lock() = Some(Instant::now());
        beep();
    }
}

/// Tiché systémové pípnutí (zvuk „Hvězdička“ z nastavení zvuků Windows).
fn beep() {
    use windows::Win32::System::Diagnostics::Debug::MessageBeep;
    use windows::Win32::UI::WindowsAndMessaging::MB_ICONASTERISK;
    let _ = unsafe { MessageBeep(MB_ICONASTERISK) };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> Gate {
        Gate { enabled: true, min_gap_ms: 30_000, ..Gate::default() }
    }

    #[test]
    fn decide_table() {
        let cases: [(&str, Gate, bool); 10] = [
            ("zapnuto, první zvuk", on(), true),
            ("vypnuto (výchozí)", Gate { enabled: false, ..on() }, false),
            ("mrtvý Ledge (hra)", Gate { dead: true, ..on() }, false),
            ("Windows nepřijímá oznámení (Nerušit / hra)", Gate { os_quiet: true, ..on() }, false),
            ("šuplík otevřený naplno", Gate { drawer_full: true, ..on() }, false),
            ("v popředí terminál / Claude", Gate { app_quiet: true, ..on() }, false),
            ("20 s po minulém zvuku", Gate { since_last_ms: Some(20_000), ..on() }, false),
            ("přesně na mezi", Gate { since_last_ms: Some(30_000), ..on() }, true),
            ("dlouho po minulém", Gate { since_last_ms: Some(300_000), ..on() }, true),
            ("vše naráz", Gate { dead: true, os_quiet: true, drawer_full: true, app_quiet: true, since_last_ms: Some(0), ..on() }, false),
        ];
        for (name, g, want) in cases {
            assert_eq!(decide(g), want, "{name}");
        }
    }

    #[test]
    fn quiet_apps_by_name() {
        let cases = [("WindowsTerminal.exe", true), ("pwsh.exe", true), ("Claude.exe", true), ("claude.exe ", true), ("chrome.exe", false), ("Code.exe", false), ("", false)];
        for (exe, want) in cases {
            assert_eq!(quiet_app(exe), want, "{exe:?}");
        }
    }
}
