//! Souborové helpery sdílené celou aplikací: atomický zápis, čtení/přejmenování s opakováním pro Windows
//! (antivir, indexer, git sync a jiné vlákno drží soubor na pár ms) a JSON načtení/uložení.
//! Žádný stav, žádné časovače — opakuje se jen uvnitř volání a jen po chybě.

use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Strop opakování: 40 × 25 ms ≈ 1 s.
const RETRIES: u32 = 40;
const STEP: Duration = Duration::from_millis(25);

/// Přechodná chyba Windows: „přístup odepřen“ (5) / porušení sdílení (32).
fn transient(e: &io::Error) -> bool {
    e.kind() == ErrorKind::PermissionDenied || matches!(e.raw_os_error(), Some(5 | 32))
}

/// `NotFound` u souboru, který ve skutečnosti je / jehož složka je — cíl se právě přepisuje.
fn vanishing(e: &io::Error, file: &Path) -> bool {
    e.kind() == ErrorKind::NotFound && file.parent().is_some_and(Path::is_dir)
}

/// Unikátní tmp vedle cíle (`<jméno>.<pid>-<čítač>.tmp`) — dva souběžné zápisy si ho nepřepíšou.
fn tmp_path(path: &Path) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    PathBuf::from(tmp)
}

/// Strop jednoho čekání a celkový rozpočet `rename_retry`.
const BACKOFF_CAP_MS: u64 = 200;
const BACKOFF_BUDGET: Duration = Duration::from_secs(3);

/// Čekání před pokusem `attempt` (od 0): exponenciálně 2·2^n ms do stropu, s jitterem 50–100 % (z `seed`).
pub(crate) fn backoff_delay(attempt: u32, seed: u64) -> Duration {
    let base = (2u64 << attempt.min(10)).min(BACKOFF_CAP_MS);
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(attempt as u64 + 1);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let half = base / 2;
    Duration::from_millis(base - half + x % (half + 1))
}

/// Semínko jitteru: nanosekundy + id vlákna (hash), bez závislosti.
fn jitter_seed() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut h);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    (h.finish() ^ t) | 1
}

/// Rename s opakováním (exponenciální backoff s jitterem, celkem max ~3 s) na přechodné chyby Windows.
pub fn rename_retry(from: &Path, to: &Path) -> io::Result<()> {
    let start = std::time::Instant::now();
    let seed = jitter_seed();
    let mut attempt = 0;
    loop {
        match std::fs::rename(from, to) {
            Err(e) if start.elapsed() < BACKOFF_BUDGET && (transient(&e) || (from.exists() && vanishing(&e, to))) => {
                std::thread::sleep(backoff_delay(attempt, seed));
                attempt += 1;
            }
            r => return r,
        }
    }
}

/// Atomický zápis: unikátní tmp, `sync_all`, rename s opakováním. Čtenář ani watcher nikdy neuvidí
/// půlku souboru; při chybě se tmp uklidí. Složku cíle nezakládá (viz `write_atomic_dir`).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = tmp_path(path);
    let res = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        rename_retry(&tmp, path)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// `write_atomic` + založení chybějící složky cíle.
pub fn write_atomic_dir(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_atomic(path, bytes)
}

/// Čtení s opakováním: zápis jiného vlákna/procesu soubor na pár ms zamkne nebo schová (rename).
/// Skutečně chybějící soubor ve skutečné složce selže po ~100 ms, ne po sekundě.
pub fn read_retry(path: &Path) -> io::Result<Vec<u8>> {
    let mut attempt = 0;
    loop {
        match std::fs::read(path) {
            Err(e) if attempt < RETRIES && transient(&e) => {
                attempt += 1;
                std::thread::sleep(STEP);
            }
            Err(e) if attempt < 4 && vanishing(&e, path) => {
                attempt += 1;
                std::thread::sleep(STEP);
            }
            r => return r,
        }
    }
}

#[cfg(test)]
/// Text varianta `read_retry` (neplatné UTF-8 = chyba `InvalidData`).
pub fn read_string_retry(path: &Path) -> io::Result<String> {
    String::from_utf8(read_retry(path)?).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
}

/// JSON ze souboru; chybějící / nečitelný / rozbitý soubor = `Default`.
pub fn load_json_or_default<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    read_retry(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Uloží hodnotu jako hezky formátovaný JSON (atomicky, se založením složky).
pub fn save_json<T: serde::Serialize>(path: &Path, v: &T) -> io::Result<()> {
    let text = serde_json::to_string_pretty(v).map_err(io::Error::other)?;
    write_atomic_dir(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    #[test]
    fn backoff_delay_table() {
        for seed in [1u64, 7, 0xDEAD_BEEF, u64::MAX] {
            let mut prev_max = 0;
            for attempt in 0..30u32 {
                let base = (2u64 << attempt.min(10)).min(200);
                let d = super::backoff_delay(attempt, seed).as_millis() as u64;
                assert!(d >= base - base / 2 && d <= base, "attempt {attempt} seed {seed}: {d} vs {base}");
                assert!(d <= 200);
                assert!(base >= prev_max);
                prev_max = base;
            }
        }
        assert!(super::backoff_delay(0, 5).as_millis() <= 2);
    }

    use super::*;

    fn leftovers(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".tmp")).collect()
    }

    #[test]
    fn replaces_and_cleans_tmp() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.json");
        write_atomic(&p, b"1").unwrap();
        write_atomic(&p, b"22").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"22");
        assert!(leftovers(d.path()).is_empty());
    }

    #[test]
    fn missing_dir_fails_without_tmp_but_dir_variant_creates_it() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x").join("a.json");
        assert!(write_atomic(&p, b"1").is_err());
        write_atomic_dir(&p, b"1").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"1");
        assert!(leftovers(&d.path().join("x")).is_empty());
    }

    #[test]
    fn tmp_names_are_unique_and_end_with_tmp() {
        let p = Path::new("c:/a/b.json");
        let (a, b) = (tmp_path(p), tmp_path(p));
        assert_ne!(a, b);
        assert!(a.to_string_lossy().starts_with("c:/a/b.json.") && a.to_string_lossy().ends_with(".tmp"));
    }

    #[test]
    fn concurrent_writers_and_readers_never_see_partial_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.txt");
        write_atomic(&p, &[b'a'; 4096]).unwrap();
        let mut hs = Vec::new();
        for i in 0..6u8 {
            let p = p.clone();
            hs.push(std::thread::spawn(move || {
                for _ in 0..25 {
                    write_atomic(&p, &[b'a' + i; 4096]).unwrap();
                }
            }));
        }
        for _ in 0..2 {
            let p = p.clone();
            hs.push(std::thread::spawn(move || {
                for _ in 0..50 {
                    let b = read_retry(&p).unwrap();
                    assert_eq!(b.len(), 4096);
                    assert!(b.iter().all(|c| *c == b[0]));
                }
            }));
        }
        hs.into_iter().for_each(|h| h.join().unwrap());
        assert!(leftovers(d.path()).is_empty());
    }

    #[test]
    fn read_retry_missing_is_not_found_quickly() {
        let d = tempfile::tempdir().unwrap();
        let t = std::time::Instant::now();
        assert_eq!(read_retry(&d.path().join("nic")).unwrap_err().kind(), ErrorKind::NotFound);
        assert!(t.elapsed() < Duration::from_millis(600));
        assert_eq!(read_retry(&d.path().join("nope").join("nic")).unwrap_err().kind(), ErrorKind::NotFound);
    }

    #[test]
    fn json_roundtrip_and_defaults() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("s").join("v.json");
        assert_eq!(load_json_or_default::<Vec<u32>>(&p), Vec::<u32>::new());
        save_json(&p, &vec![1u32, 2]).unwrap();
        assert_eq!(load_json_or_default::<Vec<u32>>(&p), vec![1, 2]);
        std::fs::write(&p, "{rozbité").unwrap();
        assert_eq!(load_json_or_default::<Vec<u32>>(&p), Vec::<u32>::new());
    }

    #[test]
    fn transient_classification() {
        assert!(transient(&io::Error::from_raw_os_error(5)));
        assert!(transient(&io::Error::from_raw_os_error(32)));
        assert!(!transient(&io::Error::from(ErrorKind::NotFound)));
    }
}
