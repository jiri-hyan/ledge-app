//! Kalendáře přihlášených účtů v kartě Kalendář: stav a stahování (adaptéry `calendar/graph.rs`,
//! `calendar/google.rs`), stejná politika jako ICS (`calendar::live`): jen s viditelnou kartou, data
//! starší než `MAX_AGE_MS`, po pokusu `RETRY_MS` pauza, nic v klidu ani ve fullscreen hře.
//! Kartě dává zdroje jako další `CalInfo` za kalendáři z odkazu (index = `Item.cal`).

use super::{Provider, session};
use crate::calendar::Occ;
use crate::calendar::live::{CalInfo, feed_status, is_old, needs_fetch};
use crate::calendar::source::{Source, SourceInfo, SourceKind, Window};
use crate::config::Config;
use crate::drawer::View;
use crate::state::{core, mark_dirty, now_ms};
use parking_lot::Mutex;

/// Pokus bez výsledku déle než tohle = zaseklý (víc stránek + obnova tokenu, každé volání 15 s).
const STUCK_MS: i64 = 90_000;

#[derive(Default)]
struct St {
    occs: Vec<Occ>,
    fetched_ms: Option<i64>,
    attempt_ms: Option<i64>,
    loading: bool,
    error: Option<String>,
    /// e-mail účtu z doby stažení (do patičky karty)
    account: String,
    /// název a barva zdroje (`Source::info`)
    info: Option<SourceInfo>,
}

static S: Mutex<[Option<St>; 2]> = Mutex::new([None, None]);

fn ix(p: Provider) -> usize {
    match p {
        Provider::Microsoft => 0,
        Provider::Google => 1,
    }
}

/// Druh zdroje kalendáře (`personal.calendars[].source`) téhož účtu.
fn kind(p: Provider) -> SourceKind {
    match p {
        Provider::Microsoft => SourceKind::Graph,
        Provider::Google => SourceKind::Google,
    }
}

/// Účet má v configu zapnutý kalendář? Je-li týž účet zapnutý i jako řádek `personal.calendars`
/// (`source` `graph` / `google`, `source::adapter`), vede ho kalendář a tady se vynechá (žádné dvojí události).
pub fn wanted(cfg: &Config, p: Provider) -> bool {
    let on = match p {
        Provider::Microsoft => cfg.accounts.microsoft.calendar,
        Provider::Google => cfg.accounts.google.calendar,
    };
    on && !cfg.personal.calendars.iter().any(|f| f.enabled && f.source == kind(p))
}

fn loading_live(st: &St, now: i64) -> bool {
    st.loading && st.attempt_ms.is_some_and(|a| now - a < STUCK_MS)
}

/// Spustí stažení kalendářů účtů, když je karta vidět a data jsou stará.
pub(crate) fn maybe_refresh(opening: bool) {
    let (want, accounts) = {
        let c = core();
        let cfg = c.cfg.read();
        let visible = crate::layout::page_has_calendar(&cfg.layout, &c.mode.lock()) && (opening || c.drawer.view() == View::Full);
        if !visible || c.dead() {
            return;
        }
        (Provider::ALL.map(|p| wanted(&cfg, p)), cfg.accounts.clone())
    };
    let now = now_ms();
    for p in Provider::ALL {
        if !want[ix(p)] || !session::connected(p) {
            continue;
        }
        let go = {
            let mut g = S.lock();
            let st = g[ix(p)].get_or_insert_with(St::default);
            let stuck = st.loading && !loading_live(st, now);
            let go = needs_fetch(now, st.fetched_ms, st.attempt_ms, st.loading && !stuck);
            if go {
                (st.loading, st.attempt_ms) = (true, Some(now));
            }
            go
        };
        if !go {
            continue;
        }
        mark_dirty();
        let src = source(p, &accounts);
        // `Source::events` blokuje (síť) — vlastní krátké vlákno mimo async runtime
        let spawned = std::thread::Builder::new().name("accounts-cal".into()).spawn(move || {
            let res = src.events(Window::around(now_ms()));
            let account = session::account(p);
            let mut g = S.lock();
            let Some(st) = g[ix(p)].as_mut() else { return }; // mezitím odhlášeno
            if st.attempt_ms != Some(now) {
                return;
            }
            (st.loading, st.account, st.info) = (false, account, Some(src.info()));
            match res {
                Ok(evs) => {
                    st.occs = evs;
                    (st.fetched_ms, st.error) = (Some(now_ms()), None);
                }
                Err(e) => st.error = Some(e),
            }
            drop(g);
            mark_dirty();
        });
        if spawned.is_err()
            && let Some(st) = S.lock()[ix(p)].as_mut()
        {
            st.loading = false;
        }
    }
}

/// Zdroj kalendáře účtu (adaptér) podle configu.
pub fn source(p: Provider, a: &super::conf::AccountsConfig) -> Box<dyn Source> {
    match p {
        Provider::Microsoft => Box::new(crate::calendar::graph::GraphSource { name: source_name(p).into(), color: a.microsoft.color.clone() }),
        Provider::Google => Box::new(crate::calendar::google::GoogleSource { name: source_name(p).into(), color: a.google.color.clone(), calendars: a.google.calendars.clone() }),
    }
}

/// Zahodí stažená data účtu (odhlášení, nové přihlášení, změna výběru kalendářů).
pub fn forget(p: Provider) {
    S.lock()[ix(p)] = None;
}

/// Změna configu: jiný výběr kalendářů Google nebo vypnutý kalendář → zahodit a případně stáhnout znovu.
pub(crate) fn on_config(old: &Config, new: &Config) {
    let (o, n) = (&old.accounts, &new.accounts);
    if o.google.calendars != n.google.calendars || (o.google.calendar && !n.google.calendar) {
        forget(Provider::Google);
    }
    if o.microsoft.calendar && !n.microsoft.calendar {
        forget(Provider::Microsoft);
    }
    if o != n {
        maybe_refresh(false);
        mark_dirty();
    }
}

/// Název zdroje v patičce karty.
pub fn source_name(p: Provider) -> &'static str {
    match p {
        Provider::Microsoft => "Outlook",
        Provider::Google => "Google",
    }
}

/// Zdroje účtů pro `calendar::live::view` (jen paměť — nesahá na Správce pověření ani `core().cfg`).
pub(crate) fn infos(cfg: &Config, now: i64) -> Vec<(CalInfo, Vec<Occ>)> {
    let g = S.lock();
    let mut out = Vec::new();
    for p in Provider::ALL {
        let Some(st) = g[ix(p)].as_ref().filter(|_| wanted(cfg, p)) else { continue };
        let live = loading_live(st, now);
        let error = st.error.clone().or_else(|| (st.loading && !live).then(|| crate::calendar::live::stuck().to_string()));
        let status = feed_status(st.fetched_ms.is_some(), error.is_some(), live, is_old(st.fetched_ms, now));
        // barva z aktuálního configu (změna se projeví hned), název ze zdroje
        let src = st.info.clone().unwrap_or_else(|| source(p, &cfg.accounts).info());
        let color = source(p, &cfg.accounts).info().color.or(src.color);
        let info = CalInfo {
            name: src.name,
            color,
            domain: st.account.clone(),
            source: kind(p).id(),
            hint: None,
            enabled: true,
            status,
            error,
            fetched_ms: st.fetched_ms,
            events: st.occs.len(),
            account: Some(p.id()),
        };
        out.push((info, st.occs.clone()));
    }
    out
}

/// Blokující běh asynchronní práce pro kontrakt `calendar::source::Source` (volat z vlákna mimo
/// async runtime nebo ze `spawn_blocking`, nikdy z async úlohy).
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    match tauri::async_runtime::TokioHandle::try_current() {
        Ok(h) => h.block_on(f),
        Err(_) => tauri::async_runtime::block_on(f),
    }
}

/// Termín pro emit vlákno: watchdog běžícího stažení (jen s viditelnou kartou — jinak nic).
pub(crate) fn next_deadline() -> Option<i64> {
    let g = S.lock();
    g.iter().flatten().filter(|s| s.loading).filter_map(|s| s.attempt_ms).map(|a| a + STUCK_MS + 1).min()
}

/// Kontrola konstant politiky (sdílené s ICS).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::live::{MAX_AGE_MS, RETRY_MS};

    #[test]
    fn policy_shared_with_ics_and_stuck() {
        let now = 10_000_000_000;
        // stejná politika jako ICS: stažení se nezkouší častěji než jednou za RETRY_MS
        assert!(!needs_fetch(now, None, Some(now - RETRY_MS + 1), false));
        assert!(needs_fetch(now, Some(now - MAX_AGE_MS), Some(now - MAX_AGE_MS), false));
        let st = St { loading: true, attempt_ms: Some(now - 1000), ..Default::default() };
        assert!(loading_live(&st, now));
        let st = St { loading: true, attempt_ms: Some(now - STUCK_MS), ..Default::default() };
        assert!(!loading_live(&st, now));
    }

    #[test]
    fn infos_only_for_active_wanted_accounts() {
        let mut cfg = Config::default();
        assert!(infos(&cfg, 0).is_empty(), "bez přihlášení žádný zdroj");
        S.lock()[0] = Some(St { occs: vec![], fetched_ms: Some(1000), account: "jana@contoso.com".into(), ..Default::default() });
        let v = infos(&cfg, 2000);
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].0.name.as_str(), v[0].0.domain.as_str(), v[0].0.status, v[0].0.account), ("Outlook", "jana@contoso.com", "ok", Some("microsoft")));
        cfg.accounts.microsoft.calendar = false;
        assert!(infos(&cfg, 2000).is_empty(), "vypnutý kalendář účtu se neukáže");
        forget(Provider::Microsoft);
        cfg.accounts.microsoft.calendar = true;
        assert!(infos(&cfg, 2000).is_empty());
    }
}
