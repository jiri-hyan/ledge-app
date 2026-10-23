//! Panel Git v kartě Claude (jako claude-squad / ccmanager): větev, ahead/behind, změněné soubory,
//! diff souboru, commit, push, pull (jen fast-forward), zahození změn v souboru a odstranění worktree.
//!
//! - Všechno jen na výslovný klik (panel se čte při rozbalení / po akci, **žádné pollování**).
//! - git vždy bez okna a s termínem (`gitinfo::capture`), argumenty jako pole, pathspecy doslova
//!   (`GIT_LITERAL_PATHSPECS`), `core.fsmonitor=false` (cwd přichází z hooku). Nikdy `--force`,
//!   `--no-verify`, `reset --hard`.
//! - UI posílá jen id session a cestu souboru; Rust si stav přečte znovu a cestu ověří proti němu,
//!   cestu worktree bere jen z `git worktree list`.
//!
//! Čistá logika: `parse.rs` (výstupy gitu), `plan.rs` (argumenty, pravidla, texty). Tauri příkazy:
//! `commands/sessiongit.rs`. Mapa: `docs/codemap/sessions.md` → Panel Git.

pub mod parse;
pub mod plan;

use crate::claude::diff::MAX_LINES;
use crate::claude::paths::path_key;
use crate::claude::preview::{self, Disk, Preview, READ_MAX, Shown};
use crate::gitinfo::{self, Captured, RunError};
use crate::i18n::tr;
use crate::sessions::Status;
use parse::{GitFile, Porcelain, Worktree};
use plan::PushPlan;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Termín čtení (status, worktree list, numstat, diff).
const READ_WAIT: Duration = Duration::from_secs(10);
/// Commit: hooky gitu (lint, testy) smějí běžet déle.
const COMMIT_WAIT: Duration = Duration::from_secs(300);
/// Push / pull přes síť.
const NET_WAIT: Duration = Duration::from_secs(120);
/// Lokální zápis (add, restore, worktree remove).
const WRITE_WAIT: Duration = Duration::from_secs(30);
/// Větší výstup `git diff` se neukáže (staré verze souboru můžou být obří).
const DIFF_MAX: usize = 1024 * 1024;

/// Termíny (testy na vytíženém stroji je natáhnou, jako `Claude::patient`).
#[derive(Debug, Clone, Copy)]
pub struct Waits {
    pub read: Duration,
    pub commit: Duration,
    pub net: Duration,
    pub write: Duration,
}

impl Default for Waits {
    fn default() -> Self {
        Waits { read: READ_WAIT, commit: COMMIT_WAIT, net: NET_WAIT, write: WRITE_WAIT }
    }
}

fn git(dir: &Path, args: &[&str], env: &[(&str, &str)], input: Option<&str>, wait: Duration) -> Result<Captured, RunError> {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.fsmonitor=false"]).args(args).current_dir(dir).envs(env.iter().copied());
    gitinfo::capture(&mut cmd, input, true, Instant::now() + wait)
}

fn read(dir: &Path, args: &[&str], wait: Duration) -> Option<String> {
    git(dir, args, &plan::ENV_READ, None, wait).ok().filter(|o| o.success).map(|o| o.stdout)
}

/// Zápisový příkaz: úspěch = výstup; chyba = hláška s posledními řádky gitu (i chyba hooku).
fn write(dir: &Path, args: &[&str], input: Option<&str>, wait: Duration, what: &str) -> Result<Captured, String> {
    match git(dir, args, &plan::ENV, input, wait) {
        Ok(o) if o.success => Ok(o),
        Ok(o) => {
            let both = format!("{}\n{}", o.stdout, o.stderr);
            let why = plan::tail(&both, 12);
            Err(if why.is_empty() { trf!("{what} selhal.", "{what} failed.") } else { trf!("{what} selhal:\n{why}", "{what} failed:\n{why}") })
        }
        Err(RunError::Timeout) => Err(trf!("{what} nedoběhl do {} s — přerušeno.", "{what} didn't finish within {} s — stopped.", wait.as_secs())),
        Err(RunError::Spawn(e)) => Err(crate::errors::spawn_text(tr("Git nejde spustit", "Can't run git"), &e)),
    }
}

/// Stav repa, ve kterém leží cwd session.
#[derive(Debug, Clone)]
pub struct Repo {
    /// kořen pracovní složky (`rev-parse --show-toplevel`, s `/`)
    pub root: String,
    pub st: Porcelain,
    /// prázdné, když `worktree list` selhal
    pub worktrees: Vec<Worktree>,
    pub remotes: Vec<String>,
    /// commity HEAD mimo všechny remote (jen u větve bez upstreamu)
    pub unpushed: Option<u32>,
}

impl Repo {
    /// Index záznamu tohoto checkoutu ve `worktrees` (0 = hlavní).
    fn wt_index(&self) -> Option<usize> {
        let k = path_key(&self.root);
        self.worktrees.iter().position(|w| path_key(&w.path) == k)
    }
    /// Tenhle checkout je přidaný worktree (ne hlavní): (on, hlavní).
    pub fn linked(&self) -> Option<(&Worktree, &Worktree)> {
        match self.wt_index()? {
            0 => None,
            i => Some((&self.worktrees[i], self.worktrees.first()?)),
        }
    }
    pub fn file(&self, path: &str) -> Option<&GitFile> {
        self.st.files.iter().find(|f| f.path == path)
    }
    pub fn push_plan(&self) -> PushPlan {
        plan::push_plan(self.st.branch.as_deref(), self.st.upstream.as_deref(), self.st.ahead, &self.remotes, self.unpushed)
    }
}

/// Přečte repo session. `Ok(None)` = cwd neexistuje nebo není v gitu.
pub fn read_repo(cwd: &Path, w: Waits) -> Result<Option<Repo>, String> {
    if !cwd.is_dir() {
        return Ok(None);
    }
    let Some(top) = read(cwd, &plan::TOPLEVEL, w.read) else { return Ok(None) };
    let root = top.trim().to_string();
    if root.is_empty() {
        return Ok(None);
    }
    let dir = PathBuf::from(&root);
    let out = git(&dir, &plan::STATUS, &plan::ENV_READ, None, w.read);
    let mut st = match out {
        Ok(o) if o.success => parse::status_z(&o.stdout),
        Ok(o) => return Err(crate::errors::git(tr("Git status selhal", "git status failed"), o.code, &o.stderr)),
        Err(RunError::Timeout) => return Err(tr("git status nedoběhl včas.", "git status didn't finish in time.").into()),
        Err(RunError::Spawn(e)) => return Err(crate::errors::spawn_text(tr("Git nejde spustit", "Can't run git"), &e)),
    };
    if !st.initial
        && let Some(n) = read(&dir, &plan::NUMSTAT, w.read)
    {
        parse::apply_numstat(&mut st.files, &parse::numstat_z(&n));
    }
    let worktrees = read(&dir, &plan::WORKTREES, w.read).map(|o| parse::worktrees(&o)).unwrap_or_default();
    let (mut remotes, mut unpushed) = (Vec::new(), None);
    if st.upstream.is_none() && st.branch.is_some() && !st.initial {
        remotes = read(&dir, &plan::REMOTES, w.read).map(|o| o.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()).unwrap_or_default();
        unpushed = read(&dir, &plan::UNPUSHED, w.read).and_then(|o| o.trim().parse().ok());
    }
    Ok(Some(Repo { root, st, worktrees, remotes, unpushed }))
}

/// Soubor v panelu + text potvrzení zahození (`None` = zahodit nejde).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    #[serde(flatten)]
    pub file: GitFile,
    pub discard: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeView {
    pub path: String,
    pub branch: Option<String>,
    /// hlavní checkout
    pub main: String,
    pub main_branch: Option<String>,
    pub locked: bool,
    /// text druhého potvrzení odstranění; `None` = odstranit nejde (zamčený)
    pub remove: Option<String>,
}

/// Co panel ukáže (`session_git`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub root: String,
    pub branch: Option<String>,
    pub initial: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// nejvýš `plan::MAX_FILES`
    pub files: Vec<FileView>,
    /// všech změněných souborů
    pub total: u32,
    pub worktree: Option<WorktreeView>,
    pub push: PushPlan,
    pub push_text: Option<String>,
    pub can_pull: bool,
    /// proč jsou destruktivní akce zablokované (živá session v té složce)
    pub block: Option<String>,
}

/// Čisté sestavení pohledu. `here` = stavy session, jejichž cwd leží v kořeni repa.
pub fn view(repo: &Repo, here: &[Status]) -> View {
    let files = repo
        .st
        .files
        .iter()
        .take(plan::MAX_FILES)
        .map(|f| FileView { file: f.clone(), discard: plan::discardable(f).then(|| plan::discard_text(f)) })
        .collect();
    let idle_here = here.contains(&Status::Idle);
    let worktree = repo.linked().map(|(w, main)| WorktreeView {
        path: w.path.clone(),
        branch: w.branch.clone(),
        main: main.path.clone(),
        main_branch: main.branch.clone(),
        locked: w.locked,
        remove: (!w.locked).then(|| plan::remove_text(&w.path, w.branch.as_deref(), &main.path, repo.unpushed, idle_here)),
    });
    let push = repo.push_plan();
    View {
        root: repo.root.clone(),
        branch: repo.st.branch.clone(),
        initial: repo.st.initial,
        upstream: repo.st.upstream.clone(),
        ahead: repo.st.ahead,
        behind: repo.st.behind,
        files,
        total: repo.st.files.len() as u32,
        worktree,
        push_text: plan::push_text(&push),
        push,
        can_pull: repo.st.upstream.is_some() && repo.st.branch.is_some(),
        block: plan::destructive_block(here),
    }
}

fn not_listed(path: &str) -> String {
    trf!("{path} už mezi změnami není — obnov panel.", "{path} is no longer among the changes — refresh the panel.")
}

/// Diff jednoho souboru: `staged` = index proti HEAD (`--cached`), jinak pracovní složka proti indexu.
/// Nesledovaný soubor = začátek nového obsahu (čte se s limitem `READ_MAX`).
pub fn file_diff(repo: &Repo, path: &str, staged: bool, w: Waits) -> Result<Preview, String> {
    let f = repo.file(path).ok_or_else(|| not_listed(path))?;
    let root = Path::new(&repo.root);
    let abs = root.join(path);
    if f.kind == parse::FileKind::Untracked {
        let mut p = parse::empty_preview(path, Shown::NewFile);
        match preview::read_capped(&abs, READ_MAX) {
            Disk::Text(t) => preview::head(&mut p, Shown::NewFile, &t),
            Disk::TooBig => p.shown = Shown::TooBig,
            Disk::Binary => p.shown = Shown::Binary,
            Disk::Missing | Disk::Unreadable => p.shown = Shown::Unread,
        }
        return Ok(p);
    }
    if !staged && std::fs::metadata(&abs).is_ok_and(|m| m.len() > READ_MAX) {
        return Ok(parse::empty_preview(path, Shown::TooBig));
    }
    let args = plan::diff_args(path, staged);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = write(root, &args, None, w.read, "git diff")?;
    if out.stdout.len() > DIFF_MAX {
        return Ok(parse::empty_preview(path, Shown::TooBig));
    }
    Ok(parse::unified(path, &out.stdout, MAX_LINES))
}

/// Commit vybraných souborů: `git add` + `git commit -m` jen těch cest. Hooky běží; jejich chyba se
/// vrátí celá (posledních 12 řádků). Vrací první řádek výstupu commitu (`[větev abc123] zpráva`).
pub fn commit(repo: &Repo, message: &str, selected: &[String], w: Waits) -> Result<String, String> {
    let msg = plan::clean_message(message)?;
    let (add, paths) = plan::selection(selected, &repo.st.files)?;
    let root = Path::new(&repo.root);
    write(root, &plan::ADD, Some(&plan::pathspec_input(&add)), w.write, "git add")?;
    let args = plan::commit_args(&msg);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = write(root, &args, Some(&plan::pathspec_input(&paths)), w.commit, "git commit")
        .map_err(|e| e + tr("\n(Vybrané soubory zůstaly připravené ve staged.)", "\n(The picked files stay staged.)"))?;
    Ok(out.stdout.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string())
}

/// `git push` podle plánu (nikdy force). Vrací poslední řádek výstupu gitu.
pub fn push(repo: &Repo, w: Waits) -> Result<String, String> {
    let plan = repo.push_plan();
    let args = match plan::push_args(&plan) {
        Some(a) => a,
        None => return Err(match plan {
            PushPlan::No { why } => why,
            _ => String::new(),
        }),
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = write(Path::new(&repo.root), &args, None, w.net, "git push")?;
    Ok(plan::tail(&format!("{}\n{}", out.stdout, out.stderr), 1))
}

/// `git pull --ff-only` — jen rychlé posunutí, jako v kartě Repa.
pub fn pull(repo: &Repo, w: Waits) -> Result<String, String> {
    if repo.st.upstream.is_none() {
        return Err(tr("Větev nemá upstream — není odkud stáhnout.", "The branch has no upstream — nothing to pull from.").into());
    }
    let out = write(Path::new(&repo.root), &plan::PULL, None, w.net, "git pull")?;
    Ok(plan::tail(&out.stdout, 1))
}

/// Zahodí změny v jednom sledovaném souboru (vrátí ho do HEAD). `here` = stavy session v repu.
pub fn discard(repo: &Repo, path: &str, here: &[Status], w: Waits) -> Result<(), String> {
    if let Some(why) = plan::destructive_block(here) {
        return Err(why);
    }
    let f = repo.file(path).ok_or_else(|| not_listed(path))?;
    if !plan::discardable(f) {
        return Err(trf!("{path} jde zahodit jen u sledovaného souboru z posledního commitu.", "{path} can only be discarded for a tracked file from the last commit."));
    }
    let args = plan::discard_args(path);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    write(Path::new(&repo.root), &args, None, w.write, "git restore").map(|_| ())
}

/// Odstraní tenhle worktree (`git worktree remove` z hlavního checkoutu, bez `--force`).
pub fn remove_worktree(repo: &Repo, here: &[Status], w: Waits) -> Result<String, String> {
    if let Some(why) = plan::destructive_block(here) {
        return Err(why);
    }
    let (wt, main) = repo.linked().ok_or_else(|| tr("Tohle není přidaný worktree — hlavní checkout se neodstraňuje.", "This isn't an added worktree — the main checkout isn't removed."))?;
    if wt.locked {
        return Err(tr("Worktree je zamčený (git worktree lock) — odemkni ho ručně.", "The worktree is locked (git worktree lock) — unlock it manually.").into());
    }
    let args = plan::remove_worktree_args(&wt.path);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    write(Path::new(&main.path), &args, None, w.write, "git worktree remove")?;
    Ok(trf!("Worktree {} odstraněn.", "Worktree {} removed.", wt.path))
}

#[cfg(test)]
mod tests;
