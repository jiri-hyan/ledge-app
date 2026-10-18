//! Průvodce, krok Aplikace: zástupci ve Start menu (uživatel i všichni uživatelé) → cíl `.exe`
//! a argumenty přes `IShellLinkW` (COM, MTA — `ShellLink` je „Both“). Odinstalátory a duplicity
//! pryč ([`keep`], [`dedupe`] jsou čisté). Aplikace ze Storu (bez `.lnk`) jsou v `store.rs`.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::core::{HSTRING, Interface};

/// Nejvíc zástupců na jeden sken (pojistka).
const MAX_LINKS: usize = 1500;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FoundApp {
    /// jméno zástupce (bez `.lnk`)
    pub name: String,
    pub exe: String,
    pub args: Vec<String>,
    /// projekt se stejným exe / jménem aplikace už v Ledge je
    pub already: bool,
}

/// `%APPDATA%` a `%ProgramData%` → `Microsoft\Windows\Start Menu\Programs`.
pub fn start_menu_dirs(var: impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    ["APPDATA", "ProgramData"]
        .iter()
        .filter_map(|k| var(k).filter(|v| !v.trim().is_empty()))
        .map(|b| Path::new(&b).join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
        .collect()
}

/// Zástupce stojí za nabídnutí: cíl je `.exe` a nejde o odinstalátor / instalátor / nápovědu.
pub fn keep(name: &str, target: &str) -> bool {
    let n = name.to_lowercase();
    let t = target.to_lowercase();
    let noise = ["uninstall", "unins0", "odinstal", "setup.exe", "readme", "release notes", "help"];
    t.ends_with(".exe") && !noise.iter().any(|w| n.contains(w) || t.rsplit('\\').next().unwrap_or("").contains(w))
}

/// Klíč duplicity: stejné exe (bez ohledu na velikost písmen, `/` = `\`, závěrečné `\`) a stejné argumenty.
fn dedupe_key(a: &FoundApp) -> (String, String) {
    (a.exe.to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string(), a.args.join("\u{1f}"))
}

/// Jeden zástupce na cíl+argumenty (kratší jméno vyhraje), seřazeno podle jména.
pub fn dedupe(mut apps: Vec<FoundApp>) -> Vec<FoundApp> {
    apps.sort_by_key(|a| a.name.len());
    let mut seen = std::collections::HashSet::new();
    apps.retain(|a| seen.insert(dedupe_key(a)));
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// Argumenty příkazové řádky zástupce → seznam podle pravidel Windows (`CommandLineToArgvW`): mezera
/// odděluje mimo uvozovky, uvozovky seskupují a samy se zahodí, `\"` je doslovná uvozovka (před uvozovkou
/// se sudý počet zpětných lomítek půlí, jinak zůstávají), `""` uvnitř uvozovek je doslovná uvozovka.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut any) = (false, false);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' => {
                let mut n = 1;
                while it.peek() == Some(&'\\') {
                    it.next();
                    n += 1;
                }
                if it.peek() == Some(&'"') {
                    cur.extend(std::iter::repeat_n('\\', n / 2));
                    if n % 2 == 1 {
                        it.next();
                        cur.push('"');
                    }
                } else {
                    cur.extend(std::iter::repeat_n('\\', n));
                }
            }
            '"' => {
                if quoted && it.peek() == Some(&'"') {
                    it.next();
                    cur.push('"');
                } else {
                    quoted = !quoted;
                }
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn abs_path_start(t: &str) -> bool {
    let b = t.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')) || t.starts_with("\\\\")
}

/// Jako [`split_args`], ale cesta bez uvozovek s mezerou (`C:\Program Files\Foo\a.txt`) zůstane jedním
/// argumentem: po rozdělení se sousední kousky, které začínají cestou, slepí zpět, **pokud takto slepená
/// cesta existuje** (`exists`). Nic neexistujícího se nelepí.
pub fn split_args_fs(s: &str, exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    let toks = split_args(s);
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        let merged = if abs_path_start(&toks[i]) { (i + 2..=toks.len().min(i + 8)).rev().map(|j| (j, toks[i..j].join(" "))).find(|(_, p)| exists(p)) } else { None };
        match merged {
            Some((j, path)) => {
                out.push(path);
                i = j;
            }
            None => {
                out.push(toks[i].clone());
                i += 1;
            }
        }
    }
    out
}

fn lnk_files(dirs: &[PathBuf], deadline: Instant) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, u8)> = dirs.iter().map(|d| (d.clone(), 0)).collect();
    while let Some((d, depth)) = stack.pop() {
        if out.len() >= MAX_LINKS || Instant::now() >= deadline {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() && !ft.is_symlink() && depth < 3 {
                stack.push((p, depth + 1));
            } else if ft.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
                out.push(p);
            }
        }
    }
    out
}

struct Com(bool);
impl Com {
    fn init() -> Self {
        Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() }
        }
    }
}

fn wide_str(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

/// Cíl a argumenty zástupce. `None` = nejde načíst, nebo nemá souborový cíl (např. MSI „advertised“).
pub fn read_lnk(path: &Path) -> Option<(String, String)> {
    let _com = Com::init();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>().ok()?.Load(&HSTRING::from(path.as_os_str()), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        link.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;
        let target = wide_str(&buf);
        let mut args = [0u16; 1024];
        let args = if link.GetArguments(&mut args).is_ok() { wide_str(&args) } else { String::new() };
        (!target.is_empty()).then_some((target, args))
    }
}

/// Celý krok: zástupci → aplikace (cíl existuje), bez šumu a duplicit.
pub fn discover(dirs: &[PathBuf], deadline: Instant, known_exes: &[String], known_names: &[String]) -> Vec<FoundApp> {
    let mut apps = Vec::new();
    for lnk in lnk_files(dirs, deadline) {
        if Instant::now() >= deadline {
            break;
        }
        let name = lnk.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let Some((exe, args)) = read_lnk(&lnk) else { continue };
        if !keep(&name, &exe) || !Path::new(&exe).is_file() {
            continue;
        }
        let already = known_exes.contains(&exe.to_lowercase()) || known_names.contains(&name.to_lowercase());
        apps.push(FoundApp { name, exe, args: split_args_fs(&args, &|p| Path::new(p).exists()).into_iter().take(16).collect(), already });
    }
    dedupe(apps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_table() {
        #[rustfmt::skip]
        let cases = [
            ("Visual Studio Code", r"C:\Programs\Code\Code.exe",            true),
            ("Uninstall Foo",      r"C:\Foo\uninst.exe",                    false),
            ("Foo",                r"C:\Foo\unins000.exe",                  false),
            ("Odinstalovat Bar",   r"C:\Bar\bar.exe",                       false),
            ("Foo Readme",         r"C:\Foo\readme.exe",                    false),
            ("Dokument",           r"C:\Foo\manual.pdf",                    false),
            ("Web",                r"C:\Foo\site.url",                      false),
        ];
        for (n, t, want) in cases {
            assert_eq!(keep(n, t), want, "{n}");
        }
    }

    #[test]
    fn split_args_table() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        #[rustfmt::skip]
        let cases: Vec<(&str, Vec<String>)> = vec![
            (r#"--profile-directory="Profile 1" -x  "" y"#,  v(&["--profile-directory=Profile 1", "-x", "", "y"])),
            (r#""C:\Program Files\App\a.txt" -v"#,           v(&[r"C:\Program Files\App\a.txt", "-v"])),
            (r#"--dir="C:\Program Files (x86)\Foo""#,        v(&[r"--dir=C:\Program Files (x86)\Foo"])),
            (r#""C:\Program Files\Foo\" next"#,              v(&[r#"C:\Program Files\Foo" next"#])),
            (r#""C:\Program Files\Foo\\" next"#,             v(&[r"C:\Program Files\Foo\", "next"])),
            (r#"a\"b c"#,                                    v(&["a\"b", "c"])),
            (r#"x\\y \z"#,                                   v(&[r"x\\y", r"\z"])),
            (r#""a ""quoted"" b""#,                          v(&["a \"quoted\" b"])),
            (r#"/e,"C:\a b""#,                               v(&[r"/e,C:\a b"])),
            ("   ",                                          v(&[])),
            ("",                                             v(&[])),
            (r#"-- "x""#,                                    v(&["--", "x"])),
        ];
        for (input, want) in cases {
            assert_eq!(split_args(input), want, "{input}");
        }
    }

    #[test]
    fn split_args_keeps_existing_unquoted_path_together() {
        let fs = |p: &str| p.eq_ignore_ascii_case(r"C:\Program Files\Foo Bar\notes.txt") || p == r"C:\Program Files";
        #[rustfmt::skip]
        let cases = [
            (r"C:\Program Files\Foo Bar\notes.txt",           vec![r"C:\Program Files\Foo Bar\notes.txt"]),
            (r"--open C:\Program Files\Foo Bar\notes.txt -v", vec!["--open", r"C:\Program Files\Foo Bar\notes.txt", "-v"]),
            (r"C:\Program Files",                             vec![r"C:\Program Files"]),
            (r"C:\Nikde Neexistuje\a.txt",                    vec![r"C:\Nikde", r"Neexistuje\a.txt"]),
            (r"-x Program Files",                             vec!["-x", "Program", "Files"]),
        ];
        for (input, want) in cases {
            assert_eq!(split_args_fs(input, &fs), want, "{input}");
        }
    }

    #[test]
    fn dedupe_table() {
        let a = |n: &str, e: &str, args: &[&str]| FoundApp { name: n.into(), exe: e.into(), args: args.iter().map(|s| s.to_string()).collect(), already: false };
        let names = |d: Vec<FoundApp>| d.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
        // stejné exe jinak psané (velikost, lomítka, závěrečné \) = jedna položka, vyhraje kratší jméno
        assert_eq!(names(dedupe(vec![a("Code (2)", r"C:\c.exe", &[]), a("code", r"C:\C.EXE", &[]), a("Alfa", r"C:\a.exe", &[])])), ["Alfa", "code"]);
        assert_eq!(names(dedupe(vec![a("Foo uživatel", r"C:\Program Files\Foo\foo.exe", &[]), a("Foo", "c:/program files/foo/foo.exe", &[])])), ["Foo"]);
        // jiné argumenty = jiná položka; ["a b"] není ["a", "b"]
        let d = dedupe(vec![a("Foo", r"C:\f.exe", &[]), a("Foo --safe", r"C:\f.exe", &["--safe"]), a("Foo x", r"C:\f.exe", &["a b"]), a("Foo y", r"C:\f.exe", &["a", "b"])]);
        assert_eq!(d.len(), 4);
        // stejné exe i stejné argumenty ze dvou Start menu
        let d = dedupe(vec![a("Edge", r"C:\Program Files\E\edge.exe", &["--app", r"C:\Program Files\X"]), a("Edge", r"C:\PROGRAM FILES\E\edge.exe", &["--app", r"C:\Program Files\X"])]);
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn start_menu_dirs_from_env() {
        let d = start_menu_dirs(|k| (k == "APPDATA").then(|| r"C:\Users\ada\AppData\Roaming".to_string()));
        assert_eq!(d, [PathBuf::from(r"C:\Users\ada\AppData\Roaming\Microsoft\Windows\Start Menu\Programs")]);
    }

    /// Skutečný `.lnk` vyrobený přes `IShellLinkW` v dočasné složce → sken ho najde a přečte cíl.
    #[test]
    fn reads_real_shortcut() {
        let t = tempfile::tempdir().unwrap();
        let sub = t.path().join("Nástroje");
        std::fs::create_dir_all(&sub).unwrap();
        let lnk = sub.join("Průzkumník.lnk");
        let exe = std::path::PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into())).join("explorer.exe");
        {
            let _com = Com::init();
            unsafe {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
                link.SetPath(&HSTRING::from(exe.as_os_str())).unwrap();
                link.SetArguments(&HSTRING::from(r#"/e,"C:\a b""#)).unwrap();
                link.cast::<IPersistFile>().unwrap().Save(&HSTRING::from(lnk.as_os_str()), true).unwrap();
            }
        }
        let (target, args) = read_lnk(&lnk).unwrap();
        assert!(target.eq_ignore_ascii_case(&exe.to_string_lossy()), "{target}");
        assert_eq!(args, r#"/e,"C:\a b""#);
        let found = discover(&[t.path().to_path_buf()], Instant::now() + std::time::Duration::from_secs(20), &[], &["průzkumník".into()]);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].args.clone(), found[0].already), ("Průzkumník", vec![r"/e,C:\a b".to_string()], true));
        assert!(read_lnk(&t.path().join("neni.lnk")).is_none());
    }
}
