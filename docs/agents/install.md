# Instalace, aktualizace, migrace z Notche

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

## Instalace a aktualizace

- **Vydání (běžná cesta):** `powershell -NoProfile -File scripts/publish.ps1` (na sdíleném stroji přes
  frontu) z čistého, pushnutého `main` vývojového repa. Veřejné repo `jiri-hyan/ledge-app` nedostává
  vývojovou historii: každé vydání = jeden nový commit „snapshot“ (`git archive HEAD`, cesty
  s `export-ignore` v `.gitattributes` se vynechají), autor `jiri-hyan` s noreply e-mailem GitHubu. Strom
  i seznam změn se před commitem prověří proti `scripts/private/denylist.txt` (jména klientů, strojů,
  osobní údaje — soubor je jen ve vývojovém repu); jediný nález = konec bez vydání. Pak lokální build
  (verze = nejvyšší `v0.2.x` v obou repech + 1, zapíše se jen dočasně přes `scripts/ci-version.ps1`)
  a podpis instalátoru (`tauri signer sign`, klíč mimo repo — [podpis-vydani.md](../podpis-vydani.md); bez klíče nebo
  s klíčem, jehož veřejná půlka není `src-tauri/signing/update.pub`, skript skončí ještě před snapshotem)
  a vydání ve veřejném repu **přes koncept** (kompatibilní s immutable releases: koncept → všechny soubory →
  kontrola seznamu → publikace; nepovedený běh nechá neveřejný koncept, další běh vydá stejnou verzi a doplní
  ho): instalátor + `<instalátor>.sig` + přílohy `SHA256SUMS.txt` (`<hex>  <soubor>`) a
  `pro-claude.md` (kopie `docs/pro-claude.md`, návod pro Clauda — „Nastav s Claudem“) + poznámky, první řádek
  `SHA256 (<setup>) = <hex>` (tvar čte `update.rs`, neměnit; řádky začínající `SHA256` se v UI z poznámek
  vypouštějí) a na konci sekce „Nastav s Claudem“ s promptem. Tag se vytvoří i ve vývojovém repu. `-DryRun` = jen snapshot
  a kontrola, `-AlsoPrivate` = stejný instalátor i do vývojového repa (přechod pro Ledge ≤ 0.2.8, které
  hledaly aktualizace tam). CI (`.github/workflows/release.yml`) je jen ruční a do veřejného repa se
  nekopíruje.
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
  významem; odškrtnuté smaže `%APPDATA%\Ledge`, `%LOCALAPPDATA%\Ledge` a `…\cz.hyan.ledge` (WebView2),
  poznámky v `%USERPROFILE%\Ledge` nikdy. Tichý běh (`/S`, aktualizace z Ledge) nic z toho nemění.
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

