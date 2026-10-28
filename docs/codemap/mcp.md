# Codemapa: MCP server pro Clauda (`mcp`)

Claude Code / Claude Desktop čte a mění nastavení Ledge (zástupci, rychlé akce, rozložení, proužek, vzhled) přes MCP. Kontrakt jmen (příkazy, nástroje, `src/mcp/api.ts`) je mezi workery kola 9 — měníš-li jméno, změň kontrakt i docs. Formát pro agenty: [agents/api-mcp.md](../agents/api-mcp.md) → „MCP server (`/mcp`)“ …

Detail (toky, invarianty, testy, co nečíst, historie): [mcp-detail.md](mcp-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/mcp/mod.rs` | config `mcp`, `Host` (co MCP potřebuje od Ledge), `Server` (klíč, Vrátit, zámek zápisů), HTTP brána v pořadí vypnuto 404 → klíč 401 → metoda 405 → velikost 413 → verze 400 → JSON-RPC | `McpConfig` `Host` `Server` `HttpReq` `HttpResp` `handle_http` `PATH` `MAX_BODY` |
| `mcp/rpc.rs` | JSON-RPC 2.0 ručně: `initialize` (vyjednání verze), `ping`, `tools/list`, `tools/call`, notifikace → 202, dávky, kódy chyb | `handle` `Outcome` `Tools` `RpcErr` `VERSIONS` `negotiate` `INSTRUCTIONS` |
| `mcp/tools.rs` | katalog nástrojů (jméno, popis pro model, schéma vstupu ze schemars, anotace) a volání; společný zápis `write` (validace → snímek → `Host::save` → Vrátit) a `undo` … | `TOOLS` `Tool` `Handler` `embed` `slim` `id_and_fields` |
| `mcp/notes.rs` · `notes_tests.rs` | nástroje `notes_*` nad kořenem poznámek (žádná vlastní logika úložiště — `notes::edit::{query, read, save, toggle_task}`, `notes::create`) … | `list` `read` `create` `append` `append_to` `toggle` `note_path` `Style` `BODY_CAP` |
| `mcp/round.rs` · `discover.rs` | čísla ve výstupu na 2 místa (f32 šum) · výstup `apps_discover` (Start menu + Store `source: "store"`/`aumid` + repa; `alreadyInLedge` jen `true`) | `clean` `two` · `output` |
| `mcp/edit.rs` | zástupci (`projects`) a akce: přidat (návrh ze složky přes `Host::detect`), upravit (sloučení), odebrat … | `tile_add` `tile_update` `tile_remove` `action_add` `action_update` `action_remove` |
| `runguard.rs` | pojistka spouštění zástupců (klik se nepotvrzuje): `run` jen skript balíčkovače nebo příkaz nalezený ve složce, `exe` jen `.exe`, `args` jen navržené; test `commands_table` | `check` `Issue` `run_ok` `exe_ok` `args_ok` `detected_runs` |
| `mcp/look.rs` | rozložení (celé, `upgrades` se převezmou), proužek (sloučení), vzhled (styl + pole navrch, `blur` = `drawer.glass`); zrcadlo tabulky stylů | `layout_get` `layout_parse` `unknown_cards` `layout_summary` `strip_get` `strip_set` |
| `mcp/schema.rs` | nad JSON Schema configu: neznámá pole (překlep = chyba), RFC 7396 sloučení, katalog výčtů, čitelné chyby serde | `patch` `parse` `merge` `unknown_key` `catalog` `enum_values` `serde_msg` |
| `mcp/undo.rs` | zásobník 20 zápisů; záznam = jen dotčené části (pointer, před, po); vrácení odmítne, když část změnil někdo jiný | `Undo` `Entry` `pick` `revert` `MAX` |
| `mcp/auth.rs` | klíč 32 B (base64url) ve Správci pověření `Ledge/mcp` přes `accounts::vault::Vault`, cache, Bearer, konstantní čas (`pkce::same`) | `Keys` (`get` `ensure` `rotate` `check`) `TARGET` `bearer` `read` `fresh` |
| `mcp/proxy.rs` | `ledge.exe --mcp`: řádky stdin → `POST /mcp` (HTTP/1.0, termín) → stdout; chyby jako JSON-RPC se stejným `id`; po 401/404 jednou znovu přečte klíč | `main` `run` `post_http` `parse_response` `classify` `Post` `Fail` `fail_text` |
| `mcp/host.rs` | skutečný `Host` nad `state::core` (uložení přes `commands::config::save_config` + událost `config-changed`, sken `commands::onboarding::scan_apps`/`scan_repos`, `open_drawer` = … | `LiveHost` `server` `endpoint` `notify_ui` `EVENT` |
| `mcp/tests.rs` · `proxy_tests.rs` | `MemHost` + `MemVault`; brána, skutečný most, každý nástroj; proxy nad `Cursor` i nad skutečnými rourami (dítě = tentýž testovací binár, `pipe_child`) | `MemHost` `server` |
| `src-tauri/src/commands/mcp.rs` | Tauri příkazy (kontrakt) + `mcp_reveal_key`; texty příkazů pro klienty | `mcp_state` `mcp_set_enabled` `mcp_rotate_key` `mcp_reveal_key` `cli_command` |
| `src/mcp/api.ts` · `key.ts` | IPC přesně dle kontraktu · klíč pro http zálohu (mimo kontrakt) | `mcpState` `mcpSetEnabled` `mcpRotateKey` · `mcpRevealKey` |
| `src/mcp/McpSettings.tsx` · `en.ts` | sekce Nastavení → Claude → Claude (MCP): přepínač (hned), Kopírovat pro Claude Code / Desktop, Zobrazit / Vyměnit klíč | `McpSettings` |
| `src/mcp/merge.ts` | politika změny zvenku vůči rozpracovanému Nastavení (test `merge.test.mjs`) | `external` `changedSections` |
