//! „Domov“ lístku (vzor: `desk/` — `DISPLACED`, `on_monitors`): uložená poloha se přepíše jen skutečným přesunem
//! uživatele. Odpojený monitor (Windows přesune okno na jiný), minimalizace (Win+D → `Moved` na -32000) a poloha
//! mimo všechny monitory domov nemění; po návratu monitoru se okno vrátí domů. Čisté funkce, okna řeší `win.rs`.

use super::{BAR_H, Mon, Pin, monitor_of, phys_size, px};

/// Co udělat s polohou z události `Moved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveVerdict {
    /// skutečný přesun → nový domov
    Save,
    /// okno stojí mimo domov (domovský monitor chybí) → domov nechat, okno si pamatovat v `DISPLACED`
    Away,
    /// minimalizace / poloha mimo všechny monitory → nic neukládat
    Ignore,
}

/// Leží lišta domova lístku na některém monitoru (dost na to, aby šla chytit)? Neumístěný lístek domov nemá.
pub fn home_on_screen(p: &Pin, mons: &[Mon], scale: f64) -> bool {
    let (Some(x), Some(y)) = (p.x, p.y) else { return false };
    monitor_of(x, y, phys_size(p, scale).0, px(BAR_H, scale), mons).is_some()
}

/// Rozhodnutí o události `Moved` na `(x, y)`. `minimized` = okno je minimalizované; `home_known` = lístek už
/// nějaký domov má. Pořadí: minimalizace → mimo monitory → chybějící domovský monitor → uložit.
pub fn move_verdict(p: &Pin, x: i32, y: i32, minimized: bool, mons: &[Mon], scale: f64) -> MoveVerdict {
    if minimized {
        return MoveVerdict::Ignore;
    }
    if monitor_of(x, y, phys_size(p, scale).0, px(BAR_H, scale), mons).is_none() {
        return MoveVerdict::Ignore;
    }
    if p.x.is_some() && !home_on_screen(p, mons, scale) {
        return MoveVerdict::Away;
    }
    MoveVerdict::Save
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webpanel::Area;

    const FHD: Area = Area { l: 0, t: 0, r: 1920, b: 1040 };
    const RIGHT: Area = Area { l: 1920, t: 0, r: 4480, b: 1400 };

    fn both() -> Vec<Mon> {
        vec![Mon { work: FHD, scale: 1.0 }, Mon { work: RIGHT, scale: 1.5 }]
    }
    fn one() -> Vec<Mon> {
        vec![Mon { work: FHD, scale: 1.0 }]
    }
    fn at(x: i32, y: i32) -> Pin {
        Pin { x: Some(x), y: Some(y), ..Pin::new(r"C:\n\a.md") }
    }

    #[test]
    fn move_verdict_table() {
        use MoveVerdict::*;
        type Case = (Pin, i32, i32, bool, Vec<Mon>, MoveVerdict, &'static str);
        let cases: &[Case] = &[
            (at(100, 100), 300, 200, false, both(), Save, "běžný tah"),
            (Pin::new("x"), 300, 200, false, both(), Save, "neumístěný lístek dostane první domov"),
            (at(100, 100), -32000, -32000, false, both(), Ignore, "Win+D: poloha mimo monitory"),
            (at(100, 100), -32000, -32000, true, both(), Ignore, "Win+D s příznakem minimalizace"),
            (at(100, 100), 300, 200, true, both(), Ignore, "minimalizované okno na platné poloze"),
            (at(2500, 100), 100, 100, false, one(), Away, "domovský monitor odpojen, Windows přesunuly okno"),
            (at(2500, 100), 2500, 100, false, both(), Save, "monitor zpět: tah je zase skutečný"),
            (at(2500, 100), 9000, 100, false, one(), Ignore, "mimo všechny monitory i s odpojeným domovem"),
            (at(100, 1035), 100, 200, false, one(), Away, "domov pod pracovní plochou (hlavní panel) nejde chytit"),
        ];
        for (p, x, y, min, mons, want, why) in cases {
            assert_eq!(move_verdict(p, *x, *y, *min, mons, 1.0), *want, "{why}");
        }
    }

    #[test]
    fn home_on_screen_table() {
        for (p, mons, want, why) in [
            (at(100, 100), both(), true, "na prvním"),
            (at(2500, 100), both(), true, "na druhém"),
            (at(2500, 100), one(), false, "druhý odpojen"),
            (Pin::new("x"), both(), false, "bez domova"),
            (at(-32000, -32000), both(), false, "uložená poloha minimalizace"),
        ] {
            assert_eq!(home_on_screen(&p, &mons, 1.0), want, "{why}");
        }
    }
}
