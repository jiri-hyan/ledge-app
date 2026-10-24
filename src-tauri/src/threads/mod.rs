//! Dlouhá vlákna jádra (každé spí, když se nic neděje): `ui` (šuplík), `emit` (stav + proužek),
//! `hover` (myš, fullscreen, lištové okno), `notes-watch` (úložiště poznámek), `notes-sync` (git), `notes-folder` (propojení se složkou).
//! Spouští je `run()` v `lib.rs`; komunikují přes `Core` (`core()`), kanály a `mark_dirty`.

pub(crate) mod emit;
pub(crate) mod hover;
pub(crate) mod notes_folder;
pub(crate) mod notes_sync;
pub(crate) mod notes_watch;
pub(crate) mod pointer;
pub(crate) mod ui;
