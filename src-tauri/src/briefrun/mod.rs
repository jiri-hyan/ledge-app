//! Běh zadání z poznámky přes API klíč („Poslat orchestrátorovi“ / „Pokračovat“): `claude -p`
//! stream-json v cwd projektu, klíč jen v env dítěte, vlastní session-id, strop $, MCP Ledge inline.
//! Čisté: `args` (argumenty, env, zprávy, role), `stream` (parser, náklad, výsledný stav), `plan`
//! (zda spustit), `key` (tvar a úložiště klíče), `note` (metadata a log poznámky). I/O: `proc`.
//! Tauri příkazy v `commands/brief.rs`.

pub mod args;
pub mod key;
pub mod note;
pub mod plan;
pub mod proc;
pub mod stream;
