# Konfigurace — vzhled, zkratky, chování, přihrádka, schránka, skripty, MCP

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

- **Vzhled** — sekce `appearance` (Nastavení → Vzhled; chybějící sekce = dnešní vzhled):
  `theme` (`dark` výchozí, `midnight`, `graphite`, `nord`, `light`, `contrast`, `terminal`, `paper`, `neon`, `brutal`, `pastel`, `aurora`, `mono`, `retro`, `oled`, `warm`), `accent` (`#rrggbb`,
  prázdné = akcent motivu), `density` (`compact|normal|comfy`), `fontScale` 0.85–1.25, `radius`
  (`sharp|normal|round`), `glass.opacity` 0.25–1 (krytí pozadí; rozmazání je `drawer.glass: "acrylic"`),
  `motion.style` (`spring|smooth|snappy|bounce|fade|notch|drop|unfold|liquid|none`; `liquid` = tekutá kapsle ze středu jako Dynamic Island) + `motion.speed` 0.5–2 (řídí i vysunutí šuplíku; starý
  `drawer.animationMs` se při načtení převede: `0` → `none`, jiná délka → rychlost), `strip.style` (`pill|line|dot|term|tag|glow|block|drop`),
  `strip.size` (`small|normal|large`), `strip.color` (prázdné = přechod akcentu, `accent`, `#rrggbb`),
  `strip.position` (`center|left|right` — proužek i šuplík u horní hrany, myš otevírá tam). Pořadí a velikost karet je v `layout`
  (dřívější `appearance.cards` se převede). Šířka šuplíku zůstává `drawer.width`
  (za běhu nejvýš 95 % monitoru), krytí proužku `strip.opacity`. Změna se projeví bez restartu.
  **Design styly (vlna 6):** `style` (`classic` výchozí = vzhled před vlnou 6, `glass`, `terminal`, `paper`,
  `neon`, `brutal`, `pastel`, `aurora`, `mono`, `retro`, `fluent`, `oled`, `warm`) je jen jméno — výběr stylu v Nastavení / paletě (`Styl: Neon`) zapíše jeho
  hodnoty do polí výš a níž, ta jdou dál přebít (krok Otevřený → „Pokročilé“, „Obnovit styl“ je vrátí). Nová pole:
  `font` (`system|mono|serif|grotesk`, jen systémová písma Windows), `material`
  (`solid|glass|paper|glow|hard|soft|frame|acrylic|tinted`), `layout` (`classic|float|band|sheet|panorama|mosaic|tiles|panel` —
  sloupce a mezery karet), `width` (`custom` = `drawer.width` | `narrow` 820 | `medium` 1080 | `wide` 1440 |
  `full` = celá šířka monitoru). Další motivy: `terminal`, `paper`, `neon`, `brutal`, `pastel`; animace navíc
  `bounce`, `fade`, `notch`; tvary proužku navíc `term`, `tag`, `glow`, `block`, `drop`. Systémové „omezit
  pohyb“ (Windows → Efekty animace) vypíná animace i vysunutí okna. Tabulka stylů: `src/theme/looks.ts`.
  **Vzhled záložek a vlastní motivy (kolo 13):** `pages` = objekt `{ "<id záložky>": { style?, theme?, accent? } }`
  (id z `layout.pages`; prázdné pole = jako celek; `theme` = vestavěný motiv nebo `id` vlastního; přepis platí na aktivní
  záložce, proužek, šířka a animace zůstávají společné). `customThemes` = nejvýš 20 motivů
  `{ id (a–z 0–9 -, ne jméno vestavěného), name (1–32), base (vestavěný motiv pro vrstvy), colors }`, kde `colors` má
  všech 11 klíčů `bg raised t1 t2 t3 accent accent-2 green amber red blue` jako `#rrggbb`; text (`t1 t2 t3 green amber red
  blue`) musí mít na `bg` i `raised` kontrast ≥ 4.5:1, jinak se config neuloží. `customTheme` = `id` vybraného vlastního
  motivu pro celý vzhled (prázdné = vestavěný `theme`; ten pak slouží jen jako základ pro nativní proužek). Všechna tři
  pole se zapisují jen když nejsou prázdná; starý config je nemá.
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
  `hotkeys.notePin` (výchozí prázdné = vypnuto) = rychlý nový lístek na plochu (nová poznámka rovnou v lístku
  s fokusem); jako `notePassthrough` platí i s `hotkeys.enabled: false`, jen se zapnutými poznámkami, a ve hře /
  herním režimu nic neudělá.
- **Chytřejší šuplík** (`smart`, kolo 20; Nastavení → Chování a Claude) — všechno jsou volby, chybějící sekce
  = dnešní chování (od opravy v kole 21 jsou `quickKeys`, `dropShortcut` i `tabWheel` ve výchozím stavu VYPNUTÉ a zapíná je člověk):
  `smart.quickKeys` (výchozí `false`): má-li otevřený šuplík fokus, `Alt+1…9` spustí N-tého zástupce na záložce
  (dlaždice projektu, rychlá akce, odkaz; čísla se ukážou při držení Alt), šipky procházejí zástupce, Enter spustí;
  nic mimo šuplík, ne v úpravách a ne s paletou. `smart.dropShortcut` (výchozí `false`): odkaz, soubor, složka nebo
  `.exe` puštěné na OTEVŘENÝ šuplík otevřou editor akce s návrhem — nic se neuloží ani nespustí, dokud člověk
  nedá Přidat (`.exe` má předvyplněné „ptát se před spuštěním“, příkaz z puštěného nevznikne nikdy); na ZAVŘENÝ proužek
  dál jde přihrádka (`shelf`). `smart.tabWheel` (výchozí `false`): kolečko myši nad záložkami v hlavičce přepíná
  záložky (kolečko nad kartami dál posouvá obsah; dřív nad záložkami nedělalo nic).
  `smart.approvalSound` (`enabled` výchozí `false`, `minGapS` 5–600, výchozí 30): jedno tiché systémové pípnutí
  (`MessageBeep`, hlasitost řídí Windows), když přijde nové schválení nebo otázka Clauda; zticha při hře / herním
  režimu, při Nerušit (QUNS), s otevřeným šuplíkem a když je v popředí terminál nebo okno Claude.
  `smart.incidents` (`enabled` výchozí `false`, `everyMin` 5–120, výchozí 10): řádek s probíhajícím incidentem
  Anthropicu v kartě Claude u limitů. Jediná síť této sekce: `GET https://status.anthropic.com/api/v2/incidents/unresolved.json`
  (veřejné API, termín 5 s, ≤ 256 kB, přesměrování jen v rámci Anthropicu), nejvýš jednou za `everyMin` minut a jen když
  je šuplík otevřený a karta s limity vidět; žádná data uživatele neodcházejí. Mezi dvěma pokusy je vždy aspoň 60 s,
  i když se volba v Nastavení opakovaně přepíná (vypnout + zapnout odstup neobejde). Text incidentu se čistí od
  řídicích i neviditelných formátovacích znaků (nulové mezery, značky směru textu).
  Přetažený soubor, který by akce stejně odmítla (`.lnk`, `.url`, `.bat`, `.cmd`, cokoli mimo povolené přípony), se
  nenavrhne; šuplík ukáže větu proč (událost `drop-refused`).
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
  Dokud otázka Clauda čeká a šuplík je zavřený, puls se opakuje po 10 s (jen po tu dobu, v klidu nic).
  Přišla-li otázka / schválení ve fullscreen hře nebo herním režimu, peek vyjede po návratu.
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
- **Widgety na ploše** — sekce `desk` (Nastavení → Šuplík a karty → Na ploše) = jen výchozí chování NOVÝCH
  widgetů: `layer` (`top` navrchu · `normal` výchozí · `desktop` jen na ploše, pod okny), `hideInGame`
  (výchozí `true`: ve hře přes celou obrazovku a v herním režimu se widget schová a nic nepočítá), `opacity`
  (30–100 %, výchozí 100), `snap` (výchozí `true`: po přesunu přichytit k okraji plochy a k ostatním widgetům).
  Chybí-li sekce = výchozí hodnoty a žádný widget; pole se špatným typem = jeho výchozí hodnota (rozbitá sekce
  nikdy neshodí celý config). KTERÉ widgety a KDE jsou, není v configu (ten se může
  synchronizovat mezi PC), ale v `%LOCALAPPDATA%\Ledge\desk.json` (`{"widgets":[{"id":"lim1","kind":"limits",
  "x":…,"y":…,"w":…,"h":…,"layer":"normal","locked":false,"through":false,"opacity":100,"hideInGame":true,
  "compact":false}],"profiles":[{"name":"Práce","widgets":[…]}],"active":"Práce"}`); druhy `limits`, `waiting`,
  `dev`, `agenda`, `widget:<soubor>`; nejvýš 6 widgetů a 8 profilů. Ruční úprava souboru za běhu se přepíše —
  měň ho jen se zavřeným Ledge. Návod `docs/widgety-na-plose.md`, mapa `docs/codemap/desk.md`.
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
  ozdoba ekvalizéru — jen dekorace, neukazuje skutečný zvuk; `tint` (výchozí `false`, `false` se nepíše) = karta
  se jemně podbarví převládající barvou obalu alba (spočítá se v UI jednou na obal z 8×8 zmenšeniny).
  Starý config bez `media` = `card`, beze změny.
  `favorites` = oblíbené playlisty a stanice jako dlaždice v kartě Právě hraje (část `favorites`, jde skrýt):
  `[{ "name": "Ráno", "uri": "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M" }]`, nejvýš 24, název ≤ 60 znaků. `uri` =
  `spotify:playlist|album|artist|track|show|episode|user:<id>` nebo `http(s)://…` bez jména v adrese
  (`open.spotify.com`, `music.youtube.com`, jakýkoli jiný web); jiné schéma config odmítne
  (`media.favorites.N.uri`). Klik odkaz jen otevře ve Spotify / prohlížeči (stejný opener jako odkazy),
  Ledge nic nepřehrává — play / pauza / další zůstávají na GSMTC. Prázdné se do souboru nepíše.
- **Kontextové návrhy** — sekce `suggest` (Nastavení → Osobní → Kontextové návrhy): malý pruh nahoře v šuplíku, max 3
  chipy, **jen z reálných událostí** (bez zdroje se nic neukáže), každý jde zavřít na 24 h (jen v paměti).
  `clipboard` (výchozí **`false`** — čte historii schránky, která musí být zapnutá: adresu, kterou ještě nemáš mezi
  odkazy, nabídne „Uložit odkaz / Do poznámky“), `devServer` (`true`: krátce po naběhnutí dev serveru z Ledge
  „Otevřít localhost:PORT“), `github` (`true`: PR čekající na tvoji revizi / spadlé CI na tvém PR „Otevřít PR #N“,
  jen se zapnutým `github.enabled` a z dat, která už Ledge načetl), `reminders` (`true`: připomínka po termínu
  „Otevřít poznámku“), `githubPulse` (`true`: krátký puls proužku, když nové PR čeká na revizi nebo spadne CI na tvém PR;
  jen se zapnutým GitHubem, ve fullscreen hře nic, bez toastu a fokusu). Mapa `docs/codemap/suggest.md`.
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

