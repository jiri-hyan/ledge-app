# Codemapa: poznámky a připomínky

Formát souboru a pravidla pro agenty: [agents/notes.md](../agents/notes.md) → „Poznámky“. Cituj symbolem, ne číslem řádku.

Detail (toky, invarianty, testy, co nečíst, historie): [notes-detail.md](notes-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/notes/mod.rs` | kořen, čas, obrázky, společné I/O … | `resolve_root` |
| `src-tauri/src/notes/frontmatter.rs` | frontmatter (čisté) | `split` |
| `src-tauri/src/notes/meta.rs` | Lístky 2.0: barva (`color:` … | `COLORS` |
| `src-tauri/src/notes/links.rs` | vazby `[[název]]` / … | `wiki_at` |
| `src-tauri/src/notes/store.rs` | `NoteView`, jména a cesty, vytvoření … | `parse_note` |
| `src/types/notes.ts` | typy poznámek pro UI (zrcadlo Rustu) … | `NoteView` |
| `src-tauri/src/notes/reminders.rs` · `draft.rs` | karta a připomínky, stav peeku · … | `due_split` |
| `src-tauri/src/notes/md.rs` | Markdown → bezpečné HTML, úkoly … | `to_html` |
| `src-tauri/src/notes/md_list.rs` | zásobník seznamů pro `to_html` … | `Lists` |
| `src-tauri/src/notes/export.rs` | export poznámky: md bez frontmatteru s … | `Format` |
| `src-tauri/src/notes/pdf.rs` | PDF přes WebView2 … | `print` |
| `src-tauri/src/notes/openwith.rs` | „Otevřít v …“: aplikace registrované … | `MD_PROGIDS_KEY` |
| `src-tauri/src/notes/when.rs` | čas v českém textu („zítra v 10“, „v … | `parse_when` |
| `src-tauri/src/notes/search.rs` | hledání v těle poznámek pro Ctrl+K … | `Index::{build, search}` |
| `src-tauri/src/notes/edit.rs` | úpravy: čtení s obrázky, uložení s … | `read` |
| `src-tauri/src/notes/lifecycle.rs` · `recycle.rs` | archiv (`_archiv`) a smazání … | `ARCHIVE_DIR` |
| `src-tauri/src/commands/noteslife.rs` | příkazy `note_archive` `note_unarchive` … | `settle` |
| `src-tauri/src/notesync/mod.rs` | sync gitem: výsledek a stav pro UI … | `Outcome` |
| `src-tauri/src/notesync/plan.rs` | čisté rozhodování (tabulkové testy) | `plan_init` |
| `src-tauri/src/notesync/git.rs` · `init.rs` · `cycle.rs` | git s termínem a bez promptu · první … | `git` |
| `src-tauri/src/notes/folder_sync.rs` | Propojit se složkou … | `LinkRun` |
| `src-tauri/src/notes/folder_sync_plan.rs` | čisté rozhodování propojení (tabulkové … | `plan` |
| `src-tauri/src/notes/folder_sync_scan.rs` | sken a bezpečné cesty (I/O) … | `scan` |
| `src-tauri/src/notes/folder_sync_apply.rs` | provedení kroků: atomický zápis pod … | `apply` |
| `src-tauri/src/notes/folder_sync_tests.rs` | testy propojení: tabulky plánu, jména … |  |
| `src-tauri/src/threads/notes_folder.rs` · `commands/notefolder.rs` | vlákno `notes-folder` (spí na kanálu … | `spawn_notes_folder` |
| `src/notes/FolderLinkSection.tsx` · `folderlink.ts` · `en-folder.ts` | sekce „Propojit se složkou“ v Nastavení … | `linkCfg` |
| `src-tauri/src/config/notes.rs` | `notes.kinds` (výchozí sada, validace) | `NotesConfig` |
| `src-tauri/src/commands/notes.rs`, `threads/notes_watch.rs`, `snapshot.rs` | tenké příkazy + stav v paměti | `note_create` |
| `src-tauri/src/commands/notelinks.rs` | příkazy Lístků 2.0: barva / štítky … | `note_set_meta` |
| `src/notes/store.ts` | sdílený stav: preset, `editPath` … | `tagFilter` |
| `src/notes/api.ts` · `meta.ts` · `wiki.ts` | most k příkazům Lístků 2.0 · čisté … | `notesApi` |
| `src/notes/WikiLinks.tsx` | vazby v editoru i lístku … | `useWikiFollow` |
| `src/notes/draft.ts` · `editdraft.ts` | koncept nové + rozepsaná úprava (paměť … | `openEdit` |
| `src/notes/lifecycle.ts` · `lifeactions.ts` · `lifelabels.tsx` · `lifeicons.tsx` | Splnit · Archivovat · Smazat … | `completeNote` |
| `src/notes/RowMenu.tsx` · `UndoBar.tsx` | ⋯ řádku (společné menu `ui/Popover.tsx` … | `runAction` |
| `src/notes/NoteEditor.tsx` | stránka „note“ (nová i úprava) … |  |
| `src/notes/lists.ts` · `listkeys.ts` | chytré seznamy v textarea (editor i … | `onEnter` |
| `src/notes/NoteMenu.tsx` · `exportActions.ts` | ⋯ editoru (Export) · Markdown / HTML / … | `exportItems` |
| `src/notes/NotesCard.tsx` | karta se záložkami, filtr podle štítku … | `copyNote` |
| `src/notes/bento.ts` | čistá logika Bento: která poznámka je … | `heroOf` |
| `src/notes/headline.ts` | hlavička poznámky složená sama „Projekt · Název · 26. 10.“ … | `headline` |
| `src/notes/group.ts` | seskupení seznamu karty (nic / projekt / kategorie / datum / připnuté) … | `groupNotes` |
| `src/notes/noteDesigns.ts` | 6 vzhledů karty Poznámky pro registr vzhledů … | `NOTES_DESIGNS` |
| `src/notes/ColorDots.tsx` · `colorset.ts` | tečky 8 barev v ⋯ řádku a editoru · zápis `color:` … | `setNoteColor` |
| `src/notes/NotesPage.tsx` | stránka „notes“ (záložky, projekt, „i … | `pageArchive` |
| `src/notes/ProjectPicker.tsx` · `RemindPicker.tsx` · `kinds.tsx` | výběry (projekt, připomínka, kategorie … |  |
| `src/notes/RemindSuggest.tsx` · `suggest.ts` | chip „⏰ Připomenout zítra 10:00?“ pod … | `RemindSuggest` |
| `src/notes/remind.ts` | čisté funkce času (rychlé volby, mřížka … | `quickPicks` |
| `src/notes/popup.ts` · `Markdown.tsx` · `notes.css` · `mdtask.css` (úkoly `.md`, sdílí šuplík i lístek) | panely výběrů v toku stránky … | `usePopup` |
| `src-tauri/src/pins/mod.rs` | lístky na plochu — čisté … | `Pin` |
| `src-tauri/src/pins/win.rs` | okna `note-<klíč>`: vznik bez fokusu … | `open` |
| `src-tauri/src/commands/pins.rs` | příkazy: šuplík (`pin_toggle` `pin_new` … | `PinInit` |
| `pin.html` · `src/pin/main.tsx` · `PinApp.tsx` · `pin.css` · `src/pin/icons.tsx` · `en.ts` | okno lístku (vlastní vstup Vite, jen … |  |
| `src/pin/PinMenu.tsx` · `pinmeta.ts` | ⋯ lístku: hlavička s křížkem (zavře / … | `PinMenu` |
| `src/pin/styles.ts` · `StyleGallery.tsx` · `gallery.css` · `hide.ts` | styly lístku (`classic` = původní … | `PIN_STYLES` |
| `src/pin/model.ts` | čisté: přidání bodu / textu (číslovaný … | `remindAskText` |
| `src/notes/pins.ts` | šuplík: připnout / odepnout, nový … | `togglePin` |

Další soubory (podrobnosti v detailu):
`src-tauri/src/commands/{notefolder.rs,notelinks.rs,notes.rs,pins.rs}` · `src-tauri/src/config/notes.rs` · `src-tauri/src/notes/{edit.rs,folder_sync.rs,folder_sync_apply.rs,folder_sync_plan.rs,folder_sync_scan.rs,lifecycle.rs,recycle.rs}` · `src-tauri/src/notesync/{cycle.rs,git.rs,init.rs,plan.rs}` · `src-tauri/src/threads/notes_folder.rs`
`src/notes/{FolderLinkSection.tsx,NoteEditor.tsx,NoteMenu.tsx,NotesCard.tsx,NotesPage.tsx,ProjectPicker.tsx,RemindPicker.tsx,RemindSuggest.tsx,RowMenu.tsx,UndoBar.tsx,WikiLinks.tsx,api.ts,bento.ts,draft.ts,editdraft.ts,en-folder.ts,exportActions.ts,folderlink.ts,kinds.tsx,lifeactions.ts,lifecycle.ts,lifeicons.tsx,lifelabels.tsx,listkeys.ts,lists.ts,meta.ts,pins.ts,popup.ts,remind.ts,store.ts,suggest.ts,wiki.ts}`
`src/pin/{PinApp.tsx,PinMenu.tsx,StyleGallery.tsx,hide.ts,icons.tsx,main.tsx,pinmeta.ts,styles.ts}`
