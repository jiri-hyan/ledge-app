# Codemapa: poznámky a připomínky

Formát souboru a pravidla pro agenty: [agents/notes.md](../agents/notes.md) → „Poznámky“. Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/notes/mod.rs` | kořen, čas, obrázky, společné I/O; re-exporty (`notes::…` platí pro vše níž) | `resolve_root` `parse_time` `iso` `end_of_day_ms` `decode_png` `b64_encode` `write_atomic` `path_lock` `DEFAULT_KIND` `GENERAL` |
| `src-tauri/src/notes/frontmatter.rs` | frontmatter (čisté) | `split` `get` `set` `unset` `render` `with_done` `yaml_value` |
| `src-tauri/src/notes/meta.rs` | Lístky 2.0: barva (`color:` — id z `COLORS` nebo `#rrggbb`) a štítky (`tags:` inline / blokový seznam ∪ `#štítek` v těle) ve frontmatteru; zápis jen inline `tags: [a, b]`, blokový seznam nahradí celý, neznámé klíče a tělo beze změny; zápis s otiskem (`set_meta`) | `COLORS` `MAX_TAGS` `valid_color` `norm_tag` `tag_char` `front_tags` `color_of` `tags_of` `MetaPatch` `apply_patch` `with_meta` `set_meta` |
| `src-tauri/src/notes/links.rs` | vazby `[[název]]` / `[[název\|popisek]]` a `#štítky` (čisté): rozpoznání na pozici pro `md::to_html` (`wiki_at`, `tag_at`), štítky v těle bez kódu (`inline_tags`), cíl vazby (`resolve`: titulek bez diakritiky, pak jméno souboru / slug, nehotová první), našeptávač (`suggest`) | `wiki_at` `tag_at` `inline_tags` `resolve` `suggest` `LinkHit` |
| `src-tauri/src/notes/store.rs` | `NoteView`, jména a cesty, vytvoření, sken, `done` | `parse_note` `slug` `project_dir_name` `inside` `check_note_path` `canonicalize_settled` `NewNote` `valid_kind` `unique_base` `create` `scan` `set_done` |
| `src/types/notes.ts` | typy poznámek pro UI (zrcadlo Rustu): `NoteView`, `NoteDetail`, `SaveNote`/`SaveOutcome`, `NoteQuery`, `QueryResult`, `PinPrefs`/`PinInit`, `NotesSyncView`, `Draft` | `NoteView` `NoteDetail` `SaveNote` `NoteQuery` `PinPrefs` `PinInit` `Draft` |
| `src-tauri/src/notes/reminders.rs` · `draft.rs` | karta a připomínky, stav peeku · koncept nové, předvyplnění projektu | `due_split` `card` `card_where` `CARD_ROWS` `reminder_key` `load_peeked` · `Draft*` `draft_*` `preselect` |
| `src-tauri/src/notes/md.rs` | Markdown → bezpečné HTML, úkoly, titulek karty (čisté funkce); `[[vazba]]` → `<a class="md-wiki" data-wiki>`, `#štítek` → `<span class="md-tag" data-tag>` (escapované, bez href) | `to_html` `tasks` `toggle_line` `title` `plain` `image_refs` `relative_image` `esc` |
| `src-tauri/src/notes/md_list.rs` | zásobník seznamů pro `to_html`: skutečné vnoření `<ul>`/`<ol>`, `start`, pokračovací řádky (čisté) | `Lists` `indent_width` |
| `src-tauri/src/notes/export.rs` | export poznámky: md bez frontmatteru s titulkem, samostatné HTML s obrázky, PDF = totéž HTML k tisku (čisté) | `Format` (`Md` `Html` `Pdf`, `filter`) `export_md` `export_html` `render` `default_name` `with_ext` |
| `src-tauri/src/notes/pdf.rs` | PDF přes WebView2 `ICoreWebView2_7::PrintToPdf` (crate `webview2-com`, žádná nová závislost): jednorázové skryté okno `pdf-export-<n>` (`visible(false)`, `focused(false)`, `skip_taskbar`, bez capability, navigace jen `about:blank` + vlastní `file:`), A4, bez hlavičky/patičky, tisk do `<cíl>.ledge-tmp` → přejmenování, termín `TIMEOUT` 30 s, úklid `Cleanup`, chyby `Fail::message` | `print` `Fail` `label` `temp_html` `partial` `file_url` `nav_allowed` `TIMEOUT` `LABEL_PREFIX` |
| `src-tauri/src/notes/openwith.rs` | „Otevřít v …“: aplikace registrované pro `.md` (`OpenWithProgids`), příkaz z registru → exe + argumenty, jméno do nabídky (čisté) | `MD_PROGIDS_KEY` `expand_env` `command_for` `display_name` |
| `src-tauri/src/notes/when.rs` | čas v českém textu („zítra v 10“, „v pondělí“, „za 2 h“, „25. 10. v 14:00“, „večer“) → rozsah shody v originálu + čas; složený text bez diakritiky (`search::Folded`), vždy budoucnost, datum bez času = 9:00, ochrana proti verzím / %, telefonům / kapitolám; popisek chipu (`label`, cs + en) | `parse_when` `When` `label` `suggest` `Suggest` |
| `src-tauri/src/notes/search.rs` | hledání v těle poznámek pro Ctrl+K: `fold_text` / `Folded` (bez diakritiky, 1 znak = 1 znak, mapa bajtů zpět), `words` `matches` `snippet`, `Index` (tělo bez obrázků, líně přestavěný při změně `rev`; hotové se nehledají) | `Index::{build, search}` `Hit` `fold_text` `Folded` `snippet` |
| `src-tauri/src/notes/edit.rs` | úpravy: čtení s obrázky, uložení s otiskem, kopie, úkol, filtr záložek (i `tag`), štítky s počty, rozepsaná úprava | `read` `save` `save_copy` `toggle_task` `apply_meta` `body_view` `hash` `query` `open_kind_counts` `tag_counts` `EditDraft` `SaveOutcome` |
| `src-tauri/src/notes/lifecycle.rs` · `recycle.rs` | archiv (`_archiv`) a smazání: čisté plány přesunu (`rel_of` `refs_of` `split_images` `plan_move` `archive_dir`), přesun s vrácením (`apply`), `archive` `unarchive` `scan_archive`, odklad `.smazane\<id>\` (`park` `unpark` `commit` `sweep`); Koš Windows `SHFileOperationW` + `FOF_ALLOWUNDO` jen na místních discích (`recycle`, do `lifecycle` jde jako parametr — testy na Koš nesahají) | `ARCHIVE_DIR` `TRASH_DIR` `valid_id` `Plan` `Mv` |
| `src-tauri/src/commands/noteslife.rs` | příkazy `note_archive` `note_unarchive` `note_delete` `note_delete_undo` (+ `keepPin`), čekající smazání `PENDING`, vlákno lhůty `notes-trash` (spí 10 s, pak `commit`), `sweep_trash` při startu z `notes-watch` | `settle` `GRACE` |
| `src-tauri/src/notesync/mod.rs` | sync gitem: výsledek a stav pro UI, plánovač, vstup běhu | `Outcome` `Status` `outcome_text` `Wake` `Sched` `Ctx` `Syncer::sync` `IGNORE` |
| `src-tauri/src/notesync/plan.rs` | čisté rozhodování (tabulkové testy) | `plan_init` `classify` `short_err` `sanitize_pc` `ensure_lines` `merge_plan` `Cmp` `Copy` |
| `src-tauri/src/notesync/git.rs` · `init.rs` · `cycle.rs` | git s termínem a bez promptu · první sync a slučování (clone vedle, prohození, záloha `.pred-sync-…`) · commit → fetch → rebase → push | `git` `setup_repo` `commit_all` · `root_state` `fresh` `clone_merge` `merge_into` · `cycle` |
| `src-tauri/src/config/notes.rs` | `notes.kinds` (výchozí sada, validace) | `NotesConfig` `NoteKind` `default_note_kinds` `validate_note_kinds` `NOTE_KIND_ICONS` |
| `src-tauri/src/commands/notes.rs`, `threads/notes_watch.rs`, `snapshot.rs` | tenké příkazy + stav v paměti | `note_create` `note_list` `note_done` `note_open` `note_export` `note_open_with_apps` `note_open_with` `note_query` `note_search` `note_suggest_remind` `note_read` `note_copy_text` `note_save` `note_toggle_task` `note_render` `note_edit_draft_*` `NotesState` `reload_notes` `snapshot` (pole `notes*`, `note_kinds`) |
| `src-tauri/src/commands/notelinks.rs` | příkazy Lístků 2.0: barva / štítky (`note_set_meta`), štítky s počty (`note_tags`), vazby (`note_link_resolve`, `note_link_suggest`, `note_link_create`) — čtení z paměti `NotesState.all` | `note_set_meta` `note_tags` `note_link_resolve` `note_link_suggest` `note_link_create` `TagCount` |
| `src/notes/store.ts` | sdílený stav: preset, `editPath`, návrat, záložka (localStorage), filtr štítku karty, `setDone`, `openExternal` | `tagFilter` `setTagFilter` |
| `src/notes/api.ts` · `meta.ts` · `wiki.ts` | most k příkazům Lístků 2.0 · čisté zrcadlo Rust `meta` / `links` (paleta, barva podle štítku, `#štítky` v textu, odebratelné = z frontmatteru) · našeptávač `[[` (je kurzor v rozepsané vazbě, vložení titulku); testy `meta.test.mjs` | `notesApi` · `NOTE_COLORS` `colorHex` `noteColor` `normTag` `inlineTags` `frontTags` `withTag` `withoutTag` · `wikiQuery` `applyWiki` |
| `src/notes/WikiLinks.tsx` | vazby v editoru i lístku: klik na `[[…]]` (`useWikiFollow` — existující otevře, jinak lišta „Vytvořit“ `WikiMissing`), našeptávač při psaní `[[` (`useWikiSuggest` + `WikiList`, zeptá se po 150 ms ticha, jen z paměti) | `useWikiFollow` `WikiMissing` `useWikiSuggest` `WikiList` |
| `src/notes/draft.ts` · `editdraft.ts` | koncept nové + rozepsaná úprava (paměť + disk), vložení obrázků · čisté rozhodnutí, co s úpravou z disku (prázdná / zastaralá / jiné poznámky), `trimBody`. `openEdit` čte vždy znovu, řadí otevírání (`openSeq`, `cancelOpen`) a rozepsanou úpravu JINÉ poznámky odloží do `parked` (lišta editoru), nepřepíná tiše | `openEdit` `loadEditFrom` `edDirty` `edStale` `restoreDraft` `pasteImages` `parked` `resumeParked` `dropParked` `syncEdit` `keepMine` `cancelOpen` · `planOther` `planSame` `trimBody` |
| `src/notes/lifecycle.ts` · `lifeactions.ts` · `lifelabels.tsx` · `lifeicons.tsx` | Splnit · Archivovat · Smazat: akce + lišta „Zpět“ (`notice`, `offerUndo`), čisté `lifeActions` / `isArchivedPath` / `shortTitle`, popisky a ikony | `completeNote` `archiveNote` `unarchiveNote` `deleteNote` `reopenNote` `runUndo` `UNDO_MS` |
| `src/notes/RowMenu.tsx` · `UndoBar.tsx` | ⋯ řádku (společné menu `ui/Popover.tsx` `PopMenu`) · lišta Zpět (šuplík ve `App`, `pin` varianta v lístku) | `runAction` |
| `src/notes/NoteEditor.tsx` | stránka „note“ (nová i úprava): název poznámky v hlavičce, konflikt (Načíst · Ponechat moje · kopie), lišta odložené úpravy, živá aktualizace při změně z lístku (`notesRev` → `runSync`), „Uložit a připnout“, náhled, klávesy Esc / Ctrl+Enter, chytré seznamy | |
| `src/notes/lists.ts` · `listkeys.ts` | chytré seznamy v textarea (editor i lístek): čisté `onEnter` (pokračuje `1.` / `-` / `- [ ]`, prázdná položka seznam ukončí) a `onTab` (vnoření, číslování od `1.`) · napojení na klávesy | `onEnter` `onTab` `parseItem` · `listKey` |
| `src/notes/NoteMenu.tsx` · `exportActions.ts` | ⋯ editoru (Export) · Markdown / HTML / PDF do souboru (`note_export`), kopie do schránky, Otevřít v … (`note_open_with_apps`, `note_open_with`) — sdílí lístek | `exportItems` `exportNote` `copyMarkdown` `openWith` |
| `src/notes/NotesCard.tsx` | karta se záložkami, filtr podle štítku (`note_tags` při změně `notesRev`, `note_query {tag}`), `NoteRow` (barva = proužek vlevo, štítky v podtitulku; Kopírovat text → `note_copy_text`), `Tabs`, `tabsOf`, `ReminderPeek`; varianta Bento (velká připnutá / nejnovější poznámka + dlaždice Nová .md, šablony, Uložit HTML, Uložit PDF, Otevřít) | `copyNote` |
| `src/notes/bento.ts` | čistá logika Bento: která poznámka je velká, náhled těla, šablony (test `bento.test.mjs`) | `heroOf` `bentoLines` `TEMPLATES` |
| `src/notes/NotesPage.tsx` | stránka „notes“ (záložky, projekt, „i hotové“, přepínač „Archiv“ → `note_query {archived}`) | `pageArchive` |
| `src/notes/ProjectPicker.tsx` · `RemindPicker.tsx` · `kinds.tsx` | výběry (projekt, připomínka, kategorie pilulkami), ikony kategorií | |
| `src/notes/RemindSuggest.tsx` · `suggest.ts` | chip „⏰ Připomenout zítra 10:00?“ pod textem editoru: po 450 ms ticha, jen bez připomínky a když text vypadá časově (`looksTimed`), jinak jádro (`note_suggest_remind`) nevolá; klik nastaví pole připomínky, × zavře | `RemindSuggest` · `looksTimed` `wantsAsk` `toLocalValue` `visibleSuggest` `SUGGEST_DELAY` |
| `src/notes/remind.ts` | čisté funkce času (rychlé volby, mřížka měsíce, české popisky) | `quickPicks` `monthGrid` `remindText` `withDay` `withTime` |
| `src/notes/popup.ts` · `Markdown.tsx` · `notes.css` · `mdtask.css` (úkoly `.md`, sdílí šuplík i lístek) | panely výběrů v toku stránky; náhled (HTML z Rustu); styly | `usePopup` `scrollWithin` `toggleLocal` |
| `src-tauri/src/pins/mod.rs` | lístky na plochu — čisté: stav lístku (i odepnutého: `open: false` + poloha, velikost, vzhled; nejvýš `MAX_CLOSED`), styl, vzhled nového lístku (naposledy použitý / výchozí), „Schovat do…“ (`hidden_until`, viditelné, termín), soubor `note-pins.json`, label okna, geometrie (DPI, mimo obrazovku) | `Pin` `PinPrefs` `PinPatch` `PinStyle` `PinLook` `new_look` `PinsFile` `ensure` `reopen` `close` `hide` `visible` `due_hidden` `next_unhide` `MAX_CLOSED` `load` `save` `label_for` `key` `same_note` `dead_plan` `place_new` `restore` `monitor_of` `Mon` `Place` `BAR_H` `TEXT_MIN` |
| `src-tauri/src/pins/win.rs` | okna `note-<klíč>`: vznik bez fokusu, příznaky, geometrie z událostí, zpožděný zápis (`pins-save`), obnova po startu, fullscreen, otisk do okna; vzhled pro všechny (`look_all`), schovat (`hide`) + jeden termín pro vlákno `emit` (`next_deadline` / `tick`) | `open` `close` `set` `apply` `watch` `toggle_through_all` `sync_dead` `restore_all` `on_notes_changed` `view` `PinsView` `flush` `take_fresh` `look_all` `hide` `next_deadline` `tick` `yielding_now` `new_pin_look` |
| `src-tauri/src/commands/pins.rs` | příkazy: šuplík (`pin_toggle` `pin_new` `pin_through_all` `pin_open`), zkratka `hotkeys.notePin` (`quick_new`, ve hře nic), okno lístku podle volajícího labelu (`pin_init` `pin_set` `pin_drag` `pin_unpin` `pin_hint_seen` `pin_look_all` `pin_hide`, `pin_edit_in_ledge` → šuplík `edit-note`) | `PinInit` `pin_label` `quick_new` |
| `pin.html` · `src/pin/main.tsx` · `PinApp.tsx` · `pin.css` · `src/pin/icons.tsx` · `en.ts` | okno lístku (vlastní vstup Vite, jen jeho kód a styly): lišta, + bod, úprava, schránka, průchozí s nápovědou, štítky pod textem, vazby `[[…]]` (klik = další lístek `pin_open`) | |
| `src/pin/PinMenu.tsx` · `pinmeta.ts` | ⋯ lístku: hlavička s křížkem (zavře / odepne celý lístek), Barva, Vzhled a průhlednost (galerie, posuvníky, Použít na všechny), Štítky, Okno, Schovat do…, Poznámka, Kopírovat jako Markdown, Exportovat…, Splnit / Archivovat / Smazat — rozbalovací části, žádné podstránky · zápis barvy / štítků s otiskem a jedním opakováním | `PinMenu` · `pinMeta` |
| `src/pin/styles.ts` · `StyleGallery.tsx` · `gallery.css` · `hide.ts` | styly lístku (`classic` = původní vzhled, `paper` `glass` `stripe` `minimal` `dark`) → CSS proměnné; galerie dlaždic s náhledem (lístek i Nastavení) · rychlé volby „Schovat do…“; testy `styles.test.mjs` | `PIN_STYLES` `pinLook` `isPinStyle` · `StyleGallery` `styleName` `colorName` · `hidePicks` `validHide` |
| `src/pin/model.ts` | čisté: přidání bodu / textu (číslovaný seznam pokračuje číslem), kurzor z dvojkliku, barvy z průhledností, porovnání cest, připnutí ze snapshotu | `remindAskText` / `pinSuggestVisible` (po „+ bod“ s časem v textu chip „⏰ Připomenout zítra 10:00?“ přes `note_suggest_remind`, jedno zeptání po `SUGGEST_DELAY`, klik = `note_save` s `remind`; jen poznámka bez připomínky) · `appendLine` `taskLine` `pointLine` `caretFromText` `pinColors` `samePath` `isPinned` `anyThrough` |
| `src/notes/pins.ts` | šuplík: připnout / odepnout, nový lístek, průchozí u všech, volby ⋯ karty | `togglePin` `newPin` `throughAll` `throughLabel` `pinMenuItems` |

`App.tsx` jen přepíná stránky (`Page` = `main|settings|note|notes`) a volá `startNote` / `editNote` / `allNotes`; událost `edit-note` (cesta) otevře editor poznámky — vysílač (Rust, „Upravit v Ledge“ z lístku) zatím chybí.

## Toky
- **Karta**: Rust `reload_notes` → `NotesState.all` (+ `kind_counts`, `rev`) → snapshot: `notes` (strop `CARD_ROWS`),
  `notesKindCounts`, `notesRev`. Záložka ≠ „Vše“ → `note_query` (z paměti) při změně `notesRev`.
- **Úprava**: klik na řádek → `editNote` → `NoteEditor` → `openEdit` → `note_read` (tělo, `hash`, obrázky jako
  data URL) → `note_save {expectedHash}` → `Saved{note}` | `Changed` (banner: načíst znovu / kopie `copy: true`).
- **Rozepsaná úprava vs. lístek** (příčina chyby „editor prázdný, lístek ukazuje text“): `.draft\edit.json` se
  při otevření vrací jen když dává smysl (`planSame`): prázdná nebo shodná s diskem → zahodit, jiný otisk než
  na disku (lístek / sync poznámku mezitím změnil) → vrátit s bannerem „změnila se jinde“ (`edStale`); úprava
  jiné poznámky přesměruje jen s nějakým textem. `persistEdit` prázdný text nezapisuje (nejde ani uložit).
- **Splnit · Archivovat · Smazat**: UI `lifecycle.ts` → `note_done` | `note_archive` | `note_delete` → lišta
  `UndoBar` 8 s (Zpět = `note_done false` | `note_unarchive` | `note_delete_undo`). Archiv: `.md` + vlastní
  obrázky do `<složka>\_archiv\` (sdílený obrázek se kopíruje; stejný už v cíli se při návratu jen dedupuje),
  `NotesState.archived` (`scan_archive`, čte jen prvních 8 kB souboru) → `Snapshot.notesArchived` a
  `note_query {archived}`; aktivní `scan` `_archiv` nevidí (jedna úroveň složek) — stejně starší Ledge.
  Smazání: `park` do `.smazane\<id>\<cesta od kořene>` → vlákno `notes-trash` po 10 s `commit` (soubory zpět
  na původní místo + `recycle` do Koše, aby měly v Koši původní umístění; selže Koš = poznámka zpět) · Zpět
  `unpark` · start `sweep`. `.smazane\` je v `IGNORE`, `local_only` a filtrech watcheru. Archivovaná /
  smazaná poznámka zavře lístek (`settle`; z lístku `keepPin` — okno se zavře samo po liště Zpět) a zahodí
  svou rozepsanou úpravu (`edit_draft_forget`).
- **Úkol v náhledu**: čistá poznámka → `note_toggle_task(path, line, checked, hash)`; s neuloženými změnami jen
  lokálně (`toggleLocal`).
- **Lístek na plochu**: špendlík / paleta → `pin_toggle` / `pin_new` → `pins::win::open` (okno `pin.html`) →
  UI `pin_init` (cesta podle labelu volajícího okna) → `note_read`. Zápis jen `note_save` / `note_toggle_task`
  s otiskem (+ bod, vložený text a obrázek = celé tělo + `images`, při `changed` načíst a jednou znovu).
  Změna zvenku: `reload_notes` → `on_notes_changed` → událost `pin-note` (otisk souboru) → UI čte znovu jen
  při jiném otisku (rozepsaná úprava → banner jako editor). Volby `pin_set` → okno + `pin-prefs`;
  šuplík vidí připnutí v `Snapshot.pins`.

## Invarianty a pasti
- **PDF**: tiskne se jen v příkazu `note_export` (async, mimo hlavní vlákno; `with_webview` běží na hlavním) —
  šuplík se nikdy nenaviguje pryč. Okno tisku se neukáže ani nevezme fokus a po tisku / chybě / termínu zmizí
  spolu s dočasným HTML v `%TEMP%`. Rozbitý tisk nepřepíše starý soubor (`partial`). Starý WebView2 bez
  `ICoreWebView2_7` = věta „aktualizuj WebView2 Runtime, nebo ulož HTML“. Tisk se v testech nespouští (GUI).
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
- **Lístky 2.0**: poloha, monitor, velikost, styl, průhlednost a „schovaný do“ jsou **jen tohoto PC**
  (`note-pins.json`, nikdy frontmatter / sync); barva a štítky jsou vlastnost poznámky (`color:`, `tags:` ve
  frontmatteru — putují s ní). Odepnutí řádek nemaže (`open: false`), znovupřipnutí ho otevře na stejném místě
  (chybí-li monitor → `place_new`). Schovaný lístek: jediný termín `pins::win::next_deadline` v poli termínů
  vlákna `emit` (žádný vlastní časovač); ve hře počká na `restore_all`. Zkratka `hotkeys.notePin` výchozí prázdná.
  Vazby a štítky v náhledu jsou jen `data-*` atributy z `md::to_html` (vše escapované, žádné href).
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
`cargo test notes` — `notes::export::tests` (formáty vč. `pdf`, jména, přípony, HTML vstup PDF), `notes::pdf::tests`
(cesty, `file:` URL, zamčené okno tisku, věty chyb; samotný tisk jen naživo: editor poznámky → ⋯ → „Uložit jako
PDF…“ → dialog → hláška „Uloženo do …“, PDF A4 s obrázky a zaškrtnutými úkoly, žádné okno ani ztráta fokusu),
`notes::{frontmatter,store,reminders,draft}::tests` + `notes::tests` (čas, base64, kořen;
pomůcky `notes/testkit.rs`), `notes::md::tests` (HTML tabulka, XSS, úkoly, titulky), `notes::edit::tests`
(round-trip s neznámými klíči, detekce změny, kopie, úkol, obrázky mimo kořen, `query`, rozepsaná úprava);
`config::tests::note_kinds_validation`; `notes::when::tests` (tabulka 60+ případů vč. negativních, rozsah v bajtech originálu, popisky cs / en), `notes::search::tests` (fold, mapa bajtů, úryvky, index nad dočasným kořenem); UI `src/notes/suggest.test.mjs`. Lístky: `cargo test pins` (label, volby a meze, soubor starý / rozbitý /
duplicity, kaskáda nových, obnova na monitoru / mimo obrazovku / DPI, paměť po odepnutí, schovat do data, strop odepnutých, styl, vzhled nového), `config::tests::pin_passthrough_hotkey`, `config::tests::pin_new_hotkey_and_pin_defaults`, `notes::meta::tests` (frontmatter barvy / štítků, neznámé klíče), `notes::links::tests` (`[[…]]`, `#štítek`, cíl vazby, našeptávač), `notes::md::tests::xss_is_escaped` (vazby a štítky),
`webpanel::tests::capabilities_never_reach_web_windows` (výjimka `note-*`); UI `src/pin/model.test.mjs`, `src/pin/styles.test.mjs` (styly, Schovat do…), `src/notes/meta.test.mjs` (barva, štítky, `[[`), `src/notes/lists.test.mjs`. Životní cyklus: `cargo test notes::lifecycle` (odkazy, vlastní × sdílené obrázky, archiv a návrat, kolize, odklad / Zpět / Koš s falešným Koše, sweep, cesty mimo kořen), `notes::recycle` (jen místní disky), UI `src/notes/{editdraft,lifeactions}.test.mjs`; cesta lístku → editor: `notes::edit::tests::sticky_text_and_image_reach_the_note_the_editor_reads`. Sync: `cargo test notesync` — `notesync::plan::tests` (tabulky),
`notesync::tests` (plánovač, stav, integrace nad bare repem v temp). Frontend: `npx tsc --noEmit`.

## Co nečíst
`notesync/` mimo práci na syncu (poznámek se týká jen tím, že soubory přicházejí zvenku), `migrate.rs`,
`edge.rs`, `rawpush.rs`, `strip.rs`, `weather.rs`, `claude/`, `procman.rs`, `update.rs`.
