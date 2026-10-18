# Ledge — návod pro agenty (Claude Code a spol.)

Ledge je always-on „šuplík“ u horního okraje obrazovky: projekty (klik = dev server / appka),
běžící servery a schvalování pro Claude Code. Tenhle soubor říká, jak ho **nastavovat a ovládat**
bez UI. Architektura je v hlavičkách souborů `src-tauri/src/*.rs`.

**Dřív se jmenoval Notch** (do 0.1.0, přejmenováno 2026-10-17; veřejná vydání: `jiri-hyan/ledge-app`). Staré cesty `%APPDATA%\Notch`, `%USERPROFILE%\Notch\notes`,
`%LOCALAPPDATA%\Notch` už **nepoužívej** — viz [Migrace z Notche](#migrace-z-notche).

## Konfigurace = jeden soubor

**Všechno jde nastavit klikem v Nastavení** (ozubené kolo v šuplíku). Panely (od vlny 6, id v
`src/settings/registry.ts`; staré id vedou dál): **Začínáme** (průvodce, tipy, novinky) · **Šuplík a karty**
(záložky, karty, přihrádka) · **Vzhled** (4 kroky: Zavřený — vzhled zavřeného Ledge a celý proužek; Po najetí; Otevřený — styl, šířka a hustota na monitoru ve skutečném poměru, barvy; Pohyb — animace; jazyk) · **Projekty,
akce a odkazy** (projekty se autodetekcí ze složky, jeden seznam akcí — odkaz je akce —, skripty) · **Claude**
(schvalování, otázky, Claude (MCP), limity, útrata, hlídač, hooky) · **Osobní** (počasí, kalendář, přehrávač, časovač,
historie schránky, GitHub, Systém) · **Poznámky** (i sync) · **Chování** (otevírání, zkratky, paleta, herní režim, šuplík nad
prohlížečem, spouštění po startu, šetřit paměť) · **Záloha a aplikace** (export / import s výběrem částí /
obnova výchozích, verze, aktualizace). Hledání (Ctrl+F) najde i staré názvy voleb. Ruční
úprava souboru níže je pro pokročilé a pro agenty; Nastavení ukáže chybu validace přímo u pole.

`%APPDATA%\Ledge\config.json`, vedle něj `config.schema.json` (generované z Rust struktur — přesný
tvar všech polí a jejich popis). Uložení se projeví **hned** (Ledge soubor sleduje). Nevalidní soubor
se neaplikuje: Ledge drží poslední dobrou verzi a chybu ukáže v šuplíku — po úpravě zkontroluj
`GET /status` nebo šuplík. Nečitelný soubor se před každým přepsáním (Nastavení, průvodce) zkopíruje
vedle jako `config.json.broken-<unix s>` — rozbitý config se tak nikdy potichu nenahradí výchozím.

**Čistý start (nový uživatel).** Když `config.json` neexistuje a není co převzít z Notche, Ledge založí
čistý config (`onboarding::fresh_config`): **žádné projekty**, `roots` = Plocha, sync poznámek vypnutý
s prázdným `remote`, `update.repo` = `jiri-hyan/ledge-app` (veřejné repo), `onboarding.done: false`. Šuplík
pak při prvním otevření ukáže **průvodce** (jazyk `ui.lang`, repa na disku, aplikace ze Start menu,
hooky Claude Code, odkazy, motiv) — každý krok se hned ukládá, nic nemaže, duplicity přeskočí. Po průvodci
(a u stávajícího uživatele jednou po aktualizaci) krátká **prohlídka** 3 bublin (✎ úpravy, ⋯ menu karty,
Ctrl+K) — přeskočitelná, bez modalu a bez braní fokusu; dokončení i přeskočení zapíše `"ui": { "tips":
["tour1"] }` a už se neukáže (smazáním `tour1` z `ui.tips` se ukáže znovu).
Ručně: Nastavení → Začínáme → Spustit průvodce. **Existující config se nemění:** chybějící `onboarding` = `done: true`
(průvodce se neukáže); chybějící `notes.sync.remote` se při načtení vezme z `origin` pracovní kopie
poznámek (kde sync běžel, běží dál), jinak zůstane sync vypnutý; `update.repo` = dřívější
`jiri-hyan/ledge` se při načtení převede na `jiri-hyan/ledge-app` (zapíše se až dalším uložením).
V kódu nejsou žádná autorova repa.

Typické úpravy:

- **Přidat projekt** — položka do `projects`:
  ```json
  { "id": "harbor", "name": "Harbor", "kind": "dev", "remote": "acme/harbor",
    "run": "npm run dev", "port": 5180, "accent": "#2563eb", "logo": "public/favicon.svg" }
  ```
  `path` neuváděj, pokud nemusíš — složka se dohledá podle `remote` v `roots` (funguje na obou
  strojích). `kind: "app"` = `.exe` (`exe` relativně k projektu, nebo `appName` nainstalované appky),
  volitelně s přípravným `run` (build).
- **Logo je volitelné.** Bez `logo` (nebo když soubor neexistuje — jiná větev) ho Ledge najde sám:
  u `appName` ikona nainstalovaného exe, jinak ikona z `<link rel="icon">` v `index.html` / web
  manifestu a obvyklé soubory (`public/favicon.*`, `build/icon.*`, `src-tauri/icons/…`), SVG a větší
  PNG mají přednost. Rastry se převádí na PNG ≤ 128 px. `logo` uváděj jen, když autodetekce vybere
  špatně. Přepočet: po změně configu, po klonu a při otevření šuplíku nejvýš 1× za 5 min.
- **Vzhled jednotlivé dlaždice** (`look`, projekt i akce, `src-tauri/src/tilelook.rs`; chybí = automaticky
  jako dřív, nic se nepíše): `"look": { "icon": { "kind": "emoji", "value": "🎮" }, "color": "#2563eb",
  "shape": "circle", "label": "Web", "hideLabel": true }`. `icon.kind`: `emoji` (≤ 8 znaků) | `glyph`
  (systémová ikona Segoe Fluent Icons, kód `E80F`, rozsah E000–F8FF) | `image` (klíč 16 hex — obrázek
  vybraný v šuplíku, zmenšený přes WIC do `%LOCALAPPDATA%\Ledge\looks\<klíč>.png`; ručně ho nezadávej,
  chybějící soubor = Auto). `color` = podklad `#rrggbb` (kresba černá / bílá podle kontrastu ≥ 4.5:1),
  `shape` `rounded` (výchozí) | `circle` | `square` | `glass`, `label` ≤ 40 znaků. Mění se v šuplíku: pravý
  klik na dlaždici / ⋯ v akcích dlaždice → „Upravit vzhled…“ (v režimu úprav stačí klik); přehled a
  „Obnovit vše“ v Nastavení → Šuplík a karty → Upravené karty a zástupci.
- **Co a odkud se spouští** (`src/projects/`, `resolve::place_of`): tooltip dlaždice = příkaz, složka, port;
  ⋯ / pravý klik → **Upravit…** (příkaz jde přepsat přímo, Enter uloží `run`), **Změnit složku…** (dialog
  Windows → `path`), Stop / Restart. `path` v configu vyhrává — **neexistující `path` = chyba na dlaždici**,
  žádné tiché hledání jinde. Bez `path` se hledá podle `remote` v `roots`; počítá se i linked worktree
  (`Desktop\app-v2` vedle `Desktop\app`). **Víc složek s jedním repem = Ledge nehádá**: dlaždice ukáže
  „vyber složku“ a klik otevře výběr (cesta + datum posledního commitu, nejčerstvější předvybraná), volba
  se uloží do `path`. Chceš zpět dohledávání podle repa → „Hledat podle repa“ (smaže `path`).
- **Menu dlaždice je jedno pro všechny velikosti** (S/M/L/XL, seznam i jen ikony): pravý klik, ⋯ (po najetí
  / fokusu) nebo Shift+F10 / klávesa menu → Spustit / Stop / Restart, Otevřít v prohlížeči, Otevřít složku,
  Zobrazit log, Změnit složku…, Změnit příkaz…, Vzhled…, Upravit…, Odebrat (skryje dlaždici na záložce, s
  Vrátit). Menu, Vzhled… i Upravit… se kreslí nad celým šuplíkem a u okraje se otočí / posunou (`src/ui/place.ts`).
  Najetí myší = jen náhled (jméno, stav, čím a odkud). Řada dlaždic vždy vyplní šířku karty (`tileColumns`).
- **Selhání startu nikdy potichu**: skončí-li server před naběhnutím, nenaběhne-li port do 90 s, nebo chybí
  složka / příkaz → dlaždice zčervená, pod ní krátká chyba + poslední řádky logu, Zobrazit log, Změnit
  složku…, Zkusit znovu. Windows oznámení jen když šuplík není otevřený (a ne ve fullscreen hře).
- **Zastavit server** jde: ■ v akcích dlaždice, pilulka „N běží“ v hlavičce (klik = seznam se Stop / Restart /
  Otevřít / Log, „Zastavit všechny“), kompaktní náhled po najetí (`drawer.hover: "compact"`; proužek sám je
  průchozí pro myš), paleta Ctrl+K („Zastavit …“, „Restartovat …“, „Zastavit všechny servery“), karta Běží,
  `POST /ctl/stop`. Stop = celý strom (Job Object). Port projektu, který drží proces, jenž Ledge
  nespustil, pilulka jen ukáže — ukončí ho až po potvrzení (jen ten jeden PID, ověřený těsně před tím).
- **Projekt, který na tomhle PC není** (dlaždice „nenalezeno“), má v rychlých akcích **Naklonovat**:
  `gh repo clone <owner/repo>` (jiný hostitel než GitHub: `git clone <url>`) do první složky z `roots`
  (kolize jména → `<repo>-<owner>`, `<repo>-2`…), bez okna, termín 15 min, pak se projekt dohledá sám.
  Nenabízí se u dvou klonů téhož repa (nejednoznačné) ani u `appName` bez `exe`.
- **Rozložení šuplíku — záložky a karty** (`layout`, `src-tauri/src/layout.rs`; jediný zdroj pravdy):
  ```json
  "layout": {
    "pages": [
      { "id": "prace", "name": "", "cards": [{ "id": "projects", "size": "xl" }, { "id": "claude", "size": "m", "h": 3 }] },
      { "id": "osobni", "name": "", "cards": [{ "id": "projects", "size": "xl",
          "tiles": { "size": "small", "label": true, "git": false, "hidden": ["harbor"],
            "folders": [{ "id": "slozka", "name": "Weby", "items": ["ledge", "kino"] }] } },
        { "id": "repos", "size": "m", "look": { "accent": "teal", "bg": "tint", "hideHead": true, "density": "compact", "view": "tiles" } }] },
      { "id": "hry", "name": "Hry", "icon": "🎮", "cards": [{ "id": "media", "size": "m" }] }
    ],
    "hiddenParts": ["claudeCosts"],
    "upgrades": ["personalProjects"]
  }
  ```
  Záložky v pořadí (1–9, klávesy 1–9 / paleta „Přejít na záložku“); `prace` a `osobni` jsou vestavěné
  (prázdný `name` = „Práce“ / „Osobní“ podle jazyka), vlastní záložka potřebuje `name`, `id` = `a-z0-9-`.
  Karty: `projects`, `running`, `claude`, `repos`, `notes`, `actions`, `shelf`, `clock`, `timer`, `weather`,
  `calendar`, `links`, `media` (Právě hraje + hlasitost), `github`, `system`, `mail` (Pošta — účet Microsoft;
  ty tři jsou jen v katalogu, na žádné záložce ve výchozím rozložení nejsou — `layout::OPT_IN`) a **dynamické** `widget:<soubor>` (widget ze
  skriptu ve složce widgetů — jen jméno souboru, bez cesty; katalog je nabízí podle složky, viz „Widgety ze
  skriptů“); stejná karta smí být na víc záložkách, na jedné
  jen jednou. `size`: `s` (1 sloupec), `m` (2), `l` (3), `xl` (celá řada) z `--deck-cols` (výchozí 4 — počet
  sloupců určuje styl rozložení ve Vzhledu). **Řada nikdy nekončí prázdným místem:** karty jdou v pořadí
  záložky a volné sloupce řady, do které se další karta nevejde, dostane největší karta té řady (pořadí se
  nemění; menší zůstane menší). Když volba velikosti nemá viditelný efekt (karta sama v řadě, málo sloupců),
  šuplík to řekne a vybere nejbližší možnou. Šuplík je vysoký nejvýš 80 % pracovní plochy, dál scrolluje.
  `h` (volitelné) = výška karty v řádcích 1–4 (řádek ≈ 104 px × hustota); chybí = podle obsahu. S pevnou
  výškou obsah scrolluje uvnitř karty, ne celý šuplík. `tiles` (jen karta `projects` = zástupci, volby
  pro tu záložku): `size` `small` (malé ikony) | `tile` (výchozí) | `large`, `label` (název pod ikonou,
  výchozí `true`), `git` (větev a stav gitu, výchozí `true`), `hidden` = id projektů, které tu nejsou
  (prázdné = všechny, nový projekt se ukáže sám), `folders` = složky zástupců (v režimu úprav tah dlaždice
  na jinou = složka, na složku = do ní, ze složky ven = vyjmout; `id` `a-z0-9-`, `name` ≤ 32 znaků, projekt
  jen v jedné složce, prázdná složka zmizí; chybí = žádné). `look` (volitelné, každá karta zvlášť — i nové
  karty; chybí = podle design stylu): `accent` (`accent`, `blue`, `green`, `amber`, `red`, `violet`, `teal`
  = barvy motivu, nebo `#rrggbb`), `bg` (`tint` jemný tón akcentu | `solid` plné | `clear` průhledné),
  `hideHead` (bez hlavičky, ⋯ zůstane při najetí), `density` (`compact` | `airy`), `view` jen u karet
  `projects` (výchozí `tiles`), `repos` a `notes` (výchozí `list`), `actions` (výchozí `tiles`): `list` |
  `tiles` | `compact`. V šuplíku ⋯ na kartě → „Vzhled karty…“ (živý náhled, Obnovit podle stylu).
  `limit` (volitelné, jen karty `claude`, `repos`, `running`; 0–50, `0` = bez stropu, chybí = 4) = kolik řádků
  karta ukáže, zbytek pod „Zobrazit další (k)“ ↔ „Méně“; session, které čekají na tebe, se nikdy neschovají
  (⋯ karty → Řádků 3/4/6/Vše, nebo Nastavení → Šuplík a karty).
  `upgrades` = jednorázové úpravy rozložení po aktualizaci,
  které už proběhly — neměň ručně: `personalProjects` dá kartu Projekty nahoru na Osobní, když
  tam nikdy nebyla (jen když ji uživatel používá na jiné záložce a Osobní existuje); odebereš-li ji
  potom, už se nevrátí.
  Karta, která není na žádné záložce, je skrytá a nic nepočítá: skryté počasí ani kalendář se nestahují
  (stahují se, jen když je karta na otevřené záložce), skrytá útrata / karta Claude neindexuje transcripty,
  skryté Právě hraje / hlasitost / Čas (baterie) zruší odběry událostí Windows, skrytý GitHub / Systém
  nevolá `gh` ani nevzorkuje. `hiddenParts` = skryté části
  karet: `claudeLimits`, `claudeCosts`, `claudeGit`, `claudeSubagents` (karta Claude), `nowPlaying`, `sound`
  (karta Právě hraje), `battery` (karta Čas). Úpravy: v šuplíku ✎ (přetáhnout, **tah za pravý okraj karty
  = šířka, za spodní = výška, za roh = obojí** — karta se mění živě s „2 × 3“, ⇔ / ↕ klikem, × do katalogu,
  „+ Přidat kartu“, vlastní záložky, Zpět / Ctrl+Z po každé změně, Obnovit výchozí s potvrzením, klávesy na ⠿:
  šipky přesun, + / − šířka, PageUp / PageDown výška; Hotovo uloží jen `layout`), ⋯ na kartě (Velikost S/M/L/XL, Výška Auto/Nízká/
  Vysoká, u Projektů dlaždice / název / git / které projekty, skrýt kartu / sekci — ukládá hned), nebo
  Nastavení → Šuplík a karty → Záložky a karty (i výška a volby Projektů). **`drawer.sections` a `modes.personalEnabled` jsou od
  vlny 6 odvozené** z `layout` (při načtení, uložení i aplikaci; ruční změna se přepíše) — čtou je moduly
  podle karty a starší Ledge. Starý config bez `layout` se při načtení převede beze ztráty: viditelnost
  z `drawer.sections`, pořadí karet Práce z dřívějšího `appearance.cards`, záložka Osobní jen při
  `modes.personalEnabled != false`, dnešní pořadí Osobní. Celé funkce dál vypínají `strip.enabled`,
  `notes.enabled`, `shelf.enabled`, `claude.costs`.
- **Po najetí myší** — `drawer.hover` (Nastavení → Vzhled → Po najetí): `"full"` = rovnou celý šuplík
  (výchozí; starý config beze změny), `"compact"` = kompaktní náhled (malý panel pod proužkem: čekající
  schválení / otázka s Povolit / Zamítnout, nejvýš 3 session Clauda, Právě hraje s ⏮⏯⏭, čas a limit 5h,
  „▾ celý šuplík“), `"island"` = roztažený ostrůvek (černá kapsle s jedním nejdůležitějším děním: čeká na tebe →
  hotovo → časovač → hudba → Claude pracuje → klid s časem, datem a počasím). Z náhledu otevře celý šuplík klik
  mimo tlačítka nebo setrvání myši `drawer.hoverExpandMs` (výchozí `1200`, `0` = jen klikem, jinak 400–10000;
  nad tlačítky a s čekajícím schválením se neodpočítává). Odjetí myši zavře jako šuplík; s čekajícím Claudem
  zůstane peek se schválením (ve všech režimech — najetí při čekání otevře peek). Náhled bere jen data, která
  Ledge už má (nic nestahuje, fokus nebere); ve hře / herním režimu se neukáže. Bez myši:
  `POST /ctl/open {"view":"compact"}` / `{"view":"island"}`.
- **Proužek** — `strip.items` (pořadí): `mode`, `clock`, `running`, `claude`, `waiting`, `repos`, volitelně
  `limits` (limity Claude 5h / týden), `custom` (vlastní text), `timer` (časovač), `media` (právě hraje:
  „♪ Název · Interpret“ jen při přehrávání, pauza / nic = žádný segment), `mail` (pošta účtu Microsoft:
  „✉ 3“ jen při nepřečtených; s touto položkou se pošta obnovuje nejvýš 1× za 5 min i se zavřeným šuplíkem,
  jen ve stylu `classic`), `github` („GitHub: 2× čeká na revizi“,
  jen když > 0 a jen se zapnutým `github.enabled`) a `system` („CPU 23 % · RAM 61 %“) — ve výchozím stavu vypnuté — a
  parametrizovaná `widget:<soubor>` (text widgetu ze skriptu ≤ 24 znaků, barva výstupu jako tečka; jde i do
  `strip.colors` / `strip.quiet` jako klíč). Config proužku žije v `src-tauri/src/stripconf.rs` (re-export z `config.rs`). Další pole:
  - `strip.custom` — text položky `custom`: krátký text a/nebo emoji, nejvýš 24 znaků (víc = chyba
    validace); prázdný = položka nic neukáže. Emoji kreslí proužek jednobarevně (DirectWrite bez barevných fontů).
  - `strip.colors` — barva textu položky, `{"clock": "teal", "custom": "accent"}`; předvolby `accent`
    (akcent motivu nebo `appearance.accent`), `text`, `muted`, `violet`, `blue`, `teal`, `green`, `amber`,
    `red`. Volný hex záměrně není: barva se v každém motivu dorovná na kontrast ≥ 4.5:1 (zesvětlí /
    ztmaví, `appearance::geom::readable`). Tečka stavu (zelená = běží…) zůstává.
  - `strip.quiet` — položky skryté „v klidu“, ukážou se jen, když se něco děje: běží server, Claude
    pracuje / čeká / je hotový, běží časovač, hraje hudba (`media`), limity ≥ 70 %, připomínka po termínu nebo barevná linka
    (neuložená repa se jako událost nepočítají — je to trvalý stav). Smysl má u `mode`, `clock`, `custom`,
    `limits`; `running`, `claude`, `waiting`, `repos`, `timer`, `media`, `github` se v klidu neukazují samy (`system` je stálá, smysl má „Jen při dění“ i u ní). Výchozí `[]`.
  - Nastavení → Vzhled → Zavřený → Pokročilé → Co proužek ukazuje (celý proužek je v panelu Vzhled): řazení tahem nebo Alt+↑/↓, u položky barva a „Jen při dění“,
    náhled s ukázkovými texty (přepínač „V klidu / Když se něco děje“).
  - `strip.look` — **styl zavřeného Ledge** (vlna 7, `src-tauri/src/looks/*`, kreslí `stripdraw.rs`): jedna volba
    určí tvar, místo, velikost i obsah. `classic` (výchozí; starý config = beze změny) = proužek podle `strip.items`
    a `appearance.strip.style`. Ostatní: `island` (černá kapsle, v klidu malá, při dění se plynule roztáhne —
    bez animace při „omezit pohyb“ / `motion.style: none`), `dots` (4 tečky: Claude, servery, repa, ty), `line`
    (3 px přes celou šířku, barevné úseky), `icons` (ikony Segoe Fluent Icons / MDL2 Assets s odznaky čísel),
    `clock` (velké hodiny + stavová tečka), `corner` (odznak v rohu; roh = `appearance.strip.position`, střed =
    pravý), `ghost` (v klidu nic, jen „čeká na tebe“ / „hotovo“; najetí k hraně šuplík otevře dál), `bar` (lišta
    přes celou šířku, monospace, vpravo limit 5h a čas), `notch` (černý výřez s oušky), `progress` (text + lišta
    průběhu: limit 5h / časovač / skladba, jen z reálných dat), `ticker` (jeden řádek; víc událostí se střídá po
    5 s, jinak žádný časovač). Priorita: čeká na tebe (i připomínka) > hotovo (session doběhlá za 5 min) >
    časovač > Claude pracuje > hudba > klid. U stylů mimo `classic` se `items`, `quiet`, `colors`, `custom`,
    `opacity` a `appearance.strip.style`/`color` nepoužijí; `appearance.strip.size` styl zvětší / zmenší.
    `strip.enabled: false` = jako dřív jen linka při událostech; nad taby prohlížeče ustoupí na linku všechny
    styly kromě `line`, `dots`, `corner`, `ghost`. Seznam s názvy pro Nastavení: `src/strip/looks.ts` (`STRIP_LOOKS`).
  - `strip.lookHide` — Pokročilé u stylů mimo `classic`: co styl nemá ukazovat (`waiting`, `claude` = pracuje /
    hotovo, `running`, `repos`, `timer`, `media`, `limits`, `clock`). Výchozí `[]` = všechno.
- **Časovač** — sekce `timer` (Nastavení → Osobní → Časovač, `src-tauri/src/timer.rs`): `presets`
  (minuty, 1–600, nejvýš 6; výchozí `[5, 15, 25, 50]`), `toast` (výchozí `true`: oznámení Windows po
  doběhnutí odpočtu — bez fokusu, ne ve hře / herním režimu / při „nerušit“ / s otevřeným šuplíkem), `strip`
  (výchozí `true`: běžící časovač se ukáže na začátku proužku i bez položky `timer`; přepínač je i v Nastavení →
  Vzhled → Co proužek ukazuje). Odpočet (předvolby, vlastní minuty, pauza, +5 min, zrušit) a stopky (start, pauza,
  reset) v kartě Časovač na záložce Osobní a v paletě Ctrl+K („Časovač 25 min“, „Stopky“, „Zrušit
  časovač“…). Proužek ukazuje „⏱ 18 min“ (zbývá, zaokrouhleno nahoru), stopky „⏱ 0:42“ (h:mm), pauza „⏸“,
  konec „⏱ čas vypršel“ (10 min, pak zmizí); překresluje se jen po minutách, sekundy počítá otevřený šuplík.
  Konec odpočtu = modré bliknutí + puls proužku (jako upozornění Clauda) a volitelný toast; ve fullscreen
  hře nic. Stav (časy, ne tik) je v `%APPDATA%\Ledge\timer.json` — zapisuje se jen při akci, takže běžící
  odpočet přežije restart Ledge (aktualizace); doběhl-li mezitím, ukáže se jen tiše „čas vypršel“. Smazání
  souboru = nic neběží po dalším startu.
- **Kalendář (jen čtení, bez přihlášení)** — `personal.calendars` (Nastavení → Osobní → Kalendáře; kód
  `src-tauri/src/calendar/`, mapa `docs/codemap/calendar.md`): seznam `{ "name", "url", "color", "enabled", "source", "remindMin", "remindToast" }`
  (nejvýš 8). `url` = **tajná adresa ve formátu iCal** (Google: Nastavení kalendáře → Integrovat kalendář;
  Outlook: Nastavení → Kalendář → Sdílené kalendáře → Publikovat kalendář → ICS), jen `https://`
  (`webcal://` se bere jako https, `http://` neprojde validací). Adresa je tajemství: Ledge ji neloguje, v chybách
  ani v UI ji neukazuje (jen doménu), cache `%LOCALAPPDATA%\Ledge\calendar-cache.json` je klíčovaná otiskem
  SHA-256; export zálohy configu ji obsahuje (jako zbytek configu). `name` prázdné = název z feedu
  (`X-WR-CALNAME`), `color` `#rrggbb` nebo `null` (podle pořadí), `enabled` výchozí `true`. Přidání v Nastavení:
  vložit odkaz → Přidat (jedno zkušební stažení, Tauri `calendar_test`). Karta Kalendář na Osobní: Dnes (bez
  skončených), Zítra, a když obojí chybí, nejbližší další den do 14 dní; celodenní nahoře, probíhající
  zvýrazněná; patička = kalendáře (klik vypne/zapne). Stahuje se jen při otevřeném šuplíku na Osobní se
  zapnutou kartou, když jsou data starší 15 min (po pokusu 2 min pauza), odpověď nejvýš 5 MB / 15 s; v klidu
  nic. Opakování (RRULE běžné tvary, EXDATE, RECURRENCE-ID) a pásma (VTIMEZONE z feedu, běžné IANA
  i Windows názvy; neznámé = místní čas, karta ho označí „≈“).
  - **Zdroj** `source` (výchozí `"ics"` = starý config beze změny): `"outlook-local"` = výchozí kalendář
    **klasického Outlooku v tomto PC** přes COM (bez přihlášení a sítě; `url` se nepoužije, v seznamu nejvýš
    jednou). Automaticky se načte jen když Outlook běží (připojí se, žádné okno ani fokus); zavřený Outlook
    jen po kliku „Načíst z Outlooku“ (COM ho spustí na pozadí). Text pozvánky (odkaz Teams) čte jen když
    Outlook volajícímu věří (`Application.IsTrusted`) — jinak by vyskočila bezpečnostní výzva Outlooku.
    Nový Outlook COM nemá → odkaz ICS. `"graph"` / `"google"` + `account` = účty Microsoft / Google
    (`calendar/graph.rs`, `google.rs` přes `source::adapter`; přihlášení v Nastavení → Účty). Bez řádku tady
    dá kalendář účtu do karty přepínač `accounts.*.calendar` (bez připomínky); je-li zapnutý řádek téhož
    účtu tady, vede ho tenhle (připomínka, Připojit) a přepínač účtu se nezdvojí.
  - **Připojit:** z vlastností schůzky (`X-MICROSOFT-SKYPETEAMSMEETINGURL`, `X-GOOGLE-CONFERENCE`,
    `CONFERENCE`), `URL`, místa a popisu (i HTML, Safe Links) Ledge vezme odkaz Teams / Meet / Zoom / Webex /
    Jitsi a u události ukáže tlačítko Připojit (Teams rovnou v aplikaci `msteams:`, jinak https; Tauri
    `calendar_join` otevře jen ověřený odkaz známé služby).
  - **Připomínka:** `remindMin` (výchozí `5`, `0` = vypnuto, nejvýš `120`) minut před začátkem časové
    události proužek ukáže „📅 Název za 5 min“ (modře, i s vypnutým proužkem; priorita mezi „čeká na
    tebe“ a „hotovo“), blikne a pulsne; `remindToast: true` (výchozí `false`) navíc pošle toast Windows
    s tlačítkem Připojit. Budík jen na čas připomínky, nic se kvůli ní nestahuje (data z posledního
    načtení / cache). Ve fullscreen hře / herním režimu nic, toast ani při „nerušit“ nebo otevřeném šuplíku.
- **Účty Microsoft a Google (OAuth)** — sekce `accounts` (Nastavení → Účty; kód `src-tauri/src/accounts/`,
  mapa `docs/codemap/accounts.md`, návod pro uživatele `docs/ucty.md`). Výchozí = nic nepřihlášeno, žádná síť.
  ```json
  "accounts": {
    "microsoft": { "clientId": "", "tenant": "common", "calendar": true, "mail": true, "color": null },
    "google": { "clientId": "", "calendar": true, "calendars": [], "color": null }
  }
  ```
  `clientId` = Client ID **vlastní registrace aplikace** uživatele (Ledge žádné výchozí nemá — veřejná
  open-source appka bez ověřeného OAuth klienta): Microsoft = GUID z Azure Portal (veřejný klient, redirect
  `http://localhost`), Google = `….apps.googleusercontent.com` (klient „Desktop app“). `tenant` = `common` /
  `organizations` / `consumers` / ID tenantu. `calendar` = události účtu v kartě Kalendář (za odkazy ICS),
  `mail` = karta Pošta + položka proužku `mail`; `calendars` = id kalendářů Google (prázdné = hlavní, nejvýš
  10); `color` `#rrggbb` nebo `null`. **Přihlášení samo v configu není:** refresh token (a u Google „client
  secret“, který u desktopových klientů tajný není) leží ve Správci pověření Windows (`Ledge/accounts/<účet>`),
  access token jen v paměti; config, log ani chyby žádné tokeny neobsahují. Oprávnění: Microsoft `offline_access
  User.Read Calendars.Read Mail.ReadBasic`, Google `calendar.readonly` (Gmail ne — „restricted“ scopes chtějí
  audit). Stahuje se jen s viditelnou kartou (kalendář jako ICS: 15 min, pošta 2 min), termín 15 s na dotaz.
  Tauri: `accounts_status`, `account_connect`, `account_cancel`, `account_disconnect`, `account_google_calendars`.
- **Připomínka resetu limitu** — `claude.limitsReminder` (výchozí `true`): když 5h nebo týdenní limit
  Claude dosáhl 100 %, v čase resetu krátký peek (viz „Limity Claude 5h / týden“).
- **Útrata Claude (odhad)** — `claude.costs` (výchozí `true`): sekce dole v kartě Claude s tokeny a odhadem
  ceny v cenách API podle projektu a dne (viz „Útrata Claude (odhad)“); `false` = sekce zmizí, nic se nečte.
- **Vzhled** — sekce `appearance` (Nastavení → Vzhled; chybějící sekce = dnešní vzhled):
  `theme` (`dark` výchozí, `midnight`, `graphite`, `nord`, `light`, `contrast`, `terminal`, `paper`, `neon`, `brutal`, `pastel`), `accent` (`#rrggbb`,
  prázdné = akcent motivu), `density` (`compact|normal|comfy`), `fontScale` 0.85–1.25, `radius`
  (`sharp|normal|round`), `glass.opacity` 0.25–1 (krytí pozadí; rozmazání je `drawer.glass: "acrylic"`),
  `motion.style` (`spring|smooth|snappy|bounce|fade|notch|none`) + `motion.speed` 0.5–2 (řídí i vysunutí šuplíku; starý
  `drawer.animationMs` se při načtení převede: `0` → `none`, jiná délka → rychlost), `strip.style` (`pill|line|dot|term|tag|glow|block|drop`),
  `strip.size` (`small|normal|large`), `strip.color` (prázdné = přechod akcentu, `accent`, `#rrggbb`),
  `strip.position` (`center|left|right` — proužek i šuplík u horní hrany, myš otevírá tam). Pořadí a velikost karet je v `layout`
  (dřívější `appearance.cards` se převede). Šířka šuplíku zůstává `drawer.width`
  (za běhu nejvýš 95 % monitoru), krytí proužku `strip.opacity`. Změna se projeví bez restartu.
  **Design styly (vlna 6):** `style` (`classic` výchozí = vzhled před vlnou 6, `glass`, `terminal`, `paper`,
  `neon`, `brutal`, `pastel`) je jen jméno — výběr stylu v Nastavení / paletě (`Styl: Neon`) zapíše jeho
  hodnoty do polí výš a níž, ta jdou dál přebít (krok Otevřený → „Pokročilé“, „Obnovit styl“ je vrátí). Nová pole:
  `font` (`system|mono|serif|grotesk`, jen systémová písma Windows), `material`
  (`solid|glass|paper|glow|hard|soft|frame`), `layout` (`classic|float|band|sheet|panorama|mosaic|tiles` —
  sloupce a mezery karet), `width` (`custom` = `drawer.width` | `narrow` 820 | `medium` 1080 | `wide` 1440 |
  `full` = celá šířka monitoru). Další motivy: `terminal`, `paper`, `neon`, `brutal`, `pastel`; animace navíc
  `bounce`, `fade`, `notch`; tvary proužku navíc `term`, `tag`, `glow`, `block`, `drop`. Systémové „omezit
  pohyb“ (Windows → Efekty animace) vypíná animace i vysunutí okna. Tabulka stylů: `src/theme/looks.ts`.
- **Zkratky** — globální zkratky jsou ve výchozím stavu **vypnuté** (nic se neregistruje, v UI se
  nikde neukazují); šuplík se otevírá myší, poznámka tlačítkem. Zapnout jde v Nastavení → Chování → Klávesové zkratky, nebo v configu:
  `hotkeys.enabled: true` + `hotkeys.toggle|approve|deny|note` (formát `Ctrl+Alt+Space`). Config bez
  `enabled` = vypnuto, i když má zkratky vypsané. Kolize zkratek se kontrolují jen při `enabled: true`.
  Když si Ledge vezme fokus (zkratka, klik do poznámky / palety / Nastavení), výslovné zavření (Esc, ✕,
  uložení poznámky, zkratka znovu) ho vrátí aplikaci, ve které jsi byl — jen pokud je Ledge pořád v
  popředí a to okno ještě existuje; spuštění akce nebo projektu fokus nechá novému oknu.
  `hotkeys.gameMode` (výchozí prázdné = bez zkratky) přepíná herní režim a platí i v něm.
  `hotkeys.notePassthrough` (výchozí `Ctrl+Alt+P`, prázdné = bez zkratky) přepne průchozí režim u všech
  lístků na ploše — **výjimka z `enabled`**: registruje se jen, když je na ploše aspoň jeden lístek, i se
  zkratkami vypnutými (na průchozí lístek nejde kliknout). Kolize se hlásí jen při `enabled: true`.
- **Paleta příkazů** — `palette.compact: true` (výchozí `false`; Nastavení → Chování →
  Kompaktní paleta příkazů): `Ctrl+K` / 🔍 ukáže jen vyhledávací pole, výsledky se rozbalí až při psaní,
  s prázdným polem šipka dolů ukáže nedávné. Chybějící sekce `palette` = dnešní chování.
  `palette.winSettings` (výchozí `true`; tamtéž → Nastavení Windows v paletě): při psaní paleta najde
  i stránky Nastavení Windows (`ms-settings:…`) a klasické ovládací panely (`control.exe /name
  Microsoft.…`) — česky i anglicky, bez diakritiky. Jen cíle z dokumentace Microsoftu
  (`src/palette/winsettings.ts`); jádro pustí jen tvar `ms-settings:<a-z0-9->` / `control:Microsoft.<Jméno>`.
  Paleta navíc bez configu počítá (`12*(3+4)`, `2^10`, `15 % z 340`, `sqrt(2)`), převádí jednotky
  (`10 km na mi`, `100 °F na °C`, `3 GB v MB`; měny ne) a s prefixem `:` hledá emoji (`:srdce`) —
  Enter výsledek zkopíruje do schránky. Jen UI (`src/palette/calc.ts`, `units.ts`, `emoji.ts`), žádná síť.
- **Proužek: kolik session čeká** — položka `strip.items` `"waiting"` („2 čekají“, jantarová pilulka):
  počet různých Claude session, které čekají na schválení nebo otázku; 0 = segment se neukáže. Nový
  config ji má za `claude`. Starý config (bez `strip.itemsRev`) ji při načtení dostane jednou za
  `claude` (bez `claude` ne) a `itemsRev: 1`; po uložení Nastavení odebraná položka zůstane odebraná.
  `strip.itemsRev` ručně neměň.
- **Upozornění Clauda** — při novém „čeká na tebe“ (schválení / otázka) nebo konci turnu (`Stop`)
  proužek krátce pulzne (jantarová / tyrkysová linka, ~1,2 s; styl pohybu `none` nebo vypnuté
  animace Windows = jen statické bliknutí linky). `claude.toast: true` (výchozí `false`) navíc pošle
  oznámení Windows: bez fokusu, nikdy ve fullscreen hře, v herním režimu, při „nerušit“ (QUNS) ani při
  otevřeném šuplíku; stejná session + událost nejvýš 1× za 30 s. Toast potřebuje nainstalovaný Ledge
  (AppUserModelID `cz.hyan.ledge` ze zástupce ve Startu), jinak ho Windows zahodí (chyba v logu).
- **Herní režim** — Ledge „mrtvý“ jako při fullscreen hře (šuplík zavřený, proužek pryč, myš, zkratky,
  peek ani připomínky nic). Zapíná se v Nastavení → Chování → Hry a celá obrazovka, v paletě Ctrl+K („Zapnout herní
  režim“) nebo zkratkou `hotkeys.gameMode`; stav je v `%APPDATA%\Ledge\game-mode.json` (`{"on": true}`,
  přežije restart; smazání souboru = vypnuto po dalším startu). Vypnutí: znovu spustit Ledge
  (zástupce / Start), hlavní zkratka `hotkeys.toggle` (vypne a otevře šuplík) nebo `hotkeys.gameMode`.
  Ve skutečné fullscreen hře tyhle cesty herní režim jen vypnou, šuplík se neotevře.
- **Přihrádka na soubory** — `shelf.enabled` (výchozí `true`; Nastavení → Šuplík a karty → Přihrádka
  na soubory). Soubory přetažené z Průzkumníka na proužek nahoře se uloží jako odkazy (jen cesty, nic se
  nekopíruje) do `%APPDATA%\Ledge\shelf.json` (`{"items":[{"path":"C:\\Users\\…","addedMs":…}]}`, nejnovější
  nahoře, nejvýš 50, stejná cesta bez ohledu na velikost písmen jen jednou). Karta „Přihrádka“: Cesta /
  @ (zmínka pro Claude Code `@C:\…`, s mezerou `@"C:\… …"`) do schránky, Otevřít, Ve složce, ✕, Kopírovat
  vše, Vyprázdnit, tah řádku ven (Průzkumník, terminál). Chybějící soubor = „chybí“, sám se nemaže.
  Nic se neposílá do session samo. Ve fullscreen hře / herním režimu nic. Mapa `docs/codemap/shelf.md`.
- **Historie schránky** — sekce `clipboard` (Nastavení → Osobní → Historie schránky; zapnout
  jde i přímo z palety): `enabled` (výchozí **`false`** — nic se neposlouchá ani neukládá), `max` (1–200,
  výchozí 25 nepřipnutých), `skipSecrets` (výchozí `true`: neukládat tvary `sk-…`, `ghp_…`/`github_pat_…`,
  `glpat-…`, `AKIA…`, `AIza…`, `xox?-…`, `sk_live_…`, JWT, PEM soukromý klíč), `excludeApps` (exe, ze kterých
  se nic neukládá; výchozí `KeePass.exe`, `KeePassXC.exe`, `1Password.exe`, `Bitwarden.exe`),
  `clearOnLock` (výchozí `true`: Win+L smaže nepřipnuté), `savePins` (výchozí `true`: připnuté do
  `%LOCALAPPDATA%\Ledge\clip-pins.json` = `{"pins":[{"text":"…","atMs":…}]}`; `false` soubor smaže).
  Jen text (`CF_UNICODETEXT`), jen paměť, nic přes síť. Nikdy se neuloží obsah se značkou
  `ExcludeClipboardContentFromMonitorProcessing` / `Clipboard Viewer Ignore` / `CanIncludeInClipboardHistory=0`
  / `CanUploadToCloudClipboard=0` (správci hesel, hesla z Chromu/Edge), prázdný, jen bílé znaky, delší než
  64 KB (neořezává se). Paleta: `v:` (+ hledání), Enter = zpět do schránky (nový záznam nevznikne),
  Ctrl+P připnout, Ctrl+Delete smazat, řádek „Vymazat historii“. Mapa `docs/codemap/clipboard.md`.
- **Přehrávač (Právě hraje)** — sekce `media` (Nastavení → Osobní → Přehrávač, nebo ikonka v rohu karty):
  `variant` = vzhled karty: `compact` (kompaktní řádek), `card` (výchozí, karta s průběhem), `cover` (velký
  rozmazaný obal), `vinyl` (deska se točí jen při přehrávání), `strip` (jen v proužku — v šuplíku tenký ovladač,
  proužek ukáže skladbu i bez položky `media` v `strip.items`); `stripTime` = čas skladby v proužku po minutách
  („1/4 min“): `auto` (výchozí, jen u `strip`), `on`, `off`; `equalizer` (výchozí `true`, jen u `strip`) =
  ozdoba ekvalizéru — jen dekorace, neukazuje skutečný zvuk. Starý config bez `media` = `card`, beze změny.
- **Skripty** — sekce `scripts` (Nastavení → Projekty, akce a odkazy → Skripty): `enabled` (výchozí `true`, ale nic
  se neukáže, dokud složka neexistuje), `dir` (chybí = `%USERPROFILE%\Ledge\scripts`, `%PROMĚNNÉ%` se
  rozbalí, jinak absolutní cesta), `inActions` (výchozí `false`; `true` = i dlaždice v kartě Rychlé
  akce). Formát skriptů viz „Skripty (Script Commands)“ níže.
- **Widgety ze skriptů** — sekce `widgets` (Nastavení → Projekty, akce a odkazy → Widgety ze skriptů):
  `enabled` (výchozí `true`, ale nic neběží, dokud widget nepřidáš na záložku / do proužku a nepovolíš ho),
  `dir` (chybí = `%USERPROFILE%\Ledge\widgets`, `%PROMĚNNÉ%` se rozbalí, jinak absolutní cesta). Starý
  config bez `widgets` = výchozí. Formát viz „Widgety ze skriptů“ níže a `docs/widgets.md`.
- **Claude (MCP)** — sekce `mcp` (Nastavení → Claude → Claude (MCP), přepínač platí hned): `enabled` (výchozí
  **`false`** → `/mcp` vrací 404). Zapnutí vytvoří klíč (32 B) ve Správci pověření Windows `Ledge/mcp` — klíč
  nikdy není v configu ani v logu. Starý config bez `mcp` = vypnuto. Co MCP umí: „MCP server (`/mcp`)“ níže.

## Právě hraje, hlasitost, baterie (záložka Osobní)

- **Právě hraje** (`drawer.sections.nowPlaying`) — aktuální mediální session Windows (GSMTC: Spotify,
  prohlížeč s YouTube, Media Player, VLC…): název, interpret, aplikace (z AUMID, `media::app_name`), hraje/pauza,
  tlačítka předchozí / přehrát-pauza / další (jen na klik), malý obal (WIC ≤ 128 px, načte se až s otevřeným
  šuplíkem, jednou na skladbu). Nic nehraje + hlasitost skrytá = karta zmizí; s hlasitostí řekne „Nic nehraje“.
  **Průběh skladby** z timeline GSMTC (`mediatime.rs`): lišta s časem, přetáčení klikem / tahem / šipkami jen když
  to přehrávač dovolí (`IsPlaybackPositionEnabled`), jinak lišta jen ukazuje; zdroj bez timeline (délka 0 —
  některé prohlížeče) = lišta se neukáže. **Náhodně / Opakovat** jen u zdroje, který je podporuje. Pět vzhledů
  karty — config `media` (viz Konfigurace).
- **Hlasitost** (`drawer.sections.sound`) — výchozí výstup (Core Audio): posuvník 0–100 (při tahu nejvýš
  ~20 změn/s), ztlumit, název zařízení v tooltipu; změna klávesou / mixérem se ukáže hned (callback).
- **Baterie** (`drawer.sections.battery`) — pod hodinami v kartě Čas: procenta, nabíjí se / v síti / na baterii,
  zbývající čas jen když ho Windows odhadne. Stroj bez baterie (desktop) neukazuje nic a nic neodebírá.
- **Výkon:** nic nepolluje. Odběry vzniknou až při prvním zobrazení záložky Osobní (Tauri `personal_live`;
  GSMTC navíc hned při startu / uložení configu, když je v zapnutém proužku položka `media`):
  vlákna `media` a `volume` pak jen čekají na události (timeline jen jako událost — hlášení, které jen potvrzuje
  dopočítaný průběh, se zahodí; průběh dopočítává UI a tiká jen s otevřeným šuplíkem při přehrávání), baterie je
  callback `PowerSettingRegisterNotification` bez vlákna a okna. Skrytá karta / vypnutá záložka Osobní odběry
  zruší (GSMTC jen tehdy, když `media` není ani v proužku). Tauri: `personal_live`, `media_control` (`"toggle"|"next"|"prev"`, `{"seek": ms}`, `{"shuffle": bool}`, `{"repeat": "none"|"track"|"list"}`), `media_cover`, `volume_set`, `volume_mute`.
- Proužek: položka `strip.items` `media` (výchozí vypnutá; vzhled `strip` ji doplní sám) — `mediaconf::strip_segment`
  (jen při přehrávání, „♪ Název · Interpret“, text ≤ 36 znaků, s `media.stripTime` navíc „· 1/4 min“, u `strip`
  s ozdobou ▂▅▃ místo ♪). Změna skladby / pauza → událost GSMTC → `mark_dirty` → překreslení; čas jen po minutách
  (≤ 1 probuzení vlákna `emit` za minutu při přehrávání), sekundy proužek neukazuje. Ve fullscreen hře / herním režimu je proužek pryč jako vždy.

## GitHub a Systém (karty `github`, `system`; Nastavení → Osobní)

Dvě karty z katalogu (✎ → + Přidat kartu), **výchozí na žádné záložce**. Starý config se nemění.

- **GitHub** — sekce `github`: `enabled` (výchozí `false`!), `review` / `mine` / `notifications` (výchozí `true`: co
  ukázat), `refreshMin` (výchozí `5`, 2–60). Zdroj je přihlášené **`gh`** (GitHub CLI; hledá se v `PATH`, pak
  `%ProgramFiles%GitHub CLI`, `%LOCALAPPDATA%ProgramsGitHub CLI` a `…MicrosoftWinGetLinks`) — Ledge nemá vlastní
  OAuth, žádný token nevidí ani neukládá. Tři dotazy souběžně, bez okna, termín 15 s každý: `gh search prs
  --review-requested=@me` (čeká na tvoji revizi), `gh api graphql` `viewer.pullRequests` (tvoje otevřená PR se stavem
  CI z `statusCheckRollup`) a `gh api notifications` (počet + 5 nejnovějších, nad 50 „50+“). Klik na položku ji otevře
  v prohlížeči (jen `https://github.com/…`). Obnova jen když je karta vidět (otevřený šuplík) nebo je zapnutá položka
  proužku `github`, vždy po `refreshMin` (tlačítko Obnovit nejdřív po 10 s); data jsou jen v paměti. Prázdné stavy:
  vypnuto („Zapnout GitHub“ jedním klikem s větou, co se čte), `gh` chybí, nepřihlášeno („gh auth login“ + Kopírovat),
  offline (zůstanou poslední data + chyba). Tauri: `github_view(force)`, `github_open(url)`, `github_copy_login`.
- **Systém** — sekce `system`: `disks` (písmena disků, prázdné = všechny pevné), `netUnit` (`bytes` = MB/s, `bits` =
  Mbit/s). CPU % z dvou vzorků `GetSystemTimes`, paměť `GlobalMemoryStatusEx`, volné místo pevných disků
  (`GetDiskFreeSpaceExW`, čte se nejvýš 1× za 30 s), síť `GetIfTable2` (jen fyzické adaptéry nahoře). Žádné WMI, žádný
  proces. Vzorkuje se **jen** s viditelnou kartou (1× za 2 s) nebo se zapnutou položkou proužku `system` (1× za 10 s);
  sparkline = posledních 60 vzorků karty v paměti. Klik na CPU / paměť otevře Správce úloh. Teplota a GPU záměrně
  ne (nespolehlivé). Tauri: `system_view`, `system_taskmgr`.
- **Proužek:** položky `github` a `system` (výchozí vypnuté) drží plánovač `integrations.rs`; bez nich žádné vlákno
  neběží. Ve fullscreen hře (Ledge „mrtvý“) se nevzorkuje ani nevolá `gh`.

## Rychlé akce (`actions`)

Dlaždice na jedno kliknutí (karta „Rychlé akce“ na Práci i Osobní) a položky palety příkazů
(`Ctrl+K` v šuplíku s fokusem, 🔍 v hlavičce). Pořadí v poli = pořadí dlaždic. Mapa kódu:
`docs/codemap/actions.md`.

```json
"actions": [
  { "id": "mapy", "label": "Mapy", "kind": "url", "target": "https://mapy.cz", "showOn": "personal" },
  { "id": "faktury", "label": "Faktury", "kind": "folder", "target": "C:\\Users\\x\\Documents\\Faktury" },
  { "id": "build", "label": "Build webu", "kind": "command", "target": "npm run build",
    "cwd": "C:\\Users\\x\\Desktop\\web", "group": "Dev", "showOn": "work", "confirm": true },
  { "id": "yt-music", "label": "YouTube Music", "kind": "web", "target": "https://music.youtube.com",
    "width": 420, "height": 640 }
]
```

- `kind`: `url` (jen http/https) · `file` (dokumenty/média — allowlist jako odkazy Osobní; skripty ne)
  · `folder` · `app` (`.exe` absolutně nebo jen jméno ze složky Windows; `args` = s argumenty) ·
  `command` (`cmd /c`, nebo `"shell": "powershell"` = `-NoProfile`; `"terminal": true` = viditelné
  okno, jinak bez okna s termínem `timeoutS` (výchozí 60 s) a výstupem do hlášky / karty) ·
  `web` (http/https ve vlastním malém okně Ledge, viz níže).
- Volitelné: `args` (pole, bez uvozovek), `cwd` (absolutní; výchozí domovská složka), `icon`
  (chybí/`"auto"` = favicon webu nebo ikona souboru, `"monogram"`, nebo cesta k png/ico/jpg/svg),
  `group`, `confirm` (ptát se před každým spuštěním), `showOn` (`work`/`personal`/`both`), `look`
  (vlastní vzhled dlaždice — stejný tvar jako u projektů výš; přebije `icon`).
  Výchozí hodnoty se do souboru nepíšou. `id` = a–z, 0–9, `-`, `_`, jednoznačné; nejvýš 64 akcí.
- **Agent smí akce přidávat a měnit** (web, složka, soubor, aplikace bez argumentů se spouští hned).
  **Příkaz a aplikace s argumenty se bez souhlasu člověka vždy ptají** — souhlas uděluje jen člověk
  zaškrtnutím v Nastavení a ukládá se mimo config (`%LOCALAPPDATA%\Ledge\actions-consent.json`, otisk
  přesného příkazu). Ten soubor nikdy nezapisuj; změna cíle/argumentů/složky souhlas zneplatní.
- Favicon se stáhne jednou (při přidání akce, `%LOCALAPPDATA%\Ledge\icons\<host>.png`, neúspěch
  `<host>.miss`); znovu jen tlačítkem „Obnovit“ v Nastavení. Nic se nestahuje periodicky.
- **Okno s webem** (`kind: "web"`, Nastavení → Projekty, akce a odkazy → typ „Okno s webem“): `target` = http/https
  adresa (ne `*.localhost` — tak Tauri podává UI Ledge —, bez jména/hesla v adrese), volitelně `width`
  (320–3840) a `height` (240–2160) v logických px, výchozí 480 × 720; titulek okna = `label`. Klik otevře
  normální okno s rámem pod proužkem u kurzoru (fokus dostane — je to výslovný klik); druhý klik okno jen
  přenese dopředu (nezavře — rozehraná hudba / rozepsaný mail). Křížek okno **zničí** (žádné skrývání,
  žádný časovač; zavřené okno nic nestojí). Okno sdílí se šuplíkem proces prohlížeče WebView2 i data dir
  (`%LOCALAPPDATA%\cz.hyan.ledge\EBWebView`), takže přihlášení přežije zavření. Stránka nemá žádné IPC
  (okno `web-<id>` není v capabilities, vzdálený původ Tauri odmítne), navigace mimo http/https se zahodí,
  `window.open` / odkazy do nového okna jdou do výchozího prohlížeče. Poloha a velikost per akce v
  `%LOCALAPPDATA%\Ledge\web-panels.json` (zapíše se při zavření; změna `width`/`height` ji zahodí). Ve
  fullscreen hře / herním režimu se nic neotevře. Ikona `auto` = favicon webu jako u `url`.
- **Odkazy = akce** (od vlny 6): odkaz je akce, která jen něco otevře — `kind` `url`, `web`, `folder`
  nebo `file`. Karta „Odkazy“ ukazuje právě tyhle akce (viditelné na své záložce podle `showOn`, v pořadí
  pole `actions`, ikona = favicon / ikona souboru); karta „Rychlé akce“ je na záložce s kartou Odkazy
  vynechá (žádné dvojí dlaždice). Nový odkaz = nová akce (Nastavení → Projekty, akce a odkazy → „Přidat
  akci nebo odkaz“; průvodce zapisuje odkazy taky do `actions`).
- **Starší `personal.links`** (`[{ "name", "target" }]`) se při každém načtení configu (a při uložení
  z Nastavení, třeba po importu staré zálohy) převedou na akce (`src-tauri/src/linkmigrate.rs`):
  `id` z názvu (bez diakritiky, jednoznačné), `label` = `name`, `kind` podle cíle bez disku (http(s) →
  `url`, `.exe` → `app`, dokument z allowlistu → `file`, jinak `folder`), `showOn: "personal"`. Cíl, který
  už má akce viditelná na Osobní, se jen odebere. Převod je idempotentní a beze ztráty: odkaz, který by
  jako akce neprošel validací (relativní cesta, plný seznam 64 akcí), **zůstane** v `personal.links`
  (karta Odkazy ho ukáže pod akcemi, Nastavení ho nabídne k opravě). Na disk se nový tvar zapíše až
  příštím uložením. Nové odkazy do `personal.links` nepiš.

### Skripty (Script Commands)

Soubory `.ps1`, `.cmd`/`.bat`, `.py`, `.js`/`.mjs` přímo ve složce skriptů (bez podsložek, ne skryté
`.x`) s hlavičkou v komentáři se objeví v paletě (`Ctrl+K`) jako „Skript“. Soubor bez jediného řádku
`@ledge.` je pomocný modul a nezobrazí se. Příklad (`vycistit-temp.ps1`):

```powershell
# @ledge.title Vyčistit temp
# @ledge.description Smaže dočasné soubory starší než týden
# @ledge.icon 🧹
# @ledge.mode output
# @ledge.args -Days 7
param([int]$Days = 7)
Get-ChildItem $env:TEMP | Where-Object LastWriteTime -lt (Get-Date).AddDays(-$Days) | Remove-Item -Recurse -ErrorAction SilentlyContinue
```

- Komentář: `#` (`.ps1`, `.py`; i uvnitř `<# … #>` / docstringu), `::` nebo `REM` (`.cmd`/`.bat` — jinak
  by se řádek provedl), `//` (`.js`/`.mjs`; i uvnitř `/* … */`). Čte se jen prvních 4 KB; u opakovaného
  klíče platí první, neznámé klíče se přeskočí.
- Klíče: `title` (**povinný**, ≤ 80 znaků) · `description` (podtitulek) · `icon` (emoji / ≤ 8 znaků místo
  monogramu) · `mode`: `output` (výchozí — bez okna, výstup v šuplíku jako u skryté akce), `hidden`
  (bez okna, jen hláška hotovo / chyba), `terminal` (viditelné okno konzole, zůstane otevřené) ·
  `confirm` (bez hodnoty / `true` = ptát se před každým spuštěním) · `args` (pevné argumenty, mezery
  v `"…"`, bez uvozovek uvnitř; ≤ 32) · `group` (skupina v kartě, výchozí „Skripty“) · `timeout` (s bez
  okna, 1–3600, výchozí 60 — pak se strom procesu zabije). Chybná hlavička = skript se ukáže
  s problémem a nespustí se.
- Spuštění (pracovní složka = složka skriptů): `.ps1` → `powershell -NoProfile -ExecutionPolicy
  Bypass -Command "& 'cesta' 'arg'"` (politika jen pro ten proces; skript s diakritikou ulož jako
  UTF-8 **s BOM**, Windows PowerShell 5.1 jinak čte ANSI) · `.cmd`/`.bat` → `cmd /d /s /c` · `.py` →
  `py.exe` / `python.exe` z PATH · `.js`/`.mjs` → `node.exe` z PATH. Aliasy Microsoft Storu
  (`…\Microsoft\WindowsApps`) se nepočítají; chybí-li interpret, skript se ukáže s problémem.
- **Souhlas**: skript je cizí kód — první spuštění se vždy potvrzuje v šuplíku (cesta + začátek
  SHA-256 obsahu). Potvrzení se uloží jako otisk cesty, obsahu, režimu a argumentů do
  `%LOCALAPPDATA%\Ledge\actions-consent.json` (stejný soubor jako souhlasy akcí); **jakákoli změna
  souboru = nové potvrzení**. Soubor se při spuštění čte a hashuje znovu (ne z cache). Agent smí
  skripty do složky psát, ale spustit je bez potvrzení člověka nejde; soubor souhlasů nikdy nezapisuj.
- Načítání bez watcheru: při otevření palety (a šuplíku, je-li `inActions`) jeden výpis složky,
  hlavička + hash jen u souborů se změněným mtime / velikostí. Nejvýš 200 skriptů (podle jména),
  soubor nad 1 MB se nespustí.

### Widgety ze skriptů

Jako xbar / SwiftBar / Übersicht: soubor ve složce widgetů (`.ps1`, `.cmd`/`.bat`, `.py`, `.js`/`.mjs`,
`.exe`) s hlavičkou `@ledge.widget` vypíše na stdout data → Ledge z nich nakreslí **kartu** (`widget:<soubor>`
v `layout`) a/nebo **položku proužku** (`widget:<soubor>` v `strip.items`). Návod pro lidi: `docs/widgets.md`,
mapa kódu `docs/codemap/widgets.md`.

```powershell
# @ledge.widget Docker
# @ledge.icon 🐳
# @ledge.refresh 1m
# @ledge.show both
# @ledge.timeout 15
@{ title = 'Docker'; strip = '🐳 2 běží'; items = @(@{ label = 'web'; value = 'Up 2h'; color = 'green'; copy = 'a1b2' }) } | ConvertTo-Json -Depth 5 -Compress
```

- Hlavička (komentáře jako u skriptů, prvních 4 KB): `widget` (**název**, povinný; prázdný = `title`) ·
  `icon` · `description` · `refresh` (`30s`, `5m`, `1h`, `1h30m`; výchozí `5m`, kratší než 30 s = 30 s,
  nejvýš `24h`) · `show` (`card` výchozí, `strip`, `both` — kam ho katalog / proužek nabídne) · `timeout`
  (1–120 s, výchozí 10) · `args`. Soubor bez `@ledge.widget` není widget. **`.exe`** má hlavičku vedle
  sebe v `<jméno>.exe.ledge` (holé řádky `@ledge.…`); otisk = program + hlavička.
- **Výstup JSON** (doporučený): `title`, `text` (víc řádků), `badge`, `progress` (0–1, nebo `"42%"`),
  `strip` (≤ 24 znaků; chybí = `badge`, pak první řádek `text`), `color`, `items[]` = `{label, value, icon,
  color, url | action}` (nejvýš 50); `action` = `{"type":"open","path":"C:\\…"}` (složka / dokument z
  allowlistu odkazů, žádné programy) · `{"type":"copy","text":"…"}` · `{"type":"run","script":"x.ps1","arg":"…"}`
  (skript ze složky **skriptů**, který člověk už povolil z palety, bez `@ledge.confirm`; `arg` se přidá za jeho
  `@ledge.args`). Zkratky na položce: `copy`, `open`, `run` + `arg`. `url` jen http(s). Barvy = předvolby
  `accent text muted violet blue teal green amber red` (+ aliasy `ok`, `warn`, `error`, `gray`… a `#rrggbb`
  podle odstínu). Pole položek, samotný text i rozbitý JSON projdou (rozbitý = text s poznámkou).
- **Výstup text** (à la xbar): 1. řádek = proužek (`| color=…`), další řádky do `---` = text karty, za `---`
  položky `Popisek | href=… color=… copy=… open=… run=… arg=… icon=… value=…` (hodnoty s mezerou v uvozovkách),
  `---` = oddělovač, `--` na začátku (podmenu) se zploští. `bash=` / `shell=` / `terminal=` Ledge **nikdy
  nespouští** (karta to řekne).
- **Běh**: jen když je karta na otevřené záložce (šuplík otevřený) nebo widget v zapnutém proužku a Ledge není
  mrtvý (fullscreen hra / herní režim); jinak vlákno `widgets` spí bez časovače. Další běh = konec + interval +
  jitter (≤ 10 % intervalu, nejvýš 15 s); nikdy dvakrát souběžně, nejvýš 3 widgety naráz. Bez okna
  (`CREATE_NO_WINDOW`), stdin prázdný, pracovní složka = složka widgetu, termín → zabití celého stromu (Job
  Object), stdout nad 256 KB = chyba, žádné zvýšení práv. Interprety jako u skriptů (PowerShell s UTF-8
  výstupem; `.ps1` s diakritikou = UTF-8 s BOM). Data jen v paměti, nic se nikam neposílá.
- **Souhlas**: nový / změněný soubor se **nespustí** — karta (nebo Nastavení) ukáže začátek skriptu a otisk
  (cesta + začátek SHA-256) a čeká na klik „Povolit a spustit“. Otisk (cesta, obsah, druh, argumenty) se
  ukládá mimo config do `%LOCALAPPDATA%\Ledge\widgets-consent.json` (jeden na soubor); před každým během se
  soubor čte a hashuje znovu. Agent smí widgety do složky psát (i z galerie), povolit je jen člověk; soubor
  souhlasů nikdy nezapisuj.
- **Galerie** (Nastavení → Widgety → Galerie příkladů, zakompilované `src-tauri/src/widgets/templates/*`):
  kurz ČNB (ps1 + py, síť www.cnb.cz), počasí jinde přes Open-Meteo (ps1 + py), Docker, GitHub CI přes `gh`,
  ping (ps1 + py), síť a VPN, místo na disku (ps1 + py), uptime (ps1 + py), odpočet (ps1 + py, py = textový
  formát). Kopie do složky nikdy nepřepíše existující soubor a sama se nespustí.
- Tauri: `widgets_list`, `widget_consent(id, confirmed)`, `widget_refresh`, `widget_output`, `widget_edit`
  (sloveso `edit`, jinak Poznámkový blok — nikdy „otevřít“, to by `.py` spustilo), `widgets_open_dir`,
  `widget_templates`, `widget_install(file)`, `widget_click(id, index, label)`.

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
- Hotovou poznámku označ `done: true`. **Nemaž** poznámky ani obrázky — mazání je jen na uživateli.

### Lístky na plochu (overlay okna)

Kteroukoli poznámku jde **připnout na plochu** jako malé okno bez rámu přes ostatní okna (špendlík u
řádku v kartě Poznámky / v editoru, ⋯ karty → „Nový lístek na plochu“, Ctrl+K → „Nový lístek na plochu“ /
„Připnout na plochu: …“). Lístek je **ta samá poznámka** — čte a ukládá se stejně (otisk, konflikt →
načíst znovu / kopie), úkoly se odklikávají, „+ bod“ přidá `- [ ] …`, Ctrl+V vloží snímek obrazovky
(PNG vedle poznámky jako v editoru) nebo text na konec. Agent na lístky nesahá — stačí upravit `.md`,
otevřený lístek se sám obnoví.

- Okna `note-<klíč>` (`pins/win.rs`): bez fokusu při otevření, mimo hlavní panel, volby per lístek:
  vždy navrchu / normálně, zamknout na místě, průchozí (myš jde na stránku pod ním; vypne zkratka
  `hotkeys.notePassthrough` nebo Ctrl+K „Lístky: vypnout průchozí režim“), průhlednost pozadí a textu
  (text nejníž 50 %), sbalení na lištu (dvojklik). Ve fullscreenu / herním režimu lístky přestanou být
  navrchu a nové se neotevřou.
- Stav (pozice ve fyzických px, velikost v logických, volby) je v `%LOCALAPPDATA%\Ledge\note-pins.json`
  (zápis se zpožděním ~0,4 s, ne v configu — je to stav okna, ne nastavení). Po startu Ledge se připnuté
  lístky vrátí na místo; mimo obrazovku → monitor proužku. ✕ = odepnout, poznámka zůstane.
  Uživatelský návod: [`docs/listky.md`](docs/listky.md).

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
- `.draft\`, `.state.json`, `*.tmp`, `PRESUNUTO-DO-LEDGE.txt` jsou v `.gitignore` — patří jen tomuto PC.
- Konflikt (stejná poznámka změněná na dvou PC) → rebase se přeruší, lokální verze zůstane
  v lokálním commitu, štítek v kartě ukáže „konflikt: …“. Vyřešit ručně, pak se sync rozjede sám.
- **Claude poznámky smí vytvářet a upravovat (`done`, `kind`), Ledge je sám commitne a pushne.** Claude
  v kořeni poznámek **necommituje, nepushuje, nepulluje ani nerebasuje** sám — sahal by do běžícího cyklu.

## Lokální API (jen 127.0.0.1:47800)

| volání | co dělá |
|---|---|
| `GET /status` | čekající schválení, sessions, běžící procesy, cesta ke configu |
| `POST /ctl/open` `{"view":"full"\|"peek"\|"compact"\|"island"}` | otevře šuplík / peek / náhled po najetí (bez fokusu, nezavírá se sám) |
| `POST /ctl/close` | zavře šuplík |
| `POST /ctl/settings` | otevře Nastavení |
| `POST /ctl/launch` `{"id":"atlas"}` | spustí projekt jako klik na logo |
| `POST /ctl/stop` `{"id":"atlas"}` | zastaví projekt (celý strom procesů) |
| `POST /ctl/reload` | znovu načte config |
| `POST /permission` | **jen pro hook `PermissionRequest`** |
| `POST /question` | **jen pro hook `PreToolUse` s matcherem `AskUserQuestion`** (otázky Clauda) |
| `POST /plan` | **jen pro hook `PreToolUse` s matcherem `ExitPlanMode`** (review plánu) |
| `POST /event` | hooky SessionStart/UserPromptSubmit/PostToolUse/Stop/SessionEnd/Notification/SubagentStart/SubagentStop |
| `POST /mcp` | MCP server pro Clauda (JSON-RPC, `Authorization: Bearer <klíč>`; vypnuto = 404) — viz níže |

Požadavky s hlavičkou `Origin` (prohlížeč) se odmítají.

### MCP server (`/mcp`)

Claude Code / Claude Desktop nastaví Ledge podle toho, co uživatel používá. Zapíná se v Nastavení → Claude →
Claude (MCP) (výchozí vypnuto). Mapa: `docs/codemap/mcp.md`.

- **Připojení (doporučené, stdio):** `claude mcp add --scope user ledge -- "<cesta k ledge.exe>" --mcp`
  (přesný příkaz zkopíruje Nastavení). `ledge.exe --mcp` je tenký proxy stdin/stdout ↔ `POST /mcp`, klíč si
  čte sám ze Správce pověření; neotevírá okno a nestartuje druhý Ledge. Ledge neběží → chyba JSON-RPC „Ledge
  neběží“, MCP vypnuté → „Zapni v Ledge Nastavení → Claude (MCP)“. Claude Desktop: `mcpServers.ledge =
  { "command": "<ledge.exe>", "args": ["--mcp"] }`.
- **Záloha (http):** `claude mcp add --transport http --scope user ledge http://127.0.0.1:47800/mcp --header
  "Authorization: Bearer <klíč>"` — klíč jen na „Zobrazit klíč“. Streamable HTTP bez SSE a bez session: `POST`
  = jedna zpráva (nebo dávka), odpověď `application/json`; notifikace → 202; `GET`/`DELETE` → 405; bez klíče /
  špatný klíč → 401 (bez detailu); tělo > 256 KB → 413. Protokol 2025-06-18, 2025-03-26, 2024-11-05.
- **Nástroje:** `ledge_status`, `apps_discover` (Start menu uživatele i ProgramData, repa na disku; nic z
  prohlížeče), `tiles_list` / `tiles_add` / `tiles_update` / `tiles_remove` (zástupci = `projects`),
  `actions_list` / `actions_add` / `actions_update` / `actions_remove`, `layout_get` / `layout_preview` /
  `layout_apply`, `strip_get` / `strip_set`, `appearance_get` / `appearance_set` (styl jen z katalogu stylů,
  hodnoty jen z výčtů schématu), `undo_last`, `open_drawer`. Každý má JSON Schema vstupu a popis pro model.
- **Zápis = stejná cesta jako Uložit v Nastavení:** načíst config → změna → `config::validate_field` (stejná
  validace, i editoru rozložení) → snímek dotčených částí do Vrátit (posledních 20, jen v paměti) →
  `save_config` → událost `config-changed`. Neplatný vstup = výsledek s `isError: true` a větou, co je špatně a
  co jde (config beze změny); chyba protokolu (neznámý nástroj, špatná zpráva) = JSON-RPC error. `undo_last`
  vrátí jen části, které ten zápis změnil, a odmítne, když je mezitím někdo změnil jinde.
- **Neuložené úpravy v Nastavení:** zápis přes MCP projde vždy. Otevřené Nastavení dostane událost: bez
  rozpracovaných změn se přenačte; rozpracované v jiné sekci zůstanou a změna zvenku se do nich sloučí; tatáž
  sekce = lišta konfliktu „Načíst nové / Přesto uložit“ (nic se nepřepíše potichu, `src/mcp/merge.ts`).
- **MCP nic nespouští** (žádné launch/run nástroje). Akce `command` / `app` s argumenty přidaná přes MCP souhlas
  nedostane — Ledge se zeptá při prvním spuštění jako u ruční. Zástupce (karta Projekty) se spouští bez
  potvrzení, proto mu MCP smí dát jen skript balíčkovače (`npm run dev`, `pnpm dev`…) nebo příkaz, který Ledge
  sám najde ve složce projektu; jiný příkaz → rychlá akce `command`.

**Schválení přes API záměrně NEJDE.** Povolit akci Clauda smí jen člověk — klikem v šuplíku (nebo
zkratkou, je-li zapnutá). Nepřidávej `/ctl/decide` ani nic podobného: z mostu by se stal způsob, jak si agent
schválí sám sebe.

## Session a hooky

**Karta Claude ukazuje všechny běžící Claude Code session na PC**, i bez hooků:

- Hlavní zdroj = `claude agents --json --all` (dokumentované). Spouští se **jen při otevřeném šuplíku**
  (otevření, změna registru, hook neznámé session), bez okna, termín 10 s, **nejvýš 1× za 5 s**
  (`src-tauri/src/sessionlive.rs`). Nikdy periodicky.
- Registr `~/.claude/sessions/<pid>.json` (nedokumentovaný) je jen spoušť (watcher, debounce 2 s) a
  obohacení (`entrypoint`, čerstvější `status`). Soubory `<pid>.<hash>.key` Ledge **nečte**. Když CLI
  selže, seznam se vezme z registru (jen živé PID).
- Hooky Ledge přidávají: čeká na schválení, hlídač nedokončené práce (●/↑) a **subagenty**
  (`SubagentStart`/`SubagentStop` → běžící: typ + od kdy; posledních 5 dokončených s trváním a
  zkrácenou poslední zprávou). `GET /status` je vrací v `sessions[].subagents`.
- Stav: `waiting` (čeká na tebe) > `working` > `idle` > `ended` (jen se skončenou nedokončenou prací).
  Logika sloučení je čistá v `sessions.rs` (`merge`, `live_status`) s tabulkovými testy.

**Klik na session** (`session_open(id)`, rozhodnutí `sessions::decide_open`, Win32 `winfocus.rs`):

| session | co se stane |
|---|---|
| běží v desktopové appce Claude | přepne do okna appky (konkrétní záložku vybrat neumíme — deep link není dokumentovaný) |
| běží v CLI | přepne do okna předka s viditelným oknem (Windows Terminal / konzole / VS Code) |
| skončila | `wt -d <cwd> cmd /c claude --resume <id>`, bez `wt` přes `cmd /c start` |
| běží na pozadí (`kind: background`) | nic — jen tlačítko **Připojit** pod řádkem (níže) |
| neznámý proces | nic, UI řekne proč |

Běžící session se **nikdy** neresumuje (prokládání zpráv). Okno se přepíná jen na výslovný klik.
Podtitul a jméno session jdou přes `sessions::clean_preview`: bloky `<system-reminder>`, `<command-*>`,
`<local-command-*>`, `<task-notification>`… (i escapované) se zahodí i s obsahem; nic nezbude = bez textu.

**Agent View — session na pozadí** (`src-tauri/src/agentview.rs` čistá logika, `agentrun.rs` I/O,
`commands/agents.rs`; UI `src/sessions/AgentActions.tsx`, `BgLaunch.tsx`, `agentview.ts`). Z `claude agents --json`
Ledge bere navíc `kind`, `id` (krátké id úlohy), `state` (working/blocked/done/failed/stopped) a `waitingFor`;
neznámé hodnoty ignoruje. Karta ukáže pilulku „na pozadí“, „čeká: oprávnění / vstup / sandbox / worker / dialog“
a u skončených úloh „selhala / zastavena / hotovo“ (skončená úloha na pozadí zůstává v kartě, dokud ji někdo
neodebere; `done` s živým procesem = nečinná). Session, která podle seznamu čeká, se počítá do segmentu proužku
„N čeká“ i bez hooků Ledge (stejné `sessionId` jako žádost z hooku = jedna); peek/`Claude::attention()` ji **ne**zapíná
(seznam se obnovuje jen při otevřeném šuplíku → peek by mohl viset na starých datech a nic v něm nejde vyřídit).

Akce pod session na pozadí (jen s platným id — písmena/číslice/`-`/`_`, 4–64 znaků, ne pomlčka na začátku; jen klikem):

| tlačítko | příkaz | kdy | potvrzení |
|---|---|---|---|
| Připojit | `claude attach <id>` v novém okně (`wt -d <cwd> cmd /c …`, bez `wt` přes `cmd /c start`) | vždy | ne |
| Logy | `claude logs <id>` bez okna, termín 15 s, konec výstupu 32 KB bez ANSI do rozbalení | vždy | ne |
| Restartovat | `claude respawn <id>` | proces žije nebo `stopped` | ano |
| Zastavit | `claude stop <id>` | proces žije nebo `working`/`blocked` | ano |
| Odebrat ze seznamu | `claude rm <id>` (smaže i worktree session; přepis zůstane pro `claude --resume`) | `done`/`failed`/`stopped` | ano |

Rust znovu najde řádek a akci ověří proti `job.actions`; měnící akce bez `confirmed` odmítne. **Claude na pozadí…**
(tlačítko pod dlaždicí projektu, paleta „Claude na pozadí v …“): pole úkolu (fokus po kliku, Ctrl+Enter, Esc) +
nepovinné jméno → `claude --bg [--name N] "<úkol>"` ve složce projektu, termín 45 s. CLI se spouští **bez shellu**
(`claude.exe` z PATH, u npm instalace binárka za `claude.cmd`; jinak `claude.cmd` s escapováním Rustu), s
`CREATE_NO_WINDOW` a **bez** Job Objectu (ten by session na pozadí zabil). Poslat zprávu do běžící session z shellu
docs neuvádějí → Ledge to nedělá (odpověď = Připojit). Nic se nečte z `~/.claude/jobs` (není stabilní rozhraní).

**Panel Git session** (řádek „▸ Git“ pod session; `src/sessions/GitPanel.tsx`, Rust `src-tauri/src/sessiongit/`,
příkazy `session_git*` v `commands/sessiongit.rs`): pro cwd v git repu větev, upstream ↑/↓ (podle posledního
fetch), worktree + hlavní checkout, změněné soubory (strop 200) s diffem po kliku (`git diff` / `--cached`,
nesledovaný = začátek obsahu, nad 512 KB / 1 MB výstupu „příliš velké“). Stav se čte jen při rozbalení, po akci
a po ↻ — **žádné pollování**. Config nemá žádné nové pole.

| akce | git | pravidla |
|---|---|---|
| Commit… | `add` + `commit -m <zpráva>` jen vybraných cest (pathspecy na stdin, `--pathspec-file-nul`) | hooky gitu běží (nikdy `--no-verify`), chyba hooku se ukáže celá; jiné připravené soubory do commitu nejdou |
| Push… | `push` (s upstreamem), bez upstreamu a s jediným remote `push -u <remote> refs/heads/<b>:refs/heads/<b>` | potvrzení s cílem a počtem commitů; **force se nenabízí vůbec** |
| Pull | `pull --ff-only` | jen s upstreamem |
| Zahodit… | `restore --source=HEAD --staged --worktree -- <soubor>` | jen sledovaný soubor z HEAD; druhé potvrzení vypíše, co se ztratí |
| Odstranit worktree… | `worktree remove <cesta>` z hlavního checkoutu | bez `--force` (s rozdělanými soubory git odmítne a chyba se ukáže); zamčený = ne |
| Terminál / Složka | `open_dir` jako u projektů | kořen repa |

Destruktivní akce blokuje (Rust i UI) session, která v té složce **pracuje nebo čeká** (`working`/`waiting`);
u nečinné potvrzení odstranění upozorní, že Windows složku nemusí smazat. Git vždy bez okna, s termínem
(čtení 10 s, commit 5 min, síť 2 min), argumenty jako pole, `GIT_LITERAL_PATHSPECS=1`, `GIT_TERMINAL_PROMPT=0`,
`core.fsmonitor=false`; UI posílá jen id session a cestu, kterou Rust ověří proti čerstvému `git status`.

**Schvalování** (hook `PermissionRequest` → `POST /permission`, `src-tauri/src/claude/mod.rs` + `claude/rules.rs`
+ `claude/preview.rs`/`diff.rs` + `claude/plan.rs`, UI `PendingRow` / peek + `src/sessions/PermExtra.tsx`,
`DiffPreview.tsx`, `PlanReview.tsx`). Povolit jen klik člověka (nebo zkratka approve/deny).
Odpovědi (`hookSpecificOutput.decision`, tvar podle dokumentace hooků):

| klik | `decision` |
|---|---|
| Povolit (i zkratka) | `{"behavior":"allow"}` |
| **Vždy povolit** (jen klik) | `{"behavior":"allow","updatedPermissions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"npm run lint:*"}],"behavior":"allow","destination":"localSettings"}]}` |
| Zamítnout (i zkratka) | `{"behavior":"deny","message":"Zamítnuto v Ledge."}` |
| **Zamítnout s důvodem…** | `{"behavior":"deny","message":"Zamítnuto v Ledge: <důvod>"}` (≤ 500 znaků) |
| termín / vyřízeno jinde | prázdné tělo = zeptá se terminál |
| plán: **Schválit** (i zkratka approve) | `{"behavior":"allow"}` |
| plán: **Schválit a povolit úpravy** (jen klik) | `{"behavior":"allow","updatedPermissions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}]}` |
| plán: **Vrátit s připomínkou…** | `{"behavior":"deny","message":"Plán vrácen z Ledge s připomínkou: <text>"}` (≤ 500 znaků; prázdná = obyčejné zamítnutí) |

„Vždy povolit“: pravidlo zapíše **Claude Code sám**, Ledge settings.json needituje. Karta před klikem ukáže přesné
pravidlo a cíl: výchozí projekt lokálně (`localSettings` → `.claude/settings.local.json` v projektu), v rozbalení
uživatelsky (`userSettings` → `~/.claude/settings.json`) nebo jen tato session (`session`, nic se nezapíše).
Pravidlo = návrh Clauda (`permission_suggestions`, jen `addRules` s `behavior:"allow"` pro týž nástroj), bez návrhu
přesný jednoduchý příkaz Bash/PowerShell (bez `*` a operátorů shellu); jinak se tlačítko neukáže (Edit/Write bez
návrhu nikdy). Tauri `decide(id, allow, extra?)`, `extra = {always?: "localSettings"|"userSettings"|"session",
reason?: string, mode?: "acceptEdits"}` — bez `extra` jako dřív; pravidlo si Rust bere z čekající žádosti, z UI jde jen cíl. Návrh je i v
`GET /status` (`pending[].always`).

**Náhled změny** (Edit/Write): karta ukáže sbalené „+12 −3 · soubor“, klik rozbalí řádkový diff (kontext 3 řádky,
strop 400 řádků s hláškou). Rust ho sestaví **jednou při příchodu žádosti** (`preview::for_request`, ne ve snapshotu):
aktuální soubor čte max 512 KB s termínem 1,5 s; Edit = diff celého souboru před/po nahrazení `old_string` (čísla
řádků; `replace_all` = všechny výskyty), když `old_string` v souboru není nebo soubor nejde přečíst, jen řetězce bez
čísel; Write = diff proti aktuálnímu obsahu, neexistující soubor = „nový soubor, N řádků“ + prvních 40 řádků;
binární / ne-UTF-8 = „nelze zobrazit“, nad 512 KB = „příliš velké“. NotebookEdit náhled nemá. V `GET /status` jako
`pending[].preview`.

**Review plánu** (`ExitPlanMode`, `claude/plan.rs` + `claude/planhook.rs`): hlavní cesta je zdokumentovaný hook
`PreToolUse` s matcherem `ExitPlanMode` → `POST /plan` (Claude Code do `tool_input` vloží `plan` a `planFilePath`).
Karta ukáže plán (`tool_input.plan`, jinak obsah `planFilePath`; ≤ 64 k znaků) jako Markdown přes renderer poznámek
(escapuje Rust), sbalitelný, se scrollem uvnitř; peek ukáže „Plán: <první nadpis>“. „Vždy povolit“ u plánu není.
Odpovědi `/plan` (`hookSpecificOutput`, `hookEventName:"PreToolUse"`):

| klik | odpověď |
|---|---|
| **Schválit** (i zkratka approve) | `"permissionDecision":"allow"` + `updatedInput` = vstup beze změny (u ExitPlanMode povinné) |
| **Vrátit s připomínkou…** | `"permissionDecision":"deny"`, `"permissionDecisionReason":"Plán vrácen z Ledge s připomínkou: <text>"` |
| Zamítnout (i zkratka) | `"deny"`, důvod „Zamítnuto v Ledge.“ |
| **Vybrat režim v Claude** / termín / Ledge neběží | prázdné tělo (nebo chyba spojení) = Claude ukáže svůj dialog s volbou režimu |

`PreToolUse` režim po schválení nastavit neumí (docs: jen `permissionDecision`, `updatedInput`, `additionalContext`)
→ „Schválit a povolit úpravy“ (`extra.mode:"acceptEdits"`, allow + `setMode`) je jen u karty z `PermissionRequest`;
u karty z `/plan` (`pending[].via:"preToolUse"`) je místo něj `extra.inClaude:true` (jen s `id`) = Ledge nerozhodne.
Cesta `PermissionRequest` s `tool_name:"ExitPlanMode"` zůstává (docs neříkají, že chodí). **Jeden plán = jedna
karta:** druhá kopie téhož `PreToolUse` (session + `tool_use_id`, jinak obsah) se hned pustí; `PermissionRequest`
plánu téže session, na který čeká karta z `/plan` nebo který Ledge před < 2 min pustil do Claude, kartu nezaloží (`Pass`).

**Zapojení hooků** (Nastavení → Claude → „Zapojit hooky“, `src-tauri/src/hooksetup.rs`): do
`~/.claude/settings.json` (nebo `%CLAUDE_CONFIG_DIR%`) přidá http hooky na `http://127.0.0.1:<claude.port>`:
`PermissionRequest` → `/permission` (timeout 600 s), `PreToolUse` s `"matcher": "AskUserQuestion"` →
`/question` (timeout 600 s), `PreToolUse` s `"matcher": "ExitPlanMode"` → `/plan` (timeout 600 s), `SessionStart`, `UserPromptSubmit`, `PostToolUse`,
`Stop`, `SessionEnd`, `Notification`, `SubagentStart`, `SubagentStop` → `/event` (timeout 5 s).
Idempotentní, pořadí klíčů a cizí hooky (i cizí `PreToolUse`) zachová, své pozná podle URL
(`127.0.0.1|localhost:<port>/permission|/event|/question|/plan`; starý port nahradí) a u `/question` a `/plan` i podle
matcheru (oba žijí pod `PreToolUse` vedle sebe). Hooky ze starší verze bez otázek → `partial`, `missing:["PreToolUse"]`
s `hint` „Doplnit otázky Clauda“; bez review plánu → `missing:["PreToolUse(ExitPlanMode)"]` s „Doplnit review plánu“
(karta Claude to doplní jedním klikem, v Nastavení → Claude → Otázky Clauda otázky). Před první změnou záloha `settings.json.bak-pred-ledge`, zápis atomicky. „Odpojit“
odebere jen hooky Ledge. Platí pro **nově spuštěné** session. Tauri: `hooks_status`, `hooks_install`,
`hooks_remove`; stav je i ve snapshotu (`hooks`). Ručně hooky do settings.json nepiš — použij tlačítko.

**Když Ledge neběží** (hooky míří na mrtvý port): podle dokumentace je selhání http hooku neblokující
chyba a tok oprávnění pokračuje normálně. Hlášený bug „PermissionRequest selže uzavřeně“
(anthropics/claude-code#46193, 2.1.100, CLI) se **nereprodukoval** v desktopové appce s Claude Code
2.1.259 (17. 10. 2026): s vypnutým Ledge se ukázal běžný dotaz na oprávnění; kontrola s testovacím
serverem, který vracel deny, potvrdila, že se hook opravdu volá. Terminálové CLI ověřené není — sonda
`powershell -File scripts/hookprobe.ps1` (potřebuje přihlášené CLI) porovná `claude -p` bez hooku a s hookem
na mrtvý port; uživatelské hooky nenačítá, takže běžící Ledge nic nevidí.

**Otázky Clauda** (`AskUserQuestion`, `src-tauri/src/claude/questions.rs`, UI `src/questions/`): hook
`PreToolUse` → `POST /question`. Ledge otázku ukáže jako chat (peek bez fokusu + karta Claude pod
session: štítek `header`, text, možnosti s `description`, vícevýběr, „Jiné…“, víc otázek po stránkách)
a požadavek drží `claude.questionHoldS` s (výchozí 540, nejvýš 550; `0` = jen ukázat a hned pustit).
Odpověď z Ledge (`question_answer(id, answers[{labels, other}])`) vrátí přesně:
`{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":{…původní
tool_input vč. questions…,"answers":{"<text otázky>":"<label>"}}}}`. Vícevýběr = labely spojené `", "`
(`questions::MULTI_SEP`, NEOVĚŘENO naživo), „Jiné“ = text uživatele (u vícevýběru na konec). Prázdná
odpověď (= otázka v okně Claude jako dřív): „Odpovědět v Claude“ (`question_release`), termín, konec turnu
(`UserPromptSubmit`/`Stop`/`SessionEnd`), nečitelný vstup, Ledge neběží. Session mezitím `waiting`.
`PermissionRequest` pro `AskUserQuestion` se **vždy hned pouští** (`Pass`) — nikdy povolovací karta s JSONem.
`GET /status` vrací otázky v `questions`. Odpovídat přes API záměrně NEJDE (stejně jako schválení).

## Limity Claude 5h / týden

Karta Claude ukazuje pruh 5h a týdenního limitu (procenta, čas resetu místně + „za 2 h 10 min“),
volitelně i proužek (`strip.items` + `limits`). **Zdroj = jen lokální soubory, žádná síť, žádné tokeny:**

1. **Stavový řádek Ledge** (Nastavení → Claude → „Zapojit stavový řádek“, `statuslinesetup.rs`):
   klíč `statusLine` v `~/.claude/settings.json` dostane `{"type":"command","command":"<ledge.exe> --statusline"}`
   (cesta s `/`; bez mezer bez uvozovek, jinak `"…"` pro Git Bash / `& '…'` pro PowerShell; ostatní klíče
   objektu — `padding`, `refreshInterval` — zůstanou). Původní hodnota `statusLine` (celá, nebo „nebyla“) se
   uloží do `%LOCALAPPDATA%\Ledge\statusline-original.json` a „Odpojit“ ji vrátí přesně. Záloha
   `settings.json.bak-pred-ledge` a atomický zápis jako u hooků; nevalidní JSON = chyba, nic se nepřepíše.
2. **Wrapper** `ledge.exe --statusline` (`statusline.rs`, zpracuje se v `main` před Tauri — žádné okno, žádná
   druhá instance): přečte stdin od Claude Code, z `rate_limits.{five_hour,seven_day}.{used_percentage,resets_at}`
   zapíše `%LOCALAPPDATA%\Ledge\ratelimits.json` (`{"at":ms,"fiveHour":{"pct":…,"resetsAt":ms},"sevenDay":…}`,
   tmp + rename, stejné hodnoty nejvýš 1×/min) a spustí **původní** příkaz stejně jako Claude Code (Git Bash
   `bash -c`, když je — `CLAUDE_CODE_GIT_BASH_PATH`, `git.exe` z PATH, Program Files —, jinak
   `powershell -NoProfile -Command`), se stejným stdin; jeho stdout vrátí byte po bytu, exit kód taky.
   Bez původního příkazu nevypíše nic. Termín 5 s, Job Object (zrušený wrapper zabije i strom příkazu),
   CREATE_NO_WINDOW, chyby tiše.
3. **Záložně** `~/.claude/usage-log/latest.json` (stejný tvar), když existuje — bere se čerstvější podle `at`.

Vzorek starší 15 min = „staré (před X min)“; po `resetsAt` se procenta neukazují (neznáme), jen „obnoveno“.
**Připomínka resetu** (`claude.limitsReminder`): okno, které dosáhlo 100 %, v čase `resetsAt` krátce vysune
peek (bez fokusu, ~15 s) a blikne proužkem zeleně; ve fullscreen nic (propadne), reset starší 30 min (PC spal)
tiše. Ledge čte soubory na událost watcheru (`limitslive.rs`), v klidu žádné pollování; budí se jen v termínu
resetu. Tauri: `statusline_status`, `statusline_install`, `statusline_remove`; stav ve snapshotu (`limits`).
Mapa: `docs/codemap/limits.md`.

## Útrata Claude (odhad)

Sekce „Útrata Claude (odhad)“ dole v kartě Claude (rozbalovací, výchozí sbalená): dnes / 7 dní / 30 dní,
top projekty, modely, sloupce po dnech. Je to **odhad v cenách API, ne účet** — předplatné Pro/Max se po
tokenech neúčtuje. Zdroj = lokální transcripty Claude Code `~/.claude/projects/<slug>/**/*.jsonl`
(`CLAUDE_CONFIG_DIR` platí), řádky `type:"assistant"` s `message.usage`; stejná odpověď API (streamované bloky,
kopie po resume/forku) se počítá jednou podle `requestId` + `message.id`. Den = místní půlnoc, projekt podle
`cwd` (worktree → hlavní checkout, jinak název složky).

Výkon: přepočet jen po rozbalení sekce / otevření šuplíku s rozbalenou sekcí, nejvýš 1× za 60 s, ve vlákně
s nízkou prioritou CPU i disku; v klidu nic. Index `%LOCALAPPDATA%\Ledge\usage-index.json` (per soubor offset,
délka, mtime; agregace den × cwd × model) + `usage-index.seen` (viděné odpovědi v okně, 12 B na odpověď) —
čtou se jen přírůstky, soubory se změnou před začátkem okna se neotevírají. Smazání obou souborů = přepočet
od nuly. Ceník (USD / 1M tokenů, datum ověření, zdroj) je v `src-tauri/src/usage/price.rs`; model, který
ceník nezná, ukáže tokeny a cenu „—“. Config `claude.costs` (výchozí `true`); skrytá sekce
`drawer.sections.claudeCosts` nebo celá karta `drawer.sections.claude` = taky nic se nečte. Tauri: `costs_refresh`; stav ve
snapshotu (`costs`). Mapa: `docs/codemap/costs.md`.

## Instalace a aktualizace

- **Vydání (běžná cesta):** `powershell -NoProfile -File scripts/publish.ps1` (na sdíleném stroji přes
  frontu) z čistého, pushnutého `main` vývojového repa. Veřejné repo `jiri-hyan/ledge-app` nedostává
  vývojovou historii: každé vydání = jeden nový commit „snapshot“ (`git archive HEAD`, cesty
  s `export-ignore` v `.gitattributes` se vynechají), autor `jiri-hyan` s noreply e-mailem GitHubu. Strom
  i seznam změn se před commitem prověří proti `scripts/private/denylist.txt` (jména klientů, strojů,
  osobní údaje — soubor je jen ve vývojovém repu); jediný nález = konec bez vydání. Pak lokální build
  (verze = nejvyšší `v0.2.x` v obou repech + 1, zapíše se jen dočasně přes `scripts/ci-version.ps1`)
  a `gh release create` ve veřejném repu: instalátor + přílohy `SHA256SUMS.txt` (`<hex>  <soubor>`) a
  `pro-claude.md` (kopie `docs/pro-claude.md`, návod pro Clauda — „Nastav s Claudem“) + poznámky, první řádek
  `SHA256 (<setup>) = <hex>` (tvar čte `update.rs`, neměnit; řádky začínající `SHA256` se v UI z poznámek
  vypouštějí) a na konci sekce „Nastav s Claudem“ s promptem. Tag se vytvoří i ve vývojovém repu. `-DryRun` = jen snapshot
  a kontrola, `-AlsoPrivate` = stejný instalátor i do vývojového repa (přechod pro Ledge ≤ 0.2.8, které
  hledaly aktualizace tam). CI (`.github/workflows/release.yml`) je jen ruční a do veřejného repa se
  nekopíruje.
- **Aktualizace z appky** (`src-tauri/src/update.rs`, config `update: { check, repo, intervalH }`):
  60 s po startu a pak každých `intervalH` h se Ledge zeptá **veřejného GitHub API**
  `GET https://api.github.com/repos/<repo>/releases/latest` (`updatenet.rs`: bez tokenu, User-Agent
  `Ledge/<verze>`, termín 20 s) — **`gh` není potřeba**. Když API selže (soukromé repo → 404, limit
  60 dotazů/h → 403/429, výpadek) a `gh` je nainstalované, zkusí záložně `gh release view --repo <repo>`
  (bez okna); obě odpovědi čte tatáž `update::evaluate`. Stavy ve `Snapshot.update`: `off` · `neovereno`
  · `aktualni` · `dostupna` · `nelze_overit {duvod}` (offline / repo nenalezeno / limit API, s chybou
  zálohy přes gh). Hlavička ukazuje **jen** `dostupna` (chip „Nová verze X“), zbytek Nastavení →
  Záloha a aplikace → Aktualizace (+ „Zkontrolovat teď“). **Nikdy neinstaluje sám:** klik → poznámky → Instalovat → stažení
  přes HTTPS jen z `https://github.com/<repo>/releases/download/<tag>/<asset>` (`updatenet::download_ok`,
  strop 200 MB; záložně `gh release download`) do `%TEMP%\ledge-update\<tag>`, SHA256 se musí shodovat
  s poznámkami (a s `digest` GitHubu) — jinak
  nic nespustí → odpojený `cmd`: ~2 s pauza, `setup /S`, start nainstalovaného `ledge.exe`
  (`autostart::install_target`); Ledge se ukončí. Běží-li spravované servery (s Ledge by umřely),
  `update_install` vrátí jejich seznam a chce druhé potvrzení. Předběžné/nevalidní tagy se ignorují.
- **Lokální vydání:** `powershell -NoProfile -File scripts/release.ps1` → zkontroluje, že `version` v
  `src-tauri/tauri.conf.json` a `src-tauri/Cargo.toml` souhlasí, pustí `pnpm tauri build --bundles nsis`
  a vypíše cestu, velikost a SHA256 `Ledge_<verze>_x64-setup.exe` (`<target>\release\bundle\nsis\`).
  `-Install` navíc nainstaluje tiše (`/S`) a nainstalovaný Ledge spustí — ale jen když Ledge neběží;
  jinak řekne „zavři Ledge“ a skončí (tichý instalátor by běžící kopii sám násilně ukončil).
- **Kam se instaluje:** NSIS `installMode: currentUser` → `%LOCALAPPDATA%\Ledge\ledge.exe` (bez admina).
  Skutečné místo zapisuje instalátor do `HKCU\…\Uninstall\Ledge` (`InstallLocation`, `MainBinaryName`).
  Aktualizace = spustit novější setup přes starou instalaci; config v `%APPDATA%\Ledge` zůstává.
- **Vzhled a chování průvodce** (`src-tauri/installer/`, náhled A): česky + anglicky s výběrem jazyka,
  obrázky `sidebar.bmp`/`header.bmp` (BMP 24 bit; po změně `.svg` → `node scripts/installer-art.mjs`,
  `--check` ověří shodu), texty `Czech.nsh`/`English.nsh` (`customLanguageFiles` = celá sada řetězců
  šablony Tauri + naše `ledge*`), úpravy stránek `hooks.nsh` (přepsaná makra MUI2, ne kopie šablony).
  Dokončení: „Spustit Ledge“ + „Spouštět s Windows“ (místo zástupce na ploše) — zapíše Run `Ledge` jako
  appka (`"<instalace>\ledge.exe" --autostart`) a stejnou hodnotu `autostart` do configu
  (`installer/autostart.ps1`), protože appka config při startu promítá do Run. Výchozí stav = config.
  Odinstalace: „Ponechat nastavení“ (výchozí ano) = zaškrtávátko Tauri „smazat data“ s obráceným
  významem; odškrtnuté smaže `%APPDATA%\Ledge`, `%LOCALAPPDATA%\Ledge` a `…\cz.hyan.ledge` (WebView2),
  poznámky v `%USERPROFILE%\Ledge` nikdy. Tichý běh (`/S`, aktualizace z Ledge) nic z toho nemění.
- **Autostart** (`HKCU\…\Run`, hodnota `Ledge`, `src-tauri/src/autostart.rs`): existuje-li nainstalovaná
  kopie, míří Run **vždy na ni** — i když běží vývojový release z `target\release`, ten Run na sebe
  nepřepíše, takže dev build nemusí zastavovat „produkční“ Ledge. Bez instalace míří na běžící exe.
  Vypnutí maže hodnotu jen, když patří Ledge. Ladicí build a testy registr nikdy nemění. Odinstalace
  hodnotu Run maže sama. V bublině MSIX (Ledge spuštěný z Claude desktop) se Run nemění — viz níže.
- **Dev vedle instalované kopie:** stejný `identifier` → `tauri_plugin_single_instance`; druhý spuštěný
  Ledge jen předá argumenty první instanci (otevře šuplík) a skončí. Release build ale pořád selže,
  když běží právě binárka z `target\release` (zamčené exe) — skript to hlásí předem.

### Instalace z Claude session / bublina MSIX

Claude desktop je balíček MSIX (`Claude_pzs8sxrjxfjjc`). Co se spustí z jeho session (terminál, instalátor,
Ledge), zapisuje do **virtualizovaného** `%APPDATA%`, `%LOCALAPPDATA%` a HKCU — fyzicky
`%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\{Roaming,Local}` a soukromý hive. Explorer při
přihlášení tu kopii nevidí: Run „v bublině“ nic nespustí, config, data i `ledge.exe` žijí jen tam.
**Změřeno 18. 10. 2026:** procesy z Claude Code session NEMAJÍ identitu balíčku (`GetCurrentPackageFullName`
→ 15700), a přesto jsou virtualizované.

- **Detekce** (`src-tauri/src/pkgctx.rs`, jednou při startu, řádek `pkgctx:` v `ledge.log`): identita
  z `GetCurrentPackageFullName`, jinak **sonda** — malý soubor do kořene `%LOCALAPPDATA%`, hledá se jeho
  kopie v `Packages\*\LocalCache\Local` (rodina = jméno složky), hned se smaže.
- **Autostart v bublině** (`autostart::gate`): Run se nezapisuje ani nemaže; Nastavení → Chování →
  Spouštění a paměť to říká u přepínače. Instance mimo bublinu Run zapíše při startu.
- **Restartovat mimo** (stejné místo v Nastavení; v hlavičce tlumený chip „V bublině Claude“ → klik otevře
  Nastavení): jednorázová úloha Plánovače `Ledge-outside-relaunch` (`outside.rs`: `InteractiveToken`, bez
  limitu běhu, i na baterii, normální priorita) spustí Ledge s `--after-pid <pid>` — nová instance počká,
  až stará skončí (jinak by ji chytil single-instance). Úloha se smaže, jakmile naběhne. Záloha:
  `explorer.exe "<exe>"` (bez argumentů; předtím se uvolní zámek single-instance). Běžící servery chtějí
  druhé potvrzení (Job Object). Běží-li Ledge z virtualizované cesty, spustí se jeho fyzická kopie v bublině.
  Config z bubliny se **nepřenáší** — Nastavení ukáže obě cesty.
- **Aktualizace v bublině** (`update.rs`): instalátor jde přes Plánovač (`Ledge-outside-update`) s `/S /R`
  → tichá instalace do skutečného `%LOCALAPPDATA%` a šablona Tauri po ní sama spustí nainstalovaný Ledge
  (`/R`). Úlohu uklidí příští start Ledge (`outside::cleanup_stale`, jen když už neběží).
- **Instalace z Claude session:** `powershell -NoProfile -File scripts/install-local.ps1 [-Setup <setup.exe>]
  [-Start]` — nejnovější `src-tauri\target\release\bundle\nsis\Ledge_*_x64-setup.exe` tiše (`/S`) přes WMI
  `Win32_Process.Create` (mimo bublinu, bez okna, čeká na PID), ověří skutečnou instalaci (`CIM_DataFile`)
  a skutečný `Uninstall`/`Run` (`StdRegProv`). `-Start` spustí Ledge mimo bublinu jednorázovou úlohou
  Plánovače s `--autostart` a Run ověří znovu. Run sám nezapisuje (zapíše ho Ledge).

## Migrace z Notche

Při každém startu (před načtením configu) `src-tauri/src/migrate.rs`, idempotentně a s logem do
`%APPDATA%\Ledge\ledge.log`:

| stav | co se stane |
|---|---|
| `%APPDATA%\Ledge\config.json` chybí, `%APPDATA%\Notch\config.json` je | config se **zkopíruje** (starý zůstává); `notes.root` mířící na `%USERPROFILE%\Notch\notes` → prázdné (= nová výchozí); schéma se vygeneruje znovu |
| nový config už je | nic — nikdy se nepřepisuje |
| `%USERPROFILE%\Ledge\notes` chybí nebo je prázdná, stará `…\Notch\notes` je a config ukládá do výchozí | poznámky se **přesunou** (rename); jiný disk → kopie, ověření počtu souborů a bajtů, pak smazání starých; jiná chyba → staré zůstanou, zkusí se při dalším startu |
| po přesunu | ve staré složce `PRESUNUTO-DO-LEDGE.txt` s novou cestou — zároveň značka „hotovo“, další start už nic nepřesouvá |
| nová složka má data / config má vlastní `notes.root` / config nejde přečíst | poznámky zůstávají, jen řádek v logu |
| `LEDGE_CONFIG_DIR` (nebo starý `NOTCH_CONFIG_DIR`) nastaven | migrace se nespouští vůbec |

Autostart v release buildu smaže starou hodnotu `HKCU\…\Run\Notch`, míří-li na `notch.exe`. Starý Notch
**neodinstalovává** (klíč `Uninstall\Notch` i `%LOCALAPPDATA%\Notch` zůstanou) a s Ledge spolu neběží
dobře: jiný `identifier` → single-instance je nechytí a oba chtějí port mostu 47800. Port se nemění —
míří na něj hooky v `~/.claude/settings.json`.

## Pravidla

- Ledge **nikdy nekrade fokus** (kromě otevření zkratkou a výslovného kliku na vstup do poznámky —
  příkaz `focus_drawer`, nikdy z peeku, připomínky ani najetí) a při hře přes celou obrazovku je mrtvý.
  Neměň to.
- Automatizace bez GUI: `LEDGE_NO_GUI=1` (starší `NOTCH_NO_GUI=1` platí taky) → proces skončí dřív, než vytvoří jakékoli okno.
- Výkonový rozpočet (měř `scripts/perf.ps1` proti release buildu): idle CPU < 0,1 %, `ledge.exe`
  < 25 MB private, se WebView2 < 90 MB.
- Testy: `cd src-tauri && cargo test` (spouští reálné procesy a porty), `pnpm typecheck`, `pnpm build`.
