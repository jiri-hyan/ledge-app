# Codemapa: rozložení šuplíku — záložky, karty, režim úprav (`layout`)

Co šuplík ukazuje, na které záložce, v jakém pořadí a jak velké. Config `layout` ([config-layout.md](../agents/config-layout.md) →
Rozložení šuplíku). Cituj symbolem, ne číslem řádku.

## Rozhodnutí: jeden model

- `layout` (`pages[] { id, name, icon?, cards[] { id, size, h?, tiles?, look?, limit? } }`, `hiddenParts[]`, `upgrades[]`) je
  JEDINÝ zdroj pravdy. Karta, která není na žádné záložce, je skrytá (katalog ji nabídne). `h` = výška
  1–4 řádky (`--deck-row`), chybí = podle obsahu; `tiles` = volby karty Projekty (zástupci) na té záložce
  (i `folders` = složky zástupců); `look` = vzhled té karty (vlna 8 P, typy v `tilelook.rs`); `limit` = strop řádků
  karet Claude / Repa / Běží (0–50, 0 = bez stropu, chybí = 4; `cap.ts` `capList`, `ShowMore.tsx` `useCap`).
- **Vzhled karty obecně na `.deck-item`** (rozhodnutí vlny 8 P): `Deck` dává položce `data-cl-bg|head|dens|accent`
  a `--accent`, pravidla jsou v `layout.css` (`.deck > .deck-item[…] > :first-child` přebije `html[data-style] .card`,
  hustota přepočítá `--s*` / `--pad-card` × 0.7 / 1.3, skrytá hlavička nechá jen `[data-cmenu]` = ⋯). Funguje tedy
  pro každou kartu, i nové karty jiných modulů; per karta je jen varianta zobrazení (`look.ts` `VIEW_CARDS`:
  Projekty, Repa, Poznámky, Akce → prop `view` → `data-view` v CSS karty). Karta bez `look` = žádný atribut.
- **Jednorázové úpravy po aktualizaci** (`layout::apply_upgrades` v `upgrade_loaded`, značka v `upgrades`):
  `personalProjects` = karta Projekty nahoru na Osobní, když tam nebyla (jen když je na jiné záložce a Osobní
  existuje). Značka se zapíše prvním uložením; co uživatel pak odebere, se nevrací. **Frontend musí
  `upgrades` přenést** (`normalizeLayout`, `clone`) — jinak by se odebrané Projekty vracely (test).
  Výchozí rozložení (`LayoutConfig::default` = `clean_default`, `defaultLayout`) už úpravy má. Pozor: `upgrades` má
  field-level `#[serde(default)]` — struct-level default by ho vzal z výchozího rozložení (= „hotovo“).
- **Mřížka bez děr** (rozhodnutí): `.deck` zůstává CSS grid se sloupci `--deck-cols` (styl), rozpětí karet
  počítá `size.ts` `packSpans` — karty v pořadí záložky, řada, do které se další nevejde, se uzavře a její
  volné sloupce dostanou karty s NEJVĚTŠÍ zvolenou velikostí v té řadě (vlna 9; dřív poslední karta → [M, S]
  vypadalo jako [M, M] a S nic nedělalo). Bez `grid-auto-flow: dense`: přeskládání by v úpravách posouvalo karty
  jinam, než je uživatel položil. Zvolená velikost (S/M/L/XL) je tedy minimum; když volba nemá viditelný efekt,
  řekne to `sizeOutcome` (`cols` málo sloupců → nejbližší možná `fitSize`, `alone` karta sama v řadě, `fill`
  vyplňuje volno) a lišta ukáže `sizeNoteText` — nikdy tiché nic. ⋯ a ⇔ nabízí jen `usableSizes(cols)`.
  Sloupce čte `Deck` z `--deck-cols` při startu a při změně vzhledu (MutationObserver na `<html>`).
- `drawer.sections` a `modes.personalEnabled` zůstaly, ale jsou **odvozené** (`layout::derive` v
  `config::load`, `config::save`, `apply_config`): karta = je na nějaké záložce, část = karta je a část
  není v `hiddenParts`; `personalEnabled` = vždy `true`. Proč: moduly, které podle karty zapínají stahování
  a odběry (`refresh.rs` počasí, `calendar/live.rs`, `costslive.rs`, `media.rs`, `volume.rs`, `battery.rs`)
  čtou dál tatáž pole → „skrytá karta nic nestahuje“ platí beze změny jejich kódu; starší Ledge po
  downgradu vidí stejné karty. Ruční změna `drawer.sections` se přepíše.
- Počasí a kalendář se stahují jen, když je karta na **otevřené** záložce (`layout::page_has(cfg.layout, mode, …)`).
- Režim (`core.mode`, `Snapshot.mode`) = id záložky; neznámé (smazaná záložka) = první (`layout::page_id`).
- **Dynamické karty** (vlna 8): vedle `CARDS` smí být karta s id podle prefixu — dnes jen widget ze skriptu
  `widget:<soubor>` (`layout::is_dynamic` → `widgets::plan::file_of`, UI `isDynamic` / `CardId`). Validace
  hlídá jen tvar, katalog je nabízí podle složky widgetů (`widgets_list`), vykreslí `widgets/WidgetCard.tsx`.
  Do `drawer.sections` se nepromítají. Mapa: [widgets.md](widgets.md).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/layout.rs` | struktura, výchozí rozložení nového configu (`clean_default`: Práce Projekty + Poznámky, Osobní Čas, Právě hraje, Poznámky, Odkazy; fixture `fixtures/layout-default.json` čtou Rust i TS test) a jeho doplnění průvodcem (`with_detected`: Claude / Repa), převod starého configu (`from_legacy` = původní plné rozložení, starý config bez `layout` se nemění), jednorázové úpravy, odvozená pole, validace (i `h` 1–4 a pořadí částí `parts`), záložka pro režim, název pro proužek; testy (tabulka starých configů, migrace Projekty → Osobní, tvar `h`/`tiles`, `load`/`save`) | `LayoutConfig` `LayoutPage` `LayoutCard` `CardSize` `TileOpts` `TileSize` `MAX_H` `UP_PERSONAL_PROJECTS` `CARDS` `PARTS` `BODY` `card_slots` `clean_default` `with_detected` `section_in_default` `derived_sections` `from_legacy` `apply_upgrades` `upgrade_loaded` `derive` `validate` `page` `page_id` `page_has` `title` `row_sizes` |
| `src-tauri/src/tilelook.rs` | typy a validace vzhledu (`TileLook` zástupce, `CardLook` karty, `TileFolder`), kontrast kresby `ink_dark` (test), vlastní obrázky do cache `looks` (WIC); testy (serde výchozí = nic se nepíše, starý config `load → save → load`, tabulky validace, kontrast přes celé RGB, cache) | `TileLook` `LookIcon` `TileShape` `CardLook` `CardBg` `CardDensity` `CardView` `TileFolder` `TILE_COLORS` `CARD_ACCENTS` `validate` `import_image` `image_url` |
| `src/layout/model.ts` | zrcadlo modelu + čistá logika: normalizace, úpravy karet a záložek (s Vrátit), výška, volby dlaždic, katalog, co záložka ukáže, cíl tažení, `withLayout` (jedno pole configu); test `model.test.mjs` (i shoda `CARDS`/`PARTS`/`UP_PERSONAL_PROJECTS`/`MAX_H` s Rustem) | `CARD_IDS` `CARD_INFO` `PARTS` `PART_INFO` `HEIGHTS` `TILE_SIZES` `DEFAULT_TILES` `QUICK_HEIGHTS` `defaultLayout` (čistý začátek) `normalizeLayout` `normalizeTiles` `tilesOf` `moveCard` `setSize` `nextSize` `setHeight` `nextHeight` `setTiles` `showProject` `visibleProjects` `removeCard` `restoreCard` `addCard` `moveToPage` `moveToNewPage` `CARD_SLOTS` `BODY` `partsOf` `orderParts` `setParts` `movePart` `setPartHidden` `catalog` `addPage` `renamePage` `removePage` `restorePage` `movePage` `pageKind` `shownCards` `gridDropIndex` `keyMove` |
| `src/layout/keep.ts` | čistá pravidla „nic se neztratí“ (test `keep.test.mjs`): skrytí / blur / znovuotevření draft nezahodí, zahodí jen Zrušit a odchod na jinou stránku; `escAction` (Esc se změnami se ptá), `showResume` | `discards` `isDirty` `escAction` `showResume` |
| `src/layout/edit.ts` | stav režimu úprav: draft, záložka v úpravách, katalog, lišta Vrátit / nápověda, cíl tažení, zásobník Zpět; Hotovo (s `layoutProblem`) / Zrušit; jedna změna odkudkoli (`applyLayout`) | `beginEdit` `change` `undoLast` `canUndo` `resetToDefault` `finishEdit` `cancelEdit` `resumeEdit` `escEdit` `offerUndo` `saveLayout` `applyLayout` `askEdit` `editAsk` |
| `src/layout/history.ts` · `drag.ts` · `validate.ts` · `cap.ts` · `cols.ts` | vlna 9, čisté funkce: Zpět (`pushUndo` / `popUndo`, `resetLayout` drží `upgrades`) · tah (`dropAction` — zrušení nic nemění, na zónu `hide`, do mřížky jiné záložky `insert` / `blocked`; `previewOrder`, `autoScrollStep`, `keyAction`) · zrcadlo `layout::validate` (fixture `src-tauri/src/fixtures/layout-validate.json` čtou oba testy) · strop řádků (`capList`, `capKey`) · sloupce teď (`colsNow`) | testy `history` `drag` `validate` `cap` `wiring` `.test.mjs` |
| `src/layout/dnd.ts` · `cardDrag.ts` | kolo 10, přirozený drag & drop karet. `dnd.ts` = čisté funkce (test `dnd.test.mjs`): podržení (`holdStep`, `canHold`, `HOLD_MS` 400 / `HOLD_SLOP` 6 px, selektory hlavičky a ovládacích prvků), cíl vložení na jiné záložce (`insertIndex`, `insertAt`), prodleva nad ouškem (`dwellStep`, `DWELL_MS` 600), zóna (`inBox`), souřadnice mřížky (`contentPoint`), duch (`grabOffset`, `flyPos`), FLIP (`flipDeltas`, `flipPlan`), přistání ducha (`landTransforms`, `offsetRect`), záložky (`NEW_TAB`, `nearestCenter`) a popisek zóny (`dropHint`). `cardDrag.ts` = řadič gesta (`createCardDrag`): pointer události na `window`, měření `offsetLeft/Top`, duch = klon `.deck-item.lay-fly`, po puštění dosedne pružinou (`flip.ts`), rAF jen během tahu | `createCardDrag` `Drag` `DragEnv` |
| `src/layout/flip.ts` | kolo 13, sdílená mechanika tahu v DOM (karty, záložky, části — jeden motor): `flipEls` = FLIP odsun prvků (id `lay-flip`, `flipPlan` podle `motionNow` + reduced-motion), `landGhost` = duch po puštění dosedne na mezeru pružinou (`motion.ts` `spring`; jen transform + opacity), bez cíle / bez animací zhasne | `flipEls` `landGhost` `reducedMotion` |
| `src/layout/tabDrag.ts` | kolo 13, tah záložky (`createTabDrag`): v úpravách od `GRAB_PX`, mimo ně podržením `HOLD_MS` (zapne úpravy); duch `.lp-tabfly` vodorovně, mezera `.lp-gap`, náhled CSS `order`, FLIP, Esc / ztráta ukazatele zruší, klik po tahu se spolkne; bez rAF | `createTabDrag` `TabDragState` `TabDragEnv` |
| `src/layout/PartSlot.tsx` · `partDrag.ts` · `parts.ts` · `dnd.css` | kolo 13, části (sekce) karet. `PartsBox` + `PartSlot` obalují části karty (Claude: limity, obsah `body`, útrata; Právě hraje: skladba, hlasitost; Čas: obsah, baterie) — pořadí CSS `order` z `layout…parts`, skrytá část se nekreslí ani nespouští, v úpravách lišta `.lp-edit` (úchyt ⠿, název, u skryté „Ukázat“), mimo ně nadpis `.lp-head` se sbalením klikem (`titled`: Limity, Hlasitost; stav v paměti, `grid-template-rows`). Výchozí pořadí mimo úpravy = obaly `display: contents` (vzhled karty beze změny). `partDrag.ts` = řadič tahu části za lištu (`createPartDrag`: duch `.lp-fly`, mezera, FLIP, zóna Skrýt → `hiddenParts`, `body` se skrýt nedá, rAF jen v tahu). `Deck` dává obalům stav přes `PartsCtx` (`PartsCtl`). `parts.ts` čisté: `partKeyAction` (šipky přesun, Delete skrýt), `toggleFold`. Test `parts.test.mjs` | `PartsBox` `PartSlot` `PartsCtx` `PartsCtl` `createPartDrag` `PartDrag` `partKeyAction` `foldKey` `toggleFold` |
| `src/layout/ShowMore.tsx` | „Zobrazit další (k) · souhrn“ ↔ „Méně“ a háček `useCap` (karty Claude, Repa, Běží); rozbalení v paměti per záložka + karta | `ShowMore` `useCap` |
| `src/layout/Deck.tsx` · `layout.css` | mřížka záložky `.deck` > `.deck-item[data-size][data-card][data-h]` (rozhraní pro styly: `--deck-cols`, výchozí 4; rozpětí `--span` z `packSpans`, výška `--h` × `--deck-row`, obsah s výškou scrolluje uvnitř), v úpravách tah celou kartou (`onPointerDown` na `.deck-item` → `cardDrag.down`; úchyt ⠿ zůstává pro klávesy), mimo úpravy podržení hlavičky (prop `begin` zapne úpravy), mezera `.lifted` / `.lay-slot`, duch `.lay-fly`, zóna `.lay-zone`, **úchyty okrajů `.lay-rs-e/-s/-se`** (tah = šířka / výška / obojí, obrys `.lay-target` s „2 × 3“, Esc zruší, dvojklik spodní = auto), ⇔, ↕, ×, zástupce karty bez dat, „+ Přidat kartu“, klidný přerušovaný obrys bez pohybu | `Deck` `sizeLabel` `heightLabel` |
| `src/layout/size.ts` | čistá mechanika mřížky: rozpětí velikosti, řada bez děr, co udělá volba velikosti, cíl tahu za okraj, px ↔ řádky, kroky velikosti / výšky; test `size.test.mjs` | `spanOf` `packSpans` `usableSizes` `fitSize` `sizeOutcome` `nextSizeFor` `stepSize` `stepHeight` `sizeForSpan` `resizeTo` `rowsForPx` `heightPx` `deckCols` |
| `src/layout/Tour.tsx` · `tips.ts` · `tour.css` | prohlídka 3 bublin (✎ `data-tip="edit"`, ⋯ první karty, 🔍 `data-tip="palette"`) při prvním otevření; viditelnost z událostí `view`/`closing`, stav `ui.tips` (`tour1`); čistá logika + test `tips.test.mjs` | `Tour` `TOUR_ID` `STEPS` `shouldShow` `withTip` `bubblePos` |
| `src/layout/HoldTip.tsx` | jednorázový tip „Podrž kartu a přetáhni ji“ (`ui.tips` `hold1`, `tips.ts` `shouldShowHold`): až po prohlídce, při prvním najetí na hlavičku karty; zavře ho „Rozumím“, 9 s, ✎ nebo první tah podržením (`edit.ts` `holdDrags`) | `HoldTip` `HOLD_TIP` `HOLD_TEXT` |
| `src/layout/Tabs.tsx` | záložky v hlavičce (N), v úpravách přejmenovat dvojklikem, smazat, pořadí tažením stejným motorem jako karty (`tabDrag.ts`; Alt + ← →), nová záložka s ikonou, „+“ (`data-newtab`) jako cíl tažené karty (`renameAsk` → hned přejmenovat); cíl tažení karty (`data-tab`, zvýraznění + proužek prodlevy `.lay-tab.drop::after`); mimo úpravy tužka „Upravit rozložení“ při najetí na záložky | `Tabs` `movedTab` |
| `src/layout/Catalog.tsx` · `EditBar.tsx` | katalog karet (název, popis, kde je) · lišta úprav (výrazně „Táhni za okraj karty = velikost“, Zrušit / Hotovo) + Vrátit | |
| `src/layout/LayoutEditor.tsx` | editor stejného modelu v Nastavení (panel Šuplík a karty, `SortList`): velikost, výška, u Projektů volby dlaždic a které projekty, části karet, „Upravit přímo v šuplíku“ | |
| `src/cards/CardMenu.tsx` · `TileOptions.tsx` | ⋯ karty (společné menu `ui/Popover.tsx` `PopMenu`; kořen `[data-cmenu]`): Velikost S/M/L/XL, Výška Auto/Nízká/Vysoká, u Projektů Dlaždice (Malé ikony/Dlaždice/Velké), název, git, které projekty, „Vzhled karty…“; ukládá hned (`saveLayout`) | `CardMenu` `MenuChoice` `TileOptions` `tileSizeName` |
| `src/layout/text.ts` | lidské názvy z párů v modelu (překlad až při vykreslení) | `tx` `cardName` `partName` `pageName` |
| `src/layout/look.ts` · `lookState.ts` · `CardLook.tsx` | vzhled karty: čistá logika (normalizace, varianty `VIEW_CARDS`, atributy pro `.deck-item`, přehled, Obnovit vše; test `look.test.mjs`) · stav bez UI (eager: `lookFor`, `openCardLook`, `closeCardLook`; potřebují je `CardMenu`, `Deck`, `MainPage`, `VariantPicker`) · panel „Vzhled karty…“ pod mřížkou, **líný chunk** přes `lazyViews.tsx` (živý náhled `lookFor`, Hotovo / klik mimo = uložit, v úpravách do draftu, Esc = zahodit) | `normalizeCardLook` `patchCardLook` `cardLookAttrs` `setCardLook` `viewOf` `customizedCards` `resetCardLooks` · `openCardLook` `closeCardLook` `lookFor` (`lookState.ts`) · `CardLookPanel` |
| `src/layout/folders.ts` | složky zástupců (`tiles.folders`): tah dlaždice na dlaždici = složka, na složku = do ní, vyjmout, prázdná zmizí, uspořádání dlaždic; test `folders.test.mjs` | `normalizeFolders` `dropOnTile` `addToFolder` `takeOut` `renameFolder` `removeFolder` `arrange` `folderOf` |
| `src/layout/LooksOverview.tsx` | Nastavení → Šuplík a karty → „Upravené karty a zástupci“ (Obnovit jednotlivě / vše, draft Nastavení) | |

## Toky

- **Vykreslení**: `MainPage` → `normalizeLayout(s.layout)` (memo podle obsahu) → `pageOf(mode)` → `Deck`
  (`For` podle id karty — nový snapshot ani změna velikosti kartu nepřestaví). Karty bez dat (`shownCards`:
  Běží bez procesů, akce na Práci bez akcí, prázdná přihrádka, Právě hraje bez dat, vypnuté poznámky) se
  schovají; v úpravách jsou zástupce.
- **Úpravy**: ✎ v hlavičce / paleta „Upravit šuplík“ / ⋯ „Upravit šuplík…“ / prázdná záložka → `beginEdit`
  (`focusDrawer`; šuplík nezavře odjetí myši) → změny jen v draftu → Hotovo: `get_config` → `withLayout` →
  `save_config` → `set_mode(záložka)`; Zrušit / výslovný odchod na jinou stránku = zahodit; **zavření šuplíku (odjetí myši, blur, skrytí) draft ani Zpět nezahodí** — žijí v modulu `edit.ts` (WebView zůstává teplý), po znovuotevření `resumeEdit` vrátí fokus (odjetí myši nezavře) a lišta na 10 s připomene „Máš neuložené úpravy rozložení — Hotovo uloží, Zrušit zahodí“. Esc mimo tah se změnami se zeptá „Zahodit?“. Restart / zničení WebView (úsporný režim) draft zahodí (jen paměť, bez sessionStorage).
- **Velikost karty**: v úpravách tah za okraj (`Deck` `resize` → `size.ts` `resizeTo`; změří mřížku jednou při
  chycení, sonda `.lay-probe` pro `--deck-row`) → obrys → pustit = `setSize` / `setHeight` v draftu; mimo
  úpravy ⋯ → `saveLayout`. Projekty: `MainPage` předá kartě `tilesOf(karta)`, dlaždice filtruje `visibleProjects`.
- **Prohlídka**: `MainPage` → `Tour` (mimo úpravy) → jednou `get_config` → `shouldShow` → po otevření šuplíku
  (`view` = full) a 700 ms bubliny; Hotovo / Přeskočit → `withTip` → `save_config`.
- **Vzhled karty**: ⋯ → „Vzhled karty…“ → `openCardLook` → panel `CardLookPanel` (MainPage pod `Deck`) → změny jen
  v draftu panelu (`lookFor` → `Deck` atributy, `viewOf` → prop `view` karet) → Hotovo / klik mimo: mimo úpravy
  `saveLayout(setCardLook)`, v úpravách `change`. Jiná záložka / konec úprav = uložit a zavřít (MainPage).
- **Složky zástupců**: jen v režimu úprav (`Projects.tsx` `startDrag`: posluchače na `window` bez pointer capture —
  klik bez pohybu musí dojít na tlačítko dlaždice = „Upravit vzhled…“) → `dropOnTile` / `addToFolder` / `takeOut`
  → `change` (s Vrátit); mimo úpravy přejmenování / zrušení / vyjmutí (pravý klik) → `saveLayout`.
- **Klávesy**: 1–9 = záložka (App), v úpravách Esc = Zrušit (se změnami po potvrzení; během tahu karty / za okraj jen zruší tah), Ctrl+Z = Zpět;
  na úchytu ⠿ šipky = přesun, + / − = šířka, PageUp / PageDown = výška (`drag.ts` `keyAction`).
- **Tah karty** (vlna 9, kolo 10): vstup `cardDrag.down` z `onPointerDown` na `.deck-item`. V úpravách se karta chytí na prázdném
  místě (obsah má `pointer-events: none` → cíl je sama položka; tlačítka a okraje `EDIT_CONTROL_SELECTOR` ne; dlaždice
  s vlastní úpravou si tah berou samy) nebo za ⠿, od 4 px. Mimo úpravy podržení `HOLD_MS` na hlavičce (`HEADER_SELECTOR`, ne na
  `CONTROL_SELECTOR`): posun nad `HOLD_SLOP` před časem = obyčejný klik / výběr textu; po čase `begin` (`beginEdit`) a
  rovnou tah (místo chycení se zapamatuje ještě před zapnutím lišty úprav; následný klik se spolkne). Pointer na `window`
  (ne capture — náhled by ho ztratil); cíl = karta pod kurzorem v PŮVODNÍM pořadí (`gridDropIndex`, rámečky z
  `offsetLeft/Top` = souřadnice obsahu mřížky, nezávislé na posunu i rozběhlém FLIPu; mezera = nejbližší obrys; daleko mimo =
  nic); náhled = CSS `order` (DOM se nepřeskládá), tažená karta je v původním místě `.lifted` jako mezera velikosti karty
  a duch (klon `.deck-item.lay-fly`, `pointer-events: none`) jede pod kurzorem; FLIP `el.animate` (id `lay-flip`, rozběhlý se
  před novým měřením zruší) podle `flipPlan(motionNow, reduced-motion)`; smyčka rAF JEN během tahu (posun u okraje — nad
  zónou ne — a `dwellStep`). Cíle: zóna `.lay-zone` dole (= `remove`, vratné), ouško záložky (`dropTab`; puštění = na konec),
  prodleva `DWELL_MS` nad ouškem → `setEditPage`, `rects` se přeměří a tah pokračuje v mřížce té záložky (`insertIndex`,
  zástupná mezera `.lay-slot`, puštění = `moveToPageAt`); karta, která tam už je, se nestěhuje (`blocked` + nápověda).
  Konec `dropAction`: Esc / pointercancel / pustit mimo okno (`buttons === 0`) = nic (vrátí se i na domovskou záložku); po
  dopadu `offerUndo` „… · Vrátit“ (přesun v záložce = inverzní `moveCard`).
- **Tah části karty** (kolo 13): vstup `PartsCtl.down` z `onPointerDown` na liště `.lp-edit` (jen v úpravách; karta se na ní netahá —
  cíl není sama položka). Cíl = nejbližší obrys části v rámci karty (`gridDropIndex`, rámečky z `offsetLeft/Top` obalu `.lp-box`),
  náhled `previewOrder` + FLIP; zóna Skrýt (sdílená `.lay-zone`) = `setPartHidden` (jen části, ne `body`); puštění `movePart`
  s „Vrátit“. Klávesy na úchytu `.lp-grip`: šipky přesun, Delete skrýt. `claudeGit` a `claudeSubagents` se přesouvat nedají
  (kreslí se u každé session zvlášť) — jen skrýt přes ⋯ / Nastavení.
- **Záložky a nová záložka** (kolo 13): tah záložky `tabDrag.ts` (viz tabulka); karta puštěná na „+“ (`dropTab === NEW_TAB`)
  → `Deck.toNewPage` = `moveToNewPage` („Nová záložka“, karta do ní) + `setEditPage` + `setRenameAsk` → `Tabs` otevře přejmenování.
  Popisky při tahu karty (`dropHint`): „Skrýt kartu“, „Na záložku X“, „Nová záložka“, „už tam je“; v mezeře cíle `data-dim`
  („2 × 3“ = výsledná velikost karty).
- **Nastavení**: panel Šuplík a karty → `LayoutEditor` mění draft Nastavení (`c.set("layout")`), uloží
  společná lišta. Pořadí karet ve Vzhledu už není.

## Invarianty a pasti

- HTML5 drag&drop v okně Tauri nefunguje → jen pointer události (`cardDrag`, `Deck`, `Tabs`, `SortList`); hlídá `wiring.test.mjs`.
- Nové pole karty = `layout.rs` + `model.ts` `normalizeLayout` + `validate.ts` + řádek ve fixture `layout-validate.json`.
- Nová přetahovatelná část karty = klíč do `layout::card_slots` + `model.ts` `CARD_SLOTS` (test `model.test.mjs` hlídá shodu) +
  `<PartSlot card k>` v kartě; třídy nové CSS mají prefix `lp-` v `dnd.css` (test kolizí tříd: jedna třída = jeden soubor).
- Nová dynamická rodina karet = prefix v `layout::is_dynamic` + `model.ts` `isDynamic` + větev `default` v `MainPage` `card`.
- Nová karta = id do `layout::CARDS` + `model.ts` `CARD_IDS`/`CARD_INFO` + případně pole `sections!`
  v `config/*` (+ `SECTION_KEYS`) + větev v `MainPage` `card`. Nová část = `PARTS` v obou + pole `sections!`.
- Vzhled karet (barvy, rohy, šířky podle stylu) patří stylům; `layout.css` drží jen mechaniku mřížky a ovládání úprav.
- Výchozí rozložení nového configu je záměrně malé (`clean_default`); karta bez zdroje (Kalendář, Pošta, GitHub, Počasí bez místa) nebo
  duplicitní (Rychlé akce vedle Odkazů) v něm není a katalog ji nabídne. `Sections::default()` se z něj odvozuje (`section_in_default`, bez serde —
  `serde(default)` by se zacyklil), takže `Config::default()` sedí s načteným. Karta Dnes (`today`) je také jen v katalogu a nekreslí se bez řádků (`Facts.today`; [suggest.md](suggest.md)). Karta `OPT_IN` (GitHub, Systém, Pošta) = id i v `layout::OPT_IN` (hlásí ji MCP). Detail [integrations.md](integrations.md).
- Vzhled karet (barvy, rohy, šířky podle stylu) patří stylům; `layout.css` drží mechaniku mřížky, ovládání úprav a
  jen PŘEBITÍ stylu pro jednu kartu z `look` (`data-cl-*`) — bez atributu se pravidla neuplatní.
- Akce dlaždice projektu jsou UVNITŘ dlaždice (`.acts` absolutně dole přes název a větev) — nikdy přes kartu
  pod ní; nová akce = ikona s `title`/`aria-label`, ne text (dlaždice je úzká).
- Menu a bubliny nad kartami kresli přes `Portal` do `.shell` s `position: fixed` (karta s výškou má
  `overflow: auto` a okno šuplíku je jen tak vysoké jako obsah).
- `src/theme/appearance.ts` `cardOrder` / `CARD_IDS` / `cards` jsou po vlně 6 mrtvé (Rust pole `appearance.cards` už nemá).

## Testy

`cargo test layout` (výchozí = čistý začátek + fixture, `with_detected`, starý config bez `layout` beze změny, převod starých configů — tabulka se skutečnými tvary, velikosti
řady, migrace Projekty → Osobní včetně „odebrané se nevrací“ přes `load` → `save` → `load`, tvar `h`/`tiles`,
odvozená pole, validace), `cargo test uiconf` (`ui.tips`). `corepack pnpm test:ui` → `src/layout/model.test.mjs`,
`size.test.mjs` (řada bez děr, tah za okraj), `tips.test.mjs` (prohlídka + tip o podržení), `dnd.test.mjs` + `drag.test.mjs`
(podržení vs klik, cíle tahu, prodleva nad ouškem, zóna, FLIP), `wiring.test.mjs` (jen pointer, rAF jen v tahu, zapojení),
`src/cards/sections.test.mjs` (shoda polí `drawer.sections` s modelem), `look.test.mjs` (karta bez vzhledu = žádný
atribut, varianty, zrcadlo `CARD_ACCENTS`), `folders.test.mjs` (vytvoření / vyjmutí / smazání prázdné), hlídače CSS
tříd a překladů. `cargo test tilelook` (starý config `load → save → load` beze změny, vzhledy tam a zpět).
