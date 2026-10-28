# Codemapa: Ledge Studio — nové Nastavení (kolo 24)

Nastavení je samostatné okno `studio` (`settings.html`), vzniká líně až na výslovný klik a zavření ho zničí.
Když okno nejde vytvořit, šuplík vykreslí TÝŽ komponent jako stránku `settings` (kompaktní rozvržení).
Platí hned: každá změna se po 400 ms ověří (`check_config`) a zapíše (`save_config`); textová pole až při
Enter / opuštění; časová osa relace s Vrátit. 8 částí v pořadí anatomie Ledge, domov „Tvůj Ledge“, průvodce,
scéna vpravo. Panely uvnitř částí = dnešní `src/settings/panels/*` → [settings.md](settings.md).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/studio.rs` | okno `studio`: líné vytvoření / zaostření + skok (`studio-goto`), geometrie (1360 × 880, 92 % plochy, min 980 × 640, pamatuje `studio-window.json` mimo config), okenní tlačítka a `wizard`/`edit` do šuplíku (`studio-ask`), přeposílání `state`/`appearance`/`config-changed` jen otevřenému oknu | `open` `open_or` `place` `clean_target` `studio_open` `studio_win` `forward` `window` |
| `src-tauri/capabilities/studio.json` | ACL okna: jen příkazy, které Nastavení volá (tabulka `STUDIO_WINDOW` v `ipcacl.rs`), bez fokusu / velikosti šuplíku | |
| `src/studio/parts.ts` | 8 částí a jejich otázky (blok = panel / krok Vzhledu, `primary` pro průvodce, `visible(cfg)`), volba / cesta → blok → část, skoky odjinud, počty „upraveno“, postup průvodce — test `parts.test.mjs` (guard: každá volba právě v jedné části, bez žargonu v hlavní vrstvě) | `PARTS` `blockOf` `partOfBlock` `resolveStudioTarget` `modifiedByPart` `guideSteps` `parseGuide` `withDone` |
| `src/studio/history.ts` | Platí hned jako čistý model: `diffPaths`, časová osa (slučování `MERGE_MS`, ruční návrat, Vrátit jednu / poslední), popisky „Volba: A → B“, `canSave`, `isTextEntry` — test `history.test.mjs` | `record` `undoEntry` `undoLast` `entryText` `canSave` `SAVE_DELAY_MS` |
| `src/studio/anatomy.ts` | domov: kresba z `layout.pages` (karta → část), zóny = 8 stanic, „Na co nezapomenout“ jen ze skutečného stavu — test `anatomy.test.mjs` | `homeModel` `cardPart` `reminders` `ZONES` |
| `src/studio/tips.ts` | domov: „Dobré vědět" (4 tipy, které existují) a „Co je nového" jen po přechodu z nižší verze (`newsState`, zavřít jde; poslední viděná verze v localStorage) — test `tips.test.mjs` | `newsState` `TIPS` `NEWS` `cmpVersion` |
| `src/studio/named.ts` | lidské názvy hodnot v časové ose (styl, barvy, rozložení, písmo, materiál, šířka, vzhled proužku, najetí) — test `named.test.mjs` | `namedValue` |
| `src/studio/autosave.ts` | Solid lepidlo autosave: debounce → ověřit → zapsat, psaní do textových polí zápis odloží, konflikt / sloučení změny zvenku, Vrátit (Ctrl+Z, položka, vše) | `useAutosave` `undo` `undoOne` `undoAll` `adoptExternal` `resolveConflict` `saveState` `history` |
| `src/studio/Studio.tsx` | skořápka: titulek (drobečky, hledání `smartSearch`, stav zápisu, Změny, Průvodce, okno), páteř, domov / část, scéna, průvodce, skoky (hash, `studio-goto`, `settingsTarget`), chyba ověření rozbalí Doladit | `Studio` `go` `reveal` `pick` |
| `src/studio/PartView.tsx` | obrazovka části: hlavička + otázky (blok uvnitř), v průvodci jen hlavní otázky a bez Doladit | `PartView` |
| `src/studio/Home.tsx` | domov „Tvůj Ledge“: klikací kresba se stanicemi, Na co nezapomenout, Rychle (ovladače), všechny části | `Home` |
| `src/studio/Scene.tsx` | scéna části (Proužek, Otevírání, Vzhled šuplíku, Obsah, Claude) z existujících náhledů; `peek*` jen tady | `Scene` `SCENE_PARTS` `DrawerMini` |
| `src/studio/StudioWindow.tsx` · `main.tsx` · `settings.html` | vstup okna: snapshot (`get_state` + `state`), Toast; líný slovník; demo jen mimo Tauri | `StudioWindow` |
| `src/studio/demo.ts` · `demo-config.json` | ukázkový hostitel JEN pro `settings.html?demo=1` mimo Tauri (snímky), config = `Config::default()` | `installDemo` |
| `src/studio/icons.tsx` · `studio.css` · `en.ts` | ikony částí a titulku, vzhled skořápky (container šířka → `.narrow`), angličtina | `PartIcon` |

## Vstupy (všechny → `palette/goto.ts` `openSettings` / `gotoSettingsPanel` → `studio_open`)

⚙ v hlavičce (`App.tsx`), paleta (`palNav.onSettings`), karty (`onSettings` + `gotoSettingsPanel`), tray / widget na ploše /
`ctl` (`studio::open_or`, záloha = šuplík se stránkou `settings`). V samotném okně jen `settingsTarget`.

## Invarianty

- V klidu nežije nic: okno vzniká na klik, zavření = `destroy`; žádný časovač kromě jednorázového autosave po změně.
- Šuplíkové příkazy (`focus_drawer`, `release_focus`, `drawer_size*`) jsou ve Studiu no-op (`api.ts` `inStudio`).
- Průvodce prvním spuštěním a „Upravit přímo v šuplíku“ ze Studia: `studio_win("wizard"|"edit")` → šuplík (`studio-ask`).
