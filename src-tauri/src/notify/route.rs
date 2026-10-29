//! Kam s oznámením (čistá): Notch, toast, nebo nic. Brána hry / „nerušit“ / otevřeného šuplíku je
//! `attention::signal` (dnešní logika) — sem přichází jako `allowed`.

use super::conf::Surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Notch,
    Toast,
    Nothing,
}

#[derive(Debug, Clone, Copy)]
pub struct Ctx {
    /// brána `attention::signal` prošla (není hra, Windows oznámení přijímá, šuplík není otevřený naplno)
    pub allowed: bool,
    pub surface: Surface,
    pub fallback_toast: bool,
    /// proužek smí vyjet (zapnutý nebo s barvou pozornosti, šuplík přes něj není, Ledge není mrtvý)
    pub stage: bool,
    /// kurzor je na monitoru proužku (jinak by si Notch nikdo nevšiml)
    pub same_monitor: bool,
    /// totéž hned ukáže peek šuplíku (Ledge drží žádost / otázku Clauda → `refresh::on_claude_change`
    /// otevře peek na místě proužku); Notch by ho jen zakryl
    pub covered: bool,
}

pub fn route(c: Ctx) -> Route {
    if !c.allowed {
        return Route::Nothing;
    }
    match c.surface {
        Surface::Toast => Route::Toast,
        Surface::Notch if c.covered && c.same_monitor => Route::Nothing,
        Surface::Notch if c.stage && c.same_monitor => Route::Notch,
        Surface::Notch if c.fallback_toast => Route::Toast,
        Surface::Notch => Route::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Route::{Nothing, Toast as T};
    use Surface::{Notch as N, Toast as ST};

    #[test]
    fn route_table() {
        // (brána, plocha, záložní toast, proužek může vyjet, stejný monitor) → cesta
        let cases = [
            (true, N, true, true, true, Route::Notch),
            (true, N, true, false, true, T),
            (true, N, true, true, false, T),
            (true, N, false, false, true, Nothing),
            (true, N, false, true, false, Nothing),
            (true, N, false, true, true, Route::Notch),
            (true, ST, false, true, true, T),
            (true, ST, true, false, false, T),
            // hra / nerušit / šuplík naplno: nic, ani Notch, ani toast
            (false, N, true, true, true, Nothing),
            (false, ST, true, true, true, Nothing),
        ];
        for (allowed, surface, fallback_toast, stage, same_monitor, want) in cases {
            let c = Ctx { allowed, surface, fallback_toast, stage, same_monitor, covered: false };
            assert_eq!(route(c), want, "{c:?}");
        }
        // peek ukáže čekající žádost sám: na stejném monitoru nic navíc, jinde záložní toast, `toast` beze změny
        let peek = |surface, fallback_toast, same_monitor| route(Ctx { allowed: true, surface, fallback_toast, stage: true, same_monitor, covered: true });
        assert_eq!(peek(N, true, true), Nothing);
        assert_eq!(peek(N, true, false), T);
        assert_eq!(peek(N, false, false), Nothing);
        assert_eq!(peek(ST, false, true), T);
    }
}
