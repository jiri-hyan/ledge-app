//! „Naklonovat“ projekt, který na tomhle PC chybí: kdy tlačítko ukázat ([`clonable`]), kam klonovat
//! ([`target_in`] — `roots[0]\<repo>`, kolize jmen), čím ([`command`] — `gh repo clone` pro GitHub,
//! jinak `git clone`) a spuštění bez okna s termínem ([`run`] přes `gitinfo::capture`).
//! Rozhodování jsou čisté funkce (testy); testy nikdy nic neklonují.

use crate::config::{Project, ProjectKind};
use crate::resolve::owner_repo_raw;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Velká repa se klonují i minuty; po termínu se strom procesu zabije.
pub const TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Rozdělané klony (id projektu) — dvojklik nespustí druhý.
static BUSY: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Zámek klonu jednoho projektu; uvolní se s dropem.
pub struct Busy(String);

impl Busy {
    pub fn claim(id: &str) -> Option<Busy> {
        BUSY.lock().get_or_insert_with(HashSet::new).insert(id.to_string()).then(|| Busy(id.to_string()))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        if let Some(s) = BUSY.lock().as_mut() {
            s.remove(&self.0);
        }
    }
}

pub fn is_busy(id: &str) -> bool {
    BUSY.lock().as_ref().is_some_and(|s| s.contains(id))
}

/// Ukázat „Naklonovat“? Složka chybí, remote je, index o žádném klonu neví (dva klony = nejednoznačné,
/// třetí nepomůže) a nejde o nainstalovanou appku (`appName` bez `exe` repo ke spuštění nepotřebuje).
pub fn clonable(p: &Project, path_ok: bool, known_clones: usize) -> bool {
    let installed_only = p.kind == ProjectKind::App && p.exe.is_none() && (p.app_name.is_some() || p.aumid.is_some());
    !path_ok && !installed_only && known_clones == 0 && p.remote.as_deref().and_then(owner_repo_raw).is_some()
}

/// Bezpečné jméno složky (Windows): jen `A-Za-z0-9._-`, bez teček na okrajích a rezervovaných jmen.
pub fn folder_name(s: &str) -> Option<String> {
    let clean: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
    let clean = clean.trim_matches(['.', '-']).to_string();
    let reserved = ["con", "prn", "aux", "nul", "com1", "lpt1"].contains(&clean.to_lowercase().as_str());
    match clean.as_str() {
        "" => None,
        _ if reserved => Some(format!("{clean}-repo")),
        _ => Some(clean),
    }
}

/// Cílová složka v `root`: `<repo>`, při kolizi `<repo>-<owner>`, pak `<repo>-2` … `<repo>-9`.
pub fn target_in(root: &Path, remote: &str, taken: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let (owner, repo) = owner_repo_raw(remote)?;
    let repo = folder_name(&repo)?;
    let mut names = vec![repo.clone()];
    names.extend(folder_name(&owner).map(|o| format!("{repo}-{o}")));
    names.extend((2..=9).map(|i| format!("{repo}-{i}")));
    names.into_iter().map(|n| root.join(n)).find(|p| !taken(p))
}

/// Program a argumenty klonu. GitHub (`owner/repo` nebo URL github.com) → `gh repo clone` (přihlášení
/// z `gh auth`); jiný hostitel → `git clone <url>`.
pub fn command(remote: &str, target: &Path) -> Option<(&'static str, Vec<String>)> {
    let r = remote.trim();
    let t = target.to_string_lossy().into_owned();
    let lower = r.to_lowercase();
    let is_url = lower.contains("://") || lower.contains('@');
    if is_url && !lower.contains("github.com") {
        return Some(("git", vec!["clone".into(), r.into(), t]));
    }
    let (owner, repo) = owner_repo_raw(r)?;
    Some(("gh", vec!["repo".into(), "clone".into(), format!("{owner}/{repo}"), t]))
}

/// Klon do `target` bez okna a promptu; při chybě smaže jen složku, kterou založil sám tenhle pokus.
pub fn run(remote: &str, target: &Path) -> Result<(), String> {
    let (prog, args) = command(remote, target).ok_or_else(|| crate::i18n::tr("Remote nejde přečíst (čekám vlastník/repo nebo URL).", "Can't read the remote (expecting owner/repo or a URL)."))?;
    if target.exists() {
        return Err(trf!("{} už existuje.", "{} already exists.", target.display()));
    }
    let mut cmd = Command::new(prog);
    cmd.args(&args).envs([("GIT_TERMINAL_PROMPT", "0"), ("GCM_INTERACTIVE", "never"), ("GH_PROMPT_DISABLED", "1")]);
    let res = crate::gitinfo::capture(&mut cmd, None, true, Instant::now() + TIMEOUT);
    let err = match res {
        Ok(out) if out.success && target.join(".git").exists() => return Ok(()),
        Ok(out) => crate::errors::git(&trf!("{prog} selhal", "{prog} failed"), out.code, &out.stderr),
        Err(crate::gitinfo::RunError::Timeout) => trf!("Klonování nestihlo {} min.", "Cloning didn't finish within {} min.", TIMEOUT.as_secs() / 60),
        Err(crate::gitinfo::RunError::Spawn(e)) if prog == "gh" => crate::errors::spawn_text(crate::i18n::tr("GitHub CLI (gh) nejde spustit", "Can't run GitHub CLI (gh)"), &e),
        Err(crate::gitinfo::RunError::Spawn(e)) => crate::errors::spawn_text(crate::i18n::tr("Git nejde spustit", "Can't run git"), &e),
    };
    // rozpracovaný klon tohohle pokusu (složka předtím neexistovala) — jinak by další pokus kolidoval
    let _ = std::fs::remove_dir_all(target);
    Err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(kind: ProjectKind, remote: Option<&str>, exe: Option<&str>, app: Option<&str>) -> Project {
        Project {
            id: "acme-app".into(),
            name: "Acme App".into(),
            enabled: true,
            kind,
            path: None,
            remote: remote.map(Into::into),
            subdir: None,
            run: Some("npm run dev".into()),
            port: None,
            pass_port: true,
            open_browser: true,
            exe: exe.map(Into::into),
            app_name: app.map(Into::into),
            aumid: None,
            args: vec![],
            accent: None,
            monogram: None,
            logo: None,
            links: vec![],
            look: None,
        }
    }

    #[test]
    fn clonable_table() {
        let dev = project(ProjectKind::Dev, Some("acme/web"), None, None);
        assert!(clonable(&dev, false, 0));
        assert!(!clonable(&dev, true, 0), "už na disku");
        assert!(!clonable(&dev, false, 2), "dva klony = nejednoznačné, třetí nepomůže");
        assert!(!clonable(&project(ProjectKind::Dev, None, None, None), false, 0), "bez remote");
        assert!(!clonable(&project(ProjectKind::Dev, Some("nesmysl"), None, None), false, 0));
        assert!(!clonable(&project(ProjectKind::App, Some("o/r"), None, Some("Acme Studio")), false, 0), "nainstalovaná appka");
        assert!(clonable(&project(ProjectKind::App, Some("o/r"), Some("node_modules/electron/dist/electron.exe"), None), false, 0));
    }

    #[test]
    fn target_and_collisions() {
        let root = Path::new(r"C:\Users\x\Desktop");
        let none = |_: &Path| false;
        assert_eq!(target_in(root, "https://github.com/acme/web.git", none).unwrap(), root.join("web"));
        assert_eq!(target_in(root, "Acme-Corp/Acme-Studio", none).unwrap(), root.join("Acme-Studio"), "velikost písmen zůstává");
        let first = |p: &Path| p.ends_with("app");
        assert_eq!(target_in(root, "acme/app", first).unwrap(), root.join("app-acme"));
        let two = |p: &Path| p.ends_with("app") || p.ends_with("app-acme");
        assert_eq!(target_in(root, "acme/app", two).unwrap(), root.join("app-2"));
        assert!(target_in(root, "o/r", |_: &Path| true).is_none(), "všechno obsazené");
        assert!(target_in(root, "bez-lomitka", none).is_none());
    }

    #[test]
    fn folder_names_are_safe() {
        for (i, o) in [("Acme.Web", Some("Acme.Web")), ("a b:c", Some("a-b-c")), ("..x..", Some("x")), ("CON", Some("CON-repo")), ("...", None)] {
            assert_eq!(folder_name(i).as_deref(), o, "{i}");
        }
    }

    #[test]
    fn command_choice() {
        let t = Path::new(r"C:\D\r");
        let (p, a) = command("acme/web", t).unwrap();
        assert_eq!((p, a), ("gh", vec!["repo".to_string(), "clone".into(), "acme/web".into(), r"C:\D\r".into()]));
        assert_eq!(command("git@github.com:o/r.git", t).unwrap().1[2], "o/r");
        let (p, a) = command("https://gitlab.com/o/r.git", t).unwrap();
        assert_eq!((p, a[1].as_str()), ("git", "https://gitlab.com/o/r.git"));
        assert!(command("nic", t).is_none());
    }

    #[test]
    fn busy_is_exclusive_and_released() {
        let a = Busy::claim("test-busy").unwrap();
        assert!(Busy::claim("test-busy").is_none() && is_busy("test-busy"));
        drop(a);
        assert!(!is_busy("test-busy") && Busy::claim("test-busy").is_some());
    }
}
