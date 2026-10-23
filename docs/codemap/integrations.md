# Codemapa: integrace — karty GitHub a Systém

Dvě karty z katalogu rozložení (`layout::CARDS`, ale `layout::OPT_IN` = na žádné záložce ve výchozím rozložení),
dvě položky proužku (`github`, `system`, výchozí vypnuté) a jeden malý plánovač. Společné pravidlo vlny 8:
**výchozí vypnuto, žádná síť bez zapnutí, drahé věci jen když je karta vidět, s termínem a mezí frekvence.**
Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/sysmon/calc.rs` | čisté výpočty (tabulkové testy): CPU % ze dvou vzorků, rychlost z čítačů, stáří vzorku, `Ring` (60), výběr disků, text a úroveň položky proužku | `cpu_pct` `rate` `fresh` `Ring` `HISTORY` `STALE_MS` `pick_disks` `normalize_letters` `mask_letters` `strip_text` `level` |
| `src-tauri/src/sysmon/win.rs` | WinAPI čtečky bez WMI: `GetSystemTimes`, `GlobalMemoryStatusEx`, `GetLogicalDrives` + `GetDriveTypeW` (jen `DRIVE_FIXED`) + `GetDiskFreeSpaceExW`, `GetIfTable2` (jen hardwarové adaptéry nahoře, bez loopbacku) | `cpu_times` `memory` `fixed_disks` `net_totals` |
| `src-tauri/src/sysmon/conf.rs` | config `system` (`disks`, `netUnit`), validace | `SystemConfig` `NetUnit` `validate` |
| `src-tauri/src/sysmon/mod.rs` | stav (předchozí čítače, poslední vzorek, historie), `poll` (vzorek nejvýš 1× za 1,5 s, čtení mimo zámek, disky 1× za 30 s, stará historie po pauze pryč), `strip_now`, `next_strip_due`, `open_task_manager` | `SystemView` `poll` `strip_now` `next_strip_due` |
| `src-tauri/src/github/parse.rs` | parsování JSON z `gh` (shovívavé, adresy jen `https://github.com/`), převod API adresy notifikace na webovou; vzorky `src/fixtures/gh-*.json` | `parse_search` `parse_mine` `parse_notifications` `web_url` `ci_of` `safe_url` `Pr` `Notif` |
| `src-tauri/src/github/run.rs` | spuštění `gh` bez okna s termínem 15 s (`gitinfo::capture`, `update::gh_exe`), třídění chyb ze stderr | `run` `classify` `GhErr` |
| `src-tauri/src/github/conf.rs` | config `github` (`enabled` výchozí `false`, `review`/`mine`/`notifications`, `refreshMin` 2–60) | `GithubConfig` `validate` `interval_ms` `any` |
| `src-tauri/src/github/mod.rs` | stav v paměti, `due` (kdy smí obnova: karta / plánovač / Obnovit), `fetch` (3 dotazy souběžně), `apply` (stav a počty z výsledků), `refresh`, `next_due`, `on_config`, `strip_now`, `open_ok`; `live_gh_smoke` (ruční, `--ignored`) | `GithubView` `Status` `Mode` `due` `apply` `refresh` |
| `src-tauri/src/integrations.rs` | plánovač položek proužku: `wants` (položka v `strip.items` + proužek zapnutý; GitHub navíc `enabled` a `review`), `plan` (čistá), vlákno `integrations` vzniká v `sync` jen s položkou a samo skončí; ve fullscreen hře nic | `Want` `wants` `plan` `sync` `SYSTEM_EVERY_MS` |
| `src-tauri/src/commands/integrations.rs` | Tauri: `system_view`, `system_taskmgr`, `github_view(force)`, `github_open`, `github_copy_login` (schránka přes Rust — šuplík nemá fokus) | |
| `src/integrations/format.ts` | čistá logika UI (test `format.test.mjs`): `fmtSize`, `fmtRate` (bajty / bity), `fmtPct`, `sparkPoints`, `loadOf`, `diskLoad`, `toggleDisk`, `CI_TONE`, `isoMs` | |
| `src/integrations/api.ts` | typy `SystemView` / `GithubView` a `intApi` (vlastní, aby se nesahalo do `types.ts` / `api.ts`) | `intApi` |
| `src/integrations/SystemCard.tsx` | karta Systém: CPU, paměť, disky, síť + sparkline; polling 2 s jen s `drawerShown()`; klik na CPU / paměť → Správce úloh | `SystemCard` |
| `src/integrations/GithubCard.tsx` | karta GitHub: sekce a prázdné stavy (vypnuto / bez `gh` / nepřihlášeno / chyba), smyčka podle `nextInMs` jen s viditelnou kartou | `GithubCard` |
| `src/integrations/IntegrationsSettings.tsx` | Nastavení → Osobní → GitHub a Systém (přepínač s větou, sekce, frekvence, disky jako dlaždice, jednotky) | `GithubSettings` `SystemSettings` |
| `src/integrations/integrations.css` · `src/i18n/en-integrations.ts` | styly (`sys-*`, `gh-*`, `int-*`) · angličtina `cards.github.*`, `cards.system.*`, `settings.github.*`, `settings.f.github.*`… | |

Napojení (jednořádkové): `config/*` (`Config.github` / `.system`, `sections!` `github` / `system` s výchozím `false`
přes `layout::OPT_IN`, validace), `layout.rs` (`CARDS`, `OPT_IN`), `stripconf.rs` + `stripview.rs` (`StripItem::Github` /
`System`; `Github` je „událost“ jako Čeká, `System` stálá), `settings.rs` `apply_config` (`github::on_config` +
`integrations::sync`), `lib.rs` (moduly, příkazy, `integrations::sync` při startu), `update.rs` (`gh_exe` je `pub(crate)`,
hledá i WinGet Links), `Cargo.toml` (`Win32_NetworkManagement_Ndis` pro `MIB_IF_ROW2`); UI: `layout/model.ts`
(`CARD_IDS`, `CARD_INFO`), `MainPage.tsx` (`case "github"` / `"system"`), `cards/sections.ts`, `settings/{labels,model,strip}.ts`,
`panels/{Personal,StripSettings,Backup}.tsx`.

## Toky

- **Návrhy a puls z GitHubu:** po každém načtení (`refresh`, po uvolnění zámku) `suggest::on_github_refreshed` → `pulse::on_refreshed`
  (nové PR k revizi / nově spadlé CI na mém PR → `attention::signal`, jen když `suggest.githubPulse`; první pozorování jen založí
  paměť) a `mark_dirty` (pruh návrhů čte `github::cached_prs`). Žádný dotaz navíc — [suggest.md](suggest.md).
- **Systém, karta:** `SystemCard` (`drawerShown()`) → `system_view` po 2 s → `sysmon::poll(record=true)` → `SystemView`
  (filtrované disky podle `system.disks`, `allDisks` pro Nastavení). Zavřený šuplík = žádný časovač, žádné čtení.
- **Systém, proužek:** položka `system` v zapnutém proužku → `integrations::sync` spustí vlákno → `poll(record=false)` po
  10 s (vzorky karty posouvají termín) → `mark_dirty` → `refresh_strip` → `sysmon::strip_now` („CPU 23 % · RAM 61 %“,
  tečka jantarová od 75 %, červená od 90 %).
- **GitHub, karta:** `GithubCard` → `github_view(false)` → `refresh(Mode::Card)`: když `due`, spustí tři `gh` souběžně (≤ 15 s),
  jinak vrátí paměť; UI si podle `nextInMs` naplánuje další volání (jen s otevřeným šuplíkem). Obnovit = `Mode::Force`
  (nejdřív po 10 s). Po `NoGh` / `Login` smí karta zkusit znovu po 15 s (uživatel to opravuje), plánovač ne.
- **GitHub, proužek:** položka `github` + `github.enabled` + `review` → plánovač volá `refresh(Mode::Auto)` po `refreshMin`
  → segment „GitHub: N× čeká na revizi“ jen když N > 0 (je to „událost“, vytáhne proužek z klidu).
- **Config:** `apply_config` → `github::on_config` (změna = další obnova smí hned, vypnutí paměť vyprázdní) → `integrations::sync`.

## Invarianty a pasti

- Dotazy na GitHub **nikdy** bez `github.enabled`; token Ledge nevidí (jen `gh`); URL otevřené z UI jen `https://github.com/…`.
- `gh` z Claude session může žít v „bublině“ MSIX — Ledge běží mimo ni, `gh_exe` hledá v `PATH` procesu Ledge a na obvyklých místech.
- `gh` argumenty jsou pole (žádný shell); GraphQL dotaz je jeden argument. Výstup `gh search prs` nemá `statusCheckRollup`,
  proto vlastní PR jdou přes GraphQL `viewer.pullRequests`.
- Zámky: plánovač čte config POD zámkem `SCHED` (ztráta probuzení), proto `sync` nikdy nevolej s drženým `cfg.read()` — předej klon.
- Sparkline kreslí `<svg>` s `vector-effect: non-scaling-stroke`; stará historie (> 30 s pauza) se zahazuje, ať osa nelže.
- Karta bez dat se neschovává (`shownCards` ji ukáže vždy) — prázdné stavy jsou její obsah.

## Testy

Rust: `sysmon::calc` (CPU %, rychlost, stáří vzorku, `Ring`, disky, text proužku), `sysmon::win` (skutečné čtení),
`sysmon::conf`, `github::parse` (vzorky `gh-search-prs.json`, `gh-graphql-viewer-prs.json`, `gh-notifications.json`, cizí
adresy, rozbitý vstup), `github::run::classify`, `github` (`due`, `apply`, jen github.com), `github::conf`,
`integrations` (`plan`, `wants`, hodina = 360 vzorků), `layout`/`config` (opt-in karty mimo výchozí rozložení).
UI: `src/integrations/format.test.mjs`, `src/layout/model.test.mjs` (katalog), hlídače překladů a CSS tříd.
Živě (ručně): `cargo test live_gh_smoke -- --ignored --nocapture`; vzhled karet a Správce úloh jen v běžícím Ledge.
