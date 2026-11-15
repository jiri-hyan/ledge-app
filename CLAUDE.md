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
| Nový příkaz Tauri | `src-tauri/src/commands/<oblast>.rs` + `mod` v `commands/mod.rs` + `generate_handler!` v `lib.rs` + ACL `allow-<příkaz>` v `capabilities/*.json` (a `ipcacl.rs` tabulka okna, např. `PIN_WINDOW`) + volání v `src/api.ts` (nebo `src/<oblast>/api.ts`) — [core.md](docs/codemap/core.md) |
| Text / překlad | `t("klíč", "česky")` + angličtina v `src/i18n/en-<oblast>.ts` (nový slovník = import v `dicts.ts`); test `src/i18n.test.mjs` — [ui-detail.md](docs/codemap/ui-detail.md) (i18n) |
| Panel nebo volba v Nastavení | `settings/registry.ts` (panel) · `catalog.ts` · `panels/*.tsx` · popisky `settings/labels.ts` + `settings/en.ts` — [settings.md](docs/codemap/settings.md) |
| Nové rozšíření (`ledge.exe --ext <id>`) | `ext/mod.rs` `REGISTRY` + tělo v `ext/child.rs` `body_for` — [ext.md](docs/codemap/ext.md) |
| Nový motiv / paleta (3 místa) | `src/theme/palettes.json` + `src/theme/themes.ts` + varianta enumu `Theme` v `appearance.rs` + EN název `appearance.theme.<id>` — [appearance-detail.md](docs/codemap/appearance-detail.md) „Nový motiv“ |
| Nový hover režim / pohled | `View` v `drawer.rs` + `hovermode.rs` + `src/hover/modes.ts` (`HOVER_MODES`) + větev `SmallView.tsx` (+ `smallWidth`) — [hover.md](docs/codemap/hover.md) |
| Nová živá služba (zdroj dat bez pollingu) | vlákno / odběr v `threads/*` (vzor `costslive.rs`) → snapshot (`snapshot.rs`) → UI čte jen snapshot; má-li být „dění“: tabulka `src/hover/priority.ts` `PRIORITY` + zrcadla `suggest/model.ts` `pick` a `attention.rs` `Event::rank` — [hover.md](docs/codemap/hover.md), [core.md](docs/codemap/core.md) |
| Nový modul / soubor | řádek v mapě své oblasti ([README](docs/codemap/README.md)); bez něj spadne `scripts/codemap-coverage.test.mjs` |

## Nepřekročitelná pravidla

- Ledge **nekrade fokus** (jen výslovný klik) a při fullscreen hře je mrtvý.
- Před jakýmkoli GUI testem (myš, okna, screenshoty) ověř, že uživatel nehraje (`SHQueryUserNotificationState`
  3/4 → nespouštět). Automatizace bez oken: `LEDGE_NO_GUI=1`.
- GUI testy nesahají na cizí okna ani plochu: žádné Win+D / `ToggleDesktop`, minimalizace, přesun, fokus, `SendInput`
  mimo vlastní okno, změna tapety. Snímky jen vlastního okna nebo malého obdélníku pod ním, nikdy celé obrazovky
  (soukromý obsah). Viditelná zkouška jen v místě plochy, které žádné okno nezakrývá (sonda), krátce a malým oknem.
- Povolit akci Clauda smí jen člověk — žádné `/ctl/decide` ani obdoba.
- Testy nikdy nesahají na skutečné `~/.claude`, registr, `%APPDATA%\Ledge`, `%USERPROFILE%\Ledge`.
- Výkonový rozpočet: idle CPU < 0,1 %, `ledge.exe` < 25 MB private; měř (`scripts/perf.ps1`), neodhaduj.
- Žádné divadlo: co nevíš z reálného zdroje, ukaž prázdné.

## Práce

- Commity česky, Conventional Commits, explicitní `git add <cesty>`, nikdy `--no-verify`.
- Nová logika = čistá funkce + tabulkový test; moduly pod ~600 řádků produkčního kódu (bez `#[cfg(test)] mod`; měří `node scripts/modsize.mjs`); při změně oblasti oprav její
  codemapu.
- Před mergem víc větví: `node scripts/premerge.mjs <větev…>` (kolize přes `git merge-tree`, překryv horkých souborů, jádra codemap nad 8 KB po mergi; `--tsc` navíc pustí `tsc --noEmit` nad zkušebním sloučením v dočasném worktree a chytí zdvojené typy/klíče).
- Ověření: `node scripts/brany.mjs --changed` (rychlá brána podle změněných souborů; změna `src-tauri/**` přidá i `ui`;
  po non-ff mergi `--changed --since HEAD~1`; na čistém stromu bez souborů varuje a končí kódem 2, záměr `--allow-empty`;
  `cargo test` běží pod zámkem `<CARGO_TARGET_DIR>/.brany-cargo.lock`, LNK1104 = 1× znovu), plně `node scripts/brany.mjs`
  (= `cargo test`, `cargo clippy --all-targets`, `npx tsc --noEmit`, `test:ui` vč. testu hniloby codemap,
  `corepack pnpm build`). Vydání: `powershell -File scripts/publish.ps1` (viz `docs/agents/install.md`).

## Workeři a kola

- Worker má větev `k<kolo>/<oblast>` (worktree `.claude/worktrees/<oblast>` uvnitř repa); necommituje mimo ni a nepushuje.
- Lead merguje non-ff: `merge: <popis> (kolo N)`. Worker na konci aktualizuje codemapu své oblasti
  (`scripts/codemap.test.mjs` hlídá, že neodkazuje na neexistující soubory ani symboly, `scripts/codemap-coverage.test.mjs`, že každý modul a zdroj má řádek; `node scripts/codemap-lint.mjs` vypíše obojí).
- Sdílený `CARGO_TARGET_DIR=src-tauri/target-agents`. Orientace: `node scripts/kde.mjs <symbol>`.
- Worktree: `node scripts/wt.mjs new <oblast> <kolo>` (větev, junction `node_modules`, kopie `dist`, rerere) a
  `rm <oblast>` (nejdřív odpojí junction, jinak by se smazal i obsah cíle; bez `--force`); `prune` čistí starou
  incremental cache (dry-run, mazání jen s `--apply`). Plné brány se při nezměněném stromu nespustí znovu (`.brany-ok`, `--force`).
