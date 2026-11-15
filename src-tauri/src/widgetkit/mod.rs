//! Šablonový widget (`*.kit.json`) bez procesu a bez kódu: soubor ve složce widgetů, který jen popisuje
//! řádky a tlačítka. Rust ho načte a ořeže (`spec`), zdroje dat a vykreslení řeší UI (`src/widgetkit`).
//! Sken složky a pohled pro UI jsou ve `widgets/` (`Kind::Kit`); nic se nespouští, souhlas není potřeba.

pub mod needs;
pub mod spec;
