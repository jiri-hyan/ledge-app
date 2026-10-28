# Codemapa: kontextové návrhy a puls proužku u GitHubu

Pruh nahoře v šuplíku (max 3 chipy) — výhradně z reálných událostí, nic se neukáže bez zdroje — a krátký puls proužku, když se u GitHubu objeví nové PR k revizi nebo spadne CI na tvém PR. Společné pravidlo: žádný vlastní polling, vlákno ani časovač …

Detail (toky, invarianty, testy, co nečíst, historie): [suggest-detail.md](suggest-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/suggest/conf.rs` | config `suggest` — `clipboard` (výchozí vypnuto: čte obsah schránky a potřebuje zapnutou historii), `devServer`, `github`, `reminders`, `githubPulse` (výchozí zapnuto) | `SuggestConfig` |
| `src-tauri/src/suggest/mod.rs` | snapshot `suggest`: zapnuté zdroje + GitHub z paměti (`github::cached_prs`, jen `Status::Ok`, nikdy dotaz na síť) … | `SuggestView` `GithubRefs` `PrRef` `refs` `view` `on_github_refreshed` `reset_github` |
| `src-tauri/src/suggest/pulse.rs` | puls proužku u GitHubu: čisté `observe` / `transitions` (nové PR k revizi, nově spadlé CI; první načtení jen založí paměť), `Dedup` (stejná událost nejvýš 1× za 30 min), `on_refreshed` → … | `GhSeen` `transitions` `Dedup` `on_refreshed` `reset` |
| `src/suggest/model.ts` | čistá logika (test `model.test.mjs`): `pick(input, dismissed, now)` — pořadí připomínka · schůzka (Připojit 10 min před začátkem do konce, jen s `join`) / poznámka ze schůzky (do 15 min po konci) · … | `pick` `dismiss` `clipUrl` `isKnown` `knownLinks` `normUrl` `devUrl` `sameSuggestions` |
| `src/suggest/SuggestBar.tsx` | pruh: schránku čte (`clip_list`) jen se zapnutým zdrojem a na událost `cliphist`; akce chipů (otevřít poznámku → `nav.onOpen`, PR → `github_open`, dev server → `open_url`, uložit odkaz, do poznámky) … | `SuggestBar` |
| `src/suggest/actions.ts` | zavřené návrhy (signál v paměti), `saveLink` (nová akce `url` v configu, `get_config` → `save_config`), `urlToNote` (`note_create`) | `closeSuggestion` `saveLink` `urlToNote` `meetingToNote` |
| `src/suggest/SuggestSettings.tsx` · `suggest.css` · `src/i18n/en-suggest.ts` | přepínače zdrojů v Nastavení → Šuplík a karty → Obsah karet · styly (`sg-*`) · angličtina `cards.suggest.*`, `settings.suggest.*`, `settings.f.suggest.*` | `SuggestSettings` |
| `src/types/suggest.ts` | zrcadlo `SuggestConfig` / `SuggestView` |  |
| `src/cards/today.ts` (+ `today.test.mjs`) | čistá logika: `todayRows(input, now)` v pořadí schůzka · připomínky · PR · limit, `sameRows` (tik času nepřekreslí) | `todayRows` `nextMeeting` `dueReminders` `reviewRow` `limitRow` `endOfDay` `sameRows` |
| `src/cards/TodayCard.tsx` · `today.css` | karta (třídy `td-*`), `todayInput` ze snapshotu, `useTodayRows` (memo, počítá se jen na záložce s kartou) | `TodayCard` `useTodayRows` `todayInput` |
