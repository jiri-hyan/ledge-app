//! Rychlé poznámky — centrální úložiště mimo repa projektů (se synchronizací vlastní git repo, `notesync.rs`) (výchozí `%USERPROFILE%\Ledge\notes\`).
//!
//! Struktura: `<root>\<projectId|_obecne>\<YYYY-MM-DD-HHMMSS>-<slug>.md`, obrázky vedle jako
//! `<stejný základ>-1.png`… a v Markdownu odkazy `![](./…png)`. Jeden `.md` = jedna poznámka s YAML
//! frontmatter:
//!
//! ```text
//! ---
//! project: acme-app
//! created: 2026-10-16T14:03:22+02:00
//! remind: 2026-10-17T09:00:00+02:00
//! done: false
//! ---
//!
//! Zavolat Řehořovi kvůli fakturám
//! ```
//!
//! Časy jsou vždy ISO 8601 s lokálním offsetem (žádné naivní časy). Parser frontmatteru je ruční a
//! tolerantní: neznámé klíče a pořadí řádků se při přepisu (`done`) zachovají, tělo zůstane bajtově
//! stejné. Mazání jde přes odklad do Koše Windows (`lifecycle`, `recycle`) s krátkým Zpět; archiv je
//! podsložka `_archiv` vedle poznámky (`lifecycle`).
//!
//! Čisté jádro (parsování, slug, cesty, výběr připomínek, předvyplnění projektu) je oddělené od I/O
//! (`create`, `scan`, `set_done`), aby šlo testovat bez disku a nikdy ne v reálném `%USERPROFILE%`.
//! Tady: kořen, čas, obrázky (base64/PNG) a společné I/O (`write_atomic`, `path_lock`). Podmoduly:
//! `frontmatter`, `store` (NoteView, create, scan, set_done), `reminders` (karta, peek), `draft`
//! (koncept, předvyplnění), `md` (Markdown náhled, úkoly, titulek), `edit` (čtení / uložení / úkol /
//! filtr karty). Veřejné API je re-exportované, cesty `notes::…` platí dál. Mapa: `docs/codemap/notes.md`.

mod draft;
pub mod edit;
pub mod export;
mod frontmatter;
pub mod lifecycle;
pub mod openwith;
pub mod pdf;
pub mod md;
mod md_list;
mod recycle;
mod reminders;
pub mod search;
mod store;
#[cfg(test)]
mod testkit;
pub mod when;

pub use draft::*;
pub use frontmatter::*;
pub use recycle::recycle;
pub use reminders::*;
pub use store::*;

use crate::i18n::tr;
use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Složka pro poznámky bez projektu. Jako hodnota `project:` = výslovně bez projektu (poznámka ve
/// složce projektu, které šéf projekt odebral — soubor se kvůli syncu nepřesouvá).
pub const GENERAL: &str = "_obecne";
/// Kategorie (`kind:`), když klíč chybí.
pub const DEFAULT_KIND: &str = "info";
/// Strop velikosti jednoho obrázku (po dekódování).
pub const MAX_IMAGE: usize = 15 * 1024 * 1024;
/// Víc obrázků na jednu poznámku nedává smysl a hlídá to paměť.
pub const MAX_IMAGES: usize = 20;
/// Poznámky větší než tohle se při skenu přeskočí (nejsou to naše poznámky).
const MAX_NOTE_BYTES: u64 = 1 << 20;
const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

// ---------- kořen ----------

/// `%USERPROFILE%\Ledge\notes`.
pub fn default_root() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join("Ledge").join("notes")
}

/// Rozbalí `%PROMENNA%` (Windows styl). Neznámá proměnná = chyba, `%%` = `%`.
pub fn expand_env(s: &str, var: impl Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let j = after.find('%').ok_or_else(|| trf!("Neuzavřené % v „{s}“.", "Unclosed % in “{s}”."))?;
        let name = &after[..j];
        if name.is_empty() {
            out.push('%');
        } else {
            out.push_str(&var(name).ok_or_else(|| trf!("Proměnná %{name}% neexistuje.", "Variable %{name}% doesn't exist."))?);
        }
        rest = &after[j + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Kořen z configu (`notes.root`); prázdné = výchozí. `%VAR%` se rozbalí. Odmítne relativní cestu,
/// `..` a příliš mělkou cestu (kořen disku nebo jen jedna složka pod ním) — Ledge do kořene zapisuje
/// složky a skryté soubory, takže `C:\` nebo `D:\Data` jako kořen nechceme.
pub fn resolve_root_with(cfg_root: &str, var: impl Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let r = cfg_root.trim();
    if r.is_empty() {
        return Ok(default_root());
    }
    let p = PathBuf::from(expand_env(r, var)?);
    if !p.is_absolute() {
        return Err(trf!("notes.root „{r}“ musí být absolutní cesta (např. s písmenem disku).", "notes.root “{r}” must be an absolute path (e.g. with a drive letter)."));
    }
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(trf!("notes.root „{r}“ nesmí obsahovat „..“.", "notes.root “{r}” must not contain “..”."));
    }
    if p.components().filter(|c| matches!(c, Component::Normal(_))).count() < 2 {
        return Err(trf!(
            "notes.root „{r}“ je moc mělko — chce aspoň dvě složky pod diskem (např. D:\\Data\\notes).",
            "notes.root “{r}” is too shallow — it needs at least two folders below the drive (e.g. D:\\Data\\notes)."
        ));
    }
    Ok(p)
}

pub fn resolve_root(cfg_root: &str) -> Result<PathBuf, String> {
    resolve_root_with(cfg_root, |n| std::env::var(n).ok())
}

// ---------- čas ----------

/// ISO 8601 s offsetem, na sekundy: `2026-10-16T14:03:22+02:00`.
pub fn iso(dt: &DateTime<FixedOffset>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Lokální čas jako ISO s offsetem.
pub fn now_local() -> DateTime<FixedOffset> {
    Local::now().fixed_offset()
}

/// Naivní lokální čas → čas s lokálním offsetem. Na podzimní změně času (dvojznačné) bere dřívější,
/// v jarní díře (neexistující čas) posune o hodinu dál.
pub fn local_from_naive(n: NaiveDateTime) -> Option<DateTime<FixedOffset>> {
    Local
        .from_local_datetime(&n)
        .earliest()
        .or_else(|| Local.from_local_datetime(&(n + ChronoDuration::hours(1))).earliest())
        .map(|d| d.fixed_offset())
}

/// Tolerantní čtení času: ISO/RFC 3339 s offsetem (i `Z`), jinak naivní tvar (z `datetime-local`,
/// ručně psaný) = lokální čas. Samotné datum = 9:00 lokálně.
pub fn parse_time(s: &str) -> Option<DateTime<FixedOffset>> {
    let s = s.trim().trim_matches(|c| c == '"' || c == '\'');
    if s.is_empty() {
        return None;
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d);
    }
    for f in ["%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M%z", "%Y-%m-%d %H:%M:%S%z", "%Y-%m-%d %H:%M%z"] {
        if let Ok(d) = DateTime::parse_from_str(s, f) {
            return Some(d);
        }
    }
    for f in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, f) {
            return local_from_naive(n);
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return local_from_naive(d.and_time(NaiveTime::from_hms_opt(9, 0, 0)?));
    }
    None
}

/// Konec dnešního dne (lokální půlnoc) v ms — hranice „připomínky na dnes“.
pub fn end_of_day_ms(now: &DateTime<FixedOffset>) -> i64 {
    let next = now.date_naive().succ_opt().and_then(|d| d.and_hms_opt(0, 0, 0));
    next.and_then(local_from_naive).map(|d| d.timestamp_millis()).unwrap_or(now.timestamp_millis() + 86_400_000)
}


// ---------- obrázky ----------

/// Standardní base64 (volitelně s prefixem `data:…;base64,`, bílé znaky ignoruje).
pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = match s.find(";base64,") {
        Some(i) if s.starts_with("data:") => &s[i + 8..],
        _ => s,
    };
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut pad = 0;
    for &b in s.as_bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                pad += 1;
                continue;
            }
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return Err(tr("Obrázek není platné base64.", "The image isn't valid base64.").into()),
        };
        if pad > 0 {
            return Err(tr("Obrázek není platné base64.", "The image isn't valid base64.").into());
        }
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Base64 PNG z UI → bajty. Kontroluje velikost (před i po dekódování) a hlavičku PNG.
pub fn decode_png(b64: &str) -> Result<Vec<u8>, String> {
    if b64.len() > MAX_IMAGE / 3 * 4 + 1024 {
        return Err(tr("Obrázek je větší než 15 MB.", "The image is larger than 15 MB.").into());
    }
    let bytes = b64_decode(b64)?;
    if bytes.len() > MAX_IMAGE {
        return Err(tr("Obrázek je větší než 15 MB.", "The image is larger than 15 MB.").into());
    }
    if bytes.len() < 33 || bytes[..8] != PNG_SIG {
        return Err(tr("Obrázek není PNG.", "The image isn't a PNG.").into());
    }
    Ok(bytes)
}

// ---------- I/O ----------

/// Atomický zápis: unikátní tmp vedle cíle (pid + čítač — dva souběžné zápisy si ho nepřepíšou),
/// `sync_all`, pak rename. Watcher ani čtenář nikdy neuvidí půlku souboru.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let err = |e: std::io::Error| crate::errors::io(&trf!("{} nejde zapsat", "Can't write {}", path.display()), &e);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    let tmp = PathBuf::from(tmp);
    let res = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        drop(f);
        rename_retry(&tmp, path)
    })();
    res.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        err(e)
    })
}

/// Rename s krátkým opakováním: na Windows ho odmítne „přístup odepřen“ / sdílení, když cíl nebo tmp
/// zrovna drží jiný proces (antivir, indexer, git sync, editor) — typicky na pár ms. Max ~250 ms.
fn rename_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 0;
    loop {
        match std::fs::rename(from, to) {
            // „neexistuje“ při existujícím tmp i složce = cíl se právě přepisuje jiným zápisem (přechodné)
            Err(e)
                if attempt < 10
                    && (e.kind() == std::io::ErrorKind::PermissionDenied
                        || e.raw_os_error() == Some(32)
                        || (e.kind() == std::io::ErrorKind::NotFound && from.exists() && to.parent().is_some_and(Path::is_dir))) =>
            {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            r => return r,
        }
    }
}

/// Zámek read-modify-write pro jednu poznámku (sdílí `set_done` i `edit`).
pub(crate) fn path_lock(real: &Path) -> std::sync::Arc<parking_lot::Mutex<()>> {
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::{Arc, OnceLock};
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut map = LOCKS.get_or_init(Default::default).lock();
    // úklid: zámky, které nikdo nedrží
    map.retain(|_, l| Arc::strong_count(l) > 1);
    map.entry(real.to_path_buf()).or_default().clone()
}

pub(crate) fn b64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::testkit::{b64, png};
    use super::*;

    #[test]
    fn parse_time_variants() {
        let d = parse_time("2026-10-17T09:00:00+02:00").unwrap();
        assert_eq!(iso(&d), "2026-10-17T09:00:00+02:00");
        assert_eq!(parse_time("2026-10-17T07:00:00Z").unwrap().timestamp(), d.timestamp());
        assert_eq!(parse_time("\"2026-10-17T09:00+02:00\"").unwrap().timestamp(), d.timestamp());
        // naivní = lokální čas: stejná místní hodina, offset doplněný
        for (s, h, m) in [("2026-10-17T09:30", 9, 30), ("2026-10-17 09:30:00", 9, 30), ("2026-10-17", 9, 0)] {
            let t = parse_time(s).unwrap();
            assert_eq!((t.format("%H").to_string().parse::<u32>().unwrap(), t.format("%M").to_string().parse::<u32>().unwrap()), (h, m), "{s}");
            assert!(iso(&t).len() == 25, "{}", iso(&t));
        }
        for bad in ["", "zítra", "2026-13-01T10:00", "17.10.2026"] {
            assert!(parse_time(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn end_of_day_is_next_local_midnight() {
        let now = now_local();
        let e = end_of_day_ms(&now);
        assert!(e > now.timestamp_millis());
        assert!(e - now.timestamp_millis() <= 25 * 3_600_000);
    }

    #[test]
    fn base64_and_png_validation() {
        let data = png(5);
        assert_eq!(b64_decode(&b64(&data)).unwrap(), data);
        assert_eq!(decode_png(&format!("data:image/png;base64,{}", b64(&data))).unwrap(), data);
        assert_eq!(b64_decode("YWJj\nZA==").unwrap(), b"abcd");
        assert!(b64_decode("YW*j").is_err());
        assert!(b64_decode("YQ==YQ==").is_err());
        assert!(decode_png(&b64(b"GIF89a-neni-png-ale-dost-dlouhe-aby-to-proslo-delkou")).unwrap_err().contains("PNG"));
        assert!(decode_png(&"A".repeat(MAX_IMAGE / 3 * 4 + 2000)).unwrap_err().contains("15 MB"));
    }

    #[test]
    fn root_expansion_and_rejections() {
        let var = |n: &str| match n {
            "USERPROFILE" => Some("C:\\Users\\ada".to_string()),
            "DATA" => Some("D:\\Data".to_string()),
            _ => None,
        };
        assert_eq!(resolve_root_with("", var).unwrap(), default_root());
        assert_eq!(resolve_root_with("  %USERPROFILE%\\Ledge\\notes ", var).unwrap(), PathBuf::from("C:\\Users\\ada\\Ledge\\notes"));
        assert_eq!(resolve_root_with("%DATA%\\notes", var).unwrap(), PathBuf::from("D:\\Data\\notes"));
        assert_eq!(resolve_root_with("\\\\srv\\share\\a\\b", var).unwrap(), PathBuf::from("\\\\srv\\share\\a\\b"));
        for bad in ["C:\\", "C:\\Data", "D:", "notes", "Ledge\\notes", "\\Ledge\\notes", "C:Ledge\\notes", "%NIC%\\notes", "%USERPROFILE", "C:\\a\\..\\..\\b", "%DATA%"] {
            assert!(resolve_root_with(bad, var).is_err(), "{bad}");
        }
        assert_eq!(expand_env("100%% %DATA%", var).unwrap(), "100% D:\\Data");
    }
}
