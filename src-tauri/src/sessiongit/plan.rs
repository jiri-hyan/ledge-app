//! Panel Git v kartě Claude — rozhodnutí bez I/O: argumenty git příkazů (vždy pole, nikdy shell),
//! co a kam se pushne, smí-li destruktivní akce běžet vedle živé session, výběr souborů ke commitu
//! a texty potvrzení (přesně co se ztratí).

use super::parse::{FileKind, GitFile};
use crate::claude::paths::path_key;
use crate::i18n::tr;
use crate::sessions::Status;
use serde::Serialize;

/// Prostředí každého volání: pathspecy doslova (soubor `*.rs` nebo `:(top)x` není vzor) a bez ptaní
/// na heslo v terminálu, který nikdo nevidí.
pub const ENV: [(&str, &str); 2] = [("GIT_LITERAL_PATHSPECS", "1"), ("GIT_TERMINAL_PROMPT", "0")];
/// Čtení navíc bez zámku indexu (nepřekáží gitu, který právě spouští Claude).
pub const ENV_READ: [(&str, &str); 3] = [("GIT_LITERAL_PATHSPECS", "1"), ("GIT_TERMINAL_PROMPT", "0"), ("GIT_OPTIONAL_LOCKS", "0")];
/// Nejvíc souborů v panelu (zbytek jen počet).
pub const MAX_FILES: usize = 200;
/// Nejdelší zpráva commitu (znaky).
pub const MESSAGE_MAX: usize = 10_000;

pub const STATUS: [&str; 5] = ["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"];
pub const WORKTREES: [&str; 3] = ["worktree", "list", "--porcelain"];
pub const NUMSTAT: [&str; 8] = ["diff", "HEAD", "--numstat", "-z", "--no-renames", "--no-ext-diff", "--no-textconv", "--no-color"];
pub const TOPLEVEL: [&str; 2] = ["rev-parse", "--show-toplevel"];
pub const REMOTES: [&str; 1] = ["remote"];
/// commity HEAD, které nejsou na žádném remote (větev bez upstreamu)
pub const UNPUSHED: [&str; 5] = ["rev-list", "--count", "HEAD", "--not", "--remotes"];
pub const PULL: [&str; 2] = ["pull", "--ff-only"];
/// `git add` vybraných souborů — pathspecy jdou na stdin oddělené NUL (`pathspec_input`), takže
/// žádná délka příkazové řádky ani metaznak v cestě nevadí
pub const ADD: [&str; 3] = ["add", "--pathspec-from-file=-", "--pathspec-file-nul"];

/// `git diff` jednoho souboru (pracovní složka proti indexu, nebo `--cached` index proti HEAD).
pub fn diff_args(path: &str, staged: bool) -> Vec<String> {
    let mut a: Vec<String> = ["diff", "--no-color", "--no-ext-diff", "--no-textconv", "-U3"].map(String::from).to_vec();
    if staged {
        a.push("--cached".into());
    }
    a.extend(["--".into(), path.to_string()]);
    a
}

/// `git commit -m <zpráva>` jen vybraných cest (pathspec ze stdinu = `--only`: jiné připravené soubory
/// do commitu nejdou). Hooky gitu běží — žádné `--no-verify`.
pub fn commit_args(message: &str) -> Vec<String> {
    vec!["commit".into(), "-m".into(), message.to_string(), "--pathspec-from-file=-".into(), "--pathspec-file-nul".into()]
}

/// Cesty na stdin pro `--pathspec-from-file=- --pathspec-file-nul`.
pub fn pathspec_input(paths: &[String]) -> String {
    paths.iter().map(|p| format!("{p}\0")).collect()
}

/// Vrátit soubor do stavu HEAD (index i pracovní složka).
pub fn discard_args(path: &str) -> Vec<String> {
    ["restore", "--source=HEAD", "--staged", "--worktree", "--", path].map(String::from).to_vec()
}

/// Odstranit worktree — bez `--force`: se změnami / nesledovanými soubory git odmítne.
pub fn remove_worktree_args(path: &str) -> Vec<String> {
    ["worktree", "remove", path].map(String::from).to_vec()
}

/// Kam a jak se pushne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PushPlan {
    /// větev má upstream: `git push`, `count` = ahead
    Upstream { target: String, count: u32 },
    /// bez upstreamu, repo má jediný remote: `git push -u <remote> refs/heads/<b>:refs/heads/<b>`;
    /// `count` = commity, které nejsou na žádném remote (`None` = nevíme)
    SetUpstream { remote: String, branch: String, count: Option<u32> },
    /// pushnout nejde / není co — proč
    No { why: String },
}

/// Jméno, které by git vzal jako přepínač nebo force refspec (`-x`, `+x`), se nepoužije.
fn safe_name(s: &str) -> bool {
    !s.is_empty() && !s.starts_with(['-', '+']) && !s.contains(['\0', '\n', ':', ' '])
}

pub fn push_plan(branch: Option<&str>, upstream: Option<&str>, ahead: u32, remotes: &[String], unpushed: Option<u32>) -> PushPlan {
    let no = |why: &str| PushPlan::No { why: why.to_string() };
    let Some(branch) = branch else {
        return no(tr("Odpojená HEAD — není větev, kterou pushnout.", "Detached HEAD — no branch to push."));
    };
    if let Some(u) = upstream {
        return if ahead == 0 { no(tr("Nic k pushnutí — upstream má všechno.", "Nothing to push — upstream has everything.")) } else { PushPlan::Upstream { target: u.to_string(), count: ahead } };
    }
    match remotes {
        [] => no(tr("Repo nemá žádný remote.", "The repo has no remote.")),
        [r] if safe_name(r) && safe_name(branch) => PushPlan::SetUpstream { remote: r.clone(), branch: branch.to_string(), count: unpushed },
        [_] => no(tr("Neobvyklé jméno remote nebo větve — pushni ručně.", "Unusual remote or branch name — push manually.")),
        _ => no(tr("Víc remote a větev nemá upstream — nastav ho ručně (git push -u <remote> <větev>).", "Several remotes and no upstream — set it manually (git push -u <remote> <branch>).")),
    }
}

/// Argumenty pushe; `None` = podle plánu pushnout nejde. Nikdy `--force` ani `+refspec`.
pub fn push_args(plan: &PushPlan) -> Option<Vec<String>> {
    match plan {
        PushPlan::Upstream { .. } => Some(vec!["push".into()]),
        PushPlan::SetUpstream { remote, branch, .. } => Some(vec!["push".into(), "-u".into(), remote.clone(), format!("refs/heads/{branch}:refs/heads/{branch}")]),
        PushPlan::No { .. } => None,
    }
}

/// Text potvrzení pushe (remote/větev a počet commitů).
pub fn push_text(plan: &PushPlan) -> Option<String> {
    match plan {
        PushPlan::Upstream { target, count } => Some(trf!("Pushnout {count} commit(ů) do {target}?", "Push {count} commit(s) to {target}?")),
        PushPlan::SetUpstream { remote, branch, count: Some(n) } => Some(trf!(
            "Pushnout větev {branch} na {remote} (nová větev na remote, nastaví se upstream)? Commitů, které nejsou na žádném remote: {n}.",
            "Push branch {branch} to {remote} (new branch on the remote, upstream gets set)? Commits not on any remote: {n}."
        )),
        PushPlan::SetUpstream { remote, branch, count: None } => Some(trf!(
            "Pushnout větev {branch} na {remote} (nová větev na remote, nastaví se upstream)?",
            "Push branch {branch} to {remote} (new branch on the remote, upstream gets set)?"
        )),
        PushPlan::No { .. } => None,
    }
}

/// Stavy session, jejichž cwd leží v `root` (nebo v něm samém).
pub fn statuses_in(root: &str, rows: &[(String, Status)]) -> Vec<Status> {
    let root = path_key(root);
    rows.iter()
        .filter(|(cwd, _)| {
            let k = path_key(cwd);
            k == root || k.strip_prefix(&root).is_some_and(|t| t.starts_with('/'))
        })
        .map(|(_, s)| *s)
        .collect()
}

/// Smí destruktivní akce (zahodit změny, odstranit worktree) běžet? Session, která v té složce právě
/// pracuje nebo čeká na schválení, by o soubory přišla pod rukama → `Some(proč)`.
pub fn destructive_block(here: &[Status]) -> Option<String> {
    let busy = here.iter().any(|s| matches!(s, Status::Working | Status::Waiting));
    busy.then(|| {
        tr(
            "Session v této složce právě pracuje nebo čeká na tebe — zahodit ani odstranit teď nejde. Počkej, až doběhne, nebo ji ukonči.",
            "A session in this folder is working or waiting for you — discarding or removing is blocked. Wait until it finishes, or end it.",
        )
        .to_string()
    })
}

/// Jde soubor zahodit? Jen sledovaný soubor, který je v HEAD (vrátí se do posledního commitu); nový,
/// nesledovaný, přejmenovaný nebo v konfliktu ne (to by soubor smazalo / rozbilo slučování).
pub fn discardable(f: &GitFile) -> bool {
    f.kind == FileKind::Changed && f.in_head
}

/// Text druhého potvrzení „Zahodit změny v souboru“ — přesně co se ztratí.
pub fn discard_text(f: &GitFile) -> String {
    let lines = match (f.added, f.removed) {
        (Some(a), Some(r)) => trf!(" (+{a} −{r} řádků proti HEAD)", " (+{a} −{r} lines vs HEAD)"),
        _ => String::new(),
    };
    let what = match (f.staged(), f.unstaged()) {
        (true, true) => tr("neuložené i připravené (staged) změny", "unstaged and staged changes"),
        (true, false) => tr("připravené (staged) změny", "staged changes"),
        _ => tr("neuložené změny", "unstaged changes"),
    };
    let path = &f.path;
    trf!(
        "Zahodit změny v {path}? Ztratí se {what}{lines} — soubor se vrátí do stavu posledního commitu (HEAD). Nejde to vrátit zpět.",
        "Discard changes in {path}? You'll lose {what}{lines} — the file goes back to the last commit (HEAD). This can't be undone."
    )
}

/// Text druhého potvrzení „Odstranit worktree“.
pub fn remove_text(path: &str, branch: Option<&str>, main: &str, unpushed: Option<u32>, idle_here: bool) -> String {
    let keep = match branch {
        Some(b) => trf!("Větev {b} a její commity zůstanou v repu ({main}).", "Branch {b} and its commits stay in the repo ({main})."),
        None => trf!("Odpojená HEAD: commity, na které neukazuje žádná větev, půjde najít jen přes reflog v {main}.", "Detached HEAD: commits no branch points to can only be found via the reflog in {main}."),
    };
    let remote = match unpushed {
        Some(n) if n > 0 => trf!(" {n} commit(ů) není na žádném remote.", " {n} commit(s) aren't on any remote."),
        _ => String::new(),
    };
    let idle = if idle_here {
        tr(" V té složce ještě běží nečinná session — Windows složku nemusí smazat, dokud ji neukončíš.", " An idle session still runs in that folder — Windows may not delete it until you end it.")
    } else {
        ""
    };
    trf!(
        "Odstranit worktree {path}? Smaže se jeho složka; s nezacommitovanými nebo nesledovanými soubory git odmítne. {keep}{remote}{idle}",
        "Remove worktree {path}? Its folder gets deleted; git refuses if there are uncommitted or untracked files. {keep}{remote}{idle}"
    )
}

/// Zpráva commitu: ořízne, zahodí řídicí znaky (kromě nového řádku a tabulátoru), prázdná = chyba.
pub fn clean_message(msg: &str) -> Result<String, String> {
    let m: String = msg.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').take(MESSAGE_MAX).collect();
    let m = m.trim().to_string();
    if m.is_empty() { Err(tr("Napiš zprávu commitu.", "Write a commit message.").into()) } else { Ok(m) }
}

/// Vybrané cesty → (pro `git add`, pro `git commit`). Jen soubory, které status opravdu ukazuje (UI
/// nemůže podstrčit jinou cestu); u přejmenování jde do commitu i původní cesta; konflikt = chyba.
pub fn selection(selected: &[String], files: &[GitFile]) -> Result<(Vec<String>, Vec<String>), String> {
    let mut add = Vec::new();
    let mut commit = Vec::new();
    for s in selected {
        let Some(f) = files.iter().find(|f| f.path == *s) else {
            return Err(trf!("{s} už mezi změnami není — obnov panel.", "{s} is no longer among the changes — refresh the panel."));
        };
        if f.kind == FileKind::Conflict {
            return Err(trf!("{s} je v konfliktu — nejdřív ho vyřeš.", "{s} has a conflict — resolve it first."));
        }
        if !add.contains(&f.path) {
            add.push(f.path.clone());
            commit.push(f.path.clone());
            commit.extend(f.orig.clone());
        }
    }
    if add.is_empty() {
        return Err(tr("Vyber aspoň jeden soubor.", "Pick at least one file.").into());
    }
    Ok((add, commit))
}

/// Krátká hláška z výstupu gitu: posledních `n` neprázdných řádků.
pub fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessiongit::parse::status_z;

    const SHELL_META: [char; 9] = ['&', '|', ';', '<', '>', '^', '%', '`', '$'];

    #[test]
    fn args_are_plain_arrays() {
        // cesty s mezerami a metaznaky jdou jako jeden argument za `--`, nic se neinterpretuje
        let p = "src/a b&c|d;e.rs";
        assert_eq!(diff_args(p, false), ["diff", "--no-color", "--no-ext-diff", "--no-textconv", "-U3", "--", p]);
        assert_eq!(diff_args(p, true)[5..], ["--cached", "--", p]);
        assert_eq!(discard_args(p), ["restore", "--source=HEAD", "--staged", "--worktree", "--", p]);
        assert_eq!(remove_worktree_args("C:/x y/wt"), ["worktree", "remove", "C:/x y/wt"]);
        let c = commit_args("feat: \"uvozovky\" & $(nic)\n\ntělo");
        assert_eq!(c[..3], ["commit", "-m", "feat: \"uvozovky\" & $(nic)\n\ntělo"]);
        assert!(!c.iter().any(|a| a == "--no-verify" || a == "-n"));
        assert_eq!(pathspec_input(&["a b".into(), "c".into()]), "a b\0c\0");
        // žádný pevný příkaz nemá metaznaky shellu ani force
        for a in STATUS.iter().chain(&WORKTREES).chain(&NUMSTAT).chain(&PULL).chain(&ADD).chain(&UNPUSHED) {
            assert!(!a.contains(SHELL_META), "{a}");
            assert!(!a.contains("force"), "{a}");
        }
    }

    #[test]
    fn push_plan_table() {
        let r = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let up = push_plan(Some("main"), Some("origin/main"), 3, &r(&["origin"]), None);
        assert_eq!(up, PushPlan::Upstream { target: "origin/main".into(), count: 3 });
        assert_eq!(push_args(&up).unwrap(), ["push"]);
        assert!(matches!(push_plan(Some("main"), Some("origin/main"), 0, &r(&["origin"]), None), PushPlan::No { .. }));
        assert!(matches!(push_plan(None, None, 0, &r(&["origin"]), None), PushPlan::No { .. }));
        let set = push_plan(Some("w3/wtree"), None, 0, &r(&["origin"]), Some(2));
        assert_eq!(set, PushPlan::SetUpstream { remote: "origin".into(), branch: "w3/wtree".into(), count: Some(2) });
        assert_eq!(push_args(&set).unwrap(), ["push", "-u", "origin", "refs/heads/w3/wtree:refs/heads/w3/wtree"]);
        assert!(matches!(push_plan(Some("x"), None, 0, &r(&[]), None), PushPlan::No { .. }));
        assert!(matches!(push_plan(Some("x"), None, 0, &r(&["a", "b"]), None), PushPlan::No { .. }));
        // jméno, které by git vzal jako přepínač / force refspec
        assert!(matches!(push_plan(Some("+main"), None, 0, &r(&["origin"]), None), PushPlan::No { .. }));
        assert!(matches!(push_plan(Some("main"), None, 0, &r(&["--mirror"]), None), PushPlan::No { .. }));
        assert_eq!(push_args(&PushPlan::No { why: String::new() }), None);
        for plan in [up, set] {
            assert!(!push_args(&plan).unwrap().iter().any(|a| a.contains("force") || a.starts_with('+')));
        }
    }

    #[test]
    fn push_text_names_target_and_count() {
        let t = push_text(&PushPlan::Upstream { target: "origin/main".into(), count: 3 }).unwrap();
        assert!(t.contains("origin/main") && t.contains('3'), "{t}");
        let t = push_text(&PushPlan::SetUpstream { remote: "origin".into(), branch: "w3/x".into(), count: Some(2) }).unwrap();
        assert!(t.contains("origin") && t.contains("w3/x") && t.contains(": 2"), "{t}");
        assert_eq!(push_text(&PushPlan::No { why: "x".into() }), None);
    }

    #[test]
    fn destructive_guard_table() {
        let rows = vec![
            ("C:\\Users\\x\\wt".to_string(), Status::Working),
            ("C:/Users/x/wt/src".to_string(), Status::Idle),
            ("C:/Users/x/wt-jiny".to_string(), Status::Waiting),
            ("c:/users/x/WT2".to_string(), Status::Ended),
        ];
        assert_eq!(statuses_in("C:/Users/x/wt", &rows), vec![Status::Working, Status::Idle]);
        assert_eq!(statuses_in("C:/Users/x/wt2", &rows), vec![Status::Ended]);
        type Case = (&'static [Status], bool);
        let cases: [Case; 6] = [
            (&[], false),
            (&[Status::Idle], false),
            (&[Status::Ended], false),
            (&[Status::Working], true),
            (&[Status::Waiting], true),
            (&[Status::Idle, Status::Waiting], true),
        ];
        for (here, blocked) in cases {
            assert_eq!(destructive_block(here).is_some(), blocked, "{here:?}");
        }
    }

    #[test]
    fn discard_rules_and_text() {
        let s = status_z(
            "1 .M N... 100644 100644 100644 a a mod.rs\0\
             1 MM N... 100644 100644 100644 a b oba.rs\0\
             1 A. N... 000000 100644 100644 0 b novy.rs\0\
             2 R. N... 100644 100644 100644 a a R100 nove.rs\0stare.rs\0\
             u UU N... 100644 100644 100644 100644 a b c k.rs\0? n.txt\0",
        );
        let ok: Vec<bool> = s.files.iter().map(discardable).collect();
        assert_eq!(ok, [true, true, false, false, false, false]);
        let mut f = s.files[1].clone();
        f.added = Some(3);
        f.removed = Some(1);
        let t = discard_text(&f);
        assert!(t.contains("oba.rs") && t.contains("neuložené i připravené") && t.contains("+3 −1") && t.contains("HEAD"), "{t}");
        assert!(discard_text(&s.files[0]).contains("neuložené změny"));
        assert!(!discard_text(&s.files[0]).contains("řádků"));
    }

    #[test]
    fn remove_text_says_what_stays_and_what_is_lost() {
        let t = remove_text("C:/wt", Some("w3/x"), "C:/main", Some(2), false);
        assert!(t.contains("C:/wt") && t.contains("w3/x") && t.contains("C:/main") && t.contains("2 commit"), "{t}");
        assert!(!remove_text("C:/wt", Some("b"), "C:/m", Some(0), false).contains("remote"));
        assert!(remove_text("C:/wt", None, "C:/m", None, true).contains("reflog"));
        assert!(remove_text("C:/wt", None, "C:/m", None, true).contains("nečinná"));
    }

    #[test]
    fn message_and_selection() {
        assert_eq!(clean_message("  feat: x\u{7}\r\n\ntělo\t ").unwrap(), "feat: x\n\ntělo");
        assert!(clean_message(" \n\t ").is_err());
        assert_eq!(clean_message(&"a".repeat(MESSAGE_MAX + 5)).unwrap().len(), MESSAGE_MAX);
        let s = status_z("1 .M N... 100644 100644 100644 a a a b.rs\u{0}2 R. N... 100644 100644 100644 a a R100 nove.rs\0stare.rs\0u UU N... 1 1 1 1 a b c k.rs\0? n.txt\0");
        let sel = |v: &[&str]| selection(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>(), &s.files);
        let (add, commit) = sel(&["a b.rs", "nove.rs", "n.txt", "a b.rs"]).unwrap();
        assert_eq!(add, ["a b.rs", "nove.rs", "n.txt"]);
        assert_eq!(commit, ["a b.rs", "nove.rs", "stare.rs", "n.txt"]);
        assert!(sel(&[]).is_err());
        assert!(sel(&["k.rs"]).unwrap_err().contains("konflikt"));
        // cesta, kterou status neukazuje (podstrčená z UI), neprojde
        assert!(sel(&["../../etc/passwd"]).is_err());
    }

    #[test]
    fn tail_keeps_last_lines() {
        assert_eq!(tail("a\n\n b \nc\n", 2), " b\nc");
        assert_eq!(tail("", 3), "");
    }
}
