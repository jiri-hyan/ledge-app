//! Čisté rozhodování synchronizace poznámek (bez I/O, testované tabulkami): plán prvního běhu
//! (`plan_init`), klasifikace chyb gitu (`classify`, `short_err`), jména a `.gitignore`
//! (`sanitize_pc`, `commit_message`, `ensure_lines`) a plán slučování lokálních souborů do klonu
//! remote (`merge_plan`). I/O je v `git.rs`, `init.rs`, `cycle.rs`.

use super::{Outcome, auth_hint};
use chrono::{DateTime, FixedOffset};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootState {
    /// má vlastní `.git`
    Repo,
    /// neexistuje nebo v něm nejsou poznámky (jen `.draft` / `.state.json` …)
    Empty,
    /// jsou v něm poznámky, ale není to repo
    LocalFiles,
    /// leží uvnitř jiného git repa (toplevel)
    InsideOtherRepo(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteState {
    Empty,
    HasCommits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitPlan {
    /// a) `init -b main` + commit všeho + push
    Fresh,
    /// b) clone vedle, sloučit lokální soubory, commit + push, prohodit složky
    CloneMerge,
    /// c) už je repo — jen ověřit `origin`
    EnsureOrigin,
    Refuse(String),
}

/// Rozhodnutí podle kořene a remote. `remote` je `None`, když se na něj kvůli stavu kořene neptáme.
pub fn plan_init(root: &RootState, remote: Option<RemoteState>) -> Option<InitPlan> {
    Some(match (root, remote) {
        (RootState::Repo, _) => InitPlan::EnsureOrigin,
        (RootState::InsideOtherRepo(top), _) => {
            InitPlan::Refuse(trf!(
                "Kořen poznámek leží uvnitř jiného git repa ({top}) — synchronizaci tam nespouštím.",
                "The notes folder is inside another git repo ({top}) — not syncing there."
            ))
        }
        (RootState::Empty | RootState::LocalFiles, Some(RemoteState::Empty)) => InitPlan::Fresh,
        (RootState::Empty | RootState::LocalFiles, Some(RemoteState::HasCommits)) => InitPlan::CloneMerge,
        // stav remote neznámý (offline) → nic nezakládat, jinak by vznikly dvě nezávislé historie
        (RootState::Empty | RootState::LocalFiles, None) => return None,
    })
}

/// Chybový výstup gitu u síťové operace → offline / nepřihlášeno / jiná chyba.
pub fn classify(stderr: &str) -> Outcome {
    let s = stderr.to_lowercase();
    let msg = short_err(stderr);
    const AUTH: [&str; 13] = [
        "authentication failed",
        "could not read username",
        "could not read password",
        "terminal prompts disabled",
        "permission denied (publickey",
        "invalid username or password",
        "logon failed",
        "error: 401",
        "error: 403",
        "repository not found",
        "host key verification failed",
        "invalid credentials",
        "permission to ",
    ];
    const NET: [&str; 13] = [
        "could not resolve host",
        "could not resolve proxy",
        "failed to connect",
        "couldn't connect",
        "could not connect",
        "connection timed out",
        "operation timed out",
        "timed out",
        "network is unreachable",
        "connection refused",
        "connection reset",
        "connection was reset",
        "no route to host",
    ];
    if AUTH.iter().any(|p| s.contains(p)) {
        Outcome::Neprihlaseno(format!("{msg} — {}", auth_hint()))
    } else if NET.iter().any(|p| s.contains(p)) {
        Outcome::Offline(msg)
    } else {
        Outcome::Chyba(if msg.is_empty() { crate::i18n::tr("git selhal bez hlášky", "git failed without a message").into() } else { msg })
    }
}

/// Podstatné řádky chyby (`fatal:` / `error:` / `remote:`), nejvýš dva, zkrácené.
pub fn short_err(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let key: Vec<&str> =
        lines.iter().copied().filter(|l| ["fatal:", "error:", "remote:"].iter().any(|p| l.to_lowercase().starts_with(p))).collect();
    let pick = if key.is_empty() { lines } else { key };
    let s = pick.into_iter().take(2).collect::<Vec<_>>().join(" · ");
    if s.chars().count() > 300 { s.chars().take(300).collect::<String>() + "…" } else { s }
}

/// `COMPUTERNAME` jako bezpečná přípona jména souboru (ASCII, `-`); prázdné = `pc`.
pub fn sanitize_pc(name: &str) -> String {
    let s: String = name.trim().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "pc".into() } else { s }
}

pub fn commit_message(pc: &str, now: &DateTime<FixedOffset>) -> String {
    format!("ledge: {pc} {}", crate::notes::iso(now))
}

/// Chybějící řádky do `.gitignore` (existující obsah se nemění). `None` = nic doplňovat.
pub fn ensure_lines(existing: &str, lines: &[&str]) -> Option<String> {
    let have: HashSet<&str> = existing.lines().map(str::trim).collect();
    let missing: Vec<&str> = lines.iter().copied().filter(|l| !have.contains(l)).collect();
    if missing.is_empty() {
        return None;
    }
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for l in missing {
        out.push_str(l);
        out.push('\n');
    }
    Some(out)
}

/// Soubor patří jen tomuto PC (gitignore) — při slučování se přenese beze změny jména.
pub(super) fn local_only(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    rel.split('/').any(|c| c == ".draft" || c == ".smazane") || name == ".state.json" || name == "PRESUNUTO-DO-LEDGE.txt"
}

pub(super) fn is_tmp(rel: &str) -> bool {
    rel.to_lowercase().ends_with(".tmp")
}

/// Jak je na tom lokální soubor proti klonu remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Missing,
    Same,
    Different,
}

/// Jeden krok slučování: zkopírovat lokální `from` do klonu jako `to`, v textu (`.md`) nahradit
/// odkazy na přejmenované obrázky.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copy {
    pub from: String,
    pub to: String,
    pub rewrite: Vec<(String, String)>,
}

fn split_rel(rel: &str) -> (&str, &str) {
    match rel.rfind('/') {
        Some(i) => (&rel[..=i], &rel[i + 1..]),
        None => ("", rel),
    }
}

fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Plán slučování lokálních souborů do klonu remote. Stejný obsah → nic, chybí v klonu → kopie,
/// jiný obsah → lokální verze pod jménem `<jméno>-<pc>` — u poznámky celá skupina (`.md` + její
/// obrázky `<základ>-N.png`, odkazy v `.md` přepsané), aby se obrázky nepomíchaly. `taken` = cesta
/// v klonu existuje. Lokální soubory (`.draft`, `.state.json`) se kopírují vždy pod svým jménem.
pub fn merge_plan(files: &[(String, Cmp)], pc: &str, taken: impl Fn(&str) -> bool) -> Vec<Copy> {
    let cmp_of = |rel: &str| files.iter().find(|(r, _)| r == rel).map(|(_, c)| *c);
    let mut out = Vec::new();
    let mut planned: HashSet<String> = HashSet::new();
    let mut claimed: HashSet<&str> = HashSet::new();
    let free = |p: &str, planned: &HashSet<String>| !taken(p) && !planned.contains(p);

    // skupiny poznámek
    for (rel, _) in files.iter().filter(|(r, _)| !local_only(r) && !is_tmp(r) && r.to_lowercase().ends_with(".md")) {
        let (dir, name) = split_rel(rel);
        let stem = &name[..name.len() - 3];
        let images: Vec<&str> = files
            .iter()
            .map(|(r, _)| r.as_str())
            .filter(|r| !claimed.contains(r))
            .filter(|r| {
                let (d, n) = split_rel(r);
                d == dir
                    && n.to_lowercase().ends_with(".png")
                    && n.strip_prefix(stem)
                        .and_then(|x| x.strip_prefix('-'))
                        .map(|x| &x[..x.len() - 4])
                        .is_some_and(|num| !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()))
            })
            .collect();
        claimed.insert(rel.as_str());
        claimed.extend(images.iter().copied());
        let members: Vec<&str> = std::iter::once(rel.as_str()).chain(images.iter().copied()).collect();
        let conflict = members.iter().any(|m| cmp_of(m) == Some(Cmp::Different));
        if !conflict {
            for m in members.iter().filter(|m| cmp_of(m) == Some(Cmp::Missing)) {
                planned.insert(m.to_string());
                out.push(Copy { from: m.to_string(), to: m.to_string(), rewrite: Vec::new() });
            }
            continue;
        }
        // nový základ, pod kterým je volná poznámka i všechny její obrázky
        let new_stem = (1..)
            .map(|n| if n == 1 { format!("{stem}-{pc}") } else { format!("{stem}-{pc}-{n}") })
            .find(|s| {
                free(&format!("{dir}{s}.md"), &planned)
                    && images.iter().all(|img| free(&format!("{dir}{s}{}", &split_rel(img).1[stem.len()..]), &planned))
            })
            .expect("nekonečná řada jmen");
        let mut rewrite = Vec::new();
        for img in &images {
            let old = split_rel(img).1;
            let new = format!("{new_stem}{}", &old[stem.len()..]);
            rewrite.push((format!("./{old}"), format!("./{new}")));
            rewrite.push((format!("({old})"), format!("({new})")));
            let to = format!("{dir}{new}");
            planned.insert(to.clone());
            out.push(Copy { from: img.to_string(), to, rewrite: Vec::new() });
        }
        let to = format!("{dir}{new_stem}.md");
        planned.insert(to.clone());
        out.push(Copy { from: rel.clone(), to, rewrite });
    }
    // zbytek (obrázky bez poznámky, jiné soubory, lokální stav)
    for (rel, c) in files.iter().filter(|(r, _)| !claimed.contains(r.as_str()) && !is_tmp(r)) {
        if local_only(rel) {
            out.push(Copy { from: rel.clone(), to: rel.clone(), rewrite: Vec::new() });
            continue;
        }
        match c {
            Cmp::Same => {}
            Cmp::Missing => {
                planned.insert(rel.clone());
                out.push(Copy { from: rel.clone(), to: rel.clone(), rewrite: Vec::new() });
            }
            Cmp::Different => {
                let (dir, name) = split_rel(rel);
                let (stem, ext) = split_ext(name);
                let to = (1..)
                    .map(|n| if n == 1 { format!("{dir}{stem}-{pc}{ext}") } else { format!("{dir}{stem}-{pc}-{n}{ext}") })
                    .find(|p| free(p, &planned))
                    .expect("nekonečná řada jmen");
                planned.insert(to.clone());
                out.push(Copy { from: rel.clone(), to, rewrite: Vec::new() });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::IGNORE;
    use super::*;

    fn now() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-10-16T14:03:22+02:00").unwrap()
    }

    #[test]
    fn plan_init_table() {
        use InitPlan::*;
        let other = RootState::InsideOtherRepo("C:/Users/x".into());
        let rows: [(&RootState, Option<RemoteState>, Option<InitPlan>); 8] = [
            (&RootState::Repo, None, Some(EnsureOrigin)),
            (&RootState::Repo, Some(RemoteState::HasCommits), Some(EnsureOrigin)),
            (&RootState::Empty, Some(RemoteState::Empty), Some(Fresh)),
            (&RootState::LocalFiles, Some(RemoteState::Empty), Some(Fresh)),
            (&RootState::Empty, Some(RemoteState::HasCommits), Some(CloneMerge)),
            (&RootState::LocalFiles, Some(RemoteState::HasCommits), Some(CloneMerge)),
            // offline: nic nezakládat
            (&RootState::LocalFiles, None, None),
            (&RootState::Empty, None, None),
        ];
        for (root, remote, want) in rows {
            assert_eq!(plan_init(root, remote), want, "{root:?} {remote:?}");
        }
        assert!(matches!(plan_init(&other, Some(RemoteState::Empty)), Some(Refuse(m)) if m.contains("C:/Users/x")));
    }

    #[test]
    fn classify_table() {
        let rows = [
            ("fatal: could not read Username for 'https://github.com': terminal prompts disabled", "neprihlaseno"),
            ("remote: Repository not found.\nfatal: repository 'https://github.com/acme/notes.git/' not found", "neprihlaseno"),
            ("remote: Invalid username or password.\nfatal: Authentication failed for 'https://github.com/x/y.git/'", "neprihlaseno"),
            ("git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.", "neprihlaseno"),
            ("fatal: unable to access 'https://github.com/x/y.git/': The requested URL returned error: 403", "neprihlaseno"),
            ("fatal: unable to access 'https://github.com/x/y.git/': Could not resolve host: github.com", "offline"),
            ("fatal: unable to access 'https://github.com/x/y.git/': Failed to connect to github.com port 443 after 21 ms: Couldn't connect to server", "offline"),
            ("ssh: connect to host github.com port 22: Connection timed out", "offline"),
            ("fatal: 'C:/neni/repo.git' does not appear to be a git repository", "chyba"),
            ("", "chyba"),
        ];
        for (err, want) in rows {
            let got = match classify(err) {
                Outcome::Neprihlaseno(m) => {
                    assert!(m.contains("gh auth setup-git"), "{m}");
                    "neprihlaseno"
                }
                Outcome::Offline(_) => "offline",
                Outcome::Chyba(_) => "chyba",
                o => panic!("{o:?}"),
            };
            assert_eq!(got, want, "{err}");
        }
        assert_eq!(short_err("hint: x\nfatal: a\nerror: b\nfatal: c\n"), "fatal: a · error: b");
        assert_eq!(short_err("jen text\n"), "jen text");
    }

    #[test]
    fn small_helpers() {
        assert_eq!(sanitize_pc("DESKTOP-AB12"), "DESKTOP-AB12");
        assert_eq!(sanitize_pc(" Jirkův PC "), "Jirk-v-PC");
        assert_eq!(sanitize_pc("  "), "pc");
        assert_eq!(commit_message("PC1", &now()), "ledge: PC1 2026-10-16T14:03:22+02:00");
        assert_eq!(ensure_lines(".draft/\n.state.json\n*.tmp\nPRESUNUTO-DO-LEDGE.txt\n.smazane/\n", &IGNORE), None);
        assert_eq!(ensure_lines("", &IGNORE).unwrap(), ".draft/\n.state.json\n*.tmp\nPRESUNUTO-DO-LEDGE.txt\n.smazane/\n");
        // starý .gitignore bez odkladu smazaných se doplní
        assert_eq!(ensure_lines(".draft/\n.state.json\n*.tmp\nPRESUNUTO-DO-LEDGE.txt\n", &IGNORE).unwrap(), ".draft/\n.state.json\n*.tmp\nPRESUNUTO-DO-LEDGE.txt\n.smazane/\n");
        assert_eq!(ensure_lines("vlastni\n.draft/", &IGNORE).unwrap(), "vlastni\n.draft/\n.state.json\n*.tmp\nPRESUNUTO-DO-LEDGE.txt\n.smazane/\n");
        assert!(local_only(".draft/draft.json") && local_only(".state.json") && local_only("PRESUNUTO-DO-LEDGE.txt") && local_only(".smazane/1-1/app/a.md"));
        assert!(!local_only("_obecne/x.md") && !local_only(".gitignore"));
    }

    #[test]
    fn merge_plan_table() {
        let f = |v: &[(&str, Cmp)]| v.iter().map(|(r, c)| (r.to_string(), *c)).collect::<Vec<_>>();
        let cp = |from: &str, to: &str| Copy { from: from.into(), to: to.into(), rewrite: Vec::new() };
        // stejné → nic, chybějící → kopie pod svým jménem, lokální stav vždy, tmp nikdy
        let files = f(&[
            ("_obecne/a.md", Cmp::Same),
            ("_obecne/b.md", Cmp::Missing),
            ("_obecne/b-1.png", Cmp::Missing),
            (".draft/draft.json", Cmp::Missing),
            (".state.json", Cmp::Different),
            ("_obecne/x.md.123-0.tmp", Cmp::Missing),
        ]);
        let plan = merge_plan(&files, "PCB", |_| false);
        assert_eq!(plan, vec![cp("_obecne/b.md", "_obecne/b.md"), cp("_obecne/b-1.png", "_obecne/b-1.png"), cp(".draft/draft.json", ".draft/draft.json"), cp(".state.json", ".state.json")]);

        // jiný obsah poznámky → celá skupina pod -PCB, odkazy přepsané; obsazené jméno → -PCB-2
        let files = f(&[("m/n.md", Cmp::Different), ("m/n-1.png", Cmp::Same), ("m/n-2.png", Cmp::Missing), ("m/n-x.png", Cmp::Different)]);
        let plan = merge_plan(&files, "PCB", |p| p == "m/n-PCB.md");
        assert_eq!(
            plan,
            vec![
                cp("m/n-1.png", "m/n-PCB-2-1.png"),
                cp("m/n-2.png", "m/n-PCB-2-2.png"),
                Copy {
                    from: "m/n.md".into(),
                    to: "m/n-PCB-2.md".into(),
                    rewrite: vec![
                        ("./n-1.png".into(), "./n-PCB-2-1.png".into()),
                        ("(n-1.png)".into(), "(n-PCB-2-1.png)".into()),
                        ("./n-2.png".into(), "./n-PCB-2-2.png".into()),
                        ("(n-2.png)".into(), "(n-PCB-2-2.png)".into()),
                    ],
                },
                // obrázek, který k poznámce nepatří (`-x`), se přejmenuje sám
                cp("m/n-x.png", "m/n-x-PCB.png"),
            ]
        );
        // jiný jen obrázek → přejmenuje se celá poznámka (aby odkaz ukazoval na lokální obrázek)
        let files = f(&[("n.md", Cmp::Same), ("n-1.png", Cmp::Different)]);
        let plan = merge_plan(&files, "P", |_| false);
        assert_eq!(plan.iter().map(|c| c.to.as_str()).collect::<Vec<_>>(), vec!["n-P-1.png", "n-P.md"]);
    }
}
