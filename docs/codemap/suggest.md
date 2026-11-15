# Codemapa: kontextové návrhy a puls proužku u GitHubu

Pruh nahoře v šuplíku (max 3 chipy) — výhradně z reálných událostí, nic se neukáže bez zdroje — a krátký puls proužku, když se u GitHubu objeví nové PR k revizi nebo spadne CI na tvém PR. Společné pravidlo: žádný vlastní polling, vlákno ani časovač …

Detail (toky, invarianty, testy, co nečíst, historie): [suggest-detail.md](suggest-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/suggest/conf.rs` | config `suggest` — `clipboard` (výchozí vypnuto: čte obsah schránky a potřebuje zapnutou historii), `devServer`, `github`, `reminders`, `digest`, `githubPulse` (výchozí zapnuto) | `SuggestConfig` |
| `src-tauri/src/suggest/mod.rs` | snapshot `suggest`: zapnuté zdroje + GitHub z paměti (`github::cached_prs`, jen `Status::Ok`, nikdy dotaz na síť) … | `SuggestView` `GithubRefs` `PrRef` `refs` `view` `on_github_refreshed` `reset_github` |
| `src-tauri/src/suggest/pulse.rs` | puls proužku u GitHubu: čisté `observe` / `transitions` (nové PR k revizi, nově spadlé CI; první načtení jen založí paměť), `Dedup` (stejná událost nejvýš 1× za 30 min), `on_refreshed` → … | `GhSeen` `transitions` `Dedup` `on_refreshed` `reset` |
| `src/suggest/model.ts` | čistá logika (test `model.test.mjs`): `pick(input, dismissed, now)` = `pickAll` ořezané na 3 — pořadí zadání · schůzka · připomínka · Dnes (úkoly po termínu / na dnes z `notesTaskCounts`, pod `on.reminders`, klik = záložka `_tasks`; ustoupí rannímu shrnutí) · schůzka (Připojit 10 min před začátkem do konce, jen s `join`) / poznámka ze schůzky (do 15 min po konci) · … | `pick` `dismiss` `clipUrl` `isKnown` `knownLinks` `normUrl` `devUrl` `sameSuggestions` `nextChange` |
| `src/suggest/morning.ts` | H5 ranní shrnutí (test `morning.test.mjs`): chip `morning` při prvním otevření po ≥ 6 h nečinnosti v okně 5–13 h, jednou denně; jen neprázdné části z existujících dat (schůzky dnes, úkoly, připomínky, zadání, PR, CI, týdenní limit ≥ 50 %); detail v suggest-detail.md | `morningParts` `isMorningOpen` `morningKey` `morningWindowEnd` `morningSig` |
| `src/suggest/since.ts` | V1 „Pruh ví“: `claudeSince(sessions, drawerHiddenAt)` = čeká / chyba / doběhlo / skončila (`activity.phase` done·error po schování šuplíku, `endReason`), `sinceTarget`, `sinceSig`; chip `briefWaiting` z `Snapshot.briefWaiting` | `claudeSince` `sinceTarget` `sinceSig` `SINCE_ORDER` |
| `src/suggest/SuggestBar.tsx` | pruh: schránku čte (`clip_list`) jen se zapnutým zdrojem a na událost `cliphist`; akce chipů (otevřít poznámku → `nav.onOpen`, PR → `github_open`, dev server → `open_url`, dev s chybami → „Chyba → zadání“ (`actions.ts` `devErrorPreset` → `nav.onNew(preset)`), uložit odkaz, do poznámky) … | `SuggestBar` |
| `src-tauri/src/procman/errline.rs` | V3 (jádro): `looks_like_error`, `mask_secrets` (tokeny, `KEY=`, heslo v URL), `ErrLines` = ≤ 8 maskovaných chybových řádků + stack → `ProcView.errorLines` (jen při `errors > 0`) | `looks_like_error` `mask_secrets` `ErrLines` |
| `src/suggest/devbrief.ts` | `devBriefBody`: tělo zadání z dev chyby (jen známé údaje) | `devBriefBody` |
| `src/suggest/chips.ts` · `label.ts` | `useChips(s, now, clip)` = reaktivní `pick` sdílený pruhem (`SuggestBar`) i klidovým ostrůvkem (`hover/Island.tsx`, H2): přepočet při změně snapshotu / zavření / schránky a na hranicích `model.ts` `nextChange` (ne každou sekundu; brána `gate` nad tikem `now`) · `chipLabel` = text chipu | `useChips` `chipLabel` `todayLabel` |
| `src/suggest/actions.ts` | zavřené návrhy (signál v paměti), `saveLink` (nová akce `url` v configu, `get_config` → `save_config`), `urlToNote` (`note_create`) | `closeSuggestion` `saveLink` `urlToNote` `meetingToNote` |
| `src/suggest/SuggestSettings.tsx` · `suggest.css` · `src/i18n/en-suggest.ts` | přepínače zdrojů v Nastavení → Šuplík a karty → Obsah karet · styly (`sg-*`) · angličtina `cards.suggest.*`, `settings.suggest.*`, `settings.f.suggest.*` | `SuggestSettings` |
| `src/types/suggest.ts` | zrcadlo `SuggestConfig` / `SuggestView` |  |
