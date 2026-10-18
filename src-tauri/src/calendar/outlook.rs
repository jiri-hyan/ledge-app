//! Zdroj `outlook-local`: výchozí kalendář **klasického Outlooku** v tomto PC přes COM (bez přihlášení,
//! bez sítě, jen čtení). Nový Outlook (WebView, `olk.exe`) COM nemá — tam jen odkaz ICS.
//!
//! - **Detekce bez spuštění:** `installed` = `CLSIDFromProgID("Outlook.Application")` (jen čtení
//!   registru, nic nespouští); `running` = proces `OUTLOOK.EXE` (snímek procesů).
//! - **Spuštění:** `CoCreateInstance` při zavřeném Outlooku spustí `OUTLOOK.EXE -Embedding` na pozadí
//!   (bez hlavního okna; s profilem „vždy se ptát“ ale vyskočí výběr profilu). Proto automatické
//!   načtení (`allow_start = false`) běží **jen když Outlook už běží** (připojí se k běžící instanci, žádné
//!   okno, žádný fokus); se zavřeným Outlookem jen po výslovném kliku „Načíst z Outlooku“. Outlook
//!   spuštěný automatizací se po uvolnění posledního odkazu obvykle sám zavře (neověřeno na všech verzích).
//! - **Ochrana objektového modelu** (Outlook „Program se pokouší získat přístup…“): čte se jen to, co
//!   ochrana nehlídá (Subject, Start/End, StartUTC/EndUTC, AllDayEvent, Location, MeetingStatus). Text
//!   pozvánky (`Body`, z něj odkaz Teams/Zoom) jen když `Application.IsTrusted` = true (aktuální antivir
//!   → Outlook výzvu neukáže); jinak se `Body` vůbec nečte a odkaz Připojit bude jen z místa.
//! - Okno: `Items.Sort("[Start]")`, `IncludeRecurrences`, `Restrict` na −1/+14 dní (data v místním
//!   krátkém formátu — tak je Outlook čte), procházení `GetFirst`/`GetNext` s mezí `MAX_OCCS` a časem
//!   `BUDGET`. Převod položek je čistá funkce `to_occ` (testy); COM vrstva jen čte hodnoty.

use super::meeting::{self, RANK_DESCRIPTION, RANK_LOCATION};
use super::source::{Event, Window};
use super::tz::{LocalTz, SystemLocal};
use super::{MAX_LOCATION, MAX_OCCS, MAX_TITLE, Occ, clip, ms, naive};
use crate::i18n::tr;
use crate::state::log;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

/// Nejdelší čtení z Outlooku (pak se vezme, co je).
const BUDGET: std::time::Duration = std::time::Duration::from_secs(8);
/// Z textu pozvánky se prohledá nejvýš tolik znaků (patička Teams je na začátku / konci).
const MAX_BODY: usize = 64 * 1024;
/// `MeetingStatus`: olMeetingCanceled, olMeetingReceivedAndCanceled
const CANCELLED: [i32; 2] = [5, 7];
/// `olFolderCalendar`
const FOLDER_CALENDAR: i32 = 9;

// ---------- čistý převod ----------

fn ole_base() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1899, 12, 30).unwrap_or_default().and_time(NaiveTime::MIN)
}

/// Datum OLE (dny od 30. 12. 1899, zlomek = čas) → čas bez pásma, na celé sekundy. Záporné / mimo
/// rozsah (rok 9999) = `None`.
pub fn naive_from_ole(d: f64) -> Option<NaiveDateTime> {
    (d.is_finite() && (0.0..2_958_466.0).contains(&d)).then(|| ole_base() + Duration::seconds((d * 86_400.0).round() as i64))
}

pub fn ole_from_naive(t: NaiveDateTime) -> f64 {
    (t - ole_base()).num_seconds() as f64 / 86_400.0
}

/// Hodnoty jedné položky kalendáře, jak je COM vrátí.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawItem {
    pub subject: String,
    pub all_day: bool,
    /// místní čas (OLE)
    pub start: f64,
    pub end: f64,
    /// `StartUTC` / `EndUTC` (Outlook 2007+); `None` = vlastnost chybí
    pub start_utc: Option<f64>,
    pub end_utc: Option<f64>,
    pub location: String,
    pub meeting_status: i32,
    /// text pozvánky — jen při `IsTrusted`, jinak prázdný
    pub body: String,
}

/// Položka → výskyt (stejný tvar jako z ICS). Zrušená / bez času = `None`.
pub fn to_occ(r: &RawItem, local: &dyn LocalTz) -> Option<Occ> {
    if CANCELLED.contains(&r.meeting_status) {
        return None;
    }
    let mut join = None;
    meeting::offer(&mut join, RANK_LOCATION, &r.location);
    if !r.body.is_empty() {
        let cut = r.body.char_indices().nth(MAX_BODY).map_or(r.body.len(), |(i, _)| i);
        meeting::offer(&mut join, RANK_DESCRIPTION, &r.body[..cut]);
    }
    let title = clip(&r.subject, MAX_TITLE);
    let location = clip(r.location.lines().find(|l| !l.trim().is_empty()).unwrap_or(""), MAX_LOCATION);
    let join = join.map(|(_, j)| j);
    if r.all_day {
        // celodenní: místní data (půlnoc jako UTC, stejně jako ICS), konec výlučně
        let s = naive_from_ole(r.start)?.date();
        let e = naive_from_ole(r.end).map(|t| t.date()).filter(|e| *e > s).unwrap_or(s + Duration::days(1));
        let mid = |d: NaiveDate| ms(d.and_time(NaiveTime::MIN));
        return Some(Occ { start: mid(s), end: mid(e), all_day: true, title, location, tz_guess: false, join });
    }
    let utc = |u: Option<f64>, l: f64| -> Option<NaiveDateTime> {
        u.and_then(naive_from_ole).or_else(|| naive_from_ole(l).map(|t| t - Duration::seconds(local.offset_local(t) as i64)))
    };
    let s = utc(r.start_utc, r.start)?;
    let e = utc(r.end_utc, r.end).filter(|e| *e >= s).unwrap_or(s);
    Some(Occ { start: ms(s), end: ms(e), all_day: false, title, location, tz_guess: false, join })
}

/// Filtr `Items.Restrict`: položky, které okno aspoň částečně překrývají (data v místním formátu).
pub fn restrict_filter(from_local: &str, to_local: &str) -> String {
    format!("[Start] <= '{to_local}' AND [End] >= '{from_local}'")
}

// ---------- texty ----------

pub fn missing() -> &'static str {
    tr(
        "Klasický Outlook v tomto PC není. Nový Outlook jiným aplikacím kalendář nepůjčí — použij odkaz ICS (Outlook na webu → Nastavení → Kalendář → Sdílené kalendáře).",
        "Classic Outlook isn't installed on this PC. The new Outlook doesn't share its calendar with other apps — use an ICS link (Outlook on the web → Settings → Calendar → Shared calendars).",
    )
}

pub fn closed() -> &'static str {
    tr("Outlook je zavřený — otevři ho, nebo klikni na Načíst z Outlooku.", "Outlook is closed — open it, or click Load from Outlook.")
}

fn com_error(e: &windows::core::Error) -> String {
    use windows::Win32::Foundation::{RPC_E_CALL_REJECTED, RPC_E_SERVERCALL_RETRYLATER};
    log(&format!("kalendář: Outlook COM {:#010x}", e.code().0 as u32));
    if e.code() == RPC_E_CALL_REJECTED || e.code() == RPC_E_SERVERCALL_RETRYLATER {
        return tr("Outlook je zaneprázdněný (má otevřený dialog?) — zkusím to později.", "Outlook is busy (is a dialog open?) — I'll try again later.").into();
    }
    tr("Outlook neodpověděl — zkus to znovu, nebo použij odkaz ICS.", "Outlook didn't answer — try again, or use an ICS link.").into()
}

// ---------- Windows: detekce ----------

fn clsid() -> Option<windows::core::GUID> {
    unsafe { windows::Win32::System::Com::CLSIDFromProgID(windows::core::w!("Outlook.Application")) }.ok()
}

/// Je klasický Outlook nainstalovaný? (jen registr, nic nespouští)
pub fn installed() -> bool {
    clsid().is_some()
}

/// Běží `OUTLOOK.EXE`?
pub fn running() -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return false };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snap, &mut e).is_ok();
        let mut found = false;
        while ok && !found {
            let n = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            found = String::from_utf16_lossy(&e.szExeFile[..n]).eq_ignore_ascii_case("outlook.exe");
            ok = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
        found
    }
}

// ---------- Windows: tenká vrstva IDispatch ----------

mod com {
    use std::mem::ManuallyDrop;
    use windows::Win32::Foundation::VARIANT_BOOL;
    use windows::Win32::System::Com::{DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS, IDispatch};
    use windows::Win32::System::Ole::{DISPID_PROPERTYPUT, VARFORMAT_NAMED_FORMAT, VarFormatDateTime};
    use windows::Win32::System::Variant::{VARENUM, VARIANT, VT_BOOL, VT_BSTR, VT_DATE, VT_DISPATCH, VT_I4, VariantClear, VariantToDouble, VariantToInt32};
    use windows::core::{BSTR, GUID, PCWSTR, Result};

    const LOCALE_USER_DEFAULT: u32 = 0x0400;

    /// VARIANT, který se po sobě uklidí (`VariantClear`).
    #[repr(transparent)]
    pub struct Var(VARIANT);

    impl Drop for Var {
        fn drop(&mut self) {
            unsafe {
                let _ = VariantClear(&mut self.0);
            }
        }
    }

    impl Var {
        fn with(vt: VARENUM, set: impl FnOnce(&mut windows::Win32::System::Variant::VARIANT_0_0_0)) -> Var {
            let mut v = VARIANT::default();
            unsafe {
                let inner = &mut *v.Anonymous.Anonymous;
                inner.vt = vt;
                set(&mut inner.Anonymous);
            }
            Var(v)
        }
        pub fn str(s: &str) -> Var {
            Var::with(VT_BSTR, |a| a.bstrVal = ManuallyDrop::new(BSTR::from(s)))
        }
        pub fn i4(n: i32) -> Var {
            Var::with(VT_I4, |a| a.lVal = n)
        }
        pub fn bool(b: bool) -> Var {
            Var::with(VT_BOOL, |a| a.boolVal = VARIANT_BOOL(if b { -1 } else { 0 }))
        }
        pub fn date(d: f64) -> Var {
            Var::with(VT_DATE, |a| a.date = d)
        }
        fn vt(&self) -> VARENUM {
            unsafe { self.0.Anonymous.Anonymous.vt }
        }
        pub fn text(&self) -> String {
            if self.vt() == VT_BSTR { unsafe { self.0.Anonymous.Anonymous.Anonymous.bstrVal.to_string() } } else { String::new() }
        }
        pub fn f64(&self) -> Option<f64> {
            if self.vt() == VT_DATE { Some(unsafe { self.0.Anonymous.Anonymous.Anonymous.date }) } else { unsafe { VariantToDouble(&self.0) }.ok() }
        }
        pub fn i32(&self) -> Option<i32> {
            unsafe { VariantToInt32(&self.0) }.ok()
        }
        pub fn truthy(&self) -> bool {
            self.vt() == VT_BOOL && unsafe { self.0.Anonymous.Anonymous.Anonymous.boolVal.0 } != 0
        }
        pub fn disp(&self) -> Option<IDispatch> {
            if self.vt() == VT_DISPATCH { unsafe { (*self.0.Anonymous.Anonymous.Anonymous.pdispVal).clone() } } else { None }
        }
        /// datum v místním krátkém formátu (`VarFormatDateTime`, jazyk uživatele — jak čte Outlook)
        pub fn format(&self, f: i32) -> String {
            unsafe { VarFormatDateTime(&self.0, VARFORMAT_NAMED_FORMAT(f), 0) }.map(|b| b.to_string()).unwrap_or_default()
        }
    }

    pub struct Disp(pub IDispatch);

    impl Disp {
        fn invoke(&self, name: &str, flags: DISPATCH_FLAGS, mut args: Vec<Var>) -> Result<Var> {
            let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
            let names = [PCWSTR(wide.as_ptr())];
            let mut id = 0i32;
            unsafe { self.0.GetIDsOfNames(&GUID::zeroed(), names.as_ptr(), 1, LOCALE_USER_DEFAULT, &mut id)? };
            // argumenty v DISPPARAMS jdou pozpátku
            args.reverse();
            let mut put = DISPID_PROPERTYPUT;
            let is_put = flags == DISPATCH_PROPERTYPUT;
            let params = DISPPARAMS {
                rgvarg: args.as_mut_ptr() as *mut VARIANT,
                rgdispidNamedArgs: if is_put { &mut put as *mut i32 } else { std::ptr::null_mut() },
                cArgs: args.len() as u32,
                cNamedArgs: u32::from(is_put),
            };
            let mut out = Var(VARIANT::default());
            unsafe { self.0.Invoke(id, &GUID::zeroed(), LOCALE_USER_DEFAULT, flags, &params, Some(&mut out.0 as *mut VARIANT), None, None)? };
            Ok(out)
        }
        pub fn get(&self, name: &str) -> Result<Var> {
            self.invoke(name, DISPATCH_PROPERTYGET, vec![])
        }
        pub fn call(&self, name: &str, args: Vec<Var>) -> Result<Var> {
            self.invoke(name, DISPATCH_METHOD | DISPATCH_PROPERTYGET, args)
        }
        pub fn put(&self, name: &str, v: Var) -> Result<()> {
            self.invoke(name, DISPATCH_PROPERTYPUT, vec![v]).map(|_| ())
        }
        /// objekt z vlastnosti / metody
        pub fn obj(&self, name: &str, args: Vec<Var>) -> Result<Disp> {
            self.call(name, args)?.disp().map(Disp).ok_or_else(|| windows::Win32::Foundation::E_NOINTERFACE.into())
        }
    }
}

/// COM na tomto vlákně (STA); uvolní jen to, co samo zapnulo.
struct ComInit(bool);

impl ComInit {
    fn new() -> Self {
        use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx};
        ComInit(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.is_ok())
    }
}

impl Drop for ComInit {
    fn drop(&mut self) {
        if self.0 {
            unsafe { windows::Win32::System::Com::CoUninitialize() }
        }
    }
}

/// Jedna položka přes COM (chyba vlastnosti = výchozí hodnota; bez začátku položku přeskočí volající).
fn read_item(it: &com::Disp, trusted: bool) -> RawItem {
    let f = |n: &str| it.get(n).ok().and_then(|v| v.f64());
    let s = |n: &str| it.get(n).map(|v| v.text()).unwrap_or_default();
    RawItem {
        subject: s("Subject"),
        all_day: it.get("AllDayEvent").is_ok_and(|v| v.truthy()),
        start: f("Start").unwrap_or(-1.0),
        end: f("End").unwrap_or(-1.0),
        start_utc: f("StartUTC"),
        end_utc: f("EndUTC"),
        location: s("Location"),
        meeting_status: it.get("MeetingStatus").ok().and_then(|v| v.i32()).unwrap_or(0),
        // Body hlídá ochrana objektového modelu — jen když Outlook volajícímu věří (bez výzvy)
        body: if trusted { s("Body") } else { String::new() },
    }
}

fn read(w: Window, local: &dyn LocalTz) -> windows::core::Result<Vec<Event>> {
    use com::{Disp, Var};
    use windows::Win32::System::Com::{CLSCTX_LOCAL_SERVER, CoCreateInstance, IDispatch};
    let Some(clsid) = clsid() else { return Ok(Vec::new()) };
    let app: IDispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER)? };
    let app = Disp(app);
    let trusted = app.get("IsTrusted").is_ok_and(|v| v.truthy());
    let ns = app.obj("GetNamespace", vec![Var::str("MAPI")])?;
    let items = ns.obj("GetDefaultFolder", vec![Var::i4(FOLDER_CALENDAR)])?.obj("Items", vec![])?;
    items.call("Sort", vec![Var::str("[Start]")])?;
    items.put("IncludeRecurrences", Var::bool(true))?;
    // okno v místním čase, krátký formát data a času podle jazyka uživatele (2 = datum, 4 = čas)
    let local_ole = |utc_ms: i64| {
        let t = naive(utc_ms);
        Var::date(ole_from_naive(t + Duration::seconds(local.offset_utc(t) as i64)))
    };
    let fmt = |v: Var| format!("{} {}", v.format(2), v.format(4));
    let filter = restrict_filter(&fmt(local_ole(w.from)), &fmt(local_ole(w.to)));
    let found = items.obj("Restrict", vec![Var::str(&filter)])?;
    let began = std::time::Instant::now();
    let mut out = Vec::new();
    let mut cur = found.call("GetFirst", vec![])?;
    while let Some(it) = cur.disp() {
        if let Some(o) = to_occ(&read_item(&Disp(it), trusted), local) {
            if o.start >= w.to {
                break; // seřazeno podle začátku — dál už jen pozdější
            }
            out.push(o);
        }
        if out.len() >= MAX_OCCS || began.elapsed() > BUDGET {
            log(&format!("kalendář: Outlook čtení utnuto ({} položek)", out.len()));
            break;
        }
        cur = found.call("GetNext", vec![])?;
    }
    Ok(out)
}

/// Výskyty z výchozího kalendáře Outlooku v okně `w`. `allow_start` = výslovný klik (smí Outlook
/// spustit na pozadí); jinak se zavřeným Outlookem chyba `closed` a nic se nespouští.
pub fn events(w: Window, allow_start: bool) -> Result<Vec<Event>, String> {
    if !installed() {
        return Err(missing().into());
    }
    if !allow_start && !running() {
        return Err(closed().into());
    }
    let _com = ComInit::new();
    read(w, &SystemLocal).map_err(|e| com_error(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::meeting::Provider;
    use crate::calendar::tz::builtin;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, 0).unwrap()
    }

    #[test]
    fn ole_dates_roundtrip() {
        let cases = [(0.0, Some(dt(1899, 12, 30, 0, 0))), (2.0, Some(dt(1900, 1, 1, 0, 0))), (46_313.375, Some(dt(2026, 10, 18, 9, 0))), (-1.0, None), (f64::NAN, None), (3e6, None)];
        for (d, want) in cases {
            assert_eq!(naive_from_ole(d), want, "{d}");
        }
        for t in [dt(2026, 10, 18, 9, 15), dt(2026, 3, 29, 2, 30), dt(2030, 1, 1, 23, 59)] {
            assert_eq!(naive_from_ole(ole_from_naive(t)), Some(t));
        }
    }

    #[test]
    fn items_to_occs_table() {
        let tz = builtin("Europe/Prague").unwrap();
        let ole = |t: NaiveDateTime| ole_from_naive(t);
        let base = RawItem { subject: "Porada".into(), start: ole(dt(2026, 10, 19, 9, 0)), end: ole(dt(2026, 10, 19, 10, 0)), start_utc: Some(ole(dt(2026, 10, 19, 7, 0))), end_utc: Some(ole(dt(2026, 10, 19, 8, 0))), ..RawItem::default() };
        let teams = "https://teams.microsoft.com/l/meetup-join/19%3ameeting_X%40thread.v2/0?context=%7b%7d";
        // (název, položka, (začátek UTC, konec UTC, celodenní, název, místo, služba))
        type Want = Option<(NaiveDateTime, NaiveDateTime, bool, &'static str, &'static str, Option<Provider>)>;
        let cases: Vec<(&str, RawItem, Want)> = vec![
            ("časová: StartUTC", base.clone(), Some((dt(2026, 10, 19, 7, 0), dt(2026, 10, 19, 8, 0), false, "Porada", "", None))),
            ("bez StartUTC: místní čas přes pásmo", RawItem { start_utc: None, end_utc: None, ..base.clone() }, Some((dt(2026, 10, 19, 7, 0), dt(2026, 10, 19, 8, 0), false, "Porada", "", None))),
            ("zimní čas bez StartUTC", RawItem { start: ole(dt(2026, 12, 1, 9, 0)), end: ole(dt(2026, 12, 1, 9, 30)), start_utc: None, end_utc: None, ..base.clone() }, Some((dt(2026, 12, 1, 8, 0), dt(2026, 12, 1, 8, 30), false, "Porada", "", None))),
            (
                "celodenní: data, konec výlučně",
                RawItem { all_day: true, start: ole(dt(2026, 10, 20, 0, 0)), end: ole(dt(2026, 10, 22, 0, 0)), subject: "Dovolená".into(), ..base.clone() },
                Some((dt(2026, 10, 20, 0, 0), dt(2026, 10, 22, 0, 0), true, "Dovolená", "", None)),
            ),
            ("celodenní bez konce = jeden den", RawItem { all_day: true, start: ole(dt(2026, 10, 20, 0, 0)), end: -1.0, ..base.clone() }, Some((dt(2026, 10, 20, 0, 0), dt(2026, 10, 21, 0, 0), true, "Porada", "", None))),
            ("zrušená", RawItem { meeting_status: 5, ..base.clone() }, None),
            ("zrušená přijatá", RawItem { meeting_status: 7, ..base.clone() }, None),
            ("bez začátku", RawItem { start: -1.0, start_utc: None, ..base.clone() }, None),
            ("konec před začátkem = nulová", RawItem { end_utc: Some(ole(dt(2026, 10, 19, 6, 0))), ..base.clone() }, Some((dt(2026, 10, 19, 7, 0), dt(2026, 10, 19, 7, 0), false, "Porada", "", None))),
            (
                "Teams: místo bez odkazu, odkaz v textu pozvánky",
                RawItem { location: "Microsoft Teams Meeting".into(), body: format!("Microsoft Teams meeting\r\nClick here to join the meeting<{teams}>\r\n"), ..base.clone() },
                Some((dt(2026, 10, 19, 7, 0), dt(2026, 10, 19, 8, 0), false, "Porada", "Microsoft Teams Meeting", Some(Provider::Teams))),
            ),
            (
                "Zoom v místě vyhrává nad popisem",
                RawItem { location: "https://firma.zoom.us/j/123".into(), body: format!("<{teams}>"), ..base.clone() },
                Some((dt(2026, 10, 19, 7, 0), dt(2026, 10, 19, 8, 0), false, "Porada", "https://firma.zoom.us/j/123", Some(Provider::Zoom))),
            ),
            (
                "víceřádkový předmět a místo",
                RawItem { subject: "Porada\r\n  týmu".into(), location: "\r\nZasedačka 2\r\npatro 3".into(), ..base.clone() },
                Some((dt(2026, 10, 19, 7, 0), dt(2026, 10, 19, 8, 0), false, "Porada týmu", "Zasedačka 2", None)),
            ),
        ];
        for (name, raw, want) in cases {
            let got = to_occ(&raw, &tz);
            let got = got.as_ref().map(|o| (naive(o.start), naive(o.end), o.all_day, o.title.as_str(), o.location.as_str(), o.join.as_ref().map(|j| j.provider)));
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn filter_shape() {
        assert_eq!(restrict_filter("17.10.2026 10:00", "01.11.2026 10:00"), "[Start] <= '01.11.2026 10:00' AND [End] >= '17.10.2026 10:00'");
        let en = |f: fn() -> &'static str| crate::i18n::with_lang(crate::uiconf::Lang::En, f);
        for f in [missing as fn() -> &'static str, closed] {
            assert!(!f().is_empty() && f() != en(f), "{}", f());
        }
    }
}
