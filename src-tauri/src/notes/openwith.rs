//! „Otevřít v aplikaci…“: aplikace, které má Windows registrované pro `.md`
//! (`HKEY_CLASSES_ROOT\.md\OpenWithProgids`), a jejich příkaz `<ProgID>\shell\open\command`.
//! Čisté funkce — registr a spuštění jsou jen v příkazech `note_open_with_apps` / `note_open_with`;
//! testy na ně nesahají. Žádná konkrétní aplikace není v kódu: nabídka je, co má uživatel nainstalované.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Třídy registrované pro `.md` (jména hodnot) — čte se z `HKEY_CLASSES_ROOT`.
pub const MD_PROGIDS_KEY: &str = r".md\OpenWithProgids";

/// `%VAR%` → hodnota z `lookup` (REG_EXPAND_SZ se čte nerozbalený); neznámá proměnná zůstane, jak je.
pub fn expand_env(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) if j > 0 => match lookup(&after[..j]) {
                Some(v) => {
                    out.push_str(&v);
                    rest = &after[j + 1..];
                }
                None => {
                    out.push('%');
                    rest = after;
                }
            },
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Příkazový řádek po slovech; uvozovky slučují, samy se zahodí.
fn split_command(cmd: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted, mut any) = (Vec::new(), String::new(), false, false);
    for ch in cmd.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// Exe a argumenty z registrovaného příkazu pro soubor `file`: `%1` / `%L` se nahradí souborem
/// (jeden argument, mezery nevadí), `%*` a další zástupné znaky se zahodí; bez `%1`/`%L` jde soubor
/// na konec. Exe musí být `.exe` (žádné `.bat`, `rundll32` bez cesty projde jen jako `.exe`).
pub fn command_for(cmd: &str, file: &Path) -> Option<(PathBuf, Vec<OsString>)> {
    let mut words = split_command(cmd).into_iter();
    let exe = words.next().filter(|e| e.to_ascii_lowercase().ends_with(".exe"))?;
    let mut args: Vec<OsString> = Vec::new();
    let mut placed = false;
    for w in words {
        let lower = w.to_ascii_lowercase();
        if lower == "%1" || lower == "%l" {
            args.push(file.as_os_str().to_owned());
            placed = true;
        } else if w.starts_with('%') && w.len() <= 3 {
            continue; // %*, %2, %I… — Ledge předává jen jeden soubor
        } else {
            args.push(w.into());
        }
    }
    if !placed {
        args.push(file.as_os_str().to_owned());
    }
    Some((PathBuf::from(exe), args))
}

/// Jméno do nabídky: `Application\ApplicationName`, jinak popis třídy — ale ne popis typu souboru
/// („Markdown Source File“, „Soubor MD“), ten nic neříká; pak jméno exe bez přípony.
pub fn display_name(app_name: Option<&str>, class_desc: Option<&str>, exe: &Path) -> String {
    let file_type = |s: &str| {
        let l = s.to_lowercase();
        ["file", "soubor", "document", "dokument", "markdown"].iter().any(|w| l.contains(w))
    };
    let pick = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    pick(app_name)
        .or_else(|| pick(class_desc).filter(|s| !file_type(s)))
        .or_else(|| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }

    #[test]
    fn command_is_split_and_the_file_placed() {
        let f = Path::new(r"C:\n y\a.md");
        type Case = (&'static str, Option<(&'static str, Vec<OsString>)>);
        let cases: &[Case] = &[
            (r#""C:\P\Cursor.exe" "%1""#, Some((r"C:\P\Cursor.exe", os(&[r"C:\n y\a.md"])))),
            (r#""C:\A b\app.exe" --app notes "%1""#, Some((r"C:\A b\app.exe", os(&["--app", "notes", r"C:\n y\a.md"])))),
            (r#"C:\x\T.EXE %L"#, Some((r"C:\x\T.EXE", os(&[r"C:\n y\a.md"])))),
            (r#""C:\x\t.exe""#, Some((r"C:\x\t.exe", os(&[r"C:\n y\a.md"])))),
            (r#""C:\x\t.exe" "%1" %*"#, Some((r"C:\x\t.exe", os(&[r"C:\n y\a.md"])))),
            (r#""C:\x\t.bat" "%1""#, None),
            ("rundll32 x", None),
            ("", None),
            (r#""" "%1""#, None),
        ];
        for (cmd, want) in cases {
            let got = command_for(cmd, f);
            let want = want.clone().map(|(e, a)| (PathBuf::from(e), a));
            assert_eq!(got, want, "{cmd}");
        }
    }

    #[test]
    fn env_vars_expand() {
        let look = |k: &str| (k == "LOCALAPPDATA").then(|| r"C:\L".to_string());
        assert_eq!(expand_env(r#""%LOCALAPPDATA%\x.exe" "%1""#, look), r#""C:\L\x.exe" "%1""#);
        assert_eq!(expand_env(r"%NOPE%\a %", look), r"%NOPE%\a %");
        assert_eq!(expand_env("bez", look), "bez");
    }

    #[test]
    fn names_skip_file_type_descriptions() {
        let exe = Path::new(r"C:\P\Cursor.exe");
        assert_eq!(display_name(None, Some("Markdown Source File"), exe), "Cursor");
        assert_eq!(display_name(None, Some("Margin"), Path::new(r"C:\x\office.exe")), "Margin");
        assert_eq!(display_name(Some("Typora"), Some("Soubor MD"), exe), "Typora");
        assert_eq!(display_name(Some(" "), None, exe), "Cursor");
        assert_eq!(display_name(None, None, Path::new("")), "");
    }
}
