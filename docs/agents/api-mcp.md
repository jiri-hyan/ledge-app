# Lokální API a MCP server

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

## Lokální API (jen 127.0.0.1:47800)

| volání | co dělá |
|---|---|
| `GET /status` | čekající schválení, sessions, běžící procesy, cesta ke configu |
| `POST /ctl/open` `{"view":"full"\|"peek"\|"compact"\|"island"}` | otevře šuplík / peek / náhled po najetí (bez fokusu, nezavírá se sám) |
| `POST /ctl/close` | zavře šuplík |
| `POST /ctl/settings` | otevře Nastavení |
| `POST /ctl/launch` `{"id":"atlas"}` | spustí projekt jako klik na logo |
| `POST /ctl/stop` `{"id":"atlas"}` | zastaví projekt (celý strom procesů) |
| `POST /ctl/reload` | znovu načte config |
| `POST /permission` | **jen pro hook `PermissionRequest`** |
| `POST /question` | **jen pro hook `PreToolUse` s matcherem `AskUserQuestion`** (otázky Clauda) |
| `POST /plan` | **jen pro hook `PreToolUse` s matcherem `ExitPlanMode`** (review plánu) |
| `POST /event` | hooky SessionStart/UserPromptSubmit/PostToolUse/Stop/SessionEnd/Notification/SubagentStart/SubagentStop |
| `POST /mcp` | MCP server pro Clauda (JSON-RPC, `Authorization: Bearer <klíč>`; vypnuto = 404) — viz níže |

Požadavky s hlavičkou `Origin` (prohlížeč) se odmítají.

### MCP server (`/mcp`)

Claude Code / Claude Desktop nastaví Ledge podle toho, co uživatel používá. Zapíná se v Nastavení → Claude →
Claude (MCP) (výchozí vypnuto). Mapa: `docs/codemap/mcp.md`.

- **Připojení (doporučené, stdio):** `claude mcp add --scope user ledge -- "<cesta k ledge.exe>" --mcp`
  (přesný příkaz zkopíruje Nastavení). `ledge.exe --mcp` je tenký proxy stdin/stdout ↔ `POST /mcp`, klíč si
  čte sám ze Správce pověření; neotevírá okno a nestartuje druhý Ledge. Ledge neběží → chyba JSON-RPC „Ledge
  neběží“, MCP vypnuté → „Zapni v Ledge Nastavení → Claude (MCP)“. Claude Desktop: `mcpServers.ledge =
  { "command": "<ledge.exe>", "args": ["--mcp"] }`.
- **Záloha (http):** `claude mcp add --transport http --scope user ledge http://127.0.0.1:47800/mcp --header
  "Authorization: Bearer <klíč>"` — klíč jen na „Zobrazit klíč“. Streamable HTTP bez SSE a bez session: `POST`
  = jedna zpráva (nebo dávka), odpověď `application/json`; notifikace → 202; `GET`/`DELETE` → 405; bez klíče /
  špatný klíč → 401 (bez detailu); tělo > 256 KB → 413. Protokol 2025-06-18, 2025-03-26, 2024-11-05.
- **Nástroje:** `ledge_status`, `apps_discover` (Start menu uživatele i ProgramData, aplikace z Microsoft
  Store ze `shell:AppsFolder` — položky `source: "store"` s `aumid` místo `exe`, repa na disku; nic z
  prohlížeče; `alreadyInLedge` jen když je `true`), `tiles_list` / `tiles_add` / `tiles_update` / `tiles_remove` (zástupci = `projects`),
  `actions_list` / `actions_add` / `actions_update` / `actions_remove`, `layout_get` / `layout_preview` /
  `layout_apply`, `strip_get` / `strip_set`, `appearance_get` / `appearance_set` (styl jen z katalogu stylů,
  hodnoty jen z výčtů schématu), `notes_list` / `notes_read` / `notes_create` / `notes_append` /
  `notes_toggle_task` / `notes_pin` (poznámky, viz níže), `undo_last`, `open_drawer`. Každý má JSON Schema
  vstupu a popis pro model.
  **Velikost:** `tools/list` < 20 kB (test `tools_list_stays_small`; schémata configu bez popisů polí, `default`
  a mezí, `*_update` a `layout_preview` bez druhé kopie schématu), výstupy nástrojů jsou jednořádkový JSON a
  neceločíselná čísla mají 2 desetinná místa (`mcp/round.rs`; config na disku se nemění).
- **Poznámky (`notes_*`, `mcp/notes.rs`):** jen zapnutí MCP + klíč jako u ostatních; poznámky vypnuté →
  věta „Poznámky jsou vypnuté“. Žádná vlastní logika úložiště: výběr `notes::edit::query`, čtení / zápis
  `notes::edit::{read, save, toggle_task}` (otisk obsahu — změna na disku mezi čtením a zápisem = chyba „načti
  znovu“, nic se nepřepíše), vytvoření `notes::create`, cesta jen uvnitř kořene (`check_note_path`; relativní
  se bere ke kořeni). `notes_create` ověří `project` / `kind` proti configu (Ledge neumí zobrazit neznámé).
  `notes_append` s `style: bullet|task` naváže na poslední seznam (stejná odrážka a odsazení, číslovaný
  pokračuje `3.` → `4.`), `text` je odstavec. `notes_pin` = `pins::win::open`/`close` (bez fokusu).
  Po zápisu `Host::notes_changed` (= `reload_notes` + sync). Zápisy do poznámek **nejsou** v `undo_last`
  (zásobník je na části configu) a drží stejný zámek zápisů jako config. `notes_read` ořízne tělo na 30 000
  znaků (`truncated`).
- **Zápis = stejná cesta jako Uložit v Nastavení:** načíst config → změna → `config::validate_field` (stejná
  validace, i editoru rozložení) → snímek dotčených částí do Vrátit (posledních 20, jen v paměti) →
  `save_config` → událost `config-changed`. Neplatný vstup = výsledek s `isError: true` a větou, co je špatně a
  co jde (config beze změny); chyba protokolu (neznámý nástroj, špatná zpráva) = JSON-RPC error. `undo_last`
  vrátí jen části, které ten zápis změnil, a odmítne, když je mezitím někdo změnil jinde.
- **Neuložené úpravy v Nastavení:** zápis přes MCP projde vždy. Otevřené Nastavení dostane událost: bez
  rozpracovaných změn se přenačte; rozpracované v jiné sekci zůstanou a změna zvenku se do nich sloučí; tatáž
  sekce = lišta konfliktu „Načíst nové / Přesto uložit“ (nic se nepřepíše potichu, `src/mcp/merge.ts`).
- **MCP nic nespouští** (žádné launch/run nástroje). Akce `command` / `app` s argumenty přidaná přes MCP souhlas
  nedostane — Ledge se zeptá při prvním spuštění jako u ruční. Zástupce (karta Projekty) se spouští bez
  potvrzení, proto mu MCP smí dát jen skript balíčkovače (`npm run dev`, `pnpm dev`…) nebo příkaz, který Ledge
  sám najde ve složce projektu; jiný příkaz → rychlá akce `command`.

**Schválení přes API záměrně NEJDE.** Povolit akci Clauda smí jen člověk — klikem v šuplíku (nebo
zkratkou, je-li zapnutá). Nepřidávej `/ctl/decide` ani nic podobného: z mostu by se stal způsob, jak si agent
schválí sám sebe.

