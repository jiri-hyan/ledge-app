# Ledge — vstup pro Claude Code

Always-on „šuplík“ u horního okraje obrazovky (Windows; Tauri 2 + Rust v `src-tauri/src`, SolidJS v
`src`). Komunikuj česky.

**Začni levně:** [`docs/codemap/README.md`](docs/codemap/README.md) → mapa tvé oblasti → teprve pak
konkrétní soubory. Moduly jsou malé (většinou < 500 řádků) — čti jen ty, které mapa jmenuje.
Symbol nebo slovo najdeš jedním příkazem: `node scripts/kde.mjs <symbol>` (definice `soubor:řádek` + které
mapy ho zmiňují); `node scripts/kde.mjs --file <cesta>` dá řádek mapy souboru + osnovu deklarací s čísly řádků (místo čtení celého souboru). Mapa = jádro ≤ 8 KB; toky, invarianty a testy jsou v `docs/codemap/<oblast>-detail.md`. [`AGENTS.md`](AGENTS.md) je rozcestník k `docs/agents/*.md` (config, formát poznámek,
API/MCP, instalace) — čti jen potřebný soubor, ne celý.
Historie kol a backlog nejsou v repu (interní předávka mimo strom).

## Kde začít podle úkolu

| Úkol | Začni tady |
|---|---|
| Nová karta | `layout.rs` `CARDS` + `src/layout/model.ts` `CARD_IDS`/`CARD_INFO` + větev `MainPage.tsx` `card` — [layout-detail.md](docs/codemap/layout-detail.md) (Invarianty) |
| Nový config klíč | struct v `src-tauri/src/config/*` (serde default + `JsonSchema`) → zrcadlo `src/types/*.ts` → zápis do `docs/agents/config*.md` (hlídá `scripts/config-keys.mjs`) → editor v Nastavení → test se starým configem — [core.md](docs/codemap/core.md) `config/*` |
| Nový příkaz Tauri | `src-tauri/src/commands/<oblast>.rs` + `mod` v `commands/mod.rs` + `generate_handler!` v `lib.rs` + volání v `src/api.ts` (nebo `src/<oblast>/api.ts`) — [core.md](docs/codemap/core.md) |
| Text / překlad | `t("klíč", "česky")` + angličtina v `src/i18n/en-<oblast>.ts` (nový slovník = import v `dicts.ts`); test `src/i18n.test.mjs` — [ui-detail.md](docs/codemap/ui-detail.md) (i18n) |
| Panel nebo volba v Nastavení | `settings/registry.ts` (panel) · `catalog.ts` · `panels/*.tsx` · popisky `settings/labels.ts` + `settings/en.ts` — [settings.md](docs/codemap/settings.md) |
| Nový modul / soubor | řádek v mapě své oblasti ([README](docs/codemap/README.md)); bez něj spadne `scripts/codemap-coverage.test.mjs` |

## Nepřekročitelná pravidla

- Ledge **nekrade fokus** (jen výslovný klik) a při fullscreen hře je mrtvý.
- Před jakýmkoli GUI testem (myš, okna, screenshoty) ověř, že uživatel nehraje (`SHQueryUserNotificationState`
  3/4 → nespouštět). Automatizace bez oken: `LEDGE_NO_GUI=1`.
- Povolit akci Clauda smí jen člověk — žádné `/ctl/decide` ani obdoba.
- Testy nikdy nesahají na skutečné `~/.claude`, registr, `%APPDATA%\Ledge`, `%USERPROFILE%\Ledge`.
- Výkonový rozpočet: idle CPU < 0,1 %, `ledge.exe` < 25 MB private; měř (`scripts/perf.ps1`), neodhaduj.
- Žádné divadlo: co nevíš z reálného zdroje, ukaž prázdné.

## Práce

- Commity česky, Conventional Commits, explicitní `git add <cesty>`, nikdy `--no-verify`.
- Nová logika = čistá funkce + tabulkový test; moduly pod ~600 řádků; při změně oblasti oprav její
  codemapu.
- Ověření: `node scripts/brany.mjs --changed` (rychlá brána podle změněných souborů), plně `node scripts/brany.mjs`
  (= `cargo test`, `cargo clippy --all-targets`, `npx tsc --noEmit`, `test:ui` vč. testu hniloby codemap,
  `corepack pnpm build`). Vydání: `powershell -File scripts/publish.ps1` (viz `docs/agents/install.md`).

## Workeři a kola

- Worker má větev `k<kolo>/<oblast>` (worktree `ledge-wt-<oblast>`); necommituje mimo ni a nepushuje.
- Lead merguje non-ff: `merge: <popis> (kolo N)`. Worker na konci aktualizuje codemapu své oblasti
  (`scripts/codemap.test.mjs` hlídá, že neodkazuje na neexistující soubory ani symboly, `scripts/codemap-coverage.test.mjs`, že každý modul a zdroj má řádek; `node scripts/codemap-lint.mjs` vypíše obojí).
- Sdílený `CARGO_TARGET_DIR=src-tauri/target-agents`. Orientace: `node scripts/kde.mjs <symbol>`.
- Worktree past: `node_modules` je junction — před `git worktree remove` nejdřív
  `cmd //c rmdir <wt>\node_modules`, jinak se smaže i obsah cíle.
