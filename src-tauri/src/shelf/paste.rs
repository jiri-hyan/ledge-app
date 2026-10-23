//! Vložení ze schránky do přihrádky (tlačítko Vložit v kartě, paleta) a zmenšení obrázku z přihrádky.
//! Soubory z Průzkumníka (`CF_HDROP`) se přidají jako odkazy (jako tah na proužek; síťové cesty UNC ne).
//! Obrázek (`PNG` / `CF_DIB`) a text (`CF_UNICODETEXT`) nemají cestu → uloží se jako NOVÝ soubor do vlastní
//! složky (`drops.rs`). Stejný filtr jako historie schránky (`cliphist::filter`): značka „neukládat“ (správci
//! hesel), vyloučené aplikace a text, který vypadá jako token / klíč, se nevloží. Obrázek nad `MAX_PIXELS` se
//! nedekóduje. Jen na výslovný klik; v klidu nic neběží. Obsah schránky se nikam neposílá ani nepíše do logu.

use crate::i18n::tr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::*;
use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{CF_HDROP, CF_UNICODETEXT};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::core::Interface;

/// Strop obrázku ze schránky (bajty) — větší se nevloží (paměť `ledge.exe`).
pub const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
/// Strop textu ze schránky (UTF-16 jednotek ≈ znaků).
pub const MAX_TEXT_UNITS: usize = 1024 * 1024;
const CF_DIB: u32 = 8;

/// Strop dekódovaného obrázku (pixely) — větší by WIC rozbalil do stovek MB.
pub const MAX_PIXELS: u64 = 100_000_000;

/// Akce navíc u položky přihrádky podle přípony (jediná pravda; UI ji dostane v `shelf_list`).
const TEXT_EXTS: [&str; 37] = [
    "txt", "md", "markdown", "json", "jsonc", "ts", "tsx", "js", "jsx", "mjs", "rs", "py", "log", "csv", "yml", "yaml", "html", "htm", "css", "scss", "toml", "xml",
    "ini", "sh", "bat", "cmd", "ps1", "sql", "c", "cpp", "h", "hpp", "cs", "go", "java", "kt", "env",
];
const IMAGE_EXTS: [&str; 9] = ["png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff", "ico"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Extra {
    /// „Obsah“ — zkopírovat text
    Text,
    /// „Zmenšit“ — PNG 1024 px
    Image,
}

pub fn extra_of(path: &str) -> Option<Extra> {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() {
        return None;
    }
    let ext = ext.to_ascii_lowercase();
    if TEXT_EXTS.contains(&ext.as_str()) {
        Some(Extra::Text)
    } else if IMAGE_EXTS.contains(&ext.as_str()) {
        Some(Extra::Image)
    } else {
        None
    }
}

/// Síťová cesta (`\\server\share`, `\\?\UNC\…`, `\\.\…`)? Lokální `\\?\C:\…` ne.
pub fn is_remote(path: &str) -> bool {
    let p = path.replace('/', "\\");
    if let Some(rest) = p.strip_prefix("\\\\?\\") {
        let b = rest.as_bytes();
        return !(b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':');
    }
    p.starts_with("\\\\")
}

/// DIB ze schránky (`CF_DIB`: BITMAPINFOHEADER + paleta + pixely) → plný BMP s 14B `BITMAPFILEHEADER`
/// (WIC umí jen soubor). Krátký vstup se vrátí beze změny (dekodér ho pak odmítne).
pub fn dib_to_bmp(dib: &[u8]) -> Vec<u8> {
    if dib.len() < 40 {
        return dib.to_vec();
    }
    let u32_at = |i: usize| u32::from_le_bytes([dib[i], dib[i + 1], dib[i + 2], dib[i + 3]]);
    let header_size = u32_at(0);
    let bit_count = u16::from_le_bytes([dib[14], dib[15]]);
    let compression = u32_at(16);
    let clr_used = u32_at(32);
    let palette_colors = if clr_used > 0 {
        clr_used.min(256)
    } else if (1..=8).contains(&bit_count) {
        1u32 << bit_count
    } else {
        0
    };
    let mut off_bits = 14u32.saturating_add(header_size).saturating_add(palette_colors * 4);
    // BI_BITFIELDS se 40B hlavičkou: za ní jsou 3 masky barev
    if compression == 3 && bit_count >= 16 && header_size == 40 {
        off_bits += 12;
    }
    let total = 14 + dib.len() as u32;
    let mut bmp = Vec::with_capacity(total as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&total.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&off_bits.to_le_bytes());
    bmp.extend_from_slice(dib);
    bmp
}

/// Obrázek v libovolném formátu, který zná WIC → PNG, volitelně zmenšený tak, aby delší strana ≤ `max_px`.
pub fn image_to_png(bytes: &[u8], max_px: Option<u32>) -> Result<Vec<u8>, String> {
    let _com = crate::iconimg::Com::init();
    let fail = |what: &str, e: windows::core::Error| format!("{}: {what} ({e})", tr("Obrázek nejde převést", "Can't convert the image"));
    unsafe {
        let f: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).map_err(|e| fail("WIC", e))?;
        let stream = f.CreateStream().map_err(|e| fail("stream", e))?;
        stream.InitializeFromMemory(bytes).map_err(|e| fail("stream", e))?;
        let dec = f.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand).map_err(|e| fail("decoder", e))?;
        let frame = dec.GetFrame(0).map_err(|e| fail("frame", e))?;
        let (mut w, mut h) = (0, 0);
        frame.GetSize(&mut w, &mut h).map_err(|e| fail("size", e))?;
        if u64::from(w) * u64::from(h) > MAX_PIXELS {
            return Err(tr("Obrázek je moc velký (nad 100 megapixelů).", "The image is too large (over 100 megapixels).").into());
        }
        let (tw, th) = match max_px {
            Some(max) => crate::iconimg::fit(w, h, max),
            None => (w, h),
        };
        let src: IWICBitmapSource = if (tw, th) == (w, h) {
            frame.cast().map_err(|e| fail("cast", e))?
        } else {
            let conv = f.CreateFormatConverter().map_err(|e| fail("convert", e))?;
            conv.Initialize(&frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None::<&IWICPalette>, 0.0, WICBitmapPaletteTypeCustom)
                .map_err(|e| fail("convert", e))?;
            let scaler = f.CreateBitmapScaler().map_err(|e| fail("scale", e))?;
            scaler.Initialize(&conv, tw, th, WICBitmapInterpolationModeFant).map_err(|e| fail("scale", e))?;
            scaler.cast().map_err(|e| fail("cast", e))?
        };
        let bgra = f.CreateFormatConverter().map_err(|e| fail("convert", e))?;
        bgra.Initialize(&src, &GUID_WICPixelFormat32bppBGRA, WICBitmapDitherTypeNone, None::<&IWICPalette>, 0.0, WICBitmapPaletteTypeCustom)
            .map_err(|e| fail("convert", e))?;
        let out = CreateStreamOnHGlobal(HGLOBAL::default(), true).map_err(|e| fail("stream", e))?;
        let enc = f.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null()).map_err(|e| fail("encoder", e))?;
        enc.Initialize(&out, WICBitmapEncoderNoCache).map_err(|e| fail("encoder", e))?;
        let (mut of, mut props) = (None, None);
        enc.CreateNewFrame(&mut of, &mut props).map_err(|e| fail("frame", e))?;
        let of = of.ok_or_else(|| tr("Obrázek nejde převést", "Can't convert the image").to_string())?;
        of.Initialize(props.as_ref()).map_err(|e| fail("frame", e))?;
        of.SetSize(tw, th).map_err(|e| fail("frame", e))?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        of.SetPixelFormat(&mut fmt).map_err(|e| fail("frame", e))?;
        of.WriteSource(&bgra, std::ptr::null()).map_err(|e| fail("write", e))?;
        of.Commit().map_err(|e| fail("commit", e))?;
        enc.Commit().map_err(|e| fail("commit", e))?;
        let mut len = 0u64;
        out.Seek(0, STREAM_SEEK_END, Some(&mut len)).map_err(|e| fail("seek", e))?;
        out.Seek(0, STREAM_SEEK_SET, None).map_err(|e| fail("seek", e))?;
        let mut buf = vec![0u8; len as usize];
        let mut read = 0u32;
        out.Read(buf.as_mut_ptr().cast(), len as u32, Some(&mut read)).ok().map_err(|e| fail("read", e))?;
        buf.truncate(read as usize);
        Ok(buf)
    }
}

/// Co bylo ve schránce (čte se pod otevřenou schránkou, ukládá až po jejím zavření).
enum Clip {
    Files(Vec<String>),
    Png(Vec<u8>),
    Dib(Vec<u8>),
    Text(String),
    TooBig,
    /// značka „neukládat“, vyloučená aplikace nebo token / klíč (`cliphist::filter`)
    Refused,
    Nothing,
}

/// Bajty z `HGLOBAL` schránky (nejvýš `max`; větší = `None`).
unsafe fn global_bytes(h: HGLOBAL, max: usize) -> Option<Vec<u8>> {
    let size = unsafe { GlobalSize(h) };
    if size == 0 || size > max {
        return None;
    }
    let ptr = unsafe { GlobalLock(h) } as *const u8;
    if ptr.is_null() {
        return None;
    }
    let v = unsafe { std::slice::from_raw_parts(ptr, size) }.to_vec();
    let _ = unsafe { GlobalUnlock(h) };
    Some(v)
}

/// Text `CF_UNICODETEXT` — jen do konce bloku (`GlobalSize`), ne podle nulového znaku naslepo.
fn utf16_until_nul(units: &[u16]) -> String {
    let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

unsafe fn read_clip(exclude_apps: &[String]) -> Clip {
    unsafe {
        let fmts = crate::cliphist::win::formats();
        if crate::cliphist::filter::pre_verdict(&fmts, crate::cliphist::win::owner_exe().as_deref(), exclude_apps).is_some() {
            return Clip::Refused;
        }
        if IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok()
            && let Ok(h) = GetClipboardData(CF_HDROP.0 as u32)
        {
            let drop = HDROP(h.0 as _);
            let n = DragQueryFileW(drop, u32::MAX, None).min(super::MAX_ITEMS as u32);
            let mut paths = Vec::new();
            for i in 0..n {
                let len = DragQueryFileW(drop, i, None) as usize;
                let mut buf = vec![0u16; len + 1];
                let got = DragQueryFileW(drop, i, Some(&mut buf)) as usize;
                if got > 0 {
                    paths.push(String::from_utf16_lossy(&buf[..got]));
                }
            }
            return Clip::Files(paths);
        }
        let png = RegisterClipboardFormatW(windows::core::w!("PNG"));
        for (fmt, png_fmt) in [(png, true), (CF_DIB, false)] {
            if fmt != 0
                && IsClipboardFormatAvailable(fmt).is_ok()
                && let Ok(h) = GetClipboardData(fmt)
            {
                return match global_bytes(HGLOBAL(h.0 as _), MAX_IMAGE_BYTES) {
                    Some(b) if png_fmt => Clip::Png(b),
                    Some(b) => Clip::Dib(b),
                    None => Clip::TooBig,
                };
            }
        }
        if IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok()
            && let Ok(h) = GetClipboardData(CF_UNICODETEXT.0 as u32)
        {
            return match global_bytes(HGLOBAL(h.0 as _), MAX_TEXT_UNITS * 2) {
                Some(b) => {
                    let units: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                    let text = utf16_until_nul(&units);
                    if crate::cliphist::filter::looks_secret(&text) { Clip::Refused } else { Clip::Text(text) }
                }
                None => Clip::TooBig,
            };
        }
        Clip::Nothing
    }
}

/// Vložit obsah schránky do přihrádky. Vrací počet přidaných položek.
pub fn paste_clipboard() -> Result<usize, String> {
    let exclude = crate::state::core().cfg.read().clipboard.exclude_apps.clone();
    let clip = unsafe {
        let mut open = false;
        for _ in 0..10 {
            if OpenClipboard(None).is_ok() {
                open = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        if !open {
            return Err(tr("Schránka je obsazená jinou aplikací — zkus to znovu.", "The clipboard is busy in another app — try again.").into());
        }
        let c = read_clip(&exclude);
        let _ = CloseClipboard();
        c
    };
    let stamp = format!("clip-{}", crate::state::now_ms().max(0));
    match clip {
        Clip::Files(paths) if !paths.is_empty() => {
            let local: Vec<String> = paths.into_iter().filter(|p| !is_remote(p)).collect();
            if local.is_empty() {
                return Err(tr("Síťové cesty (\\\\server\\…) přihrádka neukládá.", "The shelf doesn't keep network paths (\\\\server\\…).").into());
            }
            let added = super::add_paths(&local)?;
            Ok(added.new + added.moved)
        }
        Clip::Png(b) => super::drops::save(&stamp, "png", &b).map(|_| 1),
        Clip::Dib(b) => super::drops::save(&stamp, "png", &image_to_png(&dib_to_bmp(&b), None)?).map(|_| 1),
        Clip::Text(s) if !s.trim().is_empty() => super::drops::save(&stamp, "txt", s.as_bytes()).map(|_| 1),
        Clip::Refused => Err(tr("Tohle Ledge do přihrádky nevloží — vypadá to jako heslo nebo klíč, nebo to aplikace označila jako neukládat.", "Ledge won't paste this to the shelf — it looks like a password or key, or the app marked it as do-not-save.").into()),
        Clip::TooBig => Err(tr("Obsah schránky je moc velký pro přihrádku.", "The clipboard content is too large for the shelf.").into()),
        _ => Err(tr("Ve schránce nejsou soubory, obrázek ani text.", "There are no files, image or text on the clipboard.").into()),
    }
}

/// Zmenšená kopie obrázku z přihrádky (delší strana ≤ `max_px`, PNG) do `shelf-drops`; originál se nemění.
pub fn resized_copy(src: &Path, max_px: u32) -> Result<PathBuf, String> {
    let meta = std::fs::metadata(src).map_err(|e| crate::errors::io(tr("Obrázek nejde přečíst", "Can't read the image"), &e))?;
    if !meta.is_file() || meta.len() > MAX_IMAGE_BYTES as u64 || extra_of(&src.to_string_lossy()) != Some(Extra::Image) {
        return Err(tr("Tohle Ledge jako obrázek nezmenší (složka nebo víc než 32 MB).", "Ledge won't resize this as an image (a folder or over 32 MB).").into());
    }
    let bytes = std::fs::read(src).map_err(|e| crate::errors::io(tr("Obrázek nejde přečíst", "Can't read the image"), &e))?;
    let png = image_to_png(&bytes, Some(max_px))?;
    let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    super::drops::save(&format!("{stem}-{max_px}px"), "png", &png)
}

/// Textový obsah souboru z přihrádky (nejvýš `max` bajtů, jen UTF-8) — pro „Kopírovat obsah“.
pub fn text_of(src: &Path, max: u64) -> Result<String, String> {
    let meta = std::fs::metadata(src).map_err(|e| crate::errors::io(tr("Soubor nejde přečíst", "Can't read the file"), &e))?;
    if !meta.is_file() || extra_of(&src.to_string_lossy()) != Some(Extra::Text) {
        return Err(tr("Obsah jde zkopírovat jen u textového souboru (.txt, .md, .json…).", "Content can be copied only from a text file (.txt, .md, .json…).").into());
    }
    if meta.len() > max {
        return Err(trf!("Soubor je moc velký (nejvýš {} kB).", "The file is too large (max {} kB).", max / 1024));
    }
    let bytes = std::fs::read(src).map_err(|e| crate::errors::io(tr("Soubor nejde přečíst", "Can't read the file"), &e))?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes).to_vec();
    String::from_utf8(bytes).map_err(|_| tr("Soubor není čitelný text (UTF-8).", "The file is not readable text (UTF-8).").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dib(bits: u16, compression: u32, clr_used: u32, extra: usize) -> Vec<u8> {
        let mut d = vec![0u8; 40];
        d[0..4].copy_from_slice(&40u32.to_le_bytes());
        d[4..8].copy_from_slice(&2i32.to_le_bytes());
        d[8..12].copy_from_slice(&2i32.to_le_bytes());
        d[12..14].copy_from_slice(&1u16.to_le_bytes());
        d[14..16].copy_from_slice(&bits.to_le_bytes());
        d[16..20].copy_from_slice(&compression.to_le_bytes());
        d[32..36].copy_from_slice(&clr_used.to_le_bytes());
        d.extend(std::iter::repeat_n(0xFFu8, extra));
        d
    }

    #[test]
    fn dib_to_bmp_header_table() {
        // (bitů, komprese, barev v paletě) → kde začínají pixely
        let rows = [(24u16, 0u32, 0u32, 14 + 40), (32, 3, 0, 14 + 40 + 12), (8, 0, 0, 14 + 40 + 256 * 4), (8, 0, 16, 14 + 40 + 16 * 4), (1, 0, 0, 14 + 40 + 2 * 4), (32, 0, 0, 14 + 40)];
        for (bits, comp, used, off) in rows {
            let d = dib(bits, comp, used, 16);
            let bmp = dib_to_bmp(&d);
            assert_eq!(&bmp[0..2], b"BM");
            assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()), 14 + d.len() as u32);
            assert_eq!(u32::from_le_bytes(bmp[10..14].try_into().unwrap()), off, "{bits} bitů, komprese {comp}, paleta {used}");
            assert_eq!(&bmp[14..], &d[..]);
        }
        assert_eq!(dib_to_bmp(&[1, 2, 3]), vec![1, 2, 3], "krátký vstup beze změny");
    }

    #[test]
    fn extra_by_extension() {
        let rows = [("C:\\N\\poznamka.md", Some(Extra::Text)), ("D:/data/DATA.JSON", Some(Extra::Text)), ("snimek.PNG", Some(Extra::Image)), ("archiv.zip", None), ("bez-pripony", None), (".env", None), ("C:\\x.y\\soubor", None)];
        for (p, want) in rows {
            assert_eq!(extra_of(p), want, "{p}");
        }
    }

    #[test]
    fn remote_paths() {
        let rows = [("\\\\server\\share\\a.txt", true), ("\\\\?\\UNC\\server\\share\\a", true), ("\\\\.\\pipe\\x", true), ("//server/share/a", true), ("\\\\?\\C:\\dlouha\\cesta", false), ("C:\\Users\\a.txt", false), ("D:/a", false)];
        for (p, want) in rows {
            assert_eq!(is_remote(p), want, "{p}");
        }
    }

    #[test]
    fn utf16_text_stops_at_nul_or_block_end() {
        let w = |s: &str| s.encode_utf16().collect::<Vec<_>>();
        assert_eq!(utf16_until_nul(&[w("ahoj"), vec![0, 65]].concat()), "ahoj");
        assert_eq!(utf16_until_nul(&w("bez nuly")), "bez nuly", "chybějící nula = konec bloku, ne čtení za ním");
        assert_eq!(utf16_until_nul(&[]), "");
    }
}
