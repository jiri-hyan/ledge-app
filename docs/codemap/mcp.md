# Codemapa: MCP server pro Clauda (`mcp`)

Claude Code / Claude Desktop čte a mění nastavení Ledge (zástupci, rychlé akce, rozložení, proužek,
vzhled) přes MCP. Kontrakt jmen (příkazy, nástroje, `src/mcp/api.ts`) je mezi workery kola 9 — měníš-li
jméno, změň kontrakt i docs. Formát pro agenty: [agents/api-mcp.md](../agents/api-mcp.md) → „MCP server (`/mcp`)“. Cituj symbolem.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/mcp/mod.rs` | config `mcp`, `Host` (co MCP potřebuje od Ledge), `Server` (klíč, Vrátit, zámek zápisů), HTTP brána v pořadí vypnuto 404 → klíč 401 → metoda 405 → velikost 413 → verze 400 → JSON-RPC | `McpConfig` `Host` `Server` `HttpReq` `HttpResp` `handle_http` `PATH` `MAX_BODY` |
| `mcp/rpc.rs` | JSON-RPC 2.0 ručně: `initialize` (vyjednání verze), `ping`, `tools/list`, `tools/call`, notifikace → 202, dávky, kódy chyb | `handle` `Outcome` `Tools` `RpcErr` `VERSIONS` `negotiate` `INSTRUCTIONS` |
| `mcp/tools.rs` | katalog nástrojů (jméno, popis pro model, schéma vstupu ze schemars, anotace) a volání; společný zápis `write` (validace → snímek → `Host::save` → Vrátit) a `undo`; `slim` zbavuje vložená schémata popisů, `default`, mezí a null variant (`tools/list` < 20 kB) | `TOOLS` `Tool` `Handler` `embed` `slim` `id_and_fields` |
| `mcp/notes.rs` · `notes_tests.rs` | nástroje `notes_*` nad kořenem poznámek (žádná vlastní logika úložiště — `notes::edit::{query, read, save, toggle_task}`, `notes::create`): výpis, čtení s úkoly, vytvoření (projekt / kategorie proti configu), připsání (`append`: odstavec / odrážka / úkol, naváže na seznam i číslování), odškrtnutí; `Host::{notes_root, notes_changed, pin_note}` | `list` `read` `create` `append` `append_to` `toggle` `note_path` `Style` `BODY_CAP` `LIST_MAX` |
| `mcp/round.rs` · `discover.rs` | čísla ve výstupu na 2 místa (f32 šum) · výstup `apps_discover` (Start menu + Store `source: "store"`/`aumid` + repa; `alreadyInLedge` jen `true`) | `clean` `two` · `output` |
| `mcp/edit.rs` | zástupci (`projects`) a akce: přidat (návrh ze složky přes `Host::detect`), upravit (sloučení), odebrat; pravidlo příkazu zástupce; `kind: app` má `passPort`/`openBrowser` vždy `false` | `tile_add` `tile_update` `tile_remove` `action_add` `action_update` `action_remove` `check_cmd` `detected_runs` `PM_SCRIPT` `unique_id` |
| `mcp/look.rs` | rozložení (celé, `upgrades` se převezmou), proužek (sloučení), vzhled (styl + pole navrch, `blur` = `drawer.glass`); zrcadlo tabulky stylů | `layout_get` `layout_parse` `unknown_cards` `layout_summary` `strip_get` `strip_set` `appearance_get` `appearance_set` `STYLES` `Style` |
| `mcp/schema.rs` | nad JSON Schema configu: neznámá pole (překlep = chyba), RFC 7396 sloučení, katalog výčtů, čitelné chyby serde | `patch` `parse` `merge` `unknown_key` `catalog` `enum_values` `serde_msg` |
| `mcp/undo.rs` | zásobník 20 zápisů; záznam = jen dotčené části (pointer, před, po); vrácení odmítne, když část změnil někdo jiný | `Undo` `Entry` `pick` `revert` `MAX` |
| `mcp/auth.rs` | klíč 32 B (base64url) ve Správci pověření `Ledge/mcp` přes `accounts::vault::Vault`, cache, Bearer, konstantní čas (`pkce::same`) | `Keys` (`get` `ensure` `rotate` `check`) `TARGET` `bearer` `read` `fresh` |
| `mcp/proxy.rs` | `ledge.exe --mcp`: řádky stdin → `POST /mcp` (HTTP/1.0, termín) → stdout; chyby jako JSON-RPC se stejným `id`; po 401/404 jednou znovu přečte klíč | `main` `run` `post_http` `parse_response` `classify` `Post` `Fail` `fail_text` |
| `mcp/host.rs` | skutečný `Host` nad `state::core` (uložení přes `commands::config::save_config` + událost `config-changed`, sken `commands::onboarding::scan_apps`/`scan_repos`, `open_drawer` = `UiCmd::Show(Full, Remote)`), globální `Server` s `WinCred`, endpoint pro most | `LiveHost` `server` `endpoint` `notify_ui` `EVENT` |
| `mcp/tests.rs` · `proxy_tests.rs` | `MemHost` + `MemVault`; brána, skutečný most, každý nástroj; proxy nad `Cursor` i nad skutečnými rourami (dítě = tentýž testovací binár, `pipe_child`) | `MemHost` `server` |
| `src-tauri/src/commands/mcp.rs` | Tauri příkazy (kontrakt) + `mcp_reveal_key`; texty příkazů pro klienty | `mcp_state` `mcp_set_enabled` `mcp_rotate_key` `mcp_reveal_key` `cli_command` `desktop_config` `http_command` |
| `src/mcp/api.ts` · `key.ts` | IPC přesně dle kontraktu · klíč pro http zálohu (mimo kontrakt) | `mcpState` `mcpSetEnabled` `mcpRotateKey` · `mcpRevealKey` |
| `src/mcp/McpSettings.tsx` · `en.ts` | sekce Nastavení → Claude → Claude (MCP): přepínač (hned), Kopírovat pro Claude Code / Desktop, Zobrazit / Vyměnit klíč | `McpSettings` |
| `src/mcp/merge.ts` | politika změny zvenku vůči rozpracovanému Nastavení (test `merge.test.mjs`) | `external` `changedSections` |

Lepidlo: `claude/bridge.rs` (`/mcp` před ostatními cestami, vlastní strop těla, `Mcp` endpoint), `lib.rs`
(`mod mcp`, `mcp_main`, `serve(…, mcp::host::endpoint())`, příkazy), `main.rs` (`--mcp` před Tauri),
`config/*` (`Config.mcp`), `commands/onboarding.rs` (`scan_apps`/`scan_repos` `pub(crate)`),
`src/Settings.tsx` (`config-changed` → `adoptExternal`), `src/settings/ctx.ts` (`adoptBase`),
`settings/labels.ts` (`mcp.enabled`), `settings/panels/Claude.tsx`, `i18n/load.ts`, `settings.test.mjs`.

## Toky

- **Požadavek:** most (`Origin` → 403, cizí `Host` → 403) → `handle_http` → `rpc::handle` → `tools::Handler`.
  Každý požadavek ve vlastním krátkém vlákně (jako ostatní cesty mostu), nic neběží v klidu.
- **Zápis:** `Handler::write` drží `Server.write` (zápisy MCP po jednom) → `Host::config` → `edit`/`look` →
  `config::validate_field` → beze změny = nic → `undo::pick` před → `Host::save` (= `save_config` →
  `apply_config` → `state`/`appearance` do šuplíku, + `config-changed`) → `undo::pick` po → `Undo::push`.
- **Nastavení otevřené při zápisu:** `config-changed` → `adoptExternal` → `merge::external`: bez úprav
  `loadBase`; úpravy v jiné sekci `adoptBase` (sloučení, odložené `afterSave` zůstanou); stejná sekce →
  `setConflict` (lišta Načíst nové / Přesto uložit). Režim úprav rozložení v šuplíku (`layout/edit.ts`) při
  Hotovo bere `get_config` a přepíše jen `layout` — změna rozložení přes MCP během rozpracované úpravy
  rozložení se Hotovem přepíše (vědomě; jiné sekce zůstanou).
- **Poznámky:** `Handler::note_write` (zámek zápisů → `Host::notes_root` → `mcp::notes` → `Host::notes_changed`)
  — mimo Vrátit (zásobník je na části configu); chyba otisku = „načti znovu“. Skutečný `Host` bere kořen z
  `threads::notes_watch::notes_root`, připnutí z `pins::win::{open, close}`.
- **Proxy:** `main.rs --mcp` → `proxy::main` (config jen pro port a jazyk, klíč z `WinCred`) → `run` nad
  `stdin().lock()`/`stdout().lock()`; konec stdin = konec procesu.

## Invarianty a pasti

- **Bezpečnost:** výchozí vypnuto; klíč jen ve Správci pověření (ne config, log, chyba, příkaz stdio);
  `check` bez klíče v úložišti nepustí nikoho; 401/404 bez těla. Žádný nástroj nic nespouští ani
  neschvaluje; akce z MCP nedostane souhlas (`Consents::grant` jen UI). Zástupce jen s `run` z `detected_runs`
  nebo `PM_SCRIPT`, bez `args` (spouští se bez potvrzení).
- Validace je JEN ta configová (`config::validate_field`), MCP přidává jen: neznámá pole, katalog stylů,
  pravidlo příkazu zástupce, `itemsRev` nejde měnit.
- Tabulka stylů žije ve frontendu (`src/theme/looks.ts` `STYLES`); `look::STYLES` je zrcadlo — test
  `styles_mirror_frontend_table` čte TS. Nový styl = řádek v obou.
- **Velikost a výstup:** `tools/list` < 20 kB (test `tools_list_stays_small` vypíše velikost po nástrojích);
  nové pole configu nepřidává velikost (popisy se z vložených schémat stripují), nový nástroj ano. Výstupy
  nástrojů = `tools::pretty` (jméno historické: kompaktní JSON + `round::clean`).
- **`layout_*` chyby:** `layout_parse` sbírá VŠECHNA neznámá id karet (`unknown_cards`) a přidá je k první
  chybě serde („Neznámé karty: foo, bar. Povolené: …“).
- **Store aplikace:** `tiles_add {"kind":"app","aumid":…}`; spuštění `commands::projects::launch` →
  `onboarding::store::launch` (bez ProcMan, bez sledování procesu).
- Odpověď je vždy jeden řádek JSON (proxy ji posílá jako řádek); `INSTRUCTIONS` bez zalomení.
- GUI subsystém (release) dostane stdio roury od klienta stejně jako `--statusline`; ověřeno testem se
  skutečnými rourami nad testovacím binárem (konzolový), živě zbývá ověřit release exe.

## Testy

`cargo test mcp` — `round::tests`, `discover::tests`, `rpc::tests` (vyjednání verze, tabulka protokolu, dávky), `auth::tests` (Bearer, klíč,
výměna), `schema::tests` (sloučení, katalog, neznámá pole, hlášky), `undo::tests`, `look::tests` (zrcadlo
stylů), `edit::tests` (`PM_SCRIPT`, id), `mcp::tests` (brána, skutečný most: 401/403/404/413, katalog =
kontrakt, každý zápis platný + Vrátit / neplatný beze změny, fixture `layout-validate.json`, Vrátit po cizí
úpravě, starý config bez `mcp`), `proxy::tests` (smyčka, chyby se stejným `id`, HTTP i chunked, skutečné
roury), `commands::mcp::tests` (příkazy pro klienty). UI: `src/mcp/merge.test.mjs`.
