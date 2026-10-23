//! Karta Systém: CPU %, paměť, volné místo na discích a rychlost sítě z WinAPI (`win.rs`, žádné WMI,
//! žádný proces). Vzorkuje se JEN když o to někdo stojí: karta je vidět (UI volá `system_view` po 2 s,
//! jen s otevřeným šuplíkem) nebo je zapnutá položka proužku `system` (plánovač `integrations.rs`,
//! 1× za 10 s). Sparkline drží posledních 60 vzorků karty v paměti, nikam se neukládá.
//! Čisté výpočty a testy jsou v `calc.rs`, config v `conf.rs`.

pub mod calc;
pub mod conf;
mod win;

use crate::strip::{Rgba, Segment};
use calc::{CpuTimes, Disk, HISTORY, Level, Ring};
pub use conf::SystemConfig;
use parking_lot::Mutex;
use serde::Serialize;

/// Nejkratší odstup dvou skutečných vzorků (karta + proužek spolu): druhé volání vrátí poslední vzorek.
const MIN_GAP_MS: i64 = 1500;
/// Disky se čtou nejvýš jednou za tuhle dobu (volné místo se hýbe pomalu a čtení může budit pevný disk).
const DISKS_EVERY_MS: i64 = 30_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemView {
    /// kdy byl vzorek vzat (ms epochy)
    pub at_ms: i64,
    /// vytížení CPU v %; `None` = čeká se na druhý vzorek
    pub cpu: Option<f32>,
    pub mem_used: u64,
    pub mem_total: u64,
    pub mem_pct: f32,
    pub disks: Vec<Disk>,
    /// rychlost sítě v bajtech za sekundu (příjem / odesílání); `None` = čeká se na druhý vzorek
    pub rx: Option<u64>,
    pub tx: Option<u64>,
    pub cpu_hist: Vec<f32>,
    pub mem_hist: Vec<f32>,
    pub rx_hist: Vec<u64>,
    pub tx_hist: Vec<u64>,
    /// jednotky rychlosti sítě z configu (`system.netUnit`), ať karta nečte config zvlášť
    pub net_unit: conf::NetUnit,
    /// písmena VŠECH pevných disků (Nastavení z nich skládá výběr; `disks` je už podle configu vyfiltrované)
    pub all_disks: Vec<String>,
}

#[derive(Clone, Copy)]
struct Last {
    at: i64,
    cpu: Option<f32>,
    mem_used: u64,
    mem_total: u64,
    rx: Option<u64>,
    tx: Option<u64>,
}

struct State {
    prev_cpu: Option<(CpuTimes, i64)>,
    prev_net: Option<((u64, u64), i64)>,
    last: Option<Last>,
    disks: Vec<Disk>,
    disks_at: i64,
    /// kdy se naposled zapsalo do historie (stará historie se po pauze zahodí)
    hist_at: i64,
    cpu_h: Ring<f32>,
    mem_h: Ring<f32>,
    rx_h: Ring<u64>,
    tx_h: Ring<u64>,
}

static STATE: Mutex<State> = Mutex::new(State {
    prev_cpu: None,
    prev_net: None,
    last: None,
    disks: Vec::new(),
    disks_at: i64::MIN,
    hist_at: i64::MIN,
    cpu_h: Ring::new(HISTORY),
    mem_h: Ring::new(HISTORY),
    rx_h: Ring::new(HISTORY),
    tx_h: Ring::new(HISTORY),
});

fn build(st: &State, last: &Last, cfg: &SystemConfig) -> SystemView {
    SystemView {
        at_ms: last.at,
        cpu: last.cpu,
        mem_used: last.mem_used,
        mem_total: last.mem_total,
        mem_pct: calc::mem_pct(last.mem_used, last.mem_total),
        disks: calc::pick_disks(&st.disks, &cfg.disks),
        rx: last.rx,
        tx: last.tx,
        cpu_hist: st.cpu_h.to_vec(),
        mem_hist: st.mem_h.to_vec(),
        rx_hist: st.rx_h.to_vec(),
        tx_hist: st.tx_h.to_vec(),
        net_unit: cfg.net_unit,
        all_disks: st.disks.iter().map(|d| d.letter.clone()).collect(),
    }
}

/// Vezme vzorek (nebo vrátí čerstvý, který už je) a vrátí pohled. `record` = zapsat do historie
/// sparklinů (jen vzorky karty; vzorky proužku po 10 s by osu zkreslily).
pub fn poll(now: i64, record: bool, cfg: &SystemConfig) -> Option<SystemView> {
    let disks_due = {
        let st = STATE.lock();
        if let Some(l) = st.last.filter(|l| now >= l.at && now - l.at < MIN_GAP_MS) {
            return Some(build(&st, &l, cfg));
        }
        now.saturating_sub(st.disks_at) >= DISKS_EVERY_MS || now < st.disks_at
    };
    // čtení mimo zámek: WinAPI může chvíli trvat, plánovač a karta se nemají blokovat
    let cpu_now = win::cpu_times();
    let (mem_total, mem_avail) = win::memory()?;
    let net_now = win::net_totals();
    let disks = disks_due.then(win::fixed_disks);

    let mut st = STATE.lock();
    let cpu = match (cpu_now, st.prev_cpu) {
        (Some(c), Some((p, at))) if calc::fresh(at, now) => calc::cpu_pct(p, c),
        _ => None,
    };
    st.prev_cpu = cpu_now.map(|c| (c, now));
    let (rx, tx) = match (net_now, st.prev_net) {
        (Some((r, t)), Some(((pr, pt), at))) if calc::fresh(at, now) => (calc::rate(pr, r, now - at), calc::rate(pt, t, now - at)),
        _ => (None, None),
    };
    st.prev_net = net_now.map(|n| (n, now));
    if let Some(d) = disks {
        st.disks = d;
        st.disks_at = now;
    }
    let mem_used = mem_total.saturating_sub(mem_avail);
    let last = Last { at: now, cpu, mem_used, mem_total, rx, tx };
    st.last = Some(last);
    if record {
        if !calc::fresh(st.hist_at, now) {
            st.cpu_h.clear();
            st.mem_h.clear();
            st.rx_h.clear();
            st.tx_h.clear();
        }
        st.hist_at = now;
        if let Some(c) = cpu {
            st.cpu_h.push(c);
        }
        st.mem_h.push(calc::mem_pct(mem_used, mem_total));
        if let (Some(r), Some(t)) = (rx, tx) {
            st.rx_h.push(r);
            st.tx_h.push(t);
        }
    }
    Some(build(&st, &last, cfg))
}

/// Kdy je na řadě další vzorek pro proužek: `every_ms` po posledním (vzorky karty se počítají taky,
/// takže s otevřenou kartou proužek nevzorkuje navíc); bez vzorku hned.
pub fn next_strip_due(now: i64, every_ms: i64) -> i64 {
    match STATE.lock().last {
        Some(l) if l.at <= now => l.at + every_ms,
        _ => now,
    }
}

/// Segment položky proužku `system` z posledního vzorku (jen paměť). Bez vzorku nic.
pub fn strip_now() -> Vec<Segment> {
    let Some(l) = STATE.lock().last else { return Vec::new() };
    let mem = calc::mem_pct(l.mem_used, l.mem_total);
    let dot = match calc::level(l.cpu, mem) {
        Level::Calm => None,
        Level::Warm => Some(Rgba::AMBER),
        Level::Hot => Some(Rgba::RED),
    };
    vec![Segment { text: calc::strip_text(l.cpu, mem), color: Rgba::DIM, dot, strong: false, pill: false }]
}

/// Otevře Správce úloh (klik na CPU / paměť v kartě). `taskmgr` si okno vytvoří sám; Ledge žádné nové nemá.
pub fn open_task_manager() -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    crate::state::core().app.opener().open_path("taskmgr.exe", None::<&str>).map_err(|e| crate::errors::any(crate::i18n::tr("Správce úloh nejde otevřít", "Can't open Task Manager"), &e))
}
