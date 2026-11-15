//! Spuštění widgetu: program + argumenty (skript přes `scripts::run::plan` — interpret z PATH bez
//! aliasů Storu, PowerShell `-NonInteractive`, UTF-8 výstup), pracovní složka = složka widgetu, bez okna
//! (`CREATE_NO_WINDOW`), stdin prázdný, termín a strop výstupu. Zabití po termínu = celý strom
//! (`gitinfo::capture_tail` → `procman::Job`, KILL_ON_JOB_CLOSE). Žádné zvýšení práv (žádné `runas`).

use super::Widget;
use super::meta::WidgetMeta;
use crate::gitinfo::{self, RunError};
use crate::i18n::tr;
use std::process::Command;
use std::time::{Duration, Instant};

/// Nejvíc stdout (bajtů); větší výstup = chyba (useknutý JSON by stejně nešel přečíst).
pub const MAX_OUTPUT: usize = 256 * 1024;
/// Kolik výstupu si Ledge pamatuje pro „Zobrazit výstup“.
const KEEP_OUTPUT: usize = 32 * 1024;

/// Příkaz pro widget (nespouští).
pub fn command(w: &Widget, m: &WidgetMeta, path_var: &std::ffi::OsStr) -> Result<Command, String> {
    let dir = w.path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
    if w.kind == super::meta::Kind::Kit {
        return Err(tr("šablona se nespouští", "a template is never run").into());
    }
    match w.kind.lang() {
        None => {
            let mut cmd = Command::new(&w.path);
            cmd.args(&m.args).current_dir(dir);
            Ok(cmd)
        }
        Some(lang) => {
            let interp = crate::scripts::run::interp(lang, path_var)?;
            Ok(crate::scripts::run::plan(&w.path, &interp, &m.args, false)?.command(&dir))
        }
    }
}

/// Proč běh nedal data.
#[derive(Debug, PartialEq)]
pub enum Fail {
    Timeout(u32),
    TooBig,
    /// nenulový exit; konec výstupu
    Exit(String),
    Spawn(String),
}

#[derive(Debug, PartialEq)]
pub struct Ran {
    pub stdout: String,
    pub stderr: String,
}

/// Spustí a počká nejvýš `timeout_s`. Výstup nad `MAX_OUTPUT` = `TooBig`.
pub fn execute(cmd: &mut Command, timeout_s: u32) -> Result<Ran, (Fail, Option<Ran>)> {
    let deadline = Instant::now() + Duration::from_secs(timeout_s.into());
    match gitinfo::capture_tail(cmd, None, true, deadline, Some(MAX_OUTPUT + 1)) {
        Ok(out) => {
            let ran = Ran { stdout: out.stdout, stderr: out.stderr };
            if ran.stdout.len() > MAX_OUTPUT {
                Err((Fail::TooBig, None))
            } else if !out.success {
                Err((Fail::Exit(first_error_line(&ran.stdout, &ran.stderr)), Some(ran)))
            } else {
                Ok(ran)
            }
        }
        Err(RunError::Timeout) => Err((Fail::Timeout(timeout_s), None)),
        Err(RunError::Spawn(e)) => Err((Fail::Spawn(e), None)),
    }
}

/// Příčina chyby do hlášky: první neprázdný řádek stderr (u PowerShellu je příčina nahoře, dole je
/// `CategoryInfo` / `FullyQualifiedErrorId`), bez stderr poslední neprázdný řádek stdout (L242).
pub(super) fn first_error_line(stdout: &str, stderr: &str) -> String {
    let non_empty = |s: &str| s.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string);
    non_empty(stderr).or_else(|| stdout.lines().map(str::trim).rev().find(|l| !l.is_empty()).map(str::to_string)).unwrap_or_default()
}

/// Lidská hláška do karty.
pub fn fail_text(f: &Fail) -> String {
    match f {
        Fail::Timeout(s) => trf!("Skript nedoběhl do {s} s — Ledge ho ukončil. Prodluž @ledge.timeout, nebo ho zrychli.", "The script didn't finish within {s} s — Ledge stopped it. Raise @ledge.timeout or make it faster."),
        Fail::TooBig => tr("Skript vypsal víc než 256 KB — zkrať výstup.", "The script printed more than 256 KB — shorten the output.").into(),
        Fail::Exit(t) if t.is_empty() => tr("Skript skončil chybou.", "The script ended with an error.").into(),
        Fail::Exit(t) => trf!("Skript skončil chybou: {}", "The script ended with an error: {}", t.lines().last().unwrap_or("")),
        Fail::Spawn(e) => crate::errors::spawn_text(tr("Skript nejde spustit", "Can't run the script"), e),
    }
}

/// Výstup pro „Zobrazit výstup“: stdout, pak stderr; nejvýš 32 KB (začátek).
pub fn keep(r: &Ran) -> String {
    let mut s = r.stdout.clone();
    if !r.stderr.trim().is_empty() {
        s.push_str("\n--- stderr ---\n");
        s.push_str(&r.stderr);
    }
    if s.len() > KEEP_OUTPUT {
        let mut end = KEEP_OUTPUT;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
        s.push('…');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::first_error_line;

    #[test]
    fn error_line_table() {
        let ps = "\r\nNelze najít soubor 'x.txt'.\r\nAt C: w a.ps1:3 char:1\r\n+ Get-Content x.txt\r\n    + CategoryInfo          : ObjectNotFound\r\n    + FullyQualifiedErrorId : PathNotFound\r\n";
        let cases = [
            ("", ps, "Nelze najít soubor 'x.txt'."),
            ("řádek 1\nposlední\n\n", "", "poslední"),
            ("výstup\n", "  chyba  \nvíc\n", "chyba"),
            ("", "\n\n", ""),
            ("", "", ""),
        ];
        for (out, err, want) in cases {
            assert_eq!(first_error_line(out, err), want, "{err:?}");
        }
    }
}
