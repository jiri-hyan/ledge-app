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
- Tělo je Markdown (nadpisy, **tučně**, *kurzíva*, `kód`, bloky kódu, odrážky, `1.`, úkoly `- [ ]` /
  `- [x]`, odkazy jen http(s), obrázky relativně `./…png`). Raw HTML se nezobrazí (escapuje se).
  Úkoly Ledge počítá (karta „2/5“) a uživatel je odklikává v náhledu.
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
- Stav (pozice ve fyzických px, velikost v logických, volby) je v `%LOCALAPPDATA%\Ledge\note-pins.json`
  (zápis se zpožděním ~0,4 s, ne v configu — je to stav okna, ne nastavení). Po startu Ledge se připnuté
  lístky vrátí na místo; mimo obrazovku → monitor proužku. ✕ = odepnout, poznámka zůstane.
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

