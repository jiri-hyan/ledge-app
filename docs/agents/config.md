# Konfigurace — základ, projekty

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

## Konfigurace = jeden soubor

**Všechno jde nastavit klikem v Nastavení** (ozubené kolo v šuplíku). Panely (od vlny 6, id v
`src/settings/registry.ts`; staré id vedou dál): **Začínáme** (průvodce, tipy, novinky) · **Šuplík a karty**
(záložky, karty, přihrádka) · **Vzhled** (4 kroky: Zavřený — vzhled zavřeného Ledge a celý proužek; Po najetí; Otevřený — styl, šířka a hustota na monitoru ve skutečném poměru, barvy; Pohyb — animace; jazyk) · **Projekty,
akce a odkazy** (projekty se autodetekcí ze složky, jeden seznam akcí — odkaz je akce —, skripty) · **Claude**
(schvalování, otázky, Claude (MCP), limity, útrata, hlídač, hooky) · **Obsah karet** (po kartách: počasí, kalendář, přehrávač,
časovač, GitHub, Systém; historie schránky) · **Poznámky** (poznámky, lístky, propojení) · **Otevírání a klávesy** (otevírání,
zkratky, paleta, šuplík nad prohlížečem) · **Spouštění a hry** (spouštění po startu, šetřit paměť, herní režim) · **Záloha a aplikace** (export / import s výběrem částí /
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
["tour1"] }` a už se neukáže (smazáním `tour1` z `ui.tips` se ukáže znovu). Po prohlídce jednou ještě
tip „Podrž kartu a přetáhni ji“ (značka `hold1` v `ui.tips`, stejně: smazáním se ukáže znovu).
Ručně: Nastavení → Aplikace → Jazyk a průvodce → Spustit průvodce. **Existující config se nemění:** chybějící `onboarding` = `done: true`
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
  strojích). `kind: "app"` = `.exe` (`exe` relativně k projektu, nebo `appName` nainstalované appky,
  nebo `aumid` aplikace z Microsoft Store — např. `Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`, jen
  `A-Za-z0-9._!-`; spouští se jako `shell:AppsFolder\<aumid>` bez sledování procesu, `exe`/`appName` mají
  přednost), volitelně s přípravným `run` (build). Aplikace nemá `openBrowser` ani `passPort` (patří dev
  serveru; MCP je pro `app` vždy vypne).
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
- **Ztišit git upozornění u projektu** (`"muteDirty": true`; chybí / `false` = upozorňovat jako dřív, `false`
  se nepíše): repo se nepočítá do chipu „N neuloženo · M nepushnuto“ v hlavičce šuplíku ani do proužku
  (`stripview.rs`); dlaždice (odznaky zdraví) a karta Repa stav ukazují dál. Mění se klikem: chip v hlavičce
  → „Ztišit u tohoto repa“, tlačítko Ztišit v kartě Repa, Nastavení → projekt → přepínač. „Přečteno“ v témže
  menu jen zhasne zvýraznění, dokud se počty nezmění (otisk `gitAlerts.read`, přežije restart).
- **Vypnout upozornění na neuloženou práci úplně** (`"gitAlerts": { "enabled": false }`; chybí = zapnuto jako dřív,
  výchozí se nepíše): chip v hlavičce i počty v proužku zmizí, dlaždice a karta Repa stav ukazují dál. `read` = krátký
  otisk stavu označeného „Přečteno“ (a–z 0–9, ≤ 64 znaků; zapisuje ho šuplík, ručně ne). Nastavení → Projekty →
  „Upozorňovat na neuloženou práci v repech“ a pod ním seznam ztišených rep s Odztišit.
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
