//! Tauri příkazy (`invoke` z UI) po oblastech. Tenké obálky: logika žije v doménových modulech
//! (`notes.rs`, `procman.rs`, `sessions.rs`, `update.rs`…), stav v `state::Core`. Registrace v `run()`.

pub(crate) mod accounts;
pub(crate) mod actions;
pub(crate) mod cliphist;
pub(crate) mod agents;
pub(crate) mod calendar;
pub(crate) mod config;
pub(crate) mod integrations;
pub(crate) mod costs;
pub(crate) mod drawer;
pub(crate) mod limits;
pub(crate) mod mcp;
pub(crate) mod media;
pub(crate) mod notes;
pub(crate) mod onboarding;
pub(crate) mod pins;
pub(crate) mod pkgctx;
pub(crate) mod projects;
pub(crate) mod scripts;
pub(crate) mod widgets;
pub(crate) mod sessions;
pub(crate) mod sessiongit;
pub(crate) mod settings;
pub(crate) mod shelf;
pub(crate) mod tilelook;
pub(crate) mod timer;
pub(crate) mod update;
