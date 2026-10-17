//! Útrata Claude za běhu Ledge: přepočet indexu (`usage::index`) v samostatném vlákně s nízkou
//! prioritou (CPU i I/O, `THREAD_MODE_BACKGROUND_BEGIN`) a přehled v paměti pro snapshot.
//!
//! Žádný sken v klidu: přepočet jen na žádost (`request` — UI při rozbalení sekce / otevření šuplíku
//! s rozbalenou sekcí), nejvýš 1× za `THROTTLE`, nikdy dva naráz. Vlákno po doběhu skončí; index
//! zůstane na disku a v paměti jen hotový přehled (`Summary`). `view` čte jen paměť.

use crate::state::{core, log, mark_dirty, now_ms};
use crate::usage::{WINDOW_DAYS, index, price, view};
use parking_lot::Mutex;
use serde::Serialize;
use std::time::{Duration, Instant};

/// Nejkratší rozestup dvou přepočtů (od konce předchozího).
const THROTTLE: Duration = Duration::from_secs(60);
/// Průběh do UI nejvýš takhle často.
const PROGRESS_EVERY: Duration = Duration::from_millis(700);

#[derive(Default)]
struct Live {
    running: bool,
    last_start: Option<Instant>,
    /// (přečteno, celkem) bajtů běžícího přepočtu
    progress: Option<(u64, u64)>,
    summary: Option<view::Summary>,
    computed_at: Option<i64>,
    error: Option<String>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    let mut g = LIVE.lock();
    f(g.get_or_insert_with(Live::default))
}

fn enabled() -> bool {
    core().cfg.read().claude.costs
}

/// Požádá o přepočet; běží-li nebo proběhl před méně než `THROTTLE`, nic. Vypnutá karta = nic.
pub(crate) fn request() {
    if !enabled() {
        return;
    }
    let go = with(|l| {
        if l.running || l.last_start.is_some_and(|t| t.elapsed() < THROTTLE) {
            return false;
        }
        l.running = true;
        l.last_start = Some(Instant::now());
        l.progress = None;
        true
    });
    if !go {
        return;
    }
    mark_dirty();
    let spawned = std::thread::Builder::new().name("costs".into()).spawn(|| {
        background();
        let res = std::panic::catch_unwind(compute);
        with(|l| {
            l.running = false;
            l.progress = None;
            // rozestup od konce běhu (první plný přepočet může trvat desítky sekund)
            l.last_start = Some(Instant::now());
            match res {
                Ok(Ok(s)) => {
                    l.summary = Some(s);
                    l.computed_at = Some(now_ms());
                    l.error = None;
                }
                Ok(Err(e)) => l.error = Some(e),
                Err(_) => l.error = Some(crate::i18n::tr("Přepočet útraty spadl.", "Usage recount crashed.").into()),
            }
        });
        mark_dirty();
    });
    if spawned.is_err() {
        with(|l| l.running = false);
    }
}

/// Nízká priorita CPU i disku pro aktuální vlákno.
fn background() {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN};
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
    }
}

/// Místní den (dny od 1970-01-01) z ms.
pub(crate) fn local_day(ms: i64) -> i32 {
    use chrono::{Local, TimeZone};
    Local.timestamp_millis_opt(ms).single().map_or((ms / 86_400_000) as i32, |d| (d.date_naive() - chrono::NaiveDate::default()).num_days() as i32)
}

/// Místní půlnoc dne `day` v ms (při přechodu času nejdřívější možná).
fn local_midnight(day: i32) -> i64 {
    use chrono::{Local, TimeZone};
    let d = chrono::NaiveDate::default() + chrono::Duration::days(day as i64);
    Local.from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap_or_default()).earliest().map_or(day as i64 * 86_400_000, |t| t.timestamp_millis())
}

fn compute() -> Result<view::Summary, String> {
    let t0 = Instant::now();
    let Some(root) = crate::sessionlive::claude_dir().map(|d| d.join("projects")) else {
        return Err(crate::i18n::tr("Nevím, kde je složka .claude.", "I don't know where the .claude folder is.").into());
    };
    let store = crate::actionrun::local_dir();
    let now = now_ms();
    let today = local_day(now);
    let cutoff_day = today - WINDOW_DAYS as i32 + 1;
    let w = index::Window { days: WINDOW_DAYS, cutoff_ms: local_midnight(cutoff_day), cutoff_day, day_of: &local_day };
    let mut ix = index::load(&store);
    let found = index::list_files(&root);
    let mut last = Instant::now();
    let st = ix.update(&found, &w, &mut |done, total| {
        if last.elapsed() >= PROGRESS_EVERY || done == total {
            last = Instant::now();
            with(|l| l.progress = Some((done, total)));
            mark_dirty();
        }
    });
    if let Err(e) = index::save(&store, &ix) {
        log(&format!("útrata: index nejde uložit: {e}"));
    }
    let dirs = project_dirs();
    let summary = view::build(&ix, today, WINDOW_DAYS, &|cwd| project_name(&dirs, cwd));
    log(&format!(
        "útrata: {} ms, souborů {} (staré {}, beze změny {}, čteno {}), {} MB, odpovědí {} (+{} duplicit){}, index {} odpovědí",
        t0.elapsed().as_millis(),
        st.files,
        st.skipped_old,
        st.unchanged,
        st.opened,
        st.bytes / 1_000_000,
        st.records,
        st.duplicates,
        if st.rebuilt { ", přepočet od nuly" } else { "" },
        ix.seen.len()
    ));
    Ok(summary)
}

/// (název projektu, složka) z configu.
fn project_dirs() -> Vec<(String, String)> {
    let c = core();
    let cfg = c.cfg.read().clone();
    cfg.projects.iter().filter_map(|p| Some((p.name.clone(), c.project_dir(p)?.to_string_lossy().into_owned()))).collect()
}

/// Projekt podle `cwd` (i worktree → hlavní checkout); jinak název složky; bez cwd prázdno.
fn project_name(dirs: &[(String, String)], cwd: &str) -> String {
    if cwd.is_empty() {
        return String::new();
    }
    if let Some(n) = crate::notes::project_for_cwd(dirs, cwd) {
        return n.to_string();
    }
    if let Some(main) = crate::resolve::main_checkout(std::path::Path::new(cwd))
        && let Some(n) = crate::notes::project_for_cwd(dirs, &main.to_string_lossy())
    {
        return n.to_string();
    }
    cwd.trim_end_matches(['\\', '/']).rsplit(['\\', '/']).next().unwrap_or(cwd).to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostsView {
    /// off (vypnuto v Nastavení) · idle (ještě nepočítáno) · computing · ok · error
    pub state: &'static str,
    /// průběh přepočtu 0–1 (jen `computing`)
    pub progress: Option<f64>,
    pub computed_at: Option<i64>,
    pub error: Option<String>,
    pub window_days: u32,
    /// poslední hotový přehled (i během dalšího přepočtu)
    pub summary: Option<view::Summary>,
    /// datum ověření ceníku a zdroj
    pub prices_verified: &'static str,
    pub prices_source: &'static str,
}

/// Pro snapshot — jen paměť.
pub fn view() -> CostsView {
    let on = enabled();
    with(|l| CostsView {
        state: match () {
            _ if !on => "off",
            _ if l.running => "computing",
            _ if l.error.is_some() => "error",
            _ if l.summary.is_some() => "ok",
            _ => "idle",
        },
        progress: l.progress.map(|(d, t)| if t == 0 { 1.0 } else { d as f64 / t as f64 }),
        computed_at: l.computed_at,
        error: l.error.clone(),
        window_days: WINDOW_DAYS,
        summary: if on { l.summary.clone() } else { None },
        prices_verified: price::VERIFIED,
        prices_source: price::SOURCE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_fallbacks() {
        let dirs = vec![("Ledge".to_string(), "C:\\Users\\x\\Desktop\\Ledge".to_string())];
        assert_eq!(project_name(&dirs, "C:\\Users\\x\\Desktop\\Ledge\\src"), "Ledge");
        assert_eq!(project_name(&dirs, "C:\\Users\\x\\Desktop\\Ledger"), "Ledger", "hranice komponenty, jinak název složky");
        assert_eq!(project_name(&dirs, "D:\\work\\atlas\\"), "atlas");
        assert_eq!(project_name(&dirs, ""), "");
    }

    #[test]
    fn local_day_and_midnight_agree() {
        let now = 1_792_238_400_000; // 2026-10-17 12:00 UTC
        let d = local_day(now);
        let m = local_midnight(d);
        assert!(m <= now && now - m < 86_400_000 + 3_600_000);
        assert_eq!(local_day(m), d);
        assert_eq!(local_day(m - 1), d - 1);
    }
}
