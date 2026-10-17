//! Testovací pomůcky pro `notes::*::tests`: dočasný kořen (nikdy skutečný `%USERPROFILE%`),
//! falešné PNG, base64 a pevný čas. Jen `#[cfg(test)]`.

use super::PNG_SIG;
use chrono::{DateTime, FixedOffset};

pub(super) fn tmp_root() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("ledge-notes-").tempdir_in(std::env::temp_dir()).unwrap()
}

pub(super) fn png(extra: usize) -> Vec<u8> {
    let mut v = PNG_SIG.to_vec();
    v.extend(std::iter::repeat_n(7u8, 40 + extra));
    v
}

pub(super) fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub(super) fn at(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}
