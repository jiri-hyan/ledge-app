# Codemapa: kontextové návrhy a puls proužku u GitHubu

Pruh nahoře v šuplíku (max 3 chipy) — **výhradně z reálných událostí**, nic se neukáže bez zdroje — a krátký puls
proužku, když se u GitHubu objeví nové PR k revizi nebo spadne CI na tvém PR. Společné pravidlo: **žádný vlastní
polling, vlákno ani časovač**; návrhy se přepočítají při změně snapshotu / sekundovém tiku šuplíku (`now`), puls
vzniká jen při načtení GitHubu, které už dělá karta nebo plánovač proužku. Config `suggest.*`
(Nastavení → Šuplík a karty → Obsah karet → Kontextové návrhy), formát: [agents/config-behavior.md](../agents/config-behavior.md).
Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/suggest/conf.rs` | config `suggest` — `clipboard` (výchozí **vypnuto**: čte obsah schránky a potřebuje zapnutou historii), `devServer`, `github`, `reminders`, `digest` (zadání / ranní shrnutí / Dnes / Claude od posledně), `githubPulse` (výchozí zapnuto) | `SuggestConfig` |
| `src-tauri/src/suggest/mod.rs` | snapshot `suggest`: zapnuté zdroje + GitHub z paměti (`github::cached_prs`, jen `Status::Ok`, nikdy dotaz na síť): `refs` (PR k revizi a spadlé CI na mých PR, bez konceptů a bez neověřené adresy), `view`, `on_github_refreshed` (→ `mark_dirty`), `reset_github` | `SuggestView` `GithubRefs` `PrRef` `refs` `view` `on_github_refreshed` `reset_github` |
| `src-tauri/src/suggest/pulse.rs` | puls proužku u GitHubu: čisté `observe` / `transitions` (nové PR k revizi, nově spadlé CI; první načtení jen založí paměť), `Dedup` (stejná událost nejvýš 1× za 30 min), `on_refreshed` → `attention::signal` | `GhSeen` `transitions` `Dedup` `on_refreshed` `reset` |
| `src/suggest/model.ts` | čistá logika (test `model.test.mjs`): `pick(input, dismissed, now)` — pořadí připomínka · schůzka (Připojit 10 min před začátkem do konce, jen s `join`) / poznámka ze schůzky (do 15 min po konci) · spadlé CI · PR k revizi · dev server · schránka, jeden chip na druh (`more` = další téhož druhu), strop `MAX_CHIPS`; `dismiss` / `isDismissed` (24 h, v paměti), `clipUrl` (nejnovější položka historie schránky = jedna celá adresa, ≤ 30 min), `isKnown` / `knownLinks` / `normUrl` (adresa už mezi odkazy), `devUrl` (jen loopback), `sameSuggestions` (tik času chipy nepřekreslí) | `pick` `dismiss` `clipUrl` `isKnown` `knownLinks` `normUrl` `devUrl` `sameSuggestions` `MAX_CHIPS` `DISMISS_MS` |
| `src/suggest/SuggestBar.tsx` | pruh: schránku čte (`clip_list`) jen se zapnutým zdrojem a na událost `cliphist`; akce chipů (otevřít poznámku → `nav.onOpen`, PR → `github_open`, dev server → `open_url`, dev s chybami → „Chyba → zadání“ (`actions.ts` `devErrorPreset` → `nav.onNew(preset)`), uložit odkaz, do poznámky); × zavře na 24 h | `SuggestBar` |
| `src/suggest/actions.ts` | zavřené návrhy (signál v paměti), `saveLink` (nová akce `url` v configu, `get_config` → `save_config`), `urlToNote` (`note_create`) | `closeSuggestion` `saveLink` `urlToNote` `meetingToNote` |
| `src/suggest/SuggestSettings.tsx` · `suggest.css` · `src/i18n/en-suggest.ts` | přepínače zdrojů v Nastavení → Šuplík a karty → Obsah karet · styly (`sg-*`) · angličtina `cards.suggest.*`, `settings.suggest.*`, `settings.f.suggest.*` | `SuggestSettings` |
| `src/types/suggest.ts` | zrcadlo `SuggestConfig` / `SuggestView` | |

Lepidlo: `config/mod.rs` (`Config.suggest`), `snapshot.rs` (`suggest: suggest::view`), `github/mod.rs` (`cached_prs`;
`refresh` po uvolnění zámku volá `suggest::on_github_refreshed`, `on_config` při vypnutí `reset_github`),
`attention.rs` (`Event::GithubCi` / `GithubReview`), `MainPage.tsx` (`<SuggestBar />` nad mřížkou),
`settings/{labels.ts,panels/Personal.tsx}`, `i18n/dicts.ts`, `types/{index,snapshot}.ts`.

## Zdroje chipů (odkud a kdy)

- **Schůzka** (`meeting`, `meetingNote`; nezávisí na `suggest.reminders` (L102; vypíná je kalendář nebo jeho `remindMin`)) — dnešní události `personal.calendar` s ověřeným `join`, bez celodenních
  (`currentMeetings`). Kalendář skončené události ze zobrazení hned vyřadí, proto `SuggestBar` schůzky s odkazem pamatuje v paměti
  (`trackMeetings`, 15 min po konci) — poznámku ze schůzky nabídne jen u schůzky, kterou šuplík za běhu aspoň jednou viděl.
  „Připojit“ = `api.calendarJoin` (Rust znovu ověří); „Poznámka ze schůzky“ = `meetingToNote` (`note_create`, titulek
  `meetingNoteText` „{název} — {datum}“, kategorie výchozí) a hned editor (`nav.onOpen`). Klíče `meet-join:` / `meet-note:`, zavření 24 h jako ostatní.

- **Připomínka po termínu** — `Snapshot.reminders` (první 3 po termínu, už existující peek) → „Otevřít poznámku“.
  Klíč obsahuje termín: přeplánovaná připomínka se vrátí i po zavření.
- **Spadlé CI / PR k revizi** — `Snapshot.suggest.github` z paměti GitHubu; jen se zapnutým `github.enabled` a daty
  mladšími než hodina (`GH_FRESH_MS`). Nic nespouští: data se obnoví tehdy, kdy už je Ledge obnovuje (karta GitHub
  vidět / položka proužku `github`).
- **Dev server** — `Snapshot.procs` (spuštěno z Ledge, fáze `ready` / `running`, má port) jen 10 min po naběhnutí
  (`DEV_FRESH_MS`); adresa z výstupu procesu se použije jen když je loopback, jinak `http://localhost:PORT`.
- **Schránka** — nejnovější položka historie schránky ([clipboard.md](clipboard.md)), je-li to jedna celá adresa
  http(s) čerstvá ≤ 30 min a není mezi odkazy (akce `url` / `web`, starší `personal.links`). Bez zapnuté historie
  schránky nic (žádný vlastní poslech schránky).

## V1 „Pruh ví“: chipy `briefWaiting` a `claudeSince` (kolo 34)

- Pořadí `pickAll` (L175, k34): zadání čeká na mě · schůzka · připomínka · Dnes · poznámka ze schůzky · Claude od posledně · ranní shrnutí · CI · PR · dev · schránka. KAŽDÝ druh `SuggestKind` má řádek v `hover/priority.ts` `PRIORITY` / `KIND_PRIO` (typ `Record<SuggestKind, Prio>` + `priority.test.mjs` selže na druhu bez řádku); nový druh chipu = řádek tam. Schůzka je před připomínkou (za 10 min je naléhavější než splatná poznámka).
- Ranní shrnutí (H5): viz `morning.ts`. Výchozí zapnuto (jen složení existujících dat, jednou denně, jde zavřít, při prázdných zdrojích se neukáže; vypíná ho `suggest.digest` (L193; dřív sdílený `reminders`)). Známé omezení: `drawerHiddenAt` se nuluje každým schováním vč. hover doku (viz audit U, nález 1), takže po hover-peeku ráno shrnutí nenabídne.
- `briefWaiting`: Rust `notes/briefwait.rs` `brief_waiting(all)` přes všechny nehotové poznámky (`status: waiting` = otázka, `review` = kontrola; `done` se nepočítá) → `Snapshot.briefWaiting` (`waiting`, `review`, `first`). Klik = otevřít poznámku (skok na sekci Otázky není).
- `claudeSince`: vstup `Snapshot.sessions` + `Snapshot.drawerHiddenAt` (Rust `drawer.rs` `hidden_at_ms`, nastaví se při `hide`; `null` = od startu neschován → jen čekající, nic „od posledně“). Klíč dismiss `claude-since:<hiddenAt>`. Klik = `session_open` první čekající / s chybou / doběhlé, kterou jde otevřít.
- `endReason` = `SessionEnd.reason` z hooku (`claude/activity.rs` `end_reason`, `Session.end_reason` → `Row.end_reason`); text `label.ts` `endReasonLabel`, neznámý / `other` = nic (žádné „spadla“). Session bez nedokončené práce se po `SessionEnd` odebere, důvod proto vidí jen skončené session, které v kartě zůstaly.
- Gating: `briefWaiting`, ranní shrnutí, `today` a `claudeSince` pod `suggest.digest` (L193; chybí-li klíč, platí `reminders` — `digestOn`), `reminder` pod `suggest.reminders`; `claudeSince` jen se `claudeEnabled`, `briefWaiting` jen s poznámkami. Žádný časovač ani nová hranice `nextChange`.

## Puls proužku (GitHub)

`github::refresh` → `suggest::on_github_refreshed` → `pulse::on_refreshed`: porovná množiny PR (k revizi, spadlé CI)
s předchozím pozorováním (`transitions`). **První pozorování po startu / zapnutí jen založí paměť** (žádný puls za PR,
která tu byla už před Ledge). Nové PR k revizi / PR, jehož CI nově spadlo, projde `Dedup` (30 min na událost) a
`attention::signal(color, false)` — stejný mechanismus jako „Claude čeká“: bliknutí linky (`Core.flash`) a puls
(`PULSE_MS`), nic ve fullscreen hře / herním režimu (`Gate.dead`), bez toastu a bez fokusu. Vypíná `suggest.githubPulse`.
**Klid při schůzce:** `pulse::on_refreshed` zeptá `calendar::live::meeting_now` (paměť kalendářů, `view::any_ongoing`: časovaná událost právě probíhá, celodenní ne) a `decide(.., in_meeting)` pak puls nepustí; paměť pozorování se aktualizuje dál (po schůzce se nic nepřehrává), chip v návrzích zůstává.
Priorita při souběhu (`Event::rank`): čeká Claude > schůzka > spadlé CI > PR k revizi > hotovo.

## Testy

Rust: `suggest` (`refs_table`, výchozí pohled vypnutého GitHubu), `suggest::conf` (výchozí hodnoty, starý config),
`suggest::pulse` (tabulka přechodů, dedup, rozhodnutí o pulsu), `attention` (priorita nových událostí). UI:
`src/suggest/model.test.mjs`. Živé chování (chipy v šuplíku, puls po `gh` obnově) jen ručně.

## Dev chyba → zadání (V3, kolo 34)

- Jádro: `procman/errline.rs` drží posledních ≤ 8 chybových řádků (maskované `mask_secrets`: `?token=`, `Bearer`, `KEY=…`, heslo v URL, tvary GitHub/sk-/AKIA/JWT; ≤ 1024 znaků; za chybou ≤ 3 rámce stacku, do počtu `errors` nejdou). `ProcView.errorLines` jen při `errors > 0`; `cmd` je maskovaný taky. `looks_like_error` = token `error`/`*Error`/`ERR!`/`✘`/`failed to`/`EADDRINUSE`, ne cesta (`error-boundary.tsx`) ani „0 errors“.
- `model.ts` `pick`: dev proces s `errors > 0` dostane chip i po `DEV_FRESH_MS`; klíč `dev:<id>:<port>:<readyAt>:<errors>` (zavřený se vrátí jen při nových chybách); `Suggestion.errors` / `proc`.
- Tlačítko „Chyba → zadání“ (jen s `notesEnabled`): `actions.ts` `devErrorPreset` složí `NotePreset {project, kind: "zadani", body}` (`devbrief.ts` `devBriefBody`, test `devbrief.test.mjs`) a otevře editor (`NotesNav.onNew(preset)` → `NoteEditor.applyPreset`, tělo jen do prázdného konceptu). Odeslání = výslovný klik „Uložit a poslat“ (V2), žádné automatické.
