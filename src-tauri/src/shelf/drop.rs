//! Cíl přetažení přihrádky: malé neviditelné okno u hrany (vlákno `shelf-drop`, OLE `IDropTarget`).
//!
//! Proužek je click-through (`WS_EX_TRANSPARENT`) — přetažení by propadlo do okna pod ním. Proto
//! zvláštní okno: vrstvené s krytím 1/255 (neviditelné, ale zasažitelné), bez aktivace, nad vším,
//! a **ukázané jen během tahu**: vlákno `hover` (`shelf::Arm`) pozná, že kurzor vjel do zóny proužku
//! s drženým tlačítkem a že jde nejspíš o OLE tah (`ole_drag_likely`), a pošle `set_armed(Some(zóna))`;
//! po puštění / odjezdu `set_armed(None)` okno schová. V klidu okno neexistuje / je skryté a vlákno
//! spí v `GetMessageW` — žádný časovač, žádné buzení. Vlákno vznikne líně při prvním tahu.
//!
//! Cíl bere jen `CF_HDROP` (soubory z Průzkumníka) a vrací `DROPEFFECT_LINK` / `COPY` — nikdy `MOVE`
//! (zdroj by soubor po „přesunu“ smazal). Ukládají se jen cesty (`shelf::add_paths`).
//!
//! Stejné okno slouží i druhé zóně (kolo 20, `smart.dropShortcut`): nad OTEVŘENÝM šuplíkem (`Kind::Shortcut`,
//! `shortcut.rs`) bere navíc odkaz a text s cestou a puštěné jen NAVRHNE jako událost `drop-shortcut`
//! (UI otevře editor akce, člověk potvrdí). Zóna se vybírá stavem šuplíku, ne polohou — nepřekrývají se.

use super::shortcut::{self, Kind};
use super::{DONE_MS, Hint};
use crate::state::{core, log};
use parking_lot::Mutex;
use std::cell::Cell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU8, Ordering};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, POINTL, RECT, S_OK, WPARAM};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl, OleInitialize,
    RegisterDragDrop, ReleaseStgMedium,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{CLSID_DragDropHelper, DragQueryFileW, HDROP, IDropTargetHelper};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{Ref, implement, w};

const WM_ARM: u32 = WM_APP + 21;
/// pojistka: okno se schová i bez `DragLeave`/`Drop` (zdroj tah zrušil jinak)
const HIDE_TIMER: usize = 1;
const HIDE_GRACE_MS: u32 = 800;
const DONE_TIMER: usize = 2;

/// Účinek pro zdroj podle toho, co nabízí: odkaz, jinak kopie; přesun NIKDY.
pub fn pick_effect(allowed: u32) -> u32 {
    if allowed & DROPEFFECT_LINK.0 != 0 {
        DROPEFFECT_LINK.0
    } else if allowed & DROPEFFECT_COPY.0 != 0 {
        DROPEFFECT_COPY.0
    } else {
        DROPEFFECT_NONE.0
    }
}

struct Win {
    hwnd: AtomicIsize,
    /// zóna, kde má okno být (`None` = schovat)
    rect: Mutex<Option<RECT>>,
}

static WIN: OnceLock<Win> = OnceLock::new();
static LAST: Mutex<Option<(RECT, Kind)>> = Mutex::new(None);
/// zóna, která je právě aktivní (0 = přihrádka, 1 = nový zástupce); čte ji cíl při `DragEnter` / `Drop`
static KIND: AtomicU8 = AtomicU8::new(0);
/// tah je právě nad oknem (mezi `DragEnter` a `DragLeave`/`Drop`)
static OVER: AtomicBool = AtomicBool::new(false);

fn win() -> &'static Win {
    let mut fresh = false;
    let w = WIN.get_or_init(|| {
        fresh = true;
        Win { hwnd: AtomicIsize::new(0), rect: Mutex::new(None) }
    });
    // vlákno až po uložení `WIN` (sahá na něj)
    if fresh {
        std::thread::Builder::new().name("shelf-drop".into()).spawn(|| unsafe { run() }).ok();
    }
    w
}

/// Vlákno `hover`: aktivní zóna (`Some`) / konec tahu (`None`). Posílá jen změnu.
pub fn set_armed(z: Option<(RECT, Kind)>) {
    {
        let mut last = LAST.lock();
        if *last == z {
            return;
        }
        *last = z;
    }
    if z.is_none() && WIN.get().is_none() {
        return;
    }
    if let Some((_, k)) = z {
        KIND.store(k as u8, Ordering::Release);
    }
    let w = win();
    *w.rect.lock() = z.map(|(r, _)| r);
    post(w);
}

fn kind() -> Kind {
    if KIND.load(Ordering::Acquire) == Kind::Shortcut as u8 { Kind::Shortcut } else { Kind::Shelf }
}

fn post(w: &Win) {
    let h = w.hwnd.load(Ordering::Acquire);
    if h != 0 {
        unsafe {
            let _ = PostMessageW(Some(HWND(h as *mut _)), WM_ARM, WPARAM(0), LPARAM(0));
        }
    }
}

/// Je to nejspíš OLE tah (soubory), ne posun okna / tah tabu / výběr textu? Volá se jen s kurzorem
/// v zóně a drženým tlačítkem. Posun okna = `GUI_INMOVESIZE` vlákna v popředí; jinak kurzor: OLE tah
/// má vlastní kurzory (kopie / odkaz / zákaz), běžný tah standardní šipku, I-beam, ruku…
pub fn ole_drag_likely() -> bool {
    unsafe {
        let mut gui = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        if GetGUIThreadInfo(0, &mut gui).is_ok() && gui.flags.0 & GUI_INMOVESIZE.0 != 0 {
            return false;
        }
        let mut ci = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
        if GetCursorInfo(&mut ci).is_err() || ci.hCursor.0.is_null() {
            return false;
        }
        !standard_cursors().contains(&(ci.hCursor.0 as isize))
    }
}

/// Sdílené systémové kurzory (handly jsou stálé po celý běh) — jednou.
fn standard_cursors() -> &'static [isize] {
    static C: OnceLock<Vec<isize>> = OnceLock::new();
    C.get_or_init(|| {
        let ids = [IDC_ARROW, IDC_IBEAM, IDC_HAND, IDC_SIZEALL, IDC_SIZENS, IDC_SIZEWE, IDC_SIZENWSE, IDC_SIZENESW, IDC_CROSS, IDC_WAIT, IDC_APPSTARTING, IDC_UPARROW];
        ids.iter().filter_map(|id| unsafe { LoadCursorW(None, *id) }.ok().map(|c| c.0 as isize)).collect()
    })
}

fn hint(h: Hint) {
    core().strip.set(|m| m.shelf = h);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn hide(hwnd: HWND) {
    unsafe {
        let _ = KillTimer(Some(hwnd), HIDE_TIMER);
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

unsafe fn run() {
    unsafe {
        if let Err(e) = OleInitialize(None) {
            log(&format!("shelf: OleInitialize {e}"));
            return;
        }
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("LedgeShelfDrop");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        let ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
        let Ok(hwnd) = CreateWindowExW(ex, class, w!("Ledge shelf"), WS_POPUP, 0, 0, 1, 1, None, None, Some(hinst.into()), None) else {
            return;
        };
        // krytí 1/255: oko nic nevidí, hit-test (a tím OLE) okno najde
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA);
        let helper: Option<IDropTargetHelper> = CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER).ok();
        let target: IDropTarget = Target { hwnd, helper, accept: Cell::new(false) }.into();
        if let Err(e) = RegisterDragDrop(hwnd, &target) {
            log(&format!("shelf: RegisterDragDrop {e}"));
            return;
        }
        let w = WIN.get().expect("WIN");
        w.hwnd.store(hwnd.0 as isize, Ordering::Release);
        // požadavek mohl přijít dřív, než okno vzniklo
        post(w);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            // jen zprávy našeho okna (OLE má ve vlákně vlastní skrytá okna a časovače)
            let ours = msg.hwnd == hwnd;
            match (msg.message, msg.wParam.0) {
                (WM_ARM, _) if ours => match *w.rect.lock() {
                    Some(r) => {
                        let _ = KillTimer(Some(hwnd), HIDE_TIMER);
                        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                    }
                    // tah nad oknem ještě běží (Drop dorazí za chvíli) → schovat až po něm, s pojistkou
                    None if OVER.load(Ordering::Acquire) => {
                        SetTimer(Some(hwnd), HIDE_TIMER, HIDE_GRACE_MS, None);
                    }
                    None => hide(hwnd),
                },
                (WM_TIMER, HIDE_TIMER) if ours => {
                    hide(hwnd);
                    if OVER.swap(false, Ordering::AcqRel) {
                        hint(Hint::None);
                    }
                }
                (WM_TIMER, DONE_TIMER) if ours => {
                    let _ = KillTimer(Some(hwnd), DONE_TIMER);
                    core().strip.set(|m| {
                        if matches!(m.shelf, Hint::Done(_)) {
                            m.shelf = Hint::None;
                        }
                    });
                }
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
}

#[implement(IDropTarget)]
struct Target {
    hwnd: HWND,
    helper: Option<IDropTargetHelper>,
    /// nabízený obsah má soubory
    accept: Cell<bool>,
}

fn hdrop_format() -> FORMATETC {
    FORMATETC { cfFormat: CF_HDROP.0, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }
}

fn format(cf: u16) -> FORMATETC {
    FORMATETC { cfFormat: cf, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }
}

/// Registrovaný formát odkazu z prohlížeče (`UniformResourceLocatorW`, text UTF-16 zakončený nulou).
fn url_format() -> u16 {
    static F: OnceLock<u16> = OnceLock::new();
    *F.get_or_init(|| unsafe { RegisterClipboardFormatW(w!("UniformResourceLocatorW")) } as u16)
}

/// Strop počtu UTF-16 znaků, které smíme přečíst z bloku paměti o `size_bytes` bajtech (kolik zdroj skutečně
/// přidělil, ne kolik si myslíme): nejvýš 4096 a nikdy za konec bloku (zdroj nemusí text ukončit nulou).
fn text_cap(size_bytes: usize) -> usize {
    (size_bytes / 2).min(4096)
}

/// Odkaz nebo text z přetahovaných dat (nejdřív odkaz z prohlížeče, pak běžný text), strop 4096 znaků.
unsafe fn dropped_text(obj: &IDataObject) -> Option<String> {
    unsafe {
        for cf in [url_format(), CF_UNICODETEXT.0] {
            if cf == 0 {
                continue;
            }
            let fmt = format(cf);
            if obj.QueryGetData(&fmt) != S_OK {
                continue;
            }
            let Ok(mut medium) = obj.GetData(&fmt) else { continue };
            // jen paměťové médium (HGLOBAL); jiné (stream, soubor…) by `hGlobal` vykládalo špatně
            if medium.tymed != TYMED_HGLOBAL.0 as u32 {
                ReleaseStgMedium(&mut medium);
                continue;
            }
            let h = medium.u.hGlobal;
            let cap = text_cap(GlobalSize(h));
            let p = GlobalLock(h) as *const u16;
            let text = if p.is_null() || cap == 0 {
                None
            } else {
                let mut n = 0;
                while n < cap && *p.add(n) != 0 {
                    n += 1;
                }
                Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)))
            };
            let _ = GlobalUnlock(h);
            ReleaseStgMedium(&mut medium);
            if text.as_deref().is_some_and(|t| !t.trim().is_empty()) {
                return text;
            }
        }
        None
    }
}

/// Nabízí obsah něco, co zóna `k` bere? Přihrádka jen soubory, nový zástupce i odkaz / text.
unsafe fn offers(obj: &IDataObject, k: Kind) -> bool {
    unsafe {
        let files = obj.QueryGetData(&hdrop_format()) == S_OK;
        match k {
            Kind::Shelf => files,
            Kind::Shortcut => files || [url_format(), CF_UNICODETEXT.0].iter().any(|cf| *cf != 0 && obj.QueryGetData(&format(*cf)) == S_OK),
        }
    }
}

/// Cesty z `CF_HDROP`.
unsafe fn files(obj: &IDataObject) -> Vec<String> {
    unsafe {
        let Ok(mut medium) = obj.GetData(&hdrop_format()) else { return vec![] };
        let h = HDROP(medium.u.hGlobal.0);
        let n = DragQueryFileW(h, u32::MAX, None);
        let mut out = Vec::new();
        for i in 0..n.min(super::MAX_ITEMS as u32 * 4) {
            let len = DragQueryFileW(h, i, None) as usize;
            let mut buf = vec![0u16; len + 1];
            let got = DragQueryFileW(h, i, Some(&mut buf)) as usize;
            if got > 0 {
                out.push(String::from_utf16_lossy(&buf[..got]));
            }
        }
        ReleaseStgMedium(&mut medium);
        out
    }
}

fn pt(p: &POINTL) -> POINT {
    POINT { x: p.x, y: p.y }
}

impl Target {
    unsafe fn effect(&self, pdweffect: *mut DROPEFFECT) -> DROPEFFECT {
        unsafe {
            let e = if self.accept.get() && !pdweffect.is_null() { DROPEFFECT(pick_effect((*pdweffect).0)) } else { DROPEFFECT_NONE };
            if !pdweffect.is_null() {
                *pdweffect = e;
            }
            e
        }
    }
}

impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(&self, pdataobj: Ref<IDataObject>, _keys: MODIFIERKEYS_FLAGS, p: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        unsafe {
            let obj = pdataobj.ok()?;
            let k = kind();
            self.accept.set(offers(obj, k));
            let e = self.effect(pdweffect);
            if let Some(h) = &self.helper {
                let _ = h.DragEnter(self.hwnd, obj, &pt(p), e);
            }
            let _ = KillTimer(Some(self.hwnd), HIDE_TIMER);
            if self.accept.get() {
                OVER.store(true, Ordering::Release);
                if k == Kind::Shelf {
                    hint(Hint::Over);
                }
            }
        }
        Ok(())
    }

    fn DragOver(&self, _keys: MODIFIERKEYS_FLAGS, p: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        unsafe {
            let e = self.effect(pdweffect);
            if let Some(h) = &self.helper {
                let _ = h.DragOver(&pt(p), e);
            }
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        unsafe {
            if let Some(h) = &self.helper {
                let _ = h.DragLeave();
            }
            if OVER.swap(false, Ordering::AcqRel) && kind() == Kind::Shelf {
                hint(Hint::None);
            }
            if WIN.get().is_some_and(|w| w.rect.lock().is_none()) {
                hide(self.hwnd);
            }
        }
        Ok(())
    }

    fn Drop(&self, pdataobj: Ref<IDataObject>, _keys: MODIFIERKEYS_FLAGS, p: &POINTL, pdweffect: *mut DROPEFFECT) -> windows::core::Result<()> {
        unsafe {
            let e = self.effect(pdweffect);
            let obj = pdataobj.ok()?;
            if let Some(h) = &self.helper {
                let _ = h.Drop(obj, &pt(p), e);
            }
            OVER.store(false, Ordering::Release);
            hide(self.hwnd);
            if kind() == Kind::Shortcut {
                // otevřený šuplík: puštěné jen NAVRHNEME (editor akce, člověk potvrdí) — nic se neukládá ani nespouští
                if self.accept.get() {
                    propose(&files(obj), dropped_text(obj).as_deref());
                }
                return Ok(());
            }
            let paths = if self.accept.get() { files(obj) } else { vec![] };
            match (!paths.is_empty()).then(|| super::add_paths(&paths)) {
                Some(Ok(a)) => {
                    log(&format!("shelf: puštěno {} (nové {}, posunuté {}, odpadlo {})", paths.len(), a.new, a.moved, a.dropped));
                    hint(Hint::Done(a.new + a.moved));
                    SetTimer(Some(self.hwnd), DONE_TIMER, DONE_MS, None);
                }
                Some(Err(err)) => {
                    log(&format!("shelf: {err}"));
                    hint(Hint::None);
                }
                None => hint(Hint::None),
            }
        }
        Ok(())
    }
}

/// Návrh zástupce z puštěných dat → událost `drop-shortcut` pro okno šuplíku (UI otevře editor akce).
fn propose(files: &[String], text: Option<&str>) {
    use tauri::Emitter;
    let is_dir = |p: &str| std::fs::metadata(p).is_ok_and(|m| m.is_dir());
    let p = match shortcut::judge(files, text, &is_dir) {
        shortcut::Verdict::Propose(p) => p,
        shortcut::Verdict::Refuse(why) => {
            log("zástupce z puštěného: typ souboru akce neotevře");
            let _ = core().app.emit_to(crate::drawer::LABEL, "drop-refused", why);
            return;
        }
        shortcut::Verdict::Nothing => {
            log("zástupce z puštěného: nic použitelného");
            return;
        }
    };
    log(&format!("zástupce z puštěného: {} (další přeskočeno {})", p.kind, p.skipped));
    let _ = core().app.emit_to(crate::drawer::LABEL, "drop-shortcut", p);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_cap_never_passes_block_end() {
        // (velikost bloku v bajtech, strop znaků)
        for (size, want) in [(0, 0), (1, 0), (2, 1), (7, 3), (8192, 4096), (8193, 4096), (1 << 20, 4096)] {
            assert_eq!(text_cap(size), want, "{size}");
        }
    }

    #[test]
    fn effect_never_move() {
        let move_ = windows::Win32::System::Ole::DROPEFFECT_MOVE.0;
        let (c, l, n) = (DROPEFFECT_COPY.0, DROPEFFECT_LINK.0, DROPEFFECT_NONE.0);
        let cases = [(c | move_ | l, l), (c | move_, c), (move_, n), (l, l), (c, c), (n, n)];
        for (allowed, want) in cases {
            assert_eq!(pick_effect(allowed), want, "{allowed:#x}");
            assert_eq!(pick_effect(allowed) & move_, 0);
        }
    }
}
