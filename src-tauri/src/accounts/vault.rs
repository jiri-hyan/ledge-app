//! Úschova přihlášení: **Windows Credential Manager** (Správce pověření → Obecná pověření,
//! `Ledge/accounts/<poskytovatel>`). Ukládá se jen záznam `Stored` (refresh token, Client ID, u Google
//! i „client secret“, jméno účtu) — nikdy do configu ani logu. Access token je jen v paměti.
//!
//! Credential Manager drží nejvýš 2 560 bajtů na pověření (`CRED_MAX_CREDENTIAL_BLOB_SIZE`);
//! refresh tokeny Microsoftu umí být dlouhé, proto se záznam dělí na díly (`chunks`, nejvýš
//! `MAX_PARTS`): `Ledge/accounts/microsoft`, `…/microsoft#2`, … Testy jdou proti `MemVault`
//! (nikdy na skutečný Správce pověření).

use super::Provider;
use serde::{Deserialize, Serialize};

/// Nejvíc bajtů v jednom pověření (Windows `CRED_MAX_CREDENTIAL_BLOB_SIZE` = 5 × 512).
pub const PART: usize = 2560;
/// Nejvíc dílů záznamu (≈ 10 KB).
pub const MAX_PARTS: usize = 4;

/// Co Ledge o přihlášení drží mimo paměť.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stored {
    pub v: u32,
    pub client_id: String,
    /// Google „Desktop app“: dostaneš ho s Client ID, u desktopových klientů není tajný (RFC 8252 §8.5);
    /// i tak jen tady. Microsoft (veřejný klient) žádný nemá.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub client_secret: String,
    pub refresh_token: String,
    /// udělená oprávnění (mezerami), jak je vrátil poskytovatel
    #[serde(default)]
    pub scope: String,
    /// e-mail / přihlašovací jméno účtu (pro „Přihlášen jako…“)
    #[serde(default)]
    pub account: String,
    /// zobrazované jméno
    #[serde(default)]
    pub name: String,
    /// Microsoft: tenant z configu v době přihlášení (`common`, `organizations`, …)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tenant: String,
    #[serde(default)]
    pub saved_ms: i64,
}

impl std::fmt::Debug for Stored {
    /// Bez tajemství — Debug se smí objevit v logu i v panice.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stored").field("account", &self.account).field("scope", &self.scope).field("refresh_token", &"•••").finish()
    }
}

pub fn target(p: Provider, part: usize) -> String {
    if part == 0 { format!("Ledge/accounts/{}", p.id()) } else { format!("Ledge/accounts/{}#{}", p.id(), part + 1) }
}

/// Rozdělí bajty na díly po `max` (prázdný vstup = jeden prázdný díl).
pub fn chunks(bytes: &[u8], max: usize) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return vec![bytes];
    }
    bytes.chunks(max).collect()
}

/// Úložiště tajemství (trait kvůli testům).
pub trait Vault: Send + Sync {
    fn read(&self, target: &str) -> Result<Option<Vec<u8>>, String>;
    fn write(&self, target: &str, data: &[u8]) -> Result<(), String>;
    fn delete(&self, target: &str) -> Result<(), String>;
}

/// Uloží záznam (díly; přebytečné staré díly smaže).
pub fn save(v: &dyn Vault, p: Provider, s: &Stored) -> Result<(), String> {
    let json = serde_json::to_vec(s).map_err(|e| e.to_string())?;
    let parts = chunks(&json, PART);
    if parts.len() > MAX_PARTS {
        return Err(crate::i18n::tr("Přihlášení je moc dlouhé na uložení ve Správci pověření.", "The sign-in is too long to store in Credential Manager.").into());
    }
    for (i, d) in parts.iter().enumerate() {
        v.write(&target(p, i), d)?;
    }
    for i in parts.len()..MAX_PARTS {
        v.delete(&target(p, i))?;
    }
    Ok(())
}

/// Načte záznam. Žádný / nečitelný / jiná verze = `None` (nečitelný se smaže, ať nestraší).
pub fn load(v: &dyn Vault, p: Provider) -> Result<Option<Stored>, String> {
    let mut all = Vec::new();
    for i in 0..MAX_PARTS {
        match v.read(&target(p, i))? {
            Some(d) => all.extend(d),
            None => break,
        }
    }
    if all.is_empty() {
        return Ok(None);
    }
    match serde_json::from_slice::<Stored>(&all) {
        Ok(s) if s.v == 1 && !s.refresh_token.is_empty() => Ok(Some(s)),
        _ => {
            remove(v, p)?;
            Ok(None)
        }
    }
}

pub fn remove(v: &dyn Vault, p: Provider) -> Result<(), String> {
    for i in 0..MAX_PARTS {
        v.delete(&target(p, i))?;
    }
    Ok(())
}

/// Skutečný Správce pověření Windows (obecná pověření, jen tento počítač).
pub struct WinCred;

#[cfg(windows)]
impl Vault for WinCred {
    fn read(&self, target: &str) -> Result<Option<Vec<u8>>, String> {
        use windows::Win32::Foundation::ERROR_NOT_FOUND;
        use windows::Win32::Security::Credentials::{CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW};
        use windows::core::HSTRING;
        let name = HSTRING::from(target);
        let mut out: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: `out` dostane ukazatel od systému, uvolní ho `CredFree`; blob se zkopíruje předtím
        match unsafe { CredReadW(&name, CRED_TYPE_GENERIC, None, &mut out) } {
            Ok(()) => {
                let data = unsafe {
                    let c = &*out;
                    let d = if c.CredentialBlob.is_null() { Vec::new() } else { std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec() };
                    CredFree(out as *const _);
                    d
                };
                Ok(Some(data))
            }
            Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(None),
            Err(e) => Err(trf!("Správce pověření nejde přečíst ({}).", "Credential Manager can't be read ({}).", e.code().0)),
        }
    }

    fn write(&self, target: &str, data: &[u8]) -> Result<(), String> {
        use windows::Win32::Security::Credentials::{CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredWriteW};
        use windows::core::PWSTR;
        let mut name: Vec<u16> = target.encode_utf16().chain([0]).collect();
        let mut user: Vec<u16> = "Ledge".encode_utf16().chain([0]).collect();
        let mut blob = data.to_vec();
        let cred = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(name.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        // SAFETY: všechny ukazatele míří do lokálních bufferů, které žijí do konce volání
        let res = unsafe { CredWriteW(&cred, 0) };
        blob.iter_mut().for_each(|b| *b = 0);
        res.map_err(|e| trf!("Do Správce pověření nejde zapsat ({}).", "Can't write to Credential Manager ({}).", e.code().0))
    }

    fn delete(&self, target: &str) -> Result<(), String> {
        use windows::Win32::Foundation::ERROR_NOT_FOUND;
        use windows::Win32::Security::Credentials::{CRED_TYPE_GENERIC, CredDeleteW};
        use windows::core::HSTRING;
        match unsafe { CredDeleteW(&HSTRING::from(target), CRED_TYPE_GENERIC, None) } {
            Ok(()) => Ok(()),
            Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
            Err(e) => Err(trf!("Ze Správce pověření nejde mazat ({}).", "Can't delete from Credential Manager ({}).", e.code().0)),
        }
    }
}

/// Úschova v paměti (testy).
#[cfg(test)]
#[derive(Default)]
pub struct MemVault(pub parking_lot::Mutex<std::collections::HashMap<String, Vec<u8>>>);

#[cfg(test)]
impl Vault for MemVault {
    fn read(&self, t: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(self.0.lock().get(t).cloned())
    }
    fn write(&self, t: &str, d: &[u8]) -> Result<(), String> {
        assert!(d.len() <= PART, "díl přes limit Credential Manageru");
        self.0.lock().insert(t.into(), d.to_vec());
        Ok(())
    }
    fn delete(&self, t: &str) -> Result<(), String> {
        self.0.lock().remove(t);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(token_len: usize) -> Stored {
        Stored {
            v: 1,
            client_id: "11111111-2222-3333-4444-555555555555".into(),
            client_secret: String::new(),
            refresh_token: "R".repeat(token_len),
            scope: "offline_access User.Read".into(),
            account: "jana@example.com".into(),
            name: "Jana".into(),
            tenant: "common".into(),
            saved_ms: 1,
        }
    }

    #[test]
    fn roundtrip_short_and_long_tokens() {
        for len in [10, PART - 300, PART + 10, 3 * PART] {
            let v = MemVault::default();
            let s = rec(len);
            save(&v, Provider::Microsoft, &s).unwrap();
            assert_eq!(load(&v, Provider::Microsoft).unwrap().as_ref(), Some(&s), "{len}");
            // jiný poskytovatel nic nevidí
            assert!(load(&v, Provider::Google).unwrap().is_none());
        }
    }

    #[test]
    fn shorter_save_removes_old_parts_and_remove_clears() {
        let v = MemVault::default();
        save(&v, Provider::Microsoft, &rec(3 * PART)).unwrap();
        assert!(v.0.lock().contains_key("Ledge/accounts/microsoft#3"));
        save(&v, Provider::Microsoft, &rec(5)).unwrap();
        assert_eq!(v.0.lock().len(), 1);
        assert_eq!(load(&v, Provider::Microsoft).unwrap().unwrap().refresh_token, "RRRRR");
        remove(&v, Provider::Microsoft).unwrap();
        assert!(v.0.lock().is_empty());
        assert!(save(&v, Provider::Microsoft, &rec(MAX_PARTS * PART)).is_err(), "nad strop dílů");
    }

    #[test]
    fn unreadable_record_is_dropped() {
        let v = MemVault::default();
        for bad in [&b"{"[..], br#"{"v":2,"clientId":"x","refreshToken":"y"}"#, br#"{"v":1,"clientId":"x","refreshToken":""}"#] {
            v.write(&target(Provider::Google, 0), bad).unwrap();
            assert!(load(&v, Provider::Google).unwrap().is_none());
            assert!(v.0.lock().is_empty(), "nečitelný záznam se smaže");
        }
    }

    #[test]
    fn debug_hides_secrets() {
        let mut s = rec(40);
        s.client_secret = "GOCSPX-tajne".into();
        let d = format!("{s:?}");
        assert!(!d.contains("RRRR") && !d.contains("GOCSPX"), "{d}");
        assert_eq!(chunks(b"", 4).len(), 1);
        assert_eq!(chunks(b"abcdefghi", 4).len(), 3);
    }
}
