# Ledge — návod pro agenty (Claude Code a spol.)

Ledge je always-on „šuplík“ u horního okraje obrazovky: projekty (klik = dev server / appka),
běžící servery a schvalování pro Claude Code. Tenhle soubor říká, jak ho **nastavovat a ovládat**
bez UI. Architektura je v hlavičkách souborů `src-tauri/src/*.rs`.

**Dřív se jmenoval Notch** (do 0.1.0, přejmenováno 2026-10-17; veřejná vydání: `jiri-hyan/ledge-app`). Staré cesty `%APPDATA%\Notch`, `%USERPROFILE%\Notch\notes`,
`%LOCALAPPDATA%\Notch` už **nepoužívej** — viz [Migrace z Notche](docs/agents/install.md#migrace-z-notche).

**Tenhle soubor je jen rozcestník.** Detail je v `docs/agents/*.md` (obsah doslovně z dřívějšího
jednoho velkého AGENTS.md). Čti jen soubor, který potřebuješ; orientaci v kódu dá
[`docs/codemap/README.md`](docs/codemap/README.md) a `node scripts/kde.mjs <symbol>`. Odkazy v textu
typu „viz sekce …“ míří na sekce v souborech níže.

## Kde co je

| Téma | Soubor | Co tam je |
|---|---|---|
| Konfigurace: základ, projekty | [config.md](docs/agents/config.md) | `config.json` (cesty, Nastavení, čistý start), `projects`, logo, `look` dlaždice, menu, start/stop serverů, klonování |
| Konfigurace: rozložení | [config-layout.md](docs/agents/config-layout.md) | `layout` (záložky, karty, zástupci, složky), `drawer.hover`, `strip.items`, `strip.look` |
| Konfigurace: šuplík a Claude | [config-drawer.md](docs/agents/config-drawer.md) | `drawer` (spouštěč, prodlevy, `keepWarmS`, `barWindows`), `claude.approvalTimeoutS` / `enforceCommit`, `personal.weather.lat/lon`, `projects[].subdir`, `$schema` |
| Konfigurace: osobní | [config-personal.md](docs/agents/config-personal.md) | `timer`, `personal.calendars`, `accounts` (OAuth), `claude.limitsReminder`, `claude.costs` |
| Konfigurace: vzhled a chování | [config-behavior.md](docs/agents/config-behavior.md) | `appearance`, zkratky, paleta, upozornění Clauda, herní režim, `shelf`, `desk` (widgety na ploše), `clipboard`, `media`, `scripts`, `widgets`, `mcp` |
| Média a integrace | [media-integrations.md](docs/agents/media-integrations.md) | Právě hraje, hlasitost, baterie; karty GitHub (`gh`) a Systém |
| Rychlé akce, skripty, widgety | [actions.md](docs/agents/actions.md) | `actions` (kinds, souhlasy), okno s webem, odkazy = akce, Script Commands, widgety ze skriptů |
| Poznámky | [notes.md](docs/agents/notes.md) | formát souboru, kategorie, Splnit/Archivovat/Smazat, lístky na plochu, synchronizace `notes.sync` |
| Lokální API a MCP | [api-mcp.md](docs/agents/api-mcp.md) | HTTP most `127.0.0.1:47800`, `/mcp` (nástroje, zápis configu), proč schválení přes API nejde |
| Session, hooky, limity, útrata | [sessions.md](docs/agents/sessions.md) | karta Claude, otevření session, Agent View, Git panel, schvalování, plán, otázky Clauda (`claude.questionHoldS`, pojistka `claude.questionUnseenS`), hooky, limity 5h/týden, útrata |
| Instalace a aktualizace | [install.md](docs/agents/install.md) | `publish.ps1` / `release.ps1`, aktualizace z appky, NSIS/autostart, bublina MSIX, migrace z Notche |

Mapy kódu (kde co leží): [`docs/codemap/`](docs/codemap/README.md). Uživatelské návody (ne pro práci v kódu): `docs/pro-claude.md`
(instalace a ladění Ledge pro Clauda), `docs/ucty.md`, `docs/listky.md`, `docs/widgets.md`, `docs/widgety-na-plose.md`; vydavatel: `docs/podpis-vydani.md`
(podpis vydání, záloha a ztráta klíče).

Každý klíč `config.json` (odvozený z Rust struktur) musí být zmíněn v `docs/agents/*.md` — hlídá
`scripts/config-keys.mjs` (test `scripts/config-keys.test.mjs`).

## Pravidla

- Ledge **nikdy nekrade fokus** (kromě otevření zkratkou a výslovného kliku na vstup do poznámky —
  příkaz `focus_drawer`, nikdy z peeku, připomínky ani najetí) a při hře přes celou obrazovku je mrtvý.
  Neměň to.
- Automatizace bez GUI: `LEDGE_NO_GUI=1` (starší `NOTCH_NO_GUI=1` platí taky) → proces skončí dřív, než vytvoří jakékoli okno.
- Výkonový rozpočet (měř `scripts/perf.ps1` proti release buildu): idle CPU < 0,1 %, `ledge.exe`
  < 25 MB private, se WebView2 < 90 MB.
- Testy: `cd src-tauri && cargo test` (spouští reálné procesy a porty), `pnpm typecheck`, `pnpm build`.
