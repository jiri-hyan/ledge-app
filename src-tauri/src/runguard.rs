//! Pojistka spouštění zástupců: klik na zástupce (`projects[]`: `run`, `exe`, `args`) se na rozdíl od rychlé akce
//! nepotvrzuje, takže příkaz, který do configu nezapsal člověk v Nastavení (MCP, import katalogu Launcheru), smí
//! být jen:
//! - `run` = skript balíčkovače (`npm run dev`, `pnpm dev`, `yarn start`, `bun run build`) nebo příkaz, který Ledge
//!   sám najde ve složce projektu (`detected_runs`: návrh průvodce + `projectdetect`);
//! - `exe` = cesta k souboru `.exe` (žádné uvozovky, nové řádky, nulový znak);
//! - `args` = jen ty, které Ledge sám navrhne pro složku.
//!
//! Jedna pravda pro MCP (`mcp/edit.rs` `check_cmd`) i import Launcheru (`commands/settings.rs`
//! `launcher_import_check`); každý volající si k problému dá vlastní větu.

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

/// Skript balíčkovače: `npm run dev`, `pnpm dev`, `yarn start`, `bun run build`, `npm start`.
static PM_SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(npm|pnpm|yarn|bun)( run)? [a-z0-9][a-z0-9:._-]{0,63}$").unwrap());

/// Co na příkazu zástupce neprošlo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    /// `exe` není cesta k `.exe`
    Exe(String),
    /// `run` není skript balíčkovače ani příkaz nalezený ve složce; `known` = co Ledge ve složce našel
    Run { run: String, known: Vec<String> },
    /// `args`, které Ledge pro složku nenavrhuje
    Args,
}

/// (příkaz, argumenty), které Ledge sám navrhne pro složku (průvodce `repos::suggest` + `projectdetect`).
pub fn detected_runs(dir: &Path) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let facts = crate::onboarding::repos::facts(dir);
    if let (_, _, Some(p)) = crate::onboarding::repos::suggest(&facts, &name, &dir.to_string_lossy(), None) {
        out.push((p.run.unwrap_or_default(), p.args));
    }
    if let Ok(d) = crate::projectdetect::detect(dir, &[]) {
        out.push((d.run.unwrap_or_default(), Vec::new()));
    }
    out
}

pub fn exe_ok(exe: &str) -> bool {
    exe.trim().to_ascii_lowercase().ends_with(".exe") && !exe.contains(['"', '\0', '\n', '\r'])
}

pub fn run_ok(run: &str, found: &[(String, Vec<String>)]) -> bool {
    let run = run.trim();
    run.is_empty() || PM_SCRIPT.is_match(&run.to_ascii_lowercase()) || found.iter().any(|(r, _)| r == run)
}

pub fn args_ok(args: &[String], found: &[(String, Vec<String>)]) -> bool {
    args.is_empty() || found.iter().any(|(_, a)| a == args)
}

/// Kontrola jen toho, co se změnilo (`*_changed`); `found` se zjistí líně (čte složku projektu).
pub fn check(exe: Option<&str>, exe_changed: bool, run: Option<&str>, run_changed: bool, args: &[String], args_changed: bool, found: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Result<(), Issue> {
    if let Some(exe) = exe
        && exe_changed
        && !exe_ok(exe)
    {
        return Err(Issue::Exe(exe.to_string()));
    }
    if !run_changed && !args_changed {
        return Ok(());
    }
    let found = found();
    if run_changed && let Some(run) = run.map(str::trim).filter(|r| !r.is_empty()) && !run_ok(run, &found) {
        let known = found.iter().map(|(r, _)| r.clone()).filter(|r| !r.is_empty()).collect();
        return Err(Issue::Run { run: run.to_string(), known });
    }
    if args_changed && !args_ok(args, &found) {
        return Err(Issue::Args);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> Vec<(String, Vec<String>)> {
        Vec::new()
    }

    #[test]
    fn commands_table() {
        let found = || vec![("cargo tauri dev".to_string(), vec!["--release".to_string()])];
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        type Row<'a> = (Option<&'a str>, Option<&'a str>, Vec<String>, bool);
        let rows: [Row; 10] = [
            (None, Some("npm run dev"), a(&[]), true),
            (None, Some("pnpm dev"), a(&[]), true),
            (None, Some("NPM start"), a(&[]), true),
            (None, Some("cargo tauri dev"), a(&[]), true),
            (None, Some("powershell -enc SQBFAFgA"), a(&[]), false),
            (None, Some("npm run dev && del /q C:\\x"), a(&[]), false),
            (Some("build/app.exe"), None, a(&[]), true),
            (Some("cmd.exe\" /c calc"), None, a(&[]), false),
            (Some("build/app.bat"), None, a(&[]), false),
            (Some("build/app.exe"), None, a(&["--release"]), true),
        ];
        for (exe, run, args, want) in rows {
            assert_eq!(check(exe, true, run, true, &args, true, found).is_ok(), want, "{exe:?} {run:?} {args:?}");
        }
        assert_eq!(check(None, false, None, false, &a(&["--evil"]), true, none), Err(Issue::Args));
        assert!(check(Some("x.bat"), false, Some("rm -rf"), false, &[], false, none).is_ok(), "nezměněné se nekontroluje");
        match check(None, true, Some("calc"), true, &[], false, found) {
            Err(Issue::Run { run, known }) => assert_eq!((run.as_str(), known), ("calc", vec!["cargo tauri dev".to_string()])),
            x => panic!("{x:?}"),
        }
    }
}
