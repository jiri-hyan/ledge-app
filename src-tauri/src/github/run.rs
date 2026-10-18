//! Spouštění `gh` (GitHub CLI) pro kartu GitHub: bez okna, s termínem, v dočasné složce (ať `gh` nehádá
//! repo podle cwd), bez interakce. Přihlášení řeší `gh` sám — Ledge žádný token nevidí ani neukládá.
//! Chyby se třídí podle stderr (`classify`, čistá funkce) na to, co umí uživatel opravit.

use crate::gitinfo::{self, RunError};
use std::process::Command;
use std::time::{Duration, Instant};

/// Termín jednoho volání `gh`.
pub const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhErr {
    /// `gh` není nainstalované (nebo ho Ledge nenašel)
    Missing,
    /// `gh` není přihlášené (nebo token vypršel)
    NotLoggedIn,
    /// GitHub je nedostupný / bez sítě / termín
    Offline,
    /// cokoli jiného: první řádek stderr
    Other(String),
}

/// stderr `gh` → druh chyby. Čistá funkce (tabulkový test).
pub fn classify(stderr: &str) -> GhErr {
    let s = stderr.to_lowercase();
    let any = |keys: &[&str]| keys.iter().any(|k| s.contains(k));
    if any(&["gh auth login", "not logged", "authentication required", "bad credentials", "http 401"]) {
        GhErr::NotLoggedIn
    } else if any(&["error connecting", "no such host", "dial tcp", "i/o timeout", "tls handshake", "connection", "timed out", "network is unreachable"]) {
        GhErr::Offline
    } else {
        let line = stderr.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
        GhErr::Other(line.chars().take(200).collect())
    }
}

/// Spustí `gh <args>` a vrátí stdout. `Err(Missing)` bez spuštění, když `gh.exe` nenajdeme.
pub fn run(args: &[&str]) -> Result<String, GhErr> {
    let exe = crate::update::gh_exe().ok_or(GhErr::Missing)?;
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(std::env::temp_dir())
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("GH_SPINNER_DISABLED", "1")
        .env("NO_COLOR", "1");
    match gitinfo::capture(&mut cmd, None, true, Instant::now() + TIMEOUT) {
        Ok(o) if o.success => Ok(o.stdout),
        Ok(o) => Err(classify(&o.stderr)),
        Err(RunError::Timeout) => Err(GhErr::Offline),
        Err(RunError::Spawn(e)) => Err(GhErr::Other(e.chars().take(200).collect())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_table() {
        let cases: [(&str, &str, GhErr); 7] = [
            ("nepřihlášeno", "To get started with GitHub CLI, please run:  gh auth login\n", GhErr::NotLoggedIn),
            ("špatný token", "gh: Bad credentials (HTTP 401)\n", GhErr::NotLoggedIn),
            ("bez sítě", "error connecting to api.github.com\ncheck your internet connection or https://githubstatus.com", GhErr::Offline),
            ("dns", "Post \"https://api.github.com/graphql\": dial tcp: lookup api.github.com: no such host", GhErr::Offline),
            ("termín", "context deadline exceeded: i/o timeout", GhErr::Offline),
            ("jiná chyba první řádek", "\n  \nHTTP 403: Resource not accessible by integration\nvíc textu", GhErr::Other("HTTP 403: Resource not accessible by integration".into())),
            ("prázdný stderr", "", GhErr::Other(String::new())),
        ];
        for (name, stderr, want) in cases {
            assert_eq!(classify(stderr), want, "{name}");
        }
    }

    #[test]
    fn long_error_line_is_cut() {
        let GhErr::Other(s) = classify(&"x".repeat(900)) else { panic!("má být Other") };
        assert_eq!(s.chars().count(), 200);
    }
}
