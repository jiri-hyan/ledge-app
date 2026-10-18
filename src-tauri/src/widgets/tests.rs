//! Sken složky widgetů v tempdiru, cache změněných, program s hlavičkou vedle, skutečné spuštění
//! (cmd, PowerShell) s termínem a stropem výstupu, souhlas po změně souboru. Žádné `%USERPROFILE%`.

use super::consent::{Consent, Store};
use super::meta::{Kind, Show};
use super::*;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn scan_finds_widgets_and_skips_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "a.ps1", "# @ledge.widget Alfa\n# @ledge.show strip\nWrite-Output 1");
    write(d, "b.py", "# @ledge.widget\nprint(1)");
    write(d, "helper.py", "def f(): pass");
    write(d, "skript.ps1", "# @ledge.title Script Command, ne widget");
    write(d, "readme.txt", "@ledge.widget Ne");
    write(d, ".skryty.ps1", "# @ledge.widget Skrytý");
    write(d, "prog.exe", "MZ-not-really");
    write(d, "prog.exe.ledge", "@ledge.widget Program\n@ledge.args --json\n");
    write(d, "bez.exe", "MZ");
    std::fs::create_dir(d.join("sub.ps1")).unwrap();
    let s = scan_dir(d, None);
    assert!(s.exists);
    let ids: Vec<&str> = s.widgets.iter().map(|w| w.id.as_str()).collect();
    assert_eq!(ids, ["widget:a.ps1", "widget:b.py", "widget:prog.exe"]);
    let a = s.get("widget:a.ps1").unwrap();
    assert_eq!(a.meta.as_ref().unwrap().show, Show::Strip);
    assert!(a.head.contains("Write-Output"));
    assert!(s.get("widget:b.py").unwrap().meta.as_ref().unwrap_err().contains("název"));
    let p = s.get("widget:prog.exe").unwrap();
    assert_eq!((p.kind, p.meta.as_ref().unwrap().args.clone()), (Kind::Exe, vec!["--json".to_string()]));
    assert!(p.head.contains("@ledge.widget Program"));
    // chybějící složka = prázdný sken bez chyby
    let none = scan_dir(&d.join("neni"), None);
    assert!(!none.exists && none.widgets.is_empty() && none.error.is_none());
}

#[test]
fn scan_reuses_unchanged_and_sees_changes() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let a = write(d, "a.ps1", "# @ledge.widget A\n1");
    write(d, "p.exe", "MZ");
    let side = write(d, "p.exe.ledge", "@ledge.widget P\n");
    let first = scan_dir(d, None);
    let sha_a = first.get("widget:a.ps1").unwrap().sha.clone();
    let sha_p = first.get("widget:p.exe").unwrap().sha.clone();
    let again = scan_dir(d, Some(&first));
    assert_eq!(again.get("widget:a.ps1").unwrap().sha, sha_a);
    std::fs::write(&a, "# @ledge.widget A\n2 a ještě něco delšího").unwrap();
    // změna hlavičky programu (bez změny .exe) = jiný otisk
    std::fs::write(&side, "@ledge.widget P\n@ledge.args -x\n").unwrap();
    let changed = scan_dir(d, Some(&again));
    assert_ne!(changed.get("widget:a.ps1").unwrap().sha, sha_a);
    assert_ne!(changed.get("widget:p.exe").unwrap().sha, sha_p);
}

#[test]
fn fresh_rejects_paths_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "a.cmd", ":: @ledge.widget A\n@echo 1");
    assert!(fresh(d, "widget:a.cmd").is_ok());
    for bad in ["widget:", "widget:..\\a.cmd", "widget:C:a.cmd", "script:a.cmd", "widget:a.txt", "widget:neni.cmd"] {
        assert!(fresh(d, bad).is_err(), "{bad}");
    }
}

#[test]
fn dir_resolution() {
    let var = |_: &str| None::<String>;
    assert_eq!(resolve_dir_with(None, var).unwrap(), default_dir());
    assert_eq!(resolve_dir_with(Some("  "), var).unwrap(), default_dir());
    assert_eq!(resolve_dir_with(Some(r"D:\w"), var).unwrap(), PathBuf::from(r"D:\w"));
    let e = resolve_dir_with(Some(r"relativni\w"), var).unwrap_err();
    assert!(e.contains("widgets.dir"), "{e}");
    assert!(default_dir().ends_with(r"Ledge\widgets"));
}

fn run_widget(d: &Path, name: &str) -> Result<run::Ran, (run::Fail, Option<run::Ran>)> {
    let w = fresh(d, &format!("widget:{name}")).unwrap();
    let m = w.meta.clone().unwrap();
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let mut cmd = run::command(&w, &m, &path_var).unwrap();
    run::execute(&mut cmd, m.timeout_s)
}

#[test]
fn real_runs_cmd_and_powershell() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    // pracovní složka = složka widgetu
    write(d, "data.txt", "z-souboru");
    write(d, "cwd.cmd", ":: @ledge.widget Cwd\r\n@echo off\r\ntype data.txt\r\n");
    let r = run_widget(d, "cwd.cmd").unwrap();
    assert_eq!(output::parse(&r.stdout).strip.as_deref(), Some("z-souboru"));
    // .ps1 s diakritikou = UTF-8 s BOM (Windows PowerShell 5.1), jako šablony z galerie
    write(d, "j.ps1", "\u{feff}# @ledge.widget Json\n@{ text = 'Žluťoučký'; items = @(@{ label = 'a'; value = 1 }) } | ConvertTo-Json -Depth 5 -Compress\n");
    let d2 = output::parse(&run_widget(d, "j.ps1").unwrap().stdout);
    assert_eq!(d2.text.as_deref(), Some("Žluťoučký"), "UTF-8 z PowerShellu");
    assert_eq!(d2.items[0].value.as_deref(), Some("1"));
    write(d, "err.cmd", ":: @ledge.widget Err\r\n@echo nejde 1>&2\r\n@exit /b 3\r\n");
    match run_widget(d, "err.cmd") {
        Err((run::Fail::Exit(t), Some(_))) => assert!(t.contains("nejde"), "{t}"),
        x => panic!("{x:?}"),
    }
}

#[test]
fn real_run_timeout_and_output_cap() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "slow.cmd", ":: @ledge.widget Pomalý\r\n:: @ledge.timeout 1\r\n@ping -n 6 127.0.0.1 >nul\r\n");
    let t = std::time::Instant::now();
    assert!(matches!(run_widget(d, "slow.cmd"), Err((run::Fail::Timeout(1), None))));
    assert!(t.elapsed() < std::time::Duration::from_secs(5), "strom zabit po termínu");
    write(d, "big.ps1", "# @ledge.widget Velký\n# @ledge.timeout 30\n[Console]::Out.Write('x' * 300000)\n");
    assert!(matches!(run_widget(d, "big.ps1"), Err((run::Fail::TooBig, None))));
    assert!(run::fail_text(&run::Fail::TooBig).contains("256 KB"));
}

/// Příklady z galerie bez sítě opravdu běží a dají text do proužku (Python jen, když je na PATH).
#[test]
fn offline_templates_really_run() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let python = crate::scripts::run::interp(crate::scripts::meta::Lang::Python, &path_var).is_ok();
    for file in ["odpocet.ps1", "uptime.ps1", "disk.ps1", "odpocet.py", "uptime.py", "disk.py"] {
        if file.ends_with(".py") && !python {
            continue;
        }
        templates::install(d, file).unwrap();
        let mut w = fresh(d, &format!("widget:{file}")).unwrap();
        if let Ok(m) = w.meta.as_mut() {
            m.timeout_s = 60;
        }
        let r = run_widget_meta(&w).unwrap_or_else(|e| panic!("{file}: {e:?}"));
        let data = output::parse(&r.stdout);
        assert!(data.notes.is_empty(), "{file}: {:?}\n{}", data.notes, r.stdout);
        assert!(data.strip_text().is_some(), "{file}: proužek\n{}", r.stdout);
    }
}

fn run_widget_meta(w: &Widget) -> Result<run::Ran, (run::Fail, Option<run::Ran>)> {
    let m = w.meta.clone().unwrap();
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let mut cmd = run::command(w, &m, &path_var).unwrap();
    run::execute(&mut cmd, m.timeout_s)
}

#[test]
fn consent_follows_content() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let p = write(d, "w.ps1", "# @ledge.widget W\n1");
    let mut store = Store::at(dir.path().join("consent.json"));
    let w = fresh(d, "widget:w.ps1").unwrap();
    let fp = w.fingerprint().unwrap();
    assert_eq!(store.check(&w.path, &fp), Consent::New);
    assert!(w.confirm_text().contains("SHA-256"));
    store.grant(&w.path, &fp).unwrap();
    assert_eq!(store.check(&w.path, &fp), Consent::Granted);
    std::fs::write(&p, "# @ledge.widget W\nRemove-Item -Recurse C:\\").unwrap();
    let w2 = fresh(d, "widget:w.ps1").unwrap();
    assert_eq!(store.check(&w2.path, &w2.fingerprint().unwrap()), Consent::Changed, "změna souboru = znovu se ptát");
    assert_ne!(w.confirm_text(), w2.confirm_text(), "staré potvrzení neplatí");
    // chybná hlavička = žádný otisk (nejde povolit ani spustit)
    std::fs::write(&p, "# @ledge.widget W\n# @ledge.refresh brzy").unwrap();
    assert!(fresh(d, "widget:w.ps1").unwrap().fingerprint().is_none());
}

#[test]
fn problems_are_human() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "n.js", "// @ledge.widget Node\nconsole.log(1)");
    let w = fresh(d, "widget:n.js").unwrap();
    // PATH bez Node = problém, ne pád
    let p = problem(&w, std::ffi::OsStr::new(""));
    assert!(p.is_some_and(|p| p.contains("Node")));
    write(d, "ok.ps1", "# @ledge.widget OK\n1");
    assert_eq!(problem(&fresh(d, "widget:ok.ps1").unwrap(), std::ffi::OsStr::new("")), None);
}

#[test]
fn config_serde_and_schema() {
    let c: WidgetsConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(c, WidgetsConfig::default());
    assert!(c.enabled && c.dir.is_none());
    let v = serde_json::to_value(WidgetsConfig { enabled: false, dir: Some(r"D:\w".into()) }).unwrap();
    assert_eq!(v, serde_json::json!({"enabled": false, "dir": r"D:\w"}));
    // starý config bez sekce `widgets` = výchozí
    let cfg: crate::config::Config = serde_json::from_str("{}").unwrap();
    assert_eq!(cfg.widgets, WidgetsConfig::default());
    assert!(crate::config::schema_json().contains("\"widgets\""));
}
