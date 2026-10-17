//! Git testy panelu nad dočasnými repy (tempdir, vlastní identita a `core.hooksPath`, žádné globální hooky).

use super::*;
use std::fs;

fn patient() -> Waits {
    let long = Duration::from_secs(60);
    Waits { read: long, commit: long, net: long, write: long }
}

fn g(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git").args(["-c", "commit.gpgsign=false"]).args(args).current_dir(dir).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// repo s identitou, vlastní složkou hooků a jedním commitem (`base.txt`, `keep.txt`)
fn repo() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    g(d, &["init", "-q", "-b", "main"]);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", ".git/hooks"), ("core.autocrlf", "false")] {
        g(d, &["config", k, v]);
    }
    fs::write(d.join("base.txt"), "a\nb\nc\n").unwrap();
    fs::write(d.join("keep.txt"), "k\n").unwrap();
    g(d, &["add", "."]);
    g(d, &["commit", "-q", "-m", "base"]);
    t
}

fn open(d: &Path) -> Repo {
    read_repo(d, patient()).unwrap().expect("repo")
}

fn paths(r: &Repo) -> Vec<String> {
    r.st.files.iter().map(|f| f.path.clone()).collect()
}

#[test]
fn not_a_repo_or_missing_dir_is_none() {
    let t = tempfile::tempdir().unwrap();
    assert!(read_repo(t.path(), patient()).unwrap().is_none());
    assert!(read_repo(&t.path().join("neni"), patient()).unwrap().is_none());
}

#[test]
fn status_files_numstat_and_diffs() {
    let t = repo();
    let d = t.path();
    fs::create_dir(d.join("sub dir")).unwrap();
    fs::write(d.join("base.txt"), "a\nB\nc\nd\n").unwrap();
    fs::write(d.join("sub dir/nový soubor.txt"), "1\n2\n").unwrap();
    fs::write(d.join("staged.txt"), "s\n").unwrap();
    g(d, &["add", "staged.txt"]);
    // čte se i z podsložky (cwd session nemusí být kořen)
    let r = open(&d.join("sub dir"));
    assert_eq!(path_key(&r.root), path_key(&d.to_string_lossy()));
    assert_eq!(paths(&r), ["base.txt", "staged.txt", "sub dir/nový soubor.txt"]);
    let base = r.file("base.txt").unwrap();
    assert_eq!((base.added, base.removed, base.in_head), (Some(2), Some(1), true));
    let v = view(&r, &[]);
    assert_eq!((v.branch.as_deref(), v.total, v.worktree.is_none(), v.block.is_none(), v.can_pull), (Some("main"), 3, true, true, false));
    let disc: Vec<bool> = v.files.iter().map(|f| f.discard.is_some()).collect();
    assert_eq!(disc, [true, false, false]);
    assert!(matches!(v.push, PushPlan::No { .. }), "bez remote: {:?}", v.push);

    // diff pracovní složky s čísly řádků
    let p = file_diff(&r, "base.txt", false, patient()).unwrap();
    assert_eq!((p.shown, p.added, p.removed), (Shown::Diff, 2, 1));
    assert!(p.lines.iter().any(|l| l.text == "B" && l.new == Some(2)));
    // připravený soubor: --cached
    let p = file_diff(&r, "staged.txt", true, patient()).unwrap();
    assert_eq!((p.added, p.lines.len()), (1, 1));
    assert_eq!(file_diff(&r, "staged.txt", false, patient()).unwrap().lines.len(), 0);
    // nesledovaný = začátek nového obsahu
    let p = file_diff(&r, "sub dir/nový soubor.txt", false, patient()).unwrap();
    assert_eq!((p.shown, p.total), (Shown::NewFile, 2));
    // cesta mimo status neprojde
    assert!(file_diff(&r, "../mimo.txt", false, patient()).is_err());
}

#[test]
fn commit_only_selected_and_literal_paths() {
    let t = repo();
    let d = t.path();
    fs::write(d.join("[ab].txt"), "vzor\n").unwrap();
    fs::write(d.join("a.txt"), "a\n").unwrap();
    fs::write(d.join("jiny.txt"), "j\n").unwrap();
    g(d, &["add", "jiny.txt"]); // připravený, ale nevybraný → do commitu nesmí
    let r = open(d);
    let msg = "feat: \"uvozovky\" & $(nic) | x\n\ntělo";
    let out = commit(&r, msg, &["[ab].txt".into()], patient()).unwrap();
    assert!(out.contains("main"), "{out}");
    assert_eq!(g(d, &["log", "-1", "--format=%B"]).trim(), "feat: \"uvozovky\" & $(nic) | x\n\ntělo");
    assert_eq!(g(d, &["show", "--name-only", "--format=", "HEAD"]).trim(), "[ab].txt");
    let r = open(d);
    assert_eq!(paths(&r), ["jiny.txt", "a.txt"], "sledované napřed, pak nesledované");
    assert_eq!(r.file("jiny.txt").unwrap().x, "A", "jiný připravený soubor zůstal připravený");
    // prázdná zpráva / nic nevybráno / cizí cesta
    assert!(commit(&r, "  ", &["a.txt".into()], patient()).is_err());
    assert!(commit(&r, "x", &[], patient()).is_err());
    assert!(commit(&r, "x", &["base.txt".into()], patient()).is_err());
}

#[test]
fn commit_hook_failure_is_reported_not_bypassed() {
    let t = repo();
    let d = t.path();
    fs::write(d.join(".git/hooks/pre-commit"), "#!/bin/sh\necho hook-rika-ne >&2\nexit 1\n").unwrap();
    fs::write(d.join("base.txt"), "zmena\n").unwrap();
    let r = open(d);
    let before = g(d, &["rev-parse", "HEAD"]);
    let err = commit(&r, "x", &["base.txt".into()], patient()).unwrap_err();
    assert!(err.contains("hook-rika-ne") && err.contains("git commit selhal"), "{err}");
    assert_eq!(g(d, &["rev-parse", "HEAD"]), before);
}

#[test]
fn rename_commit_includes_original_path() {
    let t = repo();
    let d = t.path();
    g(d, &["mv", "keep.txt", "renamed.txt"]);
    let r = open(d);
    assert_eq!(r.file("renamed.txt").unwrap().orig.as_deref(), Some("keep.txt"));
    commit(&r, "mv", &["renamed.txt".into()], patient()).unwrap();
    assert!(open(d).st.files.is_empty(), "{:?}", paths(&open(d)));
}

#[test]
fn push_pull_with_upstream_and_new_branch() {
    let t = repo();
    let d = t.path();
    let remote = tempfile::tempdir().unwrap();
    g(remote.path(), &["init", "-q", "--bare", "-b", "main"]);
    g(d, &["remote", "add", "origin", &remote.path().to_string_lossy()]);
    // bez upstreamu, jeden remote → push -u
    let r = open(d);
    assert_eq!(r.push_plan(), PushPlan::SetUpstream { remote: "origin".into(), branch: "main".into(), count: Some(1) });
    push(&r, patient()).unwrap();
    let r = open(d);
    assert_eq!(r.st.upstream.as_deref(), Some("origin/main"));
    assert!(matches!(r.push_plan(), PushPlan::No { .. }));
    // nový commit → Upstream s počtem
    fs::write(d.join("base.txt"), "x\n").unwrap();
    commit(&open(d), "x", &["base.txt".into()], patient()).unwrap();
    let r = open(d);
    assert_eq!(r.push_plan(), PushPlan::Upstream { target: "origin/main".into(), count: 1 });
    assert!(view(&r, &[]).push_text.unwrap().contains("origin/main"));
    push(&r, patient()).unwrap();
    // pull --ff-only: druhý klon pushne, první stáhne
    let other = tempfile::tempdir().unwrap();
    g(other.path(), &["clone", "-q", "-b", "main", &remote.path().to_string_lossy(), "."]);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("core.hooksPath", ".git/hooks")] {
        g(other.path(), &["config", k, v]);
    }
    fs::write(other.path().join("z.txt"), "z\n").unwrap();
    g(other.path(), &["add", "z.txt"]);
    g(other.path(), &["commit", "-q", "-m", "z"]);
    g(other.path(), &["push", "-q"]);
    g(d, &["fetch", "-q"]);
    let r = open(d);
    assert_eq!(r.st.behind, 1);
    assert!(view(&r, &[]).can_pull);
    pull(&r, patient()).unwrap();
    assert!(d.join("z.txt").exists());
}

#[test]
fn discard_restores_head_and_respects_guard() {
    let t = repo();
    let d = t.path();
    fs::write(d.join("base.txt"), "rozbité\n").unwrap();
    g(d, &["add", "base.txt"]);
    fs::write(d.join("base.txt"), "ještě víc\n").unwrap();
    fs::write(d.join("novy.txt"), "n\n").unwrap();
    let r = open(d);
    // živá session v repu → blokace, soubor nedotčený
    assert!(discard(&r, "base.txt", &[Status::Working], patient()).is_err());
    assert_eq!(fs::read_to_string(d.join("base.txt")).unwrap(), "ještě víc\n");
    // nesledovaný soubor zahodit nejde (smazal by se)
    assert!(discard(&r, "novy.txt", &[], patient()).is_err());
    assert!(d.join("novy.txt").exists());
    discard(&r, "base.txt", &[Status::Idle], patient()).unwrap();
    assert_eq!(fs::read_to_string(d.join("base.txt")).unwrap(), "a\nb\nc\n");
    assert_eq!(paths(&open(d)), ["novy.txt"]);
}

#[test]
fn worktree_view_and_remove_without_force() {
    let t = repo();
    let d = t.path();
    let holder = tempfile::tempdir().unwrap();
    let wt = holder.path().join("wt s mezerou");
    g(d, &["worktree", "add", "-q", "-b", "w3/x", &wt.to_string_lossy()]);
    // hlavní checkout není worktree k odstranění
    let main = open(d);
    assert!(view(&main, &[]).worktree.is_none());
    assert!(remove_worktree(&main, &[], patient()).is_err());
    let r = open(&wt);
    let v = view(&r, &[Status::Idle]);
    let w = v.worktree.clone().expect("worktree");
    assert_eq!((w.branch.as_deref(), w.main_branch.as_deref(), w.locked), (Some("w3/x"), Some("main"), false));
    assert_eq!(path_key(&w.main), path_key(&d.to_string_lossy()));
    let text = w.remove.unwrap();
    assert!(text.contains("w3/x") && text.contains("nečinná"), "{text}");
    // pracující session → blokace
    assert!(remove_worktree(&r, &[Status::Waiting], patient()).is_err());
    // nesledovaný soubor → git bez --force odmítne, složka zůstane
    fs::write(wt.join("rozdelane.txt"), "x\n").unwrap();
    let err = remove_worktree(&open(&wt), &[], patient()).unwrap_err();
    assert!(err.contains("git worktree remove selhal"), "{err}");
    assert!(wt.join("rozdelane.txt").exists());
    fs::remove_file(wt.join("rozdelane.txt")).unwrap();
    remove_worktree(&open(&wt), &[], patient()).unwrap();
    assert!(!wt.exists());
    // větev zůstala
    assert!(g(d, &["branch", "--list", "w3/x"]).contains("w3/x"));
}
