# Codemapa: integrace — karty GitHub a Systém

Dvě karty z katalogu rozložení (`layout::CARDS`, ale `layout::OPT_IN` = na žádné záložce ve výchozím rozložení), dvě položky proužku (`github`, `system`, výchozí vypnuté) a jeden malý plánovač. Společné pravidlo vlny 8: výchozí vypnuto, žádná síť bez zapnutí, drahé věci jen když je karta vidět, s termínem a mezí frekvence …

Detail (toky, invarianty, testy, co nečíst, historie): [integrations-detail.md](integrations-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/sysmon/calc.rs` | čisté výpočty (tabulkové testy): CPU % ze dvou vzorků, rychlost z čítačů, stáří vzorku, `Ring` (60), výběr disků, text a úroveň položky proužku | `cpu_pct` `rate` `fresh` `Ring` `HISTORY` `STALE_MS` `pick_disks` `normalize_letters` |
| `src-tauri/src/sysmon/win.rs` | WinAPI čtečky bez WMI: `GetSystemTimes`, `GlobalMemoryStatusEx`, `GetLogicalDrives` + `GetDriveTypeW` (jen `DRIVE_FIXED`) + `GetDiskFreeSpaceExW`, `GetIfTable2` (jen hardwarové adaptéry nahoře, bez … | `cpu_times` `memory` `fixed_disks` `net_totals` |
| `src-tauri/src/sysmon/conf.rs` | config `system` (`disks`, `netUnit`), validace | `SystemConfig` `NetUnit` `validate` |
| `src-tauri/src/sysmon/mod.rs` | stav (předchozí čítače, poslední vzorek, historie), `poll` (vzorek nejvýš 1× za 1,5 s, čtení mimo zámek, disky 1× za 30 s, stará historie po pauze pryč), `strip_now`, `next_strip_due` … | `SystemView` `poll` `strip_now` `next_strip_due` |
| `src-tauri/src/github/parse.rs` | parsování JSON z `gh` (shovívavé, adresy jen `https://github.com/`), převod API adresy notifikace na webovou; vzorky `src/fixtures/gh-*.json` | `parse_search` `parse_mine` `parse_notifications` `web_url` `ci_of` `safe_url` `Pr` |
| `src-tauri/src/github/run.rs` | spuštění `gh` bez okna s termínem 15 s (`gitinfo::capture`, `update::gh_exe`), třídění chyb ze stderr | `run` `classify` `GhErr` |
| `src-tauri/src/github/conf.rs` | config `github` (`enabled` výchozí `false`, `review`/`mine`/`notifications`, `refreshMin` 2–60) | `GithubConfig` `validate` `interval_ms` `any` |
| `src-tauri/src/github/mod.rs` | stav v paměti, `due` (kdy smí obnova: karta / plánovač / Obnovit), `fetch` (3 dotazy souběžně), `apply` (stav a počty z výsledků), `refresh`, `next_due`, `on_config`, `strip_now`, `open_ok` … | `GithubView` `Status` `Mode` `due` `apply` `refresh` |
| `src-tauri/src/integrations.rs` | plánovač položek proužku: `wants` (položka v `strip.items` + proužek zapnutý; GitHub navíc `enabled` a `review`), `plan` (čistá), vlákno `integrations` vzniká v `sync` jen s položkou a samo skončí … | `Want` `wants` `plan` `sync` `SYSTEM_EVERY_MS` |
| `src-tauri/src/commands/integrations.rs` | Tauri: `system_view`, `system_taskmgr`, `github_view(force)`, `github_open`, `github_copy_login` (schránka přes Rust — šuplík nemá fokus) |  |
| `src/integrations/format.ts` | čistá logika UI (test `format.test.mjs`): `fmtSize`, `fmtRate` (bajty / bity), `fmtPct`, `sparkPoints`, `loadOf`, `diskLoad`, `toggleDisk`, `CI_TONE`, `isoMs` |  |
| `src/integrations/api.ts` | typy `SystemView` / `GithubView` a `intApi` (vlastní, aby se nesahalo do `types.ts` / `api.ts`) | `intApi` |
| `src/integrations/SystemCard.tsx` | karta Systém: CPU, paměť, disky, síť + sparkline; polling 2 s jen s `drawerShown()`; klik na CPU / paměť → Správce úloh | `SystemCard` |
| `src/integrations/SystemDesign.tsx` · `systemLooks.ts` · `systemLooks.css` | kolo 25, vzhledy karty Systém (`look.design`: `rings`, `gauges`, `bars`, `terminal`, `tiles`, `net`; `lines` = řádky v `SystemCard.tsx`). Kreslí tentýž `SystemView` (historie CPU / paměť / síť, disky), nic neměří navíc; `systemLooks.ts` čistá logika (`termCells`, `histStats`, `fullestDisk`, `loadBars`, test `systemLooks.test.mjs`), prstence a budíky z `charts/Gauge` / `Rings`, tooltipy s hodnotou, průměrem a špičkou, šířka přes container query | `SystemDesign` `termCells` `fullestDisk` `loadBars` |
| `src/integrations/GithubCard.tsx` | karta GitHub: sekce a prázdné stavy (vypnuto / bez `gh` / nepřihlášeno / chyba), smyčka podle `nextInMs` jen s viditelnou kartou | `GithubCard` |
| `src/integrations/IntegrationsSettings.tsx` | Nastavení → Šuplík a karty → Obsah karet → GitHub a Systém (přepínač s větou, sekce, frekvence, disky jako dlaždice, jednotky) | `GithubSettings` `SystemSettings` |
| `src/integrations/integrations.css` · `src/i18n/en-integrations.ts` | styly (`sys-*`, `gh-*`, `int-*`) · angličtina `cards.github.*`, `cards.system.*`, `settings.github.*`, `settings.f.github.*`… |  |
