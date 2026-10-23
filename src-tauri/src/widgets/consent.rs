//! Souhlas se spouštěním widgetu. Jako Script Commands: otisk = SHA-256 cesty, obsahu (u programu
//! i hlavičky `.exe.ledge`), druhu a argumentů — změna souboru = nový otisk = widget se zastaví a
//! v kartě se znovu zeptá s ukázkou začátku skriptu. Na rozdíl od skriptů si pamatuje JEDEN otisk na
//! soubor (`widgets-consent.json` v `%LOCALAPPDATA%\Ledge`, mimo config — nepřinese ho sync ani
//! import), takže umí říct „nový“ × „změněný“ a soubor nikdy nenaroste. Čisté kromě `Store`.

use super::meta::Kind;
use crate::i18n::tr;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 256;

/// Proč se ptát.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    Granted,
    /// soubor Ledge ještě nikdy nespustil
    New,
    /// soubor se od souhlasu změnil
    Changed,
}

/// Politika: uložený otisk pro soubor × otisk teď.
pub fn policy(stored: Option<&str>, current: &str) -> Consent {
    match stored {
        Some(s) if s == current => Consent::Granted,
        Some(_) => Consent::Changed,
        None => Consent::New,
    }
}

/// Klíč souboru v úložišti (cesta malými).
pub fn key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

/// Otisk: prefix `ledge-widget` (nepotká se s akcí ani skriptem), cesta, obsah, druh, argumenty.
pub fn fingerprint(path: &Path, content_sha: &str, kind: Kind, args: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for part in ["ledge-widget", &key(path), content_sha, kind.key()] {
        h.update(part.as_bytes());
        h.update([0u8]);
    }
    for a in args {
        h.update(a.as_bytes());
        h.update([1u8]);
    }
    crate::scripts::run::sha256_hex(h)
}

/// Co člověk potvrzuje (UI musí poslat přesně tohle zpátky): druh, cesta, argumenty, začátek otisku.
pub fn confirm_text(kind: Kind, path: &Path, args: &[String], content_sha: &str) -> String {
    let mut s = format!("{}: {}", kind.name(), path.display());
    for a in args {
        s.push(' ');
        s.push_str(&crate::actions::quote_cmd(a));
    }
    let short = content_sha.get(..12).unwrap_or(content_sha);
    s.push_str(&format!(" · SHA-256 {short}…"));
    s
}

/// Uložené souhlasy: soubor → otisk.
pub struct Store {
    path: PathBuf,
    map: Option<BTreeMap<String, String>>,
}

impl Store {
    pub const fn new() -> Self {
        Store { path: PathBuf::new(), map: None }
    }

    /// Jiný soubor (testy — nikdy `%LOCALAPPDATA%`).
    #[cfg(test)]
    pub fn at(path: PathBuf) -> Self {
        Store { path, map: None }
    }

    fn file(&self) -> PathBuf {
        if self.path.as_os_str().is_empty() { crate::actionrun::local_dir().join("widgets-consent.json") } else { self.path.clone() }
    }

    fn loaded(&mut self) -> &mut BTreeMap<String, String> {
        let path = self.file();
        self.map.get_or_insert_with(|| std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default())
    }

    pub fn get(&mut self, path: &Path) -> Option<String> {
        self.loaded().get(&key(path)).cloned()
    }

    pub fn check(&mut self, path: &Path, fp: &str) -> Consent {
        let stored = self.get(path);
        policy(stored.as_deref(), fp)
    }

    /// Uloží souhlas (přepíše starší otisk téhož souboru; atomicky přes dočasný soubor).
    pub fn grant(&mut self, path: &Path, fp: &str) -> Result<(), String> {
        let file = self.file();
        let k = key(path);
        let map = self.loaded();
        if map.get(&k).map(String::as_str) == Some(fp) {
            return Ok(());
        }
        map.insert(k.clone(), fp.to_string());
        while map.len() > MAX_ENTRIES {
            let Some(old) = map.keys().find(|x| **x != k).cloned() else { break };
            map.remove(&old);
        }
        let text = serde_json::to_string_pretty(&*map).map_err(|e| crate::errors::json(tr("Souhlas nejde uložit", "Can't save the consent"), &e))?;
        let fail = |e: std::io::Error| crate::errors::io(tr("Souhlas nejde uložit", "Can't save the consent"), &e);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(fail)?;
        }
        let tmp = file.with_extension("tmp");
        std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &file)).map_err(fail)
    }
}

/// Věta k potvrzení podle důvodu.
pub fn reason_text(c: Consent) -> &'static str {
    match c {
        Consent::Changed => tr(
            "Skript se od posledního povolení změnil — Ledge ho zastavil. Zkontroluj, co dělá, a povol ho znovu.",
            "The script changed since you last allowed it — Ledge stopped it. Check what it does and allow it again.",
        ),
        _ => tr(
            "Widget je cizí kód, který Ledge pravidelně spouští bez okna. Povol ho, jen když víš, co dělá. Souhlas platí pro tenhle obsah souboru.",
            "A widget is foreign code that Ledge runs regularly without a window. Allow it only if you know what it does. The consent covers this content of the file.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_table() {
        let cases: &[(Option<&str>, &str, Consent)] = &[(None, "a", Consent::New), (Some("a"), "a", Consent::Granted), (Some("a"), "b", Consent::Changed)];
        for (stored, cur, want) in cases {
            assert_eq!(policy(*stored, cur), *want, "{stored:?} {cur}");
        }
    }

    #[test]
    fn fingerprint_changes_with_content_kind_args() {
        let p = Path::new(r"C:\w\a.ps1");
        let base = fingerprint(p, "abc", Kind::Powershell, &[]);
        assert_eq!(base, fingerprint(Path::new(r"C:\W\A.PS1"), "abc", Kind::Powershell, &[]));
        assert_ne!(base, fingerprint(p, "abd", Kind::Powershell, &[]));
        assert_ne!(base, fingerprint(p, "abc", Kind::Exe, &[]));
        assert_ne!(base, fingerprint(p, "abc", Kind::Powershell, &["x".into()]));
        // s otiskem skriptu (Script Commands) se nepotká
        assert_ne!(base, crate::scripts::run::fingerprint(p, "abc", crate::scripts::meta::Mode::Output, &[]));
        assert_eq!(confirm_text(Kind::Powershell, p, &["a b".into()], "0123456789abcdef"), r#"PowerShell: C:\w\a.ps1 "a b" · SHA-256 0123456789ab…"#);
    }

    #[test]
    fn store_new_changed_granted_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub").join("w.json");
        let p = Path::new(r"C:\w\a.ps1");
        let mut s = Store::at(file.clone());
        assert_eq!(s.check(p, "fp1"), Consent::New);
        s.grant(p, "fp1").unwrap();
        assert_eq!(s.check(p, "fp1"), Consent::Granted);
        assert_eq!(s.check(p, "fp2"), Consent::Changed);
        // restart: ze souboru
        let mut again = Store::at(file.clone());
        assert_eq!(again.check(Path::new(r"C:\W\A.ps1"), "fp1"), Consent::Granted);
        // nový souhlas přepíše starý otisk téhož souboru (jeden na soubor)
        again.grant(p, "fp2").unwrap();
        assert_eq!(again.check(p, "fp1"), Consent::Changed);
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(text.matches("fp").count(), 1, "{text}");
    }

    #[test]
    fn store_cap_keeps_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::at(dir.path().join("w.json"));
        for i in 0..MAX_ENTRIES {
            s.grant(Path::new(&format!(r"C:\w\{i:03}.ps1")), "x").unwrap();
        }
        let first = Path::new(r"C:\w\000-a.ps1");
        s.grant(first, "y").unwrap();
        assert_eq!(s.check(first, "y"), Consent::Granted);
        assert_eq!(s.loaded().len(), MAX_ENTRIES);
    }
}
