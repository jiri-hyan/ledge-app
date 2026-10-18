// Angličtina pro Nastavení → Claude → Claude (MCP) (`mcp/McpSettings.tsx`) a volbu `settings.f.mcp.*`
// z `settings/labels.ts`.
import { registerDict } from "../i18n";

registerDict("en", {
  "settings.mcp.title": "Claude (MCP)",
  "settings.mcp.desc": "Claude Code or Claude Desktop can set Ledge up for you: it adds the apps, websites and quick actions you use and adjusts the tabs, cards and look. It may only read and change Ledge's settings — it launches nothing and sends nothing anywhere (this PC only). You can undo every change.",
  "settings.mcp.on": "Claude can now set up Ledge — copy the command below",
  "settings.mcp.off": "MCP is off — Claude won't set up Ledge any more",
  "settings.mcp.bridgeOff": "Approvals from Ledge (above) are off — MCP doesn't run without them. Turn them on and save.",
  "settings.mcp.cli": "Connect Claude Code",
  "settings.mcp.cli.hint": "Run it once in a terminal. Ledge hands over the key itself; it's not in the command.",
  "settings.mcp.copy": "Copy",
  "settings.mcp.copied": "Copied — paste it into a terminal or Claude's config",
  "settings.mcp.desktop": "Connect Claude Desktop",
  "settings.mcp.desktop.hint": "A snippet for claude_desktop_config.json (the mcpServers section), then restart Claude Desktop.",
  "settings.mcp.key": "Backup over http",
  "settings.mcp.key.hint": "Only when a client can't run a command: the address {url} and the key in a header. Never send the key to anyone.",
  "settings.mcp.key.show": "Show key",
  "settings.mcp.key.hide": "Hide key",
  "settings.mcp.rotate": "Replace key",
  "settings.mcp.rotate.ask": "Replace the MCP key?",
  "settings.mcp.rotate.detail": "The old key stops working right away. Connections through the command (stdio) pick up the new one after the client restarts; the http backup has to be added again.",
  "settings.mcp.rotate.yes": "Replace",
  "settings.mcp.rotated": "Key replaced",
  "settings.f.mcp.enabled": "Let Claude set up Ledge (MCP)",
  "settings.f.mcp.enabled.hint": "Claude may read and change Ledge's settings; it launches nothing and sends nothing anywhere. Off by default.",
  "settings.f.mcp.enabled.kw": "mcp claude server set up configure key tools",
});
