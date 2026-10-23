# Rychlé akce, skripty, widgety

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

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

