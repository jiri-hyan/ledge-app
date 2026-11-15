# Instalace, aktualizace, migrace z Notche

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

## Instalace a aktualizace

- **Vydání (běžná cesta):** `powershell -NoProfile -File scripts/publish.ps1` (na sdíleném stroji přes
  frontu) z čistého, pushnutého `main` vývojového repa. Veřejné repo `jiri-hyan/ledge-app` nedostává
  vývojovou historii: každé vydání = jeden nový commit „snapshot“ (`git archive HEAD`, cesty
  s `export-ignore` v `.gitattributes` se vynechají), autor `jiri-hyan` s noreply e-mailem GitHubu. Strom
  i seznam změn se před commitem prověří proti `scripts/private/denylist.txt` (jména klientů, strojů,
  osobní údaje — soubor je jen ve vývojovém repu); kontrolu dělá `scripts/denylist-check.mjs` (jediná implementace; stejná běží jako brána `denylist` v `brany.mjs`, výstup jen `soubor:řádek ~ vzor #index`); jediný nález = konec bez vydání. Pak build a podpis
  instalátoru v **GitHub Actions veřejného repa** (workflow ze šablony `.github/verejne-repo/`, klíč jen v tajných
  údajích repa, na PC žádný — vydávat jde z kteréhokoli PC; [podpis-vydani.md](../podpis-vydani.md)); skript počká,
  stáhne instalátor + `.sig` a ověří ID klíče proti `src-tauri/signing/update.pub`. Záloha `-Local` = lokální build
  (verze = nejvyšší vydaná v obou repech + 1, zapíše se jen dočasně přes `scripts/ci-version.ps1`) a podpis
  vlastním klíčem mimo repo (bez klíče nebo s cizím klíčem skript skončí ještě před snapshotem)
  a vydání ve veřejném repu **přes koncept** (kompatibilní s immutable releases: koncept → všechny soubory →
  kontrola seznamu → publikace; nepovedený běh nechá neveřejný koncept, další běh vydá stejnou verzi a doplní
  ho): instalátor + `<instalátor>.sig` + přílohy `SHA256SUMS.txt` (`<hex>  <soubor>`) a
  `pro-claude.md` (kopie `docs/pro-claude.md`, návod pro Clauda — „Nastav s Claudem“) + poznámky, první řádek
  `SHA256 (<setup>) = <hex>` (tvar čte `update.rs`, neměnit; řádky začínající `SHA256` se v UI z poznámek
  vypouštějí) a na konci sekce „Nastav s Claudem“ s promptem. Tag se vytvoří i ve vývojovém repu. `-DryRun` = jen snapshot
  a kontrola, `-AlsoPrivate` = stejný instalátor i do vývojového repa (přechod pro Ledge ≤ 0.2.8, které
  hledaly aktualizace tam). Pojistky před vydáním (`scripts/vydani-check.mjs`, logika `vydani-lib.mjs`): razítko plných bran `.brany-ok` musí
  odpovídat stromu HEAD (výjimka jen výslovně `-SkipGateCheck`, zapíše se do `%LOCALAPPDATA%/ledge-publish.log`) a musí existovat
  `docs/releases/<verze>.md` (bez něj skript skončí). Jediný workflow je šablona `.github/verejne-repo/workflows/vydani.yml`
  (starý `release.yml` z roku 2026-10 je smazaný). `install-local.ps1` varuje při downgradu přes vyšší nainstalovanou verzi.
- **Aktualizace z appky** (`src-tauri/src/update.rs` + `updater.rs`, config `update: { check, repo, intervalH, prefetch }`):
  60 s po startu a pak každých `intervalH` h se Ledge zeptá **veřejného GitHub API**
  `GET https://api.github.com/repos/<repo>/releases/latest` (`updatenet.rs`: bez tokenu, User-Agent
  `Ledge/<verze>`, termín 20 s) — **`gh` není potřeba**. Když API selže (soukromé repo → 404, limit
  60 dotazů/h → 403/429, výpadek) a `gh` je nainstalované, zkusí záložně `gh release view --repo <repo>`
  (bez okna); obě odpovědi čte tatáž `update::evaluate`. Stavy ve `Snapshot.update`: `off` · `neovereno`
  · `aktualni` · `dostupna` · `nelze_overit {duvod}` (offline / repo nenalezeno / limit API, s chybou
  zálohy přes gh). Hlavička ukazuje **jen** `dostupna` (chip „Nová verze X“), zbytek Nastavení →
  Záloha a aplikace → Aktualizace (+ „Zkontrolovat teď“). **Nikdy neinstaluje sám.** Stažení
  přes HTTPS jen z `https://github.com/<repo>/releases/download/<tag>/<asset>` (`updatenet::download_ok`,
  strop 200 MB; záložně `gh release download`) do cache `%LOCALAPPDATA%\Ledge\updates\<tag>\` (přes
  `.part`), SHA256 se musí shodovat s poznámkami (a s `digest` GitHubu) — jinak se soubor smaže; vedle
  něj `release.json` (SHA256, poznámky, podpis). **Podpis** (`updsign.rs`): vydání bez `<instalátor>.sig` se
  nenabídne; po stažení musí `.sig` sedět na zapečený veřejný klíč a jmenovat přesně soubor i verzi, jinak se
  instalátor smaže (detail [podpis-vydani.md](../podpis-vydani.md)). Spuštění: SHA256 i podpis se z cache
  ověřují znovu → odpojený `cmd`:
  ~2 s pauza, `setup /S`, start nainstalovaného `ledge.exe` (`autostart::install_target`); Ledge se ukončí.
  Běží-li spravované servery (s Ledge by umřely), `update_install` vrátí jejich seznam a chce druhé
  potvrzení. Předběžné/nevalidní tagy se ignorují.
  - **`update.prefetch`** (výchozí `true`, platí jen s `check: true`): hned po kontrole, která najde novější
    vydání, se instalátor **tiše stáhne a ověří** (jednou na verzi; `Snapshot.update.fetch`: `idle` ·
    `downloading {got, total}` — jen skutečné bajty, `total: null` = neurčitý průběh · `ready` · `failed
    {duvod}`). Chip ukáže „Připraveno X“ a panel „Restartovat a aktualizovat“. Selhání se samo neopakuje
    — „Zkusit znovu“ (`update_retry`). Žádná síť navíc: tytéž hosty a ověření jako dřív, jen dřív v čase.
    `prefetch: false` = postaru, stáhne se až na klik „Stáhnout a nainstalovat“.
  - **Úklid cache** (`updcache::plan_cleanup`, při startu a po stažení): zůstane instalátor běžící verze,
    cíl návratu a nejnovější stažená novější verze; přerušené a starší pryč. Staré `%TEMP%\ledge-update` se smaže.
  - **Co je nového:** `updates\seen.json` (`updseen.rs`) drží naposledy spuštěnou a předchozí verzi. Po
    prvním spuštění nové verze, jejíž poznámky jsou v cache, ukáže hlavička jednorázový chip „Co je nového
    v X“ (bez sítě, bez fokusu; „Rozumím“ = `update_notes_read`), trvale v Nastavení → Aktualizace.
    Markdown poznámek se vykreslí jako text (`src/update/model.ts`, žádné HTML ani klikací odkazy z vydání).
  - **Vrátit předchozí verzi** (Nastavení → Aktualizace, s potvrzením; `update_rollback`): ověřený
    instalátor předchozí verze z cache, a když tam není (ruční instalace, první aktualizace touto cestou),
    na klik `GET /repos/<repo>/releases/tags/v<předchozí>` + stažení se stejným ověřením. SHA256 každého
    ověřeného instalátoru si Ledge pamatuje u verze v `seen.json` (`known`, mimo cache) — vydání přepsané
    na GitHubu jiným instalátorem návrat odmítne. Návrat neběží, dokud se stahuje nová verze; úklid
    nesmaže složku běžícího stažení a spouští jen `*-setup.exe` z manifestu (`update::safe_asset`). Downgrade umí
    šablona NSIS Tauri v tichém režimu, dokud `bundle.windows.allowDowngrades` není `false` (výchozí `true`,
    v `tauri.conf.json` nenastaveno — hlídá test `nsis_allows_silent_downgrade`). Po návratu na starší
    verzi nic dalšího staršího nenabízí; zpět na novou vede běžná aktualizace (instalátor zůstal v cache).
- **Lokální vydání:** `powershell -NoProfile -File scripts/release.ps1` → zkontroluje, že `version` v
  `src-tauri/tauri.conf.json` a `src-tauri/Cargo.toml` souhlasí, pustí `pnpm tauri build --bundles nsis`
  a vypíše cestu, velikost a SHA256 `Ledge_<verze>_x64-setup.exe` (`<target>\release\bundle\nsis\`).
  `-Install` navíc nainstaluje tiše (`/S`) a nainstalovaný Ledge spustí — ale jen když Ledge neběží;
  jinak řekne „zavři Ledge“ a skončí (tichý instalátor by běžící kopii sám násilně ukončil).
- **Bez cest uživatele v binárce:** release build jde přes `scripts/remap-flags.mjs --run -- <příkaz>` (doplní `RUSTFLAGS` o
  `--remap-path-prefix` pro cargo, rustup, repo, target a profil; `release.ps1` to dělá sám, ruční QA build:
  `node scripts/remap-flags.mjs --run -- corepack pnpm tauri build --no-bundle`; změní jen buildy s touto proměnnou, ne
  debug/test cache). `node scripts/denylist-check.mjs --exe <ledge.exe>` pak projde binárku (ASCII i UTF-16LE, po blocích) proti
  denylistu a obecnému vzoru `X:\Users\<jméno>\` (výstup jen offset, kódování, vzor); `publish.ps1` ji pouští před podpisem,
  do běžných bran nepatří. Známé omezení `--tree`/`--dir`: obsah obrázků a médií se neprohledává (PNG, ICO, ICNS, JPG, WebP, BMP
  vůbec; GIF, WebM, MP4, WOFF se čtou jako text, takže zachytí jen nekomprimované ASCII) — kontrolují se jen jejich jména.
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
  významem; odškrtnuté smaže `%APPDATA%\Ledge`, `%LOCALAPPDATA%\Ledge`, `…\app.ledge.desktop` (WebView2: localStorage,
  cookies oken „web“, cache), profil starého identifikátoru (maska `cz.*.ledge`, jen s podsložkou `EBWebView`)
  (ponechaný migrací; rozdělaná kopie `EBWebView.migrating` je uvnitř nové složky a mizí s ní); poznámky v `%USERPROFILE%\Ledge` nikdy.
  Tichý běh (`/S`, aktualizace z Ledge) nic z toho nemění.
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
- **Autostart v bublině** (`autostart::gate`): Run se nezapisuje ani nemaže; Nastavení → Aplikace → Spouštění a hry →
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


## Změna identifikátoru a vydavatele (0.16.0)

Od 0.16.0 je `identifier` = `app.ledge.desktop` a vydavatel = „Ledge“ (`bundle.publisher`, výslovně). Starý identifikátor
(vydání ≤ 0.15.2) se v repu nepíše; hledá se **maskou** `cz.*.ledge` + podsložka `EBWebView`. Po vydání se identifikátor
už **nemění** (každá změna = další migrace). Test tvaru: `scripts/installer-hooks.test.mjs`.

| Co závisí na `identifier` | Kde | Při změně |
|---|---|---|
| Složka dat WebView2 `%LOCALAPPDATA%\<id>\EBWebView` (všechna okna, localStorage, cookies oken „web“) | Tauri `app_local_data_dir` | profil zkopíruje `webdata.rs` při prvním startu (jádro) |
| Mutex single-instance `<id>-sim` | `lib.rs` | stará a nová verze se nevidí → hlídač přes port mostu 47800 (jádro) |
| AppUserModelID toastů a zástupce ve Startu | `notify/*`, instalátor | zástupce přepíše tichá instalace (`/S`); **nastavení oznámení Windows pro Ledge se resetuje** |
| Vydavatel → klíč `HKCU\Software\<vydavatel>\Ledge` | šablona NSIS | `hooks.nsh`: dřívější složka instalace + úklid osiřelého klíče |
| Odinstalace „smazat data“: `$APPDATA\<id>`, `$LOCALAPPDATA\<id>` | šablona NSIS | `hooks.nsh` navíc maže profil starého identifikátoru (maska) |
| QA pojistka sdílené složky dat WebView2 | `scripts/qa/lib.mjs` `dataDirVerdict` | zná nový tvar i starý (maska), hlásí „SKUTEČNÝ PROFIL nainstalované kopie“ |

**Nezávisí na něm:** config (`%APPDATA%\Ledge`), poznámky (`%USERPROFILE%\Ledge`), klíče ve Správci pověření, klíč
odinstalace `Uninstall\Ledge`, hodnota Run `Ledge`, updater, `--ext`.

**Instalátor (`hooks.nsh`)**
- `PREINSTALL`: je-li složka instalace výchozí a nový klíč výrobce chybí, použije se dřívější složka z
  `Uninstall\Ledge\InstallLocation` (nebo ze starého klíče vydavatele) — jen když tam leží `ledge.exe`; jinak by instalace
  přes starší verzi s vlastní složkou vytvořila druhou kopii.
- `POSTINSTALL`: osiřelý klíč starého vydavatele se najde **bez jména** — podklíč `HKCU\Software\*\Ledge`, jehož výchozí hodnota
  je složka s `ledge.exe` a který není klíč nového vydavatele; smaže se on a (jen je-li prázdný) jeho rodič. Stroj bez starého
  klíče (čistá instalace, už uklizeno) se nezmění.
- `POSTUNINSTALL` s „smazat data“ (ne při aktualizaci): navíc složky `%LOCALAPPDATA%\cz.*.ledge` s podsložkou `EBWebView`
  (profil starého identifikátoru, který migrace záměrně ponechává); prostřední segment musí splnit stejné pravidlo jako
  `is_legacy_name` (`a-z 0-9 - .`), takže cizí `cz.a b.ledge` zůstane. Klíč starého vydavatele se při odinstalaci nehledá
  (opírá se o `ledge.exe`, který šablona už smazala); uklízí ho `POSTINSTALL`.
- Úvodní stránka průvodce (`LedgeWelcomePre`, přepsané `MUI_PAGE_WELCOME`): chybí-li nový klíč výrobce a je dřívější instalace,
  zapíše ho s její složkou a nabídne ji jako výchozí. Bez toho by „odinstalovat před instalací“ (`PageLeaveReinstall` čte
  `_?=` z nového klíče) předalo starému odinstalátoru prázdnou cestu. Výběr na stránce Složka už se nepřepisuje; `PREINSTALL`
  řeší jen tichou instalaci bez stránek (`$LedgePagesRan`). Vedlejší efekt: zrušený průvodce nechá nový klíč ukazovat na
  stávající instalaci (neškodné).
- **Strop opakování migrace profilu** (`webdata.rs`): `.migrate-deferred` = JSON `{tries, first}`; započítává se každý pokus
  (odklad při živé staré instanci i kopie, včetně selhání, takže selhání zapíše značku a „příště znovu“ platí). Po 3 pokusech
  NEBO 7 dnech od prvního se migrace vzdá: `.migrated-from` = `{from:null, reason:"vzdáno", tries, ver, at}`, nový profil se
  už neodsouvá a start nečeká (čekání 15 s nejvýš 3×). Odsunutí vzniklého profilu v rámci stropu se loguje řádkem „POZOR“.
- `release.ps1` před buildem maže obsah `src-tauri\permissions\autogenerated` a `src-tauri\gen\schemas` (negitové, generuje
  `tauri-build`; osiřelé po odstraněných příkazech by se zabalily do ACL binárky).
- Kompilace instalátoru se v kole 35 neověřovala (`makensis` se nespouštěl): první sestavení + ruční zkouška níže je nutná.

**QA bez čistého stroje:** `node scripts/qa.mjs --only migrace-id` založí v sandboxu atrapu `cz.acme.ledge\EBWebView` a po startu
izolované instance ověří (nad falešným `%LOCALAPPDATA%`): stará složka nedotčená, v nové jen soubory stavu (ne cache), žádný
zbytek `EBWebView.migrating` (uvnitř nové složky), značka `.migrated-from`, žádný `.migrate-deferred` (po vzdání má značka `from:null`, viz strop výše). Pokud izolovaná instance migraci nespouští, krok je SKIP (ne PASS). Logiku kroků
testuje `scripts/qa-migrace.test.mjs`.

**QA skutečného přenosu profilu:** `node scripts/qa.mjs --exe <nový ledge.exe> --old-exe <build se starým identifikátorem> --only upgrade` spustí obě binárky PO SOBĚ
nad týmž sandboxem (`<cfg>/webview2` je junction na složku dat podle identifikátoru; bez toho by izolace s CDP migrovaný profil nečetla). A: stará zapíše
localStorage + cookie; B: nová migruje a hodnoty MUSÍ přečíst (+ počty souborů/bajtů z logu, stará nedotčená, cache nepřenesená, značka); C: druhý start bez opakování;
D: odklad při zámku profilu (D1/D2 držák `FileShare.None`, D3 zámek jako živý Chromium). Bez `--old-exe` je scénář SKIP.

### Kontrolní seznam ruční zkoušky na čistém stroji (VM, ne stroj s Windows 11 Home bez virtualizace)

Příprava: čistý uživatel Windows, instalátor 0.15.2 a nový (`Ledge_*_x64-setup.exe`). Před každým krokem si poznamenej, co v
`%LOCALAPPDATA%` a v `HKCU\Software` je. Zkouška smí běžet jen tam, kde nehraješ (QUNS).

| # | Postup | Očekávaný výsledek |
|---|---|---|
| 1 | Nainstaluj 0.15.2, spusť, v okně typu `web` se přihlas k libovolné stránce, sbal kartu, zavři Ledge | vznikne `%LOCALAPPDATA%\cz.*.ledge\EBWebView` (jedna složka), `HKCU\Software\<starý vydavatel>\Ledge` |
| 2 | Spusť nový instalátor tiše přes starou: `Ledge_*_x64-setup.exe /S` | jedna položka Ledge v Apps & features, vydavatel „Ledge“, nová verze; `HKCU\Software\Ledge\Ledge` existuje, klíč starého vydavatele zmizel i s prázdným rodičem |
| 3 | Spusť Ledge | vznikne `%LOCALAPPDATA%\app.ledge.desktop\EBWebView` se značkou `.migrated-from`; sbalená karta zůstala, přihlášení v okně `web` platí; stará složka je beze změny; v `ledge.log` řádek o migraci |
| 4 | Zástupce ve Startu a toast: Nastavení → Claude → oznámení Windows (nebo schůzka s Připojit) | zástupce existuje; toast dorazí; nastavení oznámení pro Ledge ve Windows je výchozí (jednorázový reset, jak říkají poznámky k vydání) |
| 5 | Nová čistá instalace 0.15.2 do **vlastní složky** (např. `D:\Apps\Ledge`), pak nový instalátor `/S` | instaluje se do téže `D:\Apps\Ledge`, **žádná** druhá kopie v `%LOCALAPPDATA%\Ledge` (kromě dat), jedna položka v Apps & features |
| 6 | Totéž bez `/S` (průvodce) s vlastní složkou | průvodce nabídne výchozí složku, ale instaluje do dřívější; po dokončení jedna kopie |
| 7 | Souběh: spusť 0.15.2 (nezavřenou) a pak nový Ledge; potom naopak | nová verze počká nejvýš 15 s na konec staré a jede dál (profil se migruje až po zavření staré); starou nikdy nezabije; spuštění staré při běžící nové = druhé spuštění (nic nezabije) |
| 8 | Aktualizace z aplikace (Nastavení → Záloha a verze) z 0.15.2 na novou | stejné jako 2–3, instalátor zavře starou instanci sám |
| 9 | Návrat: „Vrátit předchozí verzi“ → 0.15.2 | 0.15.2 používá starou složku (stav k okamžiku migrace); nový profil zůstane; po dalším upgradu značka → bez nové kopie (nový profil má přednost) |
| 10 | Odinstalace s ponecháním nastavení (výchozí) | zmizí program a klíč odinstalace (klíč výrobce `Software\<vydavatel>\Ledge` zůstane, maže se jen při „smazat data“); složky `%LOCALAPPDATA%\app.ledge.desktop`, `cz.*.ledge`, `%APPDATA%\Ledge`, `%USERPROFILE%\Ledge` zůstanou |
| 11 | Odinstalace s odškrtnutým „Ponechat nastavení“ | zmizí `app.ledge.desktop` (včetně `EBWebView.migrating`), `cz.*.ledge`, `%APPDATA%\Ledge`, `%LOCALAPPDATA%\Ledge`; `%USERPROFILE%\Ledge` (poznámky) zůstane |
| 12 | Čistý stroj bez dřívější verze: nový instalátor `/S`, pak odinstalace | nic se nehlásí, žádná chyba, žádná mazaná složka mimo Ledge (na stroji bez starých klíčů se hook nic nedotkne) |
| 13 | Dva profily staré složky (zkopíruj `cz.*.ledge` pod jiným názvem `cz.acme.ledge`) a spusť nový Ledge | víc zdrojů: vybere se nejnovější podle času změny a zapíše do logu; nic se nesmaže |

Nejde ověřit mimo čistý stroj: kompilace a běh hooků NSIS (3 makra), skutečné přečtení zkopírovaného localStorage a cookies
WebView2 nad výchozí složkou, chování `lockfile` Chromia u běžící staré instance, souběh dvou verzí, zástupce s novým AUMID, toast
a nastavení oznámení Windows, instalace do vlastní složky.

## QA před vydáním

`node scripts/qa.mjs` (po `cargo build` / `publish.ps1`; kódy 0 = bez FAIL, 1 = FAIL, 2 = nespustitelné): druhá, izolovaná
instance Ledge (`LEDGE_ISOLATED=1`, `LEDGE_CDP_PORT`) v dočasné složce s fixture daty (port API 47811, CDP 47812, zkratky a
proužek vypnuté), WebView řízené přes CDP, `GET /status` (blok `qa`), zpráva `.qa/<razítko>/report.md` + `report.json` (PASS/FAIL/SKIP
s důkazem a snímky obsahu WebView). Scénáře `--list`: `supik`, `listky-l260` (+ `-zkratky`), `kit-*`, `hudba`, `nastaveni`, `propojeni`, `migrace-id`, `upgrade` (vyžaduje `--old-exe`).
Nepustí se, když uživatel hraje, když je port obsazený, nebo když binárka nezná izolaci. Nesahá na nainstalovanou instanci
(47800) ani na skutečná data; klíč API čte jen ze `~/.claude/settings.json` a nevypisuje ho. Přes frontu:
`node %USERPROFILE%\.devtools\fronta.mjs --auto -- node scripts/qa.mjs`.

Pojistky a ladění (L297, L299, L300): po startu instance se z příkazových řádek jejích potomků `msedgewebview2.exe` ověří, že
`--user-data-dir` leží uvnitř sandboxu (jinak se instance hned ukončí a skript skončí kódem 2). Před každým snímkem se počká na
doběhnutí konečných animací (strop 3 s, `document.getAnimations()`; krok `animace doběhly`, nedoběhlé se dokončí násilně a krok padne).
Chyba konzole nese přesný zdroj CSP (`securitypolicyviolation`: direktiva, soubor:řádek, poslední zápisy stylu se zásobníkem) ve zprávě
i v „Po cestě“.
