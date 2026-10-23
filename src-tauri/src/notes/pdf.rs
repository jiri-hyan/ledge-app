//! Export poznámky do PDF (`note_export` s `pdf`): samostatné HTML z `export::export_html` vytiskne
//! WebView2 `ICoreWebView2_7::PrintToPdf` — žádná nová závislost (crate `webview2-com`, který Tauri
//! už tahá), žádný prohlížeč ani tiskárna.
//!
//! **Proč skryté okno navíc:** šuplík tisknout nejde — jeho WebView drží rozpracované UI (editor, peek)
//! a navigace pryč by ho zahodila. Proto jednorázové okno `pdf-export-<n>`, které se nikdy neukáže:
//! `visible(false)` (Windows ho nevykreslí, nemá místo v Alt+Tab ani na liště — navíc `skip_taskbar`),
//! `focused(false)` (wry pak nevolá `MoveFocus` → fokus zůstane, kde byl), bez rámečku. Sdílí argumenty
//! prohlížeče šuplíku (`drawer::BROWSER_ARGS`) → jeden proces WebView2 a stejná datová složka (jiné
//! argumenty by vytvoření prostředí shodily). Nemá žádnou capability (`capabilities/*.json` znají jen
//! `drawer` a `note-*`) a smí navigovat jen na `about:blank` a vlastní dočasný `file:` soubor.
//! Po tisku, chybě i termínu se okno zavře a dočasné HTML smaže (`Cleanup`).
//!
//! PDF se tiskne do `<cíl>.ledge-tmp` a až hotové se přejmenuje na cíl — rozbitý tisk nepřepíše
//! starý soubor. Termín `TIMEOUT`; každá chyba = lidská věta (`Fail::message`).
//! Čisté části (cesty, URL, label, texty chyb) mají tabulkové testy; samotný tisk potřebuje WebView2
//! a okno, takže se v testech nespouští (ověření naživo popisuje `docs/codemap/notes.md`).

use crate::i18n::tr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

/// Celý tisk (načtení stránky + PDF) nejdéle tak dlouho.
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// Label skrytého okna tisku (nesmí ladit s `drawer`, `note-*` ani `web-*`).
pub const LABEL_PREFIX: &str = "pdf-export-";
/// A4 v palcích (WebView2 bere palce) a okraj 1,5 cm.
pub const A4_W_IN: f64 = 8.27;
pub const A4_H_IN: f64 = 11.69;
pub const MARGIN_IN: f64 = 0.59;
/// Přípona rozpracovaného PDF vedle cíle.
const PARTIAL_EXT: &str = "ledge-tmp";

static SEQ: AtomicU64 = AtomicU64::new(1);

/// Proč PDF nevzniklo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fail {
    /// dočasné HTML nejde zapsat
    TempFile,
    /// skryté okno / WebView2 nejde vytvořit
    Window,
    /// WebView2 je starší a tisk do PDF nemá (ICoreWebView2_7)
    TooOld,
    /// stránka se nenačetla
    Load,
    /// tisk do PDF selhal (cesta, práva, místo na disku)
    Print,
    /// nestihlo se do `TIMEOUT`
    Timeout,
    /// hotové PDF nejde přesunout na cíl
    Move,
}

impl Fail {
    /// Lidská věta pro hlášku v UI (cs / en podle jazyka UI).
    pub fn message(self) -> String {
        let secs = TIMEOUT.as_secs();
        match self {
            Fail::TempFile => tr("PDF se nepodařilo připravit — dočasný soubor nejde zapsat.", "Couldn't prepare the PDF — the temporary file can't be written.").into(),
            Fail::Window => tr("PDF se nepodařilo připravit — WebView2 se nespustil. Zkus to znovu, případně ulož HTML.", "Couldn't prepare the PDF — WebView2 didn't start. Try again or save as HTML.").into(),
            Fail::TooOld => tr("Tisk do PDF potřebuje novější Microsoft Edge WebView2 Runtime. Aktualizuj ho, nebo ulož HTML a vytiskni ho v prohlížeči.", "Printing to PDF needs a newer Microsoft Edge WebView2 Runtime. Update it, or save as HTML and print it in a browser.").into(),
            Fail::Load => tr("PDF se nepodařilo vytvořit — poznámka se nenačetla k tisku.", "Couldn't create the PDF — the note didn't load for printing.").into(),
            Fail::Print => tr("PDF se nepodařilo uložit — zkontroluj, jestli do složky jde zapisovat a je tam místo.", "Couldn't save the PDF — check that the folder is writable and has free space.").into(),
            Fail::Timeout => trf!("PDF se nestihlo vytvořit do {secs} s — zkus to znovu.", "The PDF wasn't ready within {secs} s — try again."),
            Fail::Move => tr("PDF je hotové, ale nejde ho uložit na vybrané místo — není soubor otevřený jinde?", "The PDF is ready but can't be saved to the chosen place — is the file open elsewhere?").into(),
        }
    }
}

/// Label skrytého okna tisku.
pub fn label(n: u64) -> String {
    format!("{LABEL_PREFIX}{n}")
}

/// Dočasné HTML k tisku: `<temp>\ledge-pdf-<pid>-<n>.html` (pid + pořadí → souběžné exporty se nepotkají).
pub fn temp_html(dir: &Path, pid: u32, n: u64) -> PathBuf {
    dir.join(format!("ledge-pdf-{pid}-{n}.html"))
}

/// Rozpracované PDF vedle cíle (`a.pdf` → `a.pdf.ledge-tmp`) — stejná složka, takže přejmenování je atomické.
pub fn partial(target: &Path) -> PathBuf {
    let mut s = target.as_os_str().to_os_string();
    s.push(".");
    s.push(PARTIAL_EXT);
    PathBuf::from(s)
}

/// `file:` URL dočasného HTML (mezery, diakritika a `#` správně zakódované); relativní cesta = `None`.
pub fn file_url(path: &Path) -> Option<String> {
    tauri::Url::from_file_path(path).ok().map(String::from)
}

/// Smí skryté okno tisku navigovat sem? Jen prázdná stránka a vlastní dočasný soubor.
pub fn nav_allowed(url: &str, own: &str) -> bool {
    url == "about:blank" || url == own
}

/// Vytiskne `html` do PDF `target`. Blokuje (volat z async příkazu, ne z hlavního vlákna) nejdéle `TIMEOUT`.
pub fn print(app: &tauri::AppHandle, html: &str, target: &Path) -> Result<(), String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let html_path = temp_html(&std::env::temp_dir(), std::process::id(), n);
    let mut clean = Cleanup { html: Some(html_path.clone()), win: None, partial: None };
    std::fs::write(&html_path, html).map_err(|e| {
        crate::errors::detail("pdf: dočasné HTML", &e);
        Fail::TempFile.message()
    })?;
    let url = file_url(&html_path).ok_or_else(|| Fail::TempFile.message())?;
    let own = url.clone();
    let blank = tauri::Url::parse("about:blank").map_err(|_| Fail::Window.message())?;
    let w = WebviewWindowBuilder::new(app, label(n), WebviewUrl::External(blank))
        .title("Ledge PDF")
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .decorations(false)
        .additional_browser_args(crate::drawer::BROWSER_ARGS)
        .disable_drag_drop_handler()
        .on_navigation(move |u| nav_allowed(u.as_str(), &own))
        .build()
        .map_err(|e| {
            crate::errors::detail("pdf: okno", &e);
            Fail::Window.message()
        })?;
    clean.win = Some(w.clone());
    let out = partial(target);
    clean.partial = Some(out.clone());
    let (tx, rx) = mpsc::channel::<Result<(), Fail>>();
    let out_s = out.to_string_lossy().into_owned();
    let tx0 = tx.clone();
    w.with_webview(move |wv| {
        if let Err(f) = unsafe { start(&wv, &url, &out_s, tx0.clone()) } {
            let _ = tx0.send(Err(f));
        }
    })
    .map_err(|_| Fail::Window.message())?;
    drop(tx);
    let res = rx.recv_timeout(TIMEOUT).unwrap_or(Err(Fail::Timeout));
    res.map_err(Fail::message)?;
    std::fs::rename(&out, target).map_err(|e| {
        crate::errors::detail("pdf: přesun", &e);
        Fail::Move.message()
    })?;
    clean.partial = None;
    Ok(())
}

/// Úklid po tisku (i po chybě a termínu): zavřít okno, smazat dočasné HTML a nedokončené PDF.
struct Cleanup {
    html: Option<PathBuf>,
    win: Option<tauri::WebviewWindow>,
    partial: Option<PathBuf>,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(w) = self.win.take() {
            let _ = w.destroy();
        }
        for p in [self.html.take(), self.partial.take()].into_iter().flatten() {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Hlavní vlákno (`with_webview`): nastavení tisku, po načtení vlastní stránky `PrintToPdf` → `tx`.
unsafe fn start(wv: &tauri::webview::PlatformWebview, url: &str, out: &str, tx: mpsc::Sender<Result<(), Fail>>) -> Result<(), Fail> {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use windows::core::{BOOL, HSTRING, Interface, PWSTR};
    unsafe {
        let core = wv.controller().CoreWebView2().map_err(|_| Fail::Window)?;
        let c7 = core.cast::<ICoreWebView2_7>().map_err(|_| Fail::TooOld)?;
        // A4, okraje, pozadí (zaškrtnuté úkoly), bez hlavičky a patičky (cesta k dočasnému souboru)
        let settings = core
            .cast::<ICoreWebView2_2>()
            .ok()
            .and_then(|c| c.Environment().ok())
            .and_then(|e| e.cast::<ICoreWebView2Environment6>().ok())
            .and_then(|e| e.CreatePrintSettings().ok());
        if let Some(s) = &settings {
            let _ = s.SetPageWidth(A4_W_IN);
            let _ = s.SetPageHeight(A4_H_IN);
            for f in [ICoreWebView2PrintSettings::SetMarginTop, ICoreWebView2PrintSettings::SetMarginBottom, ICoreWebView2PrintSettings::SetMarginLeft, ICoreWebView2PrintSettings::SetMarginRight] {
                let _ = f(s, MARGIN_IN);
            }
            let _ = s.SetShouldPrintBackgrounds(true);
            let _ = s.SetShouldPrintHeaderAndFooter(false);
        }
        let own = url.to_string();
        let out = HSTRING::from(out);
        let mut done = false;
        let handler = webview2_com::NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
            let Some(sender) = sender.filter(|_| !done) else { return Ok(()) };
            let mut src = PWSTR::null();
            sender.Source(&mut src)?;
            // úvodní about:blank od Tauri přeskočit — čeká se na vlastní soubor
            if webview2_com::take_pwstr(src) != own {
                return Ok(());
            }
            done = true;
            let mut ok = BOOL(0);
            if let Some(a) = &args {
                a.IsSuccess(&mut ok)?;
            }
            if !ok.as_bool() {
                let _ = tx.send(Err(Fail::Load));
                return Ok(());
            }
            let tx2 = tx.clone();
            let printed = webview2_com::PrintToPdfCompletedHandler::create(Box::new(move |res, ok| {
                let _ = tx2.send(if res.is_ok() && ok { Ok(()) } else { Err(Fail::Print) });
                Ok(())
            }));
            if c7.PrintToPdf(&out, settings.as_ref(), &printed).is_err() {
                let _ = tx.send(Err(Fail::Print));
            }
            Ok(())
        }));
        let mut token = 0i64;
        core.add_NavigationCompleted(&handler, &mut token).map_err(|_| Fail::Window)?;
        core.Navigate(&HSTRING::from(url)).map_err(|_| Fail::Load)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_urls() {
        let dir = Path::new("C:\\Temp");
        assert_eq!(temp_html(dir, 42, 7), PathBuf::from("C:\\Temp\\ledge-pdf-42-7.html"));
        assert_ne!(temp_html(dir, 42, 7), temp_html(dir, 42, 8), "souběžné exporty");
        assert_ne!(temp_html(dir, 42, 7), temp_html(dir, 43, 7), "dva Ledge");
        let cases: &[(&str, &str)] = &[
            ("C:\\x\\a.pdf", "C:\\x\\a.pdf.ledge-tmp"),
            ("C:\\x y\\Zápis.PDF", "C:\\x y\\Zápis.PDF.ledge-tmp"),
            ("a.pdf", "a.pdf.ledge-tmp"),
        ];
        for (t, want) in cases {
            let p = partial(Path::new(t));
            assert_eq!(p, PathBuf::from(want), "{t}");
            assert_eq!(p.parent(), Path::new(t).parent(), "{t}: stejná složka = atomické přejmenování");
        }
        // file: URL: mezery, diakritika a # zakódované; relativní cesta nic
        let u = file_url(Path::new("C:\\Temp\\ledge pdf #1 čž.html")).unwrap();
        assert!(u.starts_with("file:///C:/Temp/ledge%20pdf%20%231%20"), "{u}");
        assert!(!u.contains(' ') && !u.contains('#') && u.ends_with(".html"), "{u}");
        assert_eq!(file_url(Path::new("relativni.html")), None);
    }

    #[test]
    fn hidden_window_is_locked_down() {
        let own = "file:///C:/Temp/ledge-pdf-1-1.html";
        let cases = [("about:blank", true), (own, true), ("file:///C:/Temp/jiny.html", false), ("https://example.com/", false), ("http://tauri.localhost/", false)];
        for (u, want) in cases {
            assert_eq!(nav_allowed(u, own), want, "{u}");
        }
        // label nepotká šuplík, lístky ani okna s webem (ty mají capability / vlastní pravidla)
        let l = label(3);
        assert_eq!(l, "pdf-export-3");
        assert!(l != crate::drawer::LABEL && !l.starts_with(crate::pins::LABEL_PREFIX) && !l.starts_with(crate::webpanel::LABEL_PREFIX));
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
        for e in std::fs::read_dir(dir).unwrap() {
            let text = std::fs::read_to_string(e.unwrap().path()).unwrap();
            assert!(!text.contains("pdf-export") && !text.contains("\"*\""), "capability nesmí dosáhnout na okno tisku");
        }
        // A4 na výšku, okraj ~1,5 cm
        assert!(A4_H_IN > A4_W_IN && (A4_W_IN * 2.54 - 21.0).abs() < 0.05 && (MARGIN_IN * 2.54 - 1.5).abs() < 0.05);
    }

    #[test]
    fn failures_are_human_sentences() {
        let all = [Fail::TempFile, Fail::Window, Fail::TooOld, Fail::Load, Fail::Print, Fail::Timeout, Fail::Move];
        for f in all {
            let cs = f.message();
            let en = crate::i18n::with_lang(crate::uiconf::Lang::En, || f.message());
            for m in [&cs, &en] {
                assert!((m.ends_with('.') || m.ends_with('?')) && m.chars().next().is_some_and(char::is_uppercase), "{f:?}: {m}");
                assert!(!m.contains("HRESULT") && !m.contains("0x") && !m.contains("Error"), "{f:?}: žádný technický žargon: {m}");
            }
            assert_ne!(cs, en, "{f:?}");
        }
        assert!(Fail::Timeout.message().contains(&format!("{} s", TIMEOUT.as_secs())));
    }
}
