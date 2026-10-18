# Codemapa: rozložení šuplíku — záložky, karty, režim úprav (`layout`)

Co šuplík ukazuje, na které záložce, v jakém pořadí a jak velké. Config `layout` (AGENTS.md →
Konfigurace). Cituj symbolem, ne číslem řádku.

## Rozhodnutí: jeden model

- `layout` (`pages[] { id, name, icon?, cards[] { id, size } }`, `hiddenParts[]`) je JEDINÝ zdroj pravdy.
  Karta, která není na žádné záložce, je skrytá (katalog ji nabídne).
- `drawer.sections` a `modes.personalEnabled` zůstaly, ale jsou **odvozené** (`layout::derive` v
  `config::load`, `config::save`, `apply_config`): karta = je na nějaké záložce, část = karta je a část
  není v `hiddenParts`; `personalEnabled` = vždy `true`. Proč: moduly, které podle karty zapínají stahování
  a odběry (`refresh.rs` počasí, `calendar/live.rs`, `costslive.rs`, `media.rs`, `volume.rs`, `battery.rs`)
  čtou dál tatáž pole → „skrytá karta nic nestahuje“ platí beze změny jejich kódu; starší Ledge po
  downgradu vidí stejné karty. Ruční změna `drawer.sections` se přepíše.
- Počasí a kalendář se stahují jen, když je karta na **otevřené** záložce (`layout::page_has(cfg.layout, mode, …)`).
- Režim (`core.mode`, `Snapshot.mode`) = id záložky; neznámé (smazaná záložka) = první (`layout::page_id`).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/layout.rs` | struktura, výchozí rozložení, převod starého configu, odvozená pole, validace, záložka pro režim, název pro proužek; testy (tabulka starých configů, `load`/`save`) | `LayoutConfig` `LayoutPage` `LayoutCard` `CardSize` `CARDS` `PARTS` `from_legacy` `upgrade_loaded` `derive` `validate` `page` `page_id` `page_has` `title` `row_sizes` |
| `src/layout/model.ts` | zrcadlo modelu + čistá logika: normalizace, úpravy karet a záložek (s Vrátit), katalog, co záložka ukáže, cíl tažení, `withLayout` (jedno pole configu); test `model.test.mjs` (i shoda `CARDS`/`PARTS` s Rustem) | `CARD_IDS` `CARD_INFO` `PARTS` `PART_INFO` `defaultLayout` `normalizeLayout` `moveCard` `setSize` `nextSize` `removeCard` `restoreCard` `addCard` `moveToPage` `setPartHidden` `catalog` `addPage` `renamePage` `removePage` `restorePage` `movePage` `pageKind` `shownCards` `gridDropIndex` `keyMove` |
| `src/layout/edit.ts` | stav režimu úprav: draft, záložka v úpravách, katalog, lišta Vrátit, cíl tažení; Hotovo / Zrušit; jedna změna mimo úpravy (⋯) | `beginEdit` `change` `finishEdit` `cancelEdit` `offerUndo` `saveLayout` `askEdit` `editAsk` |
| `src/layout/Deck.tsx` · `layout.css` | mřížka záložky `.deck` > `.deck-item[data-size][data-card]` (rozhraní pro styly: `--deck-cols`, výchozí 4), v úpravách úchyt ⠿ (pointer, i na záložku), ⇔, ×, zástupce karty bez dat, „+ Přidat kartu“, rozvlnění | `Deck` |
| `src/layout/Tabs.tsx` | záložky v hlavičce (N), v úpravách přejmenovat dvojklikem, smazat, pořadí tažením, nová záložka s ikonou; cíl tažení karty (`data-tab`) | `Tabs` |
| `src/layout/Catalog.tsx` · `EditBar.tsx` | katalog karet (název, popis, kde je) · lišta úprav (Zrušit / Hotovo) + Vrátit | |
| `src/layout/LayoutEditor.tsx` | editor stejného modelu v Nastavení (panel Šuplík a karty, `SortList`), části karet, „Upravit přímo v šuplíku“ | |
| `src/layout/text.ts` | lidské názvy z párů v modelu (překlad až při vykreslení) | `tx` `cardName` `partName` `pageName` |

## Toky

- **Vykreslení**: `MainPage` → `normalizeLayout(s.layout)` (memo podle obsahu) → `pageOf(mode)` → `Deck`
  (`For` podle id karty — nový snapshot ani změna velikosti kartu nepřestaví). Karty bez dat (`shownCards`:
  Běží bez procesů, akce na Práci bez akcí, prázdná přihrádka, Právě hraje bez dat, vypnuté poznámky) se
  schovají; v úpravách jsou zástupce.
- **Úpravy**: ✎ v hlavičce / paleta „Upravit šuplík“ / ⋯ „Upravit šuplík…“ / prázdná záložka → `beginEdit`
  (`focusDrawer`; šuplík nezavře odjetí myši) → změny jen v draftu → Hotovo: `get_config` → `withLayout` →
  `save_config` → `set_mode(záložka)`; Zrušit / Esc / zavření šuplíku / jiná stránka = zahodit.
- **Klávesy**: 1–9 = záložka (App), v úpravách Esc = Zrušit, Alt+šipky na úchytu = přesun karty.
- **Nastavení**: panel Šuplík a karty → `LayoutEditor` mění draft Nastavení (`c.set("layout")`), uloží
  společná lišta. Pořadí karet ve Vzhledu už není.

## Invarianty a pasti

- HTML5 drag&drop v okně Tauri nefunguje → jen pointer události (`Deck`, `Tabs`, `SortList`).
- Nová karta = id do `layout::CARDS` + `model.ts` `CARD_IDS`/`CARD_INFO` + případně pole `sections!`
  v `config.rs` (+ `SECTION_KEYS`) + větev v `MainPage` `card`. Nová část = `PARTS` v obou + pole `sections!`.
- Vzhled karet (barvy, rohy, šířky podle stylu) patří stylům; `layout.css` drží jen mechaniku mřížky a ovládání úprav.
- `src/theme/appearance.ts` `cardOrder` / `CARD_IDS` / `cards` jsou po vlně 6 mrtvé (Rust pole `appearance.cards` už nemá).

## Testy

`cargo test layout` (výchozí = dnešní šuplík, převod starých configů — tabulka se skutečnými tvary, velikosti
řady, odvozená pole, validace, `load` → `save` → `load`). `corepack pnpm test:ui` → `src/layout/model.test.mjs`,
`src/cards/sections.test.mjs` (shoda polí `drawer.sections` s modelem), hlídače CSS tříd a překladů.
