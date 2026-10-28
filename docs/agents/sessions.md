# Session, hooky, limity, útrata

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

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
**Pojistka neviděné otázky** `claude.questionUnseenS` (výchozí 20 s, 5–550, `0` = vypnuto, Nastavení → Claude →
Otázky Clauda): UI potvrdí „zobrazeno“ (`question_seen`), jen když je karta otázky opravdu vidět (šuplík otevřený,
WebView viditelné, karta v záběru; jádro to bere jen při otevřeném šuplíku a mimo fullscreen hru). Bez potvrzení do
N s Ledge otázku pustí (prázdná odpověď = Claude se zeptá ve svém okně), v Ledge zůstane jen ukázaná a deník
(Nastavení → Claude → Deník posledních otázek) dostane „Nikdo neviděl — předáno do Claude“ (`releasedUnseen`).
Platí, jen když je kratší než `questionHoldS`. **Restart Ledge za držení:** spojení hooku spadne → Claude Code to
bere jako neblokující chybu hooku a zeptá se ve svém okně (jako „Ledge neběží“, naživo NEOVĚŘENO); nový Ledge o
otázce neví (stav i deník jsou jen v paměti).
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

**Podlaha nové relace a agenta** (kolo 23, v sekci Útrata): řádky „Start nové relace (první volání)“ podle projektu a
„Start agenta podle typu“ — medián kontextu (vstup + zápis cache + čtení cache) PRVNÍHO volání API každého
souboru transcriptu za posledních 7 dní, počet souborů a trend proti předchozím 7 dnům (od 3 souborů v obou
obdobích, změna pod 5 % / 1 000 tokenů = beze změny). Typ agenta z `agent-*.meta.json` (`agentType`); subagent bez
něj se nepočítá. První usage souboru se zapisuje do indexu při běžném čtení; soubory ze starého indexu se doplní
jen čtením začátku souboru (žádný přepočet). Účel: nafouklý CLAUDE.md / MCP / skills je vidět hned po změně.

**Pruh kontextu session** (kolo 23, `claude.contextBar`, výchozí vypnuto): pod každou živou session s hooky pruh
velikosti kontextu = poslední `message.usage` v transcriptu (`transcript_path` z hooku; vstup + zápis cache + čtení
cache), rysky 250 k / 350 k, text „každý tah teď čte ~N k tokenů · N× víc než na začátku“ (poměr k prvnímu volání,
od 1,5×) a nad 250 k tip „zvaž předávku a novou relaci“ (ne `/compact`). Čte se jen konec souboru (256 kB, při
obřím posledním řádku víc do 8 MB) a jen když je šuplík otevřený a řádek vykreslený; Rust drží výsledek podle
mtime + délky a čte nejvýš 1× / 15 s na soubor. Session bez hooku / bez usage = pruh se nekreslí. Cesta musí být
`<.claude>/projects/**/*.jsonl`. Tauri `session_context(id)`. Mapa: `docs/codemap/costs.md` (`ctxsize.rs`).


**Export útrat a limitů** (kolo 20): v ⋯ karty Claude „Exportovat útratu a limity (CSV / Markdown)…“ → dialog Uložit
jako → soubor. Jen data, která Ledge už drží v paměti (spočítaný přehled útrat v cenách API, poslední vzorek limitů 5h /
týden); co chybí (cena neznámého modelu, okno bez dat), zůstane prázdné. CSV má sloupce `kind,name,period,tokens,cost_usd_api_estimate,pct,resets_at`
(řádky `limit`, `total`, `project`, `model`, `day`; UTF-8 s BOM; texty začínající `= + - @` dostanou apostrof), Markdown
tabulky s větou, že jde o odhad, ne fakturaci. Tauri `export_usage(format)`, žádná síť. Mapa: `docs/codemap/smart.md`.

**Stav incidentů Anthropicu** v kartě limitů: volba `smart.incidents`, viz [config-behavior.md](config-behavior.md) (Chytřejší šuplík).
