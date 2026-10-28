# Codemapy Ledge — kde co je

Každá mapa je **jádro ≤ 8 KB** (≈ 2–4k tokenů): tabulka soubor → jeden řádek s účelem a klíčovými symboly.
Delší mapy mají vedle sebe `<oblast>-detail.md` (toky, invarianty, testy, co nečíst, historie vln a kol, plné
popisy řádků) — čti ho jen, když jádro nestačí. Mapy citují symboly, ne čísla řádků — `grep -n <symbol>` tě
dovede přesně. Když měníš odpovědnost souboru nebo přidáváš modul, oprav řádek v jádru mapy (součást hotové práce).
Tento přehled je zkrácený; plná verze je v [README-detail.md](README-detail.md).

| oblast | mapa | klíčová slova (soubory) |
|---|---|---|
| Claude session, subagenti, otevření, hooky | [sessions.md](sessions.md) | `sessions.rs` `sessionlive.rs` `winfocus.rs` `subagents.rs` `hooksetup.rs` `claude/*` `src/ClaudeSessions.tsx` |
| Limity Claude 5h / týden: wrapper stavového řádku, zapojení, připomínka resetu | [limits.md](limits.md) | `limits.rs` `limitslive.rs` `statusline.rs` `statuslinesetup.rs` `commands/limits.rs` `src/limits/*` |
| Útrata Claude (odhad v cenách API): transcripty, inkrementální index, ceník, sekce v kartě … | [costs.md](costs.md) | `usage/*` `costslive.rs` `commands/costs.rs` `src/costs/*` |
| Poznámky, kategorie, Markdown, výběry, Splnit · Archivovat · Smazat (Koš + Zpět), lístky … | [notes.md](notes.md) | `notes/*` `commands/noteslife.rs` `notesync/*` `pins/*` `src/notes/*` `src/pin/*` |
| Projekty, loga, klonování, výkon snapshotu, vzhled jednotlivých zástupců (ikona, barva … | [projects-icons-perf.md](projects-icons-perf.md) | `icons.rs` `iconimg.rs` `cloning.rs` `resolve.rs` `tilelook.rs` `src/logos.ts` `src/tilelook/*` |
| Nastavení: shell, panely, záloha, dialogy, strop výšky šuplíku | [settings.md](settings.md) | `src/Settings.tsx` `src/settings/*` `src/fit.ts` `configerr.rs` `configio.rs` `projectdetect.rs` |
| Rozložení šuplíku: záložky (i vlastní), karty se šířkou a výškou (tah za okraj, ⋯), řada … | [layout.md](layout.md) | `layout.rs` `tilelook.rs` `src/layout/*` `src/cards/CardMenu.tsx` |
| Vlastní SVG grafy: sparkline, prstenec, sloupce, pás (heat), číslo jako hrdina — bez … | [charts.md](charts.md) | `src/charts/*` |
| Showcase | [showcase.md](showcase.md) | `src/showcase/*` |
| Frontend (UI): skořápka, stránky, karty, náhled po najetí, morf, styly | [ui.md](ui.md) | `src/App.tsx` `src/MainPage.tsx` `src/cards/*` `src/hover/*` `src/api.ts` `src/types.ts` `src/motion.ts` |
| Rychlé akce a odkazy (config `actions`, odkaz = akce, převod `personal.links`, spouštění … | [actions.md](actions.md) | `actions.rs` `actionrun.rs` `actionicons.rs` `actionstate.rs` `src/actions/*` `src/palette/*` |
| Widgety na ploše („Na plochu“): karta Ledge jako okno na ploše / druhém monitoru, vrstvy … | [desk.md](desk.md) | `desk/*` `commands/desk.rs` `desk.html` `src/desk/*` |
| Widgety ze skriptů (jako xbar): složka `@ledge.widget`, parser JSON / textu, plánovač … | [widgets.md](widgets.md) | `widgets/*` `commands/widgets.rs` `src/widgets/*` |
| Vzhled a animace: design styly (tabulka, písmo, materiál, rozložení, šířka), motivy … | [appearance.md](appearance.md) | `appearance.rs` `appearance/{geom,look,stripdeco,motion}.rs` `looks/*` `stripdraw.rs` `threads/pointer.rs` |
| Průvodce prvním spuštěním, čistý config nového uživatele, krok „Claude“ (návod pro Clauda … | [onboarding.md](onboarding.md) | `onboarding/*` `uiconf.rs` `commands/onboarding.rs` `src/onboarding/*` `src/settings/panels/Start.tsx` |
| Přihrádka na soubory: tah na proužek (OLE cíl), `shelf.json`, schránka, tah ven, karta | [shelf.md](shelf.md) | `shelf/*` `commands/shelf.rs` `src/shelf/*` |
| Historie schránky v paletě (`v:`): listener událostmi, filtr hesel a tokenů, připnuté … | [clipboard.md](clipboard.md) | `cliphist/*` `commands/cliphist.rs` `src/cliphist/*` |
| Osobní: kalendář — zdroje (ICS, Outlook v PC, místo pro účty), parser, opakování, pásma … | [calendar.md](calendar.md) | `calendar/*` `commands/calendar.rs` `src/calendar/*` |
| Účty Microsoft / Google (OAuth PKCE, Správce pověření), kalendář účtů, karta Pošta … | [accounts.md](accounts.md) | `accounts/*` `calendar/graph.rs` `calendar/google.rs` `calendar/source.rs` `commands/accounts.rs` |
| Osobní: právě hraje (GSMTC, průběh skladby, 5 vzhledů karty), hlasitost (Core Audio) … | [media.md](media.md) | `media.rs` `mediatime.rs` `mediaconf.rs` `volume.rs` `battery.rs` `commands/media.rs` `src/media/*` |
| Kontextové návrhy: pruh nahoře v šuplíku (max 3 chipy z reálných událostí: připomínka, PR … | [suggest.md](suggest.md) | `suggest/*` `attention.rs` `src/suggest/*` `src/types/suggest.ts` |
| Integrace: karta GitHub (`gh`, PR k revizi / vlastní PR s CI / notifikace), karta Systém … | [integrations.md](integrations.md) | `github/*` `sysmon/*` `integrations.rs` `commands/integrations.rs` `src/integrations/*` |
| MCP server pro Clauda: `/mcp` na mostu, JSON-RPC, nástroje (zástupci, akce, rozložení … | [mcp.md](mcp.md) | `mcp/*` `commands/mcp.rs` `src/mcp/*` |
| Chytřejší šuplík: Alt+1…9 a šipky, přetažení odkazu / souboru na otevřený šuplík = nový … | [smart.md](smart.md) | `smartconf.rs` `approvalsound.rs` `incidents.rs` `dataexport.rs` `commands/smart.rs` `shelf/shortcut.rs` |
| Nové widgety kola 22: Claude dnes (útrata + časová osa po hodinách), Týden (pás dnů + … | [board.md](board.md) | `src/board/*` `usage/index.rs` `hours` `usage/view.rs` `hour_rows` `costslive.rs` `wanted` `request_desk` |
| Karta Rychlé dlaždice Windows: mikrofon, výstup zvuku, terminál ve složce projektu … | [winquick.md](winquick.md) | `winquick.rs` `winquickwin.rs` `commands/winquick.rs` `winsettings.rs` `src/winquick/*` |
| Jádro: stav, vlákna, příkazy, okna (vč. náhledu po najetí `drawer.hover`), proužek … | [core.md](core.md) | `lib.rs` `state.rs` `snapshot.rs` `threads/*` `commands/*` `drawer.rs` `hovermode.rs` `strip.rs` `edge.rs` |

Mimo mapy: [`AGENTS.md`](../../AGENTS.md) = rozcestník k `docs/agents/*.md` (config, formát poznámek, lokální API,
MCP, instalace, pravidla) — čti jen potřebný soubor.

## Najdi symbol

```
node scripts/kde.mjs <symbol|slovo>   # definice (soubor:řádek) + které mapy / docs/agents ho zmiňují
node scripts/kde.mjs --file <cesta>   # řádek mapy souboru + osnova deklarací s čísly řádků (bez těl)
```

Mapy hlídají testy v `test:ui` (`node scripts/codemap-lint.mjs` vypíše všechny nálezy najednou):
- `scripts/codemap.test.mjs` — soubory ve sloupci „soubor“ musí existovat (`x.rs` ≙ `x/mod.rs`, `x.ts` ≙
  `x/index.ts`), identifikátory ve „klíčových symbolech“ musí být v kódu (mapa nelže).
- `scripts/codemap-coverage.test.mjs` — každý modul (první úroveň `src-tauri/src` a `src`) i každý produkční
  zdroj (`.rs .ts .tsx .mjs`) musí být v některé mapě **jmenován** (holé `složka/*` na soubor nestačí;
  zkrácený zápis `oauth.rs` platí jen v mapě, která modul jmenuje celou cestou). Nový soubor tedy bez řádku
  v mapě neprojde. Výjimky s důvodem: `COVERAGE_ALLOW` v `scripts/codemap-coverage.mjs` (teď žádné).
- `scripts/config-keys.test.mjs` — každý klíč `config.json` (z Rust struktur) musí být v `docs/agents/*.md`.

## Ověření

```
node scripts/brany.mjs            # test, clippy, tsc, ui, build — jeden řádek na bránu, log jen při selhání
node scripts/brany.mjs test ui    # jen vybrané
node scripts/brany.mjs --changed  # jen to, co změněné soubory vyžadují (src → tsc ui; src-tauri → test clippy; docs → ui)
```

Ručně postaru: `cd src-tauri && cargo test` (+ `cargo clippy --all-targets`), `npx tsc --noEmit`,
`corepack pnpm test:ui`, `corepack pnpm build`. Soubory čti přes Read/Grep s rozsahem, ne `cat` celého souboru.

Těžké běhy na sdíleném stroji přes frontu (`node %USERPROFILE%\.devtools\fronta.mjs --auto -- …`),
paralelní workeři se sdíleným `CARGO_TARGET_DIR=…/src-tauri/target-agents`.
