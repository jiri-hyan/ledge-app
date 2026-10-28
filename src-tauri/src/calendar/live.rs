//! Kalendář za běhu Ledge: stav feedů, stahování (vzor `weather.rs`), cache na disku, výběr pro
//! snapshot a termíny pro emit vlákno.
//!
//! Žádný poll: stahuje se jen při otevření šuplíku na Osobní se zapnutou kartou, když jsou data starší
//! než `MAX_AGE_MS` (po pokusu aspoň `RETRY_MS` pauza). Skrytá karta = nic se nestahuje. Emit vlákno se
//! kvůli kalendáři probudí jen při otevřené kartě, a to v čase další změny výběru (`view::pick`
//! `next_change`: začátek/konec dnešní události, půlnoc). Adresa feedu se nikde neloguje; stav i cache
//! (`%LOCALAPPDATA%\Ledge\calendar-cache.json`) jsou klíčované otiskem adresy (`conf::key`; jiné zdroje
//! `source::key`). Načítání podle zdroje: `source::fetch` (ICS zde `fetch_parse`, Outlook v PC jen když
//! běží — jinak nápověda `HINT_OUTLOOK_CLOSED` a tlačítko „Načíst z Outlooku“ → `load_outlook`).
//! Připomínky (`remind.rs`) čtou výskyty přes `reminder_feeds` / `with_reminders` a po startu si
//! vyžádají cache z disku (`ensure_cache`) — nic se kvůli nim nestahuje.

use super::conf;
use super::source::{self, Target};
use super::tz::SystemLocal;
use super::cache::{CacheFeed, cache_path, decode_cache, encode_cache};
use super::{Occ, Parsed, view};
use crate::config::Config;
use crate::drawer::View;
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty, now_ms};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

/// Data mladší než tohle se při otevření znovu nestahují.
pub const MAX_AGE_MS: i64 = 15 * 60 * 1000;
/// Po pokusu (i nepovedeném) aspoň tolik pauza.
pub const RETRY_MS: i64 = 2 * 60 * 1000;
pub const TIMEOUT: Duration = Duration::from_secs(15);
/// Větší odpověď se zahodí (chyba) — paměť i čas parsování.
pub const MAX_BYTES: usize = 5 * 1024 * 1024;
/// Nejvíc řádků na kartě (zbytek jako „+ další“).
pub const CARD_ROWS: usize = 10;
/// Outlook v PC je zavřený — automaticky se nespouští (karta nabídne „Načíst z Outlooku“).
pub const HINT_OUTLOOK_CLOSED: &str = "outlook-closed";

#[derive(Default)]
struct FeedState {
    occs: Vec<Occ>,
    events: usize,
    /// `X-WR-CALNAME` z feedu
    name: String,
    fetched_ms: Option<i64>,
    attempt_ms: Option<i64>,
    loading: bool,
    error: Option<String>,
    /// nápověda pro UI bez chyby (`HINT_OUTLOOK_CLOSED`)
    hint: Option<&'static str>,
}

#[derive(Default, PartialEq)]
enum CacheLoad {
    #[default]
    No,
    Started,
}

#[derive(Default)]
struct Live {
    /// podle `conf::key(url)`
    feeds: HashMap<String, FeedState>,
    cache: CacheLoad,
    /// kdy se výběr karty změní (z posledního `view`)
    next_change: Option<i64>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    let mut g = LIVE.lock();
    f(g.get_or_insert_with(Live::default))
}

// ---------- čistá politika ----------

/// Běží pokus doopravdy? Bez odpovědi déle než 2× timeout = neúspěch (zaseklý task).
pub fn loading_live(loading: bool, attempt_ms: Option<i64>, now: i64) -> bool {
    loading && attempt_ms.is_some_and(|a| now - a < 2 * TIMEOUT.as_millis() as i64)
}

pub fn is_old(fetched_ms: Option<i64>, now: i64) -> bool {
    fetched_ms.is_none_or(|f| now - f >= MAX_AGE_MS)
}

/// Stáhnout? Ne během pokusu, ne dřív než `RETRY_MS` po posledním, jinak když data chybí / jsou stará.
pub fn needs_fetch(now: i64, fetched_ms: Option<i64>, attempt_ms: Option<i64>, loading: bool) -> bool {
    if loading_live(loading, attempt_ms, now) || attempt_ms.is_some_and(|a| now - a < RETRY_MS) {
        return false;
    }
    is_old(fetched_ms, now)
}

/// Stav jednoho kalendáře: ok · stale (data jsou, ale stará nebo poslední pokus selhal) · loading ·
/// unavailable (nic dobrého) · idle (ještě nestaženo).
pub fn feed_status(has_data: bool, error: bool, loading: bool, old: bool) -> &'static str {
    match (has_data, error || old, loading) {
        (true, true, _) => "stale",
        (true, false, _) => "ok",
        (false, _, true) => "loading",
        (false, _, false) if error => "unavailable",
        (false, ..) => "idle",
    }
}

/// Stav karty ze stavů zapnutých kalendářů: unset (žádný) · ok · stale · loading · unavailable · idle.
pub fn overall(st: &[&str]) -> &'static str {
    let any = |s: &str| st.contains(&s);
    if st.is_empty() {
        "unset"
    } else if any("ok") || any("stale") {
        if st.iter().all(|s| *s == "ok") { "ok" } else { "stale" }
    } else if any("loading") {
        "loading"
    } else if any("unavailable") || any("invalid") {
        "unavailable"
    } else {
        "idle"
    }
}

/// Chyba stažení lidskou řečí (bez HTTP kódů a bez adresy). Kód jde jen do logu.
pub fn http_error(status: u16) -> &'static str {
    match status {
        401 | 403 | 404 | 410 => tr(
            "Odkaz už nefunguje — kalendář ho nejspíš zneplatnil. Zkopíruj v Googlu/Outlooku nový a vlož ho v Nastavení.",
            "The link no longer works — the calendar probably revoked it. Copy a new one in Google/Outlook and paste it in Settings.",
        ),
        429 => tr("Kalendář teď odmítá tolik dotazů — zkusím to později sám.", "The calendar is refusing requests right now — I'll try again later."),
        500..=599 => tr("Server kalendáře má potíže — zkusím to později sám.", "The calendar server is having trouble — I'll try again later."),
        _ => tr("Kalendář odpověděl chybou — zkontroluj odkaz v Nastavení.", "The calendar answered with an error — check the link in Settings."),
    }
}

pub fn stuck() -> &'static str {
    tr("Kalendář neodpověděl včas — zkusím to znovu.", "The calendar didn't answer in time — I'll try again.")
}

fn too_big() -> String {
    tr("Kalendář je moc velký (přes 5 MB) — Ledge ho nestáhne.", "The calendar is too big (over 5 MB) — Ledge won't download it.").into()
}

// ---------- cache na disku (formát v `cache.rs`) ----------

fn save_cache() {
    let feeds: Vec<CacheFeed> = with(|l| {
        l.feeds
            .iter()
            .filter_map(|(k, s)| Some(CacheFeed { key: k.clone(), fetched_ms: s.fetched_ms?, events: s.events, name: s.name.clone(), occs: s.occs.clone() }))
            .collect()
    });
    let path = cache_path();
    let text = encode_cache(feeds);
    let res = crate::fsx::write_atomic_dir(&path, text.as_bytes());
    if let Err(e) = res {
        log(&format!("kalendář: cache nejde uložit ({e})"));
    }
}

/// Po startu: data z cache do kalendářů, které ještě nic nestáhly (karta hned není prázdná).
fn load_cache_async() {
    std::thread::Builder::new()
        .name("calendar-cache".into())
        .spawn(|| {
            let Ok(text) = std::fs::read_to_string(cache_path()) else { return };
            let feeds = decode_cache(&text, now_ms());
            let any = with(|l| {
                let mut any = false;
                for c in feeds {
                    if let Some(st) = l.feeds.get_mut(&c.key)
                        && st.fetched_ms.is_none()
                    {
                        (st.occs, st.events, st.name, st.fetched_ms) = (c.occs, c.events, c.name, Some(c.fetched_ms));
                        any = true;
                    }
                }
                any
            });
            if any {
                mark_dirty();
            }
        })
        .ok();
}

// ---------- stahování ----------

fn net_error(e: reqwest::Error) -> String {
    let e = e.without_url();
    if e.is_timeout() {
        return stuck().into();
    }
    // příčina do logu (bez adresy), uživateli lidsky
    let mut why = e.to_string();
    let mut src = std::error::Error::source(&e);
    while let Some(s) = src {
        why.push_str(&format!(": {s}"));
        src = s.source();
    }
    log(&format!("kalendář: síť ({why})"));
    tr("Ke kalendáři se nejde připojit — jsi online? Zkusím to znovu při dalším otevření.", "Can't connect to the calendar — are you online? I'll try again next time you open the drawer.").into()
}

async fn download(url: &str) -> Result<Vec<u8>, String> {
    // přesměrování jen na https a nejvýš 5×
    let policy = reqwest::redirect::Policy::custom(|a| {
        if a.url().scheme() != "https" {
            a.error("redirect away from https")
        } else if a.previous().len() >= 5 {
            a.stop()
        } else {
            a.follow()
        }
    });
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(policy)
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| trf!("HTTP klient: {}", "HTTP client: {}", e.without_url()))?;
    let mut resp = client.get(url).header(reqwest::header::ACCEPT, "text/calendar, */*;q=0.5").send().await.map_err(net_error)?;
    let status = resp.status();
    if !status.is_success() {
        log(&format!("kalendář: HTTP {}", status.as_u16()));
        return Err(http_error(status.as_u16()).into());
    }
    if resp.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
        return Err(too_big());
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(net_error)? {
        if buf.len() + chunk.len() > MAX_BYTES {
            return Err(too_big());
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// Stažení + rozparsování (parsování mimo async vlákna).
pub(crate) async fn fetch_parse(url: &str, now: i64) -> Result<Parsed, String> {
    let bytes = download(url).await?;
    let parsed = tauri::async_runtime::spawn_blocking(move || super::collect(&bytes, now, &SystemLocal))
        .await
        .map_err(|e| trf!("kalendář: {e}", "calendar: {e}"))?;
    parsed.map_err(|e| {
        log(&format!("kalendář: nečitelný feed ({e})"));
        tr(
            "Na odkazu není kalendář — zkopíruj „tajnou adresu ve formátu iCal“ (končí .ics), ne odkaz na webovou stránku.",
            "There's no calendar at the link — copy the “secret address in iCal format” (ends in .ics), not a web page link.",
        )
        .to_string()
    })
}

/// „Otestovat“ / přidání v Nastavení: jedno stažení, nic se neukládá.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    /// VEVENT celkem
    pub events: usize,
    /// výskytů od teď do +14 dní
    pub upcoming: usize,
    /// název z feedu (`X-WR-CALNAME`); prázdný = feed ho nemá
    pub name: String,
}

pub async fn test(raw: &str) -> Result<TestResult, String> {
    let url = conf::normalize_url(raw)?;
    let now = now_ms();
    let p = fetch_parse(&url, now).await?;
    Ok(TestResult { events: p.events, upcoming: p.occs.iter().filter(|o| o.end > now).count(), name: p.name })
}

// ---------- napojení na jádro ----------

/// Karta je na právě otevřené záložce a šuplík je vidět (nebo se právě otevírá).
fn card_visible(cfg: &Config, opening: bool) -> bool {
    let c = core();
    crate::layout::page_has_calendar(&cfg.layout, &c.mode.lock()) && (opening || c.drawer.view() == View::Full)
}

/// Stáhne zastaralé kalendáře, když je karta vidět. Výsledek přes `mark_dirty`. Jinak nic.
pub(crate) fn maybe_refresh(opening: bool) {
    let feeds = {
        let cfg = core().cfg.read();
        if !card_visible(&cfg, opening) {
            return;
        }
        source::targets(&cfg.personal.calendars)
    };
    if feeds.is_empty() {
        return;
    }
    // Outlook v PC se automaticky jen připojí k běžícímu (nikdy ho nespouští)
    let outlook_up = feeds.iter().any(|(_, t)| matches!(t, Target::OutlookLocal)) && super::outlook::running();
    let now = now_ms();
    let mut start = Vec::new();
    let (load, dirty) = with(|l| {
        let load = l.cache == CacheLoad::No;
        l.cache = CacheLoad::Started;
        let mut dirty = false;
        for (k, u) in &feeds {
            let st = l.feeds.entry(k.clone()).or_default();
            if st.loading && !loading_live(st.loading, st.attempt_ms, now) {
                st.loading = false;
                st.error = Some(stuck().into());
                dirty = true;
            }
            if needs_fetch(now, st.fetched_ms, st.attempt_ms, st.loading) {
                st.attempt_ms = Some(now);
                if matches!(u, Target::OutlookLocal) && !outlook_up {
                    dirty |= st.hint != Some(HINT_OUTLOOK_CLOSED);
                    st.hint = Some(HINT_OUTLOOK_CLOSED);
                    continue;
                }
                st.loading = true;
                start.push((k.clone(), u.clone()));
            }
        }
        (load, dirty)
    });
    if load {
        load_cache_async();
    }
    if dirty || !start.is_empty() {
        mark_dirty();
    }
    for (k, t) in start {
        tauri::async_runtime::spawn(async move {
            let res = source::fetch(t, now_ms(), false).await;
            store(&k, now, res);
        });
    }
}

/// Výsledek pokusu `attempt` do stavu (starší pokus, který mezitím odepsal watchdog, se zahodí).
fn store(k: &str, attempt: i64, res: Result<Parsed, String>) {
    let ok = res.is_ok();
    with(|l| {
        let Some(st) = l.feeds.get_mut(k) else { return };
        if st.attempt_ms != Some(attempt) {
            return; // mezitím novější pokus (tenhle už watchdog odepsal)
        }
        st.loading = false;
        match res {
            Ok(p) => {
                (st.occs, st.events, st.name) = (p.occs, p.events, p.name);
                st.fetched_ms = Some(now_ms());
                (st.error, st.hint) = (None, None);
            }
            Err(e) => st.error = Some(e),
        }
    });
    if ok {
        save_cache();
    }
    mark_dirty();
}

/// Výslovný klik „Načíst z Outlooku“: smí Outlook spustit na pozadí. Vrací počet výskytů od teď.
pub async fn load_outlook() -> Result<usize, String> {
    if !super::outlook::installed() {
        return Err(super::outlook::missing().into());
    }
    let now = now_ms();
    let k = "outlook-local";
    with(|l| {
        let st = l.feeds.entry(k.into()).or_default();
        (st.loading, st.attempt_ms, st.hint) = (true, Some(now), None);
    });
    mark_dirty();
    let res = source::fetch(Target::OutlookLocal, now, true).await;
    let n = res.as_ref().map(|p| p.occs.iter().filter(|o| o.end > now).count()).map_err(Clone::clone);
    store(k, now, res);
    n
}

/// Kalendáře se zapnutou připomínkou: (index, klíč stavu, předstih ms). Karta / záložka vypnutá = nic.
pub(crate) fn reminder_feeds(cfg: &Config) -> Vec<(usize, String, i64)> {
    if !(cfg.modes.personal_enabled && cfg.drawer.sections.calendar) {
        return Vec::new();
    }
    let list = &cfg.personal.calendars;
    list.iter().enumerate().filter(|(_, f)| f.enabled && f.remind_min > 0).filter_map(|(i, f)| Some((i, source::key(f).ok()?, f.remind_min as i64 * 60_000))).collect()
}

/// Výskyty kalendářů z `reminder_feeds` (jen paměť).
pub(crate) fn with_reminders<R>(feeds: &[(usize, String, i64)], f: impl FnOnce(&[(usize, &Occ, i64)]) -> R) -> R {
    with(|l| {
        let occs: Vec<(usize, &Occ, i64)> = feeds.iter().filter_map(|(i, k, lead)| Some((*i, l.feeds.get(k)?, *lead))).flat_map(|(i, st, lead)| st.occs.iter().map(move |o| (i, o, lead))).collect();
        f(&occs)
    })
}

/// Probíhá právě teď událost v některém zapnutém kalendáři? Jen paměť (žádné stahování, nezávisí na viditelnosti karty) —
/// pro `suggest::pulse`: během schůzky se proužek u GitHubu nepulsuje.
pub(crate) fn meeting_now(cfg: &Config, now: i64) -> bool {
    let keys: Vec<String> = cfg.personal.calendars.iter().filter(|f| f.enabled).filter_map(|f| source::key(f).ok()).collect();
    with(|l| view::any_ongoing(keys.iter().filter_map(|k| l.feeds.get(k)).flat_map(|s| s.occs.iter()), now))
}

/// Po startu jednou cache z disku i pro skrytou kartu (připomínky) — pro všechny zapnuté kalendáře,
/// ať ji pozdější otevření karty nemine; bez sítě.
pub(crate) fn ensure_cache(cfg: &Config) {
    let load = with(|l| {
        if l.cache != CacheLoad::No {
            return false;
        }
        l.cache = CacheLoad::Started;
        for k in cfg.personal.calendars.iter().filter(|f| f.enabled).filter_map(|f| source::key(f).ok()) {
            l.feeds.entry(k).or_default();
        }
        true
    });
    if load {
        load_cache_async();
    }
}

/// Změna configu: zahodit stav odebraných kalendářů (bez kalendářů smazat i cache) a případně stáhnout.
pub(crate) fn on_config(old: &Config, new: &Config) {
    let (o, n) = (&old.personal.calendars, &new.personal.calendars);
    if o == n && old.layout == new.layout && old.drawer.sections.calendar == new.drawer.sections.calendar {
        return;
    }
    let keep: Vec<String> = n.iter().filter_map(|f| source::key(f).ok()).collect();
    let removed = with(|l| {
        let before = l.feeds.len();
        l.feeds.retain(|k, _| keep.contains(k));
        l.feeds.len() != before
    });
    if n.is_empty() {
        std::thread::Builder::new().name("calendar-cache".into()).spawn(|| {
            let _ = std::fs::remove_file(cache_path());
        }).ok();
    } else if removed {
        std::thread::Builder::new().name("calendar-cache".into()).spawn(save_cache).ok();
    }
    maybe_refresh(false);
    mark_dirty();
}

/// Termín pro `next_wake` emit vlákna: jen s viditelnou kartou — další změna výběru, nebo watchdog
/// běžícího stahování.
pub(crate) fn next_deadline() -> Option<i64> {
    {
        let cfg = core().cfg.read();
        if !card_visible(&cfg, false) {
            return None;
        }
    }
    with(|l| {
        let watchdog = l.feeds.values().filter(|s| s.loading).filter_map(|s| s.attempt_ms).map(|a| a + 2 * TIMEOUT.as_millis() as i64 + 1).min();
        [l.next_change, watchdog].into_iter().flatten().min()
    })
}

/// Emit vlákno po probuzení: prošel čas změny výběru → nový snapshot.
pub(crate) fn tick(now: i64) {
    let due = with(|l| l.next_change.is_some_and(|t| t <= now));
    if !due {
        return;
    }
    let visible = {
        let cfg = core().cfg.read();
        card_visible(&cfg, false)
    };
    if visible {
        with(|l| l.next_change = None);
        mark_dirty();
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalInfo {
    /// název z configu, jinak z feedu, jinak doména
    pub name: String,
    pub color: Option<String>,
    /// jen doména adresy (celá adresa je tajemství); jiné zdroje než ICS prázdné
    pub domain: String,
    /// zdroj: ics · outlook-local · graph · google (`source::SourceKind`)
    pub source: &'static str,
    /// nápověda bez chyby: `outlook-closed` (karta nabídne „Načíst z Outlooku“)
    pub hint: Option<&'static str>,
    pub enabled: bool,
    /// off (vypnutý) · invalid (špatná adresa) · ok · stale · loading · unavailable · idle
    pub status: &'static str,
    pub error: Option<String>,
    pub fetched_ms: Option<i64>,
    pub events: usize,
    /// w8: Q — zdroj z přihlášeného účtu (`microsoft` / `google`, `accounts::calsync`); `None` = odkaz ICS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<&'static str>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalendarView {
    /// off (karta/záložka vypnutá) · unset · ok · stale · loading · unavailable · idle (`overall`)
    pub status: &'static str,
    pub days: Vec<view::Day>,
    pub more: usize,
    /// všechny kalendáře z configu ve stejném pořadí (index = `Item.cal`)
    pub calendars: Vec<CalInfo>,
    pub loading: bool,
}

/// Pro snapshot — jen paměť. Nesmí sahat na `core().cfg` (snapshot drží čtecí zámek).
pub(crate) fn view(cfg: &Config, now: i64) -> CalendarView {
    if !(cfg.modes.personal_enabled && cfg.drawer.sections.calendar) {
        return CalendarView { status: "off", days: vec![], more: 0, calendars: vec![], loading: false };
    }
    // w8: Q — kalendáře přihlášených účtů (jen paměť) jako další zdroje za odkazy ICS
    let accounts = crate::accounts::calsync::infos(cfg, now);
    with(|l| {
        let mut cals = Vec::new();
        let mut occs: Vec<(usize, &Occ)> = Vec::new();
        let mut loading = false;
        for (i, f) in cfg.personal.calendars.iter().enumerate() {
            let key = source::unavailable(f).map_or_else(|| source::key(f), Err);
            let st = key.as_ref().ok().and_then(|k| l.feeds.get(k));
            let feed_name = st.map(|s| s.name.as_str()).unwrap_or("");
            let name = [f.name.trim(), feed_name].into_iter().find(|n| !n.is_empty()).map_or_else(|| source::fallback_name(f), str::to_string);
            let hint = st.and_then(|s| s.hint);
            let (status, error, fetched_ms, events) = match (&key, st) {
                _ if !f.enabled => ("off", None, None, 0),
                (Err(e), _) => ("invalid", Some(e.clone()), None, 0),
                (Ok(_), None) => ("idle", None, None, 0),
                (Ok(_), Some(s)) => {
                    let live = loading_live(s.loading, s.attempt_ms, now);
                    loading |= live;
                    let error = s.error.clone().or_else(|| (s.loading && !live).then(|| stuck().to_string())).or_else(|| s.hint.map(|_| super::outlook::closed().to_string()));
                    occs.extend(s.occs.iter().map(|o| (i, o)));
                    (feed_status(s.fetched_ms.is_some(), error.is_some(), live, is_old(s.fetched_ms, now)), error, s.fetched_ms, s.events)
                }
            };
            let source = f.source.id();
            cals.push(CalInfo { name, color: f.color.clone(), domain: source::domain(f), source, hint, enabled: f.enabled, status, error, fetched_ms, events, account: None });
        }
        for (info, list) in &accounts {
            let i = cals.len();
            loading |= info.status == "loading";
            occs.extend(list.iter().map(|o| (i, o)));
            cals.push(info.clone());
        }
        let picked = view::pick(&occs, now, &SystemLocal, CARD_ROWS);
        l.next_change = picked.next_change;
        let st: Vec<&str> = cals.iter().filter(|c| c.enabled).map(|c| c.status).collect();
        CalendarView { status: overall(&st), days: picked.days, more: picked.more, calendars: cals, loading }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_policy() {
        let now = 10_000_000_000;
        assert!(needs_fetch(now, None, None, false));
        assert!(!needs_fetch(now, Some(now - MAX_AGE_MS + 1), Some(now - MAX_AGE_MS + 1), false));
        assert!(needs_fetch(now, Some(now - MAX_AGE_MS), Some(now - MAX_AGE_MS), false));
        assert!(!needs_fetch(now, None, Some(now - 1000), true));
        // zaseklý pokus (déle než 2× timeout) už nebrání novému
        assert!(needs_fetch(now, None, Some(now - RETRY_MS), true));
        assert!(!needs_fetch(now, None, Some(now - 1000), false));
        assert!(needs_fetch(now, None, Some(now - RETRY_MS), false));
        assert!(loading_live(true, Some(now - 29_999), now));
        assert!(!loading_live(true, Some(now - 30_000), now));
        assert!(!loading_live(false, Some(now), now));
    }

    #[test]
    fn statuses() {
        let cases = [
            ((true, false, false, false), "ok"),
            ((true, true, false, false), "stale"),
            ((true, false, true, true), "stale"),
            ((false, false, true, true), "loading"),
            ((false, true, false, true), "unavailable"),
            ((false, false, false, true), "idle"),
        ];
        for ((d, e, l, o), want) in cases {
            assert_eq!(feed_status(d, e, l, o), want, "{d} {e} {l} {o}");
        }
        let cases: &[(&[&str], &str)] = &[
            (&[], "unset"),
            (&["ok"], "ok"),
            (&["ok", "ok"], "ok"),
            (&["ok", "unavailable"], "stale"),
            (&["stale"], "stale"),
            (&["loading", "idle"], "loading"),
            (&["invalid"], "unavailable"),
            (&["idle"], "idle"),
        ];
        for (st, want) in cases {
            assert_eq!(overall(st), *want, "{st:?}");
        }
    }

    #[test]
    fn human_errors_without_codes() {
        for code in [400, 401, 403, 404, 410, 429, 500, 503] {
            let cs = http_error(code);
            let en = crate::i18n::with_lang(crate::uiconf::Lang::En, || http_error(code));
            assert!(!cs.contains(&code.to_string()) && !en.contains(&code.to_string()), "{code}");
            assert!(en.is_ascii() || en.contains('—'), "{en}");
            assert_ne!(cs, en);
        }
        assert!(http_error(404).contains("nový"));
    }

}
