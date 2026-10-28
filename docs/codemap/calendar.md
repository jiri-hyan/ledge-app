# Kalendář (záložka Osobní): zdroje, Připojit, připomínky

Jen čtení. Zdroje: tajný odkaz ICS (Google i Outlook dají „tajnou adresu ve formátu iCal“), klasický Outlook v tomto PC (COM, bez přihlášení), místo pro účty Microsoft/Google (dodá modul účtů). Žádná nová crate — parser, opakování i pásma jsou vlastní (chrono z existujících závislostí, sha2 na otisk adresy).

Detail (toky, invarianty, testy, co nečíst, historie): [calendar-detail.md](calendar-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `mod.rs` | feed → výskyty v okně −`PAST_DAYS`/+`AHEAD_DAYS` (UTC ms; celodenní = půlnoc data jako UTC) | `collect`, `Occ` (i `join`), `Parsed`, `clip`, `MAX_OCCS` |
| `ics.rs` | rozbalení řádků (na bajtech), parametry, escapování, VEVENT, VTIMEZONE, `X-WR-CALNAME`; filtr `keep` zahodí nepotřebné události hned … | `parse`, `Event`, `Dt`, `VTimezone`, `TzPart`, `parse_dt`, `parse_duration` |
| `expand.rs` | RRULE v místním čase události: FREQ D/W/M/Y, INTERVAL, COUNT, UNTIL, BYDAY (i `2MO`/`-1FR`), BYMONTHDAY, BYMONTH, BYSETPOS, WKST | `parse_rule`, `occurrences`, `month_days`, `Rule`, `Until` |
| `tz.rs` | pásma: VTIMEZONE z feedu → vestavěná tabulka IANA + Windows názvů (EU/US letní čas, pevné posuny) → neznámé = místní + `guess` | `Zone`, `Zones`, `Tz`, `builtin`, `LocalTz`, `SystemLocal` |
| `view.rs` | `any_ongoing` (probíhá teď časovaná událost? pro klid pulsu GitHubu, `live::meeting_now`); výběr karty: Dnes (bez skončených) / Zítra / první další den, řazení, strop řádků, čas další změny … | `pick`, `Day`, `Item`, `Picked` |
| `conf.rs` | config `personal.calendars` (`source`, `account`, `remindMin`, `remindToast`), adresa jako tajemství (webcal→https, jen https, doména, otisk), validace podle zdroje | `CalendarFeed`, `normalize_url`, `domain`, `key`, `validate`, `DEFAULT_REMIND_MIN` |
| `source.rs` | zdroj jako abstrakce: druhy, okno, sdílený typ událostí, klíč stavu, cíl načtení, dispatch, kontrakt adaptéru účtů | `SourceKind` `Event` `Occ` `Window` `Adapter` `adapter` `Target` `key` `targets` `fetch` |
| `outlook.rs` | zdroj `outlook-local`: detekce bez spuštění, tenká vrstva IDispatch, okno přes `Restrict`, převod položek (čistý) | `installed` `running` `events` `RawItem` `to_occ` `naive_from_ole` `restrict_filter` |
| `meeting.rs` | odkaz Připojit: vytažení odkazů z textu, rozbalení Safe Links / `google.com/url`, rozpoznání Teams/Meet/Zoom/Webex/Jitsi, `msteams:` jen bezpečně | `find` `classify` `offer` `rank_of` `RANK_*` `Join` `Provider` `app_url` `launch_uri` |
| `remind.rs` | připomínka schůzky: okno [začátek − předstih, začátek + 2 min), budík, text proužku, toast s Připojit, jednou na schůzku | `active` `next_alarm` `fresh` `strip_text` `toast_text` `next_deadline` `tick` `strip_now` |
| `live.rs` | stav zdrojů, stahování ICS (reqwest + native-tls, 15 s, 5 MB, redirect jen https), snapshot, termíny emit vlákna, Outlook zavřený → nápověda, výslovné načtení z Outlooku, výskyty pro připomínky | `maybe_refresh` `on_config` `view` `next_deadline` `tick` `test` `fetch_parse` `store` |
| `graph.rs` · `google.rs` | adaptéry kalendářů účtů Microsoft / Google (`adapter` pro `source::adapter`, `GraphSource`/`GoogleSource` pro `accounts::calsync`) … | `parse`, `fetch`, `adapter`, `join_of`, `GraphSource`, `GoogleSource` |
| `cache.rs` | formát cache na disku (`calendar-cache.json`, klíč = otisk zdroje, v1, stáří 7 dní; stará cache bez `join` se načte) | `CacheFeed`, `encode_cache`, `decode_cache`, `cache_path` |
| `tests.rs` | celá cesta nad `fixtures/calendar-google.ics`, `calendar-outlook.ics`, `calendar-meetings.ics` (pozvánky Teams/Meet/Zoom/Webex/Jitsi) + malé feedy | — |
| `CalendarCard.tsx` | karta (Dnes/Zítra/Příští, prázdné a chybové stavy, výzva „Přidat kalendář“, patička s vypnutím jedním klikem); u schůzky „za N min“ a Připojit (`api.calendarJoin`) … |  |
| `CalendarSettings.tsx` | editor v Nastavení → Šuplík a karty → Obsah karet (kotva `f-personal.calendars` je `Section` v `settings/panels/Personal.tsx`): dlaždice zdrojů `SOURCES` (Odkaz ICS / Outlook v tomto PC … |  |
| `model.ts` (+ `model.test.mjs`) | barvy, `normalizeUrl` (zrcadlo Rust), `domainOf`, `timeLabel`, `withCalendar`; `sourceOf`, `providerName`, `placeLabel`, `joinable`, `startsIn`, `remindOptions`, `remindOf`, `outlookFeed` |  |
| `calendar.css` | jen třídy `cal-*` |  |

Další soubory (podrobnosti v detailu):
`src-tauri/src/commands/drawer.rs` · `src/types/calendar.ts`
