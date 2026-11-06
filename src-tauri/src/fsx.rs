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

/// Nejvíc kopií `<soubor>.broken-*` u jednoho souboru.
const MAX_BROKEN: usize = 3;

/// Řazení kopie podle přípony za `.broken-` (`<sekundy>` nebo `<sekundy>-<n>`); neznámý tvar je nejstarší.
fn broken_order(suffix: &str) -> (u64, u32) {
    let (secs, n) = suffix.split_once('-').unwrap_or((suffix, "1"));
    (secs.parse().unwrap_or(0), n.parse().unwrap_or(0))
}

/// Zkopíruje existující, ale nečitelný soubor na `<jméno>.broken-<sekundy>` a smaže nejstarší kopie
/// nad `MAX_BROKEN`. Obsah se nikam nezapisuje ani se neloguje; vrací cestu kopie.
pub fn keep_broken_copy(path: &Path, secs: u64) -> io::Result<PathBuf> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "soubor".into());
    let bak = (1u32..)
        .map(|i| path.with_file_name(if i == 1 { format!("{name}.broken-{secs}") } else { format!("{name}.broken-{secs}-{i}") }))
        .find(|p| !p.exists())
        .expect("nekonečná řada");
    std::fs::copy(path, &bak)?;
    let prefix = format!("{name}.broken-");
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()).or(Some(Path::new(".")))
        && let Ok(rd) = std::fs::read_dir(dir)
    {
        let mut all: Vec<((u64, u32), PathBuf)> = rd
            .flatten()
            .filter_map(|e| {
                let fname = e.file_name().to_string_lossy().into_owned();
                fname.strip_prefix(&prefix).map(|suf| (broken_order(suf), e.path()))
            })
            .collect();
        all.sort();
        while all.len() > MAX_BROKEN {
            let (_, old) = all.remove(0);
            if old != bak {
                let _ = std::fs::remove_file(old);
            }
        }
    }
    Ok(bak)
}

/// Jako `load_json_or_default`, ale soubor, který existuje a nejde naparsovat, se napřed zazálohuje
/// (`keep_broken_copy`) — první uložení výchozího stavu ho jinak přepíše. Do logu jde jen cesta.
pub fn load_json_keep_broken<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    let Ok(bytes) = read_retry(path) else { return T::default() };
    match serde_json::from_slice(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes)) {
        Ok(v) => v,
        Err(_) => {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            note(&broken_line(path, keep_broken_copy(path, secs)));
            T::default()
        }
    }
}

/// Řádek do logu o poškozeném souboru: jen cesty (a chyba kopie), nikdy obsah souboru.
fn broken_line(path: &Path, copy: io::Result<PathBuf>) -> String {
    match copy {
        Ok(bak) => format!("Poškozený soubor {} — kopie {}", path.display(), bak.display()),
        Err(e) => format!("Poškozený soubor {} — kopie selhala: {e}", path.display()),
    }
}

/// Zápis do logu appky (`ledge.log`; release nemá konzoli, `eprintln!` by nikam nevedl). V testech se
/// nepíše nic, aby testy nesahaly na skutečný log uživatele.
fn note(line: &str) {
    #[cfg(not(test))]
    crate::state::log(line);
    #[cfg(test)]
    let _ = line;
}

/// Uloží hodnotu jako hezky formátovaný JSON (atomicky, se založením složky).
pub fn save_json<T: serde::Serialize>(path: &Path, v: &T) -> io::Result<()> {
    let text = serde_json::to_string_pretty(v).map_err(io::Error::other)?;
    write_atomic_dir(path, text.as_bytes())
}

/// Výpis jedné složky pro `list_files`.
pub struct Listing<T> {
    /// složka existuje (chybí-li, `error` je `None`)
    pub exists: bool,
    pub error: Option<String>,
    /// souborů s povolenou příponou nad `max`
    pub skipped: usize,
    /// (jméno, cesta, hodnota z přípony, metadata) podle jména bez ohledu na velikost písmen, nejvýš `max`
    pub files: Vec<(String, PathBuf, T, std::fs::Metadata)>,
}

/// Jeden výpis složky bez rekurze: jen běžné soubory bez tečky na začátku, jejichž příponu `from_ext`
/// přijme (vrátí `Some`); seřazeno podle jména, zbytek nad `max` se jen spočítá. Chybějící složka = prázdný výpis.
pub fn list_files<T>(dir: &Path, from_ext: impl Fn(&str) -> Option<T>, max: usize) -> Listing<T> {
    let mut out = Listing { exists: false, error: None, skipped: 0, files: Vec::new() };
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == ErrorKind::NotFound => return out,
        Err(e) => {
            out.exists = dir.is_dir();
            out.error = Some(e.to_string());
            return out;
        }
    };
    out.exists = true;
    out.files = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let v = from_ext(Path::new(&name).extension()?.to_str()?)?;
            let md = e.metadata().ok().filter(|m| m.is_file())?;
            (!name.starts_with('.')).then(|| (name, e.path(), v, md))
        })
        .collect();
    out.files.sort_by_key(|(n, ..)| n.to_lowercase());
    out.skipped = out.files.len().saturating_sub(max);
    out.files.truncate(max);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn broken_line_has_paths_only() {
        let p = std::path::Path::new("dir/app.json");
        let ok = super::broken_line(p, Ok(std::path::PathBuf::from("dir/app.json.broken-1")));
        assert!(ok.contains("app.json") && ok.contains("app.json.broken-1"), "{ok}");
        let err = super::broken_line(p, Err(std::io::Error::other("zamčeno")));
        assert!(err.contains("kopie selhala") && err.contains("zamčeno"), "{err}");
    }

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
    fn list_files_filters_sorts_and_limits() {
        let d = tempfile::tempdir().unwrap();
        for f in ["b.ps1", "A.ps1", "c.txt", ".skryty.ps1", "d.ps1", "bez_pripony"] {
            std::fs::write(d.path().join(f), "x").unwrap();
        }
        std::fs::create_dir(d.path().join("slozka.ps1")).unwrap();
        let ext = |e: &str| (e == "ps1").then_some(7u8);
        let all = list_files(d.path(), ext, 10);
        assert!(all.exists && all.error.is_none() && all.skipped == 0);
        assert_eq!(all.files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["A.ps1", "b.ps1", "d.ps1"]);
        assert!(all.files.iter().all(|f| f.2 == 7 && f.3.len() == 1));
        let cut = list_files(d.path(), ext, 2);
        assert_eq!((cut.files.len(), cut.skipped), (2, 1));
        let none = list_files(&d.path().join("nic"), ext, 2);
        assert!(!none.exists && none.error.is_none() && none.files.is_empty());
    }

    #[test]
    fn transient_classification() {
        assert!(transient(&io::Error::from_raw_os_error(5)));
        assert!(transient(&io::Error::from_raw_os_error(32)));
        assert!(!transient(&io::Error::from(ErrorKind::NotFound)));
    }
}

#[cfg(test)]
mod broken_tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, serde::Deserialize)]
    struct Doc {
        #[serde(default)]
        a: u32,
    }

    fn broken_files(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.contains(".broken-")).collect();
        v.sort();
        v
    }

    #[test]
    fn broken_json_is_copied_valid_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("desk.json");
        // chybějící i platný soubor: žádná kopie
        assert_eq!(load_json_keep_broken::<Doc>(&p), Doc::default());
        std::fs::write(&p, r#"{"a":5}"#).unwrap();
        assert_eq!(load_json_keep_broken::<Doc>(&p), Doc { a: 5 });
        assert!(broken_files(dir.path()).is_empty());
        // rozbitý: výchozí stav + kopie s původními bajty
        std::fs::write(&p, b"{rozbit\xFF").unwrap();
        assert_eq!(load_json_keep_broken::<Doc>(&p), Doc::default());
        let b = broken_files(dir.path());
        assert_eq!(b.len(), 1);
        assert_eq!(std::fs::read(dir.path().join(&b[0])).unwrap(), b"{rozbit\xFF");
    }

    #[test]
    fn broken_copies_are_capped_dropping_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("note-pins.json");
        std::fs::write(&p, "{").unwrap();
        // sekundy, v pořadí vzniku (stejná sekunda dává příponu -2, -3…)
        for secs in [100u64, 100, 200, 300, 300] {
            keep_broken_copy(&p, secs).unwrap();
        }
        let b = broken_files(dir.path());
        assert_eq!(b.len(), MAX_BROKEN, "{b:?}");
        assert!(!b.contains(&"note-pins.json.broken-100".to_string()) && !b.contains(&"note-pins.json.broken-100-2".to_string()), "{b:?}");
        assert!(b.contains(&"note-pins.json.broken-300-2".to_string()), "{b:?}");
        // cizí soubory se nedotknou
        assert!(p.exists());
    }

    #[test]
    fn broken_order_table() {
        let cases: &[(&str, (u64, u32))] = &[("100", (100, 1)), ("100-2", (100, 2)), ("x", (0, 1)), ("7-q", (7, 0))];
        for (s, want) in cases {
            assert_eq!(broken_order(s), *want, "{s}");
        }
    }
}
