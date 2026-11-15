# Codemapa: frontend (SolidJS, `src/`)

Cituj symbolem, ne řádkem. Poznámky: [notes.md](notes.md), karta Claude a schvalování: [sessions.md](sessions.md), loga a dlaždice: [projects-icons-perf.md](projects-icons-perf.md).

Detail: [ui-detail.md](ui-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `index.tsx` | vstup; `styles.css` PRVNÍ, pak `App` |  |
| `App.tsx` · `dormant.ts` | skořápka: store snapshotu, stránky, morf, výška okna, fokus, klávesy, reflektor; skrytý šuplík po 120 s odmontuje strom (`isDormant`) | `App` `prepare` `closing` `isDormant` |
| `lazyViews.tsx` | líné chunky: Nastavení, průvodce, paleta, Poznámky, panely úprav a lazy karty (Počasí, Kalendář, Pošta, GitHub, Systém, Týden, BigClock…) | `view` `Palette` `preloadCards` `lazyCardIds` |
| `nav.ts` | stránky a navigace (záložky šuplíku jsou `layout`, ne stránky) | `Page` `asPage` `typing` `NotesNav` |
| `snapshot.ts` | prázdný snapshot | `EMPTY_SNAPSHOT` |
| `MainPage.tsx` | hlavní stránka: karty záložky `Snapshot.mode` z `layout` → `layout/Deck.tsx` (→ [layout.md](layout.md)) … | `MainPage` `card` |
| `cards/sections.ts` | `drawer.sections` ze snapshotu — ODVOZENÉ z `layout` (zrcadlo `config::Sections`): klíče, čtení části karty … | `SECTION_KEYS` `sectionOn` `allSectionsOn` |
| `cards/CardMenu.tsx` · `TileOptions.tsx` | ⋯ v hlavičce karty (vidět při najetí; seznam = společné menu `ui/Popover.tsx` `PopMenu`) → Velikost S/M/L/XL, Výška … | `CardMenu` `MenuChoice` `HiddenNotice` |
| `cards/Head.tsx` | hlavička (✎ a 🔍 mají `data-tip` = cíle prohlídky `layout/Tour.tsx`): hodiny / Zpět + název, chipy (vč … | `Head` |
| `cards/ChipsRow.tsx` · `HeadClock.tsx` | hlavička (kolo 27): řada štítků vždy v jednom řádku s „+N“ (ResizeObserver, bez časovače) a styly štítků; pět stylů hodin (analog SVG) | `ChipsRow` `HeadClock` |
| `cards/DirtyPill.tsx` | chip „N neuloženo · M nepushnuto“: menu `PopMenu` s repy (Ztišit u tohoto repa = `muteDirty`, Terminál) a „Přečteno“ do další … | `DirtyPill` `dirtySummary` `isAcked` `sigHash` |
| `cards/UpdatePanel.tsx` | panel pod hlavičkou: jednorázové „Co je nového ve verzi X“ (Rozumím → `update_notes_read`) a nová verze … | `UpdatePanel` `install` |
| `update/model.ts` · `update/ReleaseNotes.tsx` · `update/update.css` | poznámky k vydání bezpečně: Markdown → bloky textových uzlů (nadpis, odrážka, odstavec, `kód`, tučně … | `parseNotes` `inline` `progressPct` `mb` `action` |
| `cards/Projects.tsx` · `ProjectTile.tsx` | karta Projekty (zástupci; jde na každou záložku, výchozí i na Osobní) … | `Projects` · `ProjectTile` `launch` `tilt` |
| `cards/Running.tsx` · `Repos.tsx` | karta Běží (log jen rozbalený) · karta Repa (Pull jen čisté a pozadu, Ztišit = `muteDirty`) | `Running` `PHASE` · `Repos` `pull` |
| `cards/Peek.tsx` | malý šuplík: první schválení Clauda (nebo otázka), jinak limity / `ReminderPeek` … | `Peek` `Approval` `DecideButtons` `hasApproval` |
| `hover/*` | náhled po najetí → [hover.md](hover.md) | `SmallView` `Island` `Dock` `Mid` |
| `cards/ActionsCard.tsx` · `palette/*` | karta Rychlé akce · paleta příkazů (Ctrl+K, 🔍) → [actions.md](actions.md) | `ActionsCard` · `Palette` `buildItems` `rank` |
| `cards/ClockCard.tsx` · `WeatherCard.tsx` | záložka Osobní (Čas ukazuje i baterii `media/Battery.tsx`; vzhled Velké hodiny → [board.md](board.md)) | `observedLabel` `WX_MAX_AGE_MS` |
| `cards/temp.ts` | teplota °C → °F při zobrazení (`personal.fahrenheit`), čistá funkce | `tempIn` `tempUnit` |
| `timer/*` (`timer/TimerCard.tsx`) | Osobní: karta Časovač (odpočet + stopky; stav drží Rust `timer.rs`, sekundy z tiku `now`), čistá logika `model.ts` (+ … | `TimerCard` `timerDo` `leftMs` `elapsedMs` |
| `media/*` | Osobní: Právě hraje + hlasitost, baterie, `PersonalLive` (líný start odběrů) → [media.md](media.md) | `MediaCard` `PersonalLive` `Battery` |
| `ClaudeSessions.tsx` | karta Claude → [sessions.md](sessions.md) |  |
| `studio/*` · `settings/*` · `fit.ts` | Nastavení = Ledge Studio → [studio.md](studio.md), panely → [settings.md](settings.md); `fit.ts` = strop výšky (`--drawer-max-h`) | `Studio` |
| `notes/*` | poznámky → [notes.md](notes.md) |  |
| `api.ts` | most k Rustu: `api.*` = `invoke`, `on` = `listen`; mimo Tauri no-op | `api` `on` `inTauri` `call` |
| `types/*` (`types.ts` = re-export; `types/{media,notes,onboarding,personal,projects,settings,timer}.ts`) | typy snapshotu a IPC (zrcadlo serde v Rustu) | `Snapshot` `ProjectView` `ProcView` `SessionView` |
| `motion.ts` | pružiny → CSS `linear()` (simulace `theme/springs.ts`), clip-path morf, naskakování `[data-anim]`, měření … | `spring` `collapsedClip` `bandClip` `FULL_CLIP` |
| `theme/*` | motivy a vzhled (→ [appearance.md](appearance.md)): `themes.ts` sady tokenů, `appearance.ts` čistá logika, `apply.ts` DOM | `THEMES` `appearanceVars` `applyAppearance` |
| `format.ts` | formát času a čísel podle jazyka (24h) | `ago` `uptime` `fmtClock` `fmtDate` `dayNames` |
| `pad.ts` | `pad2` + ořezy `clampNum` / `clampInt` | `pad2` `clampNum` `clampInt` |
| `ui/shortcuts.ts` | zdroj pravdy klávesy a gesta (`SHORTCUTS`, skupiny; test `shortcuts.test.mjs`: každý záznam cs+en) … | `SHORTCUTS` `shortcutsOf` `shortcutText` |
| `idleTimers.test.mjs` | hlídač: `setInterval` jen na povolených místech | — |
| `ui/lru.ts` | stropy cache (`memoLru` obaly, `lruSet` ikony tónu, `trimRecord` loga) | `lruSet` `memoLru` `trimRecord` |
| `util/{path,time,frecency,poll,pollShown}.ts` · `src/clock.ts` | cesty · `debounce` `minuteOf` · frecency · sdílený poll jen s `drawerShown()` · sekundové `now` (viz detail) | `samePath` `debounce` `minuteOf` `sharedPoll` `pollWhileShown` `recordUse` |
| `ui/Confirm.tsx` | jediné potvrzení nevratné akce (inline panel, Esc = Zrušit; fokus jen v okně, které ho má): Přihrádka … | `Confirm` |
| `ui/toast.tsx` · `ui/toast.css` · `ui/rich.tsx` | hlášky dole (`toast.css` je společné pro šuplík i lístek — ten ho přizpůsobí proměnnými `--toast-*` v `pin.css`) … | `say` `sayError` `errorText` `Toast` · `rich` |
| `uiErrors.ts` · `ui/errorline.ts` · `ui/Boundary.tsx` · `ui/boundary.css` | chyby UI: `window.onerror` / `unhandledrejection` / pád vykreslení → `ui_log`; `ErrorBoundary` s „Zkusit znovu“ … | `installErrorLog` `logUiError` `errorLine` `Boundary` |
| `i18n.ts` · `i18n/*` · `i18n/en-*.ts` · `i18nScan.ts` | překlady (viz sekce i18n) · anglické slovníky: `dicts.ts` (všechny `en-*` v jednom líném chunku), `load.ts` (registruje loader) · … | `t` `tp` `plural` `pluralForm` `lang` `setLang` |
| `logos.ts` · `icons.tsx` · `pkgctx/pkgctx.ts` (stav bubliny MSIX, jednou za proces; viz [core.md](core.md) `pkgctx.rs`) | cache log podle `logoKey` (jednou na klíč) · inline SVG ikony | `logoSrc` · `Icon*` |
| `tilelook/*` | vzhled jednotlivých zástupců (projekt, akce, odkaz) — ikona, barva, tvar, popisek … | `TileIcon` `LookEditor` `openTileMenu` `lookOf` |
| `styles.css` | tokeny `:root` (= motiv dark; škály `--fz --dens --rr --glass --speed`, barvy, `--accent*`, `--dur-*`), odvozené tokeny v … |  |
| `cards/*.css` · `settings.css` · `sessions.css` · `notes/notes.css` | styly vedle komponent, jen tokeny ze `styles.css` |  |
| `cssClasses.ts` | analýza globálních CSS tříd pro hlídací test (kolize, nedefinované) | `cssRules` `ownedClass` `classesIn` `collisions` |

Další soubory (podrobnosti v detailu):
`src/projects/dirty.ts` · `src/sessions/{DiffPreview.tsx,PermExtra.tsx,PlanReview.tsx}` · `src/settings/panels/About.tsx`
