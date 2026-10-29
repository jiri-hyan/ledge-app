# Codemapa: rozložení šuplíku — záložky, karty, režim úprav (`layout`)

Co šuplík ukazuje, na které záložce, v jakém pořadí a jak velké. Config `layout` ([config-layout.md](../agents/config-layout.md) → Rozložení šuplíku). Cituj symbolem, ne číslem řádku.

Detail (toky, invarianty, testy, co nečíst, historie): [layout-detail.md](layout-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/layout.rs` | struktura, výchozí rozložení nového configu (`clean_default`: Práce Projekty + Poznámky, Osobní Čas, Právě hraje, Poznámky, Odkazy … | `LayoutConfig` `LayoutPage` `LayoutCard` `CardSize` `TileOpts` |
| `src-tauri/src/tilelook.rs` | typy a validace vzhledu (`TileLook` zástupce, `CardLook` karty, `TileFolder`), kontrast kresby `ink_dark` (test), vlastní obrázky do cache `looks` (WIC) … | `TileLook` `LookIcon` `TileShape` `CardLook` `CardBg` `CardDensity` |
| `src/layout/model.ts` | zrcadlo modelu + čistá logika: normalizace, úpravy karet a záložek (s Vrátit), výška, volby dlaždic, katalog, co záložka ukáže, cíl tažení, `withLayout` (jedno … | `CARD_IDS` `CARD_INFO` `PARTS` `PART_INFO` `HEIGHTS` `TILE_SIZES` |
| `src/layout/keep.ts` | čistá pravidla „nic se neztratí“ (test `keep.test.mjs`): skrytí / blur / znovuotevření draft nezahodí, zahodí jen Zrušit a odchod na jinou stránku … | `discards` `isDirty` `escAction` `showResume` |
| `src/layout/edit.ts` | stav režimu úprav: draft, záložka v úpravách, katalog, lišta Vrátit / nápověda, cíl tažení, zásobník Zpět; Hotovo (s `layoutProblem`) / Zrušit … | `beginEdit` `change` `undoLast` `canUndo` `resetToDefault` |
| `src/layout/drawerSize.ts` · `ResizeHandles.tsx` · `drawerSize.css` | kolo 23, velikost šuplíku tažením v úpravách: úchyty na bocích (šířka; na středové ose symetricky, u hrany jen volný bok, notch bez boků) a dole (výška) … | `startResize` `resetEdge` `keyResize` `sizeNow` `mainHeight` |
| `src/layout/history.ts` · `drag.ts` · `validate.ts` · `cap.ts` · `cols.ts` | vlna 9, čisté funkce: Zpět (`pushUndo` / `popUndo`, `resetLayout` drží `upgrades`) · tah (`dropAction` … | testy `history` `drag` `validate` `cap` `wiring` `.test.mjs` |
| `src/layout/dnd.ts` · `cardDrag.ts` | kolo 10, přirozený drag & drop karet. `dnd.ts` = čisté funkce (test `dnd.test.mjs`) … | `createCardDrag` `Drag` `DragEnv` |
| `src/layout/flip.ts` | kolo 13, sdílená mechanika tahu v DOM (karty, záložky, části — jeden motor): `flipEls` = FLIP odsun prvků (id `lay-flip`, `flipPlan` podle `motionNow` + … | `flipEls` `landGhost` `reducedMotion` |
| `src/layout/tabDrag.ts` | kolo 13, tah záložky (`createTabDrag`): v úpravách od `GRAB_PX`, mimo ně podržením `HOLD_MS` (zapne úpravy) … | `createTabDrag` `TabDragState` `TabDragEnv` |
| `src/layout/PartSlot.tsx` · `partDrag.ts` · `parts.ts` · `dnd.css` | kolo 13, části (sekce) karet. `PartsBox` + `PartSlot` obalují části karty (Claude: limity, obsah `body`, útrata; Právě hraje: skladba, hlasitost; Čas … | `PartsBox` `PartSlot` `PartsCtx` `PartsCtl` `createPartDrag` |
| `src/layout/fullLayout.mjs` | pomůcka testů: ÚPLNÉ rozložení (všechny běžné karty) — testy úprav potřebují co upravovat; výchozí pro nového uživatele hlídá `model.test.mjs` | `fullLayout` |
| `src/layout/ShowMore.tsx` | „Zobrazit další (k) · souhrn“ ↔ „Méně“ a háček `useCap` (karty Claude, Repa, Běží); rozbalení v paměti per záložka + karta | `ShowMore` `useCap` |
| `src/layout/Deck.tsx` · `layout.css` | mřížka záložky `.deck` > `.deck-item[data-size][data-card][data-h]` (rozhraní pro styly: `--deck-cols`, výchozí 4 … | `Deck` `sizeLabel` `heightLabel` |
| `src/layout/size.ts` | čistá mechanika mřížky: rozpětí velikosti, řada bez děr, co udělá volba velikosti, cíl tahu za okraj, px ↔ řádky, kroky velikosti / výšky; test `size.test.mjs` | `spanOf` `packSpans` `usableSizes` `fitSize` `sizeOutcome` |
| `src/layout/Tour.tsx` · `tips.ts` · `tour.css` | prohlídka 3 bublin (✎ `data-tip="edit"`, ⋯ první karty, 🔍 `data-tip="palette"`) při prvním otevření … | `Tour` `TOUR_ID` `STEPS` `shouldShow` `withTip` `bubblePos` |
| `src/layout/HoldTip.tsx` | jednorázový tip „Podrž kartu a přetáhni ji“ (`ui.tips` `hold1`, `tips.ts` `shouldShowHold`): až po prohlídce, při prvním najetí na hlavičku karty … | `HoldTip` `HOLD_TIP` `HOLD_TEXT` |
| `src/layout/Tabs.tsx` | záložky v hlavičce (N), v úpravách přejmenovat dvojklikem, smazat, pořadí tažením stejným motorem jako karty (`tabDrag.ts` … | `Tabs` `movedTab` |
| `src/layout/Catalog.tsx` · `EditBar.tsx` | katalog karet (název, popis, kde je) · lišta úprav (výrazně „Táhni za okraj karty = velikost“, Zrušit / Hotovo) + Vrátit |  |
| `src/layout/LayoutEditor.tsx` | editor stejného modelu v Nastavení (panel Šuplík a karty): lišta záložek (klik = aktivní, ← → pořadí, ikony jako dlaždice), živý náhled mřížky (`packSpans` se … | `LayoutEditor` |
| `src/cards/CardMenu.tsx` · `TileOptions.tsx` | ⋯ karty (společné menu `ui/Popover.tsx` `PopMenu`; kořen `[data-cmenu]`): Velikost S/M/L/XL, Výška Auto/Nízká/Vysoká, u Projektů Dlaždice (Malé … | `CardMenu` `MenuChoice` `TileOptions` `tileSizeName` |
| `src/layout/text.ts` | lidské názvy z párů v modelu (překlad až při vykreslení) | `tx` `cardName` `partName` `pageName` |
| `src/layout/look.ts` · `lookState.ts` · `CardLook.tsx` | vzhled karty: čistá logika (normalizace, varianty `VIEW_CARDS` a `viewsOf` (Bento jen Projekty / Repa / Poznámky), atributy pro `.deck-item`, přehled, Obnovit … | `normalizeCardLook` `patchCardLook` `cardLookAttrs` `setCardLook` |
| `src/layout/hero.ts` · `HeroCard.tsx` | kolo 22, karty 2026: plocha `surface` (`glass` = sklo jen na rámu a hlavičce, pod obsahem plná plocha `::after` s poloměrem vnější − odsazení … | `heroFacts` `costSeries` `HeroFacts` `HeroCard` `DeckHero` |
| `src/layout/designs.ts` · `DesignPicker.tsx` · `designs.css` | kolo 25, registr vzhledů karet (`CardLook.design`): `CARD_DESIGNS[karta]` = seznam `{ id, name, line }`, první = výchozí dnešní vzhled (id se nezapisuje), `designsOf` / `designOf` / `storedDesign`; výběr dlaždicemi s mini náhledem v panelu Vzhled karty i v Nastavení → Šuplík a karty; vykreslení dělá komponenta karty (Claude `cards/ClaudeDesign.tsx`, Systém `integrations/SystemDesign.tsx`); test `designs.test.mjs`, angličtina `i18n/en-designs.ts` | `CARD_DESIGNS` `designsOf` `designOf` `storedDesign` `DesignPicker` |
| `src/layout/bento.ts` | kolo 22, bento záložka (`pages[].bento`): dlaždice 1×1 / 2×1 / 2×2 = `size` × `h` (v bento chybí = 2), `tileOf` / `setTile` (⋯ karty → Dlaždice), `setBento` … | `TILES` `TILE_DIM` `tileOf` `setTile` `setBento` `TEMPLATES` |
| `src/layout/CardsGallery.tsx` · `gallery.css` | kolo 22, Nastavení → Vzhled → Otevřený: galerie „Karty 2026“ (6 předvoleb `LOOK_PRESETS` na všechny karty vybrané záložky, `applyLookPreset`, zvýraznění … | `CardsGallery` `CardsMini` `peekingCards` |
| `src/layout/folders.ts` | složky zástupců (`tiles.folders`): tah dlaždice na dlaždici = složka, na složku = do ní, vyjmout, prázdná zmizí, uspořádání dlaždic; test `folders.test.mjs` | `normalizeFolders` `dropOnTile` `addToFolder` `takeOut` `renameFolder` |
| `src/layout/LooksOverview.tsx` | Nastavení → Šuplík a karty → „Upravené karty a zástupci“ (Obnovit jednotlivě / vše, draft Nastavení) |  |

Další soubory (podrobnosti v detailu):
`src/onboarding/steps/Projects.tsx` · `src/settings/panels/Projects.tsx`
