# Codemapa: poznámky a připomínky

Formát souboru a pravidla pro agenty: `AGENTS.md` → „Poznámky“. Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/notes/mod.rs` | kořen, čas, obrázky, společné I/O; re-exporty (`notes::…` platí pro vše níž) | `resolve_root` `parse_time` `iso` `end_of_day_ms` `decode_png` `b64_encode` `write_atomic` `path_lock` `DEFAULT_KIND` `GENERAL` |
| `src-tauri/src/notes/frontmatter.rs` | frontmatter (čisté) | `split` `get` `set` `unset` `render` `with_done` `yaml_value` |
| `src-tauri/src/notes/store.rs` | `NoteView`, jména a cesty, vytvoření, sken, `done` | `parse_note` `slug` `project_dir_name` `inside` `check_note_path` `NewNote` `valid_kind` `unique_base` `create` `scan` `set_done` |
| `src-tauri/src/notes/reminders.rs` · `draft.rs` | karta a připomínky, stav peeku · koncept nové, předvyplnění projektu | `due_split` `card` `CARD_ROWS` `reminder_key` `load_peeked` · `Draft*` `draft_*` `preselect` |
| `src-tauri/src/notes/md.rs` | Markdown → bezpečné HTML, úkoly, titulek karty (čisté funkce) | `to_html` `tasks` `toggle_line` `title` `plain` `image_refs` `relative_image` `esc` |
| `src-tauri/src/notes/edit.rs` | úpravy: čtení s obrázky, uložení s otiskem, kopie, úkol, filtr záložek, rozepsaná úprava | `read` `save` `save_copy` `toggle_task` `apply_meta` `body_view` `hash` `query` `open_kind_counts` `EditDraft` `SaveOutcome` |
| `src-tauri/src/notesync/mod.rs` | sync gitem: výsledek a stav pro UI, plánovač, vstup běhu | `Outcome` `Status` `outcome_text` `Wake` `Sched` `Ctx` `Syncer::sync` `IGNORE` |
| `src-tauri/src/notesync/plan.rs` | čisté rozhodování (tabulkové testy) | `plan_init` `classify` `short_err` `sanitize_pc` `ensure_lines` `merge_plan` `Cmp` `Copy` |
| `src-tauri/src/notesync/git.rs` · `init.rs` · `cycle.rs` | git s termínem a bez promptu · první sync a slučování (clone vedle, prohození, záloha `.pred-sync-…`) · commit → fetch → rebase → push | `git` `setup_repo` `commit_all` · `root_state` `fresh` `clone_merge` `merge_into` · `cycle` |
| `src-tauri/src/config.rs` | `notes.kinds` (výchozí sada, validace) | `NotesConfig` `NoteKind` `default_note_kinds` `validate_note_kinds` `NOTE_KIND_ICONS` |
| `src-tauri/src/commands/notes.rs`, `threads/notes_watch.rs`, `snapshot.rs` | tenké příkazy + stav v paměti | `note_create` `note_list` `note_done` `note_open` `note_query` `note_read` `note_save` `note_toggle_task` `note_render` `note_edit_draft_*` `NotesState` `reload_notes` `snapshot` (pole `notes*`, `note_kinds`) |
| `src/notes/store.ts` | sdílený stav: preset, `editPath`, návrat, záložka (localStorage), `setDone`, `openExternal` | |
| `src/notes/draft.ts` | koncept nové + rozepsaná úprava (paměť + disk), vložení obrázků | `openEdit` `loadEditFrom` `edDirty` `restoreDraft` `pasteImages` |
| `src/notes/NoteEditor.tsx` | stránka „note“ (nová i úprava), konflikt, náhled, klávesy Esc / Ctrl+Enter | |
| `src/notes/NotesCard.tsx` | karta se záložkami, `NoteRow`, `Tabs`, `tabsOf`, `ReminderPeek` | |
| `src/notes/NotesPage.tsx` | stránka „notes“ (záložky, projekt, „i hotové“) | |
| `src/notes/ProjectPicker.tsx` · `RemindPicker.tsx` · `kinds.tsx` | výběry (projekt, připomínka, kategorie pilulkami), ikony kategorií | |
| `src/notes/remind.ts` | čisté funkce času (rychlé volby, mřížka měsíce, české popisky) | `quickPicks` `monthGrid` `remindText` `withDay` `withTime` |
| `src/notes/popup.ts` · `Markdown.tsx` · `notes.css` | panely výběrů v toku stránky; náhled (HTML z Rustu); styly | `usePopup` `scrollWithin` `toggleLocal` |
| `src-tauri/src/pins/mod.rs` | lístky na plochu — čisté: stav lístku, soubor `note-pins.json`, label okna, geometrie (DPI, mimo obrazovku) | `Pin` `PinPrefs` `PinPatch` `PinsFile` `load` `save` `label_for` `key` `same_note` `place_new` `restore` `monitor_of` `Mon` `Place` `BAR_H` `TEXT_MIN` |
| `src-tauri/src/pins/win.rs` | okna `note-<klíč>`: vznik bez fokusu, příznaky, geometrie z událostí, zpožděný zápis (`pins-save`), obnova po startu, fullscreen, otisk do okna | `open` `close` `set` `apply` `watch` `toggle_through_all` `sync_dead` `restore_all` `on_notes_changed` `view` `PinsView` `flush` `take_fresh` |
| `src-tauri/src/commands/pins.rs` | příkazy: šuplík (`pin_toggle` `pin_new` `pin_through_all`), okno lístku podle volajícího labelu (`pin_init` `pin_set` `pin_drag` `pin_unpin` `pin_hint_seen`) | `PinInit` `pin_label` |
| `pin.html` · `src/pin/main.tsx` · `PinApp.tsx` · `pin.css` · `icons.tsx` · `en.ts` | okno lístku (vlastní vstup Vite, jen jeho kód a styly): lišta, ⋯ volby, + bod, úprava, schránka, průchozí s nápovědou | |
| `src/pin/model.ts` | čisté: přidání bodu / textu, barvy z průhledností, porovnání cest, připnutí ze snapshotu | `appendLine` `taskLine` `pinColors` `samePath` `isPinned` `anyThrough` |
| `src/notes/pins.ts` | šuplík: připnout / odepnout, nový lístek, průchozí u všech, volby ⋯ karty | `togglePin` `newPin` `throughAll` `throughLabel` `pinMenuItems` |

`App.tsx` jen přepíná stránky (`Page` = `main|settings|note|notes`) a volá `startNote` / `editNote` / `allNotes`.

## Toky
- **Karta**: Rust `reload_notes` → `NotesState.all` (+ `kind_counts`, `rev`) → snapshot: `notes` (strop `CARD_ROWS`),
  `notesKindCounts`, `notesRev`. Záložka ≠ „Vše“ → `note_query` (z paměti) při změně `notesRev`.
- **Úprava**: klik na řádek → `editNote` → `NoteEditor` → `openEdit` → `note_read` (tělo, `hash`, obrázky jako
  data URL) → `note_save {expectedHash}` → `Saved{note}` | `Changed` (banner: načíst znovu / kopie `copy: true`).
- **Úkol v náhledu**: čistá poznámka → `note_toggle_task(path, line, checked, hash)`; s neuloženými změnami jen
  lokálně (`toggleLocal`).
- **Lístek na plochu**: špendlík / paleta → `pin_toggle` / `pin_new` → `pins::win::open` (okno `pin.html`) →
  UI `pin_init` (cesta podle labelu volajícího okna) → `note_read`. Zápis jen `note_save` / `note_toggle_task`
  s otiskem (+ bod, vložený text a obrázek = celé tělo + `images`, při `changed` načíst a jednou znovu).
  Změna zvenku: `reload_notes` → `on_notes_changed` → událost `pin-note` (otisk souboru) → UI čte znovu jen
  při jiném otisku (rozepsaná úprava → banner jako editor). Volby `pin_set` → okno + `pin-prefs`;
  šuplík vidí připnutí v `Snapshot.pins`.

## Invarianty a pasti
- Zpětná kompatibilita (druhé PC se starším Ledge): nové klíče jen `kind`, `updated`; starý `with_done` je
  zachová. „Bez projektu“ ve složce projektu = `project: _obecne` (prázdná hodnota by spadla na složku).
- Uložení nikdy nemění jméno/složku souboru; zápis `write_atomic`, read-modify-write pod `path_lock`.
- Řádky úkolů se počítají v těle **bez** jednoho úvodního `\n` (`body_view`) — stejně v `md::tasks`,
  `to_html` (`data-line`) i `toggleLocal`. Bloky kódu se přeskakují.
- Náhled nesmí pustit HTML: `md::to_html` escapuje vše, odkazy jen http(s) v `data-href` (otevírá `open_url`),
  obrázky jen relativní (`data-src`, data URL doplní UI; CSP `img-src 'self' data:`).
- Šuplík je okno bez fokusu do kliknutí: stránky `note`/`notes` vstupují přes `api.focusDrawer()`, odchod
  `releaseFocus` (efekt v `App`). Výběry nejsou nativní popupy — panel `.pick-pop` je v toku lišty.
- **Rychlý záznam vrátí fokus**: zkratka `hotkeys.note` / klik si zapamatuje okno v popředí
  (`focusreturn::remember`); uložení nové poznámky a Esc/zahození (`api.close()` → `UiCmd::Close`) fokus
  vrátí, jen když je Ledge pořád v popředí a okno žije (`focusreturn::decide`, [core.md](core.md)).
  „Otevřít v editoru“ (`note_open` → `UiCmd::Hide`) fokus nevrací — dostane ho editor.
- Snapshot neroste s počtem poznámek: těla jen přes `note_read`, výběry přes `note_query` se stropem.
- **Lístky**: žádný druhý svět poznámek — stav okna (pozice fyzicky, velikost logicky, volby) je v
  `%LOCALAPPDATA%\Ledge\note-pins.json`, obsah jen v `.md`. Okno se otevírá bez fokusu (`focused(false)` +
  hned viditelné; `show()` po vzniku by aktivovalo), fokus jen „Nový lístek“. Průchozí = `set_ignore_cursor_events`
  + `set_focusable(false)` + bez změny velikosti; ven zkratkou `hotkeys.notePassthrough` (registruje se jen
  s lístkem na ploše, i s vypnutými zkratkami — `config::active_hotkeys_pins`) nebo paletou. Žádný polling:
  geometrie z událostí `Moved`/`Resized`, zápis vlákno `pins-save` (spí na kanálu). Fullscreen / herní režim
  (`sync_dead` z vlákna `hover` a `gamemode::set`) → lístky nejsou navrchu, obnova po startu počká.
  Capability `pins.json` dává oknům `note-*` jen poslech událostí; posun/velikost jdou vlastními příkazy.
  Vlastní událost z vlastního zápisu přijde dřív než odpověď IPC → UI ji během zápisu odloží (`busy`).
- Sync: git nikdy nečeká na prompt (`env_for`, termín na každém volání), konflikt = `rebase --abort` (nic se
  neztratí), lokální stav (`.draft/`, `.state.json`) je v `IGNORE`; do cizího repa se necommituje (`Refuse`).

## Testy
`cargo test notes` — `notes::{frontmatter,store,reminders,draft}::tests` + `notes::tests` (čas, base64, kořen;
pomůcky `notes/testkit.rs`), `notes::md::tests` (HTML tabulka, XSS, úkoly, titulky), `notes::edit::tests`
(round-trip s neznámými klíči, detekce změny, kopie, úkol, obrázky mimo kořen, `query`, rozepsaná úprava);
`config::tests::note_kinds_validation`. Lístky: `cargo test pins` (label, volby a meze, soubor starý / rozbitý /
duplicity, kaskáda nových, obnova na monitoru / mimo obrazovku / DPI), `config::tests::pin_passthrough_hotkey`,
`webpanel::tests::capabilities_never_reach_web_windows` (výjimka `note-*`); UI `src/pin/model.test.mjs`. Sync: `cargo test notesync` — `notesync::plan::tests` (tabulky),
`notesync::tests` (plánovač, stav, integrace nad bare repem v temp). Frontend: `npx tsc --noEmit`.

## Co nečíst
`notesync/` mimo práci na syncu (poznámek se týká jen tím, že soubory přicházejí zvenku), `migrate.rs`,
`edge.rs`, `rawpush.rs`, `strip.rs`, `weather.rs`, `claude/`, `procman.rs`, `update.rs`.
