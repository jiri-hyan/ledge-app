//! Job Object s KILL_ON_JOB_CLOSE (vyčleněno z `procman.rs`, kolo 33): spuštění procesu rovnou v jobu a zabití
//! celého stromu. Sdílí ho `procman`, `gitinfo::run`, `briefrun/proc.rs` a `ext/sup.rs` (rozšíření `--ext`).

use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    TerminateJobObject,
};
use windows::Win32::System::Threading::{
    CREATE_SUSPENDED, OpenProcess, OpenThread, PROCESS_QUERY_LIMITED_INFORMATION, ResumeThread, THREAD_SUSPEND_RESUME,
};
use windows::core::PCWSTR;

/// Příznak `CREATE_NO_WINDOW` pro `Job::spawn` (žádná viditelná konzole).
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Job Object s KILL_ON_JOB_CLOSE — sdílí ho i `gitinfo::run` (zabití celého stromu gitu).
pub(crate) struct Job(HANDLE);
unsafe impl Send for Job {}
unsafe impl Sync for Job {}

impl Job {
    pub(crate) fn new() -> Result<Self, String> {
        unsafe {
            let h = CreateJobObjectW(None, PCWSTR::null()).map_err(|e| e.to_string())?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .map_err(|e| e.to_string())?;
            Ok(Job(h))
        }
    }
    fn assign(&self, child: &Child) -> Result<(), String> {
        unsafe {
            AssignProcessToJobObject(self.0, HANDLE(child.as_raw_handle())).map_err(|e| e.to_string())
        }
    }
    /// Spustí proces rovnou v jobu: vznikne pozastavený, přiřadí se a teprve pak se rozběhne.
    /// Přiřazení až po běžném `spawn()` je závod — cmd.exe stihne pustit node dřív, než je
    /// v jobu, a vnuk pak žije mimo job (Stop ho nezabije, port drží sirotek).
    /// `flags` = ostatní příznaky (CREATE_NO_WINDOW…); `creation_flags` je přepisuje, ne sčítá.
    pub(crate) fn spawn(&self, cmd: &mut Command, flags: u32) -> Result<Child, String> {
        let mut child = cmd
            .creation_flags(flags | CREATE_SUSPENDED.0)
            .spawn()
            .map_err(|e| e.to_string())?;
        let res = self.assign(&child).map_err(|e| format!("Job Object: {e}")).and_then(|()| resume_threads(child.id()));
        if let Err(e) = res {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        Ok(child)
    }
    pub(crate) fn contains(&self, pid: u32) -> bool {
        unsafe {
            let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
            let mut res = windows::core::BOOL(0);
            let ok = IsProcessInJob(p, Some(self.0), &mut res).is_ok() && res.as_bool();
            let _ = CloseHandle(p);
            ok
        }
    }
    /// Surový handle jobu pro dotazy (`ext/measure.rs`: `QueryInformationJobObject`). Patří jobu — nezavírat.
    pub(crate) fn raw(&self) -> HANDLE {
        self.0
    }
    pub(crate) fn kill(&self) {
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0); // KILL_ON_JOB_CLOSE → zbytek stromu umře
        }
    }
}

/// Rozběhne pozastavený proces (std nevydá handle hlavního vlákna → najdeme ho přes ToolHelp).
fn resume_threads(pid: u32) -> Result<(), String> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| format!("ToolHelp: {e}"))?;
        let mut te = THREADENTRY32 { dwSize: std::mem::size_of::<THREADENTRY32>() as u32, ..Default::default() };
        let mut resumed = 0;
        let mut more = Thread32First(snap, &mut te).is_ok();
        while more {
            if te.th32OwnerProcessID == pid
                && let Ok(t) = OpenThread(THREAD_SUSPEND_RESUME, false, te.th32ThreadID)
            {
                if ResumeThread(t) != u32::MAX {
                    resumed += 1;
                }
                let _ = CloseHandle(t);
            }
            more = Thread32Next(snap, &mut te).is_ok();
        }
        let _ = CloseHandle(snap);
        if resumed == 0 {
            return Err(trf!("Nepodařilo se rozběhnout proces {pid}.", "Couldn't get process {pid} running."));
        }
        Ok(())
    }
}

