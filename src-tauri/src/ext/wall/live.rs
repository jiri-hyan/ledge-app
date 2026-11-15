//! Živá zkouška okna tapety (jen `cargo test -- --ignored`, s `LEDGE_WALL_LIVE=1`; v bránách neběží).
//! Postup jako spike kola 33 (pokus 2): ochrany (`SHQueryUserNotificationState` 2/3/4, zamčená stanice, uživatel
//! nečinný ≥ 20 s), ověření pixelů jen v malém volném obdélníku primárního monitoru (`uncovered_rects` /
//! `pick_probe`), snímek jen toho obdélníku, ukončení procesu a kontrola, že žádný nezůstal. Nikdy Win+D,
//! fokus cizích oken, změna tapety ani snímek obrazovky. Data dir WebView2 jde do dočasné složky
//! (`LEDGE_CONFIG_DIR`), ne do skutečného `%LOCALAPPDATA%\Ledge`.

#![allow(unsafe_op_in_unsafe_fn)]

use super::Wall;
use super::geom::{R, pick_probe, uncovered_rects};
use crate::ext::child::run_with;
use crate::ext::wire::{self, Down};
use crate::procman::{CREATE_NO_WINDOW, Job};
use std::ffi::c_void;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::StationsAndDesktops::{CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP, OpenInputDesktop};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::BOOL;

/// Vstup dítěte: bez `LEDGE_WALL_LIVE_CHILD` prázdný test.
#[test]
fn child_entry() {
    if std::env::var("LEDGE_WALL_LIVE_CHILD").is_err() {
        return;
    }
    {
        // libtest nechá na stdout rozepsaný řádek — protokol musí začít na novém
        println!();
        let _ = std::io::stdout().flush();
    }
    let rc = run_with("wall", Box::new(Wall), std::io::stdin(), std::io::stdout());
    std::process::exit(rc);
}

fn guard(min_idle_ms: u32) -> Result<(), String> {
    unsafe {
        if let Some(s) = crate::screen::notification_state()
            && crate::screen::qun_busy(s)
        {
            return Err(format!("notification state {s}"));
        }
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_ACCESS_FLAGS(DESKTOP_SWITCHDESKTOP.0)) {
            Ok(d) => {
                let _ = CloseDesktop(d);
            }
            Err(_) => return Err("stanice zamčená".into()),
        }
        let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        if GetLastInputInfo(&mut li).as_bool() {
            let idle = GetTickCount().wrapping_sub(li.dwTime);
            if idle < min_idle_ms {
                return Err(format!("uživatel aktivní (nečinnost {idle} ms)"));
            }
        }
    }
    Ok(())
}

/// Pracovní plocha primárního monitoru minus viditelná okna cizích procesů.
fn free_rects() -> Vec<R> {
    unsafe extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
        let out = &mut *(l.0 as *mut Vec<R>);
        if !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut c_void, 4);
        if cloaked != 0 {
            return BOOL(1);
        }
        let c = crate::ext::host::win::class_name(h);
        if matches!(c.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return BOOL(1);
        }
        let (l0, t, r, b) = crate::ext::host::win::window_rect(h);
        out.push(R::new(l0, t, r, b));
        BOOL(1)
    }
    let mut covers: Vec<R> = vec![];
    let mut area = R::new(0, 0, 0, 0);
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut covers as *mut _ as isize));
        let m = MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(m, &mut mi).as_bool() {
            let r: RECT = mi.rcWork;
            area = R::new(r.left, r.top, r.right, r.bottom);
        }
    }
    uncovered_rects(&[area], &covers)
}

/// Snímek jen obdélníku `r` (BGRA).
fn capture(r: R) -> Vec<[u8; 4]> {
    unsafe {
        let sd = GetDC(None);
        let md = CreateCompatibleDC(Some(sd));
        let bmp = CreateCompatibleBitmap(sd, r.w(), r.h());
        let old = SelectObject(md, bmp.into());
        let _ = BitBlt(md, 0, 0, r.w(), r.h(), Some(sd), r.l, r.t, SRCCOPY | CAPTUREBLT);
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: r.w(), biHeight: -r.h(), biPlanes: 1, biBitCount: 32, biCompression: 0, ..Default::default() },
            ..Default::default()
        };
        let mut buf = vec![[0u8; 4]; (r.w() * r.h()) as usize];
        GetDIBits(md, bmp, 0, r.h() as u32, Some(buf.as_mut_ptr() as *mut c_void), &mut bi, DIB_RGB_COLORS);
        SelectObject(md, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(md);
        ReleaseDC(None, sd);
        buf
    }
}

/// Podíl pixelů (v %), které se liší od základního snímku o víc než 12 na kanál.
fn diff_pct(base: &[[u8; 4]], cur: &[[u8; 4]]) -> u32 {
    let n = base.len().min(cur.len()).max(1);
    let d = base.iter().zip(cur).filter(|(b, c)| (0..3).any(|i| (b[i] as i32 - c[i] as i32).abs() > 12)).count();
    (d * 100 / n) as u32
}

/// RAM (private) a CPU (s) stromu procesů s kořenem `root` — PowerShell, bez zásahu do cizích procesů.
fn tree_stats(root: u32) -> String {
    let script = format!(
        "$all=Get-CimInstance Win32_Process; $ids=@({root}); $n=1; while($n){{$n=0; foreach($p in $all){{ if($ids -contains $p.ParentProcessId -and $ids -notcontains $p.ProcessId){{$ids+=$p.ProcessId;$n=1}} }} }}; \
         $ps=Get-Process -Id $ids -ErrorAction SilentlyContinue; \
         '{{0}} procesů, RAM private {{1:N0}} MB, CPU {{2:N2}} s' -f $ps.Count, (($ps|Measure-Object PrivateMemorySize64 -Sum).Sum/1MB), (($ps|Measure-Object CPU -Sum).Sum)"
    );
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    std::os::windows::process::CommandExt::creation_flags(&mut c, CREATE_NO_WINDOW);
    c.output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|e| format!("neměřeno: {e}"))
}

fn alive(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => {
                let mut code = 0u32;
                let still = windows::Win32::System::Threading::GetExitCodeProcess(h, &mut code).is_ok() && code == 259;
                let _ = CloseHandle(h);
                still
            }
            Err(_) => false,
        }
    }
}

#[test]
#[ignore = "živá zkouška na ploše: LEDGE_WALL_LIVE=1 cargo test wall::live -- --ignored --nocapture"]
fn live_attach_renders_in_free_probe_rect() {
    if std::env::var("LEDGE_WALL_LIVE").as_deref() != Ok("1") {
        println!("LIVE: přeskočeno (bez LEDGE_WALL_LIVE=1)");
        return;
    }
    if let Err(e) = guard(20_000) {
        println!("LIVE: neověřeno živě — {e}");
        return;
    }
    let Some(r) = pick_probe(&free_rects(), 48, 64, 8) else {
        println!("LIVE: neověřeno živě — na primárním monitoru není volné místo (vše zakryto okny)");
        return;
    };
    println!("LIVE: sonda {:?} {}x{}", (r.l, r.t), r.w(), r.h());
    let base = capture(r);
    let dir = tempfile::tempdir().unwrap();
    let job = Job::new().unwrap();
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "ext::wall::live::child_entry", "--nocapture", "--test-threads=1"])
        .env("LEDGE_WALL_LIVE_CHILD", "1")
        .env("LEDGE_CONFIG_DIR", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = job.spawn(&mut cmd, CREATE_NO_WINDOW).unwrap();
    let pid = child.id();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        while let Some(Ok(l)) = lines.next() {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    let wait = |needle: &str, secs: u64| -> bool {
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(secs) {
            if let Ok(l) = rx.recv_timeout(Duration::from_millis(200))
                && l.contains(needle)
            {
                return true;
            }
        }
        false
    };
    let init = wire::encode_down(&Down::Init { cfg: serde_json::json!({}), lang: "cs".into(), dead: false, rev: serde_json::json!({}) });
    let verdict: String;
    if wait("\"hello\"", 15) {
        let _ = stdin.write_all(init.as_bytes());
        let _ = stdin.flush();
        if wait("\"ready\"", 25) {
            // okno je vidět; pár sekund klidu, snímek malého obdélníku a měření stromu
            std::thread::sleep(Duration::from_millis(1500));
            let d = diff_pct(&base, &capture(r));
            verdict = format!("LIVE: pixelů odlišných od tapety v sondě: {d} % ({})", if d >= 80 { "VYKRESLENO" } else { "NEVIDĚT / pod ikonou" });
            println!("LIVE: {}", tree_stats(pid));
            std::thread::sleep(Duration::from_secs(4));
            println!("LIVE: po 4 s klidu: {}", tree_stats(pid));
        } else {
            verdict = "LIVE: ready nepřišlo do 25 s".into();
        }
    } else {
        verdict = "LIVE: hello nepřišlo".into();
    }
    // konec: quit, pak pojistka Job::kill
    let _ = stdin.write_all(wire::encode_down(&Down::Quit).as_bytes());
    let _ = stdin.flush();
    let t0 = Instant::now();
    while alive(pid) && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(100));
    }
    let clean = !alive(pid);
    job.kill();
    let _ = child.wait();
    println!("{verdict}");
    println!("LIVE: ukončeno {} (PID {pid}), po kill žije: {}", if clean { "čistě na quit" } else { "až kill jobu" }, alive(pid));
    assert!(!alive(pid), "osiřelý proces ext");
}
