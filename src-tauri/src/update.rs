//! Upozornění na novou verzi Ledge a instalace na klik. Appka nedrží žádný token: dotaz jde přes
//! veřejné GitHub API (`updatenet.rs`, bez přihlášení), záložně přes přihlášené `gh` (soukromé repo,
//! vyčerpaný limit API) — `gh` tedy není potřeba.
//!
//! **Odkud.** CI (`.github/workflows/release.yml`) po každém pushi na `main` vydá `v0.2.<run>`
//! s NSIS instalátorem `*-setup.exe` a v poznámkách řádek `SHA256 (<instalátor>) = <hex>`.
//! Ledge se ptá `GET /repos/<repo>/releases/latest`, záložně `gh release view --repo <repo> --json …`
//! (nejnovější vydání, bez předběžných). Obě odpovědi čte tatáž [`evaluate`].
//!
//! **Kdy.** 60 s po startu, pak každých `update.intervalH` h. Vlákno `update` spí na kanálu
//! (`recv_timeout` do dalšího termínu) — v klidu nic nedělá. „Zkontrolovat teď“ pošle zprávu do
//! téhož vlákna. Každé volání `gh` je bez okna a s termínem (`gitinfo::capture`, zabití stromu).
//!
//! **Nikdy sám neinstaluje.** Stav `dostupna` jen ukáže chip v hlavičce. Instalace až na klik
//! ([`Updater::install`]): stažení přes HTTPS (záložně `gh release download`) do
//! `%TEMP%\ledge-update\<tag>` → SHA256 souboru se
//! musí shodovat s řádkem v poznámkách (a s `digest`, který počítá GitHub, když ho vydání má) — bez
//! shody se nic nespustí → odpojený `cmd` počká ~2 s, pustí tichý instalátor `/S` a pak nainstalovaný
//! `ledge.exe`; volající (lib.rs) mezitím Ledge ukončí. Běžící dev servery (Job Object) by s Ledge
//! umřely — proto `update_install` v lib.rs chce u nich druhé potvrzení.

use crate::config::UpdateConfig;
use crate::gitinfo::{self, RunError};
use crate::updatenet;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Verze běžící binárky (CI ji zapisuje do Cargo.toml jako `0.2.<run>`).
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// První kontrola po startu — ať start appky nesoupeří se síťí a `gh`.
const FIRST_CHECK: Duration = Duration::from_secs(60);
const VIEW_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
fn auth_hint() -> &'static str {
    crate::i18n::tr("spusť v terminálu `gh auth login`", "run `gh auth login` in a terminal")
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

// ---------- verze ----------

/// Čisté `MAJOR.MINOR.PATCH` (bez předběžné verze a metadat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u64, pub u64, pub u64);

/// `v0.2.14` / `0.2.14` → verze. Předběžné (`-rc1`), s metadaty (`+b`), s úvodními nulami nebo jiného
/// tvaru → `None` (takové vydání Ledge nenabízí).
pub fn parse_version(s: &str) -> Option<Version> {
    let s = s.trim();
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let mut parts = s.split('.');
    let mut n = [0u64; 3];
    for slot in &mut n {
        let p = parts.next()?;
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) || (p.len() > 1 && p.starts_with('0')) {
            return None;
        }
        *slot = p.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(Version(n[0], n[1], n[2]))
}

/// Je `tag` novější než `current`? `None` = jedna ze stran není čisté x.y.z → ignorovat.
pub fn newer(tag: &str, current: &str) -> Option<bool> {
    Some(parse_version(tag)? > parse_version(current)?)
}

// ---------- odpověď gh ----------

/// Vydání z `gh release view --json` (camelCase) i z REST API (snake_case).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhRelease {
    #[serde(alias = "tag_name")]
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
    #[serde(default, alias = "published_at")]
    published_at: Option<String>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    /// `sha256:<hex>` — GitHub počítá sám při nahrání (starší vydání ho nemusí mít)
    #[serde(default)]
    digest: Option<String>,
    /// API: adresa ke stažení
    #[serde(default)]
    browser_download_url: Option<String>,
    /// gh: adresa ke stažení (v API je tu adresa assetu v API — tu `download_ok` odmítne)
    #[serde(default)]
    url: Option<String>,
}

/// Nové vydání, které jde nainstalovat (má instalátor i kontrolní součet).
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub tag: String,
    pub version: String,
    pub notes: String,
    pub published: String,
    /// jméno `*-setup.exe` (jen `[A-Za-z0-9._-]`)
    pub asset: String,
    /// SHA256 z poznámek k vydání (malými písmeny)
    pub sha256: String,
    /// SHA256 z `digest` assetu, když ho GitHub uvádí
    pub digest: Option<String>,
    /// HTTPS adresa instalátoru z odpovědi (ověří se `updatenet::download_ok` až při stažení)
    pub download_url: Option<String>,
}

/// Výsledek jedné kontroly.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Current,
    Available(Release),
    Unverifiable(String),
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Bezpečné jméno souboru assetu (jde do `--pattern` a do cesty) a opravdu instalátor.
fn safe_asset(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && name.to_ascii_lowercase().ends_with("-setup.exe")
}

/// Kontrolní součet instalátoru z poznámek: řádek `SHA256 (<asset>) = <64 hex>` (jak ho píše CI).
pub fn sha_for(body: &str, asset: &str) -> Option<String> {
    let tag = format!("({asset})");
    body.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("SHA256") && l.contains(&tag))
        .find_map(|l| l.split(|c: char| c.is_whitespace() || c == '`' || c == '=').rfind(|t| is_hex64(t)).map(str::to_ascii_lowercase))
}

/// Poznámky pro UI — bez řádků s kontrolním součtem.
pub fn notes_of(body: &str) -> String {
    body.lines().filter(|l| !l.trim_start().starts_with("SHA256")).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// JSON z `gh release view --json tagName,name,body,assets,publishedAt` → verdikt. Čistá funkce.
pub fn evaluate(json: &str, current: &str) -> Verdict {
    let Ok(r) = serde_json::from_str::<GhRelease>(json) else {
        return Verdict::Unverifiable(crate::i18n::tr("GitHub vrátil nečitelnou odpověď", "GitHub returned an unreadable answer").into());
    };
    match newer(&r.tag_name, current) {
        None => Verdict::Unverifiable(trf!("nejnovější vydání má tag „{}“ (není vX.Y.Z) — nenabízím", "the latest release is tagged “{}” (not vX.Y.Z) — not offering it", r.tag_name)),
        Some(false) => Verdict::Current,
        Some(true) => {
            let setups: Vec<&GhAsset> = r.assets.iter().filter(|a| a.name.to_ascii_lowercase().ends_with("-setup.exe")).collect();
            let [asset] = setups.as_slice() else {
                return Verdict::Unverifiable(trf!("vydání {} nemá právě jeden instalátor *-setup.exe ({})", "release {} doesn't have exactly one *-setup.exe installer ({})", r.tag_name, setups.len()));
            };
            if !safe_asset(&asset.name) {
                return Verdict::Unverifiable(trf!("instalátor vydání {} má nečekané jméno „{}“", "the installer of release {} has an unexpected name “{}”", r.tag_name, asset.name));
            }
            let body = r.body.as_deref().unwrap_or("");
            let Some(sha256) = sha_for(body, &asset.name) else {
                return Verdict::Unverifiable(trf!("vydání {} nemá v poznámkách SHA256 instalátoru — nejde ověřit", "release {} has no installer SHA256 in its notes — can't verify it", r.tag_name));
            };
            let digest = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")).filter(|h| is_hex64(h)).map(str::to_ascii_lowercase);
            let v = parse_version(&r.tag_name).expect("newer() už tag ověřil");
            Verdict::Available(Release {
                tag: r.tag_name.trim().to_string(),
                version: format!("{}.{}.{}", v.0, v.1, v.2),
                notes: notes_of(body),
                published: r.published_at.clone().unwrap_or_default(),
                asset: asset.name.clone(),
                sha256,
                digest,
                download_url: asset.browser_download_url.clone().or_else(|| asset.url.clone()),
            })
        }
    }
}

/// Stažený soubor (`got` = SHA256 hex) proti vydání: poznámky vždy, `digest` GitHubu, když je.
pub fn verify(got: &str, rel: &Release) -> Result<(), String> {
    if !got.eq_ignore_ascii_case(&rel.sha256) {
        return Err(trf!("SHA256 staženého instalátoru nesouhlasí s vydáním {} — nic nespouštím.", "The downloaded installer's SHA256 doesn't match release {} — not running anything.", rel.tag));
    }
    if let Some(d) = &rel.digest
        && !got.eq_ignore_ascii_case(d)
    {
        return Err(trf!("SHA256 nesouhlasí s otiskem GitHubu u vydání {} — nic nespouštím.", "SHA256 doesn't match GitHub's digest for release {} — not running anything.", rel.tag));
    }
    Ok(())
}

/// stderr `gh` → lidský důvod (s hintem, co udělat). Čistá funkce.
pub fn gh_error(stderr: &str) -> String {
    let s = stderr.to_lowercase();
    let any = |keys: &[&str]| keys.iter().any(|k| s.contains(k));
    if any(&["gh auth login", "not logged", "authentication required", "bad credentials", "http 401"]) {
        trf!("gh není přihlášené — {}", "gh isn't logged in — {}", auth_hint())
    } else if s.contains("release not found") {
        crate::i18n::tr("v repu zatím není žádné vydání (nebo repo neexistuje)", "the repo has no release yet (or the repo doesn't exist)").into()
    } else if any(&["could not resolve to a repository", "http 404"]) {
        trf!("repo nenalezeno nebo k němu účet v gh nemá přístup — zkontroluj `update.repo` a {}", "repo not found or the gh account can't access it — check `update.repo` and {}", auth_hint())
    } else if any(&["error connecting", "no such host", "dial tcp", "i/o timeout", "tls handshake", "connection"]) {
        crate::i18n::tr("GitHub je nedostupný (offline?)", "GitHub is unreachable (offline?)").into()
    } else {
        let line = stderr.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or(crate::i18n::tr("neznámá chyba", "unknown error"));
        format!("gh: {}", line.chars().take(200).collect::<String>())
    }
}

// ---------- gh ----------

/// `gh.exe` z PATH (absolutní cesta — nikdy ne z cwd), záložně výchozí instalace GitHub CLI.
fn gh_exe() -> Option<PathBuf> {
    let from_path = std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("gh.exe")).find(|p| p.is_absolute() && p.is_file()));
    from_path.or_else(|| {
        ["ProgramFiles", "LOCALAPPDATA"]
            .iter()
            .filter_map(std::env::var_os)
            .flat_map(|base| [PathBuf::from(&base).join("GitHub CLI").join("gh.exe"), PathBuf::from(&base).join("Programs").join("GitHub CLI").join("gh.exe")])
            .find(|p| p.is_file())
    })
}

fn gh(args: &[&str], timeout: Duration) -> Result<String, String> {
    let exe = gh_exe().ok_or_else(|| trf!("gh (GitHub CLI) není nainstalované — nainstaluj ho a {}", "gh (GitHub CLI) isn't installed — install it and {}", auth_hint()))?;
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(std::env::temp_dir())
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("GH_SPINNER_DISABLED", "1")
        .env("NO_COLOR", "1");
    match gitinfo::capture(&mut cmd, None, true, Instant::now() + timeout) {
        Ok(o) if o.success => Ok(o.stdout),
        Ok(o) => Err(gh_error(&o.stderr)),
        Err(RunError::Timeout) => Err(trf!("gh neodpověděl do {} s (offline?)", "gh didn't answer within {} s (offline?)", timeout.as_secs())),
        Err(RunError::Spawn(e)) => Err(trf!("gh nejde spustit: {e}", "can't run gh: {e}")),
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(format!("{:x}", h.finalize()))
}

/// Nejnovější vydání: veřejné API, při chybě `gh` (když je nainstalované). Obě chyby poctivě spolu.
fn fetch_release(repo: &str) -> Result<String, String> {
    updatenet::latest(repo).or_else(|api| {
        if gh_exe().is_none() {
            return Err(api);
        }
        gh(&["release", "view", "--repo", repo, "--json", "tagName,name,body,assets,publishedAt"], VIEW_TIMEOUT).map_err(|g| trf!("{api}; záloha přes gh: {g}", "{api}; fallback via gh: {g}"))
    })
}

fn download_and_verify(rel: &Release, repo: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("ledge-update").join(&rel.tag);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(&rel.asset);
    let _ = std::fs::remove_file(&file);
    let url = rel.download_url.as_deref().filter(|u| updatenet::download_ok(u, repo, &rel.tag, &rel.asset));
    let https = match url {
        Some(u) => updatenet::download(u, &file),
        None => Err(crate::i18n::tr("vydání neuvádí adresu instalátoru", "the release doesn't list the installer's address").into()),
    };
    if let Err(e) = https {
        if gh_exe().is_none() {
            return Err(trf!("Stažení selhalo: {e}", "Download failed: {e}"));
        }
        let dir_s = dir.to_string_lossy();
        gh(&["release", "download", &rel.tag, "--repo", repo, "--pattern", &rel.asset, "--dir", &dir_s, "--clobber"], DOWNLOAD_TIMEOUT)
            .map_err(|g| trf!("Stažení selhalo: {e}; záloha přes gh: {g}", "Download failed: {e}; fallback via gh: {g}"))?;
    }
    if !file.is_file() {
        return Err(trf!("Nic se nestáhlo ({} chybí).", "Nothing was downloaded ({} is missing).", file.display()));
    }
    let got = sha256_file(&file)?;
    if let Err(e) = verify(&got, rel) {
        let _ = std::fs::remove_file(&file);
        return Err(e);
    }
    Ok(file)
}

/// Příkazová řádka pro `cmd.exe`: ~2 s počkat (Ledge se mezitím ukončí sám, ne přes kill instalátoru),
/// tichý instalátor, pak spustit nainstalovaný Ledge. Spouští se i po neúspěšné instalaci (`&`, ne
/// `&&`) — stará kopie tak zůstane běžet a příští kontrola vydání nabídne znovu, místo aby Ledge zmizel.
/// Cesty s `"` nebo `%` (cmd by je rozbil / rozbalil) odmítne. Čistá funkce.
pub fn installer_command(setup: &Path, exe: &Path) -> Result<String, String> {
    for p in [setup, exe] {
        let s = p.to_string_lossy();
        if s.is_empty() || s.contains(['"', '%', '\r', '\n']) {
            return Err(trf!("Cesta „{s}“ se nedá bezpečně předat cmd.", "The path “{s}” can't be passed to cmd safely."));
        }
    }
    Ok(format!("/d /s /c \"ping -n 3 127.0.0.1 >nul & \"{}\" /S & start \"\" \"{}\"\"", setup.display(), exe.display()))
}

/// Spustí instalaci odpojeně od Ledge: vlastní skupina procesů, skrytá konzole (CREATE_NO_WINDOW —
/// ne DETACHED_PROCESS, s ním by si `ping` otevřel viditelné okno konzole) a mimo případný Job Object
/// rodiče (CREATE_BREAKAWAY_FROM_JOB, když ho job nedovolí, tak bez něj).
fn launch_installer(setup: &Path, exe: &Path) -> Result<(), String> {
    let line = installer_command(setup, exe)?;
    let root = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let cmd_exe = root.join("System32").join("cmd.exe");
    let spawn = |flags: u32| {
        Command::new(&cmd_exe)
            .raw_arg(&line)
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(flags)
            .spawn()
    };
    let base = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP;
    spawn(base | CREATE_BREAKAWAY_FROM_JOB).or_else(|_| spawn(base)).map(|_| ()).map_err(|e| trf!("Instalátor nejde spustit: {e}", "Can't run the installer: {e}"))
}

// ---------- stav a vlákno ----------

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
}

struct State {
    status: Status,
    repo: String,
    checking: bool,
    checked_ms: Option<i64>,
    release: Option<Release>,
    installing: bool,
    install_error: Option<String>,
}

enum Msg {
    Check,
    Config(UpdateConfig),
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

impl Updater {
    pub fn start(cfg: UpdateConfig, notify: Notify) -> Self {
        let st = Arc::new(Mutex::new(State {
            status: idle_status(&cfg),
            repo: cfg.repo.clone(),
            checking: false,
            checked_ms: None,
            release: None,
            installing: false,
            install_error: None,
        }));
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
            } else if !cfg.check {
                s.status = Status::Off;
                s.release = None;
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
        }
    }

    /// Stáhne a ověří instalátor dostupné verze a odpojeně ho spustí (pak spustí `installed_exe`).
    /// Při `Ok` musí volající Ledge ukončit. Blokuje (stažení) — volat mimo UI vlákno.
    pub fn install(&self, installed_exe: &Path) -> Result<String, String> {
        let (rel, repo) = {
            let mut s = self.st.lock();
            if s.installing {
                return Err(crate::i18n::tr("Instalace už běží.", "Installation is already running.").into());
            }
            let rel = s.release.clone().ok_or_else(|| crate::i18n::tr("Není co instalovat — zkontroluj aktualizace znovu.", "Nothing to install — check for updates again."))?;
            s.installing = true;
            s.install_error = None;
            (rel, s.repo.clone())
        };
        (self.notify)();
        let res = download_and_verify(&rel, &repo).and_then(|setup| launch_installer(&setup, installed_exe).map(|()| setup));
        if let Err(e) = &res {
            let mut s = self.st.lock();
            s.installing = false;
            s.install_error = Some(e.clone());
        }
        (self.notify)();
        res.map(|setup| trf!("{} → {} (spustí se {})", "{} → {} (then starts {})", rel.tag, setup.display(), installed_exe.display()))
    }
}

fn worker(rx: mpsc::Receiver<Msg>, st: Arc<Mutex<State>>, notify: Notify, mut cfg: UpdateConfig) {
    let start = Instant::now();
    let mut last: Option<Instant> = None;
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
        if let Some(Msg::Config(c)) = msg {
            if c.repo != cfg.repo {
                last = None;
            }
            cfg = c;
            continue;
        }
        run_check(&cfg.repo, &st, &notify);
        last = Some(Instant::now());
        // „Zkontrolovat teď“ naklikané během kontroly = jedna kontrola; změny configu se nezahodí
        while let Ok(m) = rx.try_recv() {
            if let Msg::Config(c) = m {
                if c.repo != cfg.repo {
                    last = None;
                }
                cfg = c;
            }
        }
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

    const FIXTURE: &str = include_str!("fixtures/gh-release-view.json");
    const API_FIXTURE: &str = include_str!("fixtures/github-api-release-latest.json");
    /// sha256("test") — stejný hash je ve fixture v poznámkách i v `digest`
    const SHA_TEST: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    #[test]
    fn version_parse_and_compare_table() {
        #[rustfmt::skip]
        let cases: Vec<(&str, &str, Option<bool>)> = vec![
            ("v0.2.14",       "0.2.0",  Some(true)),
            ("v0.2.14",       "0.2.14", Some(false)),
            ("v0.2.13",       "0.2.14", Some(false)),
            ("v0.2.100",      "0.2.99", Some(true)),   // číselně, ne řetězcově
            ("v0.10.0",       "0.9.9",  Some(true)),
            ("V1.0.0",        "0.2.9",  Some(true)),
            (" 0.2.15 ",      "0.2.14", Some(true)),
            ("v0.3.0-rc1",    "0.2.0",  None),         // předběžná → ignorovat
            ("v0.3.0+build1", "0.2.0",  None),
            ("v0.3",          "0.2.0",  None),
            ("v0.3.0.1",      "0.2.0",  None),
            ("v01.2.3",       "0.2.0",  None),
            ("latest",        "0.2.0",  None),
            ("",              "0.2.0",  None),
            ("v0.2.1",        "nevim",  None),
        ];
        for (tag, cur, want) in cases {
            assert_eq!(newer(tag, cur), want, "{tag} vs {cur}");
        }
        assert!(parse_version(CURRENT).is_some(), "CARGO_PKG_VERSION musí být čisté x.y.z");
    }

    #[test]
    fn evaluate_fixture_newer_current_and_older() {
        let Verdict::Available(r) = evaluate(FIXTURE, "0.2.0") else { panic!("čekám dostupnou verzi") };
        assert_eq!(r.tag, "v0.2.14");
        assert_eq!(r.version, "0.2.14");
        assert_eq!(r.asset, "Ledge_0.2.14_x64-setup.exe");
        assert_eq!(r.sha256, SHA_TEST);
        assert_eq!(r.digest.as_deref(), Some(SHA_TEST));
        assert_eq!(r.published, "2026-10-16T10:00:05Z");
        assert!(r.notes.starts_with("- feat(core)") && !r.notes.contains("SHA256"), "{}", r.notes);
        assert_eq!(evaluate(FIXTURE, "0.2.14"), Verdict::Current);
        assert_eq!(evaluate(FIXTURE, "0.3.0"), Verdict::Current);
    }

    /// Odpověď veřejného REST API (snake_case) dá totéž vydání jako `gh`; stahuje se z `browser_download_url`,
    /// nikdy z API adresy assetu (`url` v API) — tu `download_ok` odmítne.
    #[test]
    fn evaluate_rest_api_fixture_like_gh() {
        let Verdict::Available(api) = evaluate(API_FIXTURE, "0.2.0") else { panic!("čekám dostupnou verzi") };
        let Verdict::Available(gh) = evaluate(FIXTURE, "0.2.0") else { panic!() };
        assert_eq!((&api.tag, &api.sha256, &api.digest, &api.notes, &api.published), (&gh.tag, &gh.sha256, &gh.digest, &gh.notes, &gh.published));
        let repo = "jiri-hyan/ledge-app";
        for r in [&api, &gh] {
            assert!(updatenet::download_ok(r.download_url.as_deref().unwrap(), repo, &r.tag, &r.asset), "{:?}", r.download_url);
        }
        assert!(!updatenet::download_ok("https://api.github.com/repos/jiri-hyan/ledge-app/releases/assets/1", repo, &api.tag, &api.asset));
        assert_eq!(evaluate(API_FIXTURE, "0.2.14"), Verdict::Current);
        // body: null (vydání bez poznámek) → nejde ověřit, ne pád
        let no_body = API_FIXTURE.replacen("\"body\": \"SHA256", "\"body\": null, \"x\": \"SHA256", 1);
        assert!(matches!(evaluate(&no_body, "0.2.0"), Verdict::Unverifiable(_)));
    }

    #[test]
    fn evaluate_refuses_what_cannot_be_verified() {
        let with = |tag: &str, body: &str, assets: &str| {
            format!(r#"{{"tagName":"{tag}","name":"x","body":{},"assets":[{assets}],"publishedAt":"2026-10-16T10:00:05Z"}}"#, serde_json::json!(body))
        };
        let setup = r#"{"name":"Ledge_0.2.14_x64-setup.exe","digest":null}"#;
        let line = format!("SHA256 (Ledge_0.2.14_x64-setup.exe) = {SHA_TEST}");
        let unver = |v: Verdict| matches!(v, Verdict::Unverifiable(_));
        // předběžný / nevalidní tag
        assert!(unver(evaluate(&with("v0.3.0-rc1", &line, setup), "0.2.0")));
        // bez SHA řádku, SHA pro jiný soubor, zkrácený hash
        assert!(unver(evaluate(&with("v0.2.14", "- jen poznámky", setup), "0.2.0")));
        assert!(unver(evaluate(&with("v0.2.14", &format!("SHA256 (jiny-setup.exe) = {SHA_TEST}"), setup), "0.2.0")));
        assert!(unver(evaluate(&with("v0.2.14", "SHA256 (Ledge_0.2.14_x64-setup.exe) = abc", setup), "0.2.0")));
        // žádný / dva instalátory, nebezpečné jméno
        assert!(unver(evaluate(&with("v0.2.14", &line, ""), "0.2.0")));
        assert!(unver(evaluate(&with("v0.2.14", &line, &format!("{setup},{setup}")), "0.2.0")));
        assert!(unver(evaluate(&with("v0.2.14", "SHA256 (..\\x-setup.exe) = 0", r#"{"name":"..\\x-setup.exe"}"#), "0.2.0")));
        // nečitelný JSON
        assert!(unver(evaluate("<html>", "0.2.0")));
        // bez digestu (starší vydání) projde jen s poznámkami; velká písmena v hashi nevadí
        let up = format!("SHA256 (Ledge_0.2.14_x64-setup.exe) = {}", SHA_TEST.to_uppercase());
        let Verdict::Available(r) = evaluate(&with("v0.2.14", &up, setup), "0.2.0") else { panic!() };
        assert_eq!((r.sha256.as_str(), r.digest), (SHA_TEST, None));
    }

    #[test]
    fn verify_needs_both_hashes_to_match() {
        let Verdict::Available(mut r) = evaluate(FIXTURE, "0.2.0") else { panic!() };
        assert!(verify(SHA_TEST, &r).is_ok());
        assert!(verify(&SHA_TEST.to_uppercase(), &r).is_ok());
        assert!(verify(&"0".repeat(64), &r).unwrap_err().contains("nic nespouštím"));
        r.digest = Some("1".repeat(64));
        assert!(verify(SHA_TEST, &r).unwrap_err().contains("GitHubu"));
        r.digest = None;
        assert!(verify(SHA_TEST, &r).is_ok());
        // skutečný soubor: sha256("test")
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x-setup.exe");
        std::fs::write(&f, "test").unwrap();
        assert_eq!(sha256_file(&f).unwrap(), SHA_TEST);
    }

    #[test]
    fn gh_errors_are_honest_with_hint() {
        #[rustfmt::skip]
        let cases: Vec<(&str, &str)> = vec![
            ("To get started with GitHub CLI, please run:  gh auth login\nAlternatively, populate the GH_TOKEN environment variable", "gh auth login"),
            ("HTTP 401: Bad credentials (https://api.github.com/graphql)", "gh auth login"),
            ("release not found", "žádné vydání"),
            ("GraphQL: Could not resolve to a Repository with the name 'x/y'. (repository)", "update.repo"),
            ("error connecting to api.github.com\ncheck your internet connection or https://githubstatus.com", "offline"),
            ("Get \"https://api.github.com/\": dial tcp: lookup api.github.com: no such host", "offline"),
            ("něco úplně jiného\ndruhý řádek", "gh: něco úplně jiného"),
            ("", "neznámá chyba"),
        ];
        for (stderr, want) in cases {
            let got = gh_error(stderr);
            assert!(got.contains(want), "{stderr:?} → {got}");
        }
    }

    #[test]
    fn gh_errors_in_english() {
        crate::i18n::with_lang(crate::uiconf::Lang::En, || {
            assert_eq!(gh_error("HTTP 401: Bad credentials"), "gh isn't logged in — run `gh auth login` in a terminal");
            assert!(gh_error("release not found").contains("no release yet"));
            assert_eq!(gh_error(""), "gh: unknown error");
            assert!(crate::updatenet::api_error(404, None).contains("no release"));
        });
    }

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
    fn installer_command_quotes_and_rejects() {
        let setup = Path::new(r"C:\Users\x y\AppData\Local\Temp\ledge-update\v0.2.14\Ledge_0.2.14_x64-setup.exe");
        let exe = Path::new(r"C:\Users\x y\AppData\Local\Ledge\ledge.exe");
        assert_eq!(
            installer_command(setup, exe).unwrap(),
            r#"/d /s /c "ping -n 3 127.0.0.1 >nul & "C:\Users\x y\AppData\Local\Temp\ledge-update\v0.2.14\Ledge_0.2.14_x64-setup.exe" /S & start "" "C:\Users\x y\AppData\Local\Ledge\ledge.exe"""#
        );
        assert!(installer_command(Path::new(r#"C:\a"b\x-setup.exe"#), exe).is_err());
        assert!(installer_command(setup, Path::new(r"C:\%PATH%\ledge.exe")).is_err());
        assert!(installer_command(setup, Path::new("")).is_err());
    }

    #[test]
    fn status_serializes_for_ui() {
        let v = |status: Status| {
            serde_json::to_value(UpdateView { status, current: "0.2.0", repo: "a/b".into(), checking: false, checked_ms: None, installing: false, install_error: None }).unwrap()
        };
        assert_eq!(v(Status::Off)["state"], "off");
        assert_eq!(v(Status::Neovereno)["state"], "neovereno");
        assert_eq!(v(Status::Aktualni)["state"], "aktualni");
        let d = v(Status::Dostupna { verze: "0.2.14".into(), poznamky: "- x".into(), publikovano: "2026".into() });
        assert_eq!((d["state"].as_str(), d["verze"].as_str(), d["current"].as_str()), (Some("dostupna"), Some("0.2.14"), Some("0.2.0")));
        let n = v(Status::NelzeOverit { duvod: "offline".into() });
        assert_eq!((n["state"].as_str(), n["duvod"].as_str()), (Some("nelze_overit"), Some("offline")));
        assert!(n.get("checkedMs").is_some() && n.get("installError").is_some());
    }

    #[test]
    fn config_defaults_old_config_and_validation() {
        // starý config bez bloku update → zapnuto, repo jiri-hyan/ledge-app, 6 h
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert!(cfg.update.check);
        assert_eq!(cfg.update.repo, "jiri-hyan/ledge-app");
        assert_eq!(cfg.update.interval_h, 6);
        let cfg: Config = serde_json::from_str(r#"{"update":{"intervalH":12}}"#).unwrap();
        assert_eq!((cfg.update.check, cfg.update.interval_h), (true, 12));
        assert!(crate::config::schema_json().contains("intervalH"));
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
}
