// Klíč MCP pro http zálohu — jen na výslovné „Zobrazit klíč“ v Nastavení → Claude (MCP). Mimo kontrakt
// `api.ts` (ten je sdílený s průvodcem a má přesný obsah).
import { invoke } from "@tauri-apps/api/core";

export type McpKey = { key: string; httpCommand: string };
export const mcpRevealKey = () => invoke<McpKey>("mcp_reveal_key");
