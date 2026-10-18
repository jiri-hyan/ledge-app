// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `ledge.exe --statusline` = wrapper stavového řádku Claude Code (`statusline.rs`): úplně první,
    // před Tauri — žádné okno, žádný single-instance zámek, žádná druhá instance šuplíku.
    if std::env::args_os().nth(1).is_some_and(|a| a == "--statusline") {
        std::process::exit(ledge_lib::statusline_main());
    }
    // `ledge.exe --mcp` = stdio proxy MCP (`mcp/proxy.rs`) pro Claude Code / Desktop: taky před Tauri —
    // žádné okno, žádná GUI instance, nic se nepřeposílá běžícímu Ledge (ten dostane jen POST /mcp)
    if std::env::args_os().nth(1).is_some_and(|a| a == "--mcp") {
        std::process::exit(ledge_lib::mcp_main());
    }
    ledge_lib::run()
}
