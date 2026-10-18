# Codemapa: dlaždice projektů, loga, klonování, výkon snapshotu

Čti tohle místo celého jádra ([core.md](core.md)) a frontendu ([ui.md](ui.md)). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/icons.rs` | výběr loga projektu, cache výsledků | `resolve`, `Source`, `exe_policy`/`ExeIcon`, `candidates`, `load`, `raster_size`, `icon_links`, `manifest_icons`, `resolve_href`, `PATTERNS`, `Store` (`begin`/`finish`/`key`/`url`), `LIMIT`, `STALE` |
| `src-tauri/src/iconimg.rs` | rastry přes WIC: dekódování, největší snímek ICO, zmenšení ≤ 128 px, PNG; ikona z exe; ikona souboru/složky ze shellu (rychlé akce, [actions.md](actions.md)) | `to_png`, `exe_png`, `shell_png`, `fit`, `MAX_PX`, `encode` |
| `src-tauri/src/cloning.rs` | „Naklonovat“: kdy, kam, čím, spuštění | `clonable`, `target_in`, `folder_name`, `command`, `run`, `Busy`, `is_busy`, `TIMEOUT` |
| `src-tauri/src/resolve.rs` | repo index podle remote, appka podle jména | `RepoIndex::build`/`resolve`/`clones`, `parse_remote`, `owner_repo_raw`, `installed_app_exe` |
| `src-tauri/src/state.rs`, `refresh.rs`, `snapshot.rs`, `commands/projects.rs` | lepidlo | `Core.logos`, `Core::rebuild_index`, `refresh_logos`, `snapshot` (blok `projects`), `ProjectView`, `github_url`, příkazy `project_logo`, `clone_project`, `launch`, `open_in` → `open_dir` (sdílí panel Git session) |
| `src-tauri/src/onboarding/repos.rs` | návrh projektů z rep na disku (průvodce) — viz [onboarding.md](onboarding.md) | `discover`, `suggest` |
| `src/cards/Projects.tsx` · `ProjectTile.tsx` · `src/logos.ts` | karta (složky, tah v úpravách, menu pravého kliku, panel vzhledu) · jedna dlaždice (logo nebo `TileIcon`, akce uvnitř, ⋯ = Upravit vzhled) · cache log | `Projects` `startDrag` `applyTiles` · `ProjectTile` `launch` `tilt` `projectAuto` · `logoSrc`, `logoUrls` |
| `src/tilelook/*` | vzhled jednotlivého zástupce (vlna 8 P): čistá logika `model.ts` (normalizace, předvolby, `inkDark` kontrast, `withTileLook`, přehled; test `model.test.mjs`), `glyphs.ts` (77 ikon Segoe Fluent Icons ověřených v `SegoeIcons.ttf` i `segmdl2.ttf`, hledání), `state.ts` (který zástupce se upravuje, živý náhled `lookOf`, uložení jen `look`, obrázky z cache), `TileIcon.tsx` (kresba), `LookEditor.tsx` (panel pod dlaždicemi), `TileMenu.tsx` (pravý klik), `tilelook.css` (`tl-`), `en.ts` | `normalizeTileLook` `inkDark` `TILE_COLORS` `GLYPHS` `searchGlyphs` `lookOf` `openLook` `closeLook` `saveTileLook` `pickLookImage` `TileIcon` `LookEditor` `openTileMenu` |
| `src-tauri/src/tilelook.rs` · `commands/tilelook.rs` | typy a validace `look` (projekt, akce, karta, složky); vlastní obrázek → WIC (`iconimg::to_png`) → `%LOCALAPPDATA%\Ledge\looks\<fnv>.png` (SVG beze změny ≤ 512 KB); příkazy `look_image_add`, `look_image` | `import_image` `image_url` `key_ok` `glyph_ok` |
| `src/api.ts` | typy + IPC | `ProjectView` (`logoKey`, `clonable`, `cloning`), `api.projectLogo`, `api.cloneProject` |

## Tok loga

1. `Core::rebuild_index` (start, změna `roots`/`projects`, po klonu) → `refresh_logos(true)`;
   otevření šuplíku (`UiCmd::Show(Full)`) → `refresh_logos(false)` = nejvýš 1× za `icons::STALE` (5 min).
2. `refresh_logos` pod zámky jen sestaví `icons::Source` (složka, `logo`, exe/appName, politika),
   počítá vlákno `logos` → `Store::finish` (starší generace se zahodí) → `mark_dirty` jen při změně.
3. `snapshot` posílá jen `logoKey` (FNV hash obsahu); UI (`logoSrc`) si data URL vyžádá
   `project_logo` jednou na klíč. Base64 tedy nejde s každým stavem (emit až ~8×/s).

Pořadí v `icons::resolve`: `logo` z configu, když soubor existuje → `ExeIcon::First` (appName =
nainstalovaná appka, exe z registru) → `candidates` (deklarované v `index.html`/manifestu + `PATTERNS`,
seřazeno SVG > větší rastr do 128 px > deklarované; protáhlé obrázky = nápis, propadnou) →
`ExeIcon::Fallback` (vlastní exe, ne `electron.exe`).

## Invarianty a pasti

- **Vzhled zástupce** (`look`): chybí = dlaždice jde PŮVODNÍ cestou (`.logo` s přechodem barvy projektu) —
  `TileIcon` se kreslí jen pro neprázdný vzhled. Kresba na barevném podkladu je černá / bílá (`inkDark`,
  ≥ 4.5:1 pro každou barvu); obrázek, který by s podkladem splynul, dostane podložku (`useTone`). Snapshot
  nese `look` (malý), obrázek jen klíč → data URL `look_image` jednou na klíč (jako loga). Chybějící
  soubor v cache = Auto. Vybraný soubor se čte jen na klik (`pick_path` → `look_image_add`), nikdy periodicky.

- Loga se **nikdy** nepočítají v `snapshot`/na UI vlákně — čte disk, registr a COM.
- Hledání je mělké: kořen, přímé podsložky s `public/` nebo `index.html`, `apps/*`, `packages/*`;
  `node_modules`, `target`, `dist`, `build`, `docs`, tečkové složky se přeskočí (`SKIP_DIRS`).
- Šablonové obrázky (`vite.svg`, `react.svg`…) nejsou logo (`TEMPLATE_ASSETS`).
- `/x` v HTML = kořen webu → hledá se v `public/x` i `x`; relativní vedle dokumentu i v `public/`.
- Výstup rastrů je vždy PNG ≤ 128 px (WIC, zmenšení v premultiplikované alfě); SVG beze změny ≤ 512 KB.
- WIC místo `png` crate: součást Windows, jen 3 features `windows` crate navíc, žádná nová závislost.
- Klon: jen když `RepoIndex::clones == 0` (dva klony = nejednoznačné, třetí by nepomohl), cíl v `roots[0]`,
  kolize → `<repo>-<owner>`, `<repo>-2…9`. Neúspěch smaže jen složku, kterou založil tentýž pokus.
  Bez okna (`gitinfo::capture`, CREATE_NO_WINDOW), `GIT_TERMINAL_PROMPT=0`, termín 15 min.

## Testy

- `icons::tests` — pořadí kandidátů, chybějící `logo` → `favicon.ico` z HTML, web v podsložce, šablony,
  exe (explorer.exe), `Store` generace, base64.
  `measure_real_logos` (`#[ignore]`, `LEDGE_BENCH_ROOT`) měří nad skutečnými repy.
- `iconimg::tests` — `fit`, zmenšení PNG, největší snímek ICO, exe ikona.
- `cloning::tests` — `clonable`, cíle a kolize, jména složek, volba `gh`/`git`, `Busy`. Nic se neklonuje.
- `resolve::tests` — `clones`, `owner_repo_raw`; `bench_remote_regex` (`#[ignore]`).

## Výkon — co zbývá (neopraveno, s odhadem)

- `spawn_emit_thread` se budí 1×/s kvůli hodinám (formát HH:MM) a `check_reminders`; stačilo by budit
  na hranu minuty / nejbližší `next_due`. Přínos: −59 probuzení/min (hover vlákno ale běží po 100 ms).
- `refresh_strip` volá `Core::monitor` → `EnumDisplayMonitors` při každém volání (1×/s + události);
  hover vlákno má cache po 3 s. Přínos: desítky µs/s.
- `snapshot` klonuje celý `Config` a mapu `git`; s 8 projekty jednotky µs — nechat.
- `git_pull` spouští git bez termínu (`Command::output`) — přepsat na `gitinfo::capture`.

## Co nečíst

`claude/`, `notes/`, `notesync/`, `update.rs`, `migrate.rs`, `src/notes/*`, `src/ClaudeSessions.tsx`.
