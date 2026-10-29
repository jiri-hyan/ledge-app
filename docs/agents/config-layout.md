# Konfigurace — rozložení šuplíku, najetí, proužek

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

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
  ty tři jsou jen v katalogu, na žádné záložce ve výchozím rozložení nejsou — `layout::OPT_IN`), `activity` (Claude
  aktivita, kolo 23 — jen v katalogu: session s fází, souborem, poslední zprávou a schválením; na záložce zapne
  čtení posledních 64 kB přepisu session při hooku `Stop`, jinak se přepis nečte) a **dynamické** `widget:<soubor>` (widget ze
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
  `tiles` | `compact`, u `projects` / `repos` / `notes` i `bento` (kolo 18: velká dlaždice + menší kolem — u Projektů
  dva první, běžící dopředu; u Poznámek připnutá / nejnovější poznámka + dlaždice Nová .md, šablony, Uložit HTML).
  Kolo 22 (karty 2026, vše volitelné, chybí = podle stylu): `surface` (`glass` = sklo jen na rámu a hlavičce, pod
  textem plná stabilní plocha | `layer` = plná vrstva s jemnou hloubkou), `grain` (`true` = jemné zrnění ~3,5 %,
  statický vzor bez obrázků zvenku), `hero` (`true` = „číslo jako hrdina“: velké skutečné číslo s popiskem a grafem,
  jen karty `claude` — limit 5 h; bez limitů prázdný stav „Limity zatím nejsou“ s důvodem, ne jiné číslo; čeká-li schválení, karta zůstane normální — a `weather`
  — teplota; bez čísla z reálného zdroje karta zůstane jako dřív). Tokeny (základ, kontrast, poloměr, mezera) dává
  materiál design stylu.
  Kolo 25: `design` (řetězec, id vzhledu z registru `src/layout/designs.ts`; chybí = výchozí, dnešní vzhled karty; neznámé id
  se ignoruje). `claude`: `classic` | `rings` | `pace` | `hero` | `heroWeek` | `heroSpend` | `spend` | `sessions` | `gauge`;
  `system`: `lines` | `rings` | `gauges` | `bars` | `terminal` | `tiles` | `net`. Volí se v ⋯ → Vzhled karty… (dlaždice) i
  v Nastavení → Šuplík a karty.
  Kolo 26: karta `notes` má vzhledy `list` (výchozí) | `compact` | `cards` | `bento` | `kanban` | `timeline` (dřívější `view` `tiles`/`compact`/`bento`
  se při čtení převede na `design`) a volby seskupení `groupBy` (`project` | `kind` | `date` | `pinned`; chybí = bez seskupení)
  a `groupStyle` (`tag` | `cards`; chybí = čára s nadpisem). Neznámé hodnoty se ignorují.
  V šuplíku ⋯ na kartě → „Vzhled karty…“ (živý náhled, Obnovit podle stylu); pro celou záložku najednou
  Nastavení → Vzhled → Otevřený → „Karty 2026“ (galerie předvoleb).
  `limit` (volitelné, jen karty `claude`, `repos`, `running`; 0–50, `0` = bez stropu, chybí = 4) = kolik řádků
  karta ukáže, zbytek pod „Zobrazit další (k)“ ↔ „Méně“; session, které čekají na tebe, se nikdy neschovají
  (⋯ karty → Řádků 3/4/6/Vše, nebo Nastavení → Šuplík a karty).
  `parts` (volitelné, jen karty `claude`, `media`, `clock`) = pořadí částí karty shora dolů, jak je uživatel
  přetáhl v úpravách: `claude` `claudeLimits`, `body`, `claudeCosts`; `media` `nowPlaying`, `sound`, `favorites`; `clock`
  `body`, `battery` (`body` = hlavní obsah karty: seznam session, hodiny). Chybí nebo neúplné = výchozí
  pořadí (chybějící části se doplní na konec); neznámá nebo dvojitá část = chyba `layout.pages.i.cards.j.parts.k`.
  `claudeGit` a `claudeSubagents` v pořadí nejsou (kreslí se u každé session zvlášť). Skrývání částí zůstává
  v `hiddenParts` (platí pro kartu na všech záložkách).
  Záložka může mít `bento` (`true`, kolo 22; chybí = běžná mřížka): karty jako dlaždice 1×1, 2×1, 2×2… = šířka
  `size` × výška `h` v řádcích (řádek ≈ 120 px × hustota, v bento chybějící `h` = 2), mezera 16 px, volná místa
  vyplní hustý tok mřížky. ⋯ karty → Dlaždice 1 × 1 / 2 × 1 / 2 × 2. Šablony „Pracovní“, „Minimal“, „Hudba“
  (Nastavení → Vzhled → Otevřený → „Bento šablony“) přidají NOVOU záložku s 6–7 dlaždicemi; stávající záložky
  se nemění.
  `upgrades` = jednorázové úpravy rozložení po aktualizaci,
  které už proběhly — neměň ručně: `personalProjects` dá kartu Projekty nahoru na Osobní, když
  tam nikdy nebyla (jen když ji uživatel používá na jiné záložce a Osobní existuje); odebereš-li ji
  potom, už se nevrátí.
  Karta, která není na žádné záložce, je skrytá a nic nepočítá: skryté počasí ani kalendář se nestahují
  (stahují se, jen když je karta na otevřené záložce), skrytá útrata / karta Claude kvůli sobě neindexuje transcripty (indexaci zapne i karta „Claude dnes“ `usage` na záložce nebo widget na ploše; úplně ji vypne jen `claude.costs: false`),
  skryté Právě hraje / hlasitost / Čas (baterie) zruší odběry událostí Windows, skrytý GitHub / Systém
  nevolá `gh` ani nevzorkuje. `hiddenParts` = skryté části
  karet: `claudeLimits`, `claudeCosts`, `claudeGit`, `claudeSubagents` (karta Claude), `nowPlaying`, `sound`, `favorites`
  (karta Právě hraje), `battery` (karta Čas). Úpravy: v šuplíku ✎ (nebo tužka v liště záložek, nebo **podržení
  ~0,4 s na hlavičce karty** — zapne úpravy a rovnou zvedne kartu; v úpravách jde kartu chytit kdekoli, ne jen za ⠿:
  poloprůhledný duch pod kurzorem, mezera na místě dopadu, ostatní se plynule odsunou; dole zóna „Skrýt kartu“,
  tah na ouško záložky = přesun, po ~0,6 s nad ouškem se záložka přepne a tah pokračuje v její mřížce; Esc tah
  zruší, po dopadu lišta „… · Vrátit“; stejně se přetahují **záložky** (v úpravách tahem, mimo ně podržením) a **části karet**
  za lištu části — pořadí uvnitř karty, nebo dolů na „Skrýt“ = `hiddenParts`; karta puštěná na „+“ vytvoří záložku „Nová záložka“) (**tah za pravý okraj karty
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
  Nastavení → Vzhled → Po najetí → Pokročilé „Rozbalit náhled samo“;
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
  `strip.colors` / `strip.quiet` jako klíč). Config proužku žije v `src-tauri/src/stripconf.rs` (re-export z `config/*`). Další pole:
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
  - Nastavení → Vzhled → Zavřený → Co proužek ukazuje (v Základním; celý proužek je ve Vzhledu): řazení tahem nebo Alt+↑/↓, u položky barva a „Jen při dění“,
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
    5 s, jinak žádný časovač). Kolo 23: `custom` (vlastní, `strip.compose` níž). Kolo 22: `minimum` (baterie + čas; bez baterie jen čas), `trio` (počasí z paměti karty
    Počasí — proužek nic nestahuje, starší než 3 h = nic | čas a den | CPU a paměť jako u položky „Systém“ po 10 s,
    baterie), `rings` (stav Clauda: prstence limitů 5h a týden + čas; bez limitu jen prázdná dráha, točí se jen
    když Claude pracuje a animace jsou zapnuté, oranžově = čeká na tebe). Baterii čte Ledge z oznámení Windows
    o napájení (žádný časovač) za stejných podmínek jako karta Čas (`battery::allowed`). Priorita: čeká na tebe (i připomínka) > hotovo (session doběhlá za 5 min) >
    časovač > Claude pracuje > hudba > klid. U stylů mimo `classic` se `items`, `quiet`, `colors`, `custom`,
    `opacity` a `appearance.strip.style`/`color` nepoužijí; `appearance.strip.size` styl zvětší / zmenší.
    `strip.enabled: false` = jako dřív jen linka při událostech; nad taby prohlížeče ustoupí na linku všechny
    styly kromě `line`, `dots`, `corner`, `ghost`. Seznam s názvy pro Nastavení: `src/strip/looks.ts` (`STRIP_LOOKS`).
  - `strip.lookHide` — Pokročilé u stylů mimo `classic`: co styl nemá ukazovat (`waiting`, `claude` = pracuje /
    hotovo, `running`, `repos`, `timer`, `media`, `limits`, `clock`, `system` = CPU a paměť kapsle `trio` — pak se
    ani nevzorkuje). Výchozí `[]` = všechno.
  - `strip.lookShape` — tvar kapsle u stylů s tělem (`island`, `icons`, `clock`, `ghost`, `progress`, `ticker`,
    `minimum`, `trio`, `rings`; kolo 22): `round` (výchozí = dnešní tvar), `sharp` (ostré rohy), `long`
    (prodloužená), `island` („jako Mac“: oblá ze všech stran, 6 px pod hranou). Ostatní styly ho ignorují.
  - `strip.look: "custom"` + `strip.compose` — **vlastní zavřený Ledge** (kolo 23, `looks/compose.rs`; jen výslovnou
    volbou, výchozí zůstává `classic`, starý config bez `compose` = výchozí hodnoty). `compose`: `shape` (`capsule`
    výchozí, `notch`, `line` = linka přes celou hranu s úsekem na položku, `corner`, `drop`, `island`), `width`
    60–620 px (výchozí 300; u `line` se nepoužije), `height` 18–52 px (`line` 2–10, výchozí 30), `round` 0–100 %
    (výchozí 55), sloty `left` / `center` / `right` = pole `{ "item", "style" }`, dohromady 1–5 položek, každá
    nejvýš jednou. `item`: `time`, `date`, `claude`, `limit5h`, `limitWeek`, `music`, `timer`, `cpu`, `memory`,
    `battery`, `weather`, `servers`, `repos`, `activity` (kolo 23, „Claude aktivita“: čekající schválení s počtem,
    jinak fáze vedoucí session + soubor — fáze je odhad podle posledního nástroje z hooku `PostToolUse`, krok
    čtení → úpravy → testy → hotovo jako podíl prstence; bez hooků nic) (jen co má skutečný zdroj; bez dat se položka nekreslí, prstenec limitu
    = prázdná dráha). `style`: `icon`, `badge` (ikona s číslem), `text`, `ring`, `graph` (malý graf = sloupek
    z aktuální hodnoty), `dot`. `hideIdle` (v klidu skrytý, ukáže se při „čeká“ / „hotovo“), `rotate` (víc událostí
    uprostřed se střídá po 5 s). Mimo rozsah / přes 5 položek se při použití ořízne (`Compose::norm`), neznámá
    položka = chyba configu. Hudba, baterie a CPU / paměť se čtou jen, když je v `compose` ta položka. MCP
    `strip_set` `compose` ve schématu nemá (strop `tools/list`); skládá se v Nastavení → Vzhled → Zavřený → Vlastní.
    **Kolo 25** (výchozí = dnešní vzhled, starý `compose` bez nových klíčů se kreslí beze změny): `width` až 1200 px
    (tělo nejvýš na šířku monitoru), 1–12 položek, v jednom slotu nejvýš 5; co se na monitor nevejde, schová se od
    konce slotu (nejdřív širší bok, střed naposled), text se neořezává. Položka má volitelné `color` (chybí = podle
    hodnoty, `accent`, `#rrggbb`). Nové `style`: `iconText` (ikona + hodnota), `bar` (hodnota s pruhem), `spark`
    (průběh jen `cpu` / `memory` — body jen ze vzorků, které proužek stejně čte, ~10 s, bez historie = sloupek),
    `chip` (barevný podklad), `dotPct` a `ring5h` (jen `claude`: tečka stavu + 5h %, prstenec 5h v barvě stavu).
    `deco`: `edge` (`none` výchozí, `underline`, `outline`, `glow`, `gradient`), `len` (`center` = 40 %, `full`),
    `color` (`accent` výchozí, `claude` = barva podle stavu Clauda, `#rrggbb`), `color2` (#rrggbb, druhá barva
    přechodu), `width` 1–3 px (výchozí 2). `fill`: `kind` (`black` výchozí, `glass`, `tint`, `gradient`, `none` =
    bez těla, text se stínem), `from` / `to` (#rrggbb), `dir` (`right`, `down`, `diag`). `sep`: `none` výchozí,
    `gap`, `dot`, `bar`, `pills` (každý díl v kapsli), `islands` (sloty jako tři ostrůvky). `mono` (písmo
    s pevnou šířkou). Neznámá barva = výchozí, neznámý druh / oddělení = chyba configu.
