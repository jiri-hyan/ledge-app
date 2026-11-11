//! API klíč Anthropic pro zadání: Správce pověření Windows, generic `Ledge/anthropic`, uživatel `api`
//! (stejný mechanismus jako klíč MCP — `accounts::vault`). Nikdy do configu, logu, chybové hlášky ani
//! argumentů procesu; do UI jde jen „uložen / chybí“. V paměti se nedrží (čte se při startu běhu).

use crate::accounts::vault::Vault;
use crate::i18n::tr;

pub const TARGET: &str = "Ledge/anthropic";
pub const USER: &str = "api";

/// Základní tvar: `sk-ant-` + aspoň 20 znaků `[A-Za-z0-9_-]`, nejvýš 256.
pub fn valid(k: &str) -> bool {
    k.len() <= 256 && k.strip_prefix("sk-ant-").is_some_and(|r| r.len() >= 20 && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
}

/// Blob ze Správce pověření: UTF-8 (zápis z Ledge), nebo UTF-16LE (`cmdkey /pass:` ukládá široké znaky).
pub fn decode(blob: &[u8]) -> Option<String> {
    let utf16 = blob.len() >= 2 && blob.len().is_multiple_of(2) && blob.iter().skip(1).step_by(2).all(|b| *b == 0);
    let s = if utf16 {
        let w: Vec<u16> = blob.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16(&w).ok()?
    } else {
        String::from_utf8(blob.to_vec()).ok()?
    };
    let s = s.trim().to_string();
    valid(&s).then_some(s)
}

/// Uložený platný klíč (nečitelný / neplatný záznam = žádný).
pub fn get(v: &dyn Vault) -> Option<String> {
    decode(&v.read(TARGET).ok()??)
}

/// Uloží klíč (po ořezu mezer). Chyba nikdy neobsahuje klíč.
pub fn set(v: &dyn Vault, key: &str) -> Result<(), String> {
    let k = key.trim();
    if !valid(k) {
        return Err(tr("Tohle nevypadá jako API klíč Anthropic (začíná sk-ant-).", "This doesn't look like an Anthropic API key (starts with sk-ant-).").into());
    }
    v.write_as(TARGET, USER, k.as_bytes()).map_err(|_| tr("API klíč nejde uložit do Správce pověření.", "Can't save the API key to Credential Manager.").to_string())
}

pub fn clear(v: &dyn Vault) -> Result<(), String> {
    v.delete(TARGET)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::vault::MemVault;

    const K: &str = "sk-ant-api03-TESTKLIC_uzivatel-0123456789";

    #[test]
    fn shape_table() {
        for (k, ok) in [(K, true), ("sk-ant-kratky", false), ("sk-proj-0123456789012345678901", false), ("", false), ("sk-ant-api03 mezera 0123456789012", false)] {
            assert_eq!(valid(k), ok, "{k}");
        }
    }

    #[test]
    fn decode_utf8_and_cmdkey_utf16() {
        assert_eq!(decode(K.as_bytes()).as_deref(), Some(K));
        let wide: Vec<u8> = K.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(decode(&wide).as_deref(), Some(K));
        assert_eq!(decode(b"neplatne"), None);
        assert_eq!(decode(&[0xff, 0xfe, 0x00]), None);
    }

    #[test]
    fn set_get_clear_and_errors_without_key() {
        let v = MemVault::default();
        assert_eq!(get(&v), None);
        set(&v, &format!("  {K}\n")).unwrap();
        assert_eq!(get(&v).as_deref(), Some(K));
        let bad = "sk-ant-TAJNE";
        let e = set(&v, bad).unwrap_err();
        assert!(!e.contains("TAJNE"), "chyba nesmí nést klíč: {e}");
        assert_eq!(get(&v).as_deref(), Some(K), "neplatný klíč nepřepíše uložený");
        clear(&v).unwrap();
        assert_eq!(get(&v), None);
    }
}
