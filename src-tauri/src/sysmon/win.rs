//! Čtečky systémových čítačů přes WinAPI (bez WMI, bez procesů): `GetSystemTimes` (CPU),
//! `GlobalMemoryStatusEx` (paměť), `GetLogicalDrives` + `GetDiskFreeSpaceExW` (pevné disky) a
//! `GetIfTable2` (bajty fyzických síťových adaptérů). Každé volání je mikrosekundy až jednotky ms;
//! volá je jen vzorkovač, když je karta vidět nebo je zapnutá položka proužku.

use super::calc::{CpuTimes, Disk, mask_letters};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;
use windows::core::PCWSTR;

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
    }
}
