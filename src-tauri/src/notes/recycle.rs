//! Koš Windows pro smazané poznámky: soubory jdou do Koše s možností obnovy (`SHFileOperationW` s
//! `FOF_ALLOWUNDO`, bez dialogů). Žádná nová závislost — funkce je v `windows` crate (`Win32_UI_Shell`).
//! Koš existuje jen na místních discích: na síťovém / vyměnitelném by Windows soubor smazaly navždy,
//! proto se tam smazání odmítne (poznámka zůstane na místě). Testy sem nesahají — `lifecycle` dostává
//! tuhle funkci jako parametr.

use super::strip_verbatim;
use crate::i18n::tr;
use std::path::{Path, PathBuf};

/// Písmeno disku z cesty `C:\…` (`None` = UNC a jiné tvary).
fn drive_letter(p: &Path) -> Option<char> {
    let s = strip_verbatim(p).to_string_lossy().replace('/', "\\");
    let b = s.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\').then(|| b[0] as char)
}

/// `DRIVE_FIXED` z `GetDriveTypeW` (konstanta je v jiné feature, kvůli jedné hodnotě ji nezapínáme).
const DRIVE_FIXED: u32 = 3;

/// Pošle soubory do Koše (obnovitelné). Chyba = nic se nesmazalo navždy.
pub fn recycle(paths: &[PathBuf]) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows::Win32::UI::Shell::{FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW, SHFileOperationW};
    use windows::core::PCWSTR;
    if paths.is_empty() {
        return Ok(());
    }
    let no_bin = || tr("Koš je jen na místních discích — poznámka zůstala, smaž ji ručně.", "The Recycle Bin only exists on local drives — the note stayed, delete it by hand.");
    let mut from: Vec<u16> = Vec::new();
    for p in paths {
        let letter = drive_letter(p).ok_or_else(|| no_bin().to_string())?;
        let root: Vec<u16> = format!("{letter}:\\").encode_utf16().chain([0]).collect();
        // SAFETY: `root` je nulou zakončený řetězec, který žije celé volání.
        if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != DRIVE_FIXED {
            return Err(no_bin().into());
        }
        from.extend(std::ffi::OsString::from(strip_verbatim(p).to_string_lossy().replace('/', "\\")).encode_wide());
        from.push(0);
    }
    from.push(0); // seznam končí dvojitou nulou
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    // SAFETY: `op` i `from` žijí do konce volání, `pFrom` je dvojitě nulou zakončený seznam.
    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 || op.fAnyOperationsAborted.as_bool() {
        return Err(trf!("Koš nepřijal soubory (kód {code:#x}) — poznámka zůstala na místě.", "The Recycle Bin didn't accept the files (code {code:#x}) — the note stayed in place."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_drive_paths_have_a_bin() {
        assert_eq!(drive_letter(Path::new(r"C:\Users\x\Ledge\notes\a.md")), Some('C'));
        assert_eq!(drive_letter(Path::new("d:/notes/a.md")), Some('d'));
        assert_eq!(drive_letter(Path::new(r"\\?\C:\notes\a.md")), Some('C'));
        for bad in [r"\\srv\share\a.md", r"\\?\UNC\srv\share\a.md", "relativni/a.md", ""] {
            assert_eq!(drive_letter(Path::new(bad)), None, "{bad}");
        }
        assert!(recycle(&[]).is_ok());
        assert!(recycle(&[PathBuf::from(r"\\srv\share\a.md")]).is_err(), "síťová cesta se do Koše neposílá");
    }
}
