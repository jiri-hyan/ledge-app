//! PKCE (RFC 7636) a `state`: náhoda ze systémového CSPRNG (`BCryptGenRandom`), base64url bez
//! zarovnání, `code_challenge = BASE64URL(SHA256(code_verifier))` (metoda S256).

use sha2::{Digest, Sha256};

/// base64url bez `=` (RFC 4648 §5) — pro verifier, challenge i state.
pub fn b64url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for ch in bytes.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        let take = ch.len() + 1;
        for i in 0..take {
            out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// `n` bajtů ze systémového generátoru (CSPRNG Windows). Chyba = nic se nepřihlásí (nikdy slabá náhoda).
pub fn random(n: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; n];
    #[cfg(windows)]
    {
        use windows::Win32::Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom};
        // SAFETY: buffer je platný po celou dobu volání, bez handle algoritmu (systémový RNG)
        let st = unsafe { BCryptGenRandom(None, &mut buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
        if st.is_err() {
            return Err(crate::i18n::tr("Systém nevydal náhodná čísla pro přihlášení.", "The system didn't provide random numbers for sign-in.").into());
        }
    }
    #[cfg(not(windows))]
    {
        return Err("PKCE: jen Windows".into());
    }
    Ok(buf)
}

/// `code_verifier` z 32 náhodných bajtů = 43 znaků z neomezené sady (RFC 7636 §4.1: 43–128).
pub fn verifier_from(bytes: &[u8]) -> String {
    b64url(bytes)
}

/// `code_challenge` pro metodu S256.
pub fn challenge(verifier: &str) -> String {
    b64url(&Sha256::digest(verifier.as_bytes()))
}

/// Nový pár (verifier, state) pro jedno přihlášení.
pub fn fresh() -> Result<(String, String), String> {
    Ok((verifier_from(&random(32)?), b64url(&random(24)?)))
}

/// Porovnání `state` v konstantním čase (délka smí prozradit jen délku).
pub fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc7636_appendix_b_vector() {
        // RFC 7636, příloha B: 32 bajtů verifieru → verifier → challenge S256
        let bytes: [u8; 32] = [
            116, 24, 223, 180, 151, 153, 224, 37, 79, 250, 96, 125, 216, 173, 187, 186, 22, 212, 37, 77, 105, 214, 191, 240, 91, 88, 5, 88, 83, 132, 141, 121,
        ];
        let v = verifier_from(&bytes);
        assert_eq!(v, "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(challenge(&v), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn b64url_table() {
        // RFC 4648 §10 testovací vektory (bez zarovnání) + znaky - a _
        let cases: [(&[u8], &str); 9] = [
            (b"", ""),
            (b"f", "Zg"),
            (b"fo", "Zm8"),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg"),
            (b"fooba", "Zm9vYmE"),
            (b"foobar", "Zm9vYmFy"),
            (&[0xfb, 0xff], "-_8"),
            (&[0xff, 0xff, 0xff], "____"),
        ];
        for (inp, want) in cases {
            assert_eq!(b64url(inp), want, "{inp:?}");
        }
    }

    #[test]
    fn fresh_is_random_and_valid() {
        let (v1, s1) = fresh().unwrap();
        let (v2, s2) = fresh().unwrap();
        assert_ne!(v1, v2);
        assert_ne!(s1, s2);
        assert_eq!(v1.len(), 43);
        assert!(v1.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_eq!(s1.len(), 32);
    }

    #[test]
    fn state_compare() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "ab"));
        assert!(!same("", "a"));
    }
}
