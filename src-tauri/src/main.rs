// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `ledge.exe --statusline` = wrapper stavového řádku Claude Code (`statusline.rs`): úplně první,
    // před Tauri — žádné okno, žádný single-instance zámek, žádná druhá instance šuplíku.
    if std::env::args_os().nth(1).is_some_and(|a| a == "--statusline") {
        std::process::exit(ledge_lib::statusline_main());
    }
    ledge_lib::run()
}
