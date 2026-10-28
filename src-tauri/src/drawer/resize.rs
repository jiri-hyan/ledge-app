//! Změna velikosti otevřeného šuplíku z `appearance.size` (kolo 23): tažení v úpravách a přepnutí
//! záložky s vlastní velikostí. Plynule (ease-out `size::step`) nebo hned; nikdy neaktivuje okno.
//! Volá `threads::ui` (`UiCmd::Resize`).

use super::{Drawer, View, hwnd_of};
use crate::screen::Monitor;
use std::time::{Duration, Instant};
use windows::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};

impl Drawer {
    /// Velikost z `appearance.size` (kolo 23): otevřený Full plynule (`ms` > 0, ease-out jako `animate`)
    /// nebo hned na novou velikost. `fixed_h` = pevná výška záložky známá předem (přepnutí záložky).
    /// Bez aktivace okna; zavřený / malý pohled si velikost vezme při dalším otevření.
    pub fn resize(&self, mon: &Monitor, width: f64, ms: u64, fixed_h: Option<f64>) {
        if let Some(fh) = fixed_h {
            self.st.lock().full_h = fh;
        }
        if self.view() != View::Full {
            return;
        }
        let (Some(w), true) = (self.window(), ms > 0) else {
            self.place(View::Full, mon, width, false);
            return;
        };
        let Some(h) = hwnd_of(&w) else { return };
        let r = self.rect();
        let (tx, ty, tw, th) = self.target(View::Full, mon, width);
        let overlap = (self.overlap * mon.dpi as f64 / 96.0).round() as i32;
        let (from, to) = ([r.left, r.right - r.left, r.bottom - r.top + overlap], [tx, tw, th]);
        let t0 = Instant::now();
        loop {
            let t = (t0.elapsed().as_secs_f64() * 1000.0 / ms as f64).min(1.0);
            let [x, pw, ph] = crate::appearance::size::step(from, to, t);
            unsafe {
                let _ = SetWindowPos(h, Some(HWND_TOPMOST), x, ty, pw, ph, SWP_NOACTIVATE | SWP_NOZORDER);
            }
            if t >= 1.0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(12));
        }
        self.place(View::Full, mon, width, false);
    }
}
