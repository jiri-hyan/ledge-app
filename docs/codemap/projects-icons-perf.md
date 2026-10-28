# Codemapa: dlaždice projektů, loga, klonování, výkon snapshotu

Čti tohle místo celého jádra ([core.md](core.md)) a frontendu ([ui.md](ui.md)). Cituj symbolem, ne číslem řádku.

Detail (toky, invarianty, testy, co nečíst, historie): [projects-icons-perf-detail.md](projects-icons-perf-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/icons.rs` | výběr loga projektu, cache výsledků | `resolve` `Source` `exe_policy` `ExeIcon` `candidates` `load` `raster_size` `icon_links` |
| `src-tauri/src/iconimg.rs` | rastry přes WIC: dekódování, největší snímek ICO, zmenšení ≤ 128 px, PNG; ikona z exe; ikona souboru/složky ze shellu (rychlé akce, [actions.md](actions.md)) | `to_png`, `exe_png`, `shell_png`, `fit`, `MAX_PX`, `encode` |
| `src-tauri/src/cloning.rs` | „Naklonovat“: kdy, kam, čím, spuštění | `clonable`, `target_in`, `folder_name`, `command`, `run`, `Busy`, `is_busy`, `TIMEOUT` |
| `src-tauri/src/resolve.rs` | repo index podle remote (i linked worktree), odkud projekt poběží, kandidáti složek s datem commitu, appka podle jména | `RepoIndex::build` `candidates` `clones` `origin_any` `Place` `place_of` `Candidate` |
| `src/ui/place.ts` · `src/ui/Popover.tsx` | umístění plovoucích prvků (`placePopup`: pod kotvu / otočit nahoru / posunout do okna, `maxH`) a počet sloupců mřížky (`tileColumns`; test `place.test.mjs`) … | `placePopup` `tileColumns` `Popover` `boxOf` `PopMenu` `PopItem` `createMenu` `navTarget` |
| `src/projects/*` | co a odkud se spouští (vlna 9 Z; `TileTip.tsx` = náhled po najetí, `tipState`, `tipEnter`/`tipLeave`): `model.ts` (čisté … | `startProject` `openProjectPanel` `FailBox` `RunningPill` |
| `src-tauri/src/state.rs`, `refresh.rs`, `snapshot.rs`, `commands/projects.rs` | lepidlo | `Core.logos` `Core::rebuild_index` `Core::project_place` `project_dir` `refresh_logos` |
| `src-tauri/src/onboarding/repos.rs` | návrh projektů z rep na disku (průvodce) — viz [onboarding.md](onboarding.md) | `discover`, `suggest` |
| `src/cards/Projects.tsx` · `ProjectTile.tsx` · `src/logos.ts` | karta (složky, tah v úpravách, menu pravého kliku, panel vzhledu) · jedna dlaždice (logo nebo `TileIcon`, akce uvnitř, ⋯ = Upravit vzhled) · cache log | `Projects` `startDrag` `applyTiles` `ProjectTile` `launch` `tilt` `projectAuto` `logoSrc` |
| `src/tilelook/*` | vzhled jednotlivého zástupce (vlna 8 P): čistá logika `model.ts` (normalizace, předvolby, `inkDark` kontrast, `withTileLook`, přehled … | `normalizeTileLook` `inkDark` `TILE_COLORS` `GLYPHS` `searchGlyphs` `lookOf` `openLook` |
| `src-tauri/src/tilelook.rs` · `commands/tilelook.rs` | typy a validace `look` (projekt, akce, karta, složky); vlastní obrázek → WIC (`iconimg::to_png`) → `%LOCALAPPDATA%\Ledge\looks\<fnv>.png` (SVG beze změny ≤ 512 KB) … | `import_image` `image_url` `key_ok` `glyph_ok` |
| `src/api.ts` | typy + IPC | `ProjectView` (`logoKey`, `clonable`, `cloning`), `api.projectLogo`, `api.cloneProject` |

Další soubory (podrobnosti v detailu):
`src/projects/{ProjectPanel.tsx,RunningPill.tsx}` · `src/tilelook/{LookEditor.tsx,TileIcon.tsx,TileMenu.tsx,glyphs.ts}` · `src/ui/popnav.ts`
