//! Rastrové ikony přes WIC (Windows Imaging Component — součást Windows, žádná nová závislost):
//! PNG/ICO/JPEG/WebP/BMP → největší snímek → zmenšení na ≤ [`MAX_PX`] (Fant, premultiplikovaná alfa)
//! → PNG. Ikona z `.exe` přes `PrivateExtractIconsW` → `CreateBitmapFromHICON` → stejný PNG výstup.
//! Ikona souboru/složky přes `SHGetFileInfoW` (`shell_png`). Volají vlákna `logos` (`icons.rs`) a
//! `action-icons` (`actionicons.rs`); COM (MTA) si každé volání inicializuje samo.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{E_FAIL, HGLOBAL};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON, PrivateExtractIconsW};
use windows::core::{Error, Interface, Result};

/// Strana výstupu: dlaždice má 56 CSS px, při 200 % DPI 112 px.
pub const MAX_PX: u32 = 128;

/// COM pro aktuální vlákno; `S_FALSE` (už inicializováno) se párově odinicializuje taky.
struct Com(bool);

impl Com {
    fn init() -> Self {
        Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() }
        }
    }
}

fn factory() -> Result<IWICImagingFactory> {
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
}

/// Rozměr, který se vejde do `max`×`max` se zachovaným poměrem stran (nikdy nezvětšuje, min 1 px).
pub fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w <= max && h <= max {
        return (w, h);
    }
    let (w64, h64, m) = (w as u64, h as u64, max as u64);
    if w >= h { (max, ((h64 * m + w64 / 2) / w64).max(1) as u32) } else { (((w64 * m + h64 / 2) / h64).max(1) as u32, max) }
}

/// Obrázek libovolného formátu, který umí WIC → PNG ≤ [`MAX_PX`]; u ICO největší snímek.
pub fn to_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let _com = Com::init();
    decode_largest(bytes).ok()
}

fn decode_largest(bytes: &[u8]) -> Result<Vec<u8>> {
    let f = factory()?;
    unsafe {
        let stream = f.CreateStream()?;
        stream.InitializeFromMemory(bytes)?;
        let dec = f.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let mut best: Option<(u64, IWICBitmapFrameDecode)> = None;
        for i in 0..dec.GetFrameCount()?.min(64) {
            let frame = dec.GetFrame(i)?;
            let (mut w, mut h) = (0, 0);
            frame.GetSize(&mut w, &mut h)?;
            // stejná velikost → pozdější snímek (v ICO bývá 32bpp za 8bpp)
            if best.as_ref().is_none_or(|(a, _)| w as u64 * h as u64 >= *a) {
                best = Some((w as u64 * h as u64, frame));
            }
        }
        let (_, frame) = best.ok_or_else(|| Error::from(E_FAIL))?;
        encode(&f, &frame.cast()?, MAX_PX)
    }
}

/// Ikona souboru `.exe` (≤ [`MAX_PX`]) jako PNG; `None`, když soubor ikonu nemá.
pub fn exe_png(path: &Path) -> Option<Vec<u8>> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() || wide.len() >= 260 {
        return None;
    }
    let mut name = [0u16; 260];
    name[..wide.len()].copy_from_slice(&wide);
    let mut icons = [HICON::default(); 1];
    let mut id = 0u32;
    let px = MAX_PX as i32;
    let n = unsafe { PrivateExtractIconsW(&name, 0, px, px, Some(&mut icons), Some(&mut id), 0) };
    if n == 0 || n == u32::MAX || icons[0].0.is_null() {
        return None;
    }
    let hicon = icons[0];
    let png = {
        let _com = Com::init();
        factory().and_then(|f| unsafe { encode(&f, &f.CreateBitmapFromHICON(hicon)?.cast()?, MAX_PX) })
    };
    unsafe {
        let _ = DestroyIcon(hicon);
    }
    png.ok()
}

/// Ikona souboru / složky, jak ji ukazuje Průzkumník (rychlé akce): systémový seznam 48 px
/// (`SHIL_EXTRALARGE`), jinak velká 32 px. `None`, když shell ikonu nedá.
pub fn shell_png(path: &Path) -> Option<Vec<u8>> {
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Controls::{IImageList, ILD_TRANSPARENT};
    use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SYSICONINDEX, SHGetFileInfoW, SHGetImageList, SHIL_EXTRALARGE};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let _com = Com::init();
    let mut info = SHFILEINFOW::default();
    let size = std::mem::size_of::<SHFILEINFOW>() as u32;
    let flags = SHGFI_SYSICONINDEX | SHGFI_ICON | SHGFI_LARGEICON;
    let pcw = windows::core::PCWSTR(wide.as_ptr());
    if unsafe { SHGetFileInfoW(pcw, FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut info), size, flags) } == 0 {
        return None;
    }
    let big = unsafe { SHGetImageList::<IImageList>(SHIL_EXTRALARGE as i32).and_then(|l| l.GetIcon(info.iIcon, ILD_TRANSPARENT.0)) }.ok();
    let png = |h: HICON| factory().and_then(|f| unsafe { encode(&f, &f.CreateBitmapFromHICON(h)?.cast()?, MAX_PX) }).ok();
    let out = big.filter(|h| !h.0.is_null()).and_then(|h| {
        let r = png(h);
        unsafe {
            let _ = DestroyIcon(h);
        }
        r
    });
    let small = info.hIcon;
    let out = out.or_else(|| (!small.0.is_null()).then(|| png(small)).flatten());
    if !small.0.is_null() {
        unsafe {
            let _ = DestroyIcon(small);
        }
    }
    out
}

/// Zdroj → (zmenšení) → PNG 32bpp BGRA. Zmenšuje se v premultiplikované alfě (bez tmavých okrajů).
unsafe fn encode(f: &IWICImagingFactory, src: &IWICBitmapSource, max: u32) -> Result<Vec<u8>> {
    unsafe {
        let (mut w, mut h) = (0, 0);
        src.GetSize(&mut w, &mut h)?;
        let (tw, th) = fit(w, h, max);
        let src: IWICBitmapSource = if (tw, th) == (w, h) {
            src.clone()
        } else {
            let pre = convert(f, src, &GUID_WICPixelFormat32bppPBGRA)?;
            let scaler = f.CreateBitmapScaler()?;
            scaler.Initialize(&pre, tw, th, WICBitmapInterpolationModeFant)?;
            scaler.cast()?
        };
        let bgra = convert(f, &src, &GUID_WICPixelFormat32bppBGRA)?;
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        let enc = f.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let (mut frame, mut props) = (None, None);
        enc.CreateNewFrame(&mut frame, &mut props)?;
        let frame = frame.ok_or_else(|| Error::from(E_FAIL))?;
        frame.Initialize(props.as_ref())?;
        frame.SetSize(tw, th)?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut fmt)?;
        frame.WriteSource(&bgra, std::ptr::null())?;
        frame.Commit()?;
        enc.Commit()?;
        let mut len = 0u64;
        stream.Seek(0, STREAM_SEEK_END, Some(&mut len))?;
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        let mut buf = vec![0u8; len as usize];
        let mut read = 0u32;
        stream.Read(buf.as_mut_ptr().cast(), len as u32, Some(&mut read)).ok()?;
        buf.truncate(read as usize);
        Ok(buf)
    }
}

unsafe fn convert(f: &IWICImagingFactory, src: &IWICBitmapSource, fmt: &windows::core::GUID) -> Result<IWICBitmapSource> {
    unsafe {
        let conv = f.CreateFormatConverter()?;
        conv.Initialize(src, fmt, WICBitmapDitherTypeNone, None::<&IWICPalette>, 0.0, WICBitmapPaletteTypeCustom)?;
        conv.cast()
    }
}

/// Testy: PNG z holých BGRA pixelů (bez zmenšení).
#[cfg(test)]
pub fn png_from_bgra(w: u32, h: u32, px: &[u8]) -> Vec<u8> {
    let _com = Com::init();
    let f = factory().unwrap();
    unsafe {
        let bmp = f.CreateBitmapFromMemory(w, h, &GUID_WICPixelFormat32bppBGRA, w * 4, px).unwrap();
        encode(&f, &bmp.cast().unwrap(), u32::MAX).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::raster_size;

    #[test]
    fn fit_keeps_aspect_and_never_upscales() {
        for (w, h, want) in [(64, 64, (64, 64)), (128, 128, (128, 128)), (256, 256, (128, 128)), (300, 200, (128, 85)), (200, 300, (85, 128)), (4000, 10, (128, 1))] {
            assert_eq!(fit(w, h, MAX_PX), want, "{w}×{h}");
        }
    }

    fn solid(w: u32, h: u32) -> Vec<u8> {
        png_from_bgra(w, h, &[40, 120, 200, 255].repeat((w * h) as usize))
    }

    #[test]
    fn big_png_is_downscaled() {
        let big = solid(300, 200);
        assert_eq!(raster_size(&big), Some((300, 200)));
        let out = to_png(&big).unwrap();
        assert_eq!(raster_size(&out), Some((128, 85)));
    }

    /// ICO se dvěma PNG snímky (16 a 64 px) → vybere se větší.
    #[test]
    fn ico_picks_largest_frame() {
        let (a, b) = (solid(16, 16), solid(64, 64));
        let mut ico = vec![0, 0, 1, 0, 2, 0];
        let mut off = 6 + 2 * 16;
        for (px, data) in [(16u8, &a), (64u8, &b)] {
            ico.extend([px, px, 0, 0, 1, 0, 32, 0]);
            ico.extend((data.len() as u32).to_le_bytes());
            ico.extend((off as u32).to_le_bytes());
            off += data.len();
        }
        ico.extend_from_slice(&a);
        ico.extend_from_slice(&b);
        assert_eq!(raster_size(&ico), Some((64, 64)));
        assert_eq!(raster_size(&to_png(&ico).unwrap()), Some((64, 64)));
    }

    #[test]
    fn garbage_is_none() {
        assert!(to_png(b"<svg/>").is_none());
        assert!(to_png(&[]).is_none());
    }

    /// Jen čte ikonu systémového exe — nic nespouští, žádné okno.
    #[test]
    fn shell_icon_for_folder_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "x").unwrap();
        for p in [dir.path(), f.as_path()] {
            let png = shell_png(p).expect("ikona ze shellu");
            assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        }
    }

    #[test]
    fn exe_icon_from_explorer() {
        let png = exe_png(Path::new(r"C:\Windows\explorer.exe")).expect("explorer.exe má ikonu");
        let (w, h) = raster_size(&png).unwrap();
        assert!((64..=MAX_PX).contains(&w) && w == h, "{w}×{h}");
        assert!(exe_png(Path::new(r"C:\neexistuje\x.exe")).is_none());
    }
}
