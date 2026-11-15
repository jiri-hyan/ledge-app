//! Poctivá cena rozšíření (kolo 33, E1): RAM a CPU celého Jobu (proces `--ext` + jeho podprocesy, tedy i WebView2).
//! Měří se JEN při volání `ext_status` (UI ho volá po 5 s, dokud je vidět blok Rozšíření v Nastavení) — žádný
//! časovač, žádné vlákno. Definice shodné s `scripts/perf.ps1`: RAM = private bytes (`PrivateUsage`) součtem přes
//! procesy, CPU = % celého stroje (Δ CPU času / Δ času / počet jader). CPU čas bere z účetnictví Jobu
//! (`TotalUserTime + TotalKernelTime` zahrnuje i už skončené procesy). PID: seznam z Jobu + potomci hlavního
//! procesu mimo Job (kdyby se podproces z Jobu vymanil) — kolik jich je mimo, hlásí `outside_job`.
//! GPU se neukazuje (bez PDH nejde poctivě; zůstává v `perf-wall.ps1`).

use crate::procman::Job;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::JobObjects::{
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
    QueryInformationJobObject,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

/// Nejkratší odstup dvou vzorků CPU; kratší měření CPU neukáže (šum), vrátí „—“.
const MIN_CPU_DT_MS: u64 = 1000;

/// Změřená cena jednoho běžícího rozšíření.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    /// součet private bytes přes všechny procesy rozšíření
    pub ram_bytes: u64,
    /// % celého stroje od minulého měření; `None` = ještě nemáme dva vzorky (UI ukáže „—“)
    pub cpu_pct: Option<f64>,
    /// počet procesů v součtu
    pub procs: u32,
    /// kolik z nich je potomek mimo Job (0 = Job drží celý strom)
    pub outside_job: u32,
}

/// Minulý vzorek CPU času Jobu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prev {
    /// identita Jobu (nový běh = nový Job = čítač začíná od nuly)
    pub job: usize,
    pub at_ms: u64,
    pub cpu_100ns: u64,
}

/// % celého stroje: Δ CPU čas (jednotky 100 ns) / Δ čas / počet jader. `None` při nesmyslu (čas nejde dopředu,
/// čítač klesl, 0 jader, příliš krátký odstup). Výsledek se ořízne na 0–100.
pub fn cpu_pct(prev_100ns: u64, now_100ns: u64, dt_ms: u64, cores: u32) -> Option<f64> {
    if dt_ms < MIN_CPU_DT_MS || cores == 0 || now_100ns < prev_100ns {
        return None;
    }
    let cpu_ms = (now_100ns - prev_100ns) as f64 / 10_000.0;
    Some((cpu_ms / dt_ms as f64 / cores as f64 * 100.0).clamp(0.0, 100.0))
}

/// Další vzorek: z minulého (je-li pro tentýž Job) spočte CPU %, vrátí nový „minulý“. Odstup kratší než
/// `MIN_CPU_DT_MS` minulý vzorek nepřepíše (další volání porovná s ním, ne s čerstvým).
pub fn next_sample(prev: Option<Prev>, job: usize, now_ms: u64, cpu_100ns: u64, cores: u32) -> (Option<f64>, Prev) {
    let cur = Prev { job, at_ms: now_ms, cpu_100ns };
    match prev {
        Some(p) if p.job == job && now_ms >= p.at_ms => {
            let dt = now_ms - p.at_ms;
            if dt < MIN_CPU_DT_MS { (None, p) } else { (cpu_pct(p.cpu_100ns, cpu_100ns, dt, cores), cur) }
        }
        _ => (None, cur),
    }
}

/// Součet private bytes přes procesy (`None` = proces mezitím skončil / nejde otevřít → nepočítá se).
/// Vrací (bajty, počet započtených).
pub fn job_sum(samples: &[Option<u64>]) -> (u64, u32) {
    samples.iter().flatten().fold((0u64, 0u32), |(b, n), s| (b.saturating_add(*s), n + 1))
}

/// Potomci `root` podle tabulky (pid, rodič) — bez `root`, bez cyklů, každý jednou.
pub fn descendants(root: u32, table: &[(u32, u32)]) -> Vec<u32> {
    let mut seen: HashSet<u32> = HashSet::from([root]);
    let mut out = Vec::new();
    let mut frontier = vec![root];
    while let Some(p) = frontier.pop() {
        for &(pid, parent) in table {
            if parent == p && seen.insert(pid) {
                out.push(pid);
                frontier.push(pid);
            }
        }
    }
    out
}

/// PID Jobu + potomci `root` mimo Job: (všechny PID bez duplicit, kolik jich je mimo Job).
pub fn merge_pids(in_job: &[u32], tree: &[u32]) -> (Vec<u32>, u32) {
    let have: HashSet<u32> = in_job.iter().copied().collect();
    let outside: Vec<u32> = tree.iter().copied().filter(|p| !have.contains(p)).collect();
    let n = outside.len() as u32;
    let mut all = in_job.to_vec();
    all.extend(outside);
    (all, n)
}

// ---------- WinAPI ----------

/// PID všech procesů v Jobu a jeho celkový CPU čas (100 ns). `None` = dotaz selhal.
fn query_job(job: &Job) -> Option<(Vec<u32>, u64)> {
    unsafe {
        let mut acc = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        QueryInformationJobObject(
            Some(job.raw()),
            JobObjectBasicAccountingInformation,
            &mut acc as *mut _ as *mut _,
            std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            None,
        )
        .ok()?;
        let cpu = (acc.TotalUserTime.max(0) as u64).saturating_add(acc.TotalKernelTime.max(0) as u64);
        // `JOBOBJECT_BASIC_PROCESS_ID_LIST`: hlavička 2× u32 (= jedno slovo `usize`) a pole usize — buffer o `cap` PID s rezervou
        let cap = (acc.ActiveProcesses as usize + 16).max(32);
        let mut buf = vec![0usize; 1 + cap];
        QueryInformationJobObject(
            Some(job.raw()),
            JobObjectBasicProcessIdList,
            buf.as_mut_ptr() as *mut _,
            (buf.len() * std::mem::size_of::<usize>()) as u32,
            None,
        )
        .ok()?;
        let n = (*(buf.as_ptr() as *const u32).add(1) as usize).min(cap);
        Some((buf[1..1 + n].iter().map(|p| *p as u32).collect(), cpu))
    }
}

/// Private bytes procesu (`PrivateUsage`); `None` = nejde otevřít / skončil.
fn private_bytes(pid: u32) -> Option<u64> {
    unsafe {
        let h: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut c = PROCESS_MEMORY_COUNTERS_EX { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32, ..Default::default() };
        let ok = GetProcessMemoryInfo(h, &mut c as *mut _ as *mut PROCESS_MEMORY_COUNTERS, c.cb).is_ok();
        let _ = CloseHandle(h);
        ok.then_some(c.PrivateUsage as u64)
    }
}

/// Tabulka (pid, rodič) všech procesů — jeden snímek ToolHelp.
fn process_table() -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snap, &mut e).is_ok();
        while more {
            out.push((e.th32ProcessID, e.th32ParentProcessID));
            more = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    out
}

fn prev_map() -> &'static Mutex<HashMap<String, Prev>> {
    static M: OnceLock<Mutex<HashMap<String, Prev>>> = OnceLock::new();
    M.get_or_init(Default::default)
}

/// Zapomene minulý vzorek (rozšíření neběží) — další běh začne bez CPU „—“.
pub fn forget(id: &str) {
    prev_map().lock().remove(id);
}

/// Změří rozšíření `id` běžící v `job` s hlavním procesem `root_pid`. `None` = Job nejde dotázat.
pub fn measure(id: &str, job: &Arc<Job>, root_pid: u32) -> Option<Cost> {
    let (in_job, cpu) = query_job(job)?;
    let tree = descendants(root_pid, &process_table());
    let (pids, outside_job) = merge_pids(&in_job, &tree);
    let (ram_bytes, procs) = job_sum(&pids.iter().map(|p| private_bytes(*p)).collect::<Vec<_>>());
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get() as u32);
    let now = crate::state::now_ms().max(0) as u64;
    let mut g = prev_map().lock();
    let (cpu_pct, next) = next_sample(g.get(id).copied(), Arc::as_ptr(job) as usize, now, cpu, cores);
    g.insert(id.to_string(), next);
    Some(Cost { ram_bytes, cpu_pct, procs, outside_job })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_pct_table() {
        // 1 s času, 1 jádro, 0,5 s CPU = 50 %; 4 jádra = 12,5 %
        assert_eq!(cpu_pct(0, 5_000_000, 1000, 1), Some(50.0));
        assert_eq!(cpu_pct(0, 5_000_000, 1000, 4), Some(12.5));
        assert_eq!(cpu_pct(1_000_000, 1_000_000, 5000, 8), Some(0.0));
        // 5 s, 8 jader, 0,1 s CPU = 0,25 %
        assert_eq!(cpu_pct(0, 1_000_000, 5000, 8), Some(0.25));
        // nesmysly: čítač klesl, krátký odstup, 0 jader, nulový čas
        assert_eq!(cpu_pct(9, 3, 5000, 8), None);
        assert_eq!(cpu_pct(0, 5, 999, 8), None);
        assert_eq!(cpu_pct(0, 5, 5000, 0), None);
        assert_eq!(cpu_pct(0, 5, 0, 8), None);
        // víc vláken než jader (měřicí šum) se ořízne na 100
        assert_eq!(cpu_pct(0, 100_000_000, 1000, 1), Some(100.0));
    }

    #[test]
    fn next_sample_table() {
        let key = |p: Prev| (p.job, p.at_ms, p.cpu_100ns);
        // první vzorek: bez CPU, ale uloží se
        let (c, p1) = next_sample(None, 7, 10_000, 0, 4);
        assert_eq!((c, key(p1)), (None, (7, 10_000, 0)));
        // za 5 s a 1 s CPU na 4 jádrech = 5 %
        let (c, p2) = next_sample(Some(p1), 7, 15_000, 10_000_000, 4);
        assert_eq!(c, Some(5.0));
        assert_eq!(p2.at_ms, 15_000);
        // příliš brzy: bez CPU a minulý vzorek zůstane
        let (c, p3) = next_sample(Some(p2), 7, 15_400, 11_000_000, 4);
        assert_eq!((c, p3), (None, p2));
        // jiný Job (restart rozšíření): bez CPU, nový základ
        let (c, p4) = next_sample(Some(p2), 8, 20_000, 500, 4);
        assert_eq!((c, key(p4)), (None, (8, 20_000, 500)));
        // hodiny šly zpět: bez CPU
        assert_eq!(next_sample(Some(p2), 7, 100, 0, 4).0, None);
    }

    #[test]
    fn job_sum_skips_dead_processes() {
        assert_eq!(job_sum(&[]), (0, 0));
        assert_eq!(job_sum(&[Some(100), None, Some(50)]), (150, 2));
        assert_eq!(job_sum(&[Some(u64::MAX), Some(5)]), (u64::MAX, 2));
    }

    #[test]
    fn descendants_and_merge() {
        // 1 → 2 → 4, 1 → 3, 9 → 10 (cizí strom), cyklus 5 ↔ 6 mimo kořen
        let t = [(2, 1), (3, 1), (4, 2), (10, 9), (5, 6), (6, 5)];
        let mut d = descendants(1, &t);
        d.sort();
        assert_eq!(d, [2, 3, 4]);
        assert!(descendants(99, &t).is_empty());
        // Job drží 1 a 2; potomci 3 a 4 jsou mimo
        let (all, outside) = merge_pids(&[1, 2], &d);
        assert_eq!((all, outside), (vec![1, 2, 3, 4], 2));
        assert_eq!(merge_pids(&[1, 2, 3], &[2, 3]).1, 0);
    }

    /// Skutečný Job se skutečným procesem: PID je v seznamu, RAM > 0, první měření bez CPU.
    #[test]
    fn measures_real_job() {
        use std::process::{Command, Stdio};
        let job = Arc::new(Job::new().expect("job"));
        let mut c = Command::new("ping");
        c.args(["-n", "4", "127.0.0.1"]).stdout(Stdio::null()).stderr(Stdio::null());
        let mut child = job.spawn(&mut c, crate::procman::CREATE_NO_WINDOW).expect("spawn");
        let (pids, _cpu) = query_job(&job).expect("query job");
        assert!(pids.contains(&child.id()), "{pids:?}");
        let cost = measure("test-measure", &job, child.id()).expect("measure");
        assert!(cost.procs >= 1 && cost.ram_bytes > 0, "{cost:?}");
        assert_eq!(cost.cpu_pct, None, "první měření nemá CPU");
        // `ping` je konzolový: Windows mu může přidat `conhost.exe`, který občas (pod zátěží) ještě není v Jobu → ≤ 1.
        assert!(cost.outside_job <= 1, "{cost:?}");
        forget("test-measure");
        job.kill();
        let _ = child.wait();
    }
}
