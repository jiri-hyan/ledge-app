//! Dohledání složky projektu podle git remote a nainstalované aplikace podle jména.
//!
//! Port dřívějšího resolveru repozitářů (`repoResolver.ts`) vč. jeho okrajových případů (fetch řádek nesmí
//! prosáknout do URL, worktree s `.git` souborem se přeskočí, dva klony = nejednoznačné → None).

use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static ORIGIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)\[remote "origin"\][^\[]*?\burl\s*=\s*([^\r\n]+)"#).unwrap());
static OWNER_REPO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([^/\s:@]+)/([^/\s]+)$").unwrap());

/// Vlastníci, kteří jsou ve skutečnosti tentýž účet (přejmenování na GitHubu): `(starý, nový)`, malými
/// písmeny. Prázdné — přejmenovaný vlastník se stejně najde podle jednoznačného jména repa
/// (`RepoIndex::resolve`, záloha `by_repo`); alias je potřeba jen při dvou klonech stejného jména.
const OWNER_ALIASES: &[(&str, &str)] = &[];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnerRepo {
    pub owner: String,
    pub repo: String,
}

pub fn parse_remote(remote: &str) -> Option<OwnerRepo> {
    let (owner, repo) = owner_repo_raw(remote)?;
    let (mut owner, repo) = (owner.to_ascii_lowercase(), repo.to_ascii_lowercase());
    if let Some((_, to)) = OWNER_ALIASES.iter().find(|(from, _)| *from == owner) {
        owner = to.to_string();
    }
    Some(OwnerRepo { owner, repo })
}

/// `(owner, repo)` z remote v původní velikosti písmen (odkaz na GitHub, `gh repo clone`).
pub fn owner_repo_raw(remote: &str) -> Option<(String, String)> {
    let s = remote.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").or_else(|| s.strip_suffix(".GIT")).unwrap_or(s);
    let c = OWNER_REPO_RE.captures(s)?;
    Some((c[1].to_string(), c[2].to_string()))
}

pub fn read_origin(repo_dir: &Path) -> Option<String> {
    let cfg = repo_dir.join(".git").join("config");
    if !cfg.is_file() {
        return None; // worktree / submodul (.git je soubor) nebo není repo
    }
    let text = std::fs::read_to_string(cfg).ok()?;
    ORIGIN_RE.captures(&text).map(|c| c[1].trim().to_string())
}

/// Hlavní checkout z obsahu souboru `.git` linked worktree (`gitdir: <main>/.git/worktrees/<jméno>`).
/// Submodul (`gitdir: …/.git/modules/…`) ani nic jiného za worktree nepovažuje.
pub fn main_from_gitfile(text: &str) -> Option<PathBuf> {
    let gitdir = text.lines().find_map(|l| l.trim().strip_prefix("gitdir:"))?.trim();
    let p = Path::new(gitdir);
    let worktrees = p.parent()?;
    let dotgit = worktrees.parent()?;
    let is_name = |q: &Path, n: &str| q.file_name().is_some_and(|f| f.eq_ignore_ascii_case(n));
    (is_name(worktrees, "worktrees") && is_name(dotgit, ".git")).then(|| dotgit.parent().map(Path::to_path_buf)).flatten()
}

/// Složka uvnitř linked worktree → stejná relativní cesta v hlavním checkoutu (session běžící ve
/// worktree `atlas-v2` patří projektu `atlas`). `None`, když `cwd` neleží ve worktree.
pub fn main_checkout(cwd: &Path) -> Option<PathBuf> {
    for dir in cwd.ancestors() {
        let g = dir.join(".git");
        if g.is_dir() {
            return None; // běžný checkout — mapování podle složky už selhalo, není co přemapovat
        }
        if g.is_file() {
            let main = main_from_gitfile(&std::fs::read_to_string(&g).ok()?)?;
            return Some(main.join(cwd.strip_prefix(dir).ok()?));
        }
    }
    None
}

#[derive(Default, Debug)]
pub struct RepoIndex {
    by_full: HashMap<OwnerRepo, Vec<PathBuf>>,
    by_repo: HashMap<String, Vec<PathBuf>>,
}

impl RepoIndex {
    /// Projde PŘÍMÉ podsložky kořenů (rychlé — žádné rekurzivní procházení node_modules).
    pub fn build(roots: &[String]) -> Self {
        let mut idx = RepoIndex::default();
        let mut seen = std::collections::HashSet::new();
        for root in roots {
            let Ok(rd) = std::fs::read_dir(root) else { continue };
            for ent in rd.flatten() {
                let p = ent.path();
                if !p.is_dir() || !seen.insert(p.to_string_lossy().to_lowercase()) {
                    continue;
                }
                let Some(remote) = read_origin(&p) else { continue };
                let Some(or) = parse_remote(&remote) else { continue };
                idx.by_repo.entry(or.repo.clone()).or_default().push(p.clone());
                idx.by_full.entry(or).or_default().push(p);
            }
        }
        idx
    }

    /// Kolik klonů repa index zná (přesná shoda vlastník/repo, jinak podle jména repa).
    pub fn clones(&self, remote: &str) -> usize {
        let Some(or) = parse_remote(remote) else { return 0 };
        self.by_full.get(&or).or_else(|| self.by_repo.get(&or.repo)).map_or(0, Vec::len)
    }

    pub fn resolve(&self, remote: &str) -> Option<PathBuf> {
        let or = parse_remote(remote)?;
        if let Some(v) = self.by_full.get(&or) {
            if v.len() == 1 {
                return Some(v[0].clone());
            }
            return None; // dva klony téhož repa → nehádat
        }
        match self.by_repo.get(&or.repo) {
            Some(v) if v.len() == 1 => Some(v[0].clone()),
            _ => None,
        }
    }
}

/// Najde .exe nainstalované aplikace podle jména v registru odinstalace (HKCU i HKLM).
pub fn installed_app_exe(name: &str) -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    let want = name.to_lowercase();
    let roots = [
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_LOCAL_MACHINE, r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];
    for (hive, path) in roots {
        let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ) else { continue };
        for sub in key.enum_keys().flatten() {
            let Ok(app) = key.open_subkey_with_flags(&sub, KEY_READ) else { continue };
            let display: String = app.get_value("DisplayName").unwrap_or_default();
            if !display.to_lowercase().contains(&want) {
                continue;
            }
            // DisplayIcon bývá přímo exe ("C:\...\app.exe,0"); jinak InstallLocation + jméno
            let icon: String = app.get_value("DisplayIcon").unwrap_or_default();
            let icon = icon.trim().trim_matches('"');
            let icon = icon.split(',').next().unwrap_or("").trim_matches('"');
            if icon.to_lowercase().ends_with(".exe") && Path::new(icon).is_file() {
                return Some(PathBuf::from(icon));
            }
            let loc: String = app.get_value("InstallLocation").unwrap_or_default();
            let loc = PathBuf::from(loc.trim().trim_matches('"'));
            if loc.is_dir()
                && let Ok(rd) = std::fs::read_dir(&loc)
            {
                let exes: Vec<PathBuf> = rd
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")))
                    .filter(|p| !p.file_name().unwrap().to_string_lossy().to_lowercase().contains("unins"))
                    .collect();
                if exes.len() == 1 {
                    return Some(exes[0].clone());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_from_gitfile_table() {
        let m = |t: &str| main_from_gitfile(t).map(|p| p.to_string_lossy().replace('\\', "/"));
        assert_eq!(m("gitdir: C:/Users/x/Desktop/atlas/.git/worktrees/atlas-v2\n").as_deref(), Some("C:/Users/x/Desktop/atlas"));
        assert_eq!(m("gitdir: C:\\a\\repo\\.git\\worktrees\\wt").as_deref(), Some("C:/a/repo"));
        assert_eq!(m("gitdir: C:/a/repo/.git/modules/sub"), None); // submodul
        assert_eq!(m("gitdir: C:/a/repo/.git"), None);
        assert_eq!(m("nesmysl"), None);
    }

    #[test]
    fn main_checkout_maps_subdir_and_ignores_plain_repo() {
        let t = std::env::temp_dir().join(format!("ledge-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        let main = t.join("repo");
        std::fs::create_dir_all(main.join(".git").join("worktrees").join("wt")).unwrap();
        let wt = t.join("wt");
        std::fs::create_dir_all(wt.join("web").join("src")).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", main.join(".git/worktrees/wt").display())).unwrap();
        assert_eq!(main_checkout(&wt.join("web").join("src")), Some(main.join("web").join("src")));
        assert_eq!(main_checkout(&main.join("x")), None);
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn parses_remotes_like_the_old_resolver() {
        let r = parse_remote("https://github.com/AcmeLabs/Orbit.Docs.git").unwrap();
        assert_eq!((r.owner.as_str(), r.repo.as_str()), ("acmelabs", "orbit.docs"));
        let r = parse_remote("git@github.com:Acme-Old/beacon.git").unwrap();
        assert_eq!((r.owner.as_str(), r.repo.as_str()), ("acme-old", "beacon"));
        assert_eq!(parse_remote("acme/app").unwrap().repo, "app");
        assert_eq!(parse_remote("https://github.com/a/b/").unwrap().repo, "b");
        assert!(parse_remote("not-a-remote").is_none());
        assert_eq!(owner_repo_raw("git@github.com:Acme-Corp/Acme-Studio.git").unwrap(), ("Acme-Corp".into(), "Acme-Studio".into()));
    }

    /// `cargo test bench_remote_regex -- --ignored --nocapture`: dřívější `github_url` kompiloval regex
    /// při každém volání (každý projekt v každém snapshotu), teď sdílený `OWNER_REPO_RE`.
    #[test]
    #[ignore]
    #[allow(clippy::regex_creation_in_loops)] // záměr: měří právě kompilaci v každém volání
    fn bench_remote_regex() {
        let r = "https://github.com/Acme-Corp/Acme-Studio.git";
        let n = 2000;
        let t = std::time::Instant::now();
        for _ in 0..n {
            let re = Regex::new(r"([^/\s:@]+)/([^/\s]+)$").unwrap();
            assert!(re.captures(r.trim_end_matches(".git")).is_some());
        }
        let per_compile = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        let t = std::time::Instant::now();
        for _ in 0..n {
            assert!(owner_repo_raw(r).is_some());
        }
        let per_cached = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!("Regex::new + captures: {per_compile:.1} µs · sdílený regex: {per_cached:.2} µs");
    }

    fn fake_repo(root: &Path, dir: &str, url: &str) {
        let g = root.join(dir).join(".git");
        std::fs::create_dir_all(&g).unwrap();
        std::fs::write(
            g.join("config"),
            format!("[core]\n\tbare = false\n[remote \"origin\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"),
        )
        .unwrap();
    }

    #[test]
    fn resolves_renamed_folder_and_refuses_ambiguity() {
        let t = tempfile::tempdir().unwrap();
        fake_repo(t.path(), "nova-os", "https://github.com/acme/app.git");
        fake_repo(t.path(), "beacon", "https://github.com/acme-old/beacon.git");
        fake_repo(t.path(), "a", "https://github.com/x/dup.git");
        fake_repo(t.path(), "b", "https://github.com/x/dup.git");
        std::fs::create_dir_all(t.path().join("not-a-repo")).unwrap();
        let idx = RepoIndex::build(&[t.path().to_string_lossy().into_owned()]);
        assert_eq!(idx.resolve("acme/app").unwrap().file_name().unwrap(), "nova-os");
        // vlastník přejmenován, jméno repa sedí jednoznačně
        assert_eq!(idx.resolve("https://github.com/acme/beacon").unwrap().file_name().unwrap(), "beacon");
        assert!(idx.resolve("someone-else/beacon").is_some());
        assert!(idx.resolve("x/dup").is_none());
        assert!(idx.resolve("nobody/nothing").is_none());
        assert_eq!((idx.clones("x/dup"), idx.clones("nobody/nothing"), idx.clones("acme/app")), (2, 0, 1));
    }

    #[test]
    fn fetch_line_never_leaks_into_url() {
        let t = tempfile::tempdir().unwrap();
        fake_repo(t.path(), "r", "https://github.com/o/r.git");
        assert_eq!(read_origin(&t.path().join("r")).unwrap(), "https://github.com/o/r.git");
    }
}
