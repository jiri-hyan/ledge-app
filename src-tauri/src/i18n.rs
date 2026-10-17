//! Jazyk textů z jádra (hlášky, stavy ve snapshotu, nativní proužek). Jazyk je globální (`ui.lang`
//! z configu: `set_lang` při startu a v `settings::apply_config`), výchozí čeština. Text se píše na
//! místě v obou jazycích: `tr("česky", "English")`, s proměnnými `trf!("Port {p}", "Port {p}")`.
//! Testy: jazyk je per vlákno (`with_lang`), aby se paralelní testy neovlivňovaly; výchozí čeština.

use crate::uiconf::Lang;
#[cfg(not(test))]
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(not(test))]
static LANG: AtomicU8 = AtomicU8::new(0);

const fn code(l: Lang) -> u8 {
    match l {
        Lang::Cs => 0,
        Lang::En => 1,
    }
}
const fn from_code(c: u8) -> Lang {
    if c == 1 { Lang::En } else { Lang::Cs }
}

#[cfg(test)]
thread_local! {
    static LOCAL: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Aktuální jazyk textů jádra.
pub(crate) fn lang() -> Lang {
    #[cfg(not(test))]
    {
        from_code(LANG.load(Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        LOCAL.with(|c| from_code(c.get()))
    }
}

/// Nastaví jazyk (z `ui.lang`). V testech jen pro aktuální vlákno.
pub(crate) fn set_lang(l: Lang) {
    #[cfg(not(test))]
    LANG.store(code(l), Ordering::Relaxed);
    #[cfg(test)]
    LOCAL.with(|c| c.set(code(l)));
}

/// Text podle jazyka: čeština, nebo angličtina.
pub(crate) fn tr(cs: &'static str, en: &'static str) -> &'static str {
    match lang() {
        Lang::En => en,
        Lang::Cs => cs,
    }
}

/// `format!` podle jazyka: `trf!("Port {p} drží {who}", "Port {p} is held by {who}")`. Obě šablony
/// musí použít stejné argumenty (kontroluje překladač).
macro_rules! trf {
    ($cs:literal, $en:literal $(, $($arg:tt)*)?) => {
        match $crate::i18n::lang() {
            $crate::uiconf::Lang::En => format!($en $(, $($arg)*)?),
            $crate::uiconf::Lang::Cs => format!($cs $(, $($arg)*)?),
        }
    };
}

/// Spustí `f` s jazykem `l` (jen testy, aktuální vlákno) a vrátí výchozí.
#[cfg(test)]
pub(crate) fn with_lang<R>(l: Lang, f: impl FnOnce() -> R) -> R {
    let before = lang();
    set_lang(l);
    let r = f();
    set_lang(before);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_czech_and_en_switches() {
        assert_eq!(lang(), Lang::Cs);
        assert_eq!(tr("Ahoj", "Hi"), "Ahoj");
        with_lang(Lang::En, || {
            assert_eq!(tr("Ahoj", "Hi"), "Hi");
            let n = 3;
            assert_eq!(trf!("{n}× pád", "{n}× crashed"), "3× crashed");
            assert_eq!(trf!("kód {}", "code {}", 7), "code 7");
        });
        assert_eq!(lang(), Lang::Cs, "with_lang vrátí předchozí jazyk");
        let p = 5173;
        assert_eq!(trf!("Port {p}", "Port {p}!"), "Port 5173");
    }

    #[test]
    fn codes_roundtrip() {
        for l in [Lang::Cs, Lang::En] {
            assert_eq!(from_code(code(l)), l);
        }
        assert_eq!(from_code(9), Lang::Cs);
    }
}
