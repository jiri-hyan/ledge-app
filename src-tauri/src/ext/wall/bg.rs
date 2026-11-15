//! Pozadí tapety z vlastní fotky (kolo 34). Jádro (`commands/ext.rs` → `ext_wall_bg`) vezme soubor vybraný člověkem
//! v dialogu, zkontroluje ho (`check_src`: jen lokální absolutní cesta, přípona jpg/jpeg/png/webp z allowlistu
//! `personal::FILE_ALLOW`, velikost), WIC ho dekóduje a zmenšený JPEG (≤ `MAX_PX`) uloží jako KOPII do složky
//! rozšíření (`ext/wall/bg.jpg`). Rozšíření `wall` čte jen tuhle kopii (pevné jméno, žádná cesta od uživatele) a do
//! stránky ji pošle jako `data:` URL — žádné `file://`, stránka nic jiného nenačítá. Funkce berou složku jako
//! parametr (testy v dočasné složce, nikdy skutečné `%LOCALAPPDATA%\Ledge`).

use std::path::{Path, PathBuf};

/// Delší strana zmenšené fotky (1920×1080 ≈ 8 MB dekódovaných v prohlížeči; víc by tlačilo RAM tapety).
pub const MAX_PX: u32 = 1920;
/// Největší zdrojový soubor.
const MAX_SRC_BYTES: u64 = 40 << 20;
/// Největší kopie, kterou rozšíření pošle do stránky (zmenšená fotka bývá < 1 MB).
const MAX_COPY_BYTES: u64 = 6 << 20;
/// Přípony, které volba pozadí přijme (podmnožina `personal::FILE_ALLOW`).
const EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];

fn copy_path(dir: &Path) -> PathBuf {
    dir.join("bg.jpg")
}

fn name_path(dir: &Path) -> PathBuf {
    dir.join("bg.txt")
}

/// Smí být `path` zdrojem pozadí? (čistě; `size` = velikost souboru). Text chyby jde člověku do Nastavení.
pub fn check_src(path: &Path, size: u64) -> Result<(), String> {
    use crate::i18n::tr;
    let s = path.to_string_lossy();
    // UNC / zařízení (`\\server\…`, `\\?\…`) = síť nebo cizí zdroj: jen lokální disk
    if !path.is_absolute() || s.starts_with(r"\\") || s.starts_with("//") {
        return Err(tr("Vyber soubor z místního disku.", "Pick a file from a local drive.").into());
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !EXTS.contains(&ext.as_str()) || !crate::personal::FILE_ALLOW.contains(&ext.as_str()) {
        return Err(tr("Pozadí může být jen obrázek JPG, PNG nebo WebP.", "The background must be a JPG, PNG or WebP image.").into());
    }
    if size == 0 || size > MAX_SRC_BYTES {
        return Err(tr("Obrázek je prázdný nebo větší než 40 MB.", "The image is empty or larger than 40 MB.").into());
    }
    Ok(())
}

fn is_jpeg(b: &[u8]) -> bool {
    b.len() > 4 && b[..3] == [0xFF, 0xD8, 0xFF]
}

/// Název vybrané fotky pro Nastavení; `None` = pozadí není nastavené (nebo kopie chybí).
pub fn name(dir: &Path) -> Option<String> {
    copy_path(dir).is_file().then(|| std::fs::read_to_string(name_path(dir)).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| "bg.jpg".into()))
}

/// Zkontroluje, zmenší a uloží fotku `src` jako pozadí. Vrací název souboru pro Nastavení.
pub fn set_from(dir: &Path, src: &Path) -> Result<String, String> {
    use crate::i18n::tr;
    let meta = std::fs::metadata(src).map_err(|_| tr("Soubor se nepodařilo otevřít.", "Couldn't open the file.").to_string())?;
    if !meta.is_file() {
        return Err(tr("Vyber soubor, ne složku.", "Pick a file, not a folder.").into());
    }
    check_src(src, meta.len())?;
    let bytes = std::fs::read(src).map_err(|_| tr("Soubor se nepodařilo přečíst.", "Couldn't read the file.").to_string())?;
    let jpg = crate::iconimg::to_jpeg(&bytes, MAX_PX).ok_or_else(|| tr("Tohle není obrázek, který Ledge umí načíst.", "Ledge can't read this image.").to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // zápis přes dočasný soubor: rozšíření nikdy nepřečte půlku fotky
    let tmp = dir.join("bg.jpg.tmp");
    std::fs::write(&tmp, &jpg).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, copy_path(dir)).map_err(|e| e.to_string())?;
    let name: String = src.file_name().map(|n| n.to_string_lossy().chars().take(120).collect()).unwrap_or_else(|| "bg.jpg".into());
    let _ = std::fs::write(name_path(dir), &name);
    Ok(name)
}

/// Odebere pozadí (zpět přechod).
pub fn clear(dir: &Path) {
    let _ = std::fs::remove_file(copy_path(dir));
    let _ = std::fs::remove_file(name_path(dir));
}

/// Strana rozšíření: kopie jako `data:` URL do stránky. `None` = bez fotky nebo kopie není platný JPEG
/// (poškozený / podstrčený soubor se nepošle).
pub fn load_url(dir: &Path) -> Option<String> {
    let p = copy_path(dir);
    if std::fs::metadata(&p).ok()?.len() > MAX_COPY_BYTES {
        return None;
    }
    let b = std::fs::read(&p).ok()?;
    is_jpeg(&b).then(|| format!("data:image/jpeg;base64,{}", crate::icons::b64(&b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ledge-wallbg-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn check_src_table() {
        let mb = 1 << 20;
        let cases: [(&str, u64, bool, &str); 12] = [
            (r"C:\fotky\les.jpg", mb, true, "jpg"),
            (r"C:\fotky\LES.JPEG", mb, true, "velká písmena"),
            (r"D:\a\b.png", mb, true, "png"),
            (r"D:\a\b.webp", mb, true, "webp"),
            (r"C:\fotky\les.gif", mb, false, "gif mimo"),
            (r"C:\fotky\les.svg", mb, false, "svg mimo (skripty)"),
            (r"C:\fotky\les.exe", mb, false, "exe"),
            (r"C:\fotky\les", mb, false, "bez přípony"),
            (r"\\server\share\les.jpg", mb, false, "UNC"),
            (r"\\?\C:\les.jpg", mb, false, "zařízení"),
            (r"relativni\les.jpg", mb, false, "relativní"),
            (r"C:\fotky\les.jpg", 41 * mb, false, "příliš velký"),
        ];
        for (p, size, ok, name) in cases {
            assert_eq!(check_src(Path::new(p), size).is_ok(), ok, "{name}: {p}");
        }
        assert!(check_src(Path::new(r"C:\a.jpg"), 0).is_err(), "prázdný soubor");
    }

    #[test]
    fn jpeg_magic() {
        assert!(is_jpeg(&[0xFF, 0xD8, 0xFF, 0xE0, 0]));
        assert!(!is_jpeg(b"\x89PNG\r\n\x1a\n"));
        assert!(!is_jpeg(&[0xFF, 0xD8]));
    }

    #[test]
    fn set_converts_to_small_jpeg_and_clear_removes() {
        let dir = tmp("set");
        let src = dir.join("velka.png");
        // 3000×1000 px → zmenšit na 1920×640
        let px = [10u8, 120, 200, 255].repeat(3000 * 1000);
        std::fs::write(&src, crate::iconimg::png_from_bgra(3000, 1000, &px)).unwrap();
        let out = dir.join("ext");
        assert_eq!(name(&out), None, "před nastavením nic");
        assert_eq!(set_from(&out, &src).unwrap(), "velka.png");
        assert_eq!(name(&out).as_deref(), Some("velka.png"));
        let url = load_url(&out).expect("kopie je JPEG");
        assert!(url.starts_with("data:image/jpeg;base64,"));
        let jpg = std::fs::read(out.join("bg.jpg")).unwrap();
        let (w, h, _) = crate::iconimg::decode_bgra(&jpg, 100_000_000).expect("JPEG jde dekódovat");
        assert_eq!((w, h), (1920, 640));
        clear(&out);
        assert_eq!(name(&out), None);
        assert_eq!(load_url(&out), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_rejects_garbage_and_bad_paths() {
        let dir = tmp("bad");
        let out = dir.join("ext");
        let fake = dir.join("falesny.jpg");
        std::fs::write(&fake, b"to neni obrazek").unwrap();
        assert!(set_from(&out, &fake).is_err(), "text s příponou jpg");
        let txt = dir.join("poznamka.txt");
        std::fs::write(&txt, b"x").unwrap();
        assert!(set_from(&out, &txt).is_err(), "přípona mimo allowlist");
        assert!(set_from(&out, &dir).is_err(), "složka");
        assert!(set_from(&out, &dir.join("neni.jpg")).is_err(), "neexistuje");
        assert_eq!(name(&out), None, "po chybě nic nezůstane");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_url_ignores_planted_non_jpeg() {
        let dir = tmp("plant");
        std::fs::write(dir.join("bg.jpg"), b"<html><script>alert(1)</script>").unwrap();
        assert_eq!(load_url(&dir), None, "jen platný JPEG");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
