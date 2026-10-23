# Codemapa: dlaždice projektů, loga, klonování, výkon snapshotu

Čti tohle místo celého jádra ([core.md](core.md)) a frontendu ([ui.md](ui.md)). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/icons.rs` | výběr loga projektu, cache výsledků | `resolve`, `Source`, `exe_policy`/`ExeIcon`, `candidates`, `load`, `raster_size`, `icon_links`, `manifest_icons`, `resolve_href`, `PATTERNS`, `Store` (`begin`/`finish`/`key`/`url`), `LIMIT`, `STALE` |
| `src-tauri/src/iconimg.rs` | rastry přes WIC: dekódování, největší snímek ICO, zmenšení ≤ 128 px, PNG; ikona z exe; ikona souboru/složky ze shellu (rychlé akce, [actions.md](actions.md)) | `to_png`, `exe_png`, `shell_png`, `fit`, `MAX_PX`, `encode` |
| `src-tauri/src/cloning.rs` | „Naklonovat“: kdy, kam, čím, spuštění | `clonable`, `target_in`, `folder_name`, `command`, `run`, `Busy`, `is_busy`, `TIMEOUT` |
| `src-tauri/src/resolve.rs` | repo index podle remote (i linked worktree), odkud projekt poběží, kandidáti složek s datem commitu, appka podle jména | `RepoIndex::build`/`candidates`/`clones`, `origin_any`, `Place`, `place_of`, `Candidate`, `sort_candidates`, `candidates_dated`, `last_commit`, `parse_remote`, `owner_repo_raw`, `installed_app_exe` |
| `src/ui/place.ts` · `src/ui/Popover.tsx` | umístění plovoucích prvků (`placePopup`: pod kotvu / otočit nahoru / posunout do okna, `maxH`) a počet sloupců mřížky (`tileColumns`; test `place.test.mjs`); popover přes Portal do `.shell` (Vzhled…, Upravit…, náhled dlaždice); **JEDNO menu `PopMenu`** (role `menu`, ↑↓ Home End, Esc vrátí fokus na spouštěč, klik mimo / posun zavře, překlopení u okraje; `PopItem` `PopSep` `PopCap`, `createMenu` pro spouštěč; styly `.pop-*` v `ui/popover.css`, okno lístku má tokeny v `pin.css`) pro ⋯ karty, ⋯ poznámky, ⋯ řádku, pravý klik na zástupce, výběr vzhledu přehrávače a ⋯ lístku (`fill`); čistá logika `ui/popnav.ts` (`navTarget` `menuKey`, test `popnav.test.mjs`) | `placePopup` `tileColumns` `Popover` `boxOf` `PopMenu` `PopItem` `createMenu` `navTarget` |
| `src/projects/*` | co a odkud se spouští (vlna 9 Z; `TileTip.tsx` = náhled po najetí, `tipState`, `tipEnter`/`tipLeave`): `model.ts` (čisté: `placeOf`, `clickOf`, `tileTip`, `whereOf`, `preselect`/`freshest`, `withProjectFields`, `failInfo`; test `model.test.mjs`), `state.ts` (panel, `startProject`, `changeFolder`, Stop/Restart/`stopAll`), `ProjectPanel.tsx` (Upravit… / výběr složky / `FailBox`), `RunningPill.tsx` (pilulka „N běží“ + cizí procesy na portech), `projpanel.css` (`pz-`), `en.ts` | `startProject` `openProjectPanel` `FailBox` `RunningPill` |
| `src-tauri/src/state.rs`, `refresh.rs`, `snapshot.rs`, `commands/projects.rs` | lepidlo | `Core.logos`, `Core::rebuild_index`, `Core::project_place`/`project_dir`, `refresh_logos`, `on_proc_event` (toast selhání), `snapshot` (blok `projects`: `place`, `port`, `remote`, `app`), `ProjectView`, `github_url`, příkazy `project_logo`, `clone_project`, `launch` (`place_error`), `restart`, `stop_all_servers`, `project_folders`, `port_holders`, `kill_port_holder`, `open_in` → `open_dir` (sdílí panel Git session) |
| `src-tauri/src/onboarding/repos.rs` | návrh projektů z rep na disku (průvodce) — viz [onboarding.md](onboarding.md) | `discover`, `suggest` |
| `src/cards/Projects.tsx` · `ProjectTile.tsx` · `src/logos.ts` | karta (složky, tah v úpravách, menu pravého kliku, panel vzhledu) · jedna dlaždice (logo nebo `TileIcon`, akce uvnitř, ⋯ = Upravit vzhled) · cache log | `Projects` `startDrag` `applyTiles` · `ProjectTile` `launch` `tilt` `projectAuto` · `logoSrc`, `logoUrls` |
| `src/tilelook/*` | vzhled jednotlivého zástupce (vlna 8 P): čistá logika `model.ts` (normalizace, předvolby, `inkDark` kontrast, `withTileLook`, přehled; test `model.test.mjs`), `glyphs.ts` (77 ikon Segoe Fluent Icons ověřených v `SegoeIcons.ttf` i `segmdl2.ttf`, hledání), `state.ts` (který zástupce se upravuje, živý náhled `lookOf`, uložení jen `look`, obrázky z cache), `TileIcon.tsx` (kresba), `LookEditor.tsx` (panel pod dlaždicemi), `TileMenu.tsx` (pravý klik), `tilelook.css` (`tl-`), `en.ts` | `normalizeTileLook` `inkDark` `TILE_COLORS` `GLYPHS` `searchGlyphs` `lookOf` `openLook` `closeLook` `saveTileLook` `pickLookImage` `TileIcon` `LookEditor` `openTileMenu` |
| `src-tauri/src/tilelook.rs` · `commands/tilelook.rs` | typy a validace `look` (projekt, akce, karta, složky); vlastní obrázek → WIC (`iconimg::to_png`) → `%LOCALAPPDATA%\Ledge\looks\<fnv>.png` (SVG beze změny ≤ 512 KB); příkazy `look_image_add`, `look_image` | `import_image` `image_url` `key_ok` `glyph_ok` |
| `src/api.ts` | typy + IPC | `ProjectView` (`logoKey`, `clonable`, `cloning`), `api.projectLogo`, `api.cloneProject` |

## Zdraví projektu na zástupci

`projects/model.ts` `tileHealth` (neuložené změny + nepushnuté commity z `projects[].git`, „Claude tu pracuje“ = session s `projectId` projektu a stavem `working`),
`healthTone`, `healthText` (věta). `ProjectTile.tsx` kreslí v rohu `.proj-hl` (tečky, `pointer-events: none`, třídy v `cards/projects.css`);
u malých dlaždic a kompaktního vzhledu jediná tečka podle důležitosti, s git řádkem pod názvem (`tiles.git`) se dirty / ahead nezdvojují.
Věta je v náhledu po najetí (`TileTip` prop `health`, kreslí `Projects.tsx`) a v `aria-description`. Nic nepolluje — jen snapshot.

### Poznámky projektu v panelu Upravit…

`ProjectPanel.tsx` `ProjectNotes` (jen se zapnutými poznámkami a s `nav` z `MainPage` → `Projects`): JEDNO `api.noteQuery(projectNotesQuery(id))`
při otevření panelu (`createResource`, žádné obnovování) — až `PROJECT_NOTES` = 3 nehotové poznámky projektu, klik = zavřít panel a
`nav.onOpen`, „Nová poznámka k projektu“ = `onNote({ project })`, „+N dalších“ = stránka Poznámky s filtrem projektu (`setPageProject`).
Čisté: `projectNotesQuery`, `notesHidden` (`projects/model.ts`).

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

- **Jedno menu dlaždice** (`Projects.tsx` `menuFor`) pro pravý klik, ⋯ i Shift+F10 — nové akce přidávej TAM, ne do
  `.acts` (ty jsou u malých dlaždic a „jen ikony“ schované kromě ▶/■ a ⋯, `.proj-xa`). Mřížka: `fitColumns`
  (ResizeObserver + `tileColumns`) nastaví `--cols`; dlaždice se při každém snapshotu překreslují, proto kotvy
  popoverů hledají dlaždici podle `data-tl-tile`, ne podle uloženého prvku.

- **Odkud projekt poběží** (`resolve::place_of`, jediné místo): `path` vyhrává, i neexistující (= `Place::Missing`,
  chyba na dlaždici — dřív se potichu hledalo podle remote). Bez `path` jediný kandidát podle `remote`; víc kandidátů
  = `Place::Choose`, nikdy náhodný výběr. Datum commitu (`git log -1`) se zjišťuje JEN na otevření výběru
  (`project_folders`), nikdy ve snapshotu. Linked worktree je kandidát (`origin_any`) — třeba `app-v2` vedle staré `app` je worktree.
- Selhání startu: `procman` drží `cwd`, `cmd`, `tail` (`fail_tail`); `ProcEvent::Failed { id, message }` → toast
  jen přes `attention::signal` (šuplík neotevřený, Ledge není „mrtvý“). Cizí proces na portu se zabíjí jen po
  potvrzení a jen když port drží TEĎ (`kill_port_holder`), nikdy proces z vlastního jobu.

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
- `resolve::tests` — `clones`, `owner_repo_raw`, worktree jako druhý klon, řazení kandidátů (deterministicky), `place_of`,
  `last_commit` nad dočasným repem; `bench_remote_regex` (`#[ignore]`).
- `procman::tests` — `fail_tail`, `tail`/`cwd`/`cmd` u pádu, událost `Failed` s id a zprávou.
- `src/projects/model.test.mjs` — klik podle místa, tooltip, předvolba výběru, zápis `path`/`run`, `failInfo`.

## Výkon — co zbývá (neopraveno, s odhadem)

- `spawn_emit_thread` se budí 1×/s kvůli hodinám (formát HH:MM) a `check_reminders`; stačilo by budit
  na hranu minuty / nejbližší `next_due`. Přínos: −59 probuzení/min (hover vlákno ale běží po 100 ms).
- `refresh_strip` volá `Core::monitor` → `EnumDisplayMonitors` při každém volání (1×/s + události);
  hover vlákno má cache po 3 s. Přínos: desítky µs/s.
- `snapshot` klonuje celý `Config` a mapu `git`; s 8 projekty jednotky µs — nechat.
- `git_pull` spouští git bez termínu (`Command::output`) — přepsat na `gitinfo::capture`.

## Co nečíst

`claude/`, `notes/`, `notesync/`, `update.rs`, `migrate.rs`, `src/notes/*`, `src/ClaudeSessions.tsx`.
