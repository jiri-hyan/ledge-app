//! Testy skenu složky skriptů (jen tempdir), pohledu pro UI a skutečného spuštění (cmd /
//! PowerShell bez okna).

use super::*;
use std::fs;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, text).unwrap();
    p
}

#[test]
fn scan_finds_ledge_scripts_only() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    write(dir, "b-cisti.ps1", "# @ledge.title Vyčistit temp\n# @ledge.icon 🧹\nWrite-Output hotovo\n");
    write(dir, "a-zaloha.cmd", "@echo off\n:: @ledge.title Záloha\n:: @ledge.mode terminal\necho x\n");
    write(dir, "helper.py", "def f():\n    return 1\n"); // bez hlavičky = pomocný modul
    write(dir, "spatny.js", "// @ledge.mode loud\n"); // hlavička bez titulku = problém
    write(dir, "readme.txt", "# @ledge.title Ne\n"); // nepovolená přípona
    write(dir, ".skryty.ps1", "# @ledge.title Ne\n");
    fs::create_dir(dir.join("sub.ps1")).unwrap(); // složka se jménem skriptu
    let scan = scan_dir(dir, None);
    assert!(scan.exists && scan.error.is_none());
    let names: Vec<&str> = scan.scripts.iter().map(|s| s.file.as_str()).collect();
    assert_eq!(names, ["a-zaloha.cmd", "b-cisti.ps1", "spatny.js"], "podle jména, bez pomocných a skrytých");
    let ps = &scan.scripts[1];
    assert_eq!(ps.id, "script:b-cisti.ps1");
    assert_eq!(ps.lang, Lang::Powershell);
    assert_eq!(ps.meta.as_ref().unwrap().title, "Vyčistit temp");
    assert_eq!(ps.sha.len(), 64);
    assert!(scan.scripts[2].meta.as_ref().unwrap_err().contains("@ledge.title"));
}

#[test]
fn missing_dir_is_empty_not_error() {
    let d = tempfile::tempdir().unwrap();
    let scan = scan_dir(&d.path().join("neni"), None);
    assert!(!scan.exists && scan.scripts.is_empty() && scan.error.is_none());
}

/// Nezměněný soubor (stejný mtime + velikost) se nečte znovu; změněný ano.
#[test]
fn rescan_reads_only_changed_files() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    write(dir, "a.ps1", "# @ledge.title A\n");
    write(dir, "b.ps1", "# @ledge.title B\n");
    let mut first = scan_dir(dir, None);
    // podvrh v cache: kdyby se a.ps1 četl znovu, titulek by byl zase „A“
    let entry = first.stamps.get_mut("a.ps1").unwrap();
    if let Some(s) = entry.2.as_mut() {
        s.meta.as_mut().unwrap().title = "Z cache".into();
    }
    write(dir, "b.ps1", "# @ledge.title B2 — delší\n");
    let second = scan_dir(dir, Some(&first));
    let titles: Vec<String> = second.scripts.iter().map(|s| s.meta.clone().unwrap().title).collect();
    assert_eq!(titles, ["Z cache", "B2 — delší"]);
    // smazaný soubor zmizí, nový přibude
    fs::remove_file(dir.join("a.ps1")).unwrap();
    write(dir, "c.py", "# @ledge.title C\n");
    let third = scan_dir(dir, Some(&second));
    assert_eq!(third.scripts.iter().map(|s| s.file.as_str()).collect::<Vec<_>>(), ["b.ps1", "c.py"]);
    // jiná složka = cache se nepoužije
    let other = tempfile::tempdir().unwrap();
    assert!(scan_dir(other.path(), Some(&third)).scripts.is_empty());
}

#[test]
fn caps_count_and_size() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    for i in 0..MAX_SCRIPTS + 5 {
        write(dir, &format!("s{i:03}.cmd"), &format!(":: @ledge.title S{i}\n"));
    }
    let scan = scan_dir(dir, None);
    assert_eq!((scan.scripts.len(), scan.skipped), (MAX_SCRIPTS, 5));
    let big = tempfile::tempdir().unwrap();
    let mut text = String::from("# @ledge.title Velký\n");
    text.push_str(&"x".repeat(MAX_FILE as usize));
    write(big.path(), "big.py", &text);
    let s = &scan_dir(big.path(), None).scripts[0];
    assert!(s.meta.as_ref().unwrap_err().contains("1 MB") && s.sha.is_empty());
}

/// Hlavička za hranicí 4 KB se nečte; obsah za ní se do otisku počítá.
#[test]
fn header_only_first_4k_but_hash_covers_all() {
    let d = tempfile::tempdir().unwrap();
    let pad = "#\n".repeat(meta::HEADER_BYTES);
    let p = write(d.path(), "late.ps1", &format!("{pad}# @ledge.title Pozdě\n"));
    assert_eq!(load_one(&p, Lang::Powershell, fs::metadata(&p).unwrap().len()).unwrap(), None);
    let a = write(d.path(), "a.ps1", "# @ledge.title A\nWrite-Output 1\n");
    let sha1 = load_one(&a, Lang::Powershell, fs::metadata(&a).unwrap().len()).unwrap().unwrap().1;
    let mut long = String::from("# @ledge.title A\n");
    long.push_str(&"#\n".repeat(meta::HEADER_BYTES));
    let b = write(d.path(), "b.ps1", &format!("{long}Write-Output 1\n"));
    let c = write(d.path(), "c.ps1", &format!("{long}Write-Output 2\n"));
    let h = |p: &Path| load_one(p, Lang::Powershell, fs::metadata(p).unwrap().len()).unwrap().unwrap().1;
    assert_ne!(h(&b), h(&c), "změna až za 4 KB změní otisk");
    assert_ne!(sha1, h(&b));
}

#[test]
fn view_confirms_first_run_and_on_confirm() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "a.ps1", "# @ledge.title Ahoj\n# @ledge.description Pozdraví\n");
    write(d.path(), "b.ps1", "# @ledge.title Vždy\n# @ledge.confirm\n");
    write(d.path(), "c.py", "# @ledge.title Py\n");
    let scan = scan_dir(d.path(), None);
    let ok = Ok(Interp::Powershell);
    let v = view(&scan.scripts[0], Some(&ok), false, false);
    assert!(v.needs_confirm && v.confirm_note.is_some(), "poprvé se ptá");
    assert!(v.confirm_text.contains("a.ps1") && v.confirm_text.contains(&scan.scripts[0].sha[..12]));
    assert_eq!((v.label.as_str(), v.description.as_deref(), v.problem.as_deref()), ("Ahoj", Some("Pozdraví"), None));
    assert!(!view(&scan.scripts[0], Some(&ok), true, false).needs_confirm, "se souhlasem ne");
    assert!(view(&scan.scripts[1], Some(&ok), true, false).needs_confirm, "@ledge.confirm vždy");
    let missing: Result<Interp, String> = Err("Python není na PATH".into());
    assert_eq!(view(&scan.scripts[2], Some(&missing), true, false).problem.as_deref(), Some("Python není na PATH"));
    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains(r#""kind":"command""#) && json.contains(r#""script":true"#) && json.contains(r#""needsConfirm":true"#), "{json}");
}

#[test]
fn script_fingerprint_follows_content() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "a.cmd", ":: @ledge.title A\necho 1\n");
    let one = scan_dir(d.path(), None).scripts[0].fingerprint().unwrap();
    write(d.path(), "a.cmd", ":: @ledge.title A\necho 2\n");
    let two = scan_dir(d.path(), None).scripts[0].fingerprint().unwrap();
    assert_ne!(one, two, "změna souboru = nový souhlas");
    // souhlas v temp souboru (ne %LOCALAPPDATA%)
    let mut c = crate::actionrun::Consents::at(d.path().join("c.json"));
    assert!(!c.has_fp(&one));
    c.grant_fp(one.clone()).unwrap();
    assert!(c.has_fp(&one) && !c.has_fp(&two));
}

#[test]
fn dir_resolution() {
    let var = |k: &str| (k == "DATA").then(|| r"D:\data".to_string());
    assert_eq!(resolve_dir_with(Some(r"%DATA%\skripty"), var).unwrap(), PathBuf::from(r"D:\data\skripty"));
    assert!(resolve_dir_with(Some("skripty"), var).is_err());
    assert!(resolve_dir_with(Some(r"C:\a\..\b"), var).is_err());
    assert!(resolve_dir_with(Some(r"%NENI%\x"), var).is_err());
    assert!(resolve_dir_with(Some("  "), var).unwrap().ends_with(r"Ledge\scripts"));
    assert!(resolve_dir_with(None, var).unwrap().ends_with(r"Ledge\scripts"));
}

#[test]
fn config_default_and_roundtrip() {
    let cfg: crate::config::Config = serde_json::from_str("{}").unwrap();
    assert!(cfg.scripts.enabled && cfg.scripts.dir.is_none() && !cfg.scripts.in_actions);
    let cfg: crate::config::Config = serde_json::from_str(r#"{"scripts":{"inActions":true}}"#).unwrap();
    assert!(cfg.scripts.enabled && cfg.scripts.in_actions);
    let back = serde_json::to_string(&cfg.scripts).unwrap();
    assert_eq!(back, r#"{"enabled":true,"inActions":true}"#);
    assert!(crate::config::schema_json().contains("inActions"));
}

/// Skutečné spuštění bez okna: cmd i PowerShell, výstup, exit kód, diakritika.
#[test]
fn runs_cmd_and_powershell_hidden() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    let c = write(dir, "a b.cmd", ":: @ledge.title A\r\n@echo off\r\necho ahoj %1\r\n");
    let plan = run::plan(&c, &Interp::Cmd, &["svět".into()], false).unwrap();
    let out = crate::actionrun::capture(&mut plan.command(dir), "A", 20, true).unwrap();
    assert_eq!(out.output.as_deref(), Some("ahoj svět"));
    let quiet = crate::actionrun::capture(&mut plan.command(dir), "A", 20, false).unwrap();
    assert_eq!(quiet.output, None, "hidden = bez výstupu");

    // Windows PowerShell 5.1 čte UTF-8 bez BOM jako ANSI → skript s diakritikou chce BOM
    let p = dir.join("p.ps1");
    fs::write(&p, [&[0xEF, 0xBB, 0xBF][..], "# @ledge.title P\nparam($x)\nWrite-Output \"žluť $x\"\nexit 3\n".as_bytes()].concat()).unwrap();
    let plan = run::plan(&p, &Interp::Powershell, &["a'b".into()], false).unwrap();
    let e = crate::actionrun::capture(&mut plan.command(dir), "P", 30, true).unwrap_err();
    assert!(e.contains("chybou") && e.contains("žluť a'b"), "{e}");
}
