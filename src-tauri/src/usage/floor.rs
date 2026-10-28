//! Podlaha nové relace a agenta (kolo 23, E3): kontext PRVNÍHO volání API každého souboru transcriptu
//! (session = „kolik váží start v tomhle projektu“, subagent = „kolik váží start tohoto typu agenta“) —
//! medián za posledních 7 dní proti předchozím 7 dnům. Nafouklý CLAUDE.md / MCP / skills je vidět hned
//! po změně. První usage souboru se zapisuje do indexu při jeho běžném čtení (`index::read_file`, bez
//! druhého průchodu); soubory ze starého indexu se doplní jen čtením začátku (`fill`), ne přepočtem.
//! Čisté až na `scan_head` / `agent_type` (čtou jeden soubor; volá je jen vlákno `costslive`).

use super::index::{FileEntry, Index, Line, MAX_LINE, read_line_capped};
use super::parse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::path::Path;

/// Délka jednoho období trendu ve dnech (tento týden × předchozí týden).
pub const PERIOD_DAYS: i32 = 7;
/// Nejméně souborů v obou obdobích, aby se ukázal trend (menší vzorek = šum).
pub const MIN_TREND: u64 = 3;
/// Kolik projektů / typů agentů ukázat (nejvíc použité první).
pub const ROWS: usize = 6;
/// Strop `.meta.json` subagenta (skutečné mají ~300 B).
const META_MAX: u64 = 16 * 1024;

/// První odpověď API v souboru: místní den a délka promptu (vstup + zápis + čtení cache).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct First {
    pub day: i32,
    pub prompt: u64,
}

pub fn first_of(r: &parse::Rec, day: i32) -> First {
    First { day, prompt: r.tokens.prompt() }
}

/// Soubor subagenta: `…/subagents/…/agent-*.jsonl`.
pub fn is_subagent(path: &str) -> bool {
    let p = path.replace('\\', "/");
    let name = p.rsplit('/').next().unwrap_or("");
    name.starts_with("agent-") && p.split('/').rev().skip(1).any(|c| c == "subagents")
}

/// `agentType` z `agent-*.meta.json` vedle transcriptu; chybí / rozbité / prázdné = `None`.
pub fn agent_type(jsonl: &Path) -> Option<String> {
    if !is_subagent(&jsonl.to_string_lossy()) {
        return None;
    }
    let mut body = Vec::new();
    std::fs::File::open(jsonl.with_extension("meta.json")).ok()?.take(META_MAX).read_to_end(&mut body).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&body).ok()?;
    clean_type(v.get("agentType")?.as_str()?)
}

/// Název typu agenta do tabulky: bez řídicích znaků a okrajových mezer, nejvýš 60 znaků.
pub fn clean_type(s: &str) -> Option<String> {
    let t: String = s.chars().filter(|c| !c.is_control()).collect();
    let t = t.trim();
    (!t.is_empty()).then(|| t.chars().take(60).collect())
}

/// První odpověď v prvních `upto` bajtech souboru (celé řádky). Čte jen začátek a končí u první nalezené.
pub fn scan_head(path: &Path, upto: u64, day_of: &dyn Fn(i64) -> i32) -> std::io::Result<Option<First>> {
    let mut r = BufReader::with_capacity(64 * 1024, std::fs::File::open(path)?.take(upto));
    let mut line = Vec::with_capacity(32 * 1024);
    loop {
        match read_line_capped(&mut r, &mut line, MAX_LINE)? {
            Line::End => return Ok(None),
            Line::Skipped(_) => {}
            Line::Whole(_) => {
                if let Some(rec) = parse::parse_line(&line) {
                    return Ok(Some(first_of(&rec, day_of(rec.ts_ms))));
                }
            }
        }
    }
}

/// Doplní položku, kterou index ještě nezná z hlediska podlahy (starý index, nový soubor): u souboru, který se
/// už četl, projde jen začátek; u všech načte typ agenta. Chyba čtení = položka zůstane nedoplněná (příště znovu).
pub fn fill(path: &Path, e: &mut FileEntry, day_of: &dyn Fn(i64) -> i32) -> std::io::Result<()> {
    if e.first.is_none() && e.off > 0 {
        e.first = scan_head(path, e.off, day_of)?;
    }
    e.agent = agent_type(path);
    e.first_done = true;
    Ok(())
}

/// Medián (u sudého počtu průměr dvou prostředních, zaokrouhlený dolů); prázdné = `None`. Bez přetečení.
pub fn med(v: &mut [u64]) -> Option<u64> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    let m = v.len() / 2;
    Some(if v.len() % 2 == 1 { v[m] } else { (v[m - 1] & v[m]) + ((v[m - 1] ^ v[m]) >> 1) })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorRow {
    /// projekt (z `cwd`) nebo typ agenta
    pub name: String,
    /// medián délky promptu prvního volání za posledních 7 dní
    pub median: u64,
    /// kolik souborů (session / agentů) za 7 dní
    pub n: u64,
    /// totéž za předchozích 7 dní (`None` = žádný soubor)
    pub prev_median: Option<u64>,
    pub prev_n: u64,
    /// rozdíl `median - prev_median`; jen když je v obou obdobích aspoň `MIN_TREND` souborů
    pub delta: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Floor {
    pub projects: Vec<FloorRow>,
    pub agents: Vec<FloorRow>,
}

type Groups = HashMap<String, (Vec<u64>, Vec<u64>)>;

fn rows(groups: Groups) -> Vec<FloorRow> {
    let mut out: Vec<FloorRow> = groups
        .into_iter()
        .filter_map(|(name, (mut cur, mut prev))| {
            let (n, prev_n) = (cur.len() as u64, prev.len() as u64);
            let median = med(&mut cur)?;
            let prev_median = med(&mut prev);
            let delta = prev_median.filter(|_| n >= MIN_TREND && prev_n >= MIN_TREND).map(|p| median as i64 - p as i64);
            Some(FloorRow { name, median, n, prev_median, prev_n, delta })
        })
        .collect();
    out.sort_by(|a, b| b.n.cmp(&a.n).then(b.median.cmp(&a.median)).then(a.name.cmp(&b.name)));
    out.truncate(ROWS);
    out
}

/// Index → podlaha. `today` = dnešní místní den; týden = `today-6 … today`, předchozí `today-13 … today-7`.
/// Projekt podle `cwd` dodá volající; subagent bez `.meta.json` se nepočítá (typ neznáme).
pub fn build(ix: &Index, today: i32, project_of: &dyn Fn(&str) -> String) -> Floor {
    let mut projects = Groups::new();
    let mut agents = Groups::new();
    let mut names: HashMap<&str, String> = HashMap::new();
    for (path, e) in &ix.files {
        let Some(f) = e.first else { continue };
        let age = today - f.day;
        if !(0..2 * PERIOD_DAYS).contains(&age) {
            continue;
        }
        let slot = if is_subagent(path) {
            let Some(t) = e.agent.clone() else { continue };
            agents.entry(t).or_default()
        } else {
            let cwd = e.cwd.as_deref().unwrap_or_default();
            let name = names.entry(cwd).or_insert_with(|| project_of(cwd)).clone();
            projects.entry(name).or_default()
        };
        if age < PERIOD_DAYS {
            slot.0.push(f.prompt);
        } else {
            slot.1.push(f.prompt);
        }
    }
    Floor { projects: rows(projects), agents: rows(agents) }
}

#[cfg(test)]
mod tests {
    use super::super::index::{Found, Window};
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn median_table() {
        let cases: &[(&[u64], Option<u64>)] = &[
            (&[], None),
            (&[7], Some(7)),
            (&[9, 1, 5], Some(5)),
            (&[10, 20], Some(15)),
            (&[1, 2, 3, 10], Some(2)),
            (&[u64::MAX, u64::MAX], Some(u64::MAX)),
            (&[1, 2], Some(1)),
            (&[3, 4], Some(3)),
        ];
        for (v, want) in cases {
            assert_eq!(med(&mut v.to_vec()), *want, "{v:?}");
        }
    }

    #[test]
    fn subagent_path_and_type_table() {
        let cases = [
            ("C:\\u\\.claude\\projects\\p\\s1\\subagents\\agent-a1.jsonl", true),
            ("/h/.claude/projects/p/s1/subagents/workflows/w/agent-a1.jsonl", true),
            ("/h/.claude/projects/p/s1.jsonl", false),
            ("/h/.claude/projects/p/subagents/s1.jsonl", false),
            ("/h/.claude/projects/p/agent-a1.jsonl", false),
        ];
        for (p, want) in cases {
            assert_eq!(is_subagent(p), want, "{p}");
        }
        assert_eq!(clean_type("  general-purpose "), Some("general-purpose".into()));
        assert_eq!(clean_type("\u{1b}x"), Some("x".into()));
        assert_eq!(clean_type(" \n"), None);
        assert_eq!(clean_type(&"x".repeat(100)).unwrap().chars().count(), 60);
    }

    fn entry(day: i32, prompt: u64, cwd: &str, agent: Option<&str>) -> FileEntry {
        FileEntry { first: Some(First { day, prompt }), first_done: true, cwd: Some(cwd.into()), agent: agent.map(Into::into), ..FileEntry::default() }
    }

    fn ix(files: Vec<(String, FileEntry)>) -> Index {
        Index { files: files.into_iter().collect::<BTreeMap<_, _>>(), ..Index::default() }
    }

    fn proj(cwd: &str) -> String {
        match cwd {
            "C:\\p\\app" | "C:\\p\\app-wt" => "app".into(),
            "" => String::new(),
            other => other.rsplit('\\').next().unwrap().into(),
        }
    }

    #[test]
    fn build_groups_periods_and_trend() {
        let t = 1000;
        let mut files: Vec<(String, FileEntry)> = Vec::new();
        // app: tento týden 20k, 30k, 40k, 50k (medián 35k, worktree se slučuje); minulý týden 10k×3
        for (i, (cwd, p)) in [("C:\\p\\app", 20_000), ("C:\\p\\app-wt", 30_000), ("C:\\p\\app", 40_000), ("C:\\p\\app", 50_000)].into_iter().enumerate() {
            files.push((format!("p/s{i}.jsonl"), entry(t - i as i32, p, cwd, None)));
        }
        for i in 0..3 {
            files.push((format!("p/old{i}.jsonl"), entry(t - 7 - i, 10_000, "C:\\p\\app", None)));
        }
        // other: 1 soubor tento týden, 1 minulý → bez trendu
        files.push(("p/o1.jsonl".into(), entry(t, 5_000, "C:\\p\\other", None)));
        files.push(("p/o2.jsonl".into(), entry(t - 8, 4_000, "C:\\p\\other", None)));
        // starší než 14 dní, z budoucnosti, bez první odpovědi
        files.push(("p/ancient.jsonl".into(), entry(t - 14, 99, "C:\\p\\old", None)));
        files.push(("p/future.jsonl".into(), entry(t + 1, 99, "C:\\p\\future", None)));
        files.push(("p/none.jsonl".into(), FileEntry { first_done: true, ..FileEntry::default() }));
        // subagenti: typ z meta, bez meta se nepočítá, do projektů nejdou
        files.push(("p/s0/subagents/agent-1.jsonl".into(), entry(t, 60_000, "C:\\p\\app", Some("general-purpose"))));
        files.push(("p/s0/subagents/agent-2.jsonl".into(), entry(t - 1, 64_000, "C:\\p\\app", Some("general-purpose"))));
        files.push(("p/s0/subagents/agent-3.jsonl".into(), entry(t, 8_000, "C:\\p\\app", Some("pruzkumnik"))));
        files.push(("p/s0/subagents/agent-4.jsonl".into(), entry(t, 1, "C:\\p\\app", None)));
        let f = build(&ix(files), t, &proj);
        let names: Vec<_> = f.projects.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["app", "other"], "podle počtu; ancient/future/none mimo");
        assert_eq!(f.projects[0], FloorRow { name: "app".into(), median: 35_000, n: 4, prev_median: Some(10_000), prev_n: 3, delta: Some(25_000) });
        assert_eq!(f.projects[1], FloorRow { name: "other".into(), median: 5_000, n: 1, prev_median: Some(4_000), prev_n: 1, delta: None });
        assert_eq!(f.agents.len(), 2);
        assert_eq!(f.agents[0], FloorRow { name: "general-purpose".into(), median: 62_000, n: 2, prev_median: None, prev_n: 0, delta: None });
        assert_eq!(f.agents[1].name, "pruzkumnik");
        assert_eq!(build(&Index::default(), t, &proj), Floor::default());
    }

    #[test]
    fn rows_cap_and_order() {
        let t = 50;
        let files: Vec<(String, FileEntry)> = (0..9).map(|i| (format!("p/s{i}.jsonl"), entry(t, 1000 + i as u64, &format!("C:\\p\\n{i}"), None))).collect();
        let f = build(&ix(files), t, &proj);
        assert_eq!(f.projects.len(), ROWS);
        assert_eq!(f.projects[0].median, 1008, "stejný počet → větší medián první");
    }

    // ---- integrace s indexem nad tempdir ----
    const DAY: i64 = 86_400_000;
    const NOW: i64 = 1_792_238_400_000;
    fn day_of(ms: i64) -> i32 {
        (ms + 7_200_000).div_euclid(DAY) as i32
    }
    fn hour_of(ms: i64) -> u8 {
        ((ms + 7_200_000).rem_euclid(DAY) / 3_600_000) as u8
    }
    fn win() -> Window<'static> {
        let cutoff_day = day_of(NOW) - 29;
        Window { days: 30, cutoff_ms: cutoff_day as i64 * DAY - 7_200_000, cutoff_day, day_of: &day_of, hour_of: &hour_of }
    }
    fn line(rid: &str, ts: i64, input: u64, cr: u64) -> String {
        let iso = chrono::DateTime::from_timestamp_millis(ts).unwrap().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        format!(
            "{{\"type\":\"assistant\",\"requestId\":\"{rid}\",\"timestamp\":\"{iso}\",\"cwd\":\"C:\\\\p\\\\app\",\"message\":{{\"id\":\"m-{rid}\",\"model\":\"claude-opus-5-5\",\"usage\":{{\"input_tokens\":{input},\"output_tokens\":5,\"cache_read_input_tokens\":{cr}}}}}}}\n"
        )
    }
    fn found(dir: &Path) -> Vec<Found> {
        super::super::index::list_files(dir)
            .into_iter()
            .map(|mut f| {
                f.mtime = NOW;
                f
            })
            .collect()
    }

    #[test]
    fn fresh_pass_records_first_and_agent_type() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("app/s1/subagents");
        std::fs::create_dir_all(&sub).unwrap();
        let user = "{\"type\":\"user\",\"message\":{\"content\":\"x\"}}\n";
        std::fs::write(dir.path().join("app/s1.jsonl"), format!("{user}{}{}", line("r1", NOW - 5 * DAY, 3, 20_000), line("r2", NOW - DAY, 3, 90_000))).unwrap();
        std::fs::write(sub.join("agent-a.jsonl"), line("r3", NOW - 3_600_000, 2, 60_000)).unwrap();
        std::fs::write(sub.join("agent-a.meta.json"), r#"{"agentType":"general-purpose","model":"opus"}"#).unwrap();
        std::fs::write(sub.join("agent-b.jsonl"), line("r4", NOW - 3_600_000, 2, 9_000)).unwrap(); // bez meta
        let mut ix = Index::default();
        let st = ix.update(&found(dir.path()), &win(), &mut |_, _| {});
        assert_eq!((st.opened, st.filled, st.records), (3, 3, 4), "první průchod: nic se nečte dvakrát");
        let f = build(&ix, day_of(NOW), &proj);
        assert_eq!(f.projects.len(), 1);
        assert_eq!((f.projects[0].name.as_str(), f.projects[0].median, f.projects[0].n), ("app", 20_003, 1), "session: první volání 20 003; subagenti do projektů nejdou");
        assert_eq!(f.agents.iter().map(|r| (r.name.as_str(), r.median)).collect::<Vec<_>>(), [("general-purpose", 60_002)], "agent-b bez meta se nepočítá");
        // druhý běh beze změny nic neotevře
        let st = ix.update(&found(dir.path()), &win(), &mut |_, _| {});
        assert_eq!((st.opened, st.filled), (0, 0));
    }

    #[test]
    fn old_index_entries_are_filled_by_head_scan_without_recount() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        let body = format!("{}{}", line("r1", NOW - 2 * DAY, 4, 30_000), line("r2", NOW - DAY, 4, 80_000));
        let path = dir.path().join("app").join("s1.jsonl");
        std::fs::write(&path, &body).unwrap();
        // položka ve tvaru ze starého indexu (před kolem 23): žádné pole podlahy
        let old: FileEntry = serde_json::from_str(&format!("{{\"off\":{0},\"len\":{0},\"mtime\":{NOW},\"cwd\":\"C:\\\\p\\\\app\"}}", body.len())).unwrap();
        assert!(old.first.is_none() && !old.first_done && old.agent.is_none());
        let mut ix = Index { window_days: 30, ..Index::default() };
        ix.files.insert(path.to_string_lossy().into_owned(), old);
        let before = ix.agg.clone();
        let st = ix.update(&found(dir.path()), &win(), &mut |_, _| {});
        assert_eq!((st.filled, st.bytes, st.records, st.rebuilt), (1, 0, 0, false), "jen začátek souboru, žádný přepočet");
        assert_eq!(ix.agg, before, "agregace beze změny");
        assert_eq!(ix.files.values().next().unwrap().first, Some(First { day: day_of(NOW - 2 * DAY), prompt: 30_004 }));
        // uložení + načtení zachová pole a druhý běh už nic nedoplňuje
        let store = tempfile::tempdir().unwrap();
        super::super::index::save(store.path(), &ix).unwrap();
        let mut back = super::super::index::load(store.path());
        assert_eq!(back.files, ix.files);
        assert_eq!(back.update(&found(dir.path()), &win(), &mut |_, _| {}).filled, 0);
    }

    #[test]
    fn file_without_usage_yet_picks_first_later() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        let path = dir.path().join("app").join("s1.jsonl");
        std::fs::write(&path, "{\"type\":\"user\",\"message\":{\"content\":\"x\"}}\n").unwrap();
        let mut ix = Index::default();
        ix.update(&found(dir.path()), &win(), &mut |_, _| {});
        assert!(ix.files.values().next().unwrap().first.is_none());
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        std::io::Write::write_all(&mut f, line("r1", NOW - 3_600_000, 1, 15_000).as_bytes()).unwrap();
        std::io::Write::write_all(&mut f, line("r2", NOW - 1_800_000, 1, 99_000).as_bytes()).unwrap();
        drop(f); // NTFS: velikost ve výpisu složky se srovná až po zavření souboru
        let mut fo = found(dir.path());
        fo[0].mtime = NOW + 1;
        ix.update(&fo, &win(), &mut |_, _| {});
        assert_eq!(ix.files.values().next().unwrap().first.map(|f| f.prompt), Some(15_001), "první, ne poslední");
    }
}
