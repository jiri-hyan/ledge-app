//! Tauri příkazy MCP (kontrakt `src/mcp/api.ts`): stav, zapnutí jedním klikem (při prvním zapnutí
//! vznikne klíč ve Správci pověření), výměna klíče; `mcp_reveal_key` jen na výslovné „Zobrazit klíč“
//! (http záloha). Zapnutí / vypnutí se ukládá hned stejnou cestou jako Uložit (`save_config`) — jako
//! zapojení hooků v tomtéž panelu; Nastavení se pak samo přenačte (`mcp::host::notify_ui`).

use crate::mcp::host::{notify_ui, server};
use crate::state::core;
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpState {
    enabled: bool,
    url: String,
    cli_command: String,
    desktop_config: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpKey {
    key: String,
    http_command: String,
}

/// `http://127.0.0.1:<port>/mcp`
pub(crate) fn url(port: u16) -> String {
    format!("http://127.0.0.1:{port}{}", crate::mcp::PATH)
}

/// Příkaz pro Claude Code (stdio; klíč si proxy čte sám).
pub(crate) fn cli_command(exe: &str) -> String {
    format!("claude mcp add --scope user ledge -- \"{exe}\" --mcp")
}

/// Úryvek `claude_desktop_config.json` (stdio).
pub(crate) fn desktop_config(exe: &str) -> String {
    let v = serde_json::json!({ "mcpServers": { "ledge": { "command": exe, "args": ["--mcp"] } } });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Záložní příkaz přes http s klíčem v hlavičce.
pub(crate) fn http_command(port: u16, key: &str) -> String {
    format!("claude mcp add --transport http --scope user ledge {} --header \"Authorization: Bearer {key}\"", url(port))
}

fn exe() -> String {
    std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "ledge.exe".into())
}

pub(crate) fn state_for(enabled: bool, port: u16, exe: &str) -> McpState {
    McpState { enabled, url: url(port), cli_command: cli_command(exe), desktop_config: desktop_config(exe) }
}

fn state() -> McpState {
    let cfg = core().cfg.read().clone();
    state_for(cfg.mcp.enabled, cfg.claude.port, &exe())
}

#[tauri::command]
pub(crate) fn mcp_state() -> McpState {
    // zapnuté ručně v configu bez klíče → klíč dovyrobit, jinak by klienti dostávali jen 401
    if core().cfg.read().mcp.enabled
        && let Err(e) = server().keys.ensure()
    {
        crate::state::log(&format!("mcp: {e}"));
    }
    state()
}

#[tauri::command]
pub(crate) fn mcp_set_enabled(enabled: bool) -> Result<McpState, String> {
    if enabled {
        server().keys.ensure()?;
    }
    let mut cfg = core().cfg.read().clone();
    if cfg.mcp.enabled != enabled {
        cfg.mcp.enabled = enabled;
        crate::commands::config::save_config(cfg)?;
        notify_ui("settings");
    }
    Ok(state())
}

#[tauri::command]
pub(crate) fn mcp_rotate_key() -> Result<McpState, String> {
    server().keys.rotate()?;
    Ok(state())
}

/// Klíč pro http zálohu — jen na výslovný klik „Zobrazit klíč“ (nikdy v logu ani configu).
#[tauri::command]
pub(crate) fn mcp_reveal_key() -> Result<McpKey, String> {
    let key = server().keys.ensure()?;
    let port = core().cfg.read().claude.port;
    Ok(McpKey { http_command: http_command(port, &key), key })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_for_clients() {
        let exe = r"C:\Program Files\Ledge\ledge.exe";
        let s = state_for(true, 47800, exe);
        assert_eq!(s.url, "http://127.0.0.1:47800/mcp");
        assert_eq!(s.cli_command, r#"claude mcp add --scope user ledge -- "C:\Program Files\Ledge\ledge.exe" --mcp"#);
        let d: serde_json::Value = serde_json::from_str(&s.desktop_config).unwrap();
        assert_eq!(d["mcpServers"]["ledge"]["command"], exe);
        assert_eq!(d["mcpServers"]["ledge"]["args"], serde_json::json!(["--mcp"]));
        assert!(!s.cli_command.contains("Bearer") && !s.desktop_config.contains("Bearer"), "klíč nikdy v příkazu stdio");
        assert_eq!(http_command(47801, "K"), r#"claude mcp add --transport http --scope user ledge http://127.0.0.1:47801/mcp --header "Authorization: Bearer K""#);
        // tvar pro UI (kontrakt `src/mcp/api.ts`)
        let v = serde_json::to_value(&s).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["cliCommand", "desktopConfig", "enabled", "url"]);
    }
}
