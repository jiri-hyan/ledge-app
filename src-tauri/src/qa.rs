//! Izolovaná instance pro QA (L295): `LEDGE_ISOLATED=1` pustí druhý Ledge vedle nainstalovaného a nic nezapíše
//! mimo jeho složky; `LEDGE_CDP_PORT=<port>` (jen s izolací) otevře WebView2 pro CDP; `GET /status` dostane blok `qa`.
//! Env se čte jednou (`OnceLock`), bez env se chování nemění ani o bajt. Okna se s portem parkují mimo plochu.

use crate::drawer::BROWSER_ARGS;
use serde_json::{Value, json};
use std::sync::OnceLock;

/// Posun oken izolované instance s CDP mimo viditelnou plochu (bez fokusu a bez překrývání práce).
const PARK_DX: i32 = -30000;

/// `LEDGE_ISOLATED` platí jen přesně jako `1` (čistá).
pub(crate) fn isolated_from(v: Option<&str>) -> bool {
    v == Some("1")
}

#[cfg(test)]
thread_local! { static FORCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// Běží tahle instance izolovaně? Env se čte jednou za běh.
pub(crate) fn isolated() -> bool {
    #[cfg(test)]
    if FORCE.with(|f| f.get()) {
        return true;
    }
    static I: OnceLock<bool> = OnceLock::new();
    *I.get_or_init(|| isolated_from(std::env::var("LEDGE_ISOLATED").ok().as_deref()))
}

/// Test: kód v `f` vidí izolovanou instanci (jen toto vlákno).
#[cfg(test)]
pub(crate) fn with_isolated<R>(f: impl FnOnce() -> R) -> R {
    FORCE.with(|c| c.set(true));
    let r = f();
    FORCE.with(|c| c.set(false));
    r
}

/// Port CDP: jen s izolací a v rozsahu 1024–65535, jinak `None` (čistá).
pub(crate) fn cdp_port_from(isolated: bool, v: Option<&str>) -> Option<u16> {
    if !isolated {
        return None;
    }
    v?.trim().parse::<u16>().ok().filter(|p| *p >= 1024)
}

/// Argumenty WebView2 (čistá): bez CDP přesně `BROWSER_ARGS`. S CDP přibude port a vypne se okluze
/// (okno mimo plochu by jinak Chromium uspal) — vše do jednoho `--disable-features`, další by přepsalo první.
pub(crate) fn browser_args_for(isolated: bool, cdp: Option<u16>) -> String {
    let Some(port) = cdp_port_from(isolated, cdp.map(|p| p.to_string()).as_deref()) else { return BROWSER_ARGS.to_string() };
    let base = BROWSER_ARGS.replace("msSmartScreenProtection ", "msSmartScreenProtection,CalculateNativeWinOcclusion ");
    format!("{base} --disable-renderer-backgrounding --disable-backgrounding-occluded-windows --remote-debugging-port={port}")
}

/// Argumenty pro všechna okna (musí být stejné, jinak okno nad sdíleným data direm nevznikne).
pub(crate) fn browser_args() -> &'static str {
    if !isolated() {
        return BROWSER_ARGS;
    }
    static A: OnceLock<String> = OnceLock::new();
    A.get_or_init(|| {
        let port = std::env::var("LEDGE_CDP_PORT").ok().and_then(|v| cdp_port_from(true, Some(&v)));
        browser_args_for(true, port)
    })
}

/// Je okno parkované? Jen izolace s portem CDP (bez portu okna zůstávají jako vždy).
fn parked() -> bool {
    isolated() && browser_args().contains("--remote-debugging-port=")
}

/// `x` okna: s izolací a CDP mimo plochu, jinak beze změny.
pub(crate) fn park_x(x: i32) -> i32 {
    if parked() { x + PARK_DX } else { x }
}

/// Vlastní složka dat WebView2 izolované instance s CDP (`<LEDGE_CONFIG_DIR>/webview2`). Výchozí složka se bere ze
/// systémové known-folder cesty (ne z env), takže by instance sdílela data (a proces prohlížeče) s nainstalovaným Ledge
/// a Chromium nad výchozí složkou `--remote-debugging-port` ignoruje. Stejná na všech oknech (jinak okno nevznikne).
pub(crate) fn data_dir() -> Option<std::path::PathBuf> {
    if !parked() {
        return None;
    }
    std::env::var_os("LEDGE_CONFIG_DIR").map(|d| std::path::PathBuf::from(d).join("webview2"))
}

/// `.qa_data_dir()` v řetězu stavitele okna: s izolací a CDP nastaví `data_dir()`, jinak nic nemění.
pub(crate) trait QaDataDir {
    fn qa_data_dir(self) -> Self;
}
impl<'a, R: tauri::Runtime, M: tauri::Manager<R>> QaDataDir for tauri::WebviewWindowBuilder<'a, R, M> {
    fn qa_data_dir(self) -> Self {
        match data_dir() {
            Some(d) => self.data_directory(d),
            None => self,
        }
    }
}

/// Schvalování akcí Clauda je v izolované instanci mrtvé (povolit smí jen člověk; CDP umí klikat).
pub(crate) fn approvals_blocked() -> bool {
    isolated()
}

/// Zaregistrované zkratky: nastavené bez těch, které systém odmítl (čistá).
pub(crate) fn registered(reg: &[(&'static str, String)], taken: &[(&'static str, String)]) -> Vec<Value> {
    reg.iter().filter(|h| !taken.contains(h)).map(|(a, k)| json!({ "action": a, "key": k })).collect()
}

/// Blok `qa` pro `GET /status` (jen povolená pole; skládá se při dotazu, žádný polling).
pub(crate) fn status() -> Value {
    let c = crate::state::core();
    let (pins_open, pins_through) = crate::pins::win::counts();
    let (live, playing) = crate::media::qa_flags();
    json!({
        "isolated": isolated(),
        "hotkeys": registered(&c.hotkeys.lock(), &crate::hotkeys::taken()),
        "pins": { "open": pins_open, "through": pins_through },
        "desk": { "open": crate::desk::win::shown_labels().len() },
        "down": crate::guard::down_names(),
        "cfgError": c.cfg_error.lock().is_some(),
        "media": { "live": live, "playing": playing },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_only_exact_one() {
        for (v, want) in [(None, false), (Some(""), false), (Some("0"), false), (Some("true"), false), (Some("1"), true)] {
            assert_eq!(isolated_from(v), want, "{v:?}");
        }
    }

    #[test]
    fn cdp_port_table() {
        for (iso, v, want) in [
            (false, Some("9222"), None),
            (true, None, None),
            (true, Some("9222"), Some(9222)),
            (true, Some(" 9222 "), Some(9222)),
            (true, Some("1023"), None),
            (true, Some("1024"), Some(1024)),
            (true, Some("65535"), Some(65535)),
            (true, Some("65536"), None),
            (true, Some("0"), None),
            (true, Some("x"), None),
            (true, Some("-1"), None),
        ] {
            assert_eq!(cdp_port_from(iso, v), want, "{iso} {v:?}");
        }
    }

    #[test]
    fn browser_args_table() {
        // bez env = dnešní řetězec bajt po bajtu; port bez izolace / mimo rozsah se ignoruje
        assert_eq!(browser_args_for(false, None), BROWSER_ARGS);
        assert_eq!(browser_args_for(false, Some(9222)), BROWSER_ARGS);
        assert_eq!(browser_args_for(true, None), BROWSER_ARGS);
        assert_eq!(browser_args_for(true, Some(80)), BROWSER_ARGS);
        let a = browser_args_for(true, Some(9222));
        assert!(a.ends_with("--remote-debugging-port=9222"), "{a}");
        assert!(a.starts_with("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion "), "{a}");
        assert_eq!(a.matches("--disable-features=").count(), 1, "druhé --disable-features by přepsalo první");
        assert!(a.contains("--renderer-process-limit=1"));
    }

    #[test]
    fn park_only_with_isolation_and_port() {
        // test běží bez env: nic se neparkuje
        assert_eq!(park_x(100), 100);
        assert!(!approvals_blocked());
        with_isolated(|| assert!(approvals_blocked()));
        assert!(!approvals_blocked());
    }

    #[test]
    fn data_dir_only_with_isolation_and_port() {
        // test běží bez env: bez izolace a portu se složka dat nemění (výchozí chování WebView2)
        assert_eq!(data_dir(), None);
    }

    #[test]
    fn registered_skips_taken() {
        let reg = vec![("open", "Ctrl+Alt+L".to_string()), ("approve", "Ctrl+Alt+Y".to_string())];
        let taken = vec![("approve", "Ctrl+Alt+Y".to_string())];
        assert_eq!(registered(&reg, &taken), vec![json!({"action": "open", "key": "Ctrl+Alt+L"})]);
        assert_eq!(registered(&[], &taken), Vec::<Value>::new());
    }

    /// Tvar bloku `status.qa` nese jen povolená pole (žádné cesty, texty, příkazy).
    #[test]
    fn status_has_only_allowed_keys() {
        let src = include_str!("qa.rs");
        let body = &src[src.find("pub(crate) fn status()").unwrap()..src.rfind("#[cfg(test)]").unwrap()];
        for k in ["isolated", "hotkeys", "pins", "desk", "down", "cfgError", "media"] {
            assert!(body.contains(&format!("\"{k}\"")), "{k}");
        }
        for bad in ["path", "cmd", "cwd", "title"] {
            assert!(!body.contains(&format!("\"{bad}")), "{bad}");
        }
    }
}
