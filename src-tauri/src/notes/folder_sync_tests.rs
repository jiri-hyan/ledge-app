//! Testy propojení se složkou: čisté plánování tabulkově (`plan`), jména, a běhy nad dočasnými složkami
//! (`tempfile`) — nikdy skutečné poznámky ani `%LOCALAPPDATA%`.

use super::folder_sync::{LinkRun, State, broad_folders, check_folder, clear_conflicts, count_notes, event_relevant, is_broad, load_state, run, validate_link, warn_not_vault};
use super::folder_sync_scan::{Kind, Prev, is_link_tag, scan, under_frozen};
use super::folder_sync_plan::*;
use super::testkit::at;
use crate::config::{FolderLinkConfig, LinkDirection as Dir};
use std::path::{Path, PathBuf};

// ---------- čisté ----------

fn snap(rel: &str, hash: &str) -> Snap {
    Snap { rel: rel.into(), size: 1, mtime: 1, hash: hash.into(), title: Some(rel.trim_end_matches(".md").rsplit('/').next().unwrap().to_string()) }
}
fn entry(l: &str, e: &str, h: &str) -> Entry {
    Entry::new(l, e, h)
}
fn opts(dir: Dir, deletes: bool) -> Opts {
    Opts { dir, deletes, today: "2026-10-24".into() }
}
fn ops(p: &Planned) -> Vec<Op> {
    p.steps.iter().map(|s| s.op.clone()).filter(|o| *o != Op::Keep).collect()
}
fn kept(p: &Planned) -> Vec<Entry> {
    p.steps.iter().flat_map(|s| s.keep.clone()).collect()
}

#[test]
fn new_files_go_by_direction() {
    let ext = [snap("a/Note.md", "h1"), snap("b/Note.md", "h2")];
    let ledge = [snap("proj/2026-x.md", "h3")];
    // (směr, očekávané ops: import počet, export počet)
    for (dir, imports, exports) in [(Dir::Both, 2, 1), (Dir::Import, 2, 0), (Dir::Export, 0, 1)] {
        let p = plan(&[], &ledge, &ext, &opts(dir, false));
        let o = ops(&p);
        assert_eq!(o.iter().filter(|o| matches!(o, Op::Import { .. })).count(), imports, "{dir:?}");
        assert_eq!(o.iter().filter(|o| matches!(o, Op::Export { .. })).count(), exports, "{dir:?}");
    }
    let o = ops(&plan(&[], &[], &ext, &opts(Dir::Both, false)));
    // stejné jméno ze dvou podsložek → dvě různé poznámky
    assert!(matches!(&o[0], Op::Import { ledge, .. } if ledge == "_obecne/Note.md"), "{o:?}");
    assert!(matches!(&o[1], Op::Import { ledge, .. } if ledge == "_obecne/Note (2).md"), "{o:?}");
    // export: titulek jako jméno, složka projektu zůstane, _obecne = kořen složky
    let o = ops(&plan(&[], &[snap("proj/2026-x.md", "h"), snap("_obecne/y.md", "i")], &[], &opts(Dir::Both, false)));
    assert!(matches!(&o[0], Op::Export { ext, .. } if ext == "y.md"), "{o:?}");
    assert!(matches!(&o[1], Op::Export { ext, .. } if ext == "proj/2026-x.md"), "{o:?}");
}

#[test]
fn one_sided_changes_follow_direction() {
    let e = entry("_obecne/a.md", "a.md", "old");
    let same = snap("_obecne/a.md", "old");
    let xsame = snap("a.md", "old");
    // (směr, Ledge změněn, složka změněna) → očekávaný typ
    let cases = [
        (Dir::Both, true, false, "export"),
        (Dir::Both, false, true, "import"),
        (Dir::Export, true, false, "export"),
        (Dir::Export, false, true, "nic"),
        (Dir::Import, false, true, "import"),
        (Dir::Import, true, false, "nic"),
    ];
    for (dir, lch, ech, want) in cases {
        let l = if lch { snap("_obecne/a.md", "new") } else { same.clone() };
        let x = if ech { snap("a.md", "new") } else { xsame.clone() };
        let p = plan(std::slice::from_ref(&e), &[l], &[x], &opts(dir, false));
        let got = match ops(&p).first() {
            Some(Op::Export { .. }) => "export",
            Some(Op::Import { .. }) => "import",
            None => "nic",
            Some(o) => panic!("{o:?}"),
        };
        assert_eq!(got, want, "{dir:?} l={lch} e={ech}");
        // nepřenesená změna nesmí posunout hash záznamu (jinak by se ztratila)
        if want == "nic" {
            assert_eq!(kept(&p)[0].hash, "old");
        }
    }
    // stejná úprava na obou stranách = nová dohoda bez zápisu
    let p = plan(&[e], &[snap("_obecne/a.md", "n")], &[snap("a.md", "n")], &opts(Dir::Both, false));
    assert!(ops(&p).is_empty());
    assert_eq!(kept(&p)[0].hash, "n");
}

#[test]
fn conflict_keeps_both_versions() {
    let e = entry("p/a.md", "a.md", "old");
    let run = |dir| plan(std::slice::from_ref(&e), &[snap("p/a.md", "L")], &[snap("a.md", "X")], &opts(dir, false));
    let p = run(Dir::Both);
    let Op::Conflict { winner, copy_ledge, copy_ext, .. } = &ops(&p)[0] else { panic!() };
    assert_eq!((*winner, copy_ledge.as_deref(), copy_ext.as_deref()), (Side::Ext, Some("p/a (konflikt Ledge 2026-10-24).md"), Some("a (konflikt Ledge 2026-10-24).md")));
    // obě kopie jsou spárované (verze Ledge), původní dvojice drží verzi složky
    let k = kept(&p);
    assert_eq!((k[0].hash.as_str(), k[1].hash.as_str()), ("X", "L"));
    let Op::Conflict { winner, copy_ledge, copy_ext, .. } = &ops(&run(Dir::Import))[0] else { panic!() };
    assert_eq!((*winner, copy_ledge.is_some(), copy_ext.is_some()), (Side::Ext, true, false), "import nezapisuje do složky");
    let Op::Conflict { winner, copy_ledge, copy_ext, .. } = &ops(&run(Dir::Export))[0] else { panic!() };
    assert_eq!((*winner, copy_ledge.is_some(), copy_ext.is_some()), (Side::Ledge, false, true), "export: zdrojem je Ledge, verze složky se zachrání");
}

#[test]
fn deletes_are_not_propagated_by_default() {
    let e = entry("_obecne/a.md", "a.md", "h");
    let (l, x) = (snap("_obecne/a.md", "h"), snap("a.md", "h"));
    // (směr, deletes, kdo chybí) → (ops, náhrobek)
    let cases: [(Dir, bool, bool, &str, Gone); 7] = [
        (Dir::Both, false, true, "nic", Gone::Ext),
        (Dir::Both, true, true, "smazat Ledge", Gone::None),
        (Dir::Import, true, true, "smazat Ledge", Gone::None),
        (Dir::Export, true, true, "nic", Gone::Ext),
        (Dir::Both, false, false, "nic", Gone::Ledge),
        (Dir::Both, true, false, "smazat složku", Gone::None),
        (Dir::Import, true, false, "nic", Gone::Ledge),
    ];
    for (dir, deletes, ext_missing, want, gone) in cases {
        // tři nedotčené dvojice navíc: jediné smazání ze čtyř není hromadné (100 % záznamů se zastavuje vždy)
        let mut entries = vec![e.clone()];
        let (mut ledge, mut ext) = (Vec::new(), Vec::new());
        for i in 0..3 {
            entries.push(entry(&format!("_obecne/o{i}.md"), &format!("o{i}.md"), "h"));
            ledge.push(snap(&format!("_obecne/o{i}.md"), "h"));
            ext.push(snap(&format!("o{i}.md"), "h"));
        }
        if ext_missing { ledge.insert(0, l.clone()) } else { ext.insert(0, x.clone()) }
        let p = plan(&entries, &ledge, &ext, &opts(dir, deletes));
        let got = match ops(&p).first() {
            None => "nic",
            Some(Op::DeleteLedge { .. }) => "smazat Ledge",
            Some(Op::DeleteExt { .. }) => "smazat složku",
            Some(o) => panic!("{o:?}"),
        };
        assert_eq!(got, want, "{dir:?} deletes={deletes} ext_missing={ext_missing}");
        if got == "nic" {
            assert_eq!(kept(&p)[0].gone, gone);
        }
    }
    // náhrobek: dál se nic neděje, dokud se zbylá strana nezmění; pak se vrátí (nic se neztratí)
    let tomb = Entry { gone: Gone::Ext, ..e.clone() };
    assert!(ops(&plan(std::slice::from_ref(&tomb), std::slice::from_ref(&l), &[], &opts(Dir::Both, true))).is_empty());
    let o = ops(&plan(&[tomb], &[snap("_obecne/a.md", "edited")], &[], &opts(Dir::Both, false)));
    assert!(matches!(&o[0], Op::Export { ext, over: None, .. } if ext == "a.md"), "{o:?}");
    let tomb = Entry { gone: Gone::Ledge, ..e.clone() };
    let o = ops(&plan(&[tomb], &[], &[snap("a.md", "edited")], &opts(Dir::Both, false)));
    assert!(matches!(&o[0], Op::Import { ledge, over: None, .. } if ledge == "_obecne/a.md"), "{o:?}");
    // změněná strana smazání nepřebije: po smazání ve složce se Ledge upravilo → vrátí se do složky
    let o = ops(&plan(&[e], &[snap("_obecne/a.md", "edited")], &[], &opts(Dir::Both, true)));
    assert!(matches!(&o[0], Op::Export { over: None, .. }), "{o:?}");
}

#[test]
fn mass_deletes_are_held_back() {
    let entries: Vec<Entry> = (0..12).map(|i| entry(&format!("_obecne/{i}.md"), &format!("{i}.md"), "h")).collect();
    let ledge: Vec<Snap> = (0..12).map(|i| snap(&format!("_obecne/{i}.md"), "h")).collect();
    let p = plan(&entries, &ledge, &[], &opts(Dir::Both, true));
    assert_eq!(p.held_deletes, 12);
    assert!(ops(&p).is_empty() && kept(&p).iter().all(|e| e.gone == Gone::Ext));
    // pár smazání z mnoha je v pořádku
    let entries: Vec<Entry> = (0..40).map(|i| entry(&format!("_obecne/{i}.md"), &format!("{i}.md"), "h")).collect();
    let ledge: Vec<Snap> = (0..40).map(|i| snap(&format!("_obecne/{i}.md"), "h")).collect();
    let ext: Vec<Snap> = (3..40).map(|i| snap(&format!("{i}.md"), "h")).collect();
    let p = plan(&entries, &ledge, &ext, &opts(Dir::Both, true));
    assert_eq!((p.held_deletes, ops(&p).len()), (0, 3));
}

#[test]
fn rename_and_identical_files_do_not_duplicate() {
    // přejmenováno ve složce: stejný obsah, jiné jméno
    let p = plan(&[entry("_obecne/a.md", "a.md", "h")], &[snap("_obecne/a.md", "h")], &[snap("novy.md", "h")], &opts(Dir::Both, true));
    assert!(ops(&p).is_empty(), "{:?}", ops(&p));
    assert_eq!(kept(&p)[0].ext, "novy.md");
    // ztracený stav: shodné soubory na obou stranách se spárují, rozdílné se vymění
    let p = plan(&[], &[snap("_obecne/a.md", "h"), snap("_obecne/b.md", "other")], &[snap("a.md", "h"), snap("c.md", "h2")], &opts(Dir::Both, false));
    let o = ops(&p);
    assert_eq!(o.len(), 2, "{o:?}");
    assert!(kept(&p).iter().any(|e| e.ledge == "_obecne/a.md" && e.ext == "a.md"));
}

#[test]
fn names_are_windows_and_obsidian_safe() {
    for (raw, want) in [
        ("Zavolat Řehořovi", "Zavolat Řehořovi"),
        ("a/b\\c:d*e?f\"g<h>i|j", "a b c d e f g h i j"),
        ("Plán [A] #1 ^x", "Plán A 1 x"),
        ("  ..tečky..  ", "tečky"),
        ("", "Poznámka"),
        ("???", "Poznámka"),
        ("CON", "CON_"),
        ("nul.txt", "nul.txt_"),
        ("COM3", "COM3_"),
        ("COM10", "COM10"),
        ("COM¹", "COM¹_"),
        ("lpt²", "lpt²_"),
        ("LPT³.txt", "LPT³.txt_"),
        ("CONIN$", "CONIN$_"),
        ("conout$.md", "conout$.md_"),
        ("COM¹⁰", "COM¹⁰"),
        ("CONSOLE", "CONSOLE"),
        ("řádek\n\tdva", "řádek dva"),
    ] {
        assert_eq!(sanitize_name(raw), want, "{raw:?}");
    }
    assert_eq!(sanitize_name(&"x".repeat(300)).chars().count(), 100);
    for ok in ["a.md", "dir/a.md", "d1/d2/ž.md"] {
        assert!(rel_ok(ok), "{ok}");
    }
    for bad in ["", "/a.md", "../a.md", "a/../b.md", "a//b.md", "C:/x.md", "a\\b.md", "./a.md", "a/.", "x.md ", "a/b.", "a|b.md"] {
        assert!(!rel_ok(bad), "{bad}");
    }
    let mut taken = std::collections::HashSet::new();
    assert_eq!(unique(&mut taken, "d", "N"), "d/N.md");
    assert_eq!(unique(&mut taken, "d", "n"), "d/n (2).md", "velikost písmen se na Windows neliší");
    assert_eq!(unique(&mut taken, "", "N"), "N.md");
}

// ---------- běhy nad dočasnými složkami ----------

struct Fx {
    _td: tempfile::TempDir,
    root: PathBuf,
    folder: PathBuf,
    state: PathBuf,
}

fn fx() -> Fx {
    let td = tempfile::Builder::new().prefix("ledge-folder-").tempdir().unwrap();
    let (root, folder, state) = (td.path().join("notes"), td.path().join("vault"), td.path().join("state").join("link.json"));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&folder).unwrap();
    Fx { _td: td, root, folder, state }
}

fn put(p: &Path, text: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}
fn get(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}

impl Fx {
    fn go(&self, dir: Dir, deletes: bool) -> super::folder_sync::Outcome {
        run(&LinkRun { root: self.root.clone(), folder: self.folder.clone(), dir, deletes }, &self.state, at("2026-10-24T10:00:00+02:00")).unwrap()
    }
}

const OBS: &str = "---\ncolor: blue\ntags: [prace]\naliases: [Obs]\ncssclasses: wide\n---\r\n\nZávěr s [[Jiná poznámka|popisek]] a #štítek\n";

#[test]
fn import_is_bytewise_and_skips_hidden_and_foreign() {
    let f = fx();
    put(&f.folder.join("Obsidian.md"), OBS);
    put(&f.folder.join("sub/Deep.markdown"), "# Hluboko\n");
    for hidden in [".obsidian/workspace.md", ".trash/old.md", "logseq/config.md", "sub/.skryta.md", "node_modules/x.md"] {
        put(&f.folder.join(hidden), "tajné\n");
    }
    put(&f.folder.join("obrazek.png"), "png");
    put(&f.folder.join("velka.md"), &"x".repeat(1_100_000));
    let o = f.go(Dir::Import, false);
    assert_eq!(o.report.imported, 2, "{}", o.message);
    // bajtově shodné: frontmatter s cizími klíči, CRLF i [[vazby]] zůstaly
    assert_eq!(std::fs::read(f.root.join("_obecne/Obsidian.md")).unwrap(), OBS.as_bytes());
    assert_eq!(get(&f.root.join("_obecne/Deep.md")), "# Hluboko\n");
    let notes = super::scan(&f.root);
    assert_eq!(notes.len(), 2);
    assert!(notes.iter().any(|n| n.color.as_deref() == Some("blue") && n.tags.contains(&"prace".to_string())));
    assert!(o.message.contains("Přeskočeno 1 velkých") || o.message.contains("Skipped 1 large"), "{}", o.message);
    // složka se v režimu „jen import“ nezměnila
    assert!(!f.folder.join("_obecne").exists() && get(&f.folder.join("Obsidian.md")) == OBS);
    // druhý běh: beze změn
    let again = f.go(Dir::Import, false);
    assert_eq!(again.report, Default::default(), "{}", again.message);
}

#[test]
fn export_writes_title_named_md_with_images_and_project_dirs() {
    let f = fx();
    let now = at("2026-10-24T10:00:00+02:00");
    let png = super::testkit::b64(&super::testkit::png(3));
    let general = NewNoteArgs { text: "Zavolat Řehořovi: faktury [[Plán]]", project: None, images: vec![png] };
    let p1 = general.create(&f.root, &now);
    let p2 = NewNoteArgs { text: "Nápad k atlasu", project: Some("atlas"), images: vec![] }.create(&f.root, &now);
    let o = f.go(Dir::Export, false);
    assert_eq!(o.report.exported, 2, "{}", o.message);
    let exported = f.folder.join("Zavolat Řehořovi faktury Plán.md");
    assert_eq!(std::fs::read(&exported).unwrap(), std::fs::read(&p1).unwrap(), "bajtově stejný soubor vč. frontmatteru");
    assert_eq!(std::fs::read(f.folder.join("atlas/Nápad k atlasu.md")).unwrap(), std::fs::read(&p2).unwrap());
    // obrázek vedle poznámky, ať odkaz ve složce nezlomí
    let pngs: Vec<_> = std::fs::read_dir(&f.folder).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "png")).collect();
    assert_eq!(pngs.len(), 1);
    // import zpět se neděje (jen export) a druhý běh je prázdný
    assert_eq!(f.go(Dir::Export, false).report, Default::default());
    assert!(!f.root.join("_obecne/Zavolat Řehořovi faktury Plán.md").exists());
}

struct NewNoteArgs<'a> {
    text: &'a str,
    project: Option<&'a str>,
    images: Vec<String>,
}
impl NewNoteArgs<'_> {
    fn create(self, root: &Path, now: &chrono::DateTime<chrono::FixedOffset>) -> PathBuf {
        super::create(root, &super::NewNote { text: self.text.into(), project: self.project.map(str::to_string), images: self.images, ..Default::default() }, now).unwrap()
    }
}

#[test]
fn both_directions_edit_flow_and_conflict() {
    let f = fx();
    put(&f.folder.join("A.md"), "verze 1\n");
    f.go(Dir::Both, false);
    let ledge_a = f.root.join("_obecne/A.md");
    assert_eq!(get(&ledge_a), "verze 1\n");
    // úprava ve složce → Ledge
    put(&f.folder.join("A.md"), "verze 2 z Obsidianu\n");
    assert_eq!(f.go(Dir::Both, false).report.updated_ledge, 1);
    assert_eq!(get(&ledge_a), "verze 2 z Obsidianu\n");
    // úprava v Ledge → složka
    put(&ledge_a, "verze 3 z Ledge\n");
    assert_eq!(f.go(Dir::Both, false).report.updated_folder, 1);
    assert_eq!(get(&f.folder.join("A.md")), "verze 3 z Ledge\n");
    // obě strany změněné → nic se nepřepíše potichu
    put(&ledge_a, "Ledge A\n");
    put(&f.folder.join("A.md"), "Obsidian A\n");
    let o = f.go(Dir::Both, false);
    assert_eq!(o.report.conflicts.len(), 1, "{}", o.message);
    assert_eq!(get(&f.folder.join("A.md")), "Obsidian A\n", "složka zůstala");
    assert_eq!(get(&ledge_a), "Obsidian A\n", "Ledge dostal verzi složky");
    assert_eq!(get(&f.root.join("_obecne/A (konflikt Ledge 2026-10-24).md")), "Ledge A\n", "verze Ledge je uložená");
    assert_eq!(get(&f.folder.join("A (konflikt Ledge 2026-10-24).md")), "Ledge A\n", "i ve složce");
    assert!(o.message.contains("konflikt"), "{}", o.message);
    // stav pamatuje konflikt k zobrazení; „Rozumím“ ho smaže
    assert_eq!(load_state(&f.state).conflicts.len(), 1);
    // další běh nic nepřidá (kopie jsou spárované, žádné nové konflikty)
    let calm = f.go(Dir::Both, false);
    assert_eq!((calm.report.conflicts.len(), calm.report.imported, calm.report.exported), (0, 0, 0), "{}", calm.message);
    assert_eq!(calm.conflicts.len(), 1);
    clear_conflicts(&f.state).unwrap();
    assert!(load_state(&f.state).conflicts.is_empty());
}

#[test]
fn deletes_default_off_then_on() {
    let f = fx();
    put(&f.folder.join("Smazat.md"), "text\n");
    put(&f.folder.join("Zustane.md"), "text 2\n");
    f.go(Dir::Both, false);
    // smazáno ve složce, volba vypnutá → Ledge poznámka zůstane a nevrací se zpět
    std::fs::remove_file(f.folder.join("Smazat.md")).unwrap();
    let o = f.go(Dir::Both, false);
    assert_eq!(o.report.deleted_ledge, 0);
    assert!(f.root.join("_obecne/Smazat.md").exists());
    assert!(!f.folder.join("Smazat.md").exists(), "nevrací se do složky");
    // smazáno v Ledge, volba vypnutá → soubor ve složce zůstane a nevrací se do Ledge
    std::fs::remove_file(f.root.join("_obecne/Zustane.md")).unwrap();
    f.go(Dir::Both, false);
    assert!(f.folder.join("Zustane.md").exists() && !f.root.join("_obecne/Zustane.md").exists());
    // volba zapnutá: nové smazání se přenese (Ledge → Archiv, složka → .trash)
    put(&f.folder.join("C.md"), "c\n");
    put(&f.folder.join("D.md"), "d\n");
    f.go(Dir::Both, true);
    std::fs::remove_file(f.folder.join("C.md")).unwrap();
    std::fs::remove_file(f.root.join("_obecne/D.md")).unwrap();
    let o = f.go(Dir::Both, true);
    assert_eq!((o.report.deleted_ledge, o.report.deleted_folder), (1, 1), "{}", o.message);
    assert!(f.root.join("_obecne/_archiv/C.md").exists() && !f.root.join("_obecne/C.md").exists());
    assert_eq!(get(&f.folder.join(".trash/D.md")), "d\n");
    assert!(!f.folder.join("D.md").exists());
}

#[test]
fn lost_state_pairs_by_content_no_duplicates() {
    let f = fx();
    put(&f.folder.join("A.md"), "a\n");
    put(&f.folder.join("B.md"), "b\n");
    f.go(Dir::Both, false);
    std::fs::remove_file(&f.state).unwrap();
    let o = f.go(Dir::Both, false);
    assert_eq!((o.report.imported, o.report.exported, o.report.conflicts.len()), (0, 0, 0), "{}", o.message);
    assert_eq!(std::fs::read_dir(f.root.join("_obecne")).unwrap().count(), 2);
    // změna složky v configu = čistý začátek, ne smazání
    let other = f.folder.parent().unwrap().join("vault2");
    put(&other.join("Jina.md"), "j\n");
    let o = run(&LinkRun { root: f.root.clone(), folder: other, dir: Dir::Both, deletes: true }, &f.state, at("2026-10-24T10:00:00+02:00")).unwrap();
    assert_eq!(o.report.deleted_ledge + o.report.deleted_folder, 0);
    assert!(f.root.join("_obecne/A.md").exists());
}

#[test]
fn unavailable_folder_never_looks_like_mass_delete() {
    let f = fx();
    for i in 0..15 {
        put(&f.folder.join(format!("N{i}.md")), "x\n");
    }
    f.go(Dir::Both, true);
    std::fs::remove_dir_all(&f.folder).unwrap();
    let err = run(&LinkRun { root: f.root.clone(), folder: f.folder.clone(), dir: Dir::Both, deletes: true }, &f.state, at("2026-10-24T10:00:00+02:00")).unwrap_err();
    assert!(err.contains("neexistuje") || err.contains("doesn't exist"), "{err}");
    assert_eq!(std::fs::read_dir(f.root.join("_obecne")).unwrap().count(), 15, "Ledge beze změny");
}

#[test]
fn folder_must_not_overlap_notes_or_be_a_drive_root() {
    let f = fx();
    let inside = f.root.join("_obecne");
    std::fs::create_dir_all(&inside).unwrap();
    for bad in [f.root.to_string_lossy().to_string(), inside.to_string_lossy().to_string(), f.root.parent().unwrap().to_string_lossy().to_string(), "relativni".to_string(), "C:\\".to_string(), f.folder.join("nene").to_string_lossy().to_string(), format!("{}\\..\\vault", f.folder.display())] {
        assert!(check_folder(&bad, &f.root).is_err(), "{bad}");
    }
    assert!(check_folder(&f.folder.to_string_lossy(), &f.root).is_ok());
}

#[cfg(windows)]
#[test]
fn junctions_are_never_followed() {
    use std::os::windows::process::CommandExt;
    let f = fx();
    let outside = f.folder.parent().unwrap().join("outside");
    put(&outside.join("tajne.md"), "mimo složku\n");
    let junction = |link: &Path, target: &Path| {
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(link).arg(target).creation_flags(0x0800_0000).output().is_ok_and(|o| o.status.success());
        assert!(ok, "mklink /J");
    };
    junction(&f.folder.join("odkaz"), &outside);
    put(&f.folder.join("Ok.md"), "ok\n");
    // import: obsah za junctionem se nečte
    let o = f.go(Dir::Both, true);
    assert_eq!(o.report.imported, 1, "{}", o.message);
    assert!(!f.root.join("_obecne/tajne.md").exists());
    // export do složky projektu, který je ve složce junction → nezapíše se ven
    let before = std::fs::read_dir(&outside).unwrap().count();
    NewNoteArgs { text: "Z projektu", project: Some("odkaz"), images: vec![] }.create(&f.root, &at("2026-10-24T10:00:00+02:00"));
    let o = f.go(Dir::Both, true);
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), before, "{}", o.message);
    assert_eq!(get(&outside.join("tajne.md")), "mimo složku\n");
    // junction uvnitř kořene poznámek (složka projektu → ven) se taky nesleduje
    junction(&f.root.join("ven"), &outside);
    f.go(Dir::Both, true);
    assert!(!f.folder.join("tajne.md").exists() && !f.folder.join("ven").exists());
    // junction se maže jako odkaz, nikdy ne jeho cíl
    for l in [f.folder.join("odkaz"), f.root.join("ven")] {
        std::fs::remove_dir(l).unwrap();
    }
    assert_eq!(get(&outside.join("tajne.md")), "mimo složku
");
}

#[test]
fn config_validation_and_old_configs() {
    let ok = |enabled: bool, path: &str| validate_link(&FolderLinkConfig { enabled, path: path.into(), ..Default::default() });
    assert!(ok(false, "").is_ok());
    assert!(ok(true, "").is_err());
    assert!(ok(true, "relativni\\cesta").is_err());
    assert!(ok(true, "C:\\a\\..\\b").is_err());
    assert!(ok(true, "C:\\Vault").is_ok());
    // starý config bez notes.folderLink = vypnuto, výchozí směr oba, mazání nepřenášet
    let cfg: crate::config::Config = serde_json::from_str(r#"{"notes":{"enabled":true,"sync":{"enabled":false}}}"#).unwrap();
    assert_eq!(cfg.notes.folder_link, FolderLinkConfig::default());
    assert!(!cfg.notes.folder_link.enabled && !cfg.notes.folder_link.propagate_deletes && cfg.notes.folder_link.direction == Dir::Both);
    let cfg: crate::config::Config = serde_json::from_str(r#"{"notes":{"folderLink":{"enabled":true,"path":"D:\\V","direction":"import"}}}"#).unwrap();
    assert_eq!((cfg.notes.folder_link.direction, cfg.notes.folder_link.propagate_deletes), (Dir::Import, false));
    assert!(crate::config::schema_json().contains("folderLink"));
    let s: State = serde_json::from_str("{rozbité").unwrap_or_default();
    assert!(s.entries.is_empty());
}

#[test]
fn watcher_events_ignore_hidden_and_non_notes() {
    let base = PathBuf::from(r"C:\Vault");
    let p = |s: &str| vec![PathBuf::from(format!(r"C:\Vault\{s}"))];
    for yes in ["A.md", "sub\\B.markdown", "sub", "A.MD"] {
        assert!(event_relevant(&base, &p(yes)), "{yes}");
    }
    for no in [".obsidian\\workspace.md", ".trash\\x.md", "logseq\\bak\\x.md", "obrazek.png", "A.md.1-0.tmp", "sub\\.skryta.md"] {
        assert!(!event_relevant(&base, &p(no)), "{no}");
    }
    assert!(event_relevant(&base, &[]));
}
/// Hromadné mazání: (smazání, záznamů, zastavit?) — 100 % vždy, u malých počtů víc než polovina, u velkých čtvrtina.
#[test]
fn mass_delete_thresholds_table() {
    for (n, entries, held) in [(0, 0, false), (0, 5, false), (1, 1, true), (2, 2, true), (1, 5, false), (2, 5, false), (3, 5, true), (3, 6, false), (3, 7, false), (4, 7, true), (3, 40, false), (10, 40, false), (11, 40, true), (11, 44, false), (12, 12, true), (45, 100, true)] {
        assert_eq!(mass_delete(n, entries), held, "{n} z {entries}");
    }
    // i u malého vaultu: smazání všeho se nepřenese
    let p = plan(&[entry("_obecne/a.md", "a.md", "h")], &[snap("_obecne/a.md", "h")], &[], &opts(Dir::Both, true));
    assert_eq!((p.held_deletes, ops(&p).len()), (1, 0));
    assert!(kept(&p).iter().all(|e| e.gone == Gone::Ext));
    // jedno smazání z několika zůstává povolené
    let entries: Vec<Entry> = (0..5).map(|i| entry(&format!("_obecne/{i}.md"), &format!("{i}.md"), "h")).collect();
    let ledge: Vec<Snap> = (0..5).map(|i| snap(&format!("_obecne/{i}.md"), "h")).collect();
    let ext: Vec<Snap> = (1..5).map(|i| snap(&format!("{i}.md"), "h")).collect();
    let p = plan(&entries, &ledge, &ext, &opts(Dir::Both, true));
    assert_eq!((p.held_deletes, ops(&p).len()), (0, 1));
}

/// Příliš široké složky (z proměnných prostředí, vymyšlené hodnoty — nic se nečte) a jejich nadřazené.
#[test]
fn broad_folders_table() {
    let vars: std::collections::HashMap<&str, &str> = [
        ("USERPROFILE", r"C:\Users\uzivatel"),
        ("OneDrive", r"C:\Users\uzivatel\OneDrive - Firma"),
        ("APPDATA", r"C:\Users\uzivatel\AppData\Roaming"),
        ("LOCALAPPDATA", r"C:\Users\uzivatel\AppData\Local"),
        ("ProgramFiles", r"C:\Program Files"),
        ("ProgramFiles(x86)", r"C:\Program Files (x86)"),
        ("SystemRoot", r"C:\Windows"),
    ]
    .into();
    let known = broad_folders(&|n| vars.get(n).map(|v| v.to_string()));
    for (path, broad) in [
        (r"C:\Users\uzivatel", true),
        (r"C:\Users\UZIVATEL", true),
        (r"C:\Users\uzivatel\Documents", true),
        (r"C:\Users\uzivatel\Dokumenty", true),
        (r"C:\Users\uzivatel\Desktop", true),
        (r"C:\Users\uzivatel\Plocha", true),
        (r"C:\Users\uzivatel\Downloads", true),
        (r"C:\Users\uzivatel\Stažené", true),
        (r"C:\Users\uzivatel\OneDrive", true),
        (r"C:\Users\uzivatel\OneDrive - Firma", true),
        (r"C:\Users\uzivatel\AppData", true),
        (r"C:\Users\uzivatel\AppData\Roaming", true),
        (r"C:\Program Files", true),
        (r"C:\Windows", true),
        (r"C:\Users", true),
        (r"C:", true),
        (r"C:\Users\uzivatel\Documents\Vault", false),
        (r"C:\Users\uzivatel\OneDrive - Firma\Poznamky", false),
        (r"C:\Users\uzivatel\Obsidian", false),
        (r"D:\Vault", false),
        (r"C:\Users\jiny", false),
        (r"C:\Usersx", false),
    ] {
        assert_eq!(is_broad(&path.to_lowercase(), &known), broad, "{path}");
    }
    assert!(!is_broad(r"c:\users\uzivatel", &broad_folders(&|_| None)), "bez proměnných prostředí se nic nezakazuje");
}

#[test]
fn check_folder_refuses_profile_and_its_parent() {
    let f = fx();
    let profile = f.folder.parent().unwrap().join("uzivatel");
    std::fs::create_dir_all(&profile).unwrap();
    let env = profile.canonicalize().unwrap().to_string_lossy().into_owned();
    let get = |n: &str| (n == "USERPROFILE").then(|| env.clone());
    for bad in [profile.clone(), profile.parent().unwrap().to_path_buf()] {
        let err = super::folder_sync::check_folder_with(&bad.to_string_lossy(), &f.root, &get).unwrap_err();
        assert!(err.contains("moc široká") || err.contains("too broad"), "{}: {err}", bad.display());
    }
    assert!(check_folder(&f.folder.to_string_lossy(), &f.root).is_ok());
}

/// Varování před zapnutím: bez znaku vaultu a s > 200 `.md`.
#[test]
fn not_a_vault_warning_table() {
    for (sign, md, warn) in [(false, 0, false), (false, 200, false), (false, 201, true), (true, 5000, false), (false, 5000, true)] {
        assert_eq!(warn_not_vault(sign, md), warn, "znak {sign}, {md} souborů");
    }
    let f = fx();
    for i in 0..201 {
        put(&f.folder.join(format!("N{i}.md")), "x\n");
    }
    let c = count_notes(&f.folder.to_string_lossy(), &f.root).unwrap();
    assert_eq!(c.n, 201);
    assert!(c.warn.is_some_and(|w| w.contains("201")));
    std::fs::create_dir_all(f.folder.join(".obsidian")).unwrap();
    assert!(count_notes(&f.folder.to_string_lossy(), &f.root).unwrap().warn.is_none());
}

/// Jen symlink a junction jsou odkaz; cloudoví zástupci a ostatní reparse pointy jsou běžné soubory.
#[test]
fn only_symlink_and_mount_point_tags_are_links() {
    for (tag, link) in [(0xA000_000Cu32, true), (0xA000_0003, true), (0x9000_001A, false), (0x9000_501A, false), (0x8000_0017, false), (0x8000_0013, false), (0, false)] {
        assert_eq!(is_link_tag(tag), link, "{tag:#x}");
    }
}

#[test]
fn frozen_covers_the_subtree_case_insensitively() {
    let fr = vec!["Odkaz".to_string(), "a/b.md".to_string()];
    for (rel, yes) in [("odkaz", true), ("ODKAZ/x.md", true), ("odkaz/d/x.md", true), ("odkazy/x.md", false), ("a/b.md", true), ("a/b.md/c", true), ("a/c.md", false), ("", false)] {
        assert_eq!(under_frozen(&fr, rel), yes, "{rel}");
    }
}

/// Jméno mimo UTF-8 se nepřeskočí potichu: sken to započítá a běh nepřenese mazání.
#[cfg(windows)]
#[test]
fn non_utf8_names_are_counted_and_stop_deletes() {
    use std::os::windows::ffi::OsStringExt;
    let f = fx();
    for i in 0..4 {
        put(&f.folder.join(format!("N{i}.md")), "x\n");
    }
    f.go(Dir::Both, true);
    let odd = std::ffi::OsString::from_wide(&[b'x' as u16, 0xD800, b'.' as u16, b'm' as u16, b'd' as u16]);
    std::fs::write(f.folder.join(&odd), "x").unwrap();
    let s = scan(&f.folder.canonicalize().unwrap(), Kind::Ext, &Prev::new()).unwrap();
    assert_eq!((s.odd_names, s.files.len()), (1, 4));
    // soubory ve složce zmizí — s podezřelým jménem se to nepřenese do Ledge
    for i in 0..2 {
        std::fs::remove_file(f.folder.join(format!("N{i}.md"))).unwrap();
    }
    let o = f.go(Dir::Both, true);
    assert_eq!(o.report.deleted_ledge, 0, "{}", o.message);
    std::fs::remove_file(f.folder.join(&odd)).unwrap();
}

/// Dřív spárovaná složka, která je teď junction, se zmrazí — poznámky pod ní se nepovažují za smazané.
#[cfg(windows)]
#[test]
fn paired_folder_turned_junction_is_frozen_not_deleted() {
    use std::os::windows::process::CommandExt;
    let f = fx();
    let outside = f.folder.parent().unwrap().join("outside2");
    std::fs::create_dir_all(&outside).unwrap();
    for i in 0..5 {
        put(&f.folder.join(format!("Dalsi{i}.md")), "x\n");
    }
    put(&f.folder.join("sub/Pod.md"), "pod\n");
    f.go(Dir::Both, true);
    assert!(f.root.join("_obecne/Pod.md").exists());
    std::fs::remove_dir_all(f.folder.join("sub")).unwrap();
    let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(f.folder.join("sub")).arg(&outside).creation_flags(0x0800_0000).output().is_ok_and(|o| o.status.success());
    assert!(ok, "mklink /J");
    let s = scan(&f.folder.canonicalize().unwrap(), Kind::Ext, &Prev::new()).unwrap();
    assert_eq!((s.links, s.frozen.clone()), (1, vec!["sub".to_string()]));
    let o = f.go(Dir::Both, true);
    assert_eq!(o.report.deleted_ledge, 0, "{}", o.message);
    assert!(f.root.join("_obecne/Pod.md").exists(), "poznámka zůstala v Ledge");
    std::fs::remove_dir(f.folder.join("sub")).unwrap();
}


#[test]
fn custom_title_and_unknown_keys_survive_both_directions() {
    let f = fx();
    let text = "---\ntitle: \"Plán týdne\"\ncreated: 2026-10-24T09:00:00+02:00\ncssclasses: wide\ndone: false\n---\n\nPrvní řádek těla\n";
    let p = f.root.join("2026-10-24-090000-prvni-radek.md");
    put(&p, text);
    let o = f.go(Dir::Export, false);
    assert_eq!(o.report.exported, 1, "{}", o.message);
    // soubor ve složce se jmenuje podle vlastního nadpisu a je bajtově stejný (title i cizí klíč zůstaly)
    let exported = f.folder.join("Plán týdne.md");
    assert_eq!(get(&exported), text);
    // úprava ve složce se vrátí zpět s nadpisem i cizími klíči
    put(&exported, &text.replace("První řádek těla", "Upraveno ve složce"));
    let o = f.go(Dir::Both, false);
    assert!(o.message.is_empty() || !o.message.contains("chyba"), "{}", o.message);
    let back = get(&p);
    assert!(back.contains("title: \"Plán týdne\"") && back.contains("cssclasses: wide") && back.contains("Upraveno ve složce"), "{back}");
}
