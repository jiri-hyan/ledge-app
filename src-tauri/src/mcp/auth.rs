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
    cache: Mutex<Option<String>>,
}

impl Keys {
    pub fn new(vault: Box<dyn Vault>) -> Self {
        Self { vault, cache: Mutex::new(None) }
    }

    /// Klíč, je-li (cache, jinak Správce pověření). Nečitelný záznam = žádný.
    pub fn get(&self) -> Option<String> {
        let mut c = self.cache.lock();
        if c.is_none() {
            *c = read(self.vault.as_ref());
        }
        c.clone()
    }

    /// Klíč, a když není, vytvoří nový (první zapnutí).
    pub fn ensure(&self) -> Result<String, String> {
        match self.get() {
            Some(k) => Ok(k),
            None => self.rotate(),
        }
    }

    /// Nový klíč (starý přestane platit hned — i pro připojené klienty).
    pub fn rotate(&self) -> Result<String, String> {
        let k = fresh()?;
        self.vault.write(TARGET, k.as_bytes())?;
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
    let bytes = v.read(TARGET).ok()??;
    let k = String::from_utf8(bytes).ok()?;
    valid(&k).then_some(k)
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
