# Ledge

**Šuplík u horního okraje obrazovky Windows, který je pořád po ruce** — projekty a dev servery, všechny
běžící session Claude Code, poznámky a rychlé akce na jeden pohyb myší. Nikdy nebere fokus a při hře přes
celou obrazovku se odmlčí.

[English](README.md) · [Stáhnout poslední verzi](https://github.com/jiri-hyan/ledge-app/releases/latest) ·
[Pro agenty](AGENTS.md)

![Ledge — šuplík](docs/screenshots/drawer.png)

## Co umí

- **Projekty.** Dlaždice pro každý projekt: klik spustí dev server (`npm run dev`, port, log) nebo
  aplikaci. Logo si Ledge najde v projektu sám (favicon, web manifest, ikona aplikace). Běžící servery
  mají vlastní kartu s Otevřít / Zastavit; projekt, který na tomhle PC není, jde jedním klikem naklonovat.
  Upozornění na neuloženou nebo nepushnutou práci jde u jednotlivých rep ztišit nebo označit jako přečtené;
  zástupce a složky převezme z katalogu Launcheru.
- **Session Claude Code.** Všechny session Claude Code na PC — desktopová appka i CLI, včetně subagentů —
  se stavem: čeká na tebe, pracuje, nečinná, nedokončená práce. Klik přepne do jejího okna (skončenou
  session obnoví v terminálu).
- **Schvalování a otázky Clauda.** Žádosti o povolení a otázky `AskUserQuestion` se ukážou v šuplíku
  (náhled se otevře bez fokusu). Odpovídáš klikem — jedna volba, víc voleb nebo vlastní text. Povolit smí
  jen člověk; API pro schvalování záměrně neexistuje.
- **Poznámky.** Rychlé poznámky v Markdownu s kategoriemi, úkoly (`- [ ]`), obrázky, připomínkami
  a zařazením k projektu. Obyčejné soubory `.md` ve tvé složce; volitelná synchronizace mezi PC přes tvé
  vlastní soukromé git repo.
- **Rychlé akce a paleta příkazů.** Dlaždice pro weby, složky, soubory, aplikace a příkazy, všechno
  dohledatelné přes `Ctrl+K`. Příkazy a aplikace s argumenty se vždy zeptají, dokud jim v Nastavení
  nedáš souhlas.
- **Rozložení po svém.** Karty na vlastních záložkách ve velikostech S/M/L/XL s nastavitelnou výškou; podržením
  hlavičky karty (nebo ✎) se otevře režim úprav — tah karet i mezi záložkami, změna velikosti za okraj, Zpět
  (Ctrl+Z). Projekty, Repa a Poznámky umí i zobrazení Bento (velká dlaždice a menší kolem).
- **Záložka Osobní.** Hodiny, počasí (Open-Meteo), přehrávač médií (hlasitost, ztlumení mikrofonu, výběr
  zvukového výstupu, volitelně barva podle obalu alba) a tvoje odkazy, odděleně od práce.
- **Vzhled.** 13 hotových design stylů (Klasický, Sklo, Terminál, Papír, Neon, Brutal, Pastel, Polární záře,
  Mono, Retro, Fluent, OLED, Teplý), 16 barevných motivů a vlastní motivy, barva akcentu, hustota, velikost
  písma, zaoblení, průhlednost, 9 animací otevírání (mimo jiné Tekutá kapsle ve stylu Dynamic Island) a jejich
  rychlost, styl a pozice proužku.
- **Průvodce.** Při prvním spuštění najde tvoje repa, nabídne aplikace ze Start menu, zapojí hooky
  Claude Code a nastaví jazyk a motiv. Nic není předvyplněné cizími daty.
- **Čeština a angličtina**, přepínání v Nastavení → Vzhled a animace.

**Claude se ptá, ty odpovídáš ze šuplíku** — jedna volba, víc voleb nebo vlastní text:

![Otázka Claude Code v šuplíku](docs/screenshots/question.png)

**Nastavení** — všechno jde nastavit, s hledáním a živým náhledem vzhledu:

![Nastavení — Vzhled a animace](docs/screenshots/settings.png)

## Instalace

1. Stáhni `Ledge_<verze>_x64-setup.exe` z
   [Releases](https://github.com/jiri-hyan/ledge-app/releases/latest) a spusť ho. Instaluje se pro
   aktuálního uživatele do `%LOCALAPPDATA%\Ledge` — bez práv správce.
2. Windows SmartScreen může upozornit na neznámého vydavatele (instalátor není podepsaný):
   **Další informace → Přesto spustit**. Každé vydání uvádí SHA256 instalátoru pro kontrolu.
3. Najeď myší k hornímu okraji obrazovky. Při prvním spuštění se otevře průvodce.

Požadavky: Windows 10 nebo 11 (x64) s Microsoft Edge WebView2 (ve Windows 11 je předinstalovaný).
Claude Code je volitelný — bez něj je Ledge spouštěč projektů, poznámky a rychlé akce.

**Aktualizace.** Ledge kontroluje veřejná vydání tohoto repa a ukáže čip „Nová verze“. Nikdy se
neinstaluje sám: klikneš, stáhne instalátor z GitHubu přes HTTPS, ověří SHA256 a podpis vydavatele (klíč zabudovaný v Ledge) a teprve pak ho spustí.

**Nastavení s Claudem.** Pošli Claudovi odkaz
`https://github.com/jiri-hyan/ledge-app/releases/latest/download/pro-claude.md` („Nainstaluj a nastav mi Ledge
podle tohoto návodu“). Claude Ledge nainstaluje, připojí se k němu přes MCP, zeptá se, co používáš, a po tvém
schválení nastaví zástupce, odkazy a rozložení na míru. Připojení je vypnuté, dokud ho nezapneš v průvodci nebo
v Nastavení → Claude (MCP), a jde kdykoli vypnout. Návod je v [`docs/pro-claude.md`](docs/pro-claude.md).

**Odinstalace.** Nastavení Windows → Aplikace → Ledge. Config (`%APPDATA%\Ledge`) i poznámky zůstanou.

## Nastavení

Všechno jde nastavit v Nastavení (ozubené kolo v šuplíku) — projekty s autodetekcí a řazením přetažením,
chování šuplíku a proužku, Claude Code a hooky, poznámky a sync, rychlé akce, zkratky, vzhled a jazyk,
záloha (export / import s výběrem částí / obnova výchozích) a aktualizace. Nastavení má hledání
(`Ctrl+F`) a chyby ukazuje přímo u pole.

Pod kapotou je to jeden soubor `%APPDATA%\Ledge\config.json` s vygenerovaným `config.schema.json`
vedle. Změny platí hned. Nevalidní soubor se nikdy neaplikuje ani potichu nepřepíše — Ledge drží
poslední dobrou verzi a před každým zápisem rozbitý soubor zazálohuje.

## Soukromí

- **Žádná telemetrie, žádný účet, žádný cloud.** Na síť Ledge chodí jen kvůli:
  kontrole aktualizací (`api.github.com`, vydání tohoto repa, výchozí každých 6 h — jde vypnout),
  počasí (`api.open-meteo.com`, jen když nastavíš místo), jednorázovému stažení faviconu u akce typu web
  a git synchronizaci poznámek (jen na remote, který sám nastavíš).
- Lokální API poslouchá jen na `127.0.0.1` a požadavky z prohlížeče odmítá.
- Napojení na Claude Code čte seznam session z `claude agents` a hooky přijímá lokálně. Ledge sám nikdy
  nic neschválí.
- Poznámky jsou obyčejné soubory ve tvé složce (výchozí `%USERPROFILE%\Ledge\notes`).

## Pro agenty

Ledge je stavěný tak, aby ho mohli nastavovat i AI agenti. [`AGENTS.md`](AGENTS.md) popisuje config,
formát poznámek, lokální API a pravidla (agent smí přidávat projekty, akce a poznámky, schvalovat smí jen
člověk). Mapy kódu pro přispěvatele jsou v [`docs/codemap/`](docs/codemap/README.md).

## Sestavení ze zdrojů

```
corepack pnpm install
corepack pnpm tauri dev          # vývoj
cd src-tauri && cargo test       # testy Rustu
corepack pnpm test:ui            # testy logiky UI
powershell -File scripts/release.ps1   # instalátor NSIS
```

Technologie: Tauri 2, Rust, SolidJS, TypeScript. Výkonový rozpočet: nečinné CPU pod 0,1 %, `ledge.exe`
pod 25 MB soukromé paměti (měří `scripts/perf.ps1`).

## Licence

[MIT](LICENSE)
