//! Útrata Claude (odhad v cenách API, ne účet) z lokálních transcriptů Claude Code
//! `~/.claude/projects/<slug>/**/*.jsonl`. Čisté moduly: `parse` (řádek → záznam), `price` (ceník),
//! `index` (inkrementální index na disku), `view` (přehled pro kartu). Běh: `costslive.rs`.

pub mod index;
pub mod parse;
pub mod price;
pub mod view;

/// Okno přehledu (dny včetně dneška).
pub const WINDOW_DAYS: u32 = 30;
