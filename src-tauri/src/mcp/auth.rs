//! Klíč MCP: 32 náhodných bajtů (systémový CSPRNG, `accounts::pkce::random`) jako base64url, uložený
//! ve Správci pověření Windows (`Ledge/mcp`, stejný kód jako účty — `accounts::vault`). Nikdy do configu,
//! logu ani chybových hlášek. Most ho drží v paměti (cache), proxy `ledge.exe --mcp` si ho čte sám.

use crate::accounts::pkce;
use crate::accounts::vault::Vault;
use parking_lot::Mutex;

/// Jméno pověření ve Správci pověření.
pub const TARGET: &str = "Ledge/mcp";
/// Bajtů náhody v klíči (base64url = 43 znaků).
pub const KEY_BYTES: usize = 32;

/// Úložiště klíče + cache v paměti.
pub struct Keys {
    vault: Box<dyn Vault>,
    target: &'static str,
    cache: Mutex<Option<String>>,
}

impl Keys {
    pub fn new(vault: Box<dyn Vault>) -> Self {
        Self::for_target(vault, TARGET)
    }

    /// Klíč pod jiným jménem pověření (`hooksetup::HOOK_TARGET`).
    pub fn for_target(vault: Box<dyn Vault>, target: &'static str) -> Self {
        Self { vault, target, cache: Mutex::new(None) }
    }

    /// Klíč, je-li (cache, jinak Správce pověření). Nečitelný záznam = žádný.
    pub fn get(&self) -> Option<String> {
        let mut c = self.cache.lock();
        if c.is_none() {
            *c = read_at(self.vault.as_ref(), self.target);
        }
        c.clone()
    }

    /// Klíč, a když není, vytvoří nový (první zapnutí). Chyba čtení úložiště (přechodná, zamčené pověření)
    /// není „klíč chybí“: vrátí se chyba a stávající klíč se nepřepíše. Nový vzniká jen při `Ok(None)`
    /// nebo nepoužitelném záznamu.
    pub fn ensure(&self) -> Result<String, String> {
        let cached = self.cache.lock().clone();
        if let Some(k) = cached {
            return Ok(k);
        }
        match try_read_at(self.vault.as_ref(), self.target)? {
            Some(k) => {
                *self.cache.lock() = Some(k.clone());
                Ok(k)
            }
            None => self.rotate(),
        }
    }

    /// Nový klíč (starý přestane platit hned — i pro připojené klienty).
    pub fn rotate(&self) -> Result<String, String> {
        let k = fresh()?;
        self.vault.write(self.target, k.as_bytes())?;
        *self.cache.lock() = Some(k.clone());
        Ok(k)
    }

    /// Hlavička `Authorization` nese platný klíč? Bez klíče v úložišti nepustí nikoho.
    pub fn check(&self, authorization: Option<&str>) -> bool {
        let (Some(given), Some(key)) = (authorization.and_then(bearer), self.get()) else { return false };
        pkce::same(given, &key)
    }
}

/// Přečte klíč z úložiště (jen platný tvar).
pub fn read(v: &dyn Vault) -> Option<String> {
    read_at(v, TARGET)
}

fn read_at(v: &dyn Vault, target: &str) -> Option<String> {
    try_read_at(v, target).ok().flatten()
}

/// Jako `read_at`, ale chybu úložiště nezaměňuje za chybějící klíč. Záznam nepoužitelného tvaru je `Ok(None)`.
fn try_read_at(v: &dyn Vault, target: &str) -> Result<Option<String>, String> {
    let Some(bytes) = v.read(target)? else { return Ok(None) };
    Ok(String::from_utf8(bytes).ok().filter(|k| valid(k)))
}

/// Tvar klíče: base64url, 43 znaků (32 B).
pub fn valid(k: &str) -> bool {
    k.len() == 43 && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Nový náhodný klíč.
pub fn fresh() -> Result<String, String> {
    Ok(pkce::b64url(&pkce::random(KEY_BYTES)?))
}

/// Token z `Authorization: Bearer <token>` (schéma bez ohledu na velikost písmen).
pub fn bearer(h: &str) -> Option<&str> {
    let h = h.trim();
    let (scheme, rest) = h.split_once(' ')?;
    let t = rest.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !t.is_empty()).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::vault::MemVault;

    #[test]
    fn bearer_table() {
        for (h, want) in [
            ("Bearer abc", Some("abc")),
            ("bearer  abc ", Some("abc")),
            ("BEARER abc", Some("abc")),
            ("Basic abc", None),
            ("Bearer", None),
            ("Bearer ", None),
            ("abc", None),
        ] {
            assert_eq!(bearer(h), want, "{h}");
        }
    }

    #[test]
    fn ensure_rotate_check() {
        let k = Keys::new(Box::new(MemVault::default()));
        assert!(k.get().is_none());
        assert!(!k.check(Some("Bearer cokoli")), "bez klíče nikdo");
        let a = k.ensure().unwrap();
        assert!(valid(&a), "{a}");
        assert_eq!(k.ensure().unwrap(), a, "druhé zapnutí klíč nemění");
        assert!(k.check(Some(&format!("Bearer {a}"))));
        assert!(!k.check(Some(&format!("Bearer {a}x"))));
        assert!(!k.check(Some(&a)), "bez schématu ne");
        assert!(!k.check(None));
        let b = k.rotate().unwrap();
        assert_ne!(a, b);
        assert!(!k.check(Some(&format!("Bearer {a}"))), "starý klíč po výměně neplatí");
        assert!(k.check(Some(&format!("Bearer {b}"))));
    }

    /// Chyba čtení úložiště není „klíč chybí“: `ensure` selže a nic nepřepíše.
    #[test]
    fn ensure_does_not_rotate_on_read_error() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Broken(std::sync::Arc<AtomicUsize>);
        impl Vault for Broken {
            fn read(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
                Err("zamčeno".into())
            }
            fn write(&self, _: &str, _: &[u8]) -> Result<(), String> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn delete(&self, _: &str) -> Result<(), String> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }
        let writes = std::sync::Arc::new(AtomicUsize::new(0));
        let k = Keys::new(Box::new(Broken(writes.clone())));
        assert!(k.ensure().is_err());
        assert_eq!(writes.load(Ordering::SeqCst), 0, "úložiště se nesmí změnit");
        assert!(k.get().is_none());
        assert!(!k.check(Some("Bearer cokoli")), "při chybě nepustí nikoho");
    }

    #[test]
    fn targets_do_not_share_a_key() {
        #[derive(Clone, Default)]
        struct Shared(std::sync::Arc<MemVault>);
        impl Vault for Shared {
            fn read(&self, t: &str) -> Result<Option<Vec<u8>>, String> {
                self.0.read(t)
            }
            fn write(&self, t: &str, d: &[u8]) -> Result<(), String> {
                self.0.write(t, d)
            }
            fn delete(&self, t: &str) -> Result<(), String> {
                self.0.delete(t)
            }
        }
        let vault = Shared::default();
        let mcp = Keys::new(Box::new(vault.clone()));
        let hooks = Keys::for_target(Box::new(vault.clone()), "Ledge/hooks");
        let (a, b) = (mcp.ensure().unwrap(), hooks.ensure().unwrap());
        assert_ne!(a, b);
        assert!(!mcp.check(Some(&format!("Bearer {b}"))), "klíč hooků MCP neotevře");
        // po restartu se každý přečte ze svého záznamu
        assert_eq!(Keys::for_target(Box::new(vault.clone()), "Ledge/hooks").get(), Some(b));
        assert_eq!(read(&vault), Some(a));
    }

    #[test]
    fn stored_key_is_read_and_garbage_ignored() {
        let v = MemVault::default();
        v.write(TARGET, b"kratky").unwrap();
        assert!(read(&v).is_none());
        let good = fresh().unwrap();
        v.write(TARGET, good.as_bytes()).unwrap();
        assert_eq!(read(&v).as_deref(), Some(good.as_str()));
    }
}
