# Poznámky, lístky, synchronizace

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

## Poznámky

Rychlé poznámky a připomínky žijí **centrálně mimo repa projektů** (vlastní repo poznámek — viz Synchronizace níž): `%USERPROFILE%\Ledge\notes\`
(jiný kořen jen přes `notes.root` v configu). Struktura:

```
notes\
  _obecne\                         ← bez projektu
  atlas\                           ← id projektu z configu
    2026-10-16-140322-zavolat-rehorovi.md
    2026-10-16-140322-zavolat-rehorovi-1.png
```

Jeden `.md` = jedna poznámka, frontmatter + Markdown:

```markdown
---
project: atlas
created: 2026-10-16T14:03:22+02:00
kind: napad
remind: 2026-10-17T09:00:00+02:00
done: false
updated: 2026-10-18T10:12:00+02:00
---

Zavolat Řehořovi kvůli fakturám

![](./2026-10-16-140322-zavolat-rehorovi-1.png)
```

- `project` = id projektu **přesně jak je v configu** (u obecných chybí; jméno složky je jen jeho
  bezpečná podoba; `project: _obecne` = výslovně bez projektu, když soubor leží ve složce projektu),
  `created` a `remind` jsou ISO 8601 **s offsetem** (nikdy naivní čas), `remind` je
  volitelné, `done` true/false. Další klíče smíš přidat — Ledge je zachová.
- `kind` (volitelné) = kategorie, id z `notes.kinds` v configu; výchozí sada `napad` (Nápad),
  `feedback` (Feedback), `ukol` (K zapracování), `info` (Poznámka — platí, když klíč chybí). **Agent
  smí `kind` nastavit** (např. nápad ze session → `kind: napad`). Neznámé id se ukáže jako text, nic se
  neztratí. Vlastní kategorie: config `notes.kinds` (`id`, `label`, `color` `#rrggbb`, `icon` z
  `bulb chat checklist note flag star bug heart`).
- `updated` = poslední úprava v Ledge (ISO s offsetem) — píše Ledge, agent ho nastavovat nemusí.
- `color` (volitelné) = barva lístku: `yellow` `orange` `red` `pink` `purple` `blue` `teal` `green` nebo
  `"#rrggbb"`; chybí = výchozí. `tags` (volitelné) = štítky `tags: [prace, qa]` (Ledge píše jen tenhle inline
  tvar; čte i `tags: a, b` a blokový seznam `- a`). Štítkem je i `#štítek` v textu (ne v kódu, ne `# Nadpis`,
  ne `#1`). Štítek = písmena, číslice, `-` `_` `/`, Ledge ho bere malými písmeny. **Agent smí `color` i `tags`
  nastavit.** Barva podle štítku: config `notes.tagColors` (`{"qa": "red"}`), platí jen bez vlastní `color`.
- Vazba na jinou poznámku: `[[titulek poznámky]]` nebo `[[titulek|popisek]]` (jako Obsidian). Ledge cíl hledá
  podle titulku (první řádek, bez ohledu na velikost písmen a diakritiku), pak podle jména souboru; v náhledu i
  lístku je vazba klikatelná, neexistující nabídne vytvořit novou poznámku s tím titulkem.
- Tělo je Markdown (nadpisy, **tučně**, *kurzíva*, `kód`, bloky kódu, odrážky, `1.`, úkoly `- [ ]` /
  `- [x]`, odkazy jen http(s), obrázky relativně `./…png`). Raw HTML se nezobrazí (escapuje se).
  Úkoly Ledge počítá (karta „2/5“) a uživatel je odklikává v náhledu.
- **Export** (⋯ editoru / lístku, Bento karta): Markdown (bez frontmatteru), samostatné HTML (obrázky jako data URL)
  nebo PDF (A4, totéž HTML vytištěné skrytým WebView2 bez okna a bez fokusu, nejdéle 30 s). Tauri `note_export(path,
  format: "md" | "html" | "pdf")` — dialog „Uložit jako“, poznámka se nemění.
- Uživatel poznámky upravuje přímo v Ledge (klik na řádek); uložení přepíše soubor atomicky, **jméno ani
  složku nemění** (sync gitem) a zachová neznámé klíče i `created`. Když se soubor mezitím změnil
  (sync, agent), Ledge nepřepíše naslepo — nabídne načíst znovu / uložit jako kopii. Frontmatter musí začínat
  přesně `---`, mít jen řádky `klíč: hodnota` a skončit `---` do 30 řádků.
- `.draft\` (rozepsaná poznámka, `edit.json` = rozepsaná úprava) a `.state.json` (které připomínky už vyjely) patří Ledge — nesahat.
- Poznámky **smíš číst** (třeba „co si uživatel poznamenal k tomuhle projektu“) a **vytvářet** ve stejném
  formátu a pojmenování (`YYYY-MM-DD-HHMMSS-slug.md`, slug ASCII malými písmeny). Ledge složku sleduje:
  nová poznámka se hned ukáže v kartě a `remind` v proužku / peeku.
- Hotovou poznámku označ `done: true`. **Nemaž** poznámky ani obrázky a **nepřesouvej** je do `_archiv` —
  smazání a archivace jsou na uživateli (v Ledge: ⋯ na řádku, v editoru a v lístku).
- **Splnit · Archivovat · Smazat** (Ledge): Splnit = `done: true` (poznámka zmizí pod „i hotové“). Archivovat =
  přesun `.md` i jejích obrázků do podsložky `_archiv` vedle poznámky (`atlas\_archiv\x.md`; poznámka přímo v
  kořeni jde do `_obecne\_archiv`) — z karty i přehledu zmizí, najdeš ji ve stránce Poznámky → „Archiv“, odkud
  jde vrátit. Starší Ledge na druhém PC `_archiv` nevidí (skenuje jednu úroveň složek), poznámka mu prostě
  zmizí; sync to přenese jako přejmenování. Smazat = **Koš Windows** (jde obnovit z Koše) s lištou „Zpět“
  ~8 s: soubory nejdřív do odkladu `.smazane\<id>\` v kořeni (v `.gitignore`, mimo sync), po ~10 s se vrátí
  na původní místo a jdou do Koše (jen místní disky; selže-li Koš, poznámka zůstane), zbytky po pádu
  dokončí start. Obrázek se maže/archivuje s poznámkou jen když ho nepoužívá jiná poznámka ve složce.
  `.smazane\` ani `_archiv\` ručně neupravuj.

### Lístky na plochu (overlay okna)

Kteroukoli poznámku jde **připnout na plochu** jako malé okno bez rámu přes ostatní okna (špendlík u
řádku v kartě Poznámky / v editoru, ⋯ karty → „Nový lístek na plochu“, Ctrl+K → „Nový lístek na plochu“ /
„Připnout na plochu: …“). Lístek je **ta samá poznámka** — čte a ukládá se stejně (otisk, konflikt →
načíst znovu / kopie), úkoly se odklikávají, „+ bod“ přidá `- [ ] …`, Ctrl+V vloží snímek obrazovky
(PNG vedle poznámky jako v editoru) nebo text na konec. **Co se napíše, se samo uloží do `.md`** (pauza
v psaní ~0,6 s, ztráta fokusu, odepnutí; štítek „Uloženo ✓“); nový lístek se rovnou píše. ⋯ lístku má
Splnit · Archivovat · Smazat — lístek zůstane pár sekund s lištou „Zpět“ a pak se zavře. Agent na lístky nesahá — stačí upravit `.md`,
otevřený lístek se sám obnoví.

- Okna `note-<klíč>` (`pins/win.rs`): bez fokusu při otevření, mimo hlavní panel, volby per lístek:
  vždy navrchu / normálně, zamknout na místě, průchozí (myš jde na stránku pod ním; vypne zkratka
  `hotkeys.notePassthrough` nebo Ctrl+K „Lístky: vypnout průchozí režim“), průhlednost pozadí a textu
  (text nejníž 50 %), sbalení na lištu (dvojklik). Ve fullscreenu / herním režimu lístky přestanou být
  navrchu a nové se neotevřou.
- Stav (pozice ve fyzických px, velikost v logických, volby, styl) je v `%LOCALAPPDATA%\Ledge\note-pins.json`
  (zápis se zpožděním ~0,4 s, ne v configu ani v `.md` — je to stav okna **tohoto PC**, nesynchronizuje se).
  Po startu Ledge se připnuté lístky vrátí na místo; mimo obrazovku → monitor proužku. ✕ = odepnout, poznámka
  zůstane a Ledge si pamatuje polohu, velikost a vzhled — znovupřipnutí ji otevře tam (posledních 200 odepnutých).
- Lístky 2.0: styl (`classic` = původní tmavé sklo, `paper`, `glass`, `stripe`, `minimal`, `dark`), „Použít na
  všechny lístky“, „Schovat do…“ (okno zmizí a v zadaný čas se samo vrátí; termín drží vlákno `emit`, žádný
  časovač navíc), štítky a barva v ⋯ (zapisují `tags` / `color` do frontmatteru), vazby `[[…]]` otevřou další
  lístek. Výchozí vzhled nového lístku: config `notes.pins` (`style`, `bg`, `text`, `rememberLast` = převzít
  naposledy použitý, výchozí `true`). Zkratka „Nový lístek“ `hotkeys.notePin` (výchozí prázdná = vypnutá; platí
  i s vypnutými zkratkami, ve hře nic).
  Uživatelský návod: [`docs/listky.md`](../listky.md).

### Synchronizace mezi PC (`notes.sync`)

Kořen poznámek je **pracovní kopie soukromého git repa** (`notes.sync.remote`, výchozí prázdné a
`enabled: false` — každý si nastaví své repo; `intervalMin` 1–120, výchozí 5).
Synchronizuje vlákno `notes-sync` (`src-tauri/src/notesync/`): po startu, každých `intervalMin`
minut, 10 s po lokální změně (nejpozději minutu) a při otevření šuplíku (nejvýš 1× za minutu);
ručně „Synchronizovat teď“ v Nastavení. Cyklus: `add -A` → commit `ledge: <COMPUTERNAME> <ISO čas>`
→ `fetch` → `rebase --autostash` → `push`, vždy bez okna a promptu (`GIT_TERMINAL_PROMPT=0`,
`GCM_INTERACTIVE=never`). Přihlášení jednou na PC: `gh auth setup-git`.

- První sync: kořen bez repa + prázdný remote → `git init`; kořen bez repa + remote s commity (druhé
  PC) → clone vedle, lokální poznámky se sloučí (stejná cesta s jiným obsahem → lokální jako
  `<jméno>-<COMPUTERNAME>.md` i s obrázky), stará složka zůstane jako `notes.pred-sync-<čas>`.
- `.draft\`, `.state.json`, `*.tmp`, `PRESUNUTO-DO-LEDGE.txt`, `.smazane\` jsou v `.gitignore` — patří jen tomuto PC.
- Konflikt (stejná poznámka změněná na dvou PC) → rebase se přeruší, lokální verze zůstane
  v lokálním commitu, štítek v kartě ukáže „konflikt: …“. Vyřešit ručně, pak se sync rozjede sám.
- **Claude poznámky smí vytvářet a upravovat (`done`, `kind`), Ledge je sám commitne a pushne.** Claude
  v kořeni poznámek **necommituje, nepushuje, nepulluje ani nerebasuje** sám — sahal by do běžícího cyklu.

### Propojení se složkou (`notes.folderLink`)

Volitelné a výchozí **vypnuté**: poznámky Ledge se kopírují jako obyčejné `.md` soubory do a ze složky, kterou umí
jiné aplikace (vault Obsidianu, `pages` Logseq, export Joplinu / Notesnooku, VS Code). Nezávislé na gitové
synchronizaci výš (`notes.sync`), ta se nemění. Config: `notes.folderLink` = `enabled` (výchozí `false`), `path`
(absolutní cesta ke složce), `direction` (`import` | `export` | `both`, výchozí `both`), `propagateDeletes`
(výchozí `false`). Nastavení → Poznámky → „Propojit se složkou“, kód `src-tauri/src/notes/folder_sync*.rs`.

- Jedna poznámka = jeden `.md`, kopie je **bajtová** (frontmatter včetně cizích klíčů, `[[vazby]]`, `#štítky`, konce
  řádků). Agent smí takové soubory číst i upravovat jako ostatní poznámky; ve složce je Ledge přenese.
- Čte se jen `.md` / `.markdown` do 1 MB, rekurzivně; skryté složky a soubory (`.obsidian`, `.trash`, `.git`),
  `logseq/` a odkazy (symlink / junction) se ignorují. Složka nesmí být kořen disku ani se překrývat s úložištěm
  poznámek. Mimo vybranou složku a kořen poznámek se nezapisuje nikdy.
- Import jde do `_obecne\<jméno>.md`; export pojmenuje soubor podle titulku (znaky `<>:"/\|?*#^[]` se nahradí
  mezerou, rezervovaná jména Windows dostanou `_`), poznámky projektu jdou do podsložky, PNG obrázky vedle.
- Konflikt (od poslední dohody se změnily obě strany): nic se nepřepíše potichu. Zůstane verze ze složky, verze
  z Ledge se uloží jako `název (konflikt Ledge YYYY-MM-DD).md` (v Ledge i ve složce) a ukáže se v Nastavení
  s „Rozumím“. Při `direction: export` je naopak zdrojem Ledge a verze ze složky se uloží jako kopie ve složce.
- Mazání: výchozí je nepřenášet (smazaná poznámka se na druhé straně nechá být a nevrací se, dokud se nezmění).
  S `propagateDeletes` jde soubor smazaný ve složce do Archivu Ledge a poznámka smazaná nebo archivovaná v Ledge do podsložky
  `.trash` složky; nikdy natrvalo. Podezřele mnoho smazání najednou (víc než 10 a čtvrtina) se nepřenese.
- Stav (dvojice cest, hashe, otisky) je v `%LOCALAPPDATA%\Ledge\note-folder-link.json` (jen tento počítač).
  Bez stavu se shodné soubory spárují podle obsahu, takže nevznikají duplicity.
- Spouští se událostmi (změna poznámek, změna ve složce přes `notify`, otevření šuplíku, config, „Synchronizovat
  teď“), nikdy časovačem; v klidu 0 % CPU.
