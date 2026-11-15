# Codemapa: náhled po najetí (`src/hover/*`)

Malé pohledy šuplíku po najetí na proužek: peek / ostrůvek / dok / střední panel (kompaktní náhled zrušen v kole 34 → dok). Kreslí se jen ze
snapshotu (žádný fetch). Rust strana: výběr režimu a okno `hovermode.rs`, vlákno `threads/hover.rs`, `View` +
`ViewMsg` v `drawer.rs` ([core.md](core.md)); volba v Nastavení `AppearanceHover.tsx` ([appearance.md](appearance.md)).
Toky, invarianty a testy: [hover-detail.md](hover-detail.md). Skořápka a morf: [ui.md](ui.md).

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/hover/SmallView.tsx` | rozcestník podle `view`: peek (`cards/Peek.tsx`) / ostrůvek / dok / střední panel ve skořápce | `SmallView` |
| `src/hover/parts.tsx` | společné drobnosti ostrůvku a docku: popisek a čas session, limit 5h „5h 42 %“ | `Limit5h` `sessionLabel` `sessionTime` |
| `src/hover/Island.tsx` | černá kapsle s jedním děním (`islandEvent`) a tlačítky; v dokovém režimu přebírá schválení / otázku | `Island` |
| `src/hover/Dock.tsx` · `DockMusic.tsx` | dok (`drawer.hover=dock`): sloty 📝 / ♪ / ⏱ / čas / 📌; hudba s hlasitostí kolečkem a šipkami | `Dock` `DockMusic` |
| `src/hover/dockmode.ts` | režim docku z `ViewMsg.dock`, stav 📌, událost `dock-note` ze zkratky | `dockMode` `dockPinned` `dockPinDo` `listenDockView` `listenDockNote` |
| `src/hover/Mid.tsx` · `PinButton.tsx` | STŘEDNÍ panel z docku (`View::Mid`): až 3 karty z `drawer.dockCards` + 📌 | `Mid` `PinButton` `togglePin` |
| `src/hover/midCards.ts` | ověření a přepínání karet panelu (zrcadlo Rust `dock_cards`) | `dockCardsOf` `toggleDockCard` `DOCK_CARDS_MAX` `DOCK_CARDS_DEFAULT` |
| `src/hover/priority.ts` | JEDNA tabulka priorit dění + pomocné: schůzka za ≤ 10 min, limit 5h blízko, odhad dosažení | `PRIORITY` `topOf` `rankOf` `meetingNow` `limitNear` `etaAtOf` |
| `src/hover/doneAck.ts` | potvrzení „Hotovo“ (Otevřít / ✕): lokální signál, klíč session + čas dokončení | `doneAcked` `ackDoneSession` |
| `src/hover/expand.ts` | klik mimo tlačítka / setrvání → `api.expandDrawer`; nad `[data-noexpand]` odpočet stojí | `useExpand` |
| `src/hover/model.ts` | čistá logika: šířky, co ukáže ostrůvek, kdy se rozbalit, sloty docku, hlasitost a šipky | `islandEvent` `expandAfter` `smallWidth` `dockSlots` `ackDone` `volumeStep` `arrowPlan` `wheelToStep` `freshWeather` |
| `src/hover/modes.ts` | `HOVER_MODES` pro výběr v Nastavení (id `full|island|dock` = kontrakt s configem; uložené `compact` Rust načte jako `dock`) | `HOVER_MODES` `DEFAULT_HOVER_MODE` |
| `src/hover/hover.css` | styly `hov-*`, `isl-*`, `isl-dock-*` | |

Testy: `model.test.mjs` (`dockSlots`, `ackDone`, šířky, priorita dění), `priority.test.mjs` (zrcadla tabulky priorit),
`midCards.test.mjs`.

## Když přidáváš

- **Nový hover režim / pohled**: Rust `View` (`drawer.rs`) + `hovermode.rs` + `src/hover/modes.ts` (`HOVER_MODES`,
  `HoverModeId`) + větev `SmallView.tsx` + šířka v `model.ts` `smallWidth` + EN popisek; konfigurační enum `drawer.hover`.
- **Nová událost ostrůvku**: řádek v `priority.ts` `PRIORITY` (a zrcadla níže) + větev `islandEvent`.
- **Priorita dění má 3 místa (+ `KIND_PRIO` pro chipy pruhu)**: `priority.ts`, `suggest/model.ts` `pick` a Rust `attention.rs` `Event::rank`
  (hlídá `priority.test.mjs`).
