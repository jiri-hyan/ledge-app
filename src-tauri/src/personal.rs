//! Rychlé odkazy záložky Osobní — rozpoznání typu cíle a bezpečnostní kontrola.
//!
//! Config smí měnit i Claude, takže cíl odkazu je nedůvěryhodný vstup (prompt injection):
//! - web jen `http(s):`; jiné schéma (`file:`, `javascript:`, `ms-settings:`…) se odmítne;
//! - síťové a zařízení cesty (`\\server\…`, `\\?\…`, `\\.\…`) se odmítnou — ani se nestatují;
//! - jen absolutní cesta s písmenem disku; dvojtečka jinde (alternativní datový proud) se odmítne;
//! - přípona se čte z NORMALIZOVANÉ cesty (Windows zahazuje koncové tečky a mezery: `x.bat.` = `x.bat`);
//! - soubor jen z allowlistu dokumentů/médií — skripty, instalátory, zástupce (`.lnk`, `.url`, které by
//!   obešly zákaz argumentů) ani nic dalšího, co ShellExecute spustí, neprojde;
//! - aplikace jen `.exe`, spouští se bez argumentů; holé jméno se hledá jen v `%SystemRoot%\System32`
//!   a `%SystemRoot%` (ne v celém PATH — prázdné/relativní segmenty, podvržení).
//!
//! Favicony se nestahují — UI ukáže monogram a ikonu typu.

use crate::i18n::tr;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkKind {
    Web,
    Folder,
    File,
    App,
    /// nejde otevřít (důvod v `LinkTarget::Invalid`)
    Invalid,
    /// ještě neověřeno (klasifikace běží na pozadí)
    Pending,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    Web(String),
    Folder(PathBuf),
    File(PathBuf),
    App(PathBuf),
    Invalid(String),
}

impl LinkTarget {
    pub fn kind(&self) -> LinkKind {
        match self {
            LinkTarget::Web(_) => LinkKind::Web,
            LinkTarget::Folder(_) => LinkKind::Folder,
            LinkTarget::File(_) => LinkKind::File,
            LinkTarget::App(_) => LinkKind::App,
            LinkTarget::Invalid(_) => LinkKind::Invalid,
        }
    }
}

/// Přípony souborů, které odkaz smí otevřít výchozí aplikací (dokumenty, obrázky, média, zip).
pub const FILE_ALLOW: &[&str] = &[
    "txt", "md", "pdf", "doc", "docx", "xls", "xlsx", "csv", "ppt", "pptx", "odt", "ods", "png", "jpg", "jpeg", "gif", "webp",
    "svg", "mp3", "mp4", "mkv", "zip",
];

/// `scheme:` na začátku (aspoň 2 znaky, aby `C:\…` zůstala cesta).
fn scheme(s: &str) -> Option<&str> {
    let i = s.find(':')?;
    let sch = &s[..i];
    let ok = sch.len() >= 2
        && sch.starts_with(|c: char| c.is_ascii_alphabetic())
        && sch.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    ok.then_some(sch)
}

fn ext_lower(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// Cesta tak, jak ji uvidí Windows: bez koncových teček a mezer. Odmítne UNC/zařízení
/// (`\\…`, `//…`) a dvojtečku mimo písmeno disku (ADS `a.txt:evil.exe`).
pub fn normalize(t: &str) -> Result<String, String> {
    let sep = |c: char| c == '\\' || c == '/';
    let mut ch = t.chars();
    if ch.next().is_some_and(sep) && ch.next().is_some_and(sep) {
        return Err(tr("síťové a zařízení cesty (\\\\…) nejsou povolené", "network and device paths (\\\\…) aren't allowed").into());
    }
    let drive = t.len() >= 2 && t.as_bytes()[0].is_ascii_alphabetic() && t.as_bytes()[1] == b':';
    if t.char_indices().any(|(i, c)| c == ':' && !(i == 1 && drive)) {
        return Err(tr("dvojtečka v cestě (alternativní datový proud) není povolená", "a colon in the path (alternate data stream) isn't allowed").into());
    }
    let n = t.trim_end_matches(['.', ' ']);
    if n.is_empty() || n.ends_with(sep) && n.len() < t.len() && !n.ends_with(":\\") && !n.ends_with(":/") {
        // „C:\složka\.“ apod. — Windows by z toho udělal něco jiného, než je napsané
        return Err(tr("cesta končí tečkou", "the path ends with a dot").into());
    }
    Ok(n.to_string())
}

/// Rozpozná cíl. `sys_dirs` = kde hledat holé `appka.exe` (System32, Windows); `is_dir`/`is_file`
/// jsou parametry kvůli testům bez disku.
pub fn classify(
    target: &str,
    sys_dirs: &[PathBuf],
    is_dir: &dyn Fn(&Path) -> bool,
    is_file: &dyn Fn(&Path) -> bool,
) -> LinkTarget {
    let t = target.trim();
    if t.is_empty() {
        return LinkTarget::Invalid(tr("prázdný cíl", "empty target").into());
    }
    if let Some(sch) = scheme(t) {
        let sch = sch.to_ascii_lowercase();
        if sch != "http" && sch != "https" {
            return LinkTarget::Invalid(trf!("schéma „{sch}:“ není povolené (jen http/https)", "scheme “{sch}:” isn't allowed (only http/https)"));
        }
        let rest = &t[sch.len() + 1..];
        if !rest.starts_with("//") || rest.len() <= 2 || t.chars().any(char::is_whitespace) {
            return LinkTarget::Invalid(tr("neplatná adresa", "invalid address").into());
        }
        return LinkTarget::Web(t.to_string());
    }
    if t.starts_with('-') || t.contains('"') {
        return LinkTarget::Invalid(tr("cíl nesmí obsahovat argumenty ani uvozovky", "the target must not contain arguments or quotes").into());
    }
    let n = match normalize(t) {
        Ok(n) => n,
        Err(why) => return LinkTarget::Invalid(why),
    };
    let p = PathBuf::from(&n);
    // holé jméno .exe (bez složky) → jen System32 a Windows
    if p.components().count() == 1 && !p.is_absolute() && !n.contains(['\\', '/']) {
        if ext_lower(&p) != "exe" {
            return LinkTarget::Invalid(tr("cesta musí být absolutní (holé jméno jen u .exe ze složky Windows)", "the path must be absolute (a bare name only for an .exe from the Windows folder)").into());
        }
        return match sys_dirs.iter().map(|d| d.join(&p)).find(|c| is_file(c)) {
            Some(found) => LinkTarget::App(found),
            None => LinkTarget::Invalid(trf!("{n} není ve složce Windows — zadej celou cestu", "{n} isn't in the Windows folder — enter the full path")),
        };
    }
    if !p.is_absolute() {
        return LinkTarget::Invalid(tr("cesta musí být absolutní (C:\\…)", "the path must be absolute (C:\\…)").into());
    }
    if is_dir(&p) {
        return LinkTarget::Folder(p);
    }
    if !is_file(&p) {
        return LinkTarget::Invalid("cesta neexistuje".into());
    }
    match ext_lower(&p).as_str() {
        "exe" => LinkTarget::App(p),
        e if FILE_ALLOW.contains(&e) => LinkTarget::File(p),
        "" => LinkTarget::Invalid(tr("soubor bez přípony se z odkazu neotevírá", "a file without an extension isn't opened from a link").into()),
        e => LinkTarget::Invalid(trf!("typ .{e} se z odkazu neotevírá (jen dokumenty, obrázky, média, zip a .exe)", "type .{e} isn't opened from a link (only documents, images, media, zip and .exe)")),
    }
}

/// `%SystemRoot%\System32` a `%SystemRoot%` — jediná místa pro holé jméno `.exe`.
pub fn sys_dirs() -> Vec<PathBuf> {
    let root = std::env::var_os("SystemRoot").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    vec![root.join("System32"), root]
}

/// Totéž nad skutečným diskem. Stat — volat mimo UI/emit vlákno.
pub fn classify_fs(target: &str) -> LinkTarget {
    classify(target, &sys_dirs(), &|p| p.is_dir(), &|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: &[&str] = &[
        r"C:\Windows\System32\notepad.exe",
        r"C:\Windows\explorer.exe",
        r"C:\Hry\hra.exe",
        r"C:\Users\x\nakup.txt",
        r"C:\Users\x\Výpis.PDF",
        r"C:\Users\x\x.bat",
        r"C:\Users\x\x.lnk",
        r"C:\Users\x\x.url",
        r"C:\Users\x\x.PS1",
        r"C:\Users\x\a.txt",
        r"C:\Users\x\bez",
        r"C:\Tools\evil.exe",
    ];

    fn run(t: &str) -> LinkTarget {
        let dirs = vec![PathBuf::from(r"C:\Windows\System32"), PathBuf::from(r"C:\Windows")];
        let is_dir = |p: &Path| p == Path::new(r"C:\Users\x\Fotky");
        let is_file = |p: &Path| FILES.iter().any(|f| p == Path::new(f));
        classify(t, &dirs, &is_dir, &is_file)
    }

    #[test]
    fn web_only_http_s() {
        assert_eq!(run("https://mapy.cz"), LinkTarget::Web("https://mapy.cz".into()));
        assert_eq!(run(" HTTP://example.com/a?b=1 "), LinkTarget::Web("HTTP://example.com/a?b=1".into()));
        for bad in ["javascript:alert(1)", "file:///C:/x", "ms-settings:display", "ftp://x", "mailto:a@b.cz"] {
            assert_eq!(run(bad).kind(), LinkKind::Invalid, "{bad}");
        }
        assert_eq!(run("https://").kind(), LinkKind::Invalid);
        assert_eq!(run("https:mapy.cz").kind(), LinkKind::Invalid);
        assert_eq!(run("https://a b").kind(), LinkKind::Invalid);
    }

    #[test]
    fn paths_must_exist() {
        assert_eq!(run(r"C:\Users\x\Fotky"), LinkTarget::Folder(r"C:\Users\x\Fotky".into()));
        assert_eq!(run(r"C:\Users\x\nakup.txt"), LinkTarget::File(r"C:\Users\x\nakup.txt".into()));
        assert_eq!(run(r"C:\Hry\hra.exe"), LinkTarget::App(r"C:\Hry\hra.exe".into()));
        assert_eq!(run(r"C:\neni\tu"), LinkTarget::Invalid("cesta neexistuje".into()));
        assert_eq!(run(r"relativni\slozka").kind(), LinkKind::Invalid);
        assert_eq!(run(r"\bez\disku").kind(), LinkKind::Invalid);
        assert_eq!(run("").kind(), LinkKind::Invalid);
    }

    /// (cíl, očekávaný typ) — spustitelné, zástupce, ADS a triky s příponou neprojdou.
    #[test]
    fn file_types_table() {
        let cases: &[(&str, LinkKind)] = &[
            (r"C:\Users\x\a.txt", LinkKind::File),
            (r"C:\Users\x\Výpis.PDF", LinkKind::File),
            (r"C:\Users\x\a.txt.. ", LinkKind::File),
            (r"C:\Users\x\x.bat", LinkKind::Invalid),
            (r"C:\Users\x\x.bat.", LinkKind::Invalid),
            (r"C:\Users\x\x.bat . .", LinkKind::Invalid),
            (r"C:\Users\x\a.txt:evil.exe", LinkKind::Invalid),
            (r"C:\Users\x\a.txt::$DATA", LinkKind::Invalid),
            (r"C:\Users\x\x.lnk", LinkKind::Invalid),
            (r"C:\Users\x\x.url", LinkKind::Invalid),
            (r"C:\Users\x\x.PS1", LinkKind::Invalid),
            (r"C:\Users\x\bez", LinkKind::Invalid),
            (r"C:\Hry\hra.exe", LinkKind::App),
            (r"C:\Hry\hra.exe.", LinkKind::App),
        ];
        for &(t, k) in cases {
            assert_eq!(run(t).kind(), k, "{t}");
        }
        // otevírá se normalizovaná cesta, ne to, co stálo v configu
        assert_eq!(run(r"C:\Users\x\a.txt.. "), LinkTarget::File(r"C:\Users\x\a.txt".into()));
        assert!(matches!(run(r"C:\Users\x\x.bat."), LinkTarget::Invalid(w) if w.contains(".bat")));
    }

    #[test]
    fn unc_and_device_paths_rejected_without_stat() {
        let stat = std::cell::Cell::new(0);
        let count = |_: &Path| {
            stat.set(stat.get() + 1);
            true
        };
        for t in [r"\\server\share\a.txt", r"\\?\C:\Users\x\a.txt", r"\\.\PhysicalDrive0", "//server/share/a.txt", r"\/server\a.txt"] {
            let r = classify(t, &[], &count, &count);
            assert_eq!(r.kind(), LinkKind::Invalid, "{t}");
        }
        assert_eq!(stat.get(), 0, "UNC/zařízení se nesmí ani statovat");
    }

    #[test]
    fn bare_exe_only_from_windows_dirs_and_no_args() {
        assert_eq!(run("notepad.exe"), LinkTarget::App(r"C:\Windows\System32\notepad.exe".into()));
        assert_eq!(run("explorer.exe"), LinkTarget::App(r"C:\Windows\explorer.exe".into()));
        // evil.exe leží jinde (třeba v PATH) — holé jméno se tam nehledá
        assert_eq!(run("evil.exe").kind(), LinkKind::Invalid);
        assert_eq!(run("notepad").kind(), LinkKind::Invalid);
        assert_eq!(run("x.bat").kind(), LinkKind::Invalid);
        // argumenty v cíli nejdou: celý řetězec je jedna (neexistující) cesta, nic se nespustí
        assert_eq!(run(r"C:\Hry\hra.exe --cheat").kind(), LinkKind::Invalid);
        assert_eq!(run(r#""C:\Hry\hra.exe" x"#).kind(), LinkKind::Invalid);
        assert_eq!(run("-x").kind(), LinkKind::Invalid);
    }

    #[test]
    fn normalize_rules() {
        assert_eq!(normalize(r"C:\a\b.txt. . ").unwrap(), r"C:\a\b.txt");
        assert_eq!(normalize(r"C:\").unwrap(), r"C:\");
        assert!(normalize(r"C:\a\.").is_err());
        assert!(normalize(r"a:\b:c").is_err());
        assert!(normalize(r"ab:c").is_err());
        // „C:soubor“ (relativní k aktuální složce disku) projde normalizací, ale ne klasifikací
        assert_eq!(normalize(r"C:a.txt").unwrap(), r"C:a.txt");
        assert_eq!(run(r"C:a.txt").kind(), LinkKind::Invalid);
        assert!(normalize(r"\\x\y").is_err());
        assert!(normalize(".").is_err());
    }
}
