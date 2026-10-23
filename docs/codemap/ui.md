# Codemapa: frontend (SolidJS, `src/`)

Čti tohle místo celých souborů. Cituj symbolem, ne číslem řádku. Poznámky: [notes.md](notes.md),
karta Claude a schvalování: [sessions.md](sessions.md), loga a dlaždice: [projects-icons-perf.md](projects-icons-perf.md).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `index.tsx` | vstup; `styles.css` načte PRVNÍ, pak `App` | |
| `App.tsx` | skořápka: store snapshotu, stránky, morf, výška okna, fokus, klávesy, reflektor | `App` `apply` `prepare` `play` `closing` `report` `startNote` `editNote` `allNotes` `notesNav` |
| `lazyViews.tsx` | **líné chunky** mimo první obrazovku: Nastavení (i s editorem rozložení a přehledem vzhledů), průvodce, paleta, Poznámky, panel „Nová verze“, katalog karet, panel vzhledu karty — `lazy` + `<Suspense>` bez fallbacku (CSS se načte s chunkem dřív, než `import()` skončí); `preloadViews` je v `requestIdleCallback` přednačte až po prvním `view === "full"` (jeden chunk na idle). Paleta před sebou čeká na anglický slovník (`tIn("en")` v hledání). Hlavní chunk ~430 kB (bylo 984), angličtina ~164 kB zvlášť; co patří do `index-*.js`, hlídej při `corepack pnpm build` | `Settings` `Wizard` `Palette` `NotesPage` `UpdatePanel` `Catalog` `CardLookPanel` `preloadViews` |
| `nav.ts` | stránky a navigace (záložky šuplíku jsou `layout`, ne stránky) | `Page` `asPage` `typing` `NotesNav` |
| `snapshot.ts` | prázdný snapshot do startu | `EMPTY_SNAPSHOT` |
| `MainPage.tsx` | hlavní stránka: karty záložky `Snapshot.mode` z `layout` → `layout/Deck.tsx` (→ [layout.md](layout.md)); každé kartě předá `menu` (⋯), lišta úprav, katalog, prázdná záložka = „Přidej první kartu“; pravý klik na prázdné místo = menu Upravit rozložení / Přidat kartu / Vzhled / Nastavení (`openTileMenu`) | `MainPage` `card` |
| `cards/sections.ts` | `drawer.sections` ze snapshotu — ODVOZENÉ z `layout` (zrcadlo `config::Sections`): klíče, čtení části karty; test `sections.test.mjs` (shoda s config.rs a s modelem rozložení) | `SECTION_KEYS` `sectionOn` `allSectionsOn` `CLAUDE_PARTS` |
| `cards/CardMenu.tsx` · `TileOptions.tsx` | ⋯ v hlavičce karty (vidět při najetí; seznam = společné menu `ui/Popover.tsx` `PopMenu`) → Velikost S/M/L/XL, Výška Auto/Nízká/Vysoká, u Projektů volby dlaždic (`TileOptions`), „Skrýt kartu“ (z této záložky do katalogu) / „Skrýt část: …“ (`layout.hiddenParts`) / „Upravit šuplík…“; ukládá jen `layout` (`layout/edit.ts` `saveLayout`, při `cfgError` ne); lišta „… skryta · Vrátit“ 6 s | `CardMenu` `MenuChoice` `HiddenNotice` `TileOptions` |
| `cards/Head.tsx` | hlavička (✎ a 🔍 mají `data-tip` = cíle prohlídky `layout/Tour.tsx`): hodiny / Zpět + název, chipy (vč. tlumeného „V bublině Claude“ z `pkgctx/BubbleNotice.tsx` `bubble` → klik = Nastavení u autostartu), nová verze, záložky `layout/Tabs.tsx`, ✎ režim úprav, nástroje | `Head` |
| `cards/DirtyPill.tsx` | chip „N neuloženo · M nepushnuto“: menu `PopMenu` s repy (Ztišit u tohoto repa = `muteDirty`, Terminál) a „Přečteno“ do další změny počtů (otisk `sigHash` v `gitAlerts.read`, přežije restart); vypínač `gitAlerts.enabled`; logika `projects/dirty.ts` (test `dirty.test.mjs`) | `DirtyPill` `dirtySummary` `isAcked` `sigHash` |
| `cards/UpdatePanel.tsx` | poznámky k vydání → Instalovat (druhé potvrzení při běžících serverech; v bublině MSIX řádek, že instalátor poběží mimo ni) | `UpdatePanel` `install` |
| `cards/Projects.tsx` · `ProjectTile.tsx` | karta Projekty (zástupci; jde na každou záložku, výchozí i na Osobní): hlavička s ⋯, dlaždice `data-tiles` small/tile/large, varianta `data-view` tiles/list/compact, název a git podle `tiles`, skryté projekty záložky, složky (`layout/folders.ts`, tah v režimu úprav), spuštění, náklon, klon, akce UVNITŘ dlaždice dole (nikdy přes kartu pod ní; ⋯ = Upravit vzhled), vlastní vzhled (`tilelook/*`); prázdné stavy „Přidat projekt“ / „Ukázat všechny“ | `Projects` · `ProjectTile` `launch` `tilt` |
| `cards/Running.tsx` · `Repos.tsx` | karta Běží (log jen rozbalený) · karta Repa (Pull jen čisté a pozadu, Ztišit = `muteDirty`) | `Running` `PHASE` · `Repos` `pull` |
| `cards/Peek.tsx` | malý šuplík: první schválení Clauda (nebo otázka), jinak limity / `ReminderPeek`; `Approval` (schválení / otázka, `stack` = tlačítka pod textem) a `DecideButtons` sdílí náhled po najetí | `Peek` `Approval` `DecideButtons` `hasApproval` `pendingWhere` |
| `hover/*` | náhled po najetí (`drawer.hover`, Rust `View::Compact` / `View::Island`, [core.md](core.md)): `SmallView.tsx` (peek / kompaktní / ostrůvek ve skořápce), `Compact.tsx` (schválení přes `Approval`, max 3 session, Právě hraje z `media/Player.tsx`, čas, limit 5h, „▾ celý šuplík“), `Island.tsx` (černá kapsle s jedním děním a tlačítky), `expand.ts` (klik mimo tlačítka / setrvání → `api.expandDrawer`), čistá logika `model.ts` (+ `model.test.mjs`: priorita dění, co compact ukáže, kdy se rozbalit, šířky), `modes.ts` = `HOVER_MODES` pro výběr v Nastavení (id `full|compact|island` = kontrakt), styly `hover.css` (`hov-*`, `isl-*`) | `SmallView` `Compact` `Island` `useExpand` `islandEvent` `compactPlan` `expandAfter` `smallWidth` `freshWeather` `HOVER_MODES` `hoverModeOf` |
| `cards/ActionsCard.tsx` · `palette/*` | karta Rychlé akce · paleta příkazů (Ctrl+K, 🔍) → [actions.md](actions.md) | `ActionsCard` · `Palette` `buildItems` `rank` |
| `cards/ClockCard.tsx` · `WeatherCard.tsx` · `LinksCard.tsx` | záložka Osobní (Čas ukazuje i baterii `media/Battery.tsx`; Odkazy = akce typu web/složka/soubor → [actions.md](actions.md)) | `observedLabel` `WX_MAX_AGE_MS` `LINK_ICON` `linkActions` `openLink` |
| `timer/*` | Osobní: karta Časovač (odpočet + stopky; stav drží Rust `timer.rs`, sekundy z tiku `now`), čistá logika `model.ts` (+ `model.test.mjs`), akce `timerDo` (karta i paleta) | `TimerCard` `timerDo` `leftMs` `elapsedMs` `fmtDuration` `parseMinutes` `parsePresets` |
| `media/*` | Osobní: Právě hraje + hlasitost, baterie, `PersonalLive` (líný start odběrů) → [media.md](media.md) | `MediaCard` `PersonalLive` `Battery` |
| `ClaudeSessions.tsx` | karta Claude → [sessions.md](sessions.md) | |
| `Settings.tsx` · `settings/*` · `fit.ts` | Nastavení → [settings.md](settings.md); `fit.ts` = strop výšky (`--drawer-max-h`) | `Settings` |
| `notes/*` | poznámky → [notes.md](notes.md) | |
| `api.ts` | most k Rustu: `api.*` = `invoke`, `on` = `listen`; mimo Tauri no-op | `api` `on` `inTauri` `call` |
| `types/*` (`types.ts` = re-export) | typy snapshotu a IPC po oblastech (zrcadlo serde v Rustu); `api.ts` je re-exportuje | `Snapshot` `ProjectView` `ProcView` `SessionView` `NoteView` `UpdateView` `ViewMsg` |
| `motion.ts` | pružiny → CSS `linear()` (simulace `theme/springs.ts`), clip-path morf, naskakování `[data-anim]`, měření; styl/rychlost z vzhledu; co se při otevření animuje, určuje `theme/openAnim.ts` | `spring` `collapsedClip` `bandClip` `FULL_CLIP` `OVERLAP` `measure` `preHide` `revealContent` `concealContent` `setMotion` `dur` |
| `theme/*` | motivy a vzhled (→ [appearance.md](appearance.md)): `themes.ts` sady tokenů, `appearance.ts` čistá logika, `apply.ts` DOM | `THEMES` `appearanceVars` `applyAppearance` `initAppearance` `savedAppearance` |
| `format.ts` | formátování času a čísel podle jazyka (`t`, `locale()`; čas 24h v obou) | `ago` `uptime` `fmtClock` `fmtDate` `dayNames` `isoWeek` `num` `pad2` |
| `ui/shortcuts.ts` | **zdroj pravdy klávesy a gesta** (`SHORTCUTS`, skupiny; test `shortcuts.test.mjs`: každý záznam cs+en): Nastavení → Začínáme → Klávesy a gesta (`settings/panels/Start.tsx` `KeysList`, kotva `f-onboarding.keys`) a paleta „? Klávesy a gesta“ (`palette/items.ts` `app:keys`); angličtina `i18n/en-keys.ts` | `SHORTCUTS` `shortcutsOf` `shortcutText` |
| `util/path.ts` · `util/time.ts` | **jediné** `normPath` / `samePath` (Windows cesty; re-exportují `pin/model.ts` a `projects/model.ts`) · sdílený `debounce` (`flush` / `cancel` / `pending`; koncept a úprava poznámky `notes/draft.ts`, automatické ukládání lístku, lišta Zpět v Nastavení) — testy `path.test.mjs`, `time.test.mjs` | `samePath` `normPath` `debounce` |
| `ui/Confirm.tsx` | **jediné potvrzení nevratné akce** (inline panel, Esc = Zrušit; fokus jen v okně, které ho má): Přihrádka — Vyprázdnit, Git panel (Push / Zahodit / Odstranit worktree), Nastavení, `ConfirmAction` u spuštění akce; co jde vrátit, se provede hned s lištou Zpět (`notes/UndoBar`, `HiddenNotice`) | `Confirm` |
| `ui/toast.tsx` · `ui/toast.css` · `ui/rich.tsx` | hlášky dole (`toast.css` je společné pro šuplík i lístek — ten ho přizpůsobí proměnnými `--toast-*` v `pin.css`); `<Toast />` ve skořápce · přeložený text s `<b>…</b>` → uzly (bez innerHTML) | `say` `sayError` `errorText` `Toast` · `rich` |
| `i18n.ts` · `i18n/*` · `i18nScan.ts` | překlady (viz sekce i18n) · anglické slovníky: `dicts.ts` (všechny `en-*` v jednom líném chunku), `load.ts` (registruje loader) · statická kontrola pro hlídací test | `t` `tp` `plural` `pluralForm` `lang` `setLang` `loadDict` `setDictLoader` `locale` `registerDict` · `scanSource` |
| `logos.ts` · `icons.tsx` | cache log podle `logoKey` (jednou na klíč) · inline SVG ikony | `logoSrc` · `Icon*` |
| `tilelook/*` | vzhled jednotlivých zástupců (projekt, akce, odkaz) — ikona, barva, tvar, popisek; panel „Upravit vzhled…“ uvnitř karty, menu pravého kliku → [projects-icons-perf.md](projects-icons-perf.md) | `TileIcon` `LookEditor` `openTileMenu` `lookOf` |
| `styles.css` | tokeny `:root` (= motiv dark; škály `--fz --dens --rr --glass --speed`, barvy, `--accent*`, `--dur-*`), odvozené tokeny v `:root, .theme-scope` + základ (shell, karta, řádek, pill, `.btn`/`.mini`, pole, toast) čtoucí proměnné design stylu (`--m-*`, `--h-*`, `--deck-*`, `--l-pad`; výchozí = dnešní vzhled) + blok „Styly“ (plovoucí panel, ozdoby `html[data-style]`) → [appearance.md](appearance.md) | |
| `cards/*.css` · `settings.css` · `sessions.css` · `notes/notes.css` | styly vedle komponent, jen tokeny ze `styles.css` | |
| `cssClasses.ts` | čistá analýza globálních CSS tříd pro hlídací test: vlastník vs. zmínky (kolize) a třídy přidělené v TSX/TS vs. definované v CSS | `cssRules` `ownedClass` `classesIn` `collisions` · `usedClasses` `definedClasses` `missingClasses` |

## Toky

- **Snapshot**: Rust `emit("state")` → `on<Snapshot>("state", apply)` → `setS(reconcile(snap, { key: "id", merge: true }))`
  → komponenty čtou `s` přes props (`p.s.*`), žádný vlastní fetch. Start: `api.getState()` + `api.takePage()` →
  `api.uiReady()`. Tik `now` 1 s jen při `view() !== "hidden"`.
- **Morf** (jen `App.tsx` + `motion.ts`): `view` → `prepare` (složí clip-path na proužek, `preHide`, `animAck("prepared")`)
  → Rust ukáže okno → `play` (clip-path na `FULL_CLIP`, „nádech“, `revealContent`, `measure("open")`);
  `closing` → `concealContent` + zúžení → `animAck("closed")`. Statistika jde do `/status` → `drawer.anim`.
- **Výška okna**: `ResizeObserver(content)` + změna `view`/`page` → `report` → `api.reportSize(view, w, h − OVERLAP)`;
  šířka malých pohledů `hover/model.ts` `smallWidth` (peek 680, kompaktní 420, ostrůvek 480), jinak podle stránky.
- **Náhled po najetí** (`view` = `compact` / `island`, `hover/*`): kreslí se jen ze snapshotu (žádný fetch; média /
  počasí / limity jen když už jsou), stejný morf jako peek (z proužku, do celého šuplíku z tvaru náhledu).
  Celý šuplík: klik na prázdné místo nebo „▾ celý šuplík“ hned, setrvání po `ViewMsg.expandMs`
  (`drawer.hoverExpandMs`) — odpočet stojí nad tlačítky, mimo okno a s čekajícím schválením (`expandAfter`).
  `api.expandDrawer()` fokus nebere; odjetí myši zavře náhled jako šuplík (Rust `threads/hover.rs`).
- **Fokus**: šuplík je okno BEZ fokusu. Jen výslovný klik do poznámky (`startNote`/`editNote`/`allNotes`) volá
  `api.focusDrawer()`; odchod ze stránky, která chce klávesnici (`typing`), volá `api.releaseFocus()`.
  `api.close()` (Esc, ✕, uložení nové poznámky, paleta „Zavřít šuplík“) = `UiCmd::Close` → Rust vrátí fokus
  aplikaci, odkud si ho vzal (`focusreturn`, [core.md](core.md)); spuštění akce/projektu zavírá bez návratu.
- **Klávesy** (`App`): `Ctrl+K` = paleta (jen se šuplíkem s fokusem; 🔍 klikem fokus vezme; kompaktní režim
  `palette.compact` = jen pole, seznam až při psaní / ↓ — `palette/compact.ts`), Esc = zavřít paletu / zrušit úpravy šuplíku / zpět na hlavní / zavřít; `1`–`9` = záložka z `layout` (ne v polích). Na stránce `note` je
  obsluhuje `NoteEditor`.
- **Reflektor**: `pointermove` v `App` nastaví `--mx/--my` na `.card`/`.proj` (jen CSS proměnné, žádný render);
  náklon dlaždice `--rx/--ry` v `tilt`.

## i18n (cs/en)

- **Čeština je zdroj pravdy v kódu**: `t("oblast.klic", "český text", { x })`, množná čísla `tp("klíč", n, "pevný text" | ["1", "2–4", "5+"])`
  (`{n}` je vždy k dispozici). Angličtina = slovníky `src/i18n/en-<oblast>.ts` (+ `onboarding/en.ts`, Nastavení `settings/en.ts` a `settings/panels/en.ts` — pro `settings.*`, `appearance.*`, `actions.panel.*`, `questions.panel.*` autoritativní); en tvary
  množného čísla jsou klíče `klíč.one` / `klíč.other`. Chybějící en klíč = čeština, nikdy prázdno.
- Klíče podle oblasti: `cards.*`, `sessions.*`, `notes.*`, `palette.*`, `actions.*`, `format.*`, `ui.*`,
  `appearance.*`, `settings.*`, `questions.*`, `onboarding.*`. Tučné slovo v překladu: `rich(t(…, "<b>{n}</b> běží"))`.
- Data s popisky = páry `{ key, cs }` / `[key, cs]` a překlad až při vykreslení (`t(x.key, x.cs)`); **nikdy t() při
  importu modulu** (konstanta by zůstala česky) — popisky v tabulkách jako funkce (`() => t(…)`).
- Anglické slovníky jsou **líné**: `src/i18n/load.ts` (první import `App.tsx`) jen registruje loader `import("./dicts")`; při češtině se nenačítá nic, `setLang("en")` počká na chunk a pak přepne signál (poslední volání vyhrává, bez loaderu — testy — je synchronní). Okno lístku (`pin/main.tsx`) má vlastní loader. Nový slovník = řádek v `i18n/dicts.ts`.
  Jeden klíč smí být jen v jednom slovníku se stejným textem (hlídač).
- Klíč složený z id v datech: buď páry (`{ id, key, cs }`, `[key, cs]`; pole je pár jen s klíčem z `AREAS` v `i18nScan.ts`),
  nebo tabulka v `TABLES` hlídače (`FIELDS` v `settings/labels.ts` → `settings.f.<id>`, `.hint`, `.kw`) — pak (a)–(c) platí i pro ni.
- Jazyk: `ui.lang` (`cs`/`en`) v configu → `onboarding/state.ts` `checkFirstRun` → `setLang` (signál; JSX se přepíše bez restartu).
- Hlídá `src/i18n.test.mjs` (`test:ui`): en záznam pro každý klíč, jeden klíč = jedna čeština, shodné `{x}` a `<b>`,
  žádná natvrdo čeština mimo `PENDING` a `ALLOW_CZECH`, klíče mimo literál jen
  v `DYNAMIC` s důvodem, žádné osiřelé en klíče. Texty z Rustu (chyby, stavy ve snapshotu, proužek) jdou podle `ui.lang`
  přes `src-tauri/src/i18n.rs` (viz [core.md](core.md)); frontend Rustí texty jen zobrazuje, nikdy je neporovnává (stav = kód).

## Invarianty a pasti

- Logiku morfu (`prepare`/`play`/`closing`, časy, pružiny) neměň bez měření `drawer.anim` (`scripts/perf.ps1`).
- `OVERLAP` (motion.ts) = `--overlap` (styles.css) = přesah v `drawer.rs`; `FULL_CLIP` 26px = `--r-shell`.
- Nic nesmí brát fokus samo: `autofocus`/`focus()` jen na stránkách, kam se jde výslovným klikem
  (`NoteEditor`, výběry v `notes/`), nikdy v kartách hlavní stránky ani v peeku.
- Schválení Clauda (`api.decide`) jen klikem člověka — v `Peek` a `ClaudeSessions` `PendingRow` (+ „Vždy povolit“ / „Zamítnout s důvodem…“ v `sessions/PermExtra.tsx`, fokus jen po kliku na „s důvodem…“; u plánu „Schválit a povolit úpravy“ v `sessions/PlanReview.tsx`, náhled diffu `sessions/DiffPreview.tsx` jen rozbaluje).
- Pořadí CSS: `styles.css` první (index.tsx), komponentové soubory ho přepisují. **Žádné natvrdo barvy,
  px písma ani délky přechodů**: barvy = token z `COLOR_KEYS` (hodnota ve všech motivech `theme/themes.ts`),
  písmo `var(--fs-*)` nebo `calc(Npx * var(--fz))`, mezery `--s*`, rohy `--r-*`, délky `--dur-1…5`
  (0.15–0.35 s ÷ `--speed`), pružný průběh `var(--spring)` / `var(--ease)`.
- **CSS je globální** (žádné moduly): třídu, kterou soubor vlastní (stojí na začátku selektoru sama, `.proj { … }`),
  nesmí zmínit jiný soubor — ani jako modifikátor (`.st-item.proj` dostal `flex-direction` dlaždice). Nové třídy
  s prefixem oblasti: `st-` Nastavení, `ap-` Vzhled, `apv-` HTML náhledy Vzhledu (`settings/previews`), `ob-` průvodce, `act-` akce, `pal-` paleta, `q-` otázky,
  `sess-` Claude, `proj-` dlaždice, `tl-` vzhled zástupců (`tilelook/`), `repo-` karta Repa, `note-`/`pick-`/`pp-`/`rp-` poznámky. Neprefixované smějí být jen modifikátory
  (`.on`, `.bad`, `.dim`, `.unset`…) se jménem, které nikdo nevlastní, a základ ze `styles.css` (`.card`, `.row`,
  `.btn`, `.mini`…). Hlídá `src/cssClasses.test.mjs` (`test:ui`); záměrné sdílení = `SHARED` s důvodem.
- **Každá třída, kterou kód přiděluje** (`class="…"`, `class={…}` s literály, klíče `classList={{…}}`, `className`,
  `classList.add/toggle/replace`), musí mít pravidlo v nějakém `src/**/*.css` (v0.2.7 zmizela `.opt` a panely se
  rozpadly). Háček bez vzhledu = `HOOKS` s důvodem; název skládaný za běhu (`pill ${x}`) = `DYNAMIC` se všemi
  hodnotami — ty musí být definované a nesmí je vlastnit žádný soubor (`ap-strip pill` dostal písmo z `.pill`).
  Přednost má `classList={{ a: …, b: … }}`, ten se ověří staticky. Hlídá tentýž `cssClasses.test.mjs`.
- **Přístupnost**: záložky `layout/Tabs.tsx` = `tablist`/`tab`/`aria-selected` (←/→/Home/End přesouvají fokus); `@media (prefers-contrast: more)` a `(forced-colors: active)` na konci `styles.css` (+ `popover.css`, `confirm.css` pro vlastní třídy); nejmenší písmo 11 px = `--fs-2xs` (test `ui/fontmin.test.mjs` zakazuje menší px v CSS).
- Motiv se mění bez restartu (`theme/apply.ts`, událost `appearance`); světlý motiv: text na `--t1` ploše
  je `--on-light`, nejsilnější text `--t0` (ne `--white`, ta je jen na barevných logách).
- `prefers-reduced-motion` i `appearance.motion.style = none` (`html[data-motion=none]`) vypínají CSS
  animace/přechody; WAAPI morf v `motion.ts` pak běží s délkou 0 (okamžitě, handshake stejný).
- **Karta = `menu` slot**: karty hlavní stránky berou `menu?: JSX.Element` a vykreslí ho na konci `<h4>`; co je
  vidět, rozhoduje jen `layout` (`MainPage` → `layout/model.ts` `shownCards`; karty, které se bez dat schovají
  samy — Běží, akce na Práci, Přihrádka, Právě hraje — jsou tam jako „fakta“). Sekce karty Claude
  (`claudeLimits/Costs/Git/Subagents`) hlídá `ClaudeSessions` (`sectionOn` nad odvozeným `drawer.sections`);
  skrytá útrata se nevykreslí → žádné `costs_refresh`. Nová karta / část → [layout.md](layout.md) (Invarianty).
- Chyby jádra jsou string v jazyce UI (`i18n::tr`/`trf!` v Rustu) → `sayError(e)`; prázdné stavy karet říkají, co udělat (žádná vymyšlená čísla), a kde to jde, mají JEDNU akci (`button.mini`: Přidat projekt, Přidat poznámku, Přidat akci, Přidat kalendář, Nastavit místo…).
- První spuštění: krátká prohlídka 3 bublin (`layout/Tour.tsx`, `ui.tips`) — nikdy modal přes šuplík, nikdy `focus()`.

## Ověření

`npx tsc --noEmit` a `corepack pnpm build` (přes frontu). `corepack pnpm test:ui` = logika palety a akcí, motivy, kolize CSS tříd a třídy bez pravidla (`cssClasses.ts`), překlady (`i18n.test.mjs`); vizuál ověřuje lead naživo.

## Co nečíst

`types/*` celé (hledej konkrétní typ), `motion.ts` (jen když měníš morf), `icons.tsx`, `Settings.tsx` mimo
sekci, kterou měníš.
