//! Spuštění programu MIMO bublinu MSIX (viz `pkgctx.rs`): „Restartovat mimo“ a instalátor aktualizace.
//!
//! **Jak.** Proces spuštěný Plánovačem úloh nedědí kontext balíčku volajícího (spouští ho služba
//! Schedule s interaktivním tokenem uživatele) → vidí skutečné `%APPDATA%`, `%LOCALAPPDATA%` a HKCU.
//! Postup: jednorázová úloha bez spouštěče (`RegisterTask` z XML, [`task_xml`]), `LogonType =
//! InteractiveToken` (běží v přihlášené relaci, bez hesla), `Run`, a podle [`After`] smazat. Argumenty
//! projdou. Záloha: `explorer.exe "<exe>"` (spustí ho běžící shell mimo balíček; argumenty se ztratí).
//!
//! **Nastavení úlohy** (výchozí hodnoty Plánovače by Ledge škodily): bez limitu doby běhu (`PT0S`,
//! výchozí 72 h by Ledge zabilo), běží i na baterii a nepřestane při odpojení, priorita 5 (normální;
//! výchozí 7 = snížená CPU i I/O), paralelní instance.
//!
//! **Úklid.** `After::Delete` (restart) smaže úlohu, jakmile běží (nebo už doběhla). `After::Keep`
//! (instalátor — smazáním běžící úlohy bychom riskovali přerušení instalace) ji nechá a uklidí ji
//! [`cleanup_stale`] při příštím startu Ledge (jen úlohy, které už neběží).

use crate::iconimg::Com;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::SCHED_S_TASK_HAS_NOT_RUN;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::TaskScheduler::{ITaskFolder, ITaskService, TASK_CREATE_OR_UPDATE, TASK_LOGON_INTERACTIVE_TOKEN, TASK_STATE_RUNNING, TaskScheduler};
use windows::Win32::System::Variant::VARIANT;
use windows::core::BSTR;

/// Jména úloh v kořeni Plánovače (pevná → úklid je najde, nic se nehromadí).
pub const TASK_RELAUNCH: &str = "Ledge-outside-relaunch";
pub const TASK_UPDATE: &str = "Ledge-outside-update";
/// Jak dlouho po `Run` čekat, že úloha opravdu naběhla (pak se smaže, nebo selže → záloha přes Explorer).
const START_WAIT: Duration = Duration::from_secs(5);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Co s úlohou po spuštění.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    Delete,
    Keep,
}

/// XML pro text úlohy (`&`, `<`, `>`, `"`, `'`). Čistá funkce.
pub fn xml_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            c => o.push(c),
        }
    }
    o
}

/// Definice jednorázové úlohy (schéma Task Scheduler 1.2): bez spouštěče, interaktivní token, jedna
/// akce Exec. Prázdné argumenty se vynechají. Řídicí znaky v cestě/argumentech → chyba. Čistá funkce.
pub fn task_xml(cmd: &Path, args: &str, dir: &Path) -> Result<String, String> {
    let c = cmd.to_string_lossy();
    let d = dir.to_string_lossy();
    if c.trim().is_empty() || [&*c, args, &*d].iter().any(|s| s.chars().any(char::is_control)) {
        return Err(trf!("Cestu „{c}“ nejde předat Plánovači úloh.", "The path “{c}” can't be handed to Task Scheduler."));
    }
    let args = if args.trim().is_empty() { String::new() } else { format!("<Arguments>{}</Arguments>", xml_escape(args)) };
    let dir = if d.trim().is_empty() { String::new() } else { format!("<WorkingDirectory>{}</WorkingDirectory>", xml_escape(&d)) };
    Ok(format!(
        concat!(
            r#"<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">"#,
            "<RegistrationInfo><Description>Ledge: jednorázové spuštění mimo balíček MSIX (smaže se samo)</Description></RegistrationInfo>",
            r#"<Principals><Principal id="Author"><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>"#,
            "<Settings>",
            "<MultipleInstancesPolicy>Parallel</MultipleInstancesPolicy>",
            "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
            "<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>",
            "<AllowHardTerminate>false</AllowHardTerminate>",
            "<AllowStartOnDemand>true</AllowStartOnDemand>",
            "<Enabled>true</Enabled>",
            "<Hidden>false</Hidden>",
            "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>",
            "<Priority>5</Priority>",
            "</Settings>",
            r#"<Actions Context="Author"><Exec><Command>{}</Command>{}{}</Exec></Actions>"#,
            "</Task>"
        ),
        xml_escape(&c),
        args,
        dir
    ))
}

fn empty() -> VARIANT {
    VARIANT::default()
}

fn root() -> Result<ITaskFolder, String> {
    unsafe {
        let svc: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER).map_err(|e| format!("TaskScheduler: {e}"))?;
        svc.Connect(&empty(), &empty(), &empty(), &empty()).map_err(|e| format!("Connect: {e}"))?;
        svc.GetFolder(&BSTR::from("\\")).map_err(|e| format!("GetFolder: {e}"))
    }
}

/// Spustí `cmd args` mimo bublinu přes Plánovač úloh (úloha `task`). Blokuje až [`START_WAIT`] —
/// volat mimo UI vlákno.
pub fn run_task(task: &str, cmd: &Path, args: &str, dir: &Path, after: After) -> Result<(), String> {
    let xml = task_xml(cmd, args, dir)?;
    let _com = Com::init();
    let root = root()?;
    let name = BSTR::from(task);
    unsafe {
        let reg = root
            .RegisterTask(&name, &BSTR::from(xml), TASK_CREATE_OR_UPDATE.0, &empty(), &empty(), TASK_LOGON_INTERACTIVE_TOKEN, &empty())
            .map_err(|e| format!("RegisterTask: {e}"))?;
        if let Err(e) = reg.Run(&empty()) {
            let _ = root.DeleteTask(&name, 0);
            return Err(format!("Run: {e}"));
        }
        if after == After::Keep {
            return Ok(());
        }
        // smazat až po naběhnutí (smazání čekající úlohy by ji zrušilo); běžící instanci smazání nezastaví
        let deadline = Instant::now() + START_WAIT;
        let started = loop {
            let running = reg.State().is_ok_and(|s| s == TASK_STATE_RUNNING);
            let ran = reg.LastTaskResult().is_ok_and(|r| r != SCHED_S_TASK_HAS_NOT_RUN.0);
            if running || ran {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let _ = root.DeleteTask(&name, 0);
        if started { Ok(()) } else { Err(crate::i18n::tr("úloha nenaběhla do 5 s", "the task didn't start within 5 s").into()) }
    }
}

/// Záloha: `explorer.exe "<cmd>"` — exe spustí běžící shell (mimo balíček), argumenty se ztratí.
pub fn run_explorer(cmd: &Path) -> Result<(), String> {
    let root = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    std::process::Command::new(root.join("explorer.exe"))
        .arg(cmd)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Smaže zbylé úlohy Ledge (`TASK_RELAUNCH`, `TASK_UPDATE`), které už neběží. Vrátí, kolik jich zůstalo
/// běžet (instalátor ještě končí) — volající to zkusí za chvíli znovu.
pub fn cleanup_stale() -> usize {
    let _com = Com::init();
    let Ok(root) = root() else { return 0 };
    let mut left = 0;
    for name in [TASK_RELAUNCH, TASK_UPDATE] {
        let b = BSTR::from(name);
        unsafe {
            let Ok(t) = root.GetTask(&b) else { continue };
            if t.State().is_ok_and(|s| s == TASK_STATE_RUNNING) {
                left += 1;
            } else if root.DeleteTask(&b, 0).is_ok() {
                crate::log(&format!("outside: uklizena úloha {name}"));
            }
        }
    }
    left
}

/// Úklid na pozadí po startu (úlohu instalátoru může ještě držet doběhnutí `/R`): hned, pak za 30 s.
pub fn cleanup_later() {
    std::thread::Builder::new()
        .name("outside-cleanup".into())
        .spawn(|| {
            if cleanup_stale() > 0 {
                std::thread::sleep(Duration::from_secs(30));
                cleanup_stale();
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_table() {
        #[rustfmt::skip]
        let cases = [
            ("", ""),
            (r"C:\Users\x\ledge.exe", r"C:\Users\x\ledge.exe"),
            ("a&b<c>d\"e'f", "a&amp;b&lt;c&gt;d&quot;e&apos;f"),
            ("Jiří", "Jiří"),
        ];
        for (inp, want) in cases {
            assert_eq!(xml_escape(inp), want, "{inp}");
        }
    }

    #[test]
    fn task_xml_shape() {
        let x = task_xml(Path::new(r"C:\Program Files\L&L\ledge.exe"), "--after-pid 42", Path::new(r"C:\Program Files\L&L")).unwrap();
        assert!(x.contains(r"<Command>C:\Program Files\L&amp;L\ledge.exe</Command>"), "{x}");
        assert!(x.contains("<Arguments>--after-pid 42</Arguments>"));
        assert!(x.contains(r"<WorkingDirectory>C:\Program Files\L&amp;L</WorkingDirectory>"));
        assert!(x.contains("<LogonType>InteractiveToken</LogonType>"));
        // výchozí hodnoty Plánovače, které by Ledge zabily nebo zpomalily
        for must in ["<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>", "<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>", "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>", "<Priority>5</Priority>"] {
            assert!(x.contains(must), "{must}");
        }
        // žádný spouštěč — úloha běží jen na `Run`
        assert!(!x.contains("<Triggers>"));
    }

    #[test]
    fn task_xml_optional_parts_and_rejects() {
        let x = task_xml(Path::new(r"C:\a\setup.exe"), "  ", Path::new("")).unwrap();
        assert!(!x.contains("<Arguments>") && !x.contains("<WorkingDirectory>"));
        assert!(task_xml(Path::new(""), "", Path::new("")).is_err());
        assert!(task_xml(Path::new("C:\\a\nb.exe"), "", Path::new("")).is_err());
        assert!(task_xml(Path::new(r"C:\a.exe"), "x\ry", Path::new("")).is_err());
    }
}
