//! Čisté výpočty karty Systém: CPU % ze dvou vzorků `GetSystemTimes`, rychlost sítě z čítačů, kruhová
//! paměť posledních vzorků, výběr disků a text položky proužku. Žádné I/O ani stav (tabulkové testy dole).

use std::collections::VecDeque;

/// Kolik posledních vzorků drží sparkline (v paměti, nikam se neukládá).
pub const HISTORY: usize = 60;
/// Předchozí vzorek starší než tohle se k výpočtu rychlosti nepoužije (karta byla zavřená — průměr přes
/// minuty by lhal o tom, co se děje teď).
pub const STALE_MS: i64 = 30_000;

/// Součty času procesorů v setinách mikrosekundy (`FILETIME` jako `u64`); `kernel` už obsahuje `idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// Vytížení CPU v % mezi dvěma vzorky; `None` = nic nenaměřeno (stejný okamžik, čítače šly zpět).
pub fn cpu_pct(prev: CpuTimes, cur: CpuTimes) -> Option<f32> {
    let total = (cur.kernel.checked_add(cur.user)?).checked_sub(prev.kernel.checked_add(prev.user)?)?;
    if total == 0 {
        return None;
    }
    let idle = cur.idle.saturating_sub(prev.idle).min(total);
    let pct = (total - idle) as f64 / total as f64 * 100.0;
    Some(round1(pct as f32))
}

fn round1(v: f32) -> f32 {
    ((v * 10.0).round() / 10.0).clamp(0.0, 100.0)
}

/// Rychlost v bajtech za sekundu z dvou stavů čítače; `None` = čítač šel zpět (restart adaptéru, jiná
/// množina adaptérů) nebo uplynul nulový čas.
pub fn rate(prev: u64, cur: u64, dt_ms: i64) -> Option<u64> {
    if dt_ms <= 0 || cur < prev {
        return None;
    }
    Some(((cur - prev) as u128 * 1000 / dt_ms as u128) as u64)
}

/// Je předchozí vzorek dost čerstvý, aby se z něj počítalo?
pub fn fresh(prev_at: i64, now: i64) -> bool {
    let dt = now.saturating_sub(prev_at);
    dt > 0 && dt <= STALE_MS
}

/// Obsazená paměť v %.
pub fn mem_pct(used: u64, total: u64) -> f32 {
    if total == 0 { 0.0 } else { round1(used as f32 / total as f32 * 100.0) }
}

/// Posledních `cap` hodnot (nejstarší první). Nic jiného než `push` a kopie — kreslí to UI.
#[derive(Debug, Clone)]
pub struct Ring<T> {
    cap: usize,
    items: VecDeque<T>,
}

impl<T: Copy> Ring<T> {
    pub const fn new(cap: usize) -> Self {
        Self { cap, items: VecDeque::new() }
    }
    pub fn push(&mut self, v: T) {
        if self.items.len() == self.cap {
            self.items.pop_front();
        }
        self.items.push_back(v);
    }
    pub fn to_vec(&self) -> Vec<T> {
        self.items.iter().copied().collect()
    }
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

/// Pevný disk: písmeno, celková a volná velikost v bajtech.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub letter: String,
    pub total: u64,
    pub free: u64,
    /// disk s Windows (`SystemDrive`)
    pub system: bool,
}

/// Písmena disků z bitové masky `GetLogicalDrives` (bit 0 = A).
pub fn mask_letters(mask: u32) -> Vec<char> {
    (0..26u32).filter(|i| mask & (1 << i) != 0).map(|i| (b'A' + i as u8) as char).collect()
}

/// Písmena z configu: první znak, velké A–Z, bez duplicit, seřazená. Jiné záznamy se zahodí.
pub fn normalize_letters(raw: &[String]) -> Vec<String> {
    let mut v: Vec<char> = raw.iter().filter_map(|s| s.trim().chars().next()).map(|c| c.to_ascii_uppercase()).filter(char::is_ascii_uppercase).collect();
    v.sort_unstable();
    v.dedup();
    v.into_iter().map(String::from).collect()
}

/// Disky, které karta ukáže: `wanted` prázdné = všechny pevné, jinak jen vybraná písmena (která existují).
pub fn pick_disks(all: &[Disk], wanted: &[String]) -> Vec<Disk> {
    let want = normalize_letters(wanted);
    all.iter().filter(|d| want.is_empty() || want.contains(&d.letter)).cloned().collect()
}

/// Jak moc „bolí“ hodnota: tečka u položky proužku (zelená nic neříká, proto žádná).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Calm,
    Warm,
    Hot,
}

pub fn level(cpu: Option<f32>, mem: f32) -> Level {
    let top = cpu.unwrap_or(0.0).max(mem);
    if top >= 90.0 {
        Level::Hot
    } else if top >= 75.0 {
        Level::Warm
    } else {
        Level::Calm
    }
}

/// Text položky proužku: „CPU 23 % · RAM 61 %“ (bez CPU, dokud nejsou dva vzorky: jen paměť).
pub fn strip_text(cpu: Option<f32>, mem: f32) -> String {
    match cpu {
        Some(c) => format!("CPU {} % · RAM {} %", c.round() as u32, mem.round() as u32),
        None => format!("RAM {} %", mem.round() as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(idle: u64, kernel: u64, user: u64) -> CpuTimes {
        CpuTimes { idle, kernel, user }
    }

    #[test]
    fn cpu_pct_from_two_samples() {
        let cases: [(&str, CpuTimes, CpuTimes, Option<f32>); 8] = [
            // za 1000 ticků: 250 nečinnost → 75 % vytížení (kernel zahrnuje idle)
            ("čtvrtina nečinná", t(0, 0, 0), t(250, 600, 400), Some(75.0)),
            ("úplně nečinný", t(0, 0, 0), t(1000, 1000, 0), Some(0.0)),
            ("plně vytížený", t(0, 0, 0), t(0, 300, 700), Some(100.0)),
            ("od nenulového základu", t(5_000, 8_000, 2_000), t(5_100, 8_150, 2_050), Some(50.0)),
            ("zaokrouhlení na desetinu", t(0, 0, 0), t(1, 2, 1), Some(66.7)),
            ("stejný okamžik", t(10, 20, 30), t(10, 20, 30), None),
            ("čítače šly zpět", t(100, 200, 300), t(10, 20, 30), None),
            ("idle větší než součet se ořízne", t(0, 0, 0), t(500, 100, 100), Some(0.0)),
        ];
        for (name, a, b, want) in cases {
            assert_eq!(cpu_pct(a, b), want, "{name}");
        }
    }

    #[test]
    fn net_rate_from_counters() {
        let cases: [(&str, u64, u64, i64, Option<u64>); 6] = [
            ("1 MB za 2 s", 0, 2_000_000, 2000, Some(1_000_000)),
            ("půl sekundy", 1000, 1500, 500, Some(1000)),
            ("nic nepřišlo", 5, 5, 2000, Some(0)),
            ("čítač šel zpět", 100, 50, 2000, None),
            ("nulový čas", 0, 100, 0, None),
            ("velké číslo bez přetečení", 0, u64::MAX / 2, 1000, Some(u64::MAX / 2)),
        ];
        for (name, a, b, dt, want) in cases {
            assert_eq!(rate(a, b, dt), want, "{name}");
        }
    }

    #[test]
    fn stale_previous_sample_is_not_used() {
        assert!(fresh(1_000, 3_000));
        assert!(fresh(1_000, 31_000));
        assert!(!fresh(1_000, 31_001), "starší než 30 s");
        assert!(!fresh(5_000, 5_000), "stejný okamžik");
        assert!(!fresh(5_000, 4_000), "hodiny šly zpět");
    }

    #[test]
    fn ring_keeps_the_last_cap_values() {
        let mut r = Ring::new(3);
        for v in 1..=5 {
            r.push(v);
        }
        assert_eq!(r.to_vec(), vec![3, 4, 5]);
        r.clear();
        assert!(r.to_vec().is_empty());
        let mut full = Ring::new(HISTORY);
        for v in 0..(HISTORY * 2) {
            full.push(v as f32);
        }
        assert_eq!(full.to_vec().len(), HISTORY);
        assert_eq!(full.to_vec()[0], HISTORY as f32);
    }

    #[test]
    fn drive_letters_and_choice() {
        assert_eq!(mask_letters(0b101), vec!['A', 'C']);
        assert_eq!(mask_letters(0), Vec::<char>::new());
        assert_eq!(mask_letters(1 << 25), vec!['Z']);
        let strs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(normalize_letters(&strs(&["d", " c:", "D", "", "1", "č", "E\\"])), strs(&["C", "D", "E"]));
        let disk = |l: &str, system: bool| Disk { letter: l.into(), total: 100, free: 40, system };
        let all = [disk("C", true), disk("D", false), disk("E", false)];
        let letters = |v: Vec<Disk>| v.into_iter().map(|d| d.letter).collect::<Vec<_>>();
        assert_eq!(letters(pick_disks(&all, &[])), strs(&["C", "D", "E"]), "prázdný výběr = všechny pevné");
        assert_eq!(letters(pick_disks(&all, &strs(&["e", "c"]))), strs(&["C", "E"]));
        assert_eq!(letters(pick_disks(&all, &strs(&["Z"]))), Vec::<String>::new(), "neexistující disk se neukáže");
    }

    #[test]
    fn strip_text_and_level() {
        assert_eq!(strip_text(Some(23.4), 61.2), "CPU 23 % · RAM 61 %");
        assert_eq!(strip_text(Some(99.6), 100.0), "CPU 100 % · RAM 100 %");
        assert_eq!(strip_text(None, 40.0), "RAM 40 %");
        let cases = [(Some(10.0), 20.0, Level::Calm), (Some(75.0), 10.0, Level::Warm), (None, 80.0, Level::Warm), (Some(95.0), 10.0, Level::Hot), (Some(10.0), 90.0, Level::Hot)];
        for (c, m, want) in cases {
            assert_eq!(level(c, m), want, "{c:?} {m}");
        }
        assert_eq!(mem_pct(8, 16), 50.0);
        assert_eq!(mem_pct(0, 0), 0.0);
    }
}
