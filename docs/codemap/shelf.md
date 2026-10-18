# Codemapa: přihrádka na soubory (`shelf`)

Soubory přetažené z Průzkumníka na proužek u horní hrany se uloží do přihrádky — jen **odkazy**
(cesty), nic se nekopíruje ani nepřesouvá. V šuplíku karta „Přihrádka“: kopírovat cestu (prostá /
zmínka pro Claude Code), otevřít, ukázat ve složce, odebrat, vyprázdnit, přetáhnout ven. Config
`shelf.enabled` (výchozí `true`, Nastavení → Šuplík a karty). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/shelf/mod.rs` | čistá logika + úložiště `%APPDATA%\Ledge\shelf.json` (líně při prvním použití, zápis přes `.tmp` + rename), config, překryv proužku, stav „tah nad proužkem“, testy | `ShelfConfig` `Item` `add` `remove` `normalize` `same` `claude_ref` `clip_text` `Format` `split` `parse` `serialize` `add_paths` `remove_path` `clear` `contains` `items` `Hint` `overlay` `zone` `Arm` `MAX_ITEMS` `ZONE_MIN_H` `DONE_MS` |
| `src-tauri/src/shelf/drop.rs` | cíl přetažení: líně vzniklé vlákno `shelf-drop` (OLE STA), neviditelné vrstvené okno (krytí 1/255) s `IDropTarget` (+ `IDropTargetHelper` pro náhled tahu), jen `CF_HDROP`; heuristika „je to OLE tah“ | `set_armed` `ole_drag_likely` `pick_effect` `Target` `run` `OVER` |
| `src-tauri/src/shelf/dragout.rs` | tah ven: na hlavním vlákně `SHCreateDataObject` (absolutní PIDL) + `SHDoDragDrop` s výchozím zdrojem shellu | `start` `drag` |
| `src-tauri/src/shelf/clip.rs` | text do schránky (`CF_UNICODETEXT`, opakované `OpenClipboard`) | `set_text` |
| `src-tauri/src/commands/shelf.rs` | Tauri příkazy; pracují jen s cestami, které v přihrádce jsou (`contains`) | `shelf_list` `shelf_icon` `shelf_copy` `shelf_open` `shelf_reveal` `shelf_remove` `shelf_clear` `shelf_drag_out` |
| `threads/hover.rs` | `shelf::Arm::step` každý tik → `drop::set_armed`; při aktivní zóně neotevírá šuplík; mrtvý Ledge = `set_armed(None)` | |
| `strip.rs` | `StripModel.shelf` (`Hint`), před kreslením `shelf::overlay` na kopii modelu | |
| `src/shelf/ShelfCard.tsx` · `api.ts` · `model.ts` · `shelf.css` | karta (Práce i Osobní, jen neprázdná), most k jádru, čistá logika (`isDrag`, `cardVisible`, `carriesFiles`), styly `shelf-*` | `ShelfCard` `shelfApi` `DRAG_PX` |
| `src/i18n/en-shelf.ts` | angličtina `cards.shelf.*` | |

## Toky

- **Tah dovnitř**: hover (16 ms u hrany) → kurzor VJEL do `shelf::zone` (zóna proužku, výška ≥ 40 log. px)
  s drženým tlačítkem a `ole_drag_likely` (ne `GUI_INMOVESIZE`, kurzor není standardní systémový) →
  `Arm` drží zónu z vjezdu → `set_armed(Some)` → `WM_ARM` → okno nad zónou (`SWP_SHOWWINDOW`,
  `NOACTIVATE`). OLE: `DragEnter` (je `CF_HDROP`?) → `Hint::Over` → proužek „Pustit do přihrádky“
  (tyrkys, nažhavený; i s vypnutým proužkem). `Drop` → `DragQueryFileW` → `add_paths` (uložit + událost
  `shelf`) → `Hint::Done(n)` na `DONE_MS` (`WM_TIMER`, jednorázově). Puštění / odjezd → `set_armed(None)`
  → schovat (běží-li tah nad oknem, až po `Drop`/`DragLeave`, pojistka 800 ms).
- **Karta**: `wire()` jednou — `shelf_list` hned, po události `shelf` a při každé události `view` ≠ hidden
  (`missing` = stat ≤ 50 cest jen na vyžádání). Ikona `shelf_icon` jednou na cestu (`iconimg::shell_png`).
- **Schránka**: `shelf_copy(paths | [] = vše, plain|claude)` → `clip_text` → `clip::set_text`.
  Prostá = řádek na cestu (CRLF); Claude = `@C:\a.txt`, s bílým znakem `@"C:\a b.txt"`, víc na jednom řádku.
- **Tah ven**: `pointerdown` na řádku (ne na tlačítku) + `pointermove` s tlačítkem za `DRAG_PX` →
  `shelf_drag_out` → `dragout::start` (jen je-li tlačítko pořád dole) → `run_on_main_thread` → `SHDoDragDrop`
  s `COPY | LINK`.

## Invarianty a pasti

- **Nikdy `DROPEFFECT_MOVE`** (dovnitř `pick_effect`, ven `COPY | LINK`): zdroj by po „přesunu“ soubor smazal.
- Ukládají se jen cesty; chybějící soubor se označí „chybí“, nikdy se sám neodebere.
- Do Claude session se nic neposílá samo (žádné SendInput / fokus) — jen schránka a tah člověkem.
- **Click-through proužek** drop nedostane → zvláštní okno, viditelné JEN během tahu. Zóna se aktivuje jen
  vjezdem s drženým tlačítkem (klik na taby prohlížeče pod proužkem ne) a jen při OLE kurzoru (posun okna,
  tah tabu Chromu, výběr textu mají standardní kurzor) — heuristika; při chybném vyhodnocení je okno
  neviditelné a zmizí do 16 ms po puštění tlačítka.
- Mrtvý Ledge (`Core::dead`): hover `set_armed(None)`, `overlay(…, dead)` nic nekreslí. Otevřený šuplík
  (`View::Full`) = zóna vypnutá.
- Soubor puštěný do šuplíku (WebView) by se otevřel jako stránka → `wire()` na `dragover`/`drop` se
  `Files` dá `preventDefault` + `dropEffect = none` (přeřazování uvnitř šuplíku `text/plain` nechá).
- Zprávy smyčky `shelf-drop` filtruj na vlastní `hwnd` (OLE má ve vlákně skrytá okna a časovače).
- `#[implement]` potřebuje crate `windows-core` v kořeni (`Cargo.toml`).

## Výkon (rozpočet idle < 0,1 % CPU, < 25 MB private)

- V klidu nic nového: vlákno `shelf-drop` vznikne až při prvním tahu a pak spí v `GetMessageW`; časovače jen
  jednorázově po puštění. Hover nepřidává buzení — kurzor mimo zónu = jen aritmetika; v zóně +1
  `GetAsyncKeyState` na tik (16 ms tiká už dnes); s drženým tlačítkem navíc `GetGUIThreadInfo` + `GetCursorInfo`.
- Úložiště líně (první otevření karty), seznam ≤ 50 položek. Okno + OLE + `IDropTargetHelper` až po prvním tahu.
- Měř `scripts/perf.ps1` před/po (běžící instalovaný Ledge): idle CPU, private, počet vláken (+1 až po tahu).

## Testy

`cargo test shelf` — `normalize`, `add` (dedup bez velikosti písmen, pořadí, strop), `remove`, `clip_text`
(uvozovky, mezery, víc cest), `split`, serializace + tolerance poškozeného souboru, úložiště v tempdiru,
config výchozí zapnuto, `overlay`, `zone`, `Arm` (tabulka vjezd / klik v zóně / puštění / odjezd / zákaz;
tlačítko se mimo zónu nečte), `pick_effect` (nikdy MOVE). `test:ui`: `src/shelf/model.test.mjs`.
Win32 (OLE okno, tah ven, schránka) jen živě.

## Co nečíst

`strip.rs` (jen `run` → `overlay`), `hover.rs` mimo blok přihrádky, `iconimg.rs` (jen `shell_png`).
