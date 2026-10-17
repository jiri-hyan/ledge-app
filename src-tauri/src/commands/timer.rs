//! Příkaz časovače (karta Časovač, paleta Ctrl+K): jedna akce `Op` → nový stav. Logika v `timer.rs`.

use crate::timer::{self, Op, TimerState};

/// Provede akci (`{"op": "start", "minutes": 25}`, `pause`, `resume`, `cancel`, `add`, `swStart`,
/// `swPause`, `swReset`) a vrátí nový stav; proužek a šuplík se překreslí přes `mark_dirty`.
#[tauri::command]
pub(crate) fn timer(op: Op) -> TimerState {
    timer::act(op)
}
