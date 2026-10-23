//! Srozumitelné chyby: z `io::Error`, textu chyby (`os error 10048`, `(0x80070005)`), `serde_json`
//! a gitu udělá krátkou lidskou větu „co se stalo + co s tím“ (česky, při `ui.lang = en` anglicky).
//! Původní text (technický detail) jde vždy do logu, nikdy do UI. Rozpoznání je podle `ErrorKind`
//! a kódu Windows; anglický text se čte jen jako záloha pro chyby, které už jsou jen `String`.
//!
//! Použití na hranici příkazu: `.map_err(|e| errors::io(tr("Poznámku nejde uložit", "Can't save the note"), &e))`
//! → „Poznámku nejde uložit: soubor právě používá jiná aplikace. Zavři ji a zkus to znovu.“
//! Čisté jsou `kind_of_io`, `kind_of_text`, `kind_of_json_text` a `render`; zápis do logu `detail`
//! se v testech nedělá (testy nesahají na skutečný `%APPDATA%`).

use crate::i18n::tr;
use std::fmt::Display;

/// Druh chyby, jak ho uživatel potřebuje rozlišit (ne technická příčina).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    NotFound,
    Denied,
    Locked,
    DiskFull,
    PathTooLong,
    PortBusy,
    Refused,
    Timeout,
    NoNetwork,
    /// Neplatný JSON; řádek, pokud ho chyba uvádí.
    BadJson(Option<usize>),
    /// Git skončil chybou; návratový kód, pokud je znám.
    Git(Option<i32>),
    /// Program nejde spustit (chybí exe, špatný formát).
    NoExe,
    /// Git: nepřihlášeno k repozitáři.
    GitAuth,
    /// Git: větev se rozešla se vzdálenou, rychlé posunutí nejde.
    GitDiverged,
    /// Git: neuložené změny by operace přepsala.
    GitDirty,
    /// Git: složka není repozitář.
    NotRepo,
    Other,
}

/// Kód chyby Windows (`GetLastError` / `WSAGetLastError`) → druh.
fn kind_of_code(code: i32) -> Kind {
    match code {
        2 | 3 | 15 | 123 | 267 => Kind::NotFound,
        5 | 10013 => Kind::Denied,
        32 | 33 => Kind::Locked,
        39 | 112 => Kind::DiskFull,
        206 => Kind::PathTooLong,
        10048 => Kind::PortBusy,
        10061 => Kind::Refused,
        10060 | 1460 => Kind::Timeout,
        11001 | 11004 | 10051 | 10065 | 10050 | 10053 => Kind::NoNetwork,
        193 | 740 => Kind::NoExe,
        _ => Kind::Other,
    }
}

/// Druh podle `io::Error`: nejdřív kód Windows, pak `ErrorKind`.
pub(crate) fn kind_of_io(e: &std::io::Error) -> Kind {
    use std::io::ErrorKind as K;
    if let Some(k) = e.raw_os_error().map(kind_of_code).filter(|k| *k != Kind::Other) {
        return k;
    }
    match e.kind() {
        K::NotFound => Kind::NotFound,
        K::PermissionDenied => Kind::Denied,
        K::AddrInUse => Kind::PortBusy,
        K::ConnectionRefused => Kind::Refused,
        K::TimedOut => Kind::Timeout,
        K::StorageFull => Kind::DiskFull,
        K::NotConnected | K::AddrNotAvailable | K::NetworkUnreachable | K::HostUnreachable => Kind::NoNetwork,
        K::InvalidData | K::UnexpectedEof if e.get_ref().is_some_and(|i| i.is::<serde_json::Error>()) => Kind::BadJson(None),
        _ => Kind::Other,
    }
}

/// Číslo za `needle` v textu (`os error 10048` → 10048).
fn number_after(text: &str, needle: &str) -> Option<usize> {
    let rest = &text[text.find(needle)? + needle.len()..];
    let digits: String = rest.chars().skip_while(|c| !c.is_ascii_digit()).take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Řádek z textu chyby `serde_json` („… at line 3 column 5“).
pub(crate) fn json_line(text: &str) -> Option<usize> {
    number_after(text, " at line ").filter(|n| *n > 0)
}

/// Druh podle textu chyby: `(os error N)`, `(0x8007NNNN)`, tvar JSON chyby a nakonec pár anglických
/// frází (jen záloha — kde je k dispozici typ chyby, použij `kind_of_io`).
pub(crate) fn kind_of_text(text: &str) -> Kind {
    if let Some(code) = number_after(text, "os error ") {
        let k = kind_of_code(code as i32);
        if k != Kind::Other {
            return k;
        }
    }
    if let Some(h) = text.rfind("(0x8007").map(|i| &text[i + 7..]) {
        let hex: String = h.chars().take(4).collect();
        if let Ok(code) = u32::from_str_radix(&hex, 16) {
            let k = kind_of_code(code as i32);
            if k != Kind::Other {
                return k;
            }
        }
    }
    if text.contains(" at line ") && text.contains(" column ") {
        return Kind::BadJson(json_line(text));
    }
    let low = text.to_lowercase();
    let has = |s: &[&str]| s.iter().any(|p| low.contains(p));
    if has(&["address already in use", "only one usage of each socket address"]) {
        Kind::PortBusy
    } else if has(&["connection refused", "actively refused"]) {
        Kind::Refused
    } else if has(&["timed out", "timeout"]) {
        Kind::Timeout
    } else if has(&["dns error", "failed to lookup address", "could not resolve host", "no such host"]) {
        Kind::NoNetwork
    } else if has(&["used by another process", "sharing violation"]) {
        Kind::Locked
    } else if has(&["no space left", "not enough space", "disk full"]) {
        Kind::DiskFull
    } else if has(&["filename or extension is too long", "path too long"]) {
        Kind::PathTooLong
    } else if has(&["access is denied", "permission denied"]) {
        Kind::Denied
    } else if has(&["cannot find the file", "cannot find the path", "no such file"]) {
        Kind::NotFound
    } else {
        Kind::Other
    }
}

/// Věta „co se stalo“ (malým písmenem, aby šla za „Kontext: “) a rada „co s tím“.
fn reason_hint(k: Kind) -> (String, &'static str) {
    let s = |cs: &'static str, en: &'static str| tr(cs, en).to_string();
    match k {
        Kind::NotFound => (
            s("soubor nebo složka neexistuje", "the file or folder doesn't exist"),
            tr("Zkontroluj cestu — možná byla přesunutá nebo smazaná.", "Check the path — it may have been moved or deleted."),
        ),
        Kind::Denied => (
            s("k souboru nebo složce nemám přístup", "no access to the file or folder"),
            tr("Zkontroluj oprávnění, nebo zvol jiné místo.", "Check the permissions, or pick another location."),
        ),
        Kind::Locked => (
            s("soubor právě používá jiná aplikace", "another app is using the file"),
            tr("Zavři ji a zkus to znovu.", "Close it and try again."),
        ),
        Kind::DiskFull => (
            s("na disku není dost místa", "the disk is full"),
            tr("Uvolni místo a zkus to znovu.", "Free up some space and try again."),
        ),
        Kind::PathTooLong => (
            s("cesta je příliš dlouhá", "the path is too long"),
            tr("Zkrať ji, nebo přesuň soubor blíž ke kořeni disku.", "Shorten it, or move the file closer to the drive root."),
        ),
        Kind::PortBusy => (
            s("port už používá jiný program", "the port is already used by another program"),
            tr("Ukonči ho, nebo zvol jiný port.", "Close it, or choose another port."),
        ),
        Kind::Refused => (
            s("spojení bylo odmítnuto", "the connection was refused"),
            tr("Na druhé straně nic neposlouchá — spusť službu a zkus to znovu.", "Nothing is listening on the other side — start the service and try again."),
        ),
        Kind::Timeout => (
            s("vypršel čas čekání", "it took too long"),
            tr("Zkus to za chvíli znovu.", "Try again in a moment."),
        ),
        Kind::NoNetwork => (
            s("nejde se připojit k síti", "can't reach the network"),
            tr("Jsi online? Zkontroluj připojení a zkus to znovu.", "Are you online? Check the connection and try again."),
        ),
        Kind::BadJson(line) => (
            match line {
                Some(n) => trf!("soubor obsahuje chybu (řádek {n})", "the file contains an error (line {n})"),
                None => s("soubor obsahuje chybu", "the file contains an error"),
            },
            tr("Oprav ji v textovém editoru a zkus to znovu.", "Fix it in a text editor and try again."),
        ),
        Kind::Git(code) => (
            match code {
                Some(c) => trf!("git skončil s chybou (kód {c})", "git failed (exit code {c})"),
                None => s("git skončil s chybou", "git failed"),
            },
            tr("Podrobnosti jsou v logu.", "Details are in the log."),
        ),
        Kind::NoExe => (
            s("program nejde spustit", "the program can't be started"),
            tr("Zkontroluj, že je nainstalovaný a cesta k němu je správná.", "Check that it's installed and the path is correct."),
        ),
        Kind::GitAuth => (
            s("git se nepřihlásil k repozitáři", "git couldn't sign in to the repository"),
            tr("Zkontroluj přihlášení (Git Credential Manager nebo klíč SSH).", "Check your sign-in (Git Credential Manager or an SSH key)."),
        ),
        Kind::GitDiverged => (
            s("větev se rozešla se vzdálenou", "the branch has diverged from the remote"),
            tr("Rychlé posunutí nejde — sluč ji ručně.", "A fast-forward isn't possible — merge it by hand."),
        ),
        Kind::GitDirty => (
            s("neuložené změny by se přepsaly", "unsaved changes would be overwritten"),
            tr("Nejdřív je zapiš do commitu, nebo odlož.", "Commit or stash them first."),
        ),
        Kind::NotRepo => (
            s("složka není repozitář git", "the folder isn't a git repository"),
            tr("Zkontroluj cestu k projektu.", "Check the project path."),
        ),
        Kind::Other => (
            s("něco se nepovedlo", "something went wrong"),
            tr("Podrobnosti jsou v logu.", "Details are in the log."),
        ),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// Složí hlášku: „Kontext: důvod. Rada.“ (bez kontextu: „Důvod. Rada.“). Neznámá chyba nese jen
/// kontext + „něco se nepovedlo — podrobnosti jsou v logu“.
pub(crate) fn render(ctx: &str, k: Kind) -> String {
    let (reason, hint) = reason_hint(k);
    let ctx = ctx.trim().trim_end_matches(['.', ':']);
    if k == Kind::Other {
        return if ctx.is_empty() {
            capitalize(&format!("{reason} — {}.", lower_first(hint)))
        } else {
            format!("{ctx}: {reason} — {}.", lower_first(hint))
        };
    }
    if ctx.is_empty() {
        format!("{}. {hint}", capitalize(&reason))
    } else {
        format!("{ctx}: {reason}. {hint}")
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_lowercase().collect::<String>() + c.as_str()).unwrap_or_default().trim_end_matches('.').to_string()
}

/// Technický detail do logu (nikdy do UI). V testech se nezapisuje.
pub(crate) fn detail(ctx: &str, e: &dyn Display) {
    #[cfg(not(test))]
    crate::state::log(&format!("chyba: {ctx}: {e}"));
    #[cfg(test)]
    let _ = (ctx, e);
}

/// Hláška pro `io::Error` s kontextem akce; původní text do logu.
pub(crate) fn io(ctx: &str, e: &std::io::Error) -> String {
    detail(ctx, e);
    render(ctx, kind_of_io(e))
}

/// Jako `io`, ale pro neúspěšné spuštění programu: „nenalezeno“ znamená, že chybí program.
pub(crate) fn spawn(ctx: &str, e: &std::io::Error) -> String {
    detail(ctx, e);
    let k = match kind_of_io(e) {
        Kind::NotFound => Kind::NoExe,
        k => k,
    };
    render(ctx, k)
}

/// Hláška pro libovolnou chybu (text se čte jen jako záloha); původní text do logu.
pub(crate) fn any(ctx: &str, e: &dyn Display) -> String {
    detail(ctx, e);
    render(ctx, kind_of_text(&e.to_string()))
}

/// Hláška pro chybu `serde_json` (s číslem řádku); původní text do logu.
pub(crate) fn json(ctx: &str, e: &serde_json::Error) -> String {
    detail(ctx, e);
    let line = Some(e.line()).filter(|n| *n > 0);
    render(ctx, if e.is_io() { kind_of_text(&e.to_string()) } else { Kind::BadJson(line) })
}

/// Druh chyby gitu podle stderr (anglický text gitu je jediný zdroj) a kódu ukončení.
pub(crate) fn kind_of_git(code: Option<i32>, stderr: &str) -> Kind {
    let low = stderr.to_lowercase();
    let has = |s: &[&str]| s.iter().any(|p| low.contains(p));
    if has(&[
        "authentication failed", "could not read username", "terminal prompts disabled", "permission denied (publickey",
        "invalid username or password", "repository not found", "host key verification failed",
    ]) {
        Kind::GitAuth
    } else if has(&["not possible to fast-forward", "have diverged", "diverging branches"]) {
        Kind::GitDiverged
    } else if has(&["would be overwritten", "your local changes"]) {
        Kind::GitDirty
    } else if has(&["not a git repository"]) {
        Kind::NotRepo
    } else if has(&["failed to connect", "couldn't connect", "network is unreachable", "no route to host", "connection reset"]) {
        Kind::NoNetwork
    } else {
        match kind_of_text(stderr) {
            k @ (Kind::NoNetwork | Kind::Timeout | Kind::Refused | Kind::Locked | Kind::DiskFull | Kind::Denied) => k,
            _ => Kind::Git(code),
        }
    }
}

/// Hláška pro neúspěšný `git` (kód ukončení + stderr jen do logu).
pub(crate) fn git(ctx: &str, code: Option<i32>, stderr: &str) -> String {
    detail(ctx, &format!("git exit {code:?}: {}", stderr.trim()));
    render(ctx, kind_of_git(code, stderr))
}

/// Jako `spawn`, ale pro chybu, která už je jen text (`RunError::Spawn(String)`).
pub(crate) fn spawn_text(ctx: &str, e: &str) -> String {
    detail(ctx, &e);
    let k = match kind_of_text(e) {
        Kind::NotFound => Kind::NoExe,
        k => k,
    };
    render(ctx, k)
}

/// Druh síťové chyby: příznaky `reqwest` (timeout / connect) + text řetězu příčin.
pub(crate) fn kind_of_net(timeout: bool, connect: bool, chain: &str) -> Kind {
    if timeout {
        return Kind::Timeout;
    }
    match kind_of_text(chain) {
        Kind::Other if connect => Kind::NoNetwork,
        k => k,
    }
}

/// Hláška pro chybu `reqwest` (bez adresy — ta může nést souřadnice); řetěz příčin jde do logu.
pub(crate) fn net(ctx: &str, e: &reqwest::Error) -> String {
    let mut chain = e.to_string();
    let mut src = std::error::Error::source(e);
    while let Some(s) = src {
        chain.push_str(&format!(": {s}"));
        src = s.source();
    }
    detail(ctx, &chain);
    render(ctx, kind_of_net(e.is_timeout(), e.is_connect(), &chain))
}

/// „Kontext: text“ pro chybu, která už je věta (z jiného modulu); technicky vypadající text
/// převede na lidskou větu jako `any`.
pub(crate) fn prefixed(ctx: &str, text: &str) -> String {
    if looks_technical(text) { any(ctx, &text) } else { format!("{ctx}: {text}") }
}

/// Vypadá text jako surová technická chyba?
pub(crate) fn looks_technical(text: &str) -> bool {
    text.contains("os error")
        || text.contains("(0x8")
        || text.contains("stack backtrace")
        || text.contains("panicked at")
        || (text.trim_start().starts_with(['{', '[']) && text.contains('"'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;
    use std::io::{Error, ErrorKind};

    fn os(code: i32) -> Error {
        Error::from_raw_os_error(code)
    }

    #[test]
    fn io_kinds_table() {
        let t: Vec<(Error, Kind)> = vec![
            (os(2), Kind::NotFound),
            (os(3), Kind::NotFound),
            (os(5), Kind::Denied),
            (os(32), Kind::Locked),
            (os(33), Kind::Locked),
            (os(112), Kind::DiskFull),
            (os(206), Kind::PathTooLong),
            (os(10048), Kind::PortBusy),
            (os(10061), Kind::Refused),
            (os(10060), Kind::Timeout),
            (os(11001), Kind::NoNetwork),
            (os(193), Kind::NoExe),
            (Error::new(ErrorKind::NotFound, "x"), Kind::NotFound),
            (Error::new(ErrorKind::PermissionDenied, "x"), Kind::Denied),
            (Error::new(ErrorKind::AddrInUse, "x"), Kind::PortBusy),
            (Error::new(ErrorKind::ConnectionRefused, "x"), Kind::Refused),
            (Error::new(ErrorKind::TimedOut, "x"), Kind::Timeout),
            (Error::new(ErrorKind::StorageFull, "x"), Kind::DiskFull),
            (Error::other("něco"), Kind::Other),
            (os(1), Kind::Other),
        ];
        for (e, want) in t {
            assert_eq!(kind_of_io(&e), want, "{e:?}");
        }
    }

    #[test]
    fn text_kinds_table() {
        let t = [
            ("Only one usage of each socket address (os error 10048)", Kind::PortBusy),
            ("Přístup byl odepřen. (os error 5)", Kind::Denied),
            ("The system cannot find the file specified. (os error 2)", Kind::NotFound),
            ("The process cannot access the file (os error 32)", Kind::Locked),
            ("There is not enough space on the disk. (os error 112)", Kind::DiskFull),
            ("The filename or extension is too long. (os error 206)", Kind::PathTooLong),
            ("No connection could be made (os error 10061)", Kind::Refused),
            ("The system cannot find the file specified. (0x80070002)", Kind::NotFound),
            ("Access is denied. (0x80070005)", Kind::Denied),
            ("expected value at line 3 column 5", Kind::BadJson(Some(3))),
            ("trailing comma at line 12 column 1", Kind::BadJson(Some(12))),
            ("error sending request: dns error: failed to lookup address", Kind::NoNetwork),
            ("operation timed out", Kind::Timeout),
            ("Address already in use", Kind::PortBusy),
            ("nic zajímavého", Kind::Other),
            ("os error 1", Kind::Other),
        ];
        for (text, want) in t {
            assert_eq!(kind_of_text(text), want, "{text}");
        }
    }

    #[test]
    fn render_czech_and_english() {
        assert_eq!(
            render("Poznámku nejde uložit", Kind::Locked),
            "Poznámku nejde uložit: soubor právě používá jiná aplikace. Zavři ji a zkus to znovu."
        );
        assert_eq!(render("", Kind::DiskFull), "Na disku není dost místa. Uvolni místo a zkus to znovu.");
        assert_eq!(render("Uložení.", Kind::Other), "Uložení: něco se nepovedlo — podrobnosti jsou v logu.");
        assert_eq!(render("", Kind::Other), "Něco se nepovedlo — podrobnosti jsou v logu.");
        assert_eq!(
            render("Nastavení", Kind::BadJson(Some(7))),
            "Nastavení: soubor obsahuje chybu (řádek 7). Oprav ji v textovém editoru a zkus to znovu."
        );
        assert_eq!(render("Pull", Kind::Git(Some(128))), "Pull: git skončil s chybou (kód 128). Podrobnosti jsou v logu.");
        with_lang(Lang::En, || {
            assert_eq!(render("Can't save the note", Kind::Locked), "Can't save the note: another app is using the file. Close it and try again.");
            assert_eq!(render("", Kind::PortBusy), "The port is already used by another program. Close it, or choose another port.");
            assert_eq!(render("Save", Kind::Other), "Save: something went wrong — details are in the log.");
        });
    }

    #[test]
    fn every_kind_has_text_in_both_languages() {
        let kinds = [
            Kind::NotFound, Kind::Denied, Kind::Locked, Kind::DiskFull, Kind::PathTooLong, Kind::PortBusy, Kind::Refused,
            Kind::Timeout, Kind::NoNetwork, Kind::BadJson(None), Kind::BadJson(Some(2)), Kind::Git(None), Kind::NoExe, Kind::GitAuth, Kind::GitDiverged, Kind::GitDirty, Kind::NotRepo, Kind::Other,
        ];
        for lang in [Lang::Cs, Lang::En] {
            with_lang(lang, || {
                for k in kinds {
                    let r = render("X", k);
                    assert!(r.starts_with("X: "), "{r}");
                    assert!(!r.contains("os error") && !r.contains('{'), "{r}");
                }
            });
        }
    }

    #[test]
    fn spawn_not_found_means_missing_program() {
        assert!(spawn("Editor", &os(2)).contains("program nejde spustit"));
        assert!(spawn("Editor", &os(5)).contains("nemám přístup"));
    }

    #[test]
    fn json_error_has_line() {
        let e = serde_json::from_str::<serde_json::Value>("{\n  \"a\": 1,\n  \"b\": ,\n}").unwrap_err();
        let r = json("Nastavení", &e);
        assert!(r.contains("řádek 3"), "{r}");
        assert!(!r.contains("expected"), "{r}");
        assert_eq!(json_line("expected value at line 4 column 2"), Some(4));
        assert_eq!(json_line("bez řádku"), None);
    }

    #[test]
    fn io_and_any_use_the_type_or_the_text() {
        assert!(io("Zápis", &os(5)).contains("nemám přístup"));
        assert!(any("Port", &"Only one usage of each socket address (os error 10048)").contains("port už používá"));
        assert!(!any("Port", &"Only one usage of each socket address (os error 10048)").contains("10048"));
        assert_eq!(any("Zápis", &"cokoli"), "Zápis: něco se nepovedlo — podrobnosti jsou v logu.");
    }

    #[test]
    fn git_kinds_table() {
        let t = [
            ("fatal: Authentication failed for 'https://x'", Kind::GitAuth),
            ("fatal: Not possible to fast-forward, aborting.", Kind::GitDiverged),
            ("error: Your local changes to the following files would be overwritten by merge:", Kind::GitDirty),
            ("fatal: not a git repository (or any of the parent directories): .git", Kind::NotRepo),
            ("fatal: unable to access 'https://x': Failed to connect to x port 443", Kind::NoNetwork),
            ("fatal: unable to access: Could not resolve host: github.com", Kind::NoNetwork),
            ("ssh: connect to host github.com port 22: Connection timed out", Kind::Timeout),
            ("CONFLICT (content): Merge conflict", Kind::Git(Some(1))),
        ];
        for (err, want) in t {
            assert_eq!(kind_of_git(Some(1), err), want, "{err}");
        }
        assert!(spawn_text("Git", "program not found (os error 2)").contains("program nejde spustit"));
    }

    #[test]
    fn git_prefers_network_hint_over_exit_code() {
        assert!(git("Stažení", Some(128), "fatal: unable to access: Could not resolve host: github.com").contains("nejde se připojit"));
        assert!(git("Stažení", Some(1), "CONFLICT (content)").contains("kód 1"));
        assert!(git("Stažení", None, "").contains("git skončil s chybou."));
    }

    #[test]
    fn net_kinds_table() {
        assert_eq!(kind_of_net(true, false, "x"), Kind::Timeout);
        assert_eq!(kind_of_net(false, true, "error trying to connect: tcp connect error"), Kind::NoNetwork);
        assert_eq!(kind_of_net(false, true, "tcp connect error: No connection could be made (os error 10061)"), Kind::Refused);
        assert_eq!(kind_of_net(false, false, "dns error: failed to lookup address information"), Kind::NoNetwork);
        assert_eq!(kind_of_net(false, false, "body error"), Kind::Other);
    }

    #[test]
    fn prefixed_keeps_human_text() {
        assert_eq!(prefixed("Akce", "cesta neexistuje"), "Akce: cesta neexistuje");
        assert!(prefixed("Akce", "x (os error 2)").contains("neexistuje"));
    }

    #[test]
    fn technical_text_is_recognised() {
        assert!(looks_technical("{\"error\":\"x\"}"));
        assert!(looks_technical("thread panicked at src/main.rs"));
        assert!(!looks_technical("Port 5173 je obsazený."));
    }
}
