//! Co se do historie schránky NEuloží — čisté funkce nad tím, co vrstva WinAPI (`win.rs`) ze schránky
//! přečetla. Pořadí: nejdřív formáty a zdrojová aplikace (`pre_verdict`, text se vůbec nečte), pak
//! text (`text_verdict`). Všechno je tabulkově otestované; žádné I/O.
//!
//! Formáty (registrované jménem, viz dokumentace Microsoftu „Clipboard Formats“ → „Cloud Clipboard
//! and Clipboard History Formats“):
//! - `ExcludeClipboardContentFromMonitorProcessing` — přítomnost = „nemonitorovat“ (správci hesel),
//! - `Clipboard Viewer Ignore` — starší značka téhož (KeePass, některé správce hesel),
//! - `CanIncludeInClipboardHistory` = DWORD 0 — „nepatří do historie“ (Chromium/Edge u hesel),
//! - `CanUploadToCloudClipboard` = DWORD 0 — „neposílat do cloudu“; bereme přísněji i pro lokální
//!   historii (soukromí napřed; aplikace ho dávají hlavně k citlivým datům).
//!
//! DWORD formát, který je přítomný, ale nejde přečíst, se počítá jako 0 (selhat do bezpečí).

use regex::Regex;
use std::sync::OnceLock;

/// `CF_UNICODETEXT` — jediný formát, který v první verzi ukládáme.
pub const CF_UNICODETEXT: u32 = 13;
/// Nejdelší uložený text (bajty UTF-8). Delší se NEuloží vůbec (žádné tiché oříznutí).
pub const MAX_TEXT_BYTES: usize = 64 * 1024;

pub const EXCLUDE_MONITOR: &str = "ExcludeClipboardContentFromMonitorProcessing";
pub const VIEWER_IGNORE: &str = "Clipboard Viewer Ignore";
pub const CAN_HISTORY: &str = "CanIncludeInClipboardHistory";
pub const CAN_CLOUD: &str = "CanUploadToCloudClipboard";

/// Registrované formáty, u kterých vrstva WinAPI čte hodnotu DWORD.
pub const DWORD_FORMATS: [&str; 2] = [CAN_HISTORY, CAN_CLOUD];

/// Jeden formát na schránce: id, jméno (jen registrované formáty ≥ 0xC000), u `DWORD_FORMATS` hodnota.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fmt {
    pub id: u32,
    pub name: Option<String>,
    pub dword: Option<u32>,
}

/// Proč se obsah neuložil (do logu nic — jen pro testy a případné ladění).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// značka formátem (jméno formátu)
    Marked(&'static str),
    /// zdrojová aplikace je v `excludeApps`
    App,
    /// na schránce není text (obrázek, soubory…)
    NoText,
    Empty,
    Whitespace,
    TooLong,
    Secret,
}

fn named<'a>(fmts: &'a [Fmt], name: &str) -> Option<&'a Fmt> {
    fmts.iter().find(|f| f.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

/// Značky „neukládat“ ve formátech schránky.
pub fn formats_verdict(fmts: &[Fmt]) -> Option<Skip> {
    for name in [EXCLUDE_MONITOR, VIEWER_IGNORE] {
        if named(fmts, name).is_some() {
            return Some(Skip::Marked(name));
        }
    }
    for name in DWORD_FORMATS {
        if let Some(f) = named(fmts, name)
            && f.dword.unwrap_or(0) == 0
        {
            return Some(Skip::Marked(name));
        }
    }
    None
}

/// Jméno exe bez cesty a bez `.exe`, malými písmeny (`C:\x\KeePassXC.exe` → `keepassxc`).
pub fn exe_key(s: &str) -> String {
    let base = s.trim().rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
    base.strip_suffix(".exe").map(str::to_string).unwrap_or(base)
}

/// Je zdrojová aplikace (exe vlastníka schránky) na seznamu vyloučených? Neznámý vlastník = ne.
pub fn app_excluded(owner_exe: Option<&str>, list: &[String]) -> bool {
    let Some(exe) = owner_exe.map(exe_key).filter(|k| !k.is_empty()) else { return false };
    list.iter().any(|x| exe_key(x) == exe)
}

/// Rozhodnutí PŘED čtením textu: formáty, zdrojová aplikace, je vůbec text.
pub fn pre_verdict(fmts: &[Fmt], owner_exe: Option<&str>, exclude_apps: &[String]) -> Option<Skip> {
    if let Some(s) = formats_verdict(fmts) {
        return Some(s);
    }
    if app_excluded(owner_exe, exclude_apps) {
        return Some(Skip::App);
    }
    if !fmts.iter().any(|f| f.id == CF_UNICODETEXT) {
        return Some(Skip::NoText);
    }
    None
}

/// Rozhodnutí nad přečteným textem.
pub fn text_verdict(text: &str, skip_secrets: bool) -> Option<Skip> {
    if text.is_empty() {
        return Some(Skip::Empty);
    }
    if text.trim().is_empty() {
        return Some(Skip::Whitespace);
    }
    if text.len() > MAX_TEXT_BYTES {
        return Some(Skip::TooLong);
    }
    if skip_secrets && looks_secret(text) {
        return Some(Skip::Secret);
    }
    None
}

fn secret_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"\bsk-[A-Za-z0-9_\-]{20,}",                                   // OpenAI, Anthropic (sk-ant-…)
            r"|\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{30,}",                // GitHub
            r"|\bgithub_pat_[A-Za-z0-9_]{30,}",                            // GitHub fine-grained
            r"|\bglpat-[A-Za-z0-9_\-]{20,}",                               // GitLab
            r"|\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",                             // AWS access key id
            r"|\bAIza[0-9A-Za-z_\-]{35}",                                  // Google API key
            r"|\bxox[abprs]-[A-Za-z0-9\-]{10,}",                           // Slack
            r"|\b(?:sk|rk)_live_[A-Za-z0-9]{20,}",                         // Stripe
            r"|\beyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}", // JWT
            r"|-----BEGIN [A-Z ]*PRIVATE KEY-----",                        // PEM klíč
        ))
        .expect("secret regex")
    })
}

/// Vypadá text jako (nebo obsahuje) přístupový token / klíč? Heuristika nad známými tvary — heslo
/// bez tvaru nepozná (na to jsou značky formátů a `excludeApps`).
pub fn looks_secret(text: &str) -> bool {
    secret_re().is_match(text)
}

/// Text z UTF-16 bufferu schránky do prvního NUL. `None` = NUL v prvních `MAX_TEXT_BYTES + 1`
/// jednotkách není → text je delší než limit (UTF-8 má na jednotku UTF-16 aspoň 1 bajt), nečte se celý.
pub fn utf16_text(units: &[u16]) -> Result<String, Skip> {
    let cap = units.len().min(MAX_TEXT_BYTES + 1);
    match units[..cap].iter().position(|&u| u == 0) {
        Some(n) => Ok(String::from_utf16_lossy(&units[..n])),
        None if units.len() <= MAX_TEXT_BYTES => Ok(String::from_utf16_lossy(units)),
        None => Err(Skip::TooLong),
    }
}

/// Vlastní zápis Ledge: text, který právě zapisujeme, a pořadové číslo schránky po zápisu
/// (`GetClipboardSequenceNumber`). Listener ho přeskočí, aby vložení z historie nevytvořilo nový záznam.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Own {
    pub text: String,
    /// `None` = zápis právě běží (listener mohl událost dostat dřív, než ho známe)
    pub seq: Option<u32>,
}

/// Je změna schránky (`seq`, `text`) náš vlastní zápis?
pub fn is_own(own: Option<&Own>, seq: u32, text: &str) -> bool {
    match own {
        None => false,
        Some(Own { seq: Some(s), .. }) => *s == seq,
        Some(Own { seq: None, text: t }) => t == text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(id: u32, name: Option<&str>, dword: Option<u32>) -> Fmt {
        Fmt { id, name: name.map(str::to_string), dword }
    }
    const TEXT: Fmt = Fmt { id: CF_UNICODETEXT, name: None, dword: None };

    #[test]
    fn formats_table() {
        // (formáty, očekávané rozhodnutí)
        let cases: Vec<(Vec<Fmt>, Option<Skip>)> = vec![
            (vec![TEXT], None),
            (vec![TEXT, f(0xC100, Some(EXCLUDE_MONITOR), None)], Some(Skip::Marked(EXCLUDE_MONITOR))),
            (vec![TEXT, f(0xC100, Some("excludeclipboardcontentfrommonitorprocessing"), None)], Some(Skip::Marked(EXCLUDE_MONITOR))),
            (vec![TEXT, f(0xC101, Some(VIEWER_IGNORE), None)], Some(Skip::Marked(VIEWER_IGNORE))),
            (vec![TEXT, f(0xC102, Some(CAN_HISTORY), Some(0))], Some(Skip::Marked(CAN_HISTORY))),
            (vec![TEXT, f(0xC102, Some(CAN_HISTORY), Some(1))], None),
            // přítomný, ale nečitelný DWORD = 0 (selhat do bezpečí)
            (vec![TEXT, f(0xC102, Some(CAN_HISTORY), None)], Some(Skip::Marked(CAN_HISTORY))),
            (vec![TEXT, f(0xC103, Some(CAN_CLOUD), Some(0))], Some(Skip::Marked(CAN_CLOUD))),
            (vec![TEXT, f(0xC103, Some(CAN_CLOUD), Some(1))], None),
            (vec![TEXT, f(0xC104, Some("HTML Format"), None), f(0xC105, Some("Rich Text Format"), None)], None),
            // Chromium u hesla: všechny tři najednou
            (vec![TEXT, f(0xC100, Some(EXCLUDE_MONITOR), None), f(0xC102, Some(CAN_HISTORY), Some(0)), f(0xC103, Some(CAN_CLOUD), Some(0))], Some(Skip::Marked(EXCLUDE_MONITOR))),
        ];
        for (fmts, want) in cases {
            assert_eq!(formats_verdict(&fmts), want, "{fmts:?}");
        }
    }

    #[test]
    fn pre_verdict_table() {
        let apps: Vec<String> = ["KeePass.exe", "KeePassXC", "1Password.exe", "Bitwarden.exe"].map(String::from).to_vec();
        let cases: Vec<(Vec<Fmt>, Option<&str>, Option<Skip>)> = vec![
            (vec![TEXT], None, None),
            (vec![TEXT], Some(r"C:\Program Files\KeePassXC\KeePassXC.exe"), Some(Skip::App)),
            (vec![TEXT], Some("keepass.EXE"), Some(Skip::App)),
            (vec![TEXT], Some(r"C:\Windows\notepad.exe"), None),
            (vec![TEXT], Some(""), None),
            (vec![f(2, None, None)], None, Some(Skip::NoText)), // CF_BITMAP
            (vec![f(15, None, None)], None, Some(Skip::NoText)), // CF_HDROP
            (vec![], None, Some(Skip::NoText)),
            // značka vyhraje nad vším ostatním
            (vec![f(0xC100, Some(EXCLUDE_MONITOR), None)], Some("notepad.exe"), Some(Skip::Marked(EXCLUDE_MONITOR))),
        ];
        for (fmts, owner, want) in cases {
            assert_eq!(pre_verdict(&fmts, owner, &apps), want, "{fmts:?} {owner:?}");
        }
        assert_eq!(pre_verdict(&[TEXT], Some("KeePass.exe"), &[]), None, "prázdný seznam = nic nevylučuje");
    }

    #[test]
    fn exe_key_shapes() {
        for (inp, want) in [(r"C:\a\KeePass.exe", "keepass"), ("Bitwarden", "bitwarden"), ("  1Password.EXE ", "1password"), ("C:/x/y.exe", "y"), ("", "")] {
            assert_eq!(exe_key(inp), want, "{inp}");
        }
    }

    #[test]
    fn text_table() {
        let long = "a".repeat(MAX_TEXT_BYTES + 1);
        let edge = "a".repeat(MAX_TEXT_BYTES);
        let cases: Vec<(&str, bool, Option<Skip>)> = vec![
            ("ahoj", true, None),
            ("", true, Some(Skip::Empty)),
            ("   \r\n\t ", true, Some(Skip::Whitespace)),
            ("\u{3000}", true, Some(Skip::Whitespace)),
            (&long, true, Some(Skip::TooLong)),
            (&edge, true, None),
            ("sk-ant-api03-AbCdEfGhIjKlMnOpQrStUv", true, Some(Skip::Secret)),
            ("sk-ant-api03-AbCdEfGhIjKlMnOpQrStUv", false, None),
            ("klíč: sk-proj-0123456789abcdefghijKLMN konec", true, Some(Skip::Secret)),
            ("ghp_0123456789abcdefghijABCDEFGHIJ0123", true, Some(Skip::Secret)),
            ("github_pat_11ABCDEFG0123456789_abcdefghijklmnopqrstuvwxyz", true, Some(Skip::Secret)),
            ("AKIAIOSFODNN7EXAMPLE", true, Some(Skip::Secret)),
            ("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U", true, Some(Skip::Secret)),
            ("-----BEGIN OPENSSH PRIVATE KEY-----\nb3Blbn…", true, Some(Skip::Secret)),
            ("xoxb-1234567890-abcdefghij", true, Some(Skip::Secret)),
            // ne-tajemství podobného tvaru
            ("task-runner-configuration-file-name", true, None),
            ("sk-short", true, None),
            ("AKIA je zkratka", true, None),
            ("git push origin main", true, None),
            ("eyJ.eyJ.x", true, None),
        ];
        for (text, secrets, want) in cases {
            assert_eq!(text_verdict(text, secrets), want, "{}", text.chars().take(40).collect::<String>());
        }
    }

    #[test]
    fn utf16_reading() {
        let w = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
        let mut a = w("ahoj");
        a.extend([0, 0x41, 0x42]); // za NUL smetí
        assert_eq!(utf16_text(&a).unwrap(), "ahoj");
        assert_eq!(utf16_text(&w("bez nul")).unwrap(), "bez nul");
        assert_eq!(utf16_text(&[]).unwrap(), "");
        let big = vec![0x61u16; MAX_TEXT_BYTES + 5];
        assert_eq!(utf16_text(&big), Err(Skip::TooLong));
        let mut ok = vec![0x61u16; 10];
        ok.push(0);
        ok.extend(vec![0x61u16; MAX_TEXT_BYTES + 5]);
        assert_eq!(utf16_text(&ok).unwrap().len(), 10, "NUL brzo = krátký text i ve velkém bloku");
    }

    #[test]
    fn own_write_table() {
        let pending = Own { text: "x".into(), seq: None };
        let done = Own { text: "x".into(), seq: Some(7) };
        let cases: Vec<(Option<&Own>, u32, &str, bool)> = vec![
            (None, 7, "x", false),
            (Some(&done), 7, "x", true),
            (Some(&done), 7, "y", true), // seq rozhoduje (text mohl normalizovat zdroj)
            (Some(&done), 8, "x", false), // někdo zapsal stejný text znovu = nový záznam
            (Some(&pending), 3, "x", true),
            (Some(&pending), 3, "y", false),
        ];
        for (own, seq, text, want) in cases {
            assert_eq!(is_own(own, seq, text), want, "{own:?} {seq} {text}");
        }
    }
}
