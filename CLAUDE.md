# Ledge — vstup pro Claude Code

Always-on „šuplík“ u horního okraje obrazovky (Windows; Tauri 2 + Rust v `src-tauri/src`, SolidJS v
`src`). Komunikuj česky.

**Začni levně:** [`docs/codemap/README.md`](docs/codemap/README.md) → mapa tvé oblasti → teprve pak
konkrétní soubory. Moduly jsou malé (většinou < 500 řádků) — čti jen ty, které mapa jmenuje.
[`AGENTS.md`](AGENTS.md) (config, formát poznámek, API, instalace) čti jen v potřebné sekci.
Historie kol a backlog nejsou v repu (interní předávka mimo strom).

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
- Ověření: `cd src-tauri && cargo test`, `cargo clippy --all-targets`, `npx tsc --noEmit`,
  `corepack pnpm build`. Vydání: `powershell -File scripts/publish.ps1` (viz `AGENTS.md`).
