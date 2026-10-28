//! Cíl položky palety „Nastavení Windows: …“ (tabulka stránek je v `src/palette/winsettings.ts`).
//! UI posílá jen `ms-settings:<stránka>` nebo `control:Microsoft.<Jméno>`; tady se ověří tvar, aby
//! z palety nešlo spustit nic jiného (žádný shell, žádné argumenty navíc). Spuštění v
//! `commands/scripts.rs::win_settings_open` (opener / `control.exe /name …`).

use crate::i18n::tr;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum WinTarget {
    /// `ms-settings:bluetooth` → ShellExecute (opener)
    Settings(String),
    /// kanonické jméno ovládacího panelu (`Microsoft.Mouse`) → `control.exe /name …`
    Control(String),
}

/// Tvar cíle: `ms-settings:` + a–z, 0–9, `-` (bez dotazu `?…`), nebo `control:Microsoft.` + písmena.
pub fn parse(target: &str) -> Result<WinTarget, String> {
    let bad = || Err(tr("Neznámá stránka nastavení.", "Unknown settings page.").into());
    if let Some(page) = target.strip_prefix("ms-settings:") {
        if (1..=64).contains(&page.len()) && page.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
            return Ok(WinTarget::Settings(target.to_string()));
        }
        return bad();
    }
    if let Some(name) = target.strip_prefix("control:")
        && let Some(rest) = name.strip_prefix("Microsoft.")
        && (1..=48).contains(&rest.len())
        && rest.bytes().all(|b| b.is_ascii_alphabetic())
    {
        return Ok(WinTarget::Control(name.to_string()));
    }
    bad()
}

/// Odkazy dlaždice „Nastavení“ v kartě Rychlé dlaždice (kolo 23): id z UI → `ms-settings:` (UI neposílá URI).
pub const QUICK: [(&str, &str); 4] = [
    ("network", "ms-settings:network"),
    ("sound", "ms-settings:sound"),
    ("display", "ms-settings:display"),
    ("bluetooth", "ms-settings:bluetooth"),
];

pub fn quick_uri(id: &str) -> Option<&'static str> {
    QUICK.iter().find(|(k, _)| *k == id).map(|(_, u)| *u)
}

/// `%SystemRoot%\System32\control.exe` (absolutně — ne podle PATH).
pub fn control_exe() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    PathBuf::from(root).join("System32").join("control.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_table() {
        let ok: &[(&str, WinTarget)] = &[
            ("ms-settings:bluetooth", WinTarget::Settings("ms-settings:bluetooth".into())),
            ("ms-settings:network-wifi", WinTarget::Settings("ms-settings:network-wifi".into())),
            ("control:Microsoft.Mouse", WinTarget::Control("Microsoft.Mouse".into())),
        ];
        for (t, want) in ok {
            assert_eq!(&parse(t).unwrap(), want, "{t}");
        }
        for t in [
            "",
            "ms-settings:",
            "ms-settings:Bluetooth",
            "ms-settings:sound-properties?endpointId=x",
            "ms-settings:a b",
            "ms-settings:x&calc",
            "https://example.com",
            "control:Microsoft.",
            "control:Microsoft.Mouse /page",
            "control:Microsoft.Mouse\"",
            "control:Contoso.Panel",
            "control:microsoft.mouse",
            "cmd /c calc",
        ] {
            assert!(parse(t).is_err(), "{t:?} musí být odmítnut");
        }
        for (id, uri) in QUICK {
            assert_eq!(quick_uri(id), Some(uri));
            assert!(matches!(parse(uri), Ok(WinTarget::Settings(_))), "{uri}");
        }
        assert_eq!(quick_uri("calc"), None);
        assert!(control_exe().ends_with(r"System32\control.exe"));
    }
}
