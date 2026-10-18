# Codemapy Ledge — kde co je

Přečti si mapu své oblasti (≈1–2k tokenů) **místo** celých souborů. Mapy citují symboly, ne čísla
řádků — `grep -n <symbol>` tě dovede přesně. Když měníš odpovědnost souboru nebo přidáváš modul,
oprav i jeho mapu (je to součást hotové práce).

| oblast | mapa | hlavní soubory |
|---|---|---|
| Claude session, subagenti, otevření, hooky | [sessions.md](sessions.md) | `sessions.rs`, `sessionlive.rs`, `winfocus.rs`, `subagents.rs`, `hooksetup.rs`, `claude/*`, `src/ClaudeSessions.tsx` |
| Limity Claude 5h / týden: wrapper stavového řádku, zapojení, připomínka resetu | [limits.md](limits.md) | `limits.rs`, `limitslive.rs`, `statusline.rs`, `statuslinesetup.rs`, `commands/limits.rs`, `src/limits/*` |
| Útrata Claude (odhad v cenách API): transcripty, inkrementální index, ceník, sekce v kartě Claude | [costs.md](costs.md) | `usage/*` (parse, price, index, view), `costslive.rs`, `commands/costs.rs`, `src/costs/*` |
| Poznámky, kategorie, Markdown, výběry | [notes.md](notes.md) | `notes/*` (md, edit, store, frontmatter…), `notesync/*`, `src/notes/*` |
| Projekty, loga, klonování, výkon snapshotu | [projects-icons-perf.md](projects-icons-perf.md) | `icons.rs`, `iconimg.rs`, `cloning.rs`, `resolve.rs`, `src/logos.ts` |
| Nastavení: shell, panely, záloha, dialogy, strop výšky šuplíku | [settings.md](settings.md) | `src/Settings.tsx`, `src/settings/*`, `src/fit.ts`, `configerr.rs`, `configio.rs`, `projectdetect.rs`, `drawerfit.rs`, `commands/settings.rs` |
| Rozložení šuplíku: záložky (i vlastní), karty se šířkou a výškou (tah za okraj, ⋯), řada bez děr, zástupci (karta Projekty), režim úprav, katalog, prohlídka prvního spuštění, migrace starého configu | [layout.md](layout.md) | `layout.rs`, `src/layout/*`, `src/cards/CardMenu.tsx` |
| Frontend (UI): skořápka, stránky, karty, náhled po najetí, morf, styly | [ui.md](ui.md) | `src/App.tsx`, `src/MainPage.tsx`, `src/cards/*`, `src/hover/*`, `src/api.ts`, `src/types.ts`, `src/motion.ts`, `src/styles.css` |
| Rychlé akce a odkazy (config `actions`, odkaz = akce, převod `personal.links`, spouštění, ikony, souhlasy) a paleta příkazů | [actions.md](actions.md) | `actions.rs`, `actionrun.rs`, `actionicons.rs`, `actionstate.rs`, `src/actions/*`, `src/palette/*`, `src/cards/ActionsCard.tsx` |
| Vzhled a animace: design styly (tabulka, písmo, materiál, rozložení, šířka), motivy, tokeny, akcent, animace otevírání, vzhled a pozice proužku, styly zavřeného Ledge (`strip.look`), panel Vzhled | [appearance.md](appearance.md) | `appearance.rs`, `appearance/{geom,look,stripdeco,motion}.rs`, `looks/*`, `stripdraw.rs`, `src/strip/looks.ts`, `src/theme/*`, `src/settings/panels/Appearance*.tsx` |
| Průvodce prvním spuštěním, čistý config nového uživatele | [onboarding.md](onboarding.md) | `onboarding/*`, `uiconf.rs`, `commands/onboarding.rs`, `src/onboarding/*`, `src/settings/panels/Start.tsx` |
| Přihrádka na soubory: tah na proužek (OLE cíl), `shelf.json`, schránka, tah ven, karta | [shelf.md](shelf.md) | `shelf/*`, `commands/shelf.rs`, `src/shelf/*` |
| Historie schránky v paletě (`v:`): listener událostmi, filtr hesel a tokenů, připnuté, Nastavení | [clipboard.md](clipboard.md) | `cliphist/*`, `commands/cliphist.rs`, `src/cliphist/*` |
| Osobní: kalendář z odkazu ICS — parser, opakování, pásma, stahování, karta, editor | [calendar.md](calendar.md) | `calendar/*` (ics, expand, tz, view, conf, live), `commands/calendar.rs`, `src/calendar/*` |
| Osobní: právě hraje (GSMTC, průběh skladby, 5 vzhledů karty), hlasitost (Core Audio), baterie — události, žádný polling | [media.md](media.md) | `media.rs`, `mediatime.rs`, `mediaconf.rs`, `volume.rs`, `battery.rs`, `commands/media.rs`, `src/media/*` |
| Jádro: stav, vlákna, příkazy, okna (vč. náhledu po najetí `drawer.hover`), proužek, trigger, procesy | [core.md](core.md) | `lib.rs`, `state.rs`, `snapshot.rs`, `threads/*`, `commands/*`, `drawer.rs`, `hovermode.rs`, `strip.rs`, `edge.rs`, `screen.rs`, `procman.rs` |

Mimo mapy: `AGENTS.md` = config, formát poznámek, lokální API, instalace, pravidla (čti jen potřebnou
sekci).

## Ověření

```
node scripts/brany.mjs            # test, clippy, tsc, ui, build — jeden řádek na bránu, log jen při selhání
node scripts/brany.mjs test ui    # jen vybrané
```

Ručně postaru: `cd src-tauri && cargo test` (+ `cargo clippy --all-targets`), `npx tsc --noEmit`,
`corepack pnpm test:ui`, `corepack pnpm build`. Soubory čti přes Read/Grep s rozsahem, ne `cat` celého souboru.

Těžké běhy na sdíleném stroji přes frontu (`node %USERPROFILE%\.devtools\fronta.mjs --auto -- …`),
paralelní workeři se sdíleným `CARGO_TARGET_DIR=…/src-tauri/target-agents`.
