# Codemapa: náhled po najetí — detail (`src/hover/*`)

Jádro: [hover.md](hover.md). Sem patří plné popisy souborů, toky a invarianty (přesunuto z `ui-detail.md`).

## Popis souborů (plný)

náhled po najetí (`drawer.hover`, Rust `View::Island` / `View::Mid`, [core.md](core.md)): `SmallView.tsx` (peek / ostrůvek / dok ve skořápce),
  `parts.tsx` (popisek a čas session, `Limit5h`; dřívější kompaktní náhled byl v kole 34 zrušen — `drawer.hover: "compact"` se načte jako `dock`, jeho obsah dnes nese střední panel s kartou Claude),
  `Island.tsx` (černá kapsle s jedním děním a tlačítky),
  `hover/Dock.tsx` + `hover/DockMusic.tsx` (`drawer.hover=dock`, také `view: "island"`: sloty 📝 poznámka přes `notes/QuickNote` / ♪ hudba s hlasitostí kolečkem, ztlumením a šipkami / ⏱ / čas / 📌 `api.drawerPin`; schválení a otázka přebírá `Island`, hotovo je slot ✓ jméno · Otevřít · ✕ (L146); `hover/dockmode.ts` = režim z `ViewMsg.dock` (L130, jádro ho posílá vždy) + stav 📌 `dockPinned` (L137, drží se mimo `DockRow`, ať přežije přepnutí na `Island` při události; ruší ho každé `view`) + událost `dock-note` ze zkratky),
  `priority.ts` (JEDNA tabulka priorit dění `PRIORITY` + `topOf`, `meetingNow` (schůzka za ≤ 10 min / probíhá), `limitNear` (5h ≥ 90 % nebo odhad do 30 min), `etaAtOf`; `islandEvent` ji používá, pruh návrhů `suggest/model.ts` `pick` a Rust `attention.rs` `Event::rank` jsou zrcadla hlídaná `priority.test.mjs` — **Priority: 3 místa, při změně pořadí uprav tabulku i oba**; Island v klidu ukáže 1 chip z `suggest/chips.ts` `useChips`),
  `doneAck.ts` (potvrzení „Hotovo“: signál `doneAcked`, klíč session+čas dokončení,
  `model.ts` `ackDone` zahazuje klíče starší než `DONE_FRESH_MS`; Otevřít / ✕ v ostrůvku i docku),
  `hover/Mid.tsx` (STŘEDNÍ panel z docku, L147, Rust `View::Mid`: „▾ celý šuplík“ + 📌 `hover/PinButton.tsx` + až 3 karty z `drawer.dockCards` kreslené `MainPage` s `mid` = tytéž komponenty, bez ⋯ a úprav; šířka ~1/3 monitoru z jádra, bez samo-rozbalení; Poznámka / Nastavení z karty panel rozbalí do celého šuplíku — `midNav` v `App.tsx`) + `hover/midCards.ts` (ověření a přepínání karet, zrcadlo Rust `dock_cards`, test `midCards.test.mjs`),
  `expand.ts` (klik mimo tlačítka / setrvání → `api.expandDrawer`, jádro rozhodne dok → Mid, jinak Full; `toFull` pro schválení v docku; nad `[data-noexpand]` sloty a při 📌 odpočet stojí), čistá logika `model.ts` (+ `model.test.mjs`: `dockSlots` `ackDone` `wheelToStep` `volumeStep` `arrowPlan`, priorita dění, co compact ukáže, kdy se rozbalit, šířky),
  `modes.ts` = `HOVER_MODES` pro výběr v Nastavení (id `full|compact|island|dock` = kontrakt), styly `hover.css` (`hov-*`, `isl-*`, `isl-dock-*`)

## Tok

- **Náhled po najetí** (`view` = `compact` / `island`, `hover/*`): kreslí se jen ze snapshotu (žádný fetch; média /
  počasí / limity jen když už jsou), stejný morf jako peek (z proužku, do celého šuplíku z tvaru náhledu).
  Celý šuplík: klik na prázdné místo nebo „▾ celý šuplík“ hned, setrvání po `ViewMsg.expandMs`
  (`drawer.hoverExpandMs`) — odpočet stojí nad tlačítky, mimo okno a s čekajícím schválením (`expandAfter`).
  `api.expandDrawer()` fokus nebere; odjetí myši zavře náhled jako šuplík (Rust `threads/hover.rs`).
