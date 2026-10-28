//! Dohledání složky projektu podle git remote a nainstalované aplikace podle jména.
//!
//! Port dřívějšího resolveru repozitářů (`repoResolver.ts`) vč. jeho okrajových případů (fetch řádek nesmí
//! prosáknout do URL, dva klony = nejednoznačné → None). Linked worktree (`.git` soubor) se počítá jako
//! další klon téhož repa (`origin_any`) — dřív se přeskočil a projekt potichu běžel ze staré složky.
//! Víc klonů = výběr člověkem (`Place::Choose`, `candidates_dated` + `sort_candidates`), volba do `path`.

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

/// `origin` hlavního checkoutu i linked worktree (`.git` je soubor → config hlavního repa). Index rep
/// (`RepoIndex::build`) bere obojí: aktivní složka bývá právě worktree (`app-v2` vedle staré
/// `app`) a tichý výběr jen hlavních checkoutů pustil server ze staré složky. Submodul ne.
pub fn origin_any(dir: &Path) -> Option<String> {
    let g = dir.join(".git");
    if g.is_dir() {
        return read_origin(dir);
    }
    let main = main_from_gitfile(&std::fs::read_to_string(&g).ok()?)?;
    read_origin(&main)
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
                let Some(remote) = origin_any(&p) else { continue };
                let Some(or) = parse_remote(&remote) else { continue };
                idx.by_repo.entry(or.repo.clone()).or_default().push(p.clone());
                idx.by_full.entry(or).or_default().push(p);
            }
        }
        idx
    }

    /// Počet složek s rozpoznaným remote v indexu.
    pub fn len(&self) -> usize {
        self.by_repo.values().map(Vec::len).sum()
    }

    /// Kolik klonů repa index zná (přesná shoda vlastník/repo, jinak podle jména repa).
    pub fn clones(&self, remote: &str) -> usize {
        let Some(or) = parse_remote(remote) else { return 0 };
        self.by_full.get(&or).or_else(|| self.by_repo.get(&or.repo)).map_or(0, Vec::len)
    }

    /// Jediná složka repa, nebo nic (testy; jádro jde přes `place_of`, které rozliší i „víc klonů“).
    #[cfg(test)]
    pub fn resolve(&self, remote: &str) -> Option<PathBuf> {
        match self.candidates(remote) {
            [one] => Some(one.clone()),
            _ => None, // žádný, nebo dva klony téhož repa → nehádat (vybere člověk, `sort_candidates`)
        }
    }

    /// Všechny složky s tímhle repem (přesná shoda vlastník/repo, jinak podle jména repa) v pořadí
    /// objevení; pro výběr je seřadí `sort_candidates` podle data posledního commitu.
    pub fn candidates(&self, remote: &str) -> &[PathBuf] {
        let Some(or) = parse_remote(remote) else { return &[] };
        self.by_full.get(&or).or_else(|| self.by_repo.get(&or.repo)).map_or(&[], Vec::as_slice)
    }
}

/// Složka, kterou by projekt mohl použít (výběr „Našel jsem 2 složky s tímto repem…“).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub path: String,
    /// unix s posledního commitu na HEAD (`git log -1 --format=%ct`); `None` = nejde zjistit
    pub last_commit: Option<i64>,
    /// linked worktree (`.git` je soubor)
    pub worktree: bool,
}

/// Pořadí výběru: nejčerstvější commit první, bez data na konec, shoda → cesta (bez ohledu na velikost
/// písmen). Vždy stejné pro stejný vstup — nic náhodného ani podle pořadí `read_dir`.
pub fn sort_candidates(v: &mut [Candidate]) {
    v.sort_by(|a, b| {
        b.last_commit.is_some().cmp(&a.last_commit.is_some())
            .then(b.last_commit.cmp(&a.last_commit))
            .then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
            .then_with(|| a.path.cmp(&b.path))
    });
}

/// Datum posledního commitu (unix s) — git bez okna s termínem; jen na výslovné otevření výběru.
pub fn last_commit(dir: &Path) -> Option<i64> {
    let out = crate::gitinfo::run(dir, &["log", "-1", "--format=%ct"], std::time::Instant::now() + std::time::Duration::from_secs(4))?;
    out.trim().parse().ok()
}

/// Odkud projekt poběží — stav pro dlaždici (`ProjectView.place`) i start (`launch`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Place {
    /// složka je jasná (`path`, nebo jediný klon podle `remote`)
    Ok,
    /// `path` je v configu, ale složka neexistuje — chyba hned, žádné tiché hledání jinde
    Missing { path: String },
    /// bez `path` a podle `remote` je klonů víc — vybere člověk, volba se uloží do `path`
    Choose { count: usize },
    /// nic: bez `path` a klon na disku není
    None,
}

/// Čistá volba místa: `path` z configu vyhrává (i neexistující = `Missing`), jinak jediný kandidát.
pub fn place_of(path: Option<&str>, path_is_dir: bool, cands: &[PathBuf]) -> (Place, Option<PathBuf>) {
    match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) if path_is_dir => (Place::Ok, Some(PathBuf::from(p))),
        Some(p) => (Place::Missing { path: p.to_string() }, None),
        None => match cands {
            [] => (Place::None, None),
            [one] => (Place::Ok, Some(one.clone())),
            many => (Place::Choose { count: many.len() }, None),
        },
    }
}

/// Kandidáti s daty, seřazení (`sort_candidates`).
pub fn candidates_dated(paths: &[PathBuf]) -> Vec<Candidate> {
    let mut v: Vec<Candidate> = paths
        .iter()
        .map(|p| Candidate { path: p.to_string_lossy().into_owned(), last_commit: last_commit(p), worktree: p.join(".git").is_file() })
        .collect();
    sort_candidates(&mut v);
    v
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
    fn worktree_counts_as_second_clone() {
        // stará složka (hlavní checkout) + aktivní worktree vedle — dřív se worktree přeskočil a
        // `resolve` potichu vrátil starou složku
        let t = tempfile::tempdir().unwrap();
        fake_repo(t.path(), "orbit", "https://github.com/acme/orbit.git");
        let wt = t.path().join("orbit-v2");
        std::fs::create_dir_all(t.path().join("orbit/.git/worktrees/orbit-v2")).unwrap();
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", t.path().join("orbit/.git/worktrees/orbit-v2").display())).unwrap();
        assert_eq!(origin_any(&wt).as_deref(), Some("https://github.com/acme/orbit.git"));
        let idx = RepoIndex::build(&[t.path().to_string_lossy().into_owned()]);
        let mut names: Vec<_> = idx.candidates("acme/orbit").iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["orbit", "orbit-v2"]);
        assert!(idx.resolve("acme/orbit").is_none(), "dvě složky = nehádat");
        assert!(idx.candidates("nobody/x").is_empty());
    }

    #[test]
    fn candidates_sorted_freshest_first_and_deterministic() {
        let c = |path: &str, last: Option<i64>| Candidate { path: path.into(), last_commit: last, worktree: false };
        type Case = (&'static str, Vec<Candidate>, Vec<&'static str>);
        let cases: Vec<Case> = vec![
            ("čerstvější první", vec![c("C:/d/old", Some(100)), c("C:/d/new", Some(900))], vec!["C:/d/new", "C:/d/old"]),
            ("bez data na konec", vec![c("C:/d/a", None), c("C:/d/b", Some(1))], vec!["C:/d/b", "C:/d/a"]),
            ("shoda → cesta", vec![c("C:/d/Zeta", Some(5)), c("C:/d/alfa", Some(5))], vec!["C:/d/alfa", "C:/d/Zeta"]),
            ("vše bez data → cesta", vec![c("C:/d/b", None), c("C:/d/a", None)], vec!["C:/d/a", "C:/d/b"]),
            ("tři", vec![c("C:/d/m", Some(2)), c("C:/d/x", None), c("C:/d/n", Some(3))], vec!["C:/d/n", "C:/d/m", "C:/d/x"]),
        ];
        for (name, input, want) in cases {
            let mut a = input.clone();
            sort_candidates(&mut a);
            assert_eq!(a.iter().map(|x| x.path.as_str()).collect::<Vec<_>>(), want, "{name}");
            // obrácený vstup = stejný výsledek (nezávisle na pořadí read_dir)
            let mut b: Vec<Candidate> = input.into_iter().rev().collect();
            sort_candidates(&mut b);
            assert_eq!(a, b, "{name}: pořadí nesmí záviset na vstupu");
        }
    }

    #[test]
    fn place_never_guesses() {
        let one = [PathBuf::from("C:/d/a")];
        let two = [PathBuf::from("C:/d/a"), PathBuf::from("C:/d/b")];
        assert_eq!(place_of(Some("C:/d/x"), true, &two), (Place::Ok, Some(PathBuf::from("C:/d/x"))));
        // neexistující path = chyba, ne tichý pád na klon podle remote
        assert_eq!(place_of(Some("C:/d/x"), false, &one), (Place::Missing { path: "C:/d/x".into() }, None));
        assert_eq!(place_of(Some("  "), false, &one), (Place::Ok, Some(PathBuf::from("C:/d/a"))));
        assert_eq!(place_of(None, false, &one), (Place::Ok, Some(PathBuf::from("C:/d/a"))));
        assert_eq!(place_of(None, false, &two), (Place::Choose { count: 2 }, None));
        assert_eq!(place_of(None, false, &[]), (Place::None, None));
    }

    #[test]
    fn last_commit_of_real_repo_and_non_repo() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(last_commit(t.path()), None);
        let dir = t.path().join("r");
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| crate::gitinfo::run(&dir, args, std::time::Instant::now() + std::time::Duration::from_secs(20));
        if git(&["init", "-q"]).is_none() {
            return; // git na stroji není — zbytek nejde ověřit
        }
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        git(&["add", "a.txt"]).unwrap();
        if git(&["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "-m", "x"]).is_none() {
            return; // globální nastavení gitu commit nepustí (podpis, hooky) — datum nejde ověřit
        }
        // committer date = teď, stačí, že je to rozumné číslo
        let ts = last_commit(&dir).expect("datum commitu");
        assert!(ts > 1_600_000_000, "{ts}");
        let v = candidates_dated(&[t.path().to_path_buf(), dir.clone()]);
        assert_eq!(v[0].path, dir.to_string_lossy());
        assert_eq!(v[1].last_commit, None);
    }

    #[test]
    fn fetch_line_never_leaks_into_url() {
        let t = tempfile::tempdir().unwrap();
        fake_repo(t.path(), "r", "https://github.com/o/r.git");
        assert_eq!(read_origin(&t.path().join("r")).unwrap(), "https://github.com/o/r.git");
    }
}
