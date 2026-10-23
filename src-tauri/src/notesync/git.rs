//! Git příkazy synchronizace: spuštění s termínem a bez dotazů (`git`, `env_for`), převod selhání na
//! `Outcome` (`net_fail`, `local_fail`), nastavení repa (`setup_repo`) a commit všeho (`commit_all`).
//! Závisí na `gitinfo::run_env`.

use super::{ATTRIBUTES, IGNORE, LOCAL_DEADLINE, NET_DEADLINE, Outcome, classify, ensure_lines};
use crate::gitinfo;
use std::path::Path;
use std::time::Instant;

pub(super) enum Fail {
    Timeout,
    Git(String),
    /// git se nepodařilo spustit (text chyby systému)
    Spawn(String),
}

fn env_for(remote: &str) -> Vec<(&'static str, &'static str)> {
    let mut v = vec![
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GCM_INTERACTIVE", "never"),
        // anglické hlášky — `classify` je čte (git umí i česky)
        ("LC_ALL", "C"),
        ("LANGUAGE", "C"),
    ];
    // ssh bez dotazu na heslo / klíč (jen když si uživatel ssh nenastavil sám)
    let ssh = remote.starts_with("git@") || remote.starts_with("ssh://");
    if ssh && std::env::var_os("GIT_SSH_COMMAND").is_none() {
        v.push(("GIT_SSH_COMMAND", "ssh -o BatchMode=yes"));
    }
    v
}

pub(super) fn git(dir: &Path, remote: &str, args: &[&str], net: bool) -> Result<String, Fail> {
    let deadline = Instant::now() + if net { NET_DEADLINE } else { LOCAL_DEADLINE };
    match gitinfo::run_env(dir, args, &env_for(remote), deadline) {
        Err(gitinfo::RunError::Timeout) => Err(Fail::Timeout),
        Err(gitinfo::RunError::Spawn(e)) => Err(Fail::Spawn(e)),
        Ok(o) if o.success => Ok(o.stdout),
        Ok(o) => Err(Fail::Git(if o.stderr.trim().is_empty() { o.stdout } else { o.stderr })),
    }
}

/// Selhání síťové operace → stav.
pub(super) fn net_fail(f: Fail) -> Outcome {
    match f {
        Fail::Timeout => Outcome::Offline(trf!("git nestihl odpovědět do {} s", "git didn't answer within {} s", NET_DEADLINE.as_secs())),
        Fail::Git(e) => classify(&e),
        Fail::Spawn(e) => Outcome::Chyba(crate::errors::spawn_text(crate::i18n::tr("Git nejde spustit", "Can't run git"), &e)),
    }
}

pub(super) fn local_fail(what: &str, f: Fail) -> Outcome {
    match f {
        Fail::Timeout => Outcome::Chyba(trf!("{what}: git nestihl doběhnout", "{what}: git didn't finish in time")),
        Fail::Git(e) => Outcome::Chyba(crate::errors::git(what, None, &e)),
        Fail::Spawn(e) => Outcome::Chyba(crate::errors::spawn_text(what, &e)),
    }
}

pub(super) fn nul_list(s: &str) -> Vec<String> {
    s.split('\0').filter(|x| !x.is_empty()).map(str::to_string).collect()
}

/// `.gitignore`, `.gitattributes`, identita, konce řádků, podpis, `origin` — idempotentně.
pub(super) fn setup_repo(dir: &Path, remote: &str, pc: &str) -> Result<(), Outcome> {
    let gi = dir.join(".gitignore");
    let existing = std::fs::read_to_string(&gi).unwrap_or_default();
    if let Some(text) = ensure_lines(&existing, &IGNORE) {
        std::fs::write(&gi, text).map_err(|e| Outcome::Chyba(crate::errors::io(crate::i18n::tr("Soubor .gitignore nejde zapsat", "Can't write .gitignore"), &e)))?;
    }
    let ga = dir.join(".gitattributes");
    if !ga.exists() {
        std::fs::write(&ga, ATTRIBUTES).map_err(|e| Outcome::Chyba(crate::errors::io(crate::i18n::tr("Soubor .gitattributes nejde zapsat", "Can't write .gitattributes"), &e)))?;
    }
    let name = format!("Ledge ({pc})");
    let email = format!("ledge@{}.local", pc.to_lowercase());
    for (k, v) in [("user.name", name.as_str()), ("user.email", email.as_str()), ("core.autocrlf", "false"), ("commit.gpgsign", "false")] {
        git(dir, remote, &["config", "--local", k, v], false).map_err(|f| local_fail("git config", f))?;
    }
    match git(dir, remote, &["remote", "get-url", "origin"], false) {
        Ok(url) if url.trim() == remote => {}
        Ok(_) => git(dir, remote, &["remote", "set-url", "origin", remote], false).map(|_| ()).map_err(|f| local_fail("git remote", f))?,
        Err(_) => git(dir, remote, &["remote", "add", "origin", remote], false).map(|_| ()).map_err(|f| local_fail("git remote", f))?,
    }
    Ok(())
}

pub(super) fn commit_all(dir: &Path, remote: &str, msg: &str) -> Result<bool, Outcome> {
    git(dir, remote, &["add", "-A"], false).map_err(|f| local_fail("git add", f))?;
    let st = git(dir, remote, &["status", "--porcelain", "-z"], false).map_err(|f| local_fail("git status", f))?;
    if st.is_empty() {
        return Ok(false);
    }
    // --no-verify: datové repo poznámek; globální hooky pro kód (commitlint…) by automatický commit odmítly
    git(dir, remote, &["commit", "-q", "--no-verify", "-m", msg], false).map_err(|f| local_fail("git commit", f))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_list_splits() {
        assert_eq!(nul_list("a.md\0b c.md\0"), vec!["a.md".to_string(), "b c.md".into()]);
    }
}
