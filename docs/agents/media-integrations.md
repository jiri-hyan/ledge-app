# Právě hraje, hlasitost, baterie; GitHub a Systém

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

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
  **Mikrofon** — tlačítko ztlumí / zapne výchozí vstup (jen když je mikrofon; stav z callbacku, i ztlumení zvenku).
  **Výstup** — s víc aktivními výstupy menu se seznamem; přepíná se jen na zařízení, které Ledge sám nabídl.
  Přepnutí používá **nedokumentované COM rozhraní Windows `IPolicyConfig`** (stejně jako běžné přepínače zvuku);
  když selže (jiná verze Windows), Ledge otevře Nastavení zvuku Windows a řekne to větou. Mikrofon i výstup jdou
  ovládat jen se zapnutou kartou (`drawer.sections.sound` + záložka Osobní, `volume::allowed`).
- **Baterie** (`drawer.sections.battery`) — pod hodinami v kartě Čas: procenta, nabíjí se / v síti / na baterii,
  zbývající čas jen když ho Windows odhadne. Stroj bez baterie (desktop) neukazuje nic a nic neodebírá.
- **Výkon:** nic nepolluje. Odběry vzniknou až při prvním zobrazení záložky Osobní (Tauri `personal_live`;
  GSMTC navíc hned při startu / uložení configu, když je v zapnutém proužku položka `media`):
  vlákna `media` a `volume` pak jen čekají na události (timeline jen jako událost — hlášení, které jen potvrzuje
  dopočítaný průběh, se zahodí; průběh dopočítává UI a tiká jen s otevřeným šuplíkem při přehrávání), baterie je
  callback `PowerSettingRegisterNotification` bez vlákna a okna. Skrytá karta / vypnutá záložka Osobní odběry
  zruší (GSMTC jen tehdy, když `media` není ani v proužku). Tauri: `personal_live`, `media_favorite_open` (oblíbená položka, viz `media.favorites` v Konfigurace), `media_control` (`"toggle"|"next"|"prev"`, `{"seek": ms}`, `{"shuffle": bool}`, `{"repeat": "none"|"track"|"list"}`), `media_cover`, `volume_set`, `volume_mute`.
- Proužek: položka `strip.items` `media` (výchozí vypnutá; vzhled `strip` ji doplní sám) — `mediaconf::strip_segment`
  (jen při přehrávání, „♪ Název · Interpret“, text ≤ 36 znaků, s `media.stripTime` navíc „· 1/4 min“, u `strip`
  s ozdobou ▂▅▃ místo ♪). Změna skladby / pauza → událost GSMTC → `mark_dirty` → překreslení; čas jen po minutách
  (≤ 1 probuzení vlákna `emit` za minutu při přehrávání), sekundy proužek neukazuje. Ve fullscreen hře / herním režimu je proužek pryč jako vždy.

## GitHub a Systém (karty `github`, `system`; Nastavení → Šuplík a karty → Obsah karet)

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

