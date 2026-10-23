//! Stav aktualizací a vlákno `update` (rozdělené z `update.rs`, který drží vydání, ověření, stažení a
//! spuštění instalátoru): plán kontrol, tiché stažení předem (`updcache.rs`), „Co je nového“
//! (`updseen.rs`), návrat k předchozí verzi a pohled pro UI (`Snapshot.update`).

use crate::config::UpdateConfig;
use crate::update::{CURRENT, FIRST_CHECK, Release, Verdict, Version, download_and_verify, evaluate, evaluate_tag, fetch_release, fetch_release_tag, launch_installer, parse_version, verified_cached};
use crate::updcache::{self, Fetch, Rollback, Step};
use crate::updseen::{self, Seen};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Stav pro UI (`Snapshot.update`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    /// kontrola vypnutá v configu
    Off,
    /// zapnuto, první kontrola ještě neproběhla (60 s po startu)
    Neovereno,
    Aktualni,
    Dostupna { verze: String, poznamky: String, publikovano: String },
    /// offline / repo nenalezeno / limit API (a gh nepomohlo) / vydání nejde ověřit — poctivě s důvodem
    NelzeOverit { duvod: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    #[serde(flatten)]
    pub status: Status,
    /// verze běžícího Ledge
    pub current: &'static str,
    pub repo: String,
    /// kontrola právě běží
    pub checking: bool,
    /// unix ms poslední dokončené kontroly
    pub checked_ms: Option<i64>,
    /// stahuje / ověřuje / spouští instalátor
    pub installing: bool,
    pub install_error: Option<String>,
    /// tiché stažení dostupné verze
    pub fetch: Fetch,
    /// poznámky běžící verze (z cache, bez sítě); `unread` = ukázat jednorázové upozornění v šuplíku
    pub whats_new: Option<WhatsNew>,
    /// „Vrátit předchozí verzi“ — `None` = není kam
    pub rollback: Option<RollbackView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WhatsNew {
    pub verze: String,
    pub poznamky: String,
    pub unread: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackView {
    pub verze: String,
    /// instalátor je v cache (jinak se na klik stáhne)
    pub local: bool,
}

struct State {
    status: Status,
    repo: String,
    checking: bool,
    checked_ms: Option<i64>,
    release: Option<Release>,
    installing: bool,
    install_error: Option<String>,
    fetch: Fetch,
    /// `<local>/updates`
    root: PathBuf,
    seen: Seen,
    /// (verze, poznámky) běžící verze z manifestu v cache
    notes: Option<(String, String)>,
    rollback: Rollback,
    /// složka (tag) právě běžícího stažení — úklid ji nesmí smazat
    busy_dir: Option<String>,
}

impl State {
    fn new(cfg: &UpdateConfig, root: PathBuf) -> Self {
        Self {
            status: idle_status(cfg),
            repo: cfg.repo.clone(),
            checking: false,
            checked_ms: None,
            release: None,
            installing: false,
            install_error: None,
            fetch: Fetch::Idle,
            root,
            seen: Seen::default(),
            notes: None,
            rollback: Rollback::None,
            busy_dir: None,
        }
    }
}

enum Msg {
    Check,
    Config(UpdateConfig),
    /// „Zkusit znovu“ po selhaném stažení (i s vypnutým tichým stažením)
    Fetch,
}

type Notify = Arc<dyn Fn() + Send + Sync>;

pub struct Updater {
    tx: mpsc::Sender<Msg>,
    st: Arc<Mutex<State>>,
    notify: Notify,
}

fn idle_status(cfg: &UpdateConfig) -> Status {
    if cfg.check { Status::Neovereno } else { Status::Off }
}

fn interval(cfg: &UpdateConfig) -> Duration {
    Duration::from_secs(u64::from(cfg.interval_h.clamp(1, 48)) * 3600)
}

/// Kdy proběhne další automatická kontrola. `None` = vypnuto (vlákno spí, dokud nepřijde zpráva).
/// Termín v minulosti (zapnuto po dlouhé pauze, změna repa) = hned. Čistá funkce.
pub fn next_check(enabled: bool, start: Instant, last: Option<Instant>, every: Duration) -> Option<Instant> {
    enabled.then(|| match last {
        Some(l) => l + every,
        None => start + FIRST_CHECK,
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Tiché stažení platí jen se zapnutou kontrolou.
fn auto_fetch(cfg: &UpdateConfig) -> bool {
    cfg.check && cfg.prefetch
}

fn tag_of(v: Version) -> String {
    format!("v{}.{}.{}", v.0, v.1, v.2)
}

fn ver_of(v: Version) -> String {
    format!("{}.{}.{}", v.0, v.1, v.2)
}

impl Updater {
    /// `root` = cache instalátorů (`<local>/updates`; úklid a `seen.json` až ve vlákně `update`).
    pub fn start(cfg: UpdateConfig, root: PathBuf, notify: Notify) -> Self {
        let st = Arc::new(Mutex::new(State::new(&cfg, root)));
        let (tx, rx) = mpsc::channel();
        let (st2, n2) = (st.clone(), notify.clone());
        std::thread::Builder::new().name("update".into()).spawn(move || worker(rx, st2, n2, cfg)).ok();
        Self { tx, st, notify }
    }

    pub fn set_config(&self, cfg: UpdateConfig) {
        {
            let mut s = self.st.lock();
            if s.repo != cfg.repo {
                // jiné repo = dosavadní výsledek už neplatí
                s.repo = cfg.repo.clone();
                s.release = None;
                s.checked_ms = None;
                s.status = idle_status(&cfg);
                reset_fetch(&mut s);
            } else if !cfg.check {
                s.status = Status::Off;
                s.release = None;
                reset_fetch(&mut s);
            } else if s.status == Status::Off {
                s.status = Status::Neovereno;
            }
        }
        let _ = self.tx.send(Msg::Config(cfg));
        (self.notify)();
    }

    /// Ruční kontrola (i s vypnutou automatickou) — výsledek přijde přes `notify`.
    pub fn check_now(&self) {
        let _ = self.tx.send(Msg::Check);
    }

    /// „Zkusit znovu“ — stáhne dostupnou verzi ve vlákně `update`; výsledek přijde přes `notify`.
    pub fn retry_fetch(&self) {
        let _ = self.tx.send(Msg::Fetch);
    }

    /// Upozornění „Co je nového“ přečteno (klik) → `seen.json`.
    pub fn mark_notes_read(&self) -> Result<(), String> {
        let (root, seen) = {
            let mut s = self.st.lock();
            s.seen = updseen::mark_read(&s.seen, CURRENT);
            (s.root.clone(), s.seen.clone())
        };
        (self.notify)();
        updseen::save(&root, &seen)
    }

    pub fn view(&self) -> UpdateView {
        let s = self.st.lock();
        UpdateView {
            status: s.status.clone(),
            current: CURRENT,
            repo: s.repo.clone(),
            checking: s.checking,
            checked_ms: s.checked_ms,
            installing: s.installing,
            install_error: s.install_error.clone(),
            fetch: s.fetch.clone(),
            whats_new: s.notes.as_ref().map(|(verze, poznamky)| WhatsNew { verze: verze.clone(), poznamky: poznamky.clone(), unread: updseen::unread(&s.seen, CURRENT, true) }),
            rollback: match &s.rollback {
                Rollback::Local { version, .. } => Some(RollbackView { verze: ver_of(*version), local: true }),
                Rollback::Remote { version } => Some(RollbackView { verze: ver_of(*version), local: false }),
                Rollback::None => None,
            },
        }
    }

    /// Instalátor dostupné verze — z cache (znovu ověřený), jinak ho stáhne a ověří — odpojeně spustí
    /// (pak spustí `installed_exe`). Při `Ok` musí volající Ledge ukončit. Blokuje (případné stažení) —
    /// volat mimo UI vlákno. Chyba stažení jde do `fetch` („Zkusit znovu“), chyba spuštění do `install_error`.
    pub fn install(&self, installed_exe: &Path) -> Result<String, String> {
        let (rel, repo, root) = {
            let mut s = self.st.lock();
            if s.installing {
                return Err(crate::i18n::tr("Instalace už běží.", "Installation is already running.").into());
            }
            if matches!(s.fetch, Fetch::Downloading { .. }) {
                return Err(crate::i18n::tr("Nová verze se ještě stahuje — tlačítko se odemkne, až bude hotovo.", "The new version is still downloading — the button unlocks when it's done.").into());
            }
            let rel = s.release.clone().ok_or_else(|| crate::i18n::tr("Není co instalovat — zkontroluj aktualizace znovu.", "Nothing to install — check for updates again."))?;
            s.installing = true;
            s.install_error = None;
            (rel, s.repo.clone(), s.root.clone())
        };
        (self.notify)();
        let entries = updcache::scan(&root);
        let setup = match updcache::cached(&entries, &rel).map(|e| verified_cached(&root, e)) {
            Some(Ok(p)) => Ok(p),
            // není v cache, nebo neprošel novým ověřením → stáhnout teď (s průběhem)
            _ => fetch_now(&self.st, &self.notify, &rel, &repo, &root),
        };
        let res = setup.and_then(|setup| launch_installer(&setup, installed_exe).map(|()| setup).inspect_err(|e| self.st.lock().install_error = Some(e.clone())));
        if res.is_err() {
            self.st.lock().installing = false;
        }
        (self.notify)();
        res.map(|setup| trf!("{} → {} (spustí se {})", "{} → {} (then starts {})", rel.tag, setup.display(), installed_exe.display()))
    }

    /// Spustí ověřený instalátor předchozí verze: z cache, jinak ho stáhne z vydání `v<předchozí>` téhož
    /// repa se stejným ověřením. Při `Ok` musí volající Ledge ukončit. Blokuje — volat mimo UI vlákno.
    pub fn rollback(&self, installed_exe: &Path) -> Result<String, String> {
        let (plan, repo, root, seen) = {
            let mut s = self.st.lock();
            if s.installing {
                return Err(crate::i18n::tr("Instalace už běží.", "Installation is already running.").into());
            }
            // souběžně s tichým stažením ne: sdílí cache a úklid (po `installing` už žádné nezačne)
            if matches!(s.fetch, Fetch::Downloading { .. }) {
                return Err(crate::i18n::tr("Nová verze se právě stahuje — vrátit se půjde, až stahování doběhne.", "A new version is downloading right now — you can go back once it finishes.").into());
            }
            s.installing = true;
            (s.rollback.clone(), s.repo.clone(), s.root.clone(), s.seen.clone())
        };
        (self.notify)();
        let setup = match &plan {
            Rollback::Local { version, .. } => {
                let entries = updcache::scan(&root);
                updcache::of_version(&entries, *version)
                    .ok_or_else(|| crate::i18n::tr("Instalátor předchozí verze z cache zmizel.", "The previous version's installer is gone from the cache.").to_string())
                    .and_then(|e| {
                        let m = e.manifest.as_ref().map(|m| m.sha256.as_str()).unwrap_or_default();
                        updseen::check_known(&seen, &ver_of(*version), m).and_then(|()| verified_cached(&root, e))
                    })
            }
            Rollback::Remote { version } => {
                let tag = tag_of(*version);
                fetch_release_tag(&repo, &tag).and_then(|json| evaluate_tag(&json, &tag)).and_then(|rel| {
                    // přepsané staré vydání (jiný instalátor než dřív ověřený) → nic nestahovat
                    updseen::check_known(&seen, &rel.version, &rel.sha256)?;
                    self.st.lock().busy_dir = Some(rel.tag.clone());
                    let res = download_and_verify(&rel, &repo, &root, &|_, _| {});
                    self.st.lock().busy_dir = None;
                    res.inspect(|_| remember_sha(&self.st, &rel))
                })
            }
            Rollback::None => Err(crate::i18n::tr("Není k čemu se vrátit.", "There's nothing to go back to.").into()),
        };
        let res = setup.and_then(|setup| launch_installer(&setup, installed_exe).map(|()| setup));
        if res.is_err() {
            self.st.lock().installing = false;
            // neprošlé ověření smaže složku → plán znovu (z cache se stane stažení)
            refresh_cache(&self.st, None);
        }
        (self.notify)();
        let name = match &plan {
            Rollback::Local { version, .. } | Rollback::Remote { version } => tag_of(*version),
            Rollback::None => "—".into(),
        };
        res.map(|setup| trf!("návrat na {name} → {} (spustí se {})", "rollback to {name} → {} (then starts {})", setup.display(), installed_exe.display()))
    }
}

/// Nové / žádné vydání: rozpracované stažení doběhne, jinak stav stažení od začátku.
fn reset_fetch(s: &mut State) {
    if !matches!(s.fetch, Fetch::Downloading { .. }) {
        s.fetch = Fetch::Idle;
    }
}

/// Stáhne vydání do cache s průběhem ve `fetch` (UI se obnoví jen po celých procentech). Volající už
/// zajistil, že nic jiného nestahuje (`next_step` / `install`). Výsledek: `Ready`, nebo `Failed`.
fn fetch_now(st: &Mutex<State>, notify: &Notify, rel: &Release, repo: &str, root: &Path) -> Result<PathBuf, String> {
    {
        let mut s = st.lock();
        s.fetch = Fetch::Downloading { got: 0, total: None };
        s.busy_dir = Some(rel.tag.clone());
    }
    notify();
    let last = AtomicU64::new(0);
    let progress = |got: u64, total: Option<u64>| {
        if got != 0 && !updcache::progress_due(last.load(Ordering::Relaxed), got, total) {
            return;
        }
        last.store(got, Ordering::Relaxed);
        st.lock().fetch = Fetch::Downloading { got, total };
        notify();
    };
    let res = download_and_verify(rel, repo, root, &progress);
    {
        let mut s = st.lock();
        s.busy_dir = None;
        s.fetch = match &res {
            Ok(_) => Fetch::Ready,
            Err(e) => Fetch::Failed { duvod: e.clone() },
        };
    }
    if res.is_ok() {
        remember_sha(st, rel);
    }
    crate::log(&format!("update: stažení {} → {}", rel.tag, if res.is_ok() { "ověřeno" } else { "selhalo" }));
    refresh_cache(st, parse_version(&rel.tag));
    notify();
    res
}

/// Ověřený instalátor: zapamatovat jeho SHA256 u verze (`seen.json`, mimo cache) pro pozdější návrat.
fn remember_sha(st: &Mutex<State>, rel: &Release) {
    let (root, seen) = {
        let mut s = st.lock();
        s.seen = updseen::remember(&s.seen, &rel.version, &rel.sha256);
        (s.root.clone(), s.seen.clone())
    };
    if let Err(e) = updseen::save(&root, &seen) {
        crate::log(&format!("update: seen.json — {e}"));
    }
}

/// Po kontrole / „Zkusit znovu“: dostupnou verzi z cache označit připravenou, nebo stáhnout (`next_step`).
fn prefetch(st: &Mutex<State>, notify: &Notify, repo: &str, auto: bool, user: bool) {
    let Some((rel, root)) = ({
        let s = st.lock();
        s.release.clone().map(|r| (r, s.root.clone()))
    }) else {
        return;
    };
    let in_cache = updcache::cached(&updcache::scan(&root), &rel).is_some();
    {
        let mut s = st.lock();
        if s.release.as_ref() != Some(&rel) || s.installing {
            return;
        }
        match updcache::next_step(auto, user, true, in_cache, &s.fetch) {
            Step::Nothing => return,
            Step::MarkReady => s.fetch = Fetch::Ready,
            Step::Download => s.fetch = Fetch::Downloading { got: 0, total: None },
        }
    }
    if in_cache {
        notify();
        return;
    }
    let _ = fetch_now(st, notify, &rel, repo, &root);
}

/// Přečte cache: cíl návratu a poznámky běžící verze; uklidí, co už není potřeba (`plan_cleanup`).
/// `pending` = verze dostupného vydání, když je známá.
fn refresh_cache(st: &Mutex<State>, pending: Option<Version>) {
    if let Some(cur) = parse_version(CURRENT) {
        refresh_cache_at(st, cur, pending);
    }
}

fn refresh_cache_at(st: &Mutex<State>, cur: Version, pending: Option<Version>) {
    let (root, previous, busy) = {
        let s = st.lock();
        (s.root.clone(), s.seen.previous.as_deref().and_then(parse_version), s.busy_dir.clone())
    };
    let mut entries = updcache::scan(&root);
    let plan = updcache::plan_rollback(&entries, cur, previous);
    let gone = updcache::plan_cleanup(&entries, cur, &plan, pending, busy.as_deref());
    if !gone.is_empty() {
        let n = updcache::remove(&root, &gone);
        crate::log(&format!("update: úklid cache — smazáno {n} z {}", gone.len()));
        entries.retain(|e| !gone.contains(&e.name));
    }
    let notes = updcache::of_version(&entries, cur).and_then(|e| e.manifest.clone()).filter(|m| !m.notes.trim().is_empty()).map(|m| (m.version, m.notes));
    let mut s = st.lock();
    s.rollback = plan;
    s.notes = notes;
}

/// Jednou při startu vlákna: naposledy spuštěná verze (`seen.json`), úklid cache a dřívějšího místa
/// stažení `%TEMP%\ledge-update`.
fn housekeeping(st: &Mutex<State>) {
    let root = st.lock().root.clone();
    let old = updseen::load(&root);
    let seen = updseen::on_start(&old, CURRENT);
    if seen != old
        && let Err(e) = updseen::save(&root, &seen)
    {
        crate::log(&format!("update: seen.json — {e}"));
    }
    st.lock().seen = seen;
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("ledge-update"));
    refresh_cache(st, None);
}

fn worker(rx: mpsc::Receiver<Msg>, st: Arc<Mutex<State>>, notify: Notify, mut cfg: UpdateConfig) {
    let start = Instant::now();
    let mut last: Option<Instant> = None;
    housekeeping(&st);
    notify();
    loop {
        let due = next_check(cfg.check, start, last, interval(&cfg));
        let msg = match due {
            Some(t) => match rx.recv_timeout(t.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            },
        };
        match msg {
            Some(Msg::Config(c)) => {
                if c.repo != cfg.repo {
                    last = None;
                }
                cfg = c;
                continue;
            }
            Some(Msg::Fetch) => {
                prefetch(&st, &notify, &cfg.repo, auto_fetch(&cfg), true);
                continue;
            }
            Some(Msg::Check) | None => {}
        }
        run_check(&cfg.repo, &st, &notify);
        last = Some(Instant::now());
        // „Zkontrolovat teď“ naklikané během kontroly = jedna kontrola; změny configu se nezahodí
        let mut retry = false;
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Config(c) => {
                    if c.repo != cfg.repo {
                        last = None;
                    }
                    cfg = c;
                }
                Msg::Fetch => retry = true,
                Msg::Check => {}
            }
        }
        // tiché stažení hned po kontrole — jednou na verzi; selhání čeká na „Zkusit znovu“
        prefetch(&st, &notify, &cfg.repo, auto_fetch(&cfg), retry);
    }
}

fn run_check(repo: &str, st: &Mutex<State>, notify: &Notify) {
    st.lock().checking = true;
    notify();
    let res = fetch_release(repo);
    let (status, release) = match res.map(|json| evaluate(&json, CURRENT)) {
        Ok(Verdict::Current) => (Status::Aktualni, None),
        Ok(Verdict::Available(r)) => (
            Status::Dostupna { verze: r.version.clone(), poznamky: r.notes.clone(), publikovano: r.published.clone() },
            Some(r),
        ),
        Ok(Verdict::Unverifiable(why)) | Err(why) => (Status::NelzeOverit { duvod: why }, None),
    };
    crate::log(&format!("update: {repo} → {status:?}").chars().take(300).collect::<String>());
    {
        let mut s = st.lock();
        // repo se mezitím změnilo → výsledek patří starému, zahodit
        if s.repo == repo {
            if s.release.as_ref().map(|r| &r.tag) != release.as_ref().map(|r| &r.tag) {
                reset_fetch(&mut s);
            }
            s.status = status;
            s.release = release;
            s.checked_ms = Some(now_ms());
        }
        s.checking = false;
    }
    notify();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, validate};

    #[test]
    fn schedule_first_after_60s_then_interval_never_when_off() {
        let t0 = Instant::now();
        let h6 = Duration::from_secs(6 * 3600);
        assert_eq!(next_check(true, t0, None, h6), Some(t0 + FIRST_CHECK));
        let l = t0 + Duration::from_secs(100);
        assert_eq!(next_check(true, t0, Some(l), h6), Some(l + h6));
        assert_eq!(next_check(false, t0, Some(l), h6), None);
        assert_eq!(next_check(false, t0, None, h6), None);
        let c = crate::config::UpdateConfig { interval_h: 0, ..Default::default() };
        assert_eq!(interval(&c), Duration::from_secs(3600)); // nevalidní config nikdy nevede k busy loopu
    }

    #[test]
    fn status_serializes_for_ui() {
        let v = |status: Status| {
            serde_json::to_value(UpdateView {
                status,
                current: "0.2.0",
                repo: "a/b".into(),
                checking: false,
                checked_ms: None,
                installing: false,
                install_error: None,
                fetch: Fetch::Idle,
                whats_new: None,
                rollback: Some(RollbackView { verze: "0.1.9".into(), local: false }),
            })
            .unwrap()
        };
        assert_eq!(v(Status::Off)["state"], "off");
        assert_eq!(v(Status::Neovereno)["state"], "neovereno");
        assert_eq!(v(Status::Aktualni)["state"], "aktualni");
        let d = v(Status::Dostupna { verze: "0.2.14".into(), poznamky: "- x".into(), publikovano: "2026".into() });
        assert_eq!((d["state"].as_str(), d["verze"].as_str(), d["current"].as_str()), (Some("dostupna"), Some("0.2.14"), Some("0.2.0")));
        let n = v(Status::NelzeOverit { duvod: "offline".into() });
        assert_eq!((n["state"].as_str(), n["duvod"].as_str()), (Some("nelze_overit"), Some("offline")));
        assert!(n.get("checkedMs").is_some() && n.get("installError").is_some());
        assert_eq!((n["fetch"]["state"].as_str(), n["whatsNew"].is_null(), n["rollback"]["verze"].as_str()), (Some("idle"), true, Some("0.1.9")));
    }

    fn rel(tag: &str, body: &str) -> Release {
        let v = parse_version(tag).unwrap();
        Release {
            tag: tag.into(),
            version: format!("{}.{}.{}", v.0, v.1, v.2),
            notes: body.into(),
            published: String::new(),
            asset: format!("App_{}.{}.{}_x64-setup.exe", v.0, v.1, v.2),
            // sha256("test")
            sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
            digest: None,
            download_url: None,
        }
    }

    /// Instalátor do cache tak, jak ho nechá `download_and_verify` (soubor + manifest).
    fn put(root: &Path, r: &Release, content: &str) {
        let dir = root.join(&r.tag);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(&r.asset), content).unwrap();
        updcache::write_manifest(&dir, &updcache::Manifest::of(r)).unwrap();
    }


    /// Bez sítě: vydání už v cache → `Ready` hned po kontrole (jednou na verzi), s vypnutým tichým
    /// stažením i bez cache nic; poznámky běžící verze a návrat se čtou z cache; úklid drží jen potřebné.
    #[test]
    fn prefetch_and_cache_view_without_network() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let cfg = UpdateConfig::default();
        let st = Mutex::new(State::new(&cfg, root.clone()));
        let calls = Arc::new(AtomicU64::new(0));
        let c2 = calls.clone();
        let notify: Notify = Arc::new(move || {
            c2.fetch_add(1, Ordering::Relaxed);
        });
        // pevné verze (ne CARGO_PKG_VERSION — v repu je 0.2.0, starší by nebyla)
        let [prev, now, next, stale] = ["0.2.14", "0.2.15", "0.2.16", "0.2.12"].map(|v| parse_version(v).unwrap());
        let tag = |v: Version| format!("v{}.{}.{}", v.0, v.1, v.2);
        let newer = rel(&tag(next), "- feat: nové");
        // nic v cache, tiché stažení vypnuté → žádný krok (a žádná síť)
        st.lock().release = Some(newer.clone());
        prefetch(&st, &notify, "acme/app", false, false);
        assert_eq!(st.lock().fetch, Fetch::Idle);
        // v cache (stažené dřív) → připraveno bez sítě
        put(&root, &newer, "test");
        prefetch(&st, &notify, "acme/app", true, false);
        assert_eq!(st.lock().fetch, Fetch::Ready);
        let n = calls.load(Ordering::Relaxed);
        prefetch(&st, &notify, "acme/app", true, false);
        assert_eq!(calls.load(Ordering::Relaxed), n, "podruhé nic — jednou na verzi");
        // běžící a předchozí verze v cache, stará navíc → úklid ji smaže, poznámky běžící verze se ukážou
        put(&root, &rel(&tag(now), "- fix: běžící"), "test");
        put(&root, &rel(&tag(prev), ""), "test");
        put(&root, &rel(&tag(stale), ""), "test");
        st.lock().seen = updseen::on_start(&Seen { seen: Some(ver_of(prev)), ..Seen::default() }, "0.2.15");
        refresh_cache_at(&st, now, Some(next));
        let s = st.lock();
        assert_eq!(s.rollback, Rollback::Local { tag: tag(prev), version: prev });
        assert_eq!(s.notes, Some((ver_of(now), "- fix: běžící".to_string())));
        assert!(updseen::unread(&s.seen, "0.2.15", true));
        drop(s);
        let left: Vec<String> = updcache::scan(&root).into_iter().map(|e| e.name).collect();
        assert_eq!(left.len(), 3, "{left:?}");
        assert!(!left.contains(&tag(stale)));
        // podvržený instalátor v cache: nové ověření ho odmítne a složku smaže
        std::fs::write(root.join(tag(prev)).join(&rel(&tag(prev), "").asset), "jiny obsah").unwrap();
        let e = updcache::scan(&root).into_iter().find(|e| e.name == tag(prev)).unwrap();
        assert!(verified_cached(&root, &e).is_err());
        assert!(!root.join(tag(prev)).exists());
        refresh_cache_at(&st, now, Some(next));
        assert_eq!(st.lock().rollback, Rollback::Remote { version: prev }, "z cache se stane stažení podle tagu");
        // ověřený projde
        let e = updcache::scan(&root).into_iter().find(|e| e.name == tag(next)).unwrap();
        assert!(verified_cached(&root, &e).is_ok());
    }

    #[test]
    fn config_defaults_old_config_and_validation() {
        // starý config bez bloku update → zapnuto, repo jiri-hyan/ledge-app, 6 h
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert!(cfg.update.check);
        assert_eq!(cfg.update.repo, "jiri-hyan/ledge-app");
        assert_eq!(cfg.update.interval_h, 6);
        // starý blok update bez `prefetch` → tiché stažení zapnuté, ale platí jen spolu s kontrolou
        let cfg: Config = serde_json::from_str(r#"{"update":{"intervalH":12}}"#).unwrap();
        assert_eq!((cfg.update.check, cfg.update.interval_h, cfg.update.prefetch), (true, 12, true));
        assert!(auto_fetch(&cfg.update));
        let off: Config = serde_json::from_str(r#"{"update":{"check":false}}"#).unwrap();
        assert!(off.update.prefetch && !auto_fetch(&off.update), "vypnutá kontrola = nic se nestahuje");
        let no: Config = serde_json::from_str(r#"{"update":{"prefetch":false}}"#).unwrap();
        assert!(no.update.check && !auto_fetch(&no.update));
        let schema = crate::config::schema_json();
        assert!(schema.contains("intervalH") && schema.contains("prefetch"));
        let with = |j: &str| -> Result<(), String> { validate(&serde_json::from_str::<Config>(j).map_err(|e| e.to_string())?) };
        assert!(with(r#"{"update":{"intervalH":1}}"#).is_ok());
        assert!(with(r#"{"update":{"intervalH":48}}"#).is_ok());
        assert!(with(r#"{"update":{"intervalH":0}}"#).unwrap_err().contains("1–48"));
        assert!(with(r#"{"update":{"intervalH":49}}"#).is_err());
        for bad in ["", "ledge", "a/b/c", "/ledge", "jiri-hyan/", "jiri hyan/ledge", "a/..", "https://github.com/jiri-hyan/ledge-app"] {
            assert!(with(&format!(r#"{{"update":{{"repo":"{bad}"}}}}"#)).is_err(), "{bad}");
        }
        assert!(with(r#"{"update":{"repo":"Some_Org/repo.name-2"}}"#).is_ok());
        // repo se validuje i s vypnutou kontrolou — chyba se neukáže až po zapnutí
        assert!(with(r#"{"update":{"check":false,"repo":"x"}}"#).is_err());
    }

    /// Návrat neběží souběžně se stažením (bez sítě: odmítne hned) a úklid nesmaže složku běžícího stažení;
    /// přepsaný instalátor v cache (jiný SHA256 než dřív ověřený) se nespustí.
    #[test]
    fn rollback_waits_for_download_and_cleanup_spares_busy_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let (tx, _rx) = mpsc::channel();
        let mut state = State::new(&UpdateConfig::default(), root.clone());
        state.fetch = Fetch::Downloading { got: 10, total: Some(100) };
        state.rollback = Rollback::Remote { version: parse_version("0.2.14").unwrap() };
        let up = Updater { tx, st: Arc::new(Mutex::new(state)), notify: Arc::new(|| {}) };
        let err = up.rollback(Path::new(r"C:\nikde\ledge.exe")).unwrap_err();
        assert!(err.contains("stahuje"), "{err}");
        assert!(!up.st.lock().installing, "odmítnutí nic nezamklo");
        // běžící stažení v0.2.16 (zatím jen .part, bez manifestu) přežije úklid
        std::fs::create_dir_all(root.join("v0.2.16")).unwrap();
        std::fs::write(root.join("v0.2.16").join("App_0.2.16_x64-setup.exe.part"), "x").unwrap();
        std::fs::create_dir_all(root.join("v0.2.10")).unwrap();
        up.st.lock().busy_dir = Some("v0.2.16".into());
        refresh_cache_at(&up.st, parse_version("0.2.15").unwrap(), None);
        assert!(root.join("v0.2.16").is_dir() && !root.join("v0.2.10").exists());
        up.st.lock().busy_dir = None;
        refresh_cache_at(&up.st, parse_version("0.2.15").unwrap(), None);
        assert!(!root.join("v0.2.16").exists(), "po doběhnutí se přerušené stažení uklidí");
        // Local návrat: instalátor v cache, ale Ledge si u verze pamatuje jiný SHA256 → odmítne
        let prev = rel("v0.2.14", "");
        put(&root, &prev, "test");
        {
            let mut s = up.st.lock();
            s.fetch = Fetch::Idle;
            s.rollback = Rollback::Local { tag: "v0.2.14".into(), version: parse_version("0.2.14").unwrap() };
            s.seen = updseen::remember(&Seen::default(), "0.2.14", &"b".repeat(64));
        }
        let err = up.rollback(Path::new(r"C:\nikde\ledge.exe")).unwrap_err();
        assert!(err.contains("přepsalo"), "{err}");
        assert!(!up.st.lock().installing);
    }
}
