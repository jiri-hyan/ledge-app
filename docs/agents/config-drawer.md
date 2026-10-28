# Konfigurace — otevírání šuplíku, schvalování, poloha počasí

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Tohle jsou klíče `config.json`, které
dřív v návodu chyběly; hlídá je `scripts/config-keys.mjs` (test `scripts/config-keys.test.mjs`). Všechno jde
i klikem: Nastavení → Šuplík a karty → Otevírání a klávesy (otevírání, okna s lištou), Aplikace → Spouštění a hry
(šetření paměti), Claude (schvalování, hlídač).

## `drawer` — jak se šuplík otevírá a zavírá

Zdroj: `src-tauri/src/config/drawer.rs`. Uložení se projeví hned (config se sleduje, viz [config.md](config.md)).

- `drawer.trigger` — jak se šuplík otevírá myší: `hover` (najetí na proužek, výchozí) | `edge` (přitlačit
  myš k hornímu okraji) | `dwell` (podržet myš na proužku) | `off` (jen zkratkou).
- `drawer.hoverDelayMs` (140) · `drawer.edgeDelayMs` (120) · `drawer.dwellDelayMs` (450) — prodleva v ms
  před otevřením podle `trigger`; krátká prodleva chrání proti náhodnému průjezdu myši.
- `drawer.closeDelayMs` (350) — za jak dlouho se šuplík zavře, když z něj myš odjede.
- `drawer.keepWarmS` (0) — úsporný režim „Šetřit paměť“: po kolika sekundách nečinnosti se WebView šuplíku
  úplně zruší. `0` = držet připravený (okamžité otevření, ~150 MB); `>0` = v klidu desítky MB, ale první
  otevření po nečinnosti trvá několik sekund (studený start WebView2).
- `drawer.barWindows` — „Šuplík nad prohlížečem“: okna s lištou záložek nahoře (prohlížeče), kde proužek
  ustoupí a šuplík se otevírá jen tlačením myši na horní hranu, ať ho neotvírá běžné přepínání tabů:
  - `processes` — názvy exe (na velikosti písmen nezáleží; výchozí Opera, Chrome, Edge, Firefox, Arc,
    Brave, Vivaldi); platí jen u maximalizovaného okna na monitoru proužku. Přidat jde třeba `Code.exe`.
  - `trigger` — `push` (výchozí: proužek se smrskne na linku a šuplík otevře jen skutečný tlak myši
    nahoru) | `hover` (beze zvláštního chování, platí obecný `drawer.trigger`).
  - `pushDistance` (60, min. 10) — síla tlaku v „counts“ raw inputu za 600 ms (≈ 2 mm u myši s 800 DPI).
  - `pushMs` (300, min. 100) — záložní pravidlo, kde tlak měřit nejde (tablet, vzdálená plocha): jak dlouho
    souvisle stát na horní hraně.

Další klíče `drawer` (`hover`, `hoverExpandMs`, `width`, `glass`, `sections`) jsou v
[config-layout.md](config-layout.md) a [config-behavior.md](config-behavior.md).

## `claude` — schvalování a hlídač

Zdroj: `src-tauri/src/config/claude.rs`. Schvalovat smí jen člověk — žádné `/ctl/decide` ani obdoba.

- `claude.approvalTimeoutS` (45) — jak dlouho Ledge drží žádost o schválení. Pak ji vrátí terminálu
  (nikdy neschválí sám). Strop 550 s (hook `PermissionRequest` má timeout 600 s).
- `claude.enforceCommit` — hlídač nedokončených session (soubory, které session upravila a
  nezacommitovala, a commity, které nejsou na žádném remote): `off` (nehlídat, žádné volání gitu) |
  `notify` (výchozí: štítky ●/↑ v šuplíku a proužku) | `block` (hook Stop vrátí session do práce
  s důvodem, nikdy při `stop_hook_active`).

Ostatní klíče `claude` (`enabled`, `port`, `questionHoldS`, `limitsReminder`, `limitsPace`, `toast`, `costs`, `contextBar`) viz
[sessions.md](sessions.md) a [config-personal.md](config-personal.md).

## `personal.weather` — poloha počasí

`personal.weather.lat` (−90 až 90) a `personal.weather.lon` (−180 až 180) jsou souřadnice místa pro kartu
Počasí (Open-Meteo, bez klíče); `label` je jen popisek na kartě. Vyplněná jen jedna souřadnice neprojde
validací (`config/validate.rs`), obě `null` = nenastaveno (karta jen vyzve k nastavení, žádné volání sítě).
Polohu podle IP Ledge nikdy nezjišťuje. V Nastavení → Šuplík a karty → Obsah karet stačí napsat město (`src/settings/cityModel.ts`).

## `projects[].subdir`

Podsložka projektu, ve které se spouští příkaz (`run`), např. `"subdir": "web"` pro monorepo. Bez `subdir`
se spouští v kořeni repa (`state.rs` `project_dir`; zástupce bez `path` se hledá podle `remote`, `subdir` se přidá až za nalezenou složku). Řídí ji i MCP (`mcp/edit.rs`).

## `$schema`

`"$schema": "./config.schema.json"` je odkaz pro editory (našeptávání a validace v IDE); Ledge ho ignoruje
a drží ho jen kvůli pohodlí. Schéma se generuje z Rust struktur (viz [config.md](config.md)).
