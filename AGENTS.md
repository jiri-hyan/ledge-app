# Ledge — návod pro agenty (Claude Code a spol.)

Ledge je always-on „šuplík“ u horního okraje obrazovky: projekty (klik = dev server / appka),
běžící servery a schvalování pro Claude Code. Tenhle soubor říká, jak ho **nastavovat a ovládat**
bez UI. Architektura je v hlavičkách souborů `src-tauri/src/*.rs`.

**Dřív se jmenoval Notch** (do 0.1.0, přejmenováno 2026-10-17; veřejná vydání: `jiri-hyan/ledge-app`). Staré cesty `%APPDATA%\Notch`, `%USERPROFILE%\Notch\notes`,
`%LOCALAPPDATA%\Notch` už **nepoužívej** — viz [Migrace z Notche](#migrace-z-notche).

## Konfigurace = jeden soubor

**Všechno jde nastavit klikem v Nastavení** (ozubené kolo v šuplíku): projekty (přidání ze složky
s autodetekcí, pořadí přetažením), chování šuplíku a proužku, Claude Code a hooky, poznámky a sync,
Osobní, zkratky, záloha (export / import s výběrem částí / obnova výchozích) a aktualizace. Ruční
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
hooky Claude Code, odkazy, motiv) — každý krok se hned ukládá, nic nemaže, duplicity přeskočí.
Ručně: Nastavení → Průvodce. **Existující config se nemění:** chybějící `onboarding` = `done: true`
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
- **Projekt, který na tomhle PC není** (dlaždice „nenalezeno“), má v rychlých akcích **Naklonovat**:
  `gh repo clone <owner/repo>` (jiný hostitel než GitHub: `git clone <url>`) do první složky z `roots`
  (kolize jména → `<repo>-<owner>`, `<repo>-2`…), bez okna, termín 15 min, pak se projekt dohledá sám.
  Nenabízí se u dvou klonů téhož repa (nejednoznačné) ani u `appName` bez `exe`.
- **Vypnout část šuplíku** — `drawer.sections.{projects,running,claude,notes}`, záložka Osobní
  `modes.personalEnabled`, proužek `strip.enabled`, poznámky celé `notes.enabled`.
- **Vzhled** — sekce `appearance` (Nastavení → Vzhled a animace; chybějící sekce = dnešní vzhled):
  `theme` (`dark` výchozí, `midnight`, `graphite`, `nord`, `light`, `contrast`), `accent` (`#rrggbb`,
  prázdné = akcent motivu), `density` (`compact|normal|comfy`), `fontScale` 0.85–1.25, `radius`
  (`sharp|normal|round`), `glass.opacity` 0.25–1 (krytí pozadí; rozmazání je `drawer.glass: "acrylic"`),
  `motion.style` (`spring|smooth|snappy|none`) + `motion.speed` 0.5–2 (řídí i vysunutí šuplíku; starý
  `drawer.animationMs` se při načtení převede: `0` → `none`, jiná délka → rychlost), `strip.style` (`pill|line|dot`),
  `strip.size` (`small|normal|large`), `strip.color` (prázdné = přechod akcentu, `accent`, `#rrggbb`),
  `strip.position` (`center|left|right` — proužek i šuplík u horní hrany, myš otevírá tam), `cards`
  (pořadí karet Práce: `running`, `claude`, `repos`, `notes`). Šířka šuplíku zůstává `drawer.width`
  (za běhu nejvýš 95 % monitoru), krytí proužku `strip.opacity`. Změna se projeví bez restartu.
- **Zkratky** — globální zkratky jsou ve výchozím stavu **vypnuté** (nic se neregistruje, v UI se
  nikde neukazují); šuplík se otevírá myší, poznámka tlačítkem. Zapnout jde v Nastavení → Zkratky, nebo v configu:
  `hotkeys.enabled: true` + `hotkeys.toggle|approve|deny|note` (formát `Ctrl+Alt+Space`). Config bez
  `enabled` = vypnuto, i když má zkratky vypsané. Kolize zkratek se kontrolují jen při `enabled: true`.

## Rychlé akce (`actions`)

Dlaždice na jedno kliknutí (karta „Rychlé akce“ na Práci i Osobní) a položky palety příkazů
(`Ctrl+K` v šuplíku s fokusem, 🔍 v hlavičce). Pořadí v poli = pořadí dlaždic. Mapa kódu:
`docs/codemap/actions.md`.

```json
"actions": [
  { "id": "mapy", "label": "Mapy", "kind": "url", "target": "https://mapy.cz", "showOn": "personal" },
  { "id": "faktury", "label": "Faktury", "kind": "folder", "target": "C:\\Users\\x\\Documents\\Faktury" },
  { "id": "build", "label": "Build webu", "kind": "command", "target": "npm run build",
    "cwd": "C:\\Users\\x\\Desktop\\web", "group": "Dev", "showOn": "work", "confirm": true }
]
```

- `kind`: `url` (jen http/https) · `file` (dokumenty/média — allowlist jako odkazy Osobní; skripty ne)
  · `folder` · `app` (`.exe` absolutně nebo jen jméno ze složky Windows; `args` = s argumenty) ·
  `command` (`cmd /c`, nebo `"shell": "powershell"` = `-NoProfile`; `"terminal": true` = viditelné
  okno, jinak bez okna s termínem `timeoutS` (výchozí 60 s) a výstupem do hlášky / karty).
- Volitelné: `args` (pole, bez uvozovek), `cwd` (absolutní; výchozí domovská složka), `icon`
  (chybí/`"auto"` = favicon webu nebo ikona souboru, `"monogram"`, nebo cesta k png/ico/jpg/svg),
  `group`, `confirm` (ptát se před každým spuštěním), `showOn` (`work`/`personal`/`both`).
  Výchozí hodnoty se do souboru nepíšou. `id` = a–z, 0–9, `-`, `_`, jednoznačné; nejvýš 64 akcí.
- **Agent smí akce přidávat a měnit** (web, složka, soubor, aplikace bez argumentů se spouští hned).
  **Příkaz a aplikace s argumenty se bez souhlasu člověka vždy ptají** — souhlas uděluje jen člověk
  zaškrtnutím v Nastavení a ukládá se mimo config (`%LOCALAPPDATA%\Ledge\actions-consent.json`, otisk
  přesného příkazu). Ten soubor nikdy nezapisuj; změna cíle/argumentů/složky souhlas zneplatní.
- Favicon se stáhne jednou (při přidání akce, `%LOCALAPPDATA%\Ledge\icons\<host>.png`, neúspěch
  `<host>.miss`); znovu jen tlačítkem „Obnovit“ v Nastavení. Nic se nestahuje periodicky.
- Starší `personal.links` dál fungují (karta Odkazy); Nastavení → Rychlé akce nabídne převod, který
  odkazy zkopíruje (nemaže je).

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
| `POST /ctl/open` `{"view":"full"\|"peek"}` | otevře šuplík (bez fokusu, nezavírá se sám) |
| `POST /ctl/close` | zavře šuplík |
| `POST /ctl/settings` | otevře Nastavení |
| `POST /ctl/launch` `{"id":"atlas"}` | spustí projekt jako klik na logo |
| `POST /ctl/stop` `{"id":"atlas"}` | zastaví projekt (celý strom procesů) |
| `POST /ctl/reload` | znovu načte config |
| `POST /permission` | **jen pro hook `PermissionRequest`** |
| `POST /question` | **jen pro hook `PreToolUse` s matcherem `AskUserQuestion`** (otázky Clauda) |
| `POST /event` | hooky SessionStart/UserPromptSubmit/PostToolUse/Stop/SessionEnd/Notification/SubagentStart/SubagentStop |

Požadavky s hlavičkou `Origin` (prohlížeč) se odmítají.

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
| běží na pozadí / neznámý proces | nic, UI řekne proč |

Běžící session se **nikdy** neresumuje (prokládání zpráv). Okno se přepíná jen na výslovný klik.

**Zapojení hooků** (Nastavení → Claude Code → „Zapojit hooky“, `src-tauri/src/hooksetup.rs`): do
`~/.claude/settings.json` (nebo `%CLAUDE_CONFIG_DIR%`) přidá http hooky na `http://127.0.0.1:<claude.port>`:
`PermissionRequest` → `/permission` (timeout 600 s), `PreToolUse` s `"matcher": "AskUserQuestion"` →
`/question` (timeout 600 s), `SessionStart`, `UserPromptSubmit`, `PostToolUse`,
`Stop`, `SessionEnd`, `Notification`, `SubagentStart`, `SubagentStop` → `/event` (timeout 5 s).
Idempotentní, pořadí klíčů a cizí hooky (i cizí `PreToolUse`) zachová, své pozná podle URL
(`127.0.0.1|localhost:<port>/permission|/event|/question`; starý port nahradí) a u `/question` i podle matcheru.
Hooky ze starší verze bez `PreToolUse` → `partial` s `hint` „Doplnit otázky Clauda“ (karta Claude i panel
Nastavení → Otázky Clauda to doplní jedním klikem). Před první změnou záloha `settings.json.bak-pred-ledge`, zápis atomicky. „Odpojit“
odebere jen hooky Ledge. Platí pro **nově spuštěné** session. Tauri: `hooks_status`, `hooks_install`,
`hooks_remove`; stav je i ve snapshotu (`hooks`). Ručně hooky do settings.json nepiš — použij tlačítko.

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

## Instalace a aktualizace

- **Vydání (běžná cesta):** `powershell -NoProfile -File scripts/publish.ps1` (na sdíleném stroji přes
  frontu) z čistého, pushnutého `main` vývojového repa. Veřejné repo `jiri-hyan/ledge-app` nedostává
  vývojovou historii: každé vydání = jeden nový commit „snapshot“ (`git archive HEAD`, cesty
  s `export-ignore` v `.gitattributes` se vynechají), autor `jiri-hyan` s noreply e-mailem GitHubu. Strom
  i seznam změn se před commitem prověří proti `scripts/private/denylist.txt` (jména klientů, strojů,
  osobní údaje — soubor je jen ve vývojovém repu); jediný nález = konec bez vydání. Pak lokální build
  (verze = nejvyšší `v0.2.x` v obou repech + 1, zapíše se jen dočasně přes `scripts/ci-version.ps1`)
  a `gh release create` ve veřejném repu: instalátor + poznámky, první řádek `SHA256 (<setup>) = <hex>`
  (tvar čte `update.rs`, neměnit). Tag se vytvoří i ve vývojovém repu. `-DryRun` = jen snapshot
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
  Aktualizace (+ „Zkontrolovat teď“). **Nikdy neinstaluje sám:** klik → poznámky → Instalovat → stažení
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
- **Autostart** (`HKCU\…\Run`, hodnota `Ledge`, `src-tauri/src/autostart.rs`): existuje-li nainstalovaná
  kopie, míří Run **vždy na ni** — i když běží vývojový release z `target\release`, ten Run na sebe
  nepřepíše, takže dev build nemusí zastavovat „produkční“ Ledge. Bez instalace míří na běžící exe.
  Vypnutí maže hodnotu jen, když patří Ledge. Ladicí build a testy registr nikdy nemění. Odinstalace
  hodnotu Run maže sama.
- **Dev vedle instalované kopie:** stejný `identifier` → `tauri_plugin_single_instance`; druhý spuštěný
  Ledge jen předá argumenty první instanci (otevře šuplík) a skončí. Release build ale pořád selže,
  když běží právě binárka z `target\release` (zamčené exe) — skript to hlásí předem.

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
