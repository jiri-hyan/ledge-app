//! Velikost kontextu živé session (kolo 23, E1): kolik tokenů teď čte každý tah. Zdroj = JEDINÝ reálný: poslední
//! `message.usage` v transcriptu session (`transcript_path` z hooku) — vstup + zápis cache + čtení cache; poměr
//! k PRVNÍMU volání téže session. Nic se neodhaduje: chybí-li usage, vrací se `None` (UI ukáže „—“ / nic).
//! Čte se jen konec souboru (256 kB, při velkém posledním řádku víc do 8 MB) a začátek (do prvního usage),
//! výsledek je v paměti podle (mtime, délka) a nejvýš 1× za `THROTTLE_MS` na soubor. Volá se jen z UI, když je
//! karta session vidět (`session_context`) — žádné vlákno ani časovač.

use crate::usage::{floor, parse};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::LazyLock;

/// Od kolika tokenů je kontext „velký“ (tvrdá hranice z pravidel týmu: zeptat se na předávku).
pub const WARN: u64 = 250_000;
/// Od kolika je „příliš velký“.
pub const HIGH: u64 = 350_000;
/// Postupně zkoušené délky konce souboru; poslední usage je skoro vždy v prvním.
const TAIL_STEPS: [u64; 3] = [256 * 1024, 2 * 1024 * 1024, 8 * 1024 * 1024];
/// Kolik začátku souboru se prohledá kvůli prvnímu volání.
const HEAD_MAX: u64 = 4 * 1024 * 1024;
/// Nejčastější přečtení jednoho souboru (ms).
pub const THROTTLE_MS: u64 = 15_000;
/// Kolik souborů si pamatovat.
const CACHE_MAX: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ctx {
    /// velikost kontextu posledního volání: vstup + zápis cache + čtení cache
    pub tokens: u64,
    /// totéž u prvního volání session (`None` = začátek souboru nenesl usage)
    pub first: Option<u64>,
    /// `tokens / first` na 1 desetinu; `None` bez prvního volání
    pub ratio: Option<f64>,
    /// `ok` | `warn` (od `WARN`) | `high` (od `HIGH`)
    pub level: &'static str,
}

pub fn level(tokens: u64) -> &'static str {
    if tokens >= HIGH {
        "high"
    } else if tokens >= WARN {
        "warn"
    } else {
        "ok"
    }
}

/// `now / first` na 1 desetinu; bez prvního volání (nebo 0) nic.
pub fn ratio(now: u64, first: Option<u64>) -> Option<f64> {
    let f = first.filter(|f| *f > 0)?;
    Some(((now as f64 / f as f64) * 10.0).round() / 10.0)
}

pub fn context(last: Option<u64>, first: Option<u64>) -> Option<Ctx> {
    let tokens = last.filter(|t| *t > 0)?;
    Some(Ctx { tokens, first, ratio: ratio(tokens, first), level: level(tokens) })
}

/// Poslední odpověď API v konci souboru → velikost kontextu. `from_start` = výřez začíná na začátku souboru
/// (jinak je první řádek useknutý a přeskočí se). Řádky subagentů (`isSidechain`) mají jiný kontext → přeskočí se.
pub fn last_usage(tail: &[u8], from_start: bool) -> Option<u64> {
    let lines: Vec<&[u8]> = tail.split(|b| *b == b'\n').collect();
    for (i, line) in lines.iter().enumerate().rev() {
        if i == 0 && !from_start {
            break;
        }
        if memchr::memmem::find(line, b"\"isSidechain\":true").is_some() {
            continue;
        }
        if let Some(r) = parse::parse_line(line) {
            return Some(r.tokens.prompt());
        }
    }
    None
}

pub(crate) fn read_tail(path: &Path, want: u64) -> std::io::Result<(Vec<u8>, bool)> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let n = want.min(len);
    f.seek(SeekFrom::Start(len - n))?;
    let mut buf = Vec::with_capacity(n as usize);
    f.take(n).read_to_end(&mut buf)?;
    Ok((buf, n == len))
}

/// Velikost kontextu souboru: poslední usage od konce + první usage od začátku. Čte disk.
#[cfg(test)]
pub fn read_context(path: &Path) -> Option<Ctx> {
    read_with(path, None).0
}

/// Jako `read_context`, ale známé první volání (`known_first`) nečte znovu z hlavy souboru (až 4 MB).
/// Vrací kontext, první volání a zda se hlava četla.
fn read_with(path: &Path, known_first: Option<u64>) -> (Option<Ctx>, Option<u64>, bool) {
    let mut last = None;
    for step in TAIL_STEPS {
        let Ok((buf, whole)) = read_tail(path, step) else { return (None, known_first, false) };
        last = last_usage(&buf, whole);
        if last.is_some() || whole {
            break;
        }
    }
    if known_first.is_some() {
        return (context(last, known_first), known_first, false);
    }
    let Some(len) = std::fs::metadata(path).ok().map(|m| m.len()) else { return (None, None, false) };
    let first = floor::scan_head(path, len.min(HEAD_MAX), &|_| 0).ok().flatten().map(|f| f.prompt);
    (context(last, first), first, true)
}

/// Transcript smí číst jen z `<.claude>/projects` a jen `*.jsonl` (cesta přichází z hooku). Bez známého kořene
/// se nečte nic; cesty se porovnávají bez ohledu na velikost písmen (Windows).
pub fn allowed(path: &Path, projects_root: Option<&Path>) -> bool {
    let Some(root) = projects_root else { return false };
    let lower = |p: &Path| std::path::PathBuf::from(p.to_string_lossy().to_lowercase());
    path.is_absolute()
        && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("jsonl"))
        && lower(path).starts_with(lower(root))
        && !path.components().any(|c| c.as_os_str() == "..")
}

pub use crate::util::now_ms;

struct Entry {
    mtime: i64,
    len: u64,
    checked_ms: u64,
    ctx: Option<Ctx>,
    /// první volání session (nemění se, nečte se znovu)
    first: Option<u64>,
}

static CACHE: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn stat(path: &Path) -> Option<(i64, u64)> {
    let m = std::fs::metadata(path).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as i64;
    Some((t, m.len()))
}

/// Kontext session z cache; při změně souboru (a ne dřív než `THROTTLE_MS`) znovu přečte konec.
pub fn for_path(path: &Path, now_ms: u64) -> Option<Ctx> {
    let key = path.to_string_lossy().into_owned();
    if let Some(e) = CACHE.lock().get(&key).filter(|e| now_ms.saturating_sub(e.checked_ms) < THROTTLE_MS) {
        return e.ctx;
    }
    let (mtime, len) = stat(path)?;
    if let Some(e) = CACHE.lock().get_mut(&key).filter(|e| e.mtime == mtime && e.len == len) {
        e.checked_ms = now_ms;
        return e.ctx;
    }
    let known_first = CACHE.lock().get(&key).and_then(|e| e.first);
    let (ctx, first, _) = read_with(path, known_first);
    let mut c = CACHE.lock();
    if c.len() >= CACHE_MAX
        && !c.contains_key(&key)
        && let Some(old) = c.iter().min_by_key(|(_, e)| e.checked_ms).map(|(k, _)| k.clone())
    {
        c.remove(&old);
    }
    c.insert(key, Entry { mtime, len, checked_ms: now_ms, ctx, first });
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(rid: &str, input: u64, cw: u64, cr: u64, extra: &str) -> String {
        format!(
            "{{\"type\":\"assistant\",\"requestId\":\"{rid}\",\"timestamp\":\"2026-10-28T10:00:00.000Z\"{extra},\"message\":{{\"id\":\"m-{rid}\",\"model\":\"claude-opus-5-5\",\"usage\":{{\"input_tokens\":{input},\"output_tokens\":9,\"cache_creation_input_tokens\":{cw},\"cache_read_input_tokens\":{cr}}}}}}}\n"
        )
    }

    #[test]
    fn level_and_ratio_tables() {
        let levels = [(0, "ok"), (249_999, "ok"), (250_000, "warn"), (349_999, "warn"), (350_000, "high"), (u64::MAX, "high")];
        for (t, want) in levels {
            assert_eq!(level(t), want, "{t}");
        }
        let ratios = [(60_000, Some(20_000), Some(3.0)), (182_000, Some(54_000), Some(3.4)), (20_000, Some(20_000), Some(1.0)), (5, None, None), (5, Some(0), None)];
        for (n, f, want) in ratios {
            assert_eq!(ratio(n, f), want, "{n} {f:?}");
        }
        assert_eq!(context(None, Some(5)), None);
        assert_eq!(context(Some(0), Some(5)), None, "nulová velikost není údaj");
        assert_eq!(context(Some(300_000), Some(100_000)), Some(Ctx { tokens: 300_000, first: Some(100_000), ratio: Some(3.0), level: "warn" }));
        assert_eq!(context(Some(10), None).map(|c| (c.ratio, c.level)), Some((None, "ok")));
    }

    #[test]
    fn last_usage_table() {
        let a = line("a", 1, 100, 1_000, "");
        let b = line("b", 2, 0, 50_000, "");
        let user = "{\"type\":\"user\",\"message\":{\"content\":\"x\"}}\n";
        let side = line("s", 9, 0, 900_000, ",\"isSidechain\":true");
        let cases: Vec<(&str, String, bool, Option<u64>)> = vec![
            ("poslední z více", format!("{a}{user}{b}{user}"), true, Some(50_002)),
            ("součet vstup + zápis + čtení", a.clone(), true, Some(1_101)),
            ("výstup se nepočítá", line("o", 0, 0, 7, ""), true, Some(7)),
            ("useknutý první řádek se přeskočí", format!("{}{b}", &a[40..]), false, Some(50_002)),
            ("useknutý první řádek je jediný", a[40..].to_string(), false, None),
            ("od začátku se první řádek bere", a.clone(), true, Some(1_101)),
            ("subagent řádek se přeskočí", format!("{a}{side}"), true, Some(1_101)),
            ("rozepsaný poslední řádek", format!("{a}{}", &b[..b.len() / 2]), true, Some(1_101)),
            ("bez usage", user.to_string(), true, None),
            ("prázdné", String::new(), true, None),
        ];
        for (name, body, from_start, want) in cases {
            assert_eq!(last_usage(body.as_bytes(), from_start), want, "{name}");
        }
    }

    #[test]
    fn allowed_paths() {
        let root = Path::new("/h/.claude/projects");
        let cases = [
            ("/h/.claude/projects/p/s.jsonl", true),
            ("/h/.claude/projects/p/s.txt", false),
            ("/h/.claude/other/s.jsonl", false),
            ("/h/.claude/projects/../x/s.jsonl", false),
            ("relative/s.jsonl", false),
            ("/H/.Claude/Projects/p/S.JSONL", true),
        ];
        for (p, want) in cases {
            // „absolutní“ jen na Unixu; na Windows doplníme disk, ať test platí na obou
            let path = if cfg!(windows) && p.starts_with('/') { format!("C:{p}") } else { p.to_string() };
            let r = if cfg!(windows) { Path::new("C:/h/.claude/projects") } else { root };
            assert_eq!(allowed(Path::new(&path), Some(r)), want, "{p}");
        }
        let abs = if cfg!(windows) { "C:/h/.claude/projects/p/s.jsonl" } else { "/h/.claude/projects/p/s.jsonl" };
        assert!(!allowed(Path::new(abs), None), "bez kořene se nečte nic");
    }

    #[test]
    fn reads_tail_and_head_from_disk_and_caches() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        let big = format!("{{\"type\":\"user\",\"message\":{{\"content\":\"{}\"}}}}\n", "x".repeat(400_000));
        // první volání 20k, uprostřed obří řádek (> 256 kB), poslední volání 182k
        std::fs::write(&p, format!("{}{big}{}{big}", line("a", 1, 0, 19_999, ""), line("z", 2, 1_000, 180_998, ""))).unwrap();
        let c = read_context(&p).expect("kontext");
        assert_eq!((c.tokens, c.first, c.ratio, c.level), (182_000, Some(20_000), Some(9.1), "ok"));
        // poslední usage je za obřím řádkem dál než 256 kB od konce → rozšíření výřezu
        std::fs::write(&p, format!("{}{big}{big}", line("a", 1, 0, 19_999, ""))).unwrap();
        assert_eq!(read_context(&p).map(|c| c.tokens), Some(20_000), "našlo se ve větším výřezu");
        // cache: do THROTTLE_MS se soubor nečte (smazaný soubor vrací starou hodnotu), potom se stat opakuje
        std::fs::write(&p, line("a", 1, 0, 299_999, "")).unwrap();
        let t0 = 1_000_000;
        assert_eq!(for_path(&p, t0).map(|c| (c.tokens, c.level)), Some((300_000, "warn")));
        std::fs::write(&p, line("a", 1, 0, 1_399_999, "")).unwrap();
        assert_eq!(for_path(&p, t0 + THROTTLE_MS - 1).map(|c| c.tokens), Some(300_000), "v rámci pauzy cache");
        assert_eq!(for_path(&p, t0 + THROTTLE_MS).map(|c| (c.tokens, c.level)), Some((1_400_000, "high")));
        std::fs::remove_file(&p).unwrap();
        assert_eq!(for_path(&p, t0 + 10 * THROTTLE_MS), None, "chybějící soubor = nic");
        assert_eq!(read_context(&dir.path().join("nic.jsonl")), None);
    }

    #[test]
    fn second_read_skips_head() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(&p, format!("{}{}", line("a", 1, 0, 19_999, ""), line("z", 2, 0, 59_998, ""))).unwrap();
        let (c, first, head) = read_with(&p, None);
        assert!(head, "poprvé se hlava čte");
        assert_eq!((c.map(|c| c.tokens), first), (Some(60_000), Some(20_000)));
        let (c, first, head) = read_with(&p, first);
        assert!(!head, "podruhé se hlava nečte");
        assert_eq!((c.map(|c| (c.tokens, c.ratio)), first), (Some((60_000, Some(3.0))), Some(20_000)));
    }
}
