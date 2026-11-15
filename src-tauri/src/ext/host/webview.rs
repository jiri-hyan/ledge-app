//! WebView2 pro okno rozšíření bez Tauri (raw `webview2-com`, vzor spike kola 33): vlastní prostředí s datovou
//! složkou rozšíření (kvůli měření v Jobu a vlastním argumentům), připojení k HWND, načtení HTML, zprávy
//! stránka ↔ Rust a uspání (`TrySuspend`) při pauze. Vše na vlákně okna (STA), čekání přes `win::pump_until`.

#![allow(unsafe_op_in_unsafe_fn)]

use super::win::pump_until;
use std::path::Path;
use std::sync::mpsc;
use webview2_com::CoreWebView2EnvironmentOptions;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows::core::{HSTRING, Interface, PCWSTR};

/// Argumenty prohlížeče rozšíření. Okno-dítě `WorkerW` by Chromium mohl pokládat za zakryté a utlumit
/// vykreslování (`CalculateNativeWinOcclusion`); zvuk / video bez gesta kvůli budoucí scéně.
pub const BROWSER_ARGS: &str = "--disable-features=CalculateNativeWinOcclusion --autoplay-policy=no-user-gesture-required";

const WAIT_MS: u64 = 20_000;

/// COM pro vlákno okna (STA). Chyba „už inicializováno“ nevadí.
pub fn init_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

/// Prostředí WebView2 s datovou složkou `dir` (vlastní proces prohlížeče rozšíření). `None` = runtime chybí
/// nebo složku nejde použít.
pub fn create_env(dir: &Path) -> Option<ICoreWebView2Environment> {
    let _ = std::fs::create_dir_all(dir);
    unsafe {
        let (tx, rx) = mpsc::channel();
        let done = webview2_com::CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |e, env| {
            e?;
            if let Some(env) = env {
                let _ = tx.send(env);
            }
            Ok(())
        }));
        let opts = CoreWebView2EnvironmentOptions::default();
        opts.set_additional_browser_arguments(BROWSER_ARGS.to_string());
        let opts: ICoreWebView2EnvironmentOptions = opts.into();
        CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), &HSTRING::from(dir.to_string_lossy().as_ref()), Some(&opts), &done).ok()?;
        pump_until(&rx, WAIT_MS)
    }
}

/// Jedna stránka v HWND.
pub struct Page {
    ctrl: ICoreWebView2Controller,
    core: ICoreWebView2,
}

impl Page {
    /// Připojí WebView2 k `hwnd`; zprávy ze stránky (`window.chrome.webview.postMessage`) přijdou jako JSON
    /// text do `on_msg`. Vrací až po `NavigationCompleted` nového dokumentu `html`.
    pub fn attach(env: &ICoreWebView2Environment, hwnd: HWND, html: &str, on_msg: impl FnMut(String) + 'static) -> Option<Page> {
        unsafe {
            let (tx, rx) = mpsc::channel();
            let done = webview2_com::CreateCoreWebView2ControllerCompletedHandler::create(Box::new(move |e, c| {
                e?;
                if let Some(c) = c {
                    let _ = tx.send(c);
                }
                Ok(())
            }));
            env.CreateCoreWebView2Controller(hwnd, &done).ok()?;
            let ctrl: ICoreWebView2Controller = pump_until(&rx, WAIT_MS)?;
            let core = ctrl.CoreWebView2().ok()?;
            if let Ok(s) = core.Settings() {
                let _ = s.SetAreDefaultContextMenusEnabled(false);
                let _ = s.SetIsZoomControlEnabled(false);
                let _ = s.SetIsStatusBarEnabled(false);
                let _ = s.SetIsBuiltInErrorPageEnabled(false);
            }
            let page = Page { ctrl, core };
            page.fit(hwnd);
            // statická scéna (překreslení 1× za minutu) nepotřebuje cache: trvale nízký cíl paměti
            page.set_memory_low();
            let _ = page.ctrl.SetIsVisible(true);
            let mut on_msg = on_msg;
            let h = webview2_com::WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut s = windows::core::PWSTR::null();
                args.WebMessageAsJson(&mut s)?;
                on_msg(webview2_com::take_pwstr(s));
                Ok(())
            }));
            let mut tok = 0i64;
            let _ = page.core.add_WebMessageReceived(&h, &mut tok);
            let (ntx, nrx) = mpsc::channel();
            let nh = webview2_com::NavigationCompletedEventHandler::create(Box::new(move |_, _| {
                let _ = ntx.send(());
                Ok(())
            }));
            let _ = page.core.add_NavigationCompleted(&nh, &mut tok);
            page.core.NavigateToString(&HSTRING::from(html)).ok()?;
            pump_until(&nrx, WAIT_MS)?;
            Some(page)
        }
    }

    /// Velikost WebView = klientská oblast okna.
    pub fn fit(&self, hwnd: HWND) {
        unsafe {
            let mut r = RECT::default();
            let _ = GetClientRect(hwnd, &mut r);
            let _ = self.ctrl.SetBounds(r);
        }
    }

    /// JSON text do stránky (`window.chrome.webview` událost `message`).
    pub fn post_json(&self, json: &str) {
        unsafe {
            let _ = self.core.PostWebMessageAsJson(&HSTRING::from(json));
        }
    }

    /// Trvale nízký cíl paměti (`MemoryUsageTargetLevel LOW`): WebView2 pustí cache, scéna se překresluje zřídka.
    pub fn set_memory_low(&self) {
        unsafe {
            if let Ok(c19) = self.core.cast::<ICoreWebView2_19>() {
                let _ = c19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
            }
        }
    }

    /// Pauza: stránka dostane `visibilitychange` a `TrySuspend`, zpět `Resume`. Nízký cíl paměti zůstává
    /// (`set_memory_low`), i po návratu z pauzy.
    pub fn set_paused(&self, paused: bool) {
        unsafe {
            self.set_memory_low();
            if let Ok(c3) = self.core.cast::<ICoreWebView2_3>() {
                if paused {
                    let _ = self.ctrl.SetIsVisible(false);
                    let h = webview2_com::TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                    let _ = c3.TrySuspend(&h);
                } else {
                    let _ = c3.Resume();
                    let _ = self.ctrl.SetIsVisible(true);
                }
            } else {
                let _ = self.ctrl.SetIsVisible(!paused);
            }
        }
    }

    pub fn close(self) {
        unsafe {
            let _ = self.ctrl.Close();
        }
    }
}
