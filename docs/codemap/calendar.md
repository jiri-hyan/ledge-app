# Kalendář z odkazu ICS (záložka Osobní)

Jen čtení, bez OAuth: Google i Outlook dají „tajnou adresu ve formátu iCal“. Žádná nová crate — parser,
opakování i pásma jsou vlastní (chrono z existujících závislostí, sha2 na otisk adresy).

## Rust `src-tauri/src/calendar/`

| soubor | co dělá | hlavní symboly |
|---|---|---|
| `mod.rs` | feed → výskyty v okně −`PAST_DAYS`/+`AHEAD_DAYS` (UTC ms; celodenní = půlnoc data jako UTC) | `collect`, `Occ`, `Parsed`, `clip`, `MAX_OCCS` |
| `ics.rs` | rozbalení řádků (na bajtech), parametry, escapování, VEVENT, VTIMEZONE, `X-WR-CALNAME`; filtr `keep` zahodí nepotřebné události hned | `parse`, `Event`, `Dt`, `VTimezone`, `TzPart`, `parse_dt`, `parse_duration` |
| `expand.rs` | RRULE v místním čase události: FREQ D/W/M/Y, INTERVAL, COUNT, UNTIL, BYDAY (i `2MO`/`-1FR`), BYMONTHDAY, BYMONTH, BYSETPOS, WKST | `parse_rule`, `occurrences`, `month_days`, `Rule`, `Until` |
| `tz.rs` | pásma: VTIMEZONE z feedu → vestavěná tabulka IANA + Windows názvů (EU/US letní čas, pevné posuny) → neznámé = místní + `guess` | `Zone`, `Zones`, `Tz`, `builtin`, `LocalTz`, `SystemLocal` |
| `view.rs` | výběr karty: Dnes (bez skončených) / Zítra / první další den, řazení, strop řádků, čas další změny | `pick`, `Day`, `Item`, `Picked` |
| `conf.rs` | config `personal.calendars`, adresa jako tajemství (webcal→https, jen https, doména, otisk) | `CalendarFeed`, `normalize_url`, `domain`, `key`, `validate` |
| `live.rs` | stav feedů, stahování (reqwest + native-tls, 15 s, 5 MB, redirect jen https), cache na disku, snapshot, termíny emit vlákna, lidské chyby | `maybe_refresh`, `on_config`, `view`, `next_deadline`, `tick`, `test`, `http_error`, `needs_fetch` |
| `tests.rs` | celá cesta nad `fixtures/calendar-google.ics` a `calendar-outlook.ics` + malé feedy | — |

Napojení (jednořádkové): `threads/ui.rs` (otevření šuplíku), `threads/emit.rs` (`deadlines` + `tick` +
`maybe_refresh`), `commands/drawer.rs` `set_mode`, `settings.rs` `apply_config` → `on_config`,
`snapshot.rs` `PersonalView.calendar`, `config.rs` (`sections!` → `calendar`, `PersonalConfig.calendars`,
`validate_field`), `commands/calendar.rs` `calendar_test`.

**Výkon:** nic nepolluje. Stahuje se jen s viditelnou kartou (šuplík Full a karta Kalendář na otevřené
záložce — `layout::page_has`), data starší `MAX_AGE_MS` (15 min). Emit vlákno dostane termín jen
s viditelnou kartou (`view::pick` → `next_change`: začátek/konec dnešní události, místní půlnoc). V paměti
jen výskyty v okně (nejvýš `MAX_OCCS` na feed); text odpovědi se po parsování zahodí.

**Tajemství:** adresa se nikdy neloguje (reqwest chyby přes `without_url`), chyby ji neobsahují (testy
`validate_table_never_leaks_url`, `cache_roundtrip_and_age`), UI dostává jen `domain`.

## UI `src/calendar/`

| soubor | co dělá |
|---|---|
| `CalendarCard.tsx` | karta (Dnes/Zítra/Příští, prázdné a chybové stavy, výzva „Přidat kalendář“, patička s vypnutím jedním klikem); `openCalendarSettings` |
| `CalendarSettings.tsx` | editor v Nastavení → Osobní (kotva `f-personal.calendars` je `Section` v `settings/panels/Personal.tsx`): přidat odkazem, barva, název, zapnutí, Otestovat, Změnit odkaz, Odebrat s Vrátit, návod |
| `model.ts` (+ `model.test.mjs`) | barvy, `normalizeUrl` (zrcadlo Rust), `domainOf`, `timeLabel`, `withCalendar` |
| `calendar.css` | jen třídy `cal-*` |

Angličtina: `src/i18n/en-calendar.ts`. Typy: `types.ts` `CalendarView`, `CalendarItem`, `CalendarDay`,
`CalendarInfo`, `CalendarTest`; `api.calendarTest`. Sekce `calendar` v `cards/sections.ts`.
