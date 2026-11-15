//! API klíč propojené aplikace: Správce pověření Windows, generic `Ledge/connector`, uživatel `api` (stejný
//! mechanismus jako `briefrun/key.rs` — `accounts::vault`). Nikdy do configu, logu, chybové hlášky
//! ani do UI (jen „uložen / chybí“).

use crate::accounts::vault::Vault;
use crate::i18n::tr;

pub const TARGET: &str = "Ledge/connector";
pub const USER: &str = "api";

/// Obecný tvar: 16–256 znaků, žádný bílý ani řídicí znak (bez předpony; hodnotu vydává cizí server).
pub fn valid(k: &str) -> bool {
    (16..=256).contains(&k.len()) && !k.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Uložený platný klíč (nečitelný / neplatný záznam = žádný; UTF-8 i UTF-16LE z `cmdkey`).
pub fn get(v: &dyn Vault) -> Option<String> {
    crate::briefrun::key::decode_with(&v.read(TARGET).ok()??, valid)
}

/// Uloží klíč (po ořezu mezer). Chyba nikdy neobsahuje klíč.
pub fn set(v: &dyn Vault, key: &str) -> Result<(), String> {
    let k = key.trim();
    if !valid(k) {
        return Err(tr("Tohle nevypadá jako klíč (16–256 znaků bez mezer).", "This doesn't look like a key (16–256 characters, no spaces).").into());
    }
    v.write_as(TARGET, USER, k.as_bytes()).map_err(|_| tr("Klíč nejde uložit do Správce pověření.", "Can't save the key to Credential Manager.").to_string())
}

pub fn clear(v: &dyn Vault) -> Result<(), String> {
    v.delete(TARGET)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::vault::MemVault;

    const K: &str = "k0123456789abcdef0123456789abcdef";

    #[test]
    fn shape_table() {
        for (k, ok) in [(K, true), ("kratky", false), ("sk-ant-api03-0123456789012345678901", true), ("", false), ("0123456789 abcdef012345", false), ("0123456789	abcdef012345", false)] {
            assert_eq!(valid(k), ok, "{k}");
        }
        assert!(valid(&"x".repeat(256)) && !valid(&"x".repeat(257)) && !valid(&"x".repeat(15)) && valid(&"x".repeat(16)));
    }

    #[test]
    fn set_get_clear_and_errors_without_key() {
        let v = MemVault::default();
        assert_eq!(get(&v), None);
        set(&v, &format!("  {K}\n")).unwrap();
        assert_eq!(get(&v).as_deref(), Some(K));
        let e = set(&v, "TAJNE").unwrap_err();
        assert!(!e.contains("TAJNE"), "chyba nesmí nést klíč: {e}");
        assert_eq!(get(&v).as_deref(), Some(K), "neplatný klíč nepřepíše uložený");
        let wide: Vec<u8> = K.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        v.write(TARGET, &wide).unwrap();
        assert_eq!(get(&v).as_deref(), Some(K), "cmdkey UTF-16");
        clear(&v).unwrap();
        assert_eq!(get(&v), None);
    }
}
