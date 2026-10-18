import { invoke } from "@tauri-apps/api/core";

export type McpState = {
  enabled: boolean;
  /** http://127.0.0.1:47800/mcp */
  url: string;
  /** Příkaz pro Claude Code (stdio, klíč se nikam nevypisuje): claude mcp add --scope user ledge -- "<ledge.exe>" --mcp */
  cliCommand: string;
  /** JSON úryvek pro claude_desktop_config.json (mcpServers.ledge, stdio) */
  desktopConfig: string;
};

export const mcpState = () => invoke<McpState>("mcp_state");
export const mcpSetEnabled = (enabled: boolean) => invoke<McpState>("mcp_set_enabled", { enabled });
export const mcpRotateKey = () => invoke<McpState>("mcp_rotate_key");
