//! První spuštění: šuplík se NOVÉMU uživateli jednou otevře sám a ukáže Vítej — jinak by o Ledge nevěděl
//! (tenký proužek u okraje nikdo nehledá). Otevře se jako `/ctl/open` (`Opener::Remote`: bez fokusu,
//! nic nekrade), jen když neběží fullscreen hra ani herní režim. „Jednou“ drží značka vedle configu
//! ([`FILE`]); config sám se kvůli tomu nemění. Rozhodnutí je čistá funkce [`should_open`].

use crate::drawer::{Opener, View};
use crate::state::{UiCmd, core, log};
use std::path::Path;
use std::time::Duration;

/// Značka „už jsem šuplík jednou sám otevřel“ (v `config::config_dir`).
pub const FILE: &str = "first-run-shown";

/// Počkat po startu (studený WebView2 + předehřátí šuplíku), než se šuplík otevře.
const DELAY: Duration = Duration::from_millis(4000);
/// Když zrovna běží hra / je zapnutý herní režim, zkusit to znovu za tuhle dobu…
const RETRY: Duration = Duration::from_secs(30);
/// …nejvýš tolikrát (pak to vzdát — nikdy nevyrušit později).
const MAX_RETRIES: u32 = 10;

/// Co se má stát při pokusu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// otevřít šuplík a poznamenat si to
    Open,
    /// teď ne (hra), zkusit později
    Later,
    /// už nikdy (průvodce proběhl, už jsme otevřeli, nebo šuplík uživatel otevřel sám)
    Never,
}

/// Rozhodnutí. `done` = `onboarding.done`, `shown` = značka existuje, `dead` = fullscreen hra / herní režim,
/// `open_now` = šuplík už je vidět (uživatel ho otevřel sám a průvodce mu naskočil). Čistá funkce.
pub fn should_open(done: bool, shown: bool, dead: bool, open_now: bool) -> Verdict {
    if done || shown || open_now {
        Verdict::Never
    } else if dead {
        Verdict::Later
    } else {
        Verdict::Open
    }
}

pub fn was_shown(dir: &Path) -> bool {
    dir.join(FILE).is_file()
}

pub fn mark_shown(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(FILE), b"1\n")
}

/// Naplánuje první otevření (volá `setup`): vlákno jen spí a po pokusu skončí; bez nového uživatele skončí hned.
pub fn schedule() {
    let dir = crate::config::config_dir();
    if core().cfg.read().onboarding.done || was_shown(&dir) {
        return;
    }
    std::thread::Builder::new()
        .name("first-run".into())
        .spawn(move || {
            std::thread::sleep(DELAY);
            for _ in 0..=MAX_RETRIES {
                let c = core();
                let done = c.cfg.read().onboarding.done;
                match should_open(done, was_shown(&dir), c.dead(), c.drawer.view() != View::Hidden) {
                    Verdict::Open => {
                        if let Err(e) = mark_shown(&dir) {
                            log(&format!("první spuštění: značka nejde uložit: {e}"));
                        }
                        log("první spuštění: otevírám šuplík s Vítej (bez fokusu)");
                        c.send_ui(UiCmd::Show(View::Full, Opener::Remote));
                        return;
                    }
                    Verdict::Never => return,
                    Verdict::Later => std::thread::sleep(RETRY),
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_table() {
        use Verdict::*;
        // (průvodce hotov, už otevřeno, hra, šuplík vidět) → rozhodnutí
        let cases = [
            (false, false, false, false, Open),
            (false, false, true, false, Later),
            (true, false, false, false, Never),
            (true, false, true, false, Never),
            (false, true, false, false, Never),
            (false, true, true, false, Never),
            (false, false, false, true, Never),
            (false, false, true, true, Never),
        ];
        for (done, shown, dead, open_now, want) in cases {
            assert_eq!(should_open(done, shown, dead, open_now), want, "done {done}, shown {shown}, dead {dead}, open {open_now}");
        }
    }

    #[test]
    fn marker_persists_in_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(!was_shown(d.path()));
        mark_shown(d.path()).unwrap();
        assert!(was_shown(d.path()));
        mark_shown(d.path()).unwrap();
        let nested = d.path().join("nova");
        mark_shown(&nested).unwrap();
        assert!(was_shown(&nested), "složka se založí");
        assert!(!was_shown(&d.path().join("jina")));
    }
}
