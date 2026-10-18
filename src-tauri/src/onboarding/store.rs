//! Aplikace z Microsoft Store (Poznámkový blok, Kalkulačka…): nemají `.lnk` ve Start menu, jsou jen ve
//! `shell:AppsFolder`. Výčet (COM, jen na vyžádání — nástroj `apps_discover`, nic v klidu) dá jméno a
//! AppUserModelID; spuštění je `shell:AppsFolder\<aumid>` přes ShellExecute. Výběr a párování
//! ([`pick`]) jsou čisté funkce, samotný výčet a spuštění sahají na systém a netestují se na skutečném
//! Windows (jen ručně: `cargo test store -- --ignored`).

use crate::config::valid_aumid;
use serde::Serialize;
use std::collections::HashSet;
use std::time::Duration;

/// Výčet `shell:AppsFolder` obvykle trvá desítky ms; déle než tohle ho vzdáme (vlákno doběhne samo).
const ENUM_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreApp {
    pub name: String,
    pub aumid: String,
    /// zástupce se stejným aumid / jménem už v Ledge je
    pub already: bool,
}

/// Surové položky `shell:AppsFolder` (jméno, AUMID) → aplikace ze Storu k nabídnutí: jen skutečné
/// balíčky (`Balicek_hash!App`; klasické programy mají v AppsFolderu cestu nebo GUID a už jsou ve Start
/// menu jako `.lnk`), bez prázdných jmen, bez duplicit (aumid, jméno) a bez toho, co už nabídlo Start menu
/// (`taken_names`). Řazeno podle jména.
pub fn pick(raw: Vec<(String, String)>, taken_names: &[String], known_aumids: &[String], known_names: &[String]) -> Vec<StoreApp> {
    let mut seen_id = HashSet::new();
    let mut seen_name: HashSet<String> = taken_names.iter().map(|n| n.to_lowercase()).collect();
    let mut out: Vec<StoreApp> = raw
        .into_iter()
        .filter_map(|(name, aumid)| {
            let name = name.trim().to_string();
            (!name.is_empty() && valid_aumid(&aumid) && seen_id.insert(aumid.to_lowercase()) && seen_name.insert(name.to_lowercase())).then(|| StoreApp {
                already: known_aumids.iter().any(|k| k.eq_ignore_ascii_case(&aumid)) || known_names.iter().any(|k| k.eq_ignore_ascii_case(&name)),
                name,
                aumid,
            })
        })
        .collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

/// Cíl pro ShellExecute; `None`, když AUMID nesedí (nic jiného shellu se podstrčit nesmí).
pub fn shell_target(aumid: &str) -> Option<String> {
    valid_aumid(aumid).then(|| format!("shell:AppsFolder\\{aumid}"))
}

/// Výčet `shell:AppsFolder` (jméno, AUMID) s termínem. Prázdný výsledek = nic nenalezeno / selhalo.
pub fn list_raw() -> Vec<(String, String)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new().name("store-apps".into()).spawn(move || {
        let _ = tx.send(sys::enumerate());
    });
    if spawned.is_err() {
        return Vec::new();
    }
    rx.recv_timeout(ENUM_TIMEOUT).unwrap_or_default()
}

/// Spustí aplikaci ze Storu (jako dvojklik v nabídce Start). Nic nesleduje — okno si otevře sama.
pub fn launch(aumid: &str) -> Result<(), String> {
    let target = shell_target(aumid).ok_or_else(|| trf!("aumid „{aumid}“ nesedí.", "aumid “{aumid}” doesn't fit."))?;
    sys::open(&target).map_err(|code| trf!("Aplikaci „{aumid}“ nejde spustit (kód {code}). Je ještě nainstalovaná?", "Can't start “{aumid}” (code {code}). Is it still installed?"))
}

mod sys {
    use windows::Win32::System::Com::*;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};

    /// Pojistka počtu položek jednoho výčtu.
    const MAX_ITEMS: usize = 2000;

    /// Shell namespace chce STA; vlákno si ho zakládá samo a na konci uvolní.
    struct Sta(bool);
    impl Sta {
        fn init() -> Self {
            Sta(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.is_ok())
        }
    }
    impl Drop for Sta {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() }
            }
        }
    }

    fn name_of(item: &IShellItem, kind: SIGDN) -> Option<String> {
        unsafe {
            let p = item.GetDisplayName(kind).ok()?;
            let s = p.to_string().ok();
            CoTaskMemFree(Some(p.0 as _));
            s
        }
    }

    pub fn enumerate() -> Vec<(String, String)> {
        let _sta = Sta::init();
        let mut out = Vec::new();
        unsafe {
            let Ok(folder) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) else { return out };
            let Ok(items) = folder.BindToHandler::<_, IEnumShellItems>(None::<&IBindCtx>, &BHID_EnumItems) else { return out };
            while out.len() < MAX_ITEMS {
                let mut one = [None];
                let mut got = 0u32;
                if items.Next(&mut one, Some(&mut got)).is_err() || got == 0 {
                    break;
                }
                let Some(item) = one[0].take() else { break };
                if let (Some(name), Some(id)) = (name_of(&item, SIGDN_NORMALDISPLAY), name_of(&item, SIGDN_PARENTRELATIVEPARSING)) {
                    out.push((name, id));
                }
            }
        }
        out
    }

    /// `Err(kód)`, když ShellExecute selže (návratová hodnota ≤ 32).
    pub fn open(target: &str) -> Result<(), isize> {
        let _sta = Sta::init();
        let t = HSTRING::from(target);
        let r = unsafe { ShellExecuteW(None, w!("open"), &t, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
        let code = r.0 as isize;
        if code > 32 { Ok(()) } else { Err(code) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items.iter().map(|(n, a)| (n.to_string(), a.to_string())).collect()
    }

    #[test]
    fn pick_table() {
        let found = raw(&[
            ("Kalkulačka", "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            ("Poznámkový blok", "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App"),
            ("Poznámkový blok", "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App"), // duplicita (stejné aumid)
            ("Kalkulačka 2", "microsoft.windowscalculator_8wekyb3d8bbwe!app"), // stejné aumid jinak psané
            ("Správce souborů", r"C:\Files\files.exe"),                        // klasický program: cesta
            ("Prohlížeč", "Browser"),                                          // bez `!`
            ("Skrytý", r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\cmd.exe"),    // GUID + cesta
            ("  ", "Some.Pkg_abc!App"),                                        // prázdné jméno
            ("Počasí", "Microsoft.BingWeather_8wekyb3d8bbwe!App"),
            ("Již nabídnuto", "Other.Pkg_abc!App"),
        ]);
        let got = pick(found, &["Již Nabídnuto".to_string()], &["Microsoft.BingWeather_8wekyb3d8bbwe!App".to_string()], &["kalkulačka".to_string()]);
        let view: Vec<(&str, bool)> = got.iter().map(|a| (a.name.as_str(), a.already)).collect();
        assert_eq!(view, [("Kalkulačka", true), ("Poznámkový blok", false), ("Počasí", true)]);
        assert_eq!(got[0].aumid, "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App");
    }

    #[test]
    fn shell_target_only_for_plain_aumid() {
        #[rustfmt::skip]
        let cases = [
            ("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App", Some(r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App")),
            (r"..\..\Windows\notepad.exe",                    None),
            (r"Pkg_abc!App\..\x",                             None),
            ("Pkg_abc!App && calc",                           None),
            ("Pkg_abc",                                       None),
            ("",                                              None),
        ];
        for (a, want) in cases {
            assert_eq!(shell_target(a).as_deref(), want, "{a}");
        }
        assert!(launch("evil && calc").is_err());
    }

    /// Ručně na skutečném Windows: `cargo test store -- --ignored --nocapture`. Nic nespouští, jen vypíše výčet.
    #[test]
    #[ignore]
    fn lists_real_apps_folder() {
        let raw = list_raw();
        let got = pick(raw.clone(), &[], &[], &[]);
        eprintln!("AppsFolder: {} položek, ze Storu {}", raw.len(), got.len());
        for a in got.iter().take(8) {
            eprintln!("  {} -> {}", a.name, a.aumid);
        }
        assert!(!raw.is_empty());
    }
}
