//! Karta GitHub: PR, které čekají na tvoji revizi, tvoje otevřené PR se stavem CI a nepřečtené
//! notifikace — z přihlášeného `gh` (GitHub CLI), bez vlastního OAuth. Vypnuto, dokud to uživatel nezapne
//! (`github.enabled`). Dotazy na GitHub spouští jen karta, když je vidět (UI volá `github_view` při zobrazení
//! a pak ve chvíli `nextInMs`), nebo plánovač `integrations.rs`, když je zapnutá položka proužku `github` —
//! vždy s odstupem aspoň `refreshMin` (2–60 min; tlačítko Obnovit smí po 10 s) a s termínem 15 s na `gh`.
//! Data jsou jen v paměti; token Ledge nikdy nevidí. Parsování je v `parse.rs`, spouštění v `run.rs`.

pub mod conf;
pub mod parse;
mod run;

use crate::strip::{Rgba, Segment};
pub use conf::GithubConfig;
use parking_lot::Mutex;
use parse::{Notif, Pr};
use run::GhErr;
use serde::Serialize;

/// Kolik PR k revizi / vlastních se žádá najednou (víc se nikdy neukazuje).
const LIMIT: usize = 20;
/// Kolik notifikací se žádá (počet nad tuhle hodnotu se ukáže jako „50+“).
const NOTIF_LIMIT: usize = 50;
/// Nejkratší odstup ručního „Obnovit“ (ochrana proti zbytečnému klikání).
const FORCE_GAP_MS: i64 = 10_000;
/// Po chybě „gh chybí / nepřihlášeno“ smí karta zkusit znovu rychleji (uživatel to právě opravuje).
const FIX_GAP_MS: i64 = 15_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// GitHub je v configu vypnutý (nebo nic k četbě není zapnuté)
    Off,
    /// `gh` není nainstalované
    NoGh,
    /// `gh` není přihlášené
    Login,
    Ok,
    /// GitHub je nedostupný nebo `gh` selhalo; zůstanou poslední data
    Error,
}

/// Kdo obnovu žádá — rozhoduje o nejkratším odstupu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// plánovač proužku: vždy celý odstup
    Auto,
    /// karta je vidět
    Card,
    /// tlačítko Obnovit
    Force,
}

#[derive(Clone)]
struct Data {
    review: Vec<Pr>,
    review_count: usize,
    review_more: bool,
    mine: Vec<Pr>,
    mine_count: usize,
    notifs: Vec<Notif>,
    notif_count: usize,
    notif_more: bool,
}

const EMPTY: Data = Data { review: Vec::new(), review_count: 0, review_more: false, mine: Vec::new(), mine_count: 0, notifs: Vec::new(), notif_count: 0, notif_more: false };

struct State {
    data: Data,
    status: Status,
    /// co se nepovedlo (text se skládá až při zobrazení, ať sedí jazyk)
    error: Option<GhErr>,
    fetched_at: Option<i64>,
    attempt_at: Option<i64>,
    loading: bool,
}

static STATE: Mutex<State> = Mutex::new(State { data: EMPTY, status: Status::Off, error: None, fetched_at: None, attempt_at: None, loading: false });

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubView {
    pub status: Status,
    /// lidská věta o poslední chybě (u `Error` i při částečném výpadku)
    pub error: Option<String>,
    pub review_on: bool,
    pub mine_on: bool,
    pub notifs_on: bool,
    pub review: Vec<Pr>,
    pub review_count: usize,
    /// počet je jen spodní odhad (víc než se žádalo)
    pub review_more: bool,
    pub mine: Vec<Pr>,
    pub mine_count: usize,
    pub notifs: Vec<Notif>,
    pub notif_count: usize,
    pub notif_more: bool,
    pub fetched_ms: Option<i64>,
    pub loading: bool,
    /// za kolik ms smí karta znovu zavolat `github_view` (0 = hned)
    pub next_in_ms: i64,
}

/// Má se teď sáhnout na GitHub? Čistá funkce (tabulkový test): nikdy při běžícím dotazu, jinak podle
/// toho, kdo žádá a v jakém je stavu poslední pokus.
pub fn due(now: i64, attempt_at: Option<i64>, status: Status, interval_ms: i64, mode: Mode) -> bool {
    let Some(at) = attempt_at else { return true };
    now.saturating_sub(at) >= gap(status, interval_ms, mode) || now < at
}

fn gap(status: Status, interval_ms: i64, mode: Mode) -> i64 {
    match (mode, status) {
        (Mode::Force, _) => FORCE_GAP_MS,
        (Mode::Card, Status::NoGh | Status::Login) => FIX_GAP_MS,
        _ => interval_ms,
    }
}

/// Výsledky tří dotazů; `None` = dotaz se nepožadoval (volba vypnutá).
struct Fetched {
    review: Option<Result<Vec<Pr>, GhErr>>,
    mine: Option<Result<(usize, Vec<Pr>), GhErr>>,
    notifs: Option<Result<(usize, Vec<Notif>), GhErr>>,
}

/// Zapíše výsledky do dat a určí stav: `Missing` / `NotLoggedIn` mají přednost, jinak `Ok`, když aspoň
/// jeden dotaz prošel, jinak `Error`. Vypnuté části se vyprázdní. Vrací (stav, první chyba).
fn apply(data: &mut Data, f: Fetched) -> (Status, Option<GhErr>) {
    let mut first_err: Option<GhErr> = None;
    let mut ok = 0;
    let mut note = |e: GhErr| {
        first_err.get_or_insert(e);
    };
    match f.review {
        Some(Ok(v)) => {
            data.review_count = v.len();
            data.review_more = v.len() >= LIMIT;
            data.review = v.into_iter().take(parse::SHOWN).collect();
            ok += 1;
        }
        Some(Err(e)) => note(e),
        None => (data.review, data.review_count, data.review_more) = (Vec::new(), 0, false),
    }
    match f.mine {
        Some(Ok((total, v))) => {
            data.mine_count = total;
            data.mine = v.into_iter().take(parse::SHOWN).collect();
            ok += 1;
        }
        Some(Err(e)) => note(e),
        None => (data.mine, data.mine_count) = (Vec::new(), 0),
    }
    match f.notifs {
        Some(Ok((n, v))) => {
            data.notif_count = n;
            data.notif_more = n >= NOTIF_LIMIT;
            data.notifs = v;
            ok += 1;
        }
        Some(Err(e)) => note(e),
        None => (data.notifs, data.notif_count, data.notif_more) = (Vec::new(), 0, false),
    }
    match &first_err {
        Some(GhErr::Missing) => (Status::NoGh, first_err),
        Some(GhErr::NotLoggedIn) => (Status::Login, first_err),
        Some(_) if ok == 0 => (Status::Error, first_err),
        _ => (Status::Ok, first_err),
    }
}

fn err_text(e: &GhErr) -> String {
    match e {
        GhErr::Missing => crate::i18n::tr("GitHub CLI (gh) není nainstalované.", "GitHub CLI (gh) isn't installed.").into(),
        GhErr::NotLoggedIn => crate::i18n::tr("gh není přihlášené.", "gh isn't logged in.").into(),
        GhErr::Offline => crate::i18n::tr("GitHub je nedostupný (offline?).", "GitHub is unreachable (offline?).").into(),
        GhErr::Other(s) if s.is_empty() => crate::i18n::tr("gh selhalo.", "gh failed.").into(),
        GhErr::Other(s) => format!("gh: {s}"),
    }
}

fn view(st: &State, cfg: &GithubConfig, now: i64) -> GithubView {
    let on = cfg.any();
    let (status, data) = if on { (st.status, st.data.clone()) } else { (Status::Off, EMPTY) };
    let next = match st.attempt_at {
        Some(a) if on => (a + gap(status, cfg.interval_ms(), Mode::Card) - now).max(0),
        _ => 0,
    };
    GithubView {
        status,
        error: st.error.as_ref().filter(|_| on).map(err_text),
        review_on: on && cfg.review,
        mine_on: on && cfg.mine,
        notifs_on: on && cfg.notifications,
        review: data.review,
        review_count: data.review_count,
        review_more: data.review_more,
        mine: data.mine,
        mine_count: data.mine_count,
        notifs: data.notifs,
        notif_count: data.notif_count,
        notif_more: data.notif_more,
        fetched_ms: st.fetched_at,
        loading: st.loading,
        next_in_ms: next,
    }
}

/// Pohled bez sahání na síť (jen testy).
#[cfg(test)]
pub fn peek(cfg: &GithubConfig) -> GithubView {
    view(&STATE.lock(), cfg, crate::state::now_ms())
}

/// Tři dotazy `gh` souběžně (každý s vlastním termínem). Blokuje volající vlákno nejvýš `run::TIMEOUT`.
fn fetch(cfg: &GithubConfig) -> Fetched {
    const SEARCH_FIELDS: &str = "number,title,url,repository,author,updatedAt,isDraft";
    const MINE_QUERY: &str = "query($n:Int!){viewer{pullRequests(first:$n,states:OPEN,orderBy:{field:UPDATED_AT,direction:DESC}){totalCount nodes{number title url isDraft updatedAt repository{nameWithOwner} reviewDecision commits(last:1){nodes{commit{statusCheckRollup{state}}}}}}}}";
    let odd = || GhErr::Other(crate::i18n::tr("gh vrátilo neočekávanou odpověď.", "gh returned an unexpected response.").into());
    let limit = LIMIT.to_string();
    let n_arg = format!("n={LIMIT}");
    let notif_path = format!("notifications?per_page={NOTIF_LIMIT}");
    std::thread::scope(|s| {
        let review = cfg.review.then(|| {
            s.spawn(|| {
                let args = ["search", "prs", "--review-requested=@me", "--state=open", "--sort", "updated", "--order", "desc", "--limit", limit.as_str(), "--json", SEARCH_FIELDS];
                run::run(&args).and_then(|o| parse::parse_search(&o).ok_or_else(odd))
            })
        });
        let mine = cfg.mine.then(|| {
            s.spawn(|| {
                let q = format!("query={MINE_QUERY}");
                run::run(&["api", "graphql", "-f", q.as_str(), "-F", n_arg.as_str()]).and_then(|o| parse::parse_mine(&o).ok_or_else(odd))
            })
        });
        let notifs = cfg.notifications.then(|| s.spawn(|| run::run(&["api", notif_path.as_str()]).and_then(|o| parse::parse_notifications(&o).ok_or_else(odd))));
        let crashed = || GhErr::Other(crate::i18n::tr("interní chyba", "internal error").into());
        Fetched {
            review: review.map(|h| h.join().unwrap_or_else(|_| Err(crashed()))),
            mine: mine.map(|h| h.join().unwrap_or_else(|_| Err(crashed()))),
            notifs: notifs.map(|h| h.join().unwrap_or_else(|_| Err(crashed()))),
        }
    })
}

/// Obnoví data (blokuje až `run::TIMEOUT`), pokud to `mode` a stav dovolí, a vrátí aktuální pohled.
/// Běžící dotaz druhý nespustí (vrátí pohled s `loading`).
pub fn refresh(cfg: &GithubConfig, mode: Mode) -> GithubView {
    let now = crate::state::now_ms();
    {
        let mut st = STATE.lock();
        if !cfg.any() {
            st.data = EMPTY;
            st.error = None;
            st.status = Status::Off;
            return view(&st, cfg, now);
        }
        if st.loading || !due(now, st.attempt_at, st.status, cfg.interval_ms(), mode) {
            return view(&st, cfg, now);
        }
        st.loading = true;
        st.attempt_at = Some(now);
    }
    let fetched = fetch(cfg);
    let mut st = STATE.lock();
    let (status, err) = apply(&mut st.data, fetched);
    if matches!(status, Status::NoGh | Status::Login) {
        st.data = EMPTY;
    }
    (st.status, st.error, st.loading) = (status, err, false);
    if matches!(status, Status::Ok) {
        st.fetched_at = Some(crate::state::now_ms());
    }
    let out = view(&st, cfg, crate::state::now_ms());
    drop(st);
    // návrhy a puls proužku (`suggest/`): nové PR k revizi / spadlé CI na mém PR; zámek už je pryč
    crate::suggest::on_github_refreshed();
    out
}

/// Paměť bez sahání na síť: (PR k revizi, vlastní PR, čas posledního úspěšného načtení) — jen při zapnutém
/// GitHubu a stavu `Ok`, jinak `None`. Pro návrhy a puls (`suggest/`); nic nespouští.
pub fn cached_prs(cfg: &GithubConfig) -> Option<(Vec<Pr>, Vec<Pr>, Option<i64>)> {
    let st = STATE.lock();
    (cfg.any() && st.status == Status::Ok).then(|| (st.data.review.clone(), st.data.mine.clone(), st.fetched_at))
}

/// Kdy plánovač proužku smí příště sáhnout na GitHub; `None` = nic se nemá číst.
pub fn next_due(cfg: &GithubConfig, now: i64) -> Option<i64> {
    if !cfg.any() {
        return None;
    }
    let st = STATE.lock();
    Some(match st.attempt_at {
        Some(a) if a <= now => a + cfg.interval_ms(),
        Some(_) => now,
        None => now,
    })
}

/// Změna configu GitHubu: data se musí znovu načíst (zapnutá nová sekce nemá co ukázat), takže další
/// obnova smí hned; vypnutí paměť vyprázdní.
pub fn on_config(old: &GithubConfig, new: &GithubConfig) {
    if old == new {
        return;
    }
    {
        let mut st = STATE.lock();
        st.attempt_at = None;
        if !new.any() {
            st.data = EMPTY;
            st.error = None;
            st.status = Status::Off;
            st.fetched_at = None;
        }
    }
    if !new.any() {
        // po vypnutí GitHubu se pozorování PR zahodí — po zapnutí se zase nejdřív jen načte (bez pulsu)
        crate::suggest::reset_github();
    }
}

/// Segment položky proužku `github`: „GitHub: 2× čeká na revizi“; jen když je co ukázat.
pub fn strip_now(cfg: &GithubConfig) -> Vec<Segment> {
    let st = STATE.lock();
    if !(cfg.any() && cfg.review && st.status == Status::Ok && st.data.review_count > 0) {
        return Vec::new();
    }
    let n = st.data.review_count;
    let more = if st.data.review_more { "+" } else { "" };
    let text = trf!("GitHub: {n}{more}× čeká na revizi", "GitHub: {n}{more} to review");
    vec![Segment { text, color: Rgba::TEXT, dot: Some(Rgba::AMBER), strong: false, pill: true }]
}

/// Smí se adresa z UI otevřít? Jen `https://github.com/…` (data z `gh` jsou cizí text).
pub fn open_ok(url: &str) -> bool {
    !parse::safe_url(url).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(n: u64) -> Pr {
        Pr { repo: "a/b".into(), number: n, title: "t".into(), url: String::new(), author: String::new(), draft: false, updated: String::new(), ci: parse::Ci::None, decision: String::new() }
    }
    fn data() -> Data {
        EMPTY
    }

    #[test]
    fn due_table() {
        let min = 120_000;
        let cases = [
            ("první pokus", 1000, None, Status::Off, Mode::Auto, true),
            ("před odstupem (karta)", 100_000, Some(0), Status::Ok, Mode::Card, false),
            ("po odstupu (karta)", 120_000, Some(0), Status::Ok, Mode::Card, true),
            ("před odstupem (plánovač)", 119_999, Some(0), Status::Ok, Mode::Auto, false),
            ("Obnovit po 10 s", 10_000, Some(0), Status::Ok, Mode::Force, true),
            ("Obnovit hned po minulém", 9_999, Some(0), Status::Ok, Mode::Force, false),
            ("gh chybí → karta zkusí po 15 s", 15_000, Some(0), Status::NoGh, Mode::Card, true),
            ("gh chybí → karta ne dřív", 14_999, Some(0), Status::Login, Mode::Card, false),
            ("gh chybí → plánovač čeká celý odstup", 60_000, Some(0), Status::NoGh, Mode::Auto, false),
            ("chyba sítě → celý odstup", 60_000, Some(0), Status::Error, Mode::Card, false),
            ("hodiny šly zpět", 5, Some(1000), Status::Ok, Mode::Auto, true),
            ("po výpadku dlouho", 10_000_000, Some(0), Status::Error, Mode::Auto, true),
        ];
        for (name, now, at, st, mode, want) in cases {
            assert_eq!(due(now, at, st, min, mode), want, "{name}");
        }
    }

    fn fetched(r: Option<Result<Vec<Pr>, GhErr>>, m: Option<Result<(usize, Vec<Pr>), GhErr>>, n: Option<Result<(usize, Vec<Notif>), GhErr>>) -> Fetched {
        Fetched { review: r, mine: m, notifs: n }
    }

    #[test]
    fn apply_status_table() {
        let ok_r = || Some(Ok(vec![pr(1), pr(2)]));
        let ok_m = || Some(Ok((9, vec![pr(3)])));
        let ok_n = || Some(Ok((3, vec![])));
        let cases: [(&str, Fetched, Status, bool); 8] = [
            ("vše prošlo", fetched(ok_r(), ok_m(), ok_n()), Status::Ok, false),
            ("nepřihlášeno vyhrává i nad daty", fetched(Some(Err(GhErr::NotLoggedIn)), ok_m(), ok_n()), Status::Login, true),
            ("gh chybí", fetched(Some(Err(GhErr::Missing)), Some(Err(GhErr::Missing)), None), Status::NoGh, true),
            ("všechny dotazy spadly (offline)", fetched(Some(Err(GhErr::Offline)), Some(Err(GhErr::Offline)), Some(Err(GhErr::Offline))), Status::Error, true),
            ("částečný výpadek = Ok + poznámka", fetched(ok_r(), Some(Err(GhErr::Other("403".into()))), ok_n()), Status::Ok, true),
            ("jen jedna část zapnutá a prošla", fetched(ok_r(), None, None), Status::Ok, false),
            ("jedna zapnutá a spadla", fetched(None, None, Some(Err(GhErr::Offline))), Status::Error, true),
            ("nic nezapnuto", fetched(None, None, None), Status::Ok, false),
        ];
        for (name, f, want, has_err) in cases {
            let mut d = data();
            let (status, err) = apply(&mut d, f);
            assert_eq!((status, err.is_some()), (want, has_err), "{name}");
        }
    }

    #[test]
    fn apply_fills_counts_and_clears_disabled_parts() {
        let mut d = data();
        let many: Vec<Pr> = (0..LIMIT as u64).map(pr).collect();
        let n_many: Vec<Notif> = vec![];
        apply(&mut d, fetched(Some(Ok(many)), Some(Ok((23, (0..8).map(pr).collect()))), Some(Ok((NOTIF_LIMIT, n_many)))));
        assert_eq!((d.review.len(), d.review_count, d.review_more), (parse::SHOWN, LIMIT, true), "ukáže 5, počet 20+");
        assert_eq!((d.mine.len(), d.mine_count), (parse::SHOWN, 23), "celkový počet je z GitHubu, ne z ukázaných");
        assert_eq!((d.notif_count, d.notif_more), (NOTIF_LIMIT, true));
        // vypnutá volba vyprázdní svoji část
        apply(&mut d, fetched(None, Some(Ok((1, vec![pr(1)]))), None));
        assert_eq!((d.review.len(), d.review_count, d.notif_count), (0, 0, 0));
        assert_eq!(d.mine_count, 1);
    }

    #[test]
    fn only_github_urls_open() {
        assert!(open_ok("https://github.com/acme/ledge/pull/1"));
        for bad in ["", "http://github.com/a", "https://github.com.evil.example/a", "https://evil.example/https://github.com/", "file:///c:/x", "https://github.com/a b", "https://github.com/a\nb"] {
            assert!(!open_ok(bad), "{bad:?}");
        }
    }

    /// Živý dotaz na GitHub přes přihlášené `gh` (jen čtení, vypisuje jen počty). Ruční kontrola, že tvar
    /// odpovědí pořád sedí: `cargo test live_gh_smoke -- --ignored --nocapture`.
    #[test]
    #[ignore = "sahá na síť a vyžaduje přihlášené gh"]
    fn live_gh_smoke() {
        let cfg = GithubConfig { enabled: true, ..Default::default() };
        let f = fetch(&cfg);
        let review = f.review.expect("požadováno").map(|v| v.len());
        let mine = f.mine.expect("požadováno").map(|(n, v)| (n, v.len()));
        let notifs = f.notifs.expect("požadováno").map(|(n, v)| (n, v.len()));
        println!("review: {review:?}, mine: {mine:?}, notifs: {notifs:?}");
        assert!(review.is_ok() && mine.is_ok() && notifs.is_ok(), "gh chybí / nepřihlášeno / offline");
    }

    #[test]
    fn view_when_disabled_is_empty() {
        let off = GithubConfig::default();
        let v = peek(&off);
        assert_eq!(v.status, Status::Off);
        assert!(!v.review_on && !v.mine_on && !v.notifs_on && v.review.is_empty() && v.next_in_ms == 0);
        assert!(strip_now(&off).is_empty());
        assert_eq!(next_due(&off, 1), None);
    }
}
