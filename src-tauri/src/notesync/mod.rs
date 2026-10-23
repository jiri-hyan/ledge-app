//! Synchronizace poznámek mezi počítači: kořen poznámek (`notes::resolve_root`) je pracovní kopie
//! soukromého git repa (`notes.sync.remote`). Ledge sám commitne lokální změny, stáhne cizí (rebase)
//! a pushne — nikdy se neptá oknem ani promptem (`GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`,
//! každé volání s termínem přes [`gitinfo::run_env`]).
//!
//! Stavový automat jednoho běhu ([`Syncer::sync`]):
//!
//! | kořen                    | remote      | plán ([`plan_init`])                                     |
//! | ------------------------ | ----------- | -------------------------------------------------------- |
//! | je repo (`.git`)         | (neptá se)  | `EnsureOrigin` — ověř/nastav `origin`, pak cyklus        |
//! | není repo, jen lokální   | prázdný     | `Fresh` — `init -b main`, `.gitignore`, commit, push      |
//! | není repo                | má commity  | `CloneMerge` — clone vedle, sloučit lokální, prohodit    |
//! | uvnitř cizího repa       | —           | `Refuse` — nic nedělat (do cizího repa necommitujeme)     |
//!
//! Cyklus: `add -A` → commit jen se změnami → `fetch` → `rebase --autostash` (jen když remote
//! utekl) → `push` (jen když jsme napřed). Konflikt = `rebase --abort` (lokální commit zůstane,
//! nic se neztratí) a stav `Konflikt`. Síť / přihlášení se rozliší ze stderr ([`classify`]).
//!
//! Čisté rozhodování (plán, klasifikace chyb, plán slučování, plánovač, stav pro UI) je oddělené od
//! I/O a testované tabulkami; integrační testy jedou proti bare repu v `temp_dir` (žádná síť).
//!
//! Soubory: tady výsledek a stav pro UI (`Outcome`, `Status`), plánovač (`Sched`) a vstup běhu
//! (`Syncer::sync`); `plan.rs` čisté rozhodování, `git.rs` git příkazy, `init.rs` první sync a
//! slučování, `cycle.rs` cyklus commit/fetch/rebase/push. Vlákno syncu žije v `lib.rs`.
//! Mapa: `docs/codemap/notes.md`.

mod cycle;
mod git;
mod init;
mod plan;
#[cfg(test)]
mod tests;

pub use plan::*;

use crate::i18n::tr;
use chrono::{DateTime, FixedOffset};
use cycle::cycle;
use git::{git, net_fail, setup_repo};
use init::root_state;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Řádky `.gitignore` v kořeni poznámek: lokální koncept, stav peeku, tmp zápisů, značka migrace,
/// odklad smazaných poznámek (`.smazane/`, než jdou do Koše).
pub const IGNORE: [&str; 5] = [".draft/", ".state.json", "*.tmp", "PRESUNUTO-DO-LEDGE.txt", ".smazane/"];
/// Žádné převody konců řádků: poznámka je na obou PC bajtově stejná (jinak falešné změny a konflikty).
const ATTRIBUTES: &str = "* -text\n";
const LOCAL_DEADLINE: Duration = Duration::from_secs(30);
const NET_DEADLINE: Duration = Duration::from_secs(90);
/// Debounce lokální změny (jeden zápis = víc událostí, víc poznámek za sebou = jeden commit) …
pub const LOCAL_DEBOUNCE: Duration = Duration::from_secs(10);
/// … ale při nepřetržitých změnách nejpozději po minutě.
pub const LOCAL_MAX_WAIT: Duration = Duration::from_secs(60);
/// Otevření šuplíku spustí sync nejvýš jednou za minutu.
pub const OPEN_THROTTLE: Duration = Duration::from_secs(60);
pub fn auth_hint() -> &'static str {
    crate::i18n::tr("spusť `gh auth setup-git` (jednou na každém PC)", "run `gh auth setup-git` (once on each PC)")
}

// ---------- výsledek a stav pro UI ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ok { pulled: bool, pushed: bool, swapped: Option<PathBuf> },
    Offline(String),
    Neprihlaseno(String),
    Konflikt { files: Vec<String>, message: String },
    Chyba(String),
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// sync vypnutý (nebo vypnuté poznámky)
    #[default]
    Off,
    Ok,
    /// právě běží (i před prvním dokončeným během)
    Syncing,
    Offline,
    Neprihlaseno,
    Konflikt,
    /// jiná chyba (kořen v cizím repu, rozbitý remote …) — text v `message`
    Chyba,
}

/// `Snapshot.notesSync`.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: State,
    /// poslední úspěšný sync (ms od epochy) — jen z tohoto běhu Ledge, nic se nedopočítává
    pub last_ok_ms: Option<i64>,
    pub message: Option<String>,
    /// soubory v konfliktu
    pub files: Vec<String>,
    /// kořen poznámek (title štítku)
    pub root: Option<String>,
}

impl Status {
    /// Výsledek běhu → stav. `last_ok_ms` drží přes chyby (UI ukáže „poslední sync před X“).
    pub fn apply(&mut self, o: &Outcome, now_ms: i64) {
        self.files.clear();
        self.message = None;
        match o {
            Outcome::Ok { .. } => {
                self.state = State::Ok;
                self.last_ok_ms = Some(now_ms);
            }
            Outcome::Offline(m) => (self.state, self.message) = (State::Offline, Some(m.clone())),
            Outcome::Neprihlaseno(m) => (self.state, self.message) = (State::Neprihlaseno, Some(m.clone())),
            Outcome::Konflikt { files, message } => {
                self.state = State::Konflikt;
                self.files = files.clone();
                self.message = Some(message.clone());
            }
            Outcome::Chyba(m) => (self.state, self.message) = (State::Chyba, Some(m.clone())),
        }
    }
}

/// Krátká odpověď pro „Synchronizovat teď“.
pub fn outcome_text(o: &Outcome) -> Result<String, String> {
    match o {
        Outcome::Ok { pulled, pushed, swapped } => Ok(match (swapped.is_some(), pulled, pushed) {
            (true, _, _) => tr("Poznámky napojené na repo a sloučené.", "Notes connected to the repo and merged.").into(),
            (_, true, true) => tr("Synchronizováno: staženo i odesláno.", "Synced: pulled and pushed.").into(),
            (_, true, false) => tr("Synchronizováno: staženy změny.", "Synced: pulled changes.").into(),
            (_, false, true) => tr("Synchronizováno: odesláno.", "Synced: pushed.").into(),
            _ => tr("Synchronizováno, beze změn.", "Synced, no changes.").into(),
        }),
        Outcome::Offline(m) => Err(format!("Offline: {m}")),
        Outcome::Neprihlaseno(m) => Err(trf!("Nepřihlášeno: {m}", "Not logged in: {m}")),
        Outcome::Konflikt { files, .. } => Err(trf!("Konflikt: {}", "Conflict: {}", files.join(", "))),
        Outcome::Chyba(m) => Err(m.clone()),
    }
}

// ---------- plánovač (kdy synchronizovat) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    /// lokální změna poznámek (ne od syncu samotného)
    Local,
    /// otevřený šuplík
    Opened,
    /// změna `notes` v configu
    Config,
    /// tlačítko „Synchronizovat teď“
    Now,
    /// vypršel `recv_timeout`
    Timer,
}

/// Kdy příště synchronizovat. Bez posledního běhu (start Ledge) = hned.
#[derive(Debug, Clone, Default)]
pub struct Sched {
    last_run: Option<Instant>,
    local_due: Option<Instant>,
    local_first: Option<Instant>,
}

impl Sched {
    /// Nejbližší termín: periodický (`last_run + interval`) nebo debounce lokální změny.
    pub fn deadline(&self, now: Instant, interval: Duration) -> Instant {
        let periodic = self.last_run.map_or(now, |t| t + interval);
        self.local_due.map_or(periodic, |l| l.min(periodic))
    }

    /// Zpráva / timeout → synchronizovat hned?
    pub fn on(&mut self, w: Wake, now: Instant, interval: Duration) -> bool {
        match w {
            Wake::Config | Wake::Now => true,
            Wake::Opened => self.last_run.is_none_or(|t| now.duration_since(t) >= OPEN_THROTTLE),
            Wake::Local => {
                let first = *self.local_first.get_or_insert(now);
                self.local_due = Some((now + LOCAL_DEBOUNCE).min(first + LOCAL_MAX_WAIT));
                false
            }
            Wake::Timer => self.deadline(now, interval) <= now,
        }
    }

    pub fn ran(&mut self, at: Instant) {
        self.last_run = Some(at);
        self.local_due = None;
        self.local_first = None;
    }
}

/// Kontext jednoho běhu.
pub struct Ctx<'a> {
    pub root: &'a Path,
    pub remote: &'a str,
    /// přípona jmen a identita commitů (`sanitize_pc(COMPUTERNAME)`)
    pub pc: &'a str,
    pub now: DateTime<FixedOffset>,
}

/// Drží, pro který kořen + remote je repo už nastavené (ať se `git config` nespouští každý cyklus).
#[derive(Default)]
pub struct Syncer {
    prepared: Option<(PathBuf, String)>,
    /// log řádky z posledního běhu (zálohy, přejmenování) — volající je zapíše do logu
    pub notes: Vec<String>,
}

impl Syncer {
    /// Inicializace (podle [`plan_init`]) + jeden cyklus synchronizace.
    pub fn sync(&mut self, ctx: &Ctx) -> Outcome {
        self.notes.clear();
        if let Err(e) = std::fs::create_dir_all(ctx.root) {
            return Outcome::Chyba(crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", ctx.root.display()), &e));
        }
        let key = (ctx.root.to_path_buf(), ctx.remote.to_string());
        let mut swapped = None;
        if self.prepared.as_ref() != Some(&key) || !ctx.root.join(".git").exists() {
            let state = root_state(ctx.root, ctx.remote);
            let remote_state = if matches!(state, RootState::Empty | RootState::LocalFiles) {
                match git(ctx.root.parent().unwrap_or(ctx.root), ctx.remote, &["ls-remote", ctx.remote], true) {
                    Ok(out) => Some(if out.trim().is_empty() { RemoteState::Empty } else { RemoteState::HasCommits }),
                    Err(f) => return net_fail(f),
                }
            } else {
                None
            };
            let Some(plan) = plan_init(&state, remote_state) else {
                return Outcome::Chyba(tr("Stav remote neznámý.", "Remote state unknown.").into());
            };
            let res = match plan {
                InitPlan::Refuse(m) => return Outcome::Chyba(m),
                InitPlan::EnsureOrigin => setup_repo(ctx.root, ctx.remote, ctx.pc),
                InitPlan::Fresh => self.fresh(ctx),
                InitPlan::CloneMerge => self.clone_merge(ctx).map(|b| swapped = Some(b)),
            };
            if let Err(o) = res {
                return o;
            }
            self.prepared = Some(key);
        }
        match cycle(ctx) {
            Outcome::Ok { pulled, pushed, .. } => Outcome::Ok { pulled, pushed, swapped },
            // prohozené složky platí i při chybě cyklu (push dožene další běh)
            other => other,
        }
    }
}
