# Codemapa: showcase — snímky a GIFy Ledge (kolo 24)

Obrázky a GIFy na GitHubu a webu jsou **skutečné UI Ledge vyrenderované headless s ukázkovými daty** na vlastní
tapetě — žádná fotka obrazovky, žádný `ledge.exe`. Šuplík = `index.html?demo=1`, Studio = `settings.html?demo=1`,
scéna `scripts/showcase/stage.html` je složí na abstraktní pozadí. Jedním příkazem a idempotentně:
`node scripts/showcase/shoot.mjs [--only id,id] [--list]` (fronta: `fronta.mjs --auto -- node scripts/showcase/shoot.mjs`).
Potřebuje `playwright-core` (devDependency; jinak `PLAYWRIGHT_CORE=<složka node_modules>`) a nainstalovaný Edge.
Fixture: jen smyšlená jména a `example.com`; v Tauri se demo nikdy nezapne (`setDemoHost` tam nic neudělá).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/showcase/demo.ts` | ukázkový hostitel šuplíku: URL parametry (set, style, theme, page, cards, ask, nopending, w, open), `window.__demo` pro Playwright, `report_size` → `postMessage` rodiči | `installShowcase` |
| `src/showcase/fixtures.ts` | kolo 25, doplňková ukázková data pro přehlídku vzhledů: počasí s předpovědí (`weatherFull`, den / noc / déšť), útrata 14 dní (`costsFull`), odběr systému s historií (`systemFull`); zapíná je `?full=1` | `weatherFull` `costsFull` `systemFull` |
| `scripts/showcase/designs.mjs` | kolo 25, kontaktní archy PNG do `shots-k25/` (necommitovat): každý vzhled karet Claude / Systém / Počasí / Týden × 3 šířky + prázdné stavy, pozadí šuplíku s kartami, float / notch; `--only cards\|empty\|backdrops` | |
| `src/showcase/sets.ts` | tři sady dat (studio, creator, minimal): projekty, session + schválení, otázka, limity, poznámky, počasí, kalendář, skladba — čas relativně k `now`, šířka šuplíku (`drawerWidth`) — test `sets.test.mjs` | `buildSet` `askFor` `SET_IDS` `COVER` `drawerWidth` |
| `src/showcase/stage.tsx` · `stage.css` | scéna: tapeta, proužek (`StripLookPreview`), iframe šuplíku / Studia; scény drawer · closed · open · studio · pair · strip | `Strip` `Drawer` `ClosedGallery` `OpenGallery` `PairScene` `StripScene` |
| `scripts/showcase/stage.html` | vstup scény pro Vite (`/scripts/showcase/stage.html?scene=…`) | |
| `scripts/showcase/shoot.mjs` | spustí Vite na volném portu, Edge (`channel: "msedge"`), nasnímá / nahraje vše ze `shots.mjs`, zabalí; `browser.close()` ve `finally` | |
| `scripts/showcase/pw.mjs` | sdílený `loadPlaywright(root)` (shoot.mjs, qa.mjs) | `loadPlaywright` |
| `scripts/qa.mjs` | QA před vydáním (L295, L296): izolovaná instance (`LEDGE_ISOLATED`, `LEDGE_CDP_PORT`) v sandboxu, CDP + `/status.qa`, zpráva PASS/FAIL/SKIP; `node scripts/qa.mjs [--only] [--list] [--keep]`, kód 0/1/2 (2 i při WebView2 s `--user-data-dir` mimo sandbox, `checkDataDir`) | |
| `scripts/qa/lib.mjs` · `scenarios.mjs` · `run.mjs` · `migrace.mjs` · `upgrade.mjs` | čisté funkce (`rawKeys`, fixture, verdikt, zpráva) · scénáře `supik` `listky-l260` `kit` `hudba` `nastaveni` + `konzole` · I/O (pojistky, sandbox, start, API, CDP; `ticho.ps1` `hraje.ps1`; `upgrade` = 2 binárky po sobě, `--old-exe`); test `scripts/qa.test.mjs` | `rawKeys` `checkField` `settleStep` `dataDirVerdict` `RUNNERS` `makeSandbox` |
| `scripts/showcase/shots.mjs` | seznam snímků a GIFů (`SHOTS`): scéna, velikost, cíl v `docs/screenshots`, `docs/media`, `site/img`, `site/media` | `SHOTS` |
| `scripts/showcase/pack.py` | komprese: PNG (paleta), WebP, GIF z framů (ffmpeg od Playwrightu GIF neumí), Pillow | |
| `src/api.ts` | `emitDemo` — sběrnice událostí jen pro demo (mimo Tauri; `on` na ni poslouchá jen s demo hostitelem) | `emitDemo` `setDemoHost` |

## Pravidla

- Srovnávací galerie (`looks-closed`, `looks-open`, GIF styles / themes / strip) drží JEDNU sadu (`studio`) — mění se jen vzhled.
- Výstupy: PNG ≤ 400 kB, GIF ≤ 3 MB, `site/media/*.webm` (VP9) pro web; pod snímky vždy „ukázková data“.
- Nic z toho neběží v klidu aplikace: demo načte jen `index.tsx` při `?demo=1` mimo Tauri (dynamický import).
