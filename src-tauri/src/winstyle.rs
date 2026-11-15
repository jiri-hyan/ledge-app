//! Jediné místo pro styly oken „nekrade fokus“: `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW`. Tabulka `Win` +
//! `profile` = co má které okno Ledge při vzniku (invariant „každé okno má profil“, test níže). Proužek, šuplík,
//! přihrádka, plocha (desk) a lístky (pin) vznikají bez aktivace; šuplík a lístek smí po VÝSLOVNÉM kliku přijmout
//! klávesnici (`no_activate(h, false)`), lístek ji po ztrátě fokusu zase pustí. Okna, která vznikají aktivní
//! záměrně (webpanel, Studio, PDF náhled), jsou `Win::UserOpened`.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WINDOW_EX_STYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Okna Ledge s vlastním HWND a jejich profil stylu. Tabulka pro test a dokumentaci (varianty bez volání v produkci
/// hlídá `scripts/window-focus.test.mjs`).
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Win {
    Strip,
    Drawer,
    ShelfDrop,
    Desk,
    Pin,
    /// Okna otevřená výslovnou akcí uživatele (webpanel, Studio, PDF náhled): smí být aktivní od začátku.
    UserOpened,
}

/// Žádoucí `WS_EX_*` (NOACTIVATE + TOOLWINDOW, nebo nic) okna při vzniku / po ztrátě fokusu. Čistá funkce.
pub fn profile(w: Win) -> u32 {
    match w {
        Win::Strip | Win::Drawer | Win::ShelfDrop | Win::Desk | Win::Pin => WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0,
        Win::UserOpened => 0,
    }
}

/// Rozšířený styl při vzniku okna (`CreateWindowExW`): bez aktivace a mimo Alt+Tab / panel úloh.
pub fn no_activate_ex() -> WINDOW_EX_STYLE {
    WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
}

/// Nová hodnota `GWL_EXSTYLE`: přidat `add`, odebrat `remove` (odebrání vyhrává). Čistá funkce.
pub fn ex_with(ex: isize, add: u32, remove: u32) -> isize {
    ((ex as u32 | add) & !remove) as isize
}

/// Nasadí profil `w` na hotové okno (okna z tao: styl se přidá hned po `build`).
pub fn apply_profile(hwnd: HWND, w: Win) {
    let add = profile(w);
    if add != 0 {
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_with(ex, add, 0));
        }
    }
}

/// `on` = okno nikdy nebere fokus (TOOLWINDOW + NOACTIVATE); `off` = odebrat jen NOACTIVATE (výslovný klik / zkratka
/// chce psát; TOOLWINDOW zůstane, okno se nepřidá do Alt+Tab).
pub fn no_activate(hwnd: HWND, on: bool) {
    let (add, remove) = if on { (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0, 0) } else { (0, WS_EX_NOACTIVATE.0) };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_with(ex, add, remove));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ex_with_table() {
        let (na, tw) = (WS_EX_NOACTIVATE.0, WS_EX_TOOLWINDOW.0);
        // (původní styl, přidat, odebrat, čekaný styl)
        let cases = [
            (0x8_0000u32, na | tw, 0, 0x8_0000 | na | tw), // vrstvené okno + zapnout
            (na | tw, 0, na, tw),                         // odebrat jen NOACTIVATE, TOOLWINDOW zůstane
            (tw, 0, na, tw),                              // odebrat, co tam není: beze změny
            (na | tw, na | tw, 0, na | tw),               // zapnout dvakrát = jednou
            (0, 0, 0, 0),
        ];
        for (ex, add, remove, want) in cases {
            assert_eq!(ex_with(ex as isize, add, remove), want as isize, "{ex:#x} +{add:#x} -{remove:#x}");
        }
        assert_eq!(no_activate_ex(), WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE);
    }

    #[test]
    fn profile_table() {
        let (na, tw) = (WS_EX_NOACTIVATE.0, WS_EX_TOOLWINDOW.0);
        // okno → čekané ex-styly (každé okno Ledge musí mít řádek)
        let cases = [
            (Win::Strip, na | tw),
            (Win::Drawer, na | tw),
            (Win::ShelfDrop, na | tw),
            (Win::Desk, na | tw),
            (Win::Pin, na | tw),
            (Win::UserOpened, 0),
        ];
        for (w, want) in cases {
            assert_eq!(profile(w), want, "{w:?}");
        }
        assert_eq!(profile(Win::Strip), no_activate_ex().0); // jediný zdroj
        assert_eq!(ex_with(0x8_0000, profile(Win::Pin), 0), (0x8_0000 | na | tw) as isize); // ostatní příznaky zůstanou
    }
}
