//! Čtečky systémových čítačů přes WinAPI (bez WMI, bez procesů): `GetSystemTimes` (CPU),
//! `GlobalMemoryStatusEx` (paměť), `D3DKMTQueryStatistics` (GPU), `GetLogicalDrives` + `GetDiskFreeSpaceExW`
//! (pevné disky) a `GetIfTable2` (bajty fyzických síťových adaptérů). Každé volání je mikrosekundy až jednotky ms;
//! volá je jen vzorkovač, když je karta vidět nebo je zapnutá položka proužku.

use super::calc::{CpuTimes, Disk, mask_letters};
use std::sync::OnceLock;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;
use windows::core::{PCWSTR, s, w};

/// `DRIVE_FIXED`
const DRIVE_FIXED: u32 = 3;
/// `IF_TYPE_SOFTWARE_LOOPBACK`
const IF_TYPE_LOOPBACK: u32 = 24;

fn ticks(f: FILETIME) -> u64 {
    (f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64
}

/// Součty času procesorů od startu.
pub fn cpu_times() -> Option<CpuTimes> {
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()? };
    Some(CpuTimes { idle: ticks(idle), kernel: ticks(kernel), user: ticks(user) })
}

/// (celková, volná) fyzická paměť v bajtech.
pub fn memory() -> Option<(u64, u64)> {
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
    Some((m.ullTotalPhys, m.ullAvailPhys))
}

/// Písmeno systémového disku (`SystemDrive`, obvykle `C`).
fn system_letter() -> Option<char> {
    std::env::var("SystemDrive").ok().and_then(|s| s.chars().next()).map(|c| c.to_ascii_uppercase())
}

/// Pevné disky s velikostí. Výměnná média, síťové disky a optické mechaniky se nečtou (mohly by se
/// probouzet nebo viset); disk, který neodpoví, se přeskočí.
pub fn fixed_disks() -> Vec<Disk> {
    let sys = system_letter();
    let mask = unsafe { GetLogicalDrives() };
    mask_letters(mask)
        .into_iter()
        .filter_map(|l| {
            let root: Vec<u16> = format!("{l}:\\").encode_utf16().chain(std::iter::once(0)).collect();
            if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != DRIVE_FIXED {
                return None;
            }
            let (mut free, mut total) = (0u64, 0u64);
            unsafe { GetDiskFreeSpaceExW(PCWSTR(root.as_ptr()), Some(&mut free), Some(&mut total), None).ok()? };
            (total > 0).then(|| Disk { letter: l.to_string(), total, free, system: Some(l) == sys })
        })
        .collect()
}

/// (přijaté, odeslané) bajty od startu, součet fyzických adaptérů, které jsou nahoře. Virtuální (VPN,
/// Hyper-V, loopback) se nepočítají — provoz VPN by se jinak započítal dvakrát.
pub fn net_totals() -> Option<(u64, u64)> {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if unsafe { GetIfTable2(&mut table) }.0 != 0 || table.is_null() {
        return None;
    }
    let (mut rx, mut tx) = (0u64, 0u64);
    unsafe {
        let n = (*table).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), n);
        for r in rows {
            let hardware = r.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
            if hardware && r.OperStatus == IfOperStatusUp && r.Type != IF_TYPE_LOOPBACK {
                rx = rx.saturating_add(r.InOctets);
                tx = tx.saturating_add(r.OutOctets);
            }
        }
        FreeMibTable(table as *const _);
    }
    Some((rx, tx))
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AdapterInfo {
    h_adapter: u32,
    luid_low: u32,
    luid_high: i32,
    num_sources: u32,
    present_move_regions_preferred: u32,
}

#[repr(C)]
struct EnumAdapters2 {
    num_adapters: u32,
    p_adapters: *mut AdapterInfo,
}

#[repr(C)]
struct CloseAdapter {
    h_adapter: u32,
}

/// `D3DKMT_QUERYSTATISTICS` z `d3dkmthk.h` (0x328 = 808 B na x64): `kind` 0 = `ADAPTER` (`NodeCount` v horních
/// 32 bitech `result[0]`), `kind` 5 = `NODE` (`GlobalInformation.RunningTime` v 100ns ticích v `result[0]`).
#[repr(C)]
struct QueryStatistics {
    kind: u32,
    luid_low: u32,
    luid_high: i32,
    _pad: u32,
    h_process: usize,
    result: [u64; 97],
    query_node_id: u64,
}

const _: () = assert!(std::mem::size_of::<QueryStatistics>() == 0x328);

type FnEnumAdapters2 = unsafe extern "system" fn(*mut EnumAdapters2) -> i32;
type FnQueryStatistics = unsafe extern "system" fn(*mut QueryStatistics) -> i32;
type FnCloseAdapter = unsafe extern "system" fn(*const CloseAdapter) -> i32;

struct D3dkmt {
    enum_adapters: FnEnumAdapters2,
    query_stats: FnQueryStatistics,
    close_adapter: FnCloseAdapter,
}

fn d3dkmt() -> Option<&'static D3dkmt> {
    static API: OnceLock<Option<D3dkmt>> = OnceLock::new();
    API.get_or_init(|| unsafe {
        let h = LoadLibraryW(w!("gdi32.dll")).ok()?;
        let e = GetProcAddress(h, s!("D3DKMTEnumAdapters2"))?;
        let q = GetProcAddress(h, s!("D3DKMTQueryStatistics"))?;
        let c = GetProcAddress(h, s!("D3DKMTCloseAdapter"))?;
        Some(D3dkmt {
            enum_adapters: std::mem::transmute::<unsafe extern "system" fn() -> isize, FnEnumAdapters2>(e),
            query_stats: std::mem::transmute::<unsafe extern "system" fn() -> isize, FnQueryStatistics>(q),
            close_adapter: std::mem::transmute::<unsafe extern "system" fn() -> isize, FnCloseAdapter>(c),
        })
    })
    .as_ref()
}

/// Součty běhu enginů (uzlů) grafických adaptérů v setinách mikrosekundy (100 ns) přes `D3DKMTQueryStatistics`
/// (`gdi32.dll`, stejný zdroj jako Správce úloh; žádné WMI, žádný proces).
pub fn gpu_nodes() -> Option<Vec<u64>> {
    let api = d3dkmt()?;
    let mut adapters = [AdapterInfo::default(); 16];
    let mut ea = EnumAdapters2 { num_adapters: adapters.len() as u32, p_adapters: adapters.as_mut_ptr() };
    if unsafe { (api.enum_adapters)(&mut ea) } < 0 || ea.num_adapters == 0 {
        return None;
    }
    let count = (ea.num_adapters as usize).min(adapters.len());
    let mut out = Vec::new();
    for a in &adapters[..count] {
        let mut qs = QueryStatistics {
            kind: 0,
            luid_low: a.luid_low,
            luid_high: a.luid_high,
            _pad: 0,
            h_process: 0,
            result: [0; 97],
            query_node_id: 0,
        };
        if unsafe { (api.query_stats)(&mut qs) } >= 0 {
            let nodes = ((qs.result[0] >> 32) as u32).min(64);
            for node_id in 0..nodes {
                let mut nq = QueryStatistics {
                    kind: 5,
                    luid_low: a.luid_low,
                    luid_high: a.luid_high,
                    _pad: 0,
                    h_process: 0,
                    result: [0; 97],
                    query_node_id: node_id as u64,
                };
                if unsafe { (api.query_stats)(&mut nq) } >= 0 {
                    out.push(nq.result[0]);
                }
            }
        }
        let ca = CloseAdapter { h_adapter: a.h_adapter };
        unsafe {
            let _ = (api.close_adapter)(&ca);
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skutečné volání WinAPI (jen čtení, nic nezapisuje): čítače existují a dávají smysl.
    #[test]
    fn readers_return_sane_values() {
        let t = cpu_times().expect("GetSystemTimes");
        assert!(t.kernel >= t.idle && t.kernel + t.user > 0);
        let (total, avail) = memory().expect("GlobalMemoryStatusEx");
        assert!(total > 0 && avail <= total);
        assert!(!fixed_disks().is_empty(), "aspoň systémový disk");
        assert!(net_totals().is_some());
        if let Some(nodes) = gpu_nodes() {
            assert!(!nodes.is_empty());
        }
    }
}
