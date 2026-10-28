//! Inkrementální index spotřeby nad transcripty Claude Code. Na disku (`%LOCALAPPDATA%\Ledge\
//! usage-index.json` + `usage-index.seen`): per soubor offset + délka + mtime, agregace
//! den × cwd × model × pásmo, po hodinách za dnešek a včerejšek (časová osa karty Claude dnes, kolo 22)
//! a množina viděných klíčů odpovědí (jen pro dny v okně).
//!
//! Pravidla výkonu: soubor s mtime před začátkem okna se neotevře (jen stat z výpisu složky);
//! nezměněný soubor (délka i mtime) se neotevře; změněný se čte od uloženého offsetu a jen celé
//! řádky (poslední bez `\n` se přečte příště). Kratší soubor než offset = celý index se přepočítá.
//! Žádné I/O mimo `list_files` / `update` / `load` / `save`; volá je jen vlákno `costslive`.

use super::floor;
use super::parse::{self, Tokens};
use super::price;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Verze formátu indexu — jiná na disku = přepočet od nuly. 2 = přibyly hodiny (`hours`, kolo 22):
/// starý index je nemá, takže dnešní časová osa by byla neúplná → raději jednou přepočítat.
pub const VERSION: u32 = 2;
/// Kolik dní zpět (včetně dneška) se drží po hodinách — dnešek + včerejšek (přes půlnoc), víc karta neukazuje.
pub const HOUR_DAYS: i32 = 2;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    /// kam až jsou zpracované celé řádky
    pub off: u64,
    pub len: u64,
    pub mtime: i64,
    /// poslední `cwd` viděné v souboru (záloha pro záznamy bez vlastního)
    #[serde(default)]
    pub cwd: Option<String>,
    /// první odpověď API v souboru (podlaha nové relace, kolo 23 — `floor.rs`); starý index ji nemá
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<floor::First>,
    /// `first` a `agent` už jsou zjištěné (jinak je doplní `floor::fill` — jen čtením začátku souboru)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub first_done: bool,
    /// `agentType` z `agent-*.meta.json` (jen soubory subagentů)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Key {
    /// místní den (dny od 1970-01-01)
    pub day: i32,
    pub cwd: String,
    /// kanonické id modelu (`price::canonical`)
    pub model: String,
    pub tier: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Sum {
    pub t: Tokens,
    /// počet odpovědí API
    pub n: u64,
}

/// Klíč hodinové agregace: místní den, místní hodina 0–23, `cwd`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HourKey {
    pub day: i32,
    pub hour: u8,
    pub cwd: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Index {
    pub window_days: u32,
    pub files: BTreeMap<String, FileEntry>,
    pub agg: HashMap<Key, Sum>,
    /// po hodinách, jen posledních `HOUR_DAYS` dní okna
    pub hours: HashMap<HourKey, Sum>,
    /// klíč odpovědi → den (den kvůli pročištění starých)
    pub seen: HashMap<u64, i32>,
}

#[derive(Serialize, Deserialize)]
struct Disk {
    v: u32,
    window_days: u32,
    files: BTreeMap<String, FileEntry>,
    agg: Vec<(Key, Sum)>,
    #[serde(default)]
    hours: Vec<(HourKey, Sum)>,
}

/// Soubor z výpisu složky: cesta, délka, mtime (ms).
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub path: PathBuf,
    pub len: u64,
    pub mtime: i64,
}

/// Okno přepočtu: záznamy a soubory před `cutoff_ms` se nepočítají; `day_of` = místní den z ms.
pub struct Window<'a> {
    pub days: u32,
    pub cutoff_ms: i64,
    pub cutoff_day: i32,
    pub day_of: &'a dyn Fn(i64) -> i32,
    /// místní hodina 0–23 z ms
    pub hour_of: &'a dyn Fn(i64) -> u8,
}

impl Window<'_> {
    /// Dnešek = poslední den okna.
    pub fn today(&self) -> i32 {
        self.cutoff_day + self.days as i32 - 1
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub files: usize,
    /// starší než okno — neotevřené
    pub skipped_old: usize,
    /// beze změny — neotevřené
    pub unchanged: usize,
    pub opened: usize,
    /// doplněno podlahou (`floor::fill`) — start souboru / typ agenta, bez přepočtu
    pub filled: usize,
    pub bytes: u64,
    pub records: u64,
    pub duplicates: u64,
    pub rebuilt: bool,
}

fn mtime_ms(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64)
}

/// Všechny `*.jsonl` pod `root` (rekurzivně: session, `subagents/`, `subagents/workflows/…`).
/// Metadata z výpisu složky — soubory se neotvírají.
pub fn list_files(root: &Path) -> Vec<Found> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let path = e.path();
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() && path.extension().is_some_and(|x| x == "jsonl")
                && let Ok(m) = e.metadata()
            {
                out.push(Found { path, len: m.len(), mtime: mtime_ms(&m) });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

impl Index {
    fn clear(&mut self, days: u32) {
        *self = Index { window_days: days, ..Index::default() };
    }

    fn prune(&mut self, w: &Window) {
        self.agg.retain(|k, _| k.day >= w.cutoff_day);
        let first_hour_day = w.today() - HOUR_DAYS + 1;
        self.hours.retain(|k, _| k.day >= first_hour_day);
        self.seen.retain(|_, d| *d >= w.cutoff_day);
    }

    /// Přidá záznam (dedup podle klíče). Vrací `false` u duplikátu nebo záznamu mimo okno.
    pub fn add(&mut self, r: &parse::Rec, file_cwd: Option<&str>, w: &Window) -> Option<bool> {
        if r.ts_ms < w.cutoff_ms {
            return None;
        }
        let day = (w.day_of)(r.ts_ms);
        if day < w.cutoff_day {
            return None;
        }
        if self.seen.contains_key(&r.key) {
            return Some(false);
        }
        self.seen.insert(r.key, day);
        let model = price::canonical(&r.model);
        let tier = price::tier(&model, &r.tokens, r.fast, r.geo_us);
        let cwd = r.cwd.as_deref().or(file_cwd).unwrap_or_default().to_string();
        if day > w.today() - HOUR_DAYS {
            let h = self.hours.entry(HourKey { day, hour: (w.hour_of)(r.ts_ms).min(23), cwd: cwd.clone() }).or_default();
            h.t.add(&r.tokens);
            h.n += 1;
        }
        let s = self.agg.entry(Key { day, cwd, model, tier }).or_default();
        s.t.add(&r.tokens);
        s.n += 1;
        Some(true)
    }

    /// Přepočet nad výpisem souborů. `progress(přečteno, celkem)` v bajtech k přečtení.
    pub fn update(&mut self, found: &[Found], w: &Window, progress: &mut dyn FnMut(u64, u64)) -> Stats {
        let mut st = Stats { files: found.len(), ..Stats::default() };
        let live: Vec<&Found> = found.iter().filter(|f| f.mtime >= w.cutoff_ms).collect();
        st.skipped_old = found.len() - live.len();
        // jiné okno nebo zkrácený soubor (přepsaný, ne připsaný) = přepočet od nuly
        let shrunk = live.iter().any(|f| self.files.get(&key(&f.path)).is_some_and(|e| f.len < e.off));
        if self.window_days != w.days || shrunk {
            self.clear(w.days);
            st.rebuilt = true;
        }
        self.prune(w);
        // soubory mimo okno / smazané z indexu ven (index nebobtná)
        let names: std::collections::HashSet<String> = live.iter().map(|f| key(&f.path)).collect();
        self.files.retain(|k, _| names.contains(k));
        let todo: Vec<(&Found, FileEntry)> = live
            .iter()
            .filter_map(|f| {
                let e = self.files.get(&key(&f.path)).cloned().unwrap_or_default();
                (e.len != f.len || e.mtime != f.mtime || !e.first_done).then_some((*f, e))
            })
            .collect();
        st.unchanged = live.len() - todo.len();
        let total: u64 = todo.iter().map(|(f, e)| f.len.saturating_sub(e.off)).sum();
        let mut done = 0u64;
        progress(0, total);
        for (f, mut e) in todo {
            st.opened += 1;
            let before = e.off;
            if self.read_file(f, &mut e, w, &mut st).is_err() {
                continue; // nečitelný soubor (zamčený, smazaný mezitím) — zkusí se příště
            }
            st.bytes += e.off - before;
            e.len = f.len;
            e.mtime = f.mtime;
            self.files.insert(key(&f.path), e);
            done += f.len.saturating_sub(before);
            progress(done.min(total), total);
        }
        st
    }

    fn read_file(&mut self, f: &Found, e: &mut FileEntry, w: &Window, st: &mut Stats) -> std::io::Result<()> {
        if !e.first_done {
            floor::fill(&f.path, e, w.day_of)?;
            st.filled += 1;
        }
        let mut file = std::fs::File::open(&f.path)?;
        file.seek(SeekFrom::Start(e.off))?;
        // jen do délky z výpisu: co se připsalo mezitím, přečte příští běh (mtime se změní)
        let mut r = BufReader::with_capacity(256 * 1024, file.take(f.len.saturating_sub(e.off)));
        let mut line = Vec::with_capacity(64 * 1024);
        loop {
            let n = match read_line_capped(&mut r, &mut line, MAX_LINE)? {
                Line::End => break, // konec nebo useknutý poslední řádek — offset zůstane před ním
                Line::Skipped(n) => {
                    e.off += n; // řádek nad strop (obří výstup nástroje): přeskočen bez načtení do paměti
                    continue;
                }
                Line::Whole(n) => n,
            };
            e.off += n;
            if let Some(rec) = parse::parse_line(&line) {
                if e.first.is_none() {
                    e.first = Some(floor::first_of(&rec, (w.day_of)(rec.ts_ms)));
                }
                if rec.cwd.is_some() {
                    e.cwd.clone_from(&rec.cwd);
                }
                if e.cwd.is_none() {
                    e.cwd = parse::line_cwd(&line);
                }
                match self.add(&rec, e.cwd.as_deref(), w) {
                    Some(true) => st.records += 1,
                    Some(false) => st.duplicates += 1,
                    None => {}
                }
            } else if e.cwd.is_none() {
                e.cwd = parse::line_cwd(&line);
            }
        }
        Ok(())
    }
}

/// Nejdelší řádek transcriptu, který se parsuje (8 MB). Delší (obří výstup nástroje, vložený soubor) se přeskočí:
/// `read_until` by ho jinak celý nacpal do paměti a Ledge má držet pod 25 MB.
pub const MAX_LINE: usize = 8 * 1024 * 1024;

/// Výsledek čtení jednoho řádku.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Line {
    /// konec vstupu, nebo poslední řádek bez `\n` (příště)
    End,
    /// celý řádek (s `\n`) v bufferu; počet přečtených bajtů
    Whole(u64),
    /// řádek delší než strop, přeskočený (buffer prázdný); počet přečtených bajtů včetně `\n`
    Skipped(u64),
}

/// Přečte jeden řádek do `buf` (smaže ho předem), ale nikdy neukládá víc než `max` bajtů: delší řádek dočte po
/// kouscích bez ukládání a vrátí `Skipped`. Paměť je tedy omezená `max` + velikostí bufferu čtečky.
pub(super) fn read_line_capped(r: &mut impl BufRead, buf: &mut Vec<u8>, max: usize) -> std::io::Result<Line> {
    buf.clear();
    let (mut total, mut skipping) = (0u64, false);
    loop {
        let chunk = r.fill_buf()?;
        if chunk.is_empty() {
            buf.clear();
            return Ok(Line::End);
        }
        let (take, done) = match chunk.iter().position(|b| *b == b'\n') {
            Some(i) => (i + 1, true),
            None => (chunk.len(), false),
        };
        if !skipping && buf.len() + take > max {
            skipping = true;
            buf.clear();
        }
        if !skipping {
            buf.extend_from_slice(&chunk[..take]);
        }
        r.consume(take);
        total += take as u64;
        if done {
            return Ok(if skipping { Line::Skipped(total) } else { Line::Whole(total) });
        }
    }
}

fn key(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn json_path(dir: &Path) -> PathBuf {
    dir.join("usage-index.json")
}

pub fn seen_path(dir: &Path) -> PathBuf {
    dir.join("usage-index.seen")
}

/// Načte index ze složky; chybějící / rozbitý / jiná verze = prázdný (přepočet).
pub fn load(dir: &Path) -> Index {
    let Some(d) = std::fs::read(json_path(dir)).ok().and_then(|b| serde_json::from_slice::<Disk>(&b).ok()).filter(|d| d.v == VERSION) else {
        return Index::default();
    };
    let Some(seen) = std::fs::read(seen_path(dir)).ok().and_then(|b| decode_seen(&b)) else {
        return Index::default(); // bez množiny viděných by se přírůstky počítaly dvakrát
    };
    Index { window_days: d.window_days, files: d.files, agg: d.agg.into_iter().collect(), hours: d.hours.into_iter().collect(), seen }
}

/// Uloží index (tmp + rename, nejdřív `.seen`, pak `.json`).
pub fn save(dir: &Path, ix: &Index) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut agg: Vec<(Key, Sum)> = ix.agg.iter().map(|(k, v)| (k.clone(), *v)).collect();
    agg.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hours: Vec<(HourKey, Sum)> = ix.hours.iter().map(|(k, v)| (k.clone(), *v)).collect();
    hours.sort_by(|a, b| a.0.cmp(&b.0));
    let disk = Disk { v: VERSION, window_days: ix.window_days, files: ix.files.clone(), agg, hours };
    write_atomic(&seen_path(dir), &encode_seen(&ix.seen))?;
    write_atomic(&json_path(dir), &serde_json::to_vec(&disk).map_err(std::io::Error::other)?)
}

fn write_atomic(p: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = p.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, p)
}

const SEEN_MAGIC: &[u8; 4] = b"LUS1";

/// `LUS1` + opakovaně (u64 klíč, i32 den) little-endian — 12 B na odpověď.
pub fn encode_seen(seen: &HashMap<u64, i32>) -> Vec<u8> {
    let mut v: Vec<(&u64, &i32)> = seen.iter().collect();
    v.sort();
    let mut out = Vec::with_capacity(4 + v.len() * 12);
    out.extend_from_slice(SEEN_MAGIC);
    for (k, d) in v {
        out.extend_from_slice(&k.to_le_bytes());
        out.extend_from_slice(&d.to_le_bytes());
    }
    out
}

pub fn decode_seen(b: &[u8]) -> Option<HashMap<u64, i32>> {
    let body = b.strip_prefix(SEEN_MAGIC)?;
    if body.len() % 12 != 0 {
        return None;
    }
    let (chunks, _) = body.as_chunks::<12>();
    Some(chunks.iter().map(|c| (u64::from_le_bytes(c[..8].try_into().unwrap()), i32::from_le_bytes(c[8..].try_into().unwrap()))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_line_capped_table() {
        // (vstup, strop, očekávané výsledky po řádcích: (výsledek, obsah bufferu))
        type Case<'a> = (&'a [u8], usize, Vec<(Line, &'a str)>);
        let cases: [Case; 5] = [
            (b"ab\ncd\n", 10, vec![(Line::Whole(3), "ab\n"), (Line::Whole(3), "cd\n"), (Line::End, "")]),
            (b"ab\ncd", 10, vec![(Line::Whole(3), "ab\n"), (Line::End, "")]),
            (b"ab\n0123456789X\nzz\n", 5, vec![(Line::Whole(3), "ab\n"), (Line::Skipped(12), ""), (Line::Whole(3), "zz\n")]),
            (b"0123456789", 5, vec![(Line::End, "")]),
            (b"\n\n", 5, vec![(Line::Whole(1), "\n"), (Line::Whole(1), "\n")]),
        ];
        for (input, max, want) in cases {
            // malý buffer čtečky = řádek se čte po kouscích (jako velký řádek v souboru)
            let mut r = BufReader::with_capacity(4, input);
            let mut buf = Vec::new();
            for (res, text) in want {
                assert_eq!(read_line_capped(&mut r, &mut buf, max).unwrap(), res, "{input:?}");
                assert_eq!(String::from_utf8_lossy(&buf), text, "{input:?}");
                assert!(buf.len() <= max, "buffer nad strop");
            }
        }
    }

    #[test]
    fn read_line_capped_never_holds_more_than_max() {
        // 1 MB řádek se stropem 1 kB: buffer nikdy neroste nad strop, následující řádek se přečte
        let mut data = vec![b'x'; 1_000_000];
        data.extend_from_slice(b"\nok\n");
        let mut r = BufReader::with_capacity(8 * 1024, &data[..]);
        let mut buf = Vec::new();
        assert_eq!(read_line_capped(&mut r, &mut buf, 1024).unwrap(), Line::Skipped(1_000_001));
        assert!(buf.capacity() < 100_000, "nealokuje se pro obří řádek");
        assert_eq!(read_line_capped(&mut r, &mut buf, 1024).unwrap(), Line::Whole(3));
        assert_eq!(buf, b"ok\n");
    }

    const DAY: i64 = 86_400_000;
    /// „místní“ čas pro testy: UTC+2 (půlnoc je v 22:00 UTC)
    const OFF: i64 = 2 * 3_600_000;
    fn day_of(ms: i64) -> i32 {
        (ms + OFF).div_euclid(DAY) as i32
    }
    fn hour_of(ms: i64) -> u8 {
        ((ms + OFF).rem_euclid(DAY) / 3_600_000) as u8
    }
    /// 2026-10-17 12:00 UTC
    const NOW: i64 = 1_792_238_400_000;

    fn win(days: u32) -> (i32, i64) {
        let today = day_of(NOW);
        let cutoff_day = today - days as i32 + 1;
        (cutoff_day, cutoff_day as i64 * DAY - OFF)
    }

    fn with_win<R>(days: u32, f: impl FnOnce(&Window) -> R) -> R {
        let (cutoff_day, cutoff_ms) = win(days);
        f(&Window { days, cutoff_ms, cutoff_day, day_of: &day_of, hour_of: &hour_of })
    }

    fn iso(ms: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ms).unwrap().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    fn rec_line(rid: &str, ts: i64, cwd: &str, model: &str, out: u64) -> String {
        format!(
            "{{\"type\":\"assistant\",\"requestId\":\"{rid}\",\"timestamp\":\"{}\",\"cwd\":{},\"message\":{{\"id\":\"m-{rid}\",\"model\":\"{model}\",\"usage\":{{\"input_tokens\":1,\"output_tokens\":{out},\"cache_read_input_tokens\":100}}}}}}\n",
            iso(ts),
            serde_json::to_string(cwd).unwrap()
        )
    }

    fn user_line() -> String {
        "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"content\":\"\\\"usage\\\" assistant\"}]}}\n".into()
    }

    struct Fx {
        dir: tempfile::TempDir,
    }

    impl Fx {
        fn new() -> Self {
            Fx { dir: tempfile::tempdir().unwrap() }
        }
        fn path(&self, rel: &str) -> PathBuf {
            self.dir.path().join(rel)
        }
        fn write(&self, rel: &str, body: &str) {
            let p = self.path(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        fn append(&self, rel: &str, body: &str) {
            let mut f = std::fs::OpenOptions::new().append(true).open(self.path(rel)).unwrap();
            f.write_all(body.as_bytes()).unwrap();
        }
        /// výpis s mtime přepsaným na `mtime` (testy nesahají na skutečné časy souborů)
        fn found(&self, mtime: &dyn Fn(&Path) -> i64) -> Vec<Found> {
            list_files(self.dir.path()).into_iter().map(|mut f| {
                f.mtime = mtime(&f.path);
                f
            }).collect()
        }
    }

    fn total_out(ix: &Index) -> u64 {
        ix.agg.values().map(|s| s.t.output).sum()
    }

    fn run(ix: &mut Index, found: &[Found], days: u32) -> Stats {
        with_win(days, |w| ix.update(found, w, &mut |_, _| {}))
    }

    #[test]
    fn dedup_within_file_and_across_resume_copy() {
        let fx = Fx::new();
        let a = rec_line("req_A", NOW - 3_600_000, "C:\\p\\ledge", "claude-opus-5-5", 10);
        // stejná odpověď 3× (streamované bloky) + jiná odpověď
        fx.write("C--p-ledge/s1.jsonl", &format!("{a}{}{a}{a}{}", user_line(), rec_line("req_B", NOW - 60_000, "C:\\p\\ledge", "claude-opus-5-5", 5)));
        // resume kopie v jiném souboru (i v podsložce subagentů) nese tytéž odpovědi
        fx.write("C--p-ledge/s2.jsonl", &format!("{a}{}", rec_line("req_C", NOW - 30_000, "C:\\p\\ledge", "claude-sonnet-5-5", 7)));
        fx.write("C--p-ledge/s1/subagents/agent-x.jsonl", &a);
        fx.write("C--p-ledge/s1/tool-results/x.txt", &a);
        let found = fx.found(&|_| NOW);
        assert_eq!(found.len(), 3, "jen *.jsonl, i v podsložkách");
        let mut ix = Index::default();
        let st = run(&mut ix, &found, 30);
        assert_eq!(total_out(&ix), 10 + 5 + 7);
        assert_eq!((st.records, st.duplicates), (3, 4));
        let n: u64 = ix.agg.values().map(|s| s.n).sum();
        assert_eq!(n, 3);
    }

    #[test]
    fn increments_from_offset_and_torn_line() {
        let fx = Fx::new();
        let l1 = rec_line("r1", NOW - 1000, "C:\\p", "claude-opus-5-5", 1);
        let l2 = rec_line("r2", NOW - 900, "C:\\p", "claude-opus-5-5", 20);
        let l3 = rec_line("r3", NOW - 800, "C:\\p", "claude-opus-5-5", 300);
        let rel = "proj/s.jsonl";
        // poslední řádek bez \n = Claude Code ho zrovna píše → nečíst
        fx.write(rel, &format!("{l1}{}", l2.trim_end()));
        let mut ix = Index::default();
        let mut t = NOW;
        let st = run(&mut ix, &fx.found(&|_| t), 30);
        assert_eq!(total_out(&ix), 1);
        assert_eq!(ix.files.values().next().unwrap().off, l1.len() as u64);
        assert_eq!(st.opened, 1);

        // dopsaný konec řádku + nový řádek → čte od offsetu, nic dvakrát
        fx.append(rel, &format!("\n{l3}"));
        t += 1;
        let st = run(&mut ix, &fx.found(&|_| t), 30);
        assert_eq!(total_out(&ix), 321);
        assert_eq!(st.bytes, (l2.len() + l3.len()) as u64, "jen přírůstek");

        // beze změny → neotevře se
        let st = run(&mut ix, &fx.found(&|_| t), 30);
        assert_eq!((st.opened, st.unchanged), (0, 1));

        // zkrácený soubor = přepočet od nuly
        fx.write(rel, &l3);
        t += 1;
        let st = run(&mut ix, &fx.found(&|_| t), 30);
        assert!(st.rebuilt);
        assert_eq!(total_out(&ix), 300);
    }

    #[test]
    fn old_files_skipped_without_open_and_old_records_dropped() {
        let fx = Fx::new();
        let (_, cutoff_ms) = win(30);
        fx.write("p/old.jsonl", &rec_line("o", cutoff_ms - 5000, "C:\\p", "claude-opus-5-5", 1000));
        // soubor v okně, ale se starším záznamem na začátku (dlouhá session)
        fx.write("p/new.jsonl", &format!("{}{}", rec_line("o2", cutoff_ms - 1, "C:\\p", "claude-opus-5-5", 50), rec_line("n", cutoff_ms, "C:\\p", "claude-opus-5-5", 7)));
        let found = fx.found(&|p| if p.ends_with("old.jsonl") { cutoff_ms - 1 } else { NOW });
        // starý soubor neexistuje-li pro otevření, nesmí to vadit: smažeme ho, výpis ho ale nese
        std::fs::remove_file(fx.path("p/old.jsonl")).unwrap();
        let mut ix = Index::default();
        let st = run(&mut ix, &found, 30);
        assert_eq!((st.skipped_old, st.opened), (1, 1));
        assert_eq!(total_out(&ix), 7, "záznam před půlnocí začátku okna se nepočítá");
        assert_eq!(ix.files.len(), 1);
    }

    #[test]
    fn aggregates_by_local_day_and_project() {
        let fx = Fx::new();
        let today = day_of(NOW);
        let midnight = today as i64 * DAY - OFF; // místní půlnoc dneška v UTC ms
        fx.write(
            "p/s.jsonl",
            &[
                rec_line("a", midnight - 1, "C:\\p\\a", "claude-opus-5-5", 1),
                rec_line("b", midnight, "C:\\p\\a", "claude-opus-5-5", 2),
                rec_line("c", midnight + 5, "C:\\p\\b", "claude-opus-5-5", 4),
                rec_line("d", midnight + 6, "C:\\p\\b", "claude-sonnet-5-5-20261001", 8),
            ]
            .concat(),
        );
        let mut ix = Index::default();
        run(&mut ix, &fx.found(&|_| NOW), 30);
        let get = |day: i32, cwd: &str, model: &str| ix.agg.get(&Key { day, cwd: cwd.into(), model: model.into(), tier: 0 }).map(|s| s.t.output);
        assert_eq!(get(today - 1, "C:\\p\\a", "claude-opus-5-5"), Some(1), "minuta před místní půlnocí = včera");
        assert_eq!(get(today, "C:\\p\\a", "claude-opus-5-5"), Some(2));
        assert_eq!(get(today, "C:\\p\\b", "claude-opus-5-5"), Some(4));
        assert_eq!(get(today, "C:\\p\\b", "claude-sonnet-5-5"), Some(8), "model kanonicky bez data");
    }

    #[test]
    fn cwd_fallback_from_file() {
        let fx = Fx::new();
        let no_cwd = "{\"type\":\"assistant\",\"requestId\":\"x\",\"timestamp\":\"2026-10-17T11:00:00Z\",\"message\":{\"id\":\"m\",\"model\":\"claude-opus-5-5\",\"usage\":{\"output_tokens\":3}}}\n";
        fx.write("p/s.jsonl", &format!("{{\"type\":\"user\",\"cwd\":\"C:\\\\w\"}}\n{no_cwd}"));
        let mut ix = Index::default();
        run(&mut ix, &fx.found(&|_| NOW), 30);
        assert_eq!(ix.agg.keys().next().unwrap().cwd, "C:\\w");
    }

    #[test]
    fn window_change_and_prune() {
        let fx = Fx::new();
        fx.write("p/s.jsonl", &format!("{}{}", rec_line("a", NOW - 10 * DAY, "C:\\p", "m", 1), rec_line("b", NOW, "C:\\p", "m", 2)));
        let mut ix = Index::default();
        run(&mut ix, &fx.found(&|_| NOW), 30);
        assert_eq!(total_out(&ix), 3);
        let st = run(&mut ix, &fx.found(&|_| NOW), 7);
        assert!(st.rebuilt, "jiné okno = přepočet");
        assert_eq!(total_out(&ix), 2);
        assert_eq!(ix.seen.len(), 1);
    }

    #[test]
    fn save_load_roundtrip_and_broken_files() {
        let fx = Fx::new();
        fx.write("p/s.jsonl", &rec_line("a", NOW, "C:\\p", "claude-opus-5-5", 9));
        let mut ix = Index::default();
        run(&mut ix, &fx.found(&|_| NOW), 30);
        let store = fx.path("store");
        save(&store, &ix).unwrap();
        assert_eq!(load(&store), ix);
        // po načtení se nic nepočítá dvakrát
        let mut again = load(&store);
        let st = run(&mut again, &fx.found(&|_| NOW + 1), 30);
        assert_eq!((st.opened, st.duplicates), (1, 0), "jen mtime změněn, offset na konci");
        assert_eq!(total_out(&again), 9);
        std::fs::write(seen_path(&store), b"LUS1abc").unwrap();
        assert_eq!(load(&store), Index::default(), "rozbitá množina = přepočet");
        std::fs::write(json_path(&store), b"{").unwrap();
        assert_eq!(load(&store), Index::default());
        assert_eq!(load(&fx.path("nikde")), Index::default());
        let mut s = HashMap::new();
        s.insert(u64::MAX, -3);
        s.insert(7, 20_000);
        assert_eq!(decode_seen(&encode_seen(&s)), Some(s));
        assert_eq!(decode_seen(b"XXXX"), None);
    }

    #[test]
    fn hours_today_and_yesterday_only() {
        let fx = Fx::new();
        let h = 3_600_000;
        // NOW = 14:00 místního času (UTC+2); řádky: dnes 13:xx, dnes 9:xx, včera 23:xx, předevčírem
        let today_13 = NOW - 30 * 60_000;
        let today_9 = NOW - 5 * h;
        let yday_23 = NOW - 14 * h - 30 * 60_000;
        let older = NOW - 2 * DAY;
        fx.write(
            "p/s.jsonl",
            &format!(
                "{}{}{}{}{}",
                rec_line("a", today_13, "C:/p/app", "claude-opus-5-5", 10),
                rec_line("b", today_13 + 60_000, "C:/p/app", "claude-opus-5-5", 5),
                rec_line("c", today_9, "C:/p/app-v2", "claude-opus-5-5", 7),
                rec_line("d", yday_23, "C:/p/app", "claude-opus-5-5", 3),
                rec_line("e", older, "C:/p/app", "claude-opus-5-5", 1),
            ),
        );
        let mut ix = Index::default();
        run(&mut ix, &fx.found(&|_| NOW), 30);
        let today = day_of(NOW);
        let got = |day: i32, hour: u8, cwd: &str| ix.hours.get(&HourKey { day, hour, cwd: cwd.into() }).map(|s| (s.t.output, s.n));
        assert_eq!(got(today, 13, "C:/p/app"), Some((15, 2)), "dvě odpovědi ve 13:xx");
        assert_eq!(got(today, 9, "C:/p/app-v2"), Some((7, 1)));
        assert_eq!(got(today - 1, 23, "C:/p/app"), Some((3, 1)), "včerejšek zůstává (přes půlnoc)");
        assert_eq!(ix.hours.len(), 3, "předevčírem po hodinách ne");
        assert_eq!(total_out(&ix), 26, "denní součty beze změny");
        // další den: včerejšek se stane předevčírem a vypadne
        let next = NOW + DAY;
        let (cutoff_day, cutoff_ms) = {
            let t = day_of(next);
            let c = t - 30 + 1;
            (c, c as i64 * DAY - OFF)
        };
        ix.update(&fx.found(&|_| NOW), &Window { days: 30, cutoff_ms, cutoff_day, day_of: &day_of, hour_of: &hour_of }, &mut |_, _| {});
        assert!(ix.hours.keys().all(|k| k.day >= today), "drží se jen dva dny");
        // uložení a načtení drží hodiny
        let store = fx.path("store");
        save(&store, &ix).unwrap();
        assert_eq!(load(&store).hours, ix.hours);
    }

    #[test]
    fn old_index_version_is_rebuilt() {
        let fx = Fx::new();
        let store = fx.path("store");
        std::fs::create_dir_all(&store).unwrap();
        // index verze 1 (bez hodin) z dřívějšího Ledge = prázdný → přepočet od nuly
        std::fs::write(json_path(&store), br#"{"v":1,"window_days":30,"files":{},"agg":[]}"#).unwrap();
        std::fs::write(seen_path(&store), encode_seen(&HashMap::new())).unwrap();
        assert_eq!(load(&store), Index::default());
    }
}
