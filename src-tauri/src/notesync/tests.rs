//! Testy `notesync`: plánovač, stav pro UI a integrace — bare repo v `temp_dir` jako remote, dvě
//! složky = dvě PC (žádná síť, nikdy skutečný kořen poznámek).

use super::git::nul_list;
use super::init::rel_path;
use super::*;
use crate::gitinfo;
use chrono::{DateTime, FixedOffset};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn now() -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339("2026-10-16T14:03:22+02:00").unwrap()
}

#[test]
fn sched_table() {
    let t0 = Instant::now();
    let iv = Duration::from_secs(300);
    let s = |secs: u64| t0 + Duration::from_secs(secs);
    let mut sc = Sched::default();
    // start: žádný běh → hned
    assert_eq!(sc.deadline(t0, iv), t0);
    assert!(sc.on(Wake::Timer, t0, iv));
    sc.ran(t0);
    assert_eq!(sc.deadline(s(1), iv), s(300));
    assert!(!sc.on(Wake::Timer, s(299), iv));
    assert!(sc.on(Wake::Timer, s(300), iv));
    // otevření šuplíku: throttle 60 s od posledního běhu
    assert!(!sc.on(Wake::Opened, s(59), iv));
    assert!(sc.on(Wake::Opened, s(60), iv));
    // config / teď = hned
    assert!(sc.on(Wake::Config, s(1), iv) && sc.on(Wake::Now, s(1), iv));
    // lokální změna: debounce 10 s, další změna posune, strop 60 s od první
    assert!(!sc.on(Wake::Local, s(100), iv));
    assert_eq!(sc.deadline(s(100), iv), s(110));
    assert!(!sc.on(Wake::Local, s(105), iv));
    assert_eq!(sc.deadline(s(105), iv), s(115));
    for t in (110..=158).step_by(8) {
        sc.on(Wake::Local, s(t), iv);
    }
    assert_eq!(sc.deadline(s(158), iv), s(160), "strop minuta od první změny");
    assert!(sc.on(Wake::Timer, s(160), iv));
    sc.ran(s(160));
    assert_eq!(sc.deadline(s(161), iv), s(460));
}

#[test]
fn status_apply_keeps_last_ok() {
    let mut st = Status::default();
    st.apply(&Outcome::Ok { pulled: false, pushed: true, swapped: None }, 1000);
    assert_eq!((st.state, st.last_ok_ms), (State::Ok, Some(1000)));
    st.apply(&Outcome::Offline("Could not resolve host".into()), 2000);
    assert_eq!((st.state, st.last_ok_ms, st.message.as_deref()), (State::Offline, Some(1000), Some("Could not resolve host")));
    st.apply(&Outcome::Konflikt { files: vec!["a.md".into()], message: "m".into() }, 3000);
    assert_eq!((st.state, st.files.clone()), (State::Konflikt, vec!["a.md".to_string()]));
    st.apply(&Outcome::Ok { pulled: true, pushed: false, swapped: None }, 4000);
    assert!(st.files.is_empty() && st.message.is_none() && st.last_ok_ms == Some(4000));
    let j = serde_json::to_value(&st).unwrap();
    assert_eq!(j["state"], "ok");
    assert_eq!(j["lastOkMs"], 4000);
    let mut st = Status::default();
    st.apply(&Outcome::Neprihlaseno("x".into()), 1);
    assert_eq!(serde_json::to_value(&st).unwrap()["state"], "neprihlaseno");
    assert_eq!(st.last_ok_ms, None);
    assert!(outcome_text(&Outcome::Ok { pulled: false, pushed: false, swapped: None }).is_ok());
    assert!(outcome_text(&Outcome::Offline("x".into())).is_err());
}

// ---------- integrace: bare repo v temp jako remote, dvě složky = dvě PC ----------

struct Lab {
    dir: tempfile::TempDir,
    remote: String,
}

fn lab() -> Lab {
    let dir = tempfile::Builder::new().prefix("ledge-sync-").tempdir_in(std::env::temp_dir()).unwrap();
    let bare = dir.path().join("remote.git");
    let d = Instant::now() + Duration::from_secs(20);
    gitinfo::run(dir.path(), &["init", "-q", "--bare", "-b", "main", &bare.to_string_lossy()], d).expect("git init --bare");
    Lab { remote: bare.to_string_lossy().replace('\\', "/"), dir }
}

impl Lab {
    fn root(&self, pc: &str) -> PathBuf {
        self.dir.path().join(pc).join("notes")
    }
    fn sync(&self, s: &mut Syncer, pc: &str) -> Outcome {
        let root = self.root(pc);
        s.sync(&Ctx { root: &root, remote: &self.remote, pc, now: now() })
    }
}

fn write(root: &Path, rel: &str, text: &[u8]) {
    let p = rel_path(root, rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(rel_path(root, rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn ok(o: Outcome) -> (bool, bool, Option<PathBuf>) {
    match o {
        Outcome::Ok { pulled, pushed, swapped } => (pulled, pushed, swapped),
        o => panic!("čekal jsem Ok, je {o:?}"),
    }
}

fn tracked(root: &Path) -> Vec<String> {
    let out = gitinfo::run(root, &["ls-files", "-z"], Instant::now() + Duration::from_secs(10)).unwrap();
    nul_list(&out)
}

#[test]
fn two_pcs_sync_through_bare_repo() {
    let lab = lab();
    let (mut sa, mut sb) = (Syncer::default(), Syncer::default());
    let a = lab.root("A");
    // a) PC A: lokální poznámky + koncept, remote prázdný → init, commit, push
    write(&a, "_obecne/2026-10-16-140000-prvni.md", b"---\ndone: false\n---\n\nprvni\n");
    write(&a, ".draft/draft.json", b"{}");
    write(&a, ".state.json", b"[]");
    let (_, pushed, swapped) = ok(lab.sync(&mut sa, "A"));
    assert!(pushed && swapped.is_none());
    let t = tracked(&a);
    assert!(t.contains(&"_obecne/2026-10-16-140000-prvni.md".to_string()) && t.contains(&".gitignore".to_string()), "{t:?}");
    assert!(!t.iter().any(|f| f.starts_with(".draft") || f == ".state.json"), "lokální stav se necommituje: {t:?}");
    let ident = gitinfo::run(&a, &["log", "-1", "--format=%an <%ae>|%s"], Instant::now() + Duration::from_secs(10)).unwrap();
    assert_eq!(ident.trim(), "Ledge (A) <ledge@a.local>|ledge: A 2026-10-16T14:03:22+02:00");

    // b) PC B bez poznámek → clone + prohození; poznámka z A je na B
    let b = lab.root("B");
    ok(lab.sync(&mut sb, "B"));
    assert!(b.join(".git").exists());
    assert_eq!(read(&b, "_obecne/2026-10-16-140000-prvni.md"), "---\ndone: false\n---\n\nprvni\n");

    // zápis na A → sync A → sync B
    write(&a, "atlas/2026-10-16-150000-druha.md", b"druha\n");
    write(&a, "atlas/2026-10-16-150000-druha-1.png", b"\x89PNG obrazek");
    ok(lab.sync(&mut sa, "A"));
    let (pulled, _, _) = ok(lab.sync(&mut sb, "B"));
    assert!(pulled);
    assert_eq!(read(&b, "atlas/2026-10-16-150000-druha.md"), "druha\n");
    assert!(b.join("atlas/2026-10-16-150000-druha-1.png").exists());

    // souběžné nové poznámky na obou → obě na obou
    write(&a, "_obecne/z-a.md", b"z A\n");
    write(&b, "_obecne/z-b.md", b"z B\n");
    ok(lab.sync(&mut sa, "A"));
    ok(lab.sync(&mut sb, "B"));
    ok(lab.sync(&mut sa, "A"));
    for r in [&a, &b] {
        assert_eq!(read(r, "_obecne/z-a.md"), "z A\n");
        assert_eq!(read(r, "_obecne/z-b.md"), "z B\n");
    }
    // stejný `done` na obou (identická změna) → žádný konflikt
    write(&a, "_obecne/z-a.md", b"z A hotovo\n");
    write(&b, "_obecne/z-a.md", b"z A hotovo\n");
    ok(lab.sync(&mut sa, "A"));
    ok(lab.sync(&mut sb, "B"));

    // stejná poznámka změněná různě na obou → remote zůstane, lokální verze jako kopie, sync jede dál
    write(&a, "_obecne/2026-10-16-140000-prvni.md", b"---\ndone: true\n---\n\nprvni A\n");
    write(&b, "_obecne/2026-10-16-140000-prvni.md", b"---\ndone: false\n---\n\nprvni B\n");
    write(&b, "_obecne/jina.md", b"jina z B\n");
    ok(lab.sync(&mut sa, "A"));
    let (pulled, pushed, _) = ok(lab.sync(&mut sb, "B"));
    assert!(pulled && pushed, "konflikt nezastavil sync");
    assert_eq!(read(&b, "_obecne/2026-10-16-140000-prvni.md"), "---\ndone: true\n---\n\nprvni A\n", "na místě verze z remote");
    let copy = "_obecne/2026-10-16-140000-prvni (konflikt Ledge 2026-10-16).md";
    assert_eq!(read(&b, copy), "---\ndone: false\n---\n\nprvni B\n", "lokální verze v kopii");
    assert!(!b.join(".git/rebase-merge").exists() && !b.join(".git/MERGE_HEAD").exists());
    assert!(sb.notes.iter().any(|n| n.contains("kopie") && n.contains(copy)), "{:?}", sb.notes);
    // druhý PC dostane kopii i nezávislou poznámku; další běhy jsou klidné
    ok(lab.sync(&mut sa, "A"));
    assert_eq!(read(&a, copy), "---\ndone: false\n---\n\nprvni B\n");
    assert_eq!(read(&a, "_obecne/jina.md"), "jina z B\n");
    ok(lab.sync(&mut sb, "B"));
}

/// Tabulka scénářů konfliktu dvou PC: žádná verze se neztratí, sync nestojí.
#[test]
fn conflict_scenarios_keep_all_data() {
    let lab = lab();
    let (mut sa, mut sb) = (Syncer::default(), Syncer::default());
    let (a, b) = (lab.root("A"), lab.root("B"));
    write(&a, "n/zmena.md", b"puvodni\n");
    write(&a, "n/smaz-vs-zmena.md", b"puvodni\n");
    write(&a, "n/zmena-vs-smaz.md", b"puvodni\n");
    write(&a, "n/obrazek.png", b"\x89PNG puvodni");
    ok(lab.sync(&mut sa, "A"));
    ok(lab.sync(&mut sb, "B"));
    // A: změna, změna, smazání, nová, obrázek
    write(&a, "n/zmena.md", b"puvodni\nA\n");
    write(&a, "n/smaz-vs-zmena.md", b"puvodni\nA\n");
    std::fs::remove_file(a.join("n/zmena-vs-smaz.md")).unwrap();
    write(&a, "n/nova.md", b"nova A\n");
    write(&a, "n/obrazek.png", b"\x89PNG A");
    ok(lab.sync(&mut sa, "A"));
    // B: jiná změna, smazání, změna, nová se stejným názvem, jiný obrázek
    write(&b, "n/zmena.md", b"puvodni\nB\n");
    std::fs::remove_file(b.join("n/smaz-vs-zmena.md")).unwrap();
    write(&b, "n/zmena-vs-smaz.md", b"puvodni\nB\n");
    write(&b, "n/nova.md", b"nova B\n");
    write(&b, "n/obrazek.png", b"\x89PNG B");
    let (pulled, pushed, _) = ok(lab.sync(&mut sb, "B"));
    assert!(pulled && pushed);
    let day = "(konflikt Ledge 2026-10-16)";
    // obě změnily → remote na místě, lokální v kopii
    assert_eq!(read(&b, "n/zmena.md"), "puvodni\nA\n");
    assert_eq!(read(&b, &format!("n/zmena {day}.md")), "puvodni\nB\n");
    // smazáno lokálně vs. změněno na remote → zůstává změna
    assert_eq!(read(&b, "n/smaz-vs-zmena.md"), "puvodni\nA\n");
    // změněno lokálně vs. smazáno na remote → zůstává změna
    assert_eq!(read(&b, "n/zmena-vs-smaz.md"), "puvodni\nB\n");
    // nová na obou se stejným názvem
    assert_eq!(read(&b, "n/nova.md"), "nova A\n");
    assert_eq!(read(&b, &format!("n/nova {day}.md")), "nova B\n");
    // binární soubor: bajty obou verzí
    assert_eq!(std::fs::read(b.join("n/obrazek.png")).unwrap(), b"\x89PNG A");
    assert_eq!(std::fs::read(b.join(format!("n/obrazek {day}.png"))).unwrap(), b"\x89PNG B");
    assert!(sb.notes.len() == 3, "kopie jen u 3 souborů s oběma verzemi: {:?}", sb.notes);
    // A po dalším syncu = totéž co B (konvergence), nic nezůstalo ve slučování
    ok(lab.sync(&mut sa, "A"));
    let (ta, tb) = (tracked(&a), tracked(&b));
    assert_eq!(ta.iter().collect::<std::collections::BTreeSet<_>>(), tb.iter().collect::<std::collections::BTreeSet<_>>());
    for f in &ta {
        assert_eq!(std::fs::read(a.join(f)).unwrap(), std::fs::read(b.join(f)).unwrap(), "{f}");
    }
    assert!(ta.iter().all(|f| !f.ends_with(".md") || !read(&a, f).contains("<<<<<<<")), "žádné konfliktní značky");
}

#[test]
fn second_pc_with_local_notes_merges_and_keeps_backup() {
    use notify::{RecursiveMode, Watcher};
    let lab = lab();
    let a = lab.root("A");
    write(&a, "_obecne/spolecna.md", b"stejna\n");
    write(&a, "_obecne/kolize.md", b"verze A\n![](./kolize-1.png)\n");
    write(&a, "_obecne/kolize-1.png", b"obrazek A");
    ok(lab.sync(&mut Syncer::default(), "A"));

    let b = lab.root("B");
    write(&b, "_obecne/spolecna.md", b"stejna\n");
    write(&b, "_obecne/kolize.md", b"verze B\n![](./kolize-1.png)\n");
    write(&b, "_obecne/kolize-1.png", b"obrazek B");
    write(&b, "atlas/jen-b.md", b"jen B\n");
    write(&b, ".draft/draft.json", b"{\"text\":\"rozepsane\"}");
    write(&b, ".state.json", b"[\"x\"]");
    // Ledge kořen sleduje watcherem — prohození složek s ním musí projít
    let mut w = notify::recommended_watcher(|_res: notify::Result<notify::Event>| {}).unwrap();
    w.watch(&b, RecursiveMode::Recursive).unwrap();

    let (_, _, swapped) = ok(lab.sync(&mut Syncer::default(), "B"));
    let backup = swapped.expect("prohozené složky");
    drop(w);
    assert!(backup.file_name().unwrap().to_string_lossy().starts_with("notes.pred-sync-"));
    // záloha zůstala celá
    assert_eq!(read(&backup, "_obecne/kolize.md"), "verze B\n![](./kolize-1.png)\n");
    // nový kořen: remote verze, lokální kolize přejmenovaná i s obrázkem a odkazem
    assert_eq!(read(&b, "_obecne/kolize.md"), "verze A\n![](./kolize-1.png)\n");
    assert_eq!(read(&b, "_obecne/kolize-B.md"), "verze B\n![](./kolize-B-1.png)\n");
    assert_eq!(read(&b, "_obecne/kolize-B-1.png"), "obrazek B");
    assert_eq!(read(&b, "_obecne/kolize-1.png"), "obrazek A");
    assert_eq!(read(&b, "atlas/jen-b.md"), "jen B\n");
    assert!(!b.join("_obecne/spolecna-B.md").exists(), "stejný obsah se nezdvojí");
    assert_eq!(read(&b, ".draft/draft.json"), "{\"text\":\"rozepsane\"}");
    assert_eq!(read(&b, ".state.json"), "[\"x\"]");
    let t = tracked(&b);
    assert!(!t.iter().any(|f| f.starts_with(".draft") || f == ".state.json"), "{t:?}");
    // žádný dočasný klon nezůstal
    let leftovers: Vec<_> = std::fs::read_dir(b.parent().unwrap()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("sync-clone")).collect();
    assert!(leftovers.is_empty());
    // a A dostane sloučené poznámky z B
    ok(lab.sync(&mut Syncer::default(), "A"));
    assert_eq!(read(&a, "_obecne/kolize-B.md"), "verze B\n![](./kolize-B-1.png)\n");
    assert_eq!(read(&a, "atlas/jen-b.md"), "jen B\n");
}

#[test]
fn refuses_root_inside_other_repo_and_unknown_remote() {
    let lab = lab();
    let outer = lab.dir.path().join("outer");
    std::fs::create_dir_all(&outer).unwrap();
    gitinfo::run(&outer, &["init", "-q"], Instant::now() + Duration::from_secs(10)).unwrap();
    let root = outer.join("Ledge").join("notes");
    write(&root, "_obecne/x.md", b"x\n");
    let o = Syncer::default().sync(&Ctx { root: &root, remote: &lab.remote, pc: "A", now: now() });
    assert!(matches!(&o, Outcome::Chyba(m) if m.contains("jiného git repa")), "{o:?}");
    assert!(!root.join(".git").exists());
    // remote, který neexistuje → chyba, kořen se nezaloží jako repo
    let root = lab.root("C");
    write(&root, "_obecne/x.md", b"x\n");
    let bad = lab.dir.path().join("neni.git").to_string_lossy().replace('\\', "/");
    let o = Syncer::default().sync(&Ctx { root: &root, remote: &bad, pc: "C", now: now() });
    assert!(matches!(o, Outcome::Chyba(_)), "{o:?}");
    assert!(!root.join(".git").exists());
}

#[test]
fn messages_follow_language() {
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;
    let ok = Outcome::Ok { pulled: true, pushed: true, swapped: None };
    assert_eq!(outcome_text(&ok).unwrap(), "Synchronizováno: staženo i odesláno.");
    with_lang(Lang::En, || {
        assert_eq!(outcome_text(&ok).unwrap(), "Synced: pulled and pushed.");
        assert_eq!(outcome_text(&Outcome::Konflikt { files: vec!["a.md".into()], message: String::new() }).unwrap_err(), "Notes diverged in: a.md. Open the notes folder and pick a version.");
        let Outcome::Neprihlaseno(m) = classify("fatal: Authentication failed for 'https://x'") else { panic!("auth") };
        assert!(m.ends_with("run `gh auth setup-git` (once on each PC)"), "{m}");
    });
}

#[test]
fn note_with_conflict_markers_is_never_committed_or_pushed() {
    let lab = lab();
    let (mut sa, mut sb) = (Syncer::default(), Syncer::default());
    let a = lab.root("A");
    write(&a, "_obecne/a.md", b"puvodni\n");
    ok(lab.sync(&mut sa, "A"));
    let head = |r: &Path| gitinfo::run(r, &["rev-parse", "HEAD"], Instant::now() + Duration::from_secs(10)).unwrap();
    let before = head(&a);
    // zbytek po neúspěšném slučování (např. pop autostash) — značky v poznámce
    write(&a, "_obecne/a.md", b"<<<<<<< Updated upstream\nA\n=======\nB\n>>>>>>> Stashed changes\n");
    match lab.sync(&mut sa, "A") {
        Outcome::Konflikt { files, .. } => assert_eq!(files, vec!["_obecne/a.md".to_string()]),
        o => panic!("čekal jsem konflikt, je {o:?}"),
    }
    assert_eq!(head(&a), before, "nic se necommitlo");
    assert!(read(&a, "_obecne/a.md").contains("<<<<<<<"), "soubor zůstal beze změny pro ruční opravu");
    // po opravě sync pokračuje a na druhý PC jde jen čistá verze
    write(&a, "_obecne/a.md", b"opraveno\n");
    ok(lab.sync(&mut sa, "A"));
    let b = lab.root("B");
    ok(lab.sync(&mut sb, "B"));
    assert_eq!(read(&b, "_obecne/a.md"), "opraveno\n");
}
