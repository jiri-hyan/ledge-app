# Codemapy Ledge — kde co je

Přečti si mapu své oblasti (≈1–2k tokenů) **místo** celých souborů. Mapy citují symboly, ne čísla
řádků — `grep -n <symbol>` tě dovede přesně. Když měníš odpovědnost souboru nebo přidáváš modul,
oprav i jeho mapu (je to součást hotové práce).

| oblast | mapa | hlavní soubory |
|---|---|---|
| Claude session, subagenti, otevření, hooky | [sessions.md](sessions.md) | `sessions.rs`, `sessionlive.rs`, `winfocus.rs`, `subagents.rs`, `hooksetup.rs`, `claude/*`, `src/ClaudeSessions.tsx` |
| Limity Claude 5h / týden: wrapper stavového řádku, zapojení, připomínka resetu | [limits.md](limits.md) | `limits.rs`, `limitslive.rs`, `statusline.rs`, `statuslinesetup.rs`, `commands/limits.rs`, `src/limits/*` |
| Poznámky, kategorie, Markdown, výběry | [notes.md](notes.md) | `notes/*` (md, edit, store, frontmatter…), `notesync/*`, `src/notes/*` |
| Projekty, loga, klonování, výkon snapshotu | [projects-icons-perf.md](projects-icons-perf.md) | `icons.rs`, `iconimg.rs`, `cloning.rs`, `resolve.rs`, `src/logos.ts` |
| Nastavení: shell, panely, záloha, dialogy, strop výšky šuplíku | [settings.md](settings.md) | `src/Settings.tsx`, `src/settings/*`, `src/fit.ts`, `configerr.rs`, `configio.rs`, `projectdetect.rs`, `drawerfit.rs`, `commands/settings.rs` |
| Frontend (UI): skořápka, stránky, karty, morf, styly | [ui.md](ui.md) | `src/App.tsx`, `src/MainPage.tsx`, `src/cards/*`, `src/api.ts`, `src/types.ts`, `src/motion.ts`, `src/styles.css` |
| Rychlé akce (config `actions`, spouštění, ikony, souhlasy) a paleta příkazů | [actions.md](actions.md) | `actions.rs`, `actionrun.rs`, `actionicons.rs`, `actionstate.rs`, `src/actions/*`, `src/palette/*`, `src/cards/ActionsCard.tsx` |
| Vzhled a animace: motivy, tokeny, akcent, styl pohybu, vzhled a pozice proužku, panel Vzhled | [appearance.md](appearance.md) | `appearance.rs`, `appearance/geom.rs`, `src/theme/*`, `src/settings/panels/Appearance*.tsx` |
| Průvodce prvním spuštěním, čistý config nového uživatele | [onboarding.md](onboarding.md) | `onboarding/*`, `uiconf.rs`, `commands/onboarding.rs`, `src/onboarding/*`, `src/settings/panels/Onboarding.tsx` |
| Jádro: stav, vlákna, příkazy, okna, proužek, trigger, procesy | [core.md](core.md) | `lib.rs`, `state.rs`, `snapshot.rs`, `threads/*`, `commands/*`, `drawer.rs`, `strip.rs`, `edge.rs`, `screen.rs`, `procman.rs` |

Mimo mapy: `AGENTS.md` = config, formát poznámek, lokální API, instalace, pravidla (čti jen potřebnou
sekci).

## Ověření

```
cd src-tauri && cargo test        # + cargo clippy --all-targets
npx tsc --noEmit && corepack pnpm build
```

Těžké běhy na sdíleném stroji přes frontu (`node %USERPROFILE%\.devtools\fronta.mjs --auto -- …`),
paralelní workeři se sdíleným `CARGO_TARGET_DIR=…/src-tauri/target-agents`.
