# Lokální API a MCP server

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

## Lokální API (jen 127.0.0.1:47800)

`GET /status` a `POST /ctl/*` chtějí hlavičku `Authorization: Bearer <klíč hooků>` (bez ní 401; skripty ji vezmou z `scripts/ledgeapi.ps1::Get-LedgeAuth`). Hooky a `/mcp` mají klíč už dřív.

| volání | co dělá |
|---|---|
| `GET /status` | čekající schválení, sessions, běžící procesy (jen povolená pole, bez `cmd`/`cwd`/výstupu), stav šuplíku, blok `qa` (viz „Izolovaná instance pro QA“); bez cesty ke configu (L247, `ctl.rs` `PROC_ALLOW`) |
| `POST /ctl/open` `{"view":"full"\|"peek"\|"island"\|"mid"}` | otevře šuplík / peek / náhled po najetí (bez fokusu, nezavírá se sám); ve hře / herním režimu 400 a nic |
| `POST /ctl/close` | zavře šuplík |
| `POST /ctl/settings` | otevře Nastavení |
| `POST /ctl/launch` `{"id":"atlas"}` | spustí projekt jako klik na logo |
| `POST /ctl/stop` `{"id":"atlas"}` | zastaví projekt (celý strom procesů) |
| `POST /ctl/reload` | znovu načte config |
| `POST /permission` | **jen pro hook `PermissionRequest`** |
| `POST /question` | **jen pro hook `PreToolUse` s matcherem `AskUserQuestion`** (otázky Clauda) |
| `POST /plan` | **jen pro hook `PreToolUse` s matcherem `ExitPlanMode`** (review plánu) |
| `POST /event` | hooky SessionStart/UserPromptSubmit/PostToolUse/Stop/SessionEnd/Notification/SubagentStart/SubagentStop/PostToolUseFailure; bez platného klíče jen hlásí aktivitu a nikdy neuvolní kartu (`handle_event_as`) |
| `POST /mcp` | MCP server pro Clauda (JSON-RPC, `Authorization: Bearer <klíč>`; vypnuto = 404) — viz níže |

Hooky nesou `Authorization: Bearer <klíč hooků>` (`Ledge/hooks` ve Správci pověření, jiný než klíč MCP). `/permission`,
`/question` a `/plan` bez platného klíče vrací prázdnou 200 bez karty, terminál se zeptá sám. `/event` bez hlavičky projde
(session spuštěná před aktualizací), se špatnou se zahodí. Klíč doplní hookům ze starší verze start Ledge
(`hooksetup::migrate_on_start`), jen když už zapojené jsou.

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
  `notes_toggle_task` / `notes_pin` / `notes_set_status` / `notes_ask` / `notes_propose_done` / `notes_log` (poznámky,
  viz níže), `undo_last`, `open_drawer`. Každý má JSON Schema
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
  **Zadání (kolo 19, formát v [notes.md](notes.md) → „Zadání“):** `notes_read` vrací u bodů `state` (`open|proposed|done`),
  `evidence`, `images [{path, caption}]` (absolutní cesta — čti ji nástrojem Read), `refs [{raw, path, line, exists}]`,
  dále `questions [{index, text, answer}]`, `looseImages`, `status`/`cost`/`budget`/`model`/`session`. `notes_ask` přidá
  otázky (1–10) do `## Otázky`, `notes_log` připíše řádek do `## Log`, `notes_set_status` smí jen `running|waiting|review`,
  `notes_propose_done(path, text, evidence, line?)` nastaví `[~]` (bod se hledá podle `text`, `line` je nápověda;
  nejednoznačné = chyba). **Model nikdy nedá `[x]`:** `notes_toggle_task` smí jen znovu otevřít, `notes_append` /
  `notes_create` odmítnou nový `[x]` (test `model_can_never_write_done_over_mcp`); potvrzuje člověk v Ledge
  (`note_task_set`). `tools/list` neposílá `title` (šetří místo; jméno stačí).
- **Zápis = stejná cesta jako Uložit v Nastavení:** načíst config → změna → `config::validate_field` (stejná
  validace, i editoru rozložení) → snímek dotčených částí do Vrátit (posledních 20, jen v paměti) →
  `save_config` → událost `config-changed`. Neplatný vstup = výsledek s `isError: true` a větou, co je špatně a
  co jde (config beze změny); chyba protokolu (neznámý nástroj, špatná zpráva) = JSON-RPC error. `undo_last`
  vrátí jen části, které ten zápis změnil, a odmítne, když je mezitím někdo změnil jinde.
- **Neuložené úpravy v Nastavení:** zápis přes MCP projde vždy. Otevřené Nastavení dostane událost: bez
  rozpracovaných změn se přenačte; rozpracované v jiné sekci zůstanou a změna zvenku se do nich sloučí; tatáž
  sekce = lišta konfliktu „Načíst nové / Přesto uložit“ (nic se nepřepíše potichu, `src/mcp/merge.ts`).
- **Čtení stavu pro jiné nástroje (`live_state`, kolo 34, `mcp/read.rs`):** jen čtení, volba `mcp.readData` (Nastavení →
  Claude → Claude (MCP) → „Dát Claudovi číst stav Ledge“, výchozí **vypnuto**, platí jen se zapnutým MCP). Bez volby
  nástroj není v `tools/list` a volání vrátí `isError`. Přenos a klíč jsou stávající (`POST /mcp`, 127.0.0.1,
  `Authorization: Bearer <klíč MCP>`, bez `Origin`) — žádný druhý server; skript volá `tools/call` `live_state` přímo
  (`{"part":"limits"}`; části `sessions`, `limits`, `agenda`, `approvals`, `devServers`, `briefs`, bez `part` všechny).
  Pole: sessions `{id, project, folder, status, since, updatedAt, ended, endReason, alive, uncommittedCount, unpushed,
  subagentsRunning, activity{phase, tool, at}}`; limits `{at, source, staleMs, fiveHour{pct, resetsAt}, sevenDay{…}}`; agenda =
  dnešní události `{start, end, allDay, title, ongoing}`; approvals `{questions, pending[{tool, folder, createdAt,
  expiresAt}]}`; devServers `{id, phase, port, url, startedAt, readyAt, exitCode, errors, errorLines}` (`errorLines` už
  maskované v `procman`); briefs `{waiting, review}` (jen počty). **Nevystaví se:** titulky a jména sessions (text promptů),
  cesty (`cwd`, soubory, `configPath`), příkazy a shrnutí schválení, poslední zpráva/soubor aktivity, umístění a účastníci
  událostí, odkazy na schůzky, `cmd`/`tail`/`lastLine` procesů, názvy a obsah poznámek, e-maily. Pravidlo drží **povolený
  seznam** `read::ALLOW` (default deny: nové pole zdroje je skryté, dokud ho někdo nepřidá do seznamu i do tabulky v
  `read::tests::exposed_table`). Nic nepotvrzuje ani nespouští; `GET /status` (širší, s cestou configu) zůstává jen pro
  klíč hooků. Nepřidávej nástroj, který schvaluje.
- **MCP nic nespouští** (žádné launch/run nástroje). Akce `command` / `app` (i bez argumentů) přidaná přes MCP souhlas
  nedostane — Ledge se zeptá při prvním spuštění jako u ruční. Zástupce (karta Projekty) se spouští bez
  potvrzení, proto mu MCP smí dát jen skript balíčkovače (`npm run dev`, `pnpm dev`…) nebo příkaz, který Ledge
  sám najde ve složce projektu; jiný příkaz → rychlá akce `command`.

**Schválení přes API záměrně NEJDE.** Povolit akci Clauda smí jen člověk — klikem v šuplíku (nebo
zkratkou, je-li zapnutá). Nepřidávej `/ctl/decide` ani nic podobného: z mostu by se stal způsob, jak si agent
schválí sám sebe.

## Izolovaná instance pro QA (L295)

Druhý Ledge vedle nainstalovaného, pro živé ověření bez šéfa (`src-tauri/src/qa.rs`). Jen env, v UI ani configu nic není:

- `LEDGE_ISOLATED=1` — bez zámku jedné instance; nezapíše nic mimo svou složku (registr Run, Správce pověření, `~/.claude`/`settings.json`, motiv Windows, instalace aktualizace = vypnuto); bez ikony v oznamovací oblasti, hover vlákna (myš, globální hook) a nativního proužku; **schvalování akcí Clauda a odpovědi na otázky nefungují** (`/ctl/*`, zkratky ani UI). Složky nasměruj proměnnými `LEDGE_CONFIG_DIR`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `CLAUDE_CONFIG_DIR` (`dirs.rs`) a port mostu v configu `claude.port`; v configu sandboxu vypni `hotkeys.enabled`.
- `LEDGE_CDP_PORT=<1024–65535>` — platí jen s izolací (jinak se ignoruje): všechna okna dostanou `--remote-debugging-port` (+ vypnutou okluzi) a parkují se na `x − 30000` mimo plochu, bez fokusu; připoj `connectOverCDP("http://127.0.0.1:<port>")`. **Porty CDP vyžadují** (změřeno, WebView2 157): (1) vlastní složku dat WebView2 `<LEDGE_CONFIG_DIR>/webview2` (`qa::data_dir`, na všech oknech; výchozí složka se bere z known-folderu, ne z env, takže by instance sdílela data i proces prohlížeče s nainstalovaným Ledge), (2) **nepřepsaný `USERPROFILE`** (s cizím domovem WebView2 port neotevře; `APPDATA`/`LOCALAPPDATA` přepsat jde). Argumenty zůstávají produkční (`BROWSER_ARGS` + port + vypnutá okluze), příznaky CDP nerozbíjejí.
- `GET /status` → `qa`: `{isolated, hotkeys:[{action,key}], pins:{open,through}, desk:{open}, down:[vlákno…], cfgError, media:{live,playing}}` (skládá se při dotazu).
- Klíč hooků čte izolovaná instance ze sdíleného Správce pověření (jen čtení); nový nevytvoří.
- Scénář `upgrade` (`scripts/qa/upgrade.mjs`, `--old-exe`): dvě binárky po sobě nad jedním sandboxem; složka dat WebView2 `<LEDGE_CONFIG_DIR>/webview2` je u něj junction na `<LOCALAPPDATA><identifikátor binárky>` (postup a kroky v `docs/agents/install.md`).
- Scénář `propojeni` (Propojená aplikace): klíč `Ledge/connector` izolovaná instance nezapisuje (`vault.rs`), `connector_state.hasKey` ale čte skutečný Správce pověření; scénář proto ověřuje jen stavy bez klíče a falešný server (127.0.0.1) počítá požadavky.
