# Kalendář (záložka Osobní): zdroje, Připojit, připomínky

Jen čtení. Zdroje: tajný odkaz ICS (Google i Outlook dají „tajnou adresu ve formátu iCal“), klasický
Outlook v tomto PC (COM, bez přihlášení), místo pro účty Microsoft/Google (dodá modul účtů). Žádná nová
crate — parser, opakování i pásma jsou vlastní (chrono z existujících závislostí, sha2 na otisk adresy).

## Rust `src-tauri/src/calendar/`

| soubor | co dělá | hlavní symboly |
|---|---|---|
| `mod.rs` | feed → výskyty v okně −`PAST_DAYS`/+`AHEAD_DAYS` (UTC ms; celodenní = půlnoc data jako UTC) | `collect`, `Occ` (i `join`), `Parsed`, `clip`, `MAX_OCCS` |
| `ics.rs` | rozbalení řádků (na bajtech), parametry, escapování, VEVENT, VTIMEZONE, `X-WR-CALNAME`; filtr `keep` zahodí nepotřebné události hned; z vlastností schůzky / URL / místa / popisu jen odkaz Připojit (popis se nedrží) | `parse`, `Event`, `Dt`, `VTimezone`, `TzPart`, `parse_dt`, `parse_duration` |
| `expand.rs` | RRULE v místním čase události: FREQ D/W/M/Y, INTERVAL, COUNT, UNTIL, BYDAY (i `2MO`/`-1FR`), BYMONTHDAY, BYMONTH, BYSETPOS, WKST | `parse_rule`, `occurrences`, `month_days`, `Rule`, `Until` |
| `tz.rs` | pásma: VTIMEZONE z feedu → vestavěná tabulka IANA + Windows názvů (EU/US letní čas, pevné posuny) → neznámé = místní + `guess` | `Zone`, `Zones`, `Tz`, `builtin`, `LocalTz`, `SystemLocal` |
| `view.rs` | výběr karty: Dnes (bez skončených) / Zítra / první další den, řazení, strop řádků, čas další změny; `Item.join` | `pick`, `Day`, `Item`, `Picked` |
| `conf.rs` | config `personal.calendars` (`source`, `account`, `remindMin`, `remindToast`), adresa jako tajemství (webcal→https, jen https, doména, otisk), validace podle zdroje | `CalendarFeed`, `normalize_url`, `domain`, `key`, `validate`, `DEFAULT_REMIND_MIN` |
| `source.rs` | **zdroj jako abstrakce**: druhy, okno, sdílený typ událostí, klíč stavu, cíl načtení, dispatch, kontrakt adaptéru účtů | `SourceKind`, `Event` (= `Occ`), `Window`, `Adapter`, `adapter`, `Target`, `key`, `targets`, `fetch`, `finish`, `unavailable` |
| `outlook.rs` | zdroj `outlook-local`: detekce bez spuštění, tenká vrstva IDispatch, okno přes `Restrict`, převod položek (čistý) | `installed`, `running`, `events`, `RawItem`, `to_occ`, `naive_from_ole`, `restrict_filter`, `missing`, `closed` |
| `meeting.rs` | odkaz Připojit: vytažení odkazů z textu, rozbalení Safe Links / `google.com/url`, rozpoznání Teams/Meet/Zoom/Webex/Jitsi, `msteams:` jen bezpečně | `find`, `classify`, `offer`, `rank_of`, `RANK_*`, `Join`, `Provider`, `app_url`, `launch_uri` |
| `remind.rs` | připomínka schůzky: okno [začátek − předstih, začátek + 2 min), budík, text proužku, toast s Připojit, jednou na schůzku | `active`, `next_alarm`, `fresh`, `strip_text`, `toast_text`, `next_deadline`, `tick`, `strip_now`, `GRACE_MS` |
| `live.rs` | stav zdrojů, stahování ICS (reqwest + native-tls, 15 s, 5 MB, redirect jen https), snapshot, termíny emit vlákna, Outlook zavřený → nápověda, výslovné načtení z Outlooku, výskyty pro připomínky | `maybe_refresh`, `on_config`, `view`, `next_deadline`, `tick`, `test`, `fetch_parse`, `store`, `load_outlook`, `reminder_feeds`, `with_reminders`, `ensure_cache`, `HINT_OUTLOOK_CLOSED` |
| `graph.rs` · `google.rs` | adaptéry kalendářů účtů Microsoft / Google (`adapter` pro `source::adapter`, `GraphSource`/`GoogleSource` pro `accounts::calsync`); Připojit z `onlineMeeting.joinUrl` / `hangoutLink` přes `meeting::classify` (`graph::join_of`) — mapa [accounts.md](accounts.md) | `parse`, `fetch`, `adapter`, `join_of`, `GraphSource`, `GoogleSource` |
| `cache.rs` | formát cache na disku (`calendar-cache.json`, klíč = otisk zdroje, v1, stáří 7 dní; stará cache bez `join` se načte) | `CacheFeed`, `encode_cache`, `decode_cache`, `cache_path` |
| `tests.rs` | celá cesta nad `fixtures/calendar-google.ics`, `calendar-outlook.ics`, `calendar-meetings.ics` (pozvánky Teams/Meet/Zoom/Webex/Jitsi) + malé feedy | — |

Napojení (jednořádkové): `threads/ui.rs` (otevření šuplíku), `threads/emit.rs` (`deadlines` + `tick` +
`maybe_refresh`; `remind::next_deadline` + `remind::tick`), `commands/drawer.rs` `set_mode`, `settings.rs`
`apply_config` → `on_config`, `snapshot.rs` `PersonalView.calendar`, `config/*` (`sections!` → `calendar`,
`PersonalConfig.calendars`, `validate_field`), `stripview.rs` (`remind::strip_now`: segment + ambientní
barva), `attention.rs` (`Event::Meeting`), `toast.rs` (`toast_xml_actions`), `commands/calendar.rs`
(`calendar_test`, `calendar_join`, `calendar_outlook_probe`, `calendar_outlook_load`), `lib.rs` (registrace).
Kalendáře účtů bez řádku v `personal.calendars`: `live::view` přidá za ně zdroje z `accounts::calsync::infos`
(`CalInfo.account`; čip v kartě vede do Nastavení → Účty; bez připomínky). Řádek `graph`/`google` má přednost.

### Zdroje (`source.rs`)

| `source` | odkud | kdy se načítá |
|---|---|---|
| `ics` (výchozí, starý config) | tajný odkaz | viditelná karta, data starší 15 min |
| `outlook-local` | klasický Outlook přes COM | viditelná karta **a Outlook běží**; zavřený = jen klik „Načíst z Outlooku“ (`load_outlook`, COM ho spustí na pozadí) |
| `graph` / `google` (+ `account`) | účet Microsoft / Google (`graph.rs` / `google.rs`) | jako ICS (viditelná karta, 15 min); nepřihlášený účet = chyba zdroje |

**Kontrakt pro účty** (worker Q): `type Adapter = fn(&CalendarFeed, Window) -> Result<Vec<Event>, String>`
— běží na blokujícím vlákně (`spawn_blocking`), vlastní timeout, výskyty v okně `Window { now, from, to }`
s rozbalenými opakováními, `Event` = `Occ` (UTC ms, `all_day`, `title`, `location`, `join` — odkaz jde
vytáhnout `meeting::find` / `offer`). Chyba = lidská věta bez tokenu. Id zdroje = `source::key` (otisk
`druh:účet`), název = `CalendarFeed.name` (prázdný → `"Outlook"`/doména), barva = `CalendarFeed.color`.
Zapojení = jedna větev v `source::adapter` + dlaždice v `CalendarSettings.tsx` `SOURCES` (účty vedou do Nastavení → Účty).

### Outlook v PC (`outlook.rs`)

`installed` = `CLSIDFromProgID("Outlook.Application")` (jen registr, nic nespouští); `running` = proces
`OUTLOOK.EXE`. `CoCreateInstance` při zavřeném Outlooku spustí `OUTLOOK.EXE -Embedding` na pozadí (bez
okna; s profilem „vždy se ptát“ vyskočí výběr profilu) — proto automaticky jen k běžící instanci. Čte se
`GetNamespace("MAPI")` → `GetDefaultFolder(9)` → `Items` → `Sort("[Start]")`, `IncludeRecurrences`,
`Restrict` (datum v místním krátkém formátu přes `VarFormatDateTime`), `GetFirst`/`GetNext`, mez
`MAX_OCCS` a 8 s. **Ochrana objektového modelu:** Subject, Start/End, StartUTC/EndUTC, AllDayEvent,
Location, MeetingStatus nehlídá; `Body` (odkaz Teams v pozvánce) jen při `Application.IsTrusted`, jinak se
nečte (žádná výzva Outlooku, Připojit pak jen z místa). Nový Outlook COM nemá → UI to řekne a nabídne ICS.

### Připojit (`meeting.rs`)

Pořadí zdrojů v události: `X-MICROSOFT-SKYPETEAMSMEETINGURL` / `X-GOOGLE-CONFERENCE` / `CONFERENCE`
→ `URL` → `LOCATION` → `DESCRIPTION` / `X-ALT-DESC`. Jen https na známých doménách a cestách (Teams
`/l/meetup-join/`, `/meet/`; Meet `abc-defg-hij`; Zoom `/j/ /w/ /my/ /s/`; Webex `j.php`, `/meet/`; Jitsi
`meet.jit.si`, `8x8.vc`), nápovědy a „Meeting options“ ne. Klik: `calendar_join` znovu ověří (`classify`)
a otevře `launch_uri` — `msteams:` jen pro `teams.microsoft.com` s ověřenou cestou a nainstalovanými
Teams (`AssocQueryStringW`), jinak https (při chybě aplikace fallback na https).

### Připomínka (`remind.rs`)

`remindMin` (výchozí 5, 0 = vypnuto) per kalendář. V proužku `📅 Název za N min` / `začíná` modře
(`Rgba::BLUE`; styly zavřeného Ledge — ostrůvek… — ji dostanou jako `Facts.reminder`, když není poznámka po termínu, `stripview::look_facts`) — ambientní barva s prioritou **čeká na tebe / připomínka poznámky (amber) > schůzka (modrá) >
pád serveru (červená) > bliknutí „hotovo“** (`attention::Event::rank`: NeedsInput 0, Meeting 1, Done 2).
Při vstupu do okna jednou `attention::signal` (bliknutí + puls) a s `remindToast` toast s tlačítkem Připojit
(aktivace protokolem, Ledge nebere fokus). Mrtvý Ledge (fullscreen hra / herní režim) nic; toast ne při
„nerušit“ / prezentaci / otevřeném šuplíku. Budík = `next_alarm` (začátek okna, začátek, konec „začíná“);
data z paměti — po startu `ensure_cache` načte cache z disku i se skrytou kartou; kvůli připomínce se nic
nestahuje (čerstvost = poslední otevření karty / načtení z Outlooku).

**Výkon:** nic nepolluje. ICS i Outlook jen s viditelnou kartou (šuplík Full a karta Kalendář na otevřené
záložce — `layout::page_has`), data starší `MAX_AGE_MS` (15 min). Emit vlákno dostane termín karty jen
s viditelnou kartou (`view::pick` → `next_change`), termín připomínky jen s kalendářem s připomínkou.
V paměti jen výskyty v okně (nejvýš `MAX_OCCS` na zdroj); text odpovědi i popisy událostí se po parsování
zahodí (zůstane jen odkaz Připojit).

**Tajemství:** adresa se nikdy neloguje (reqwest chyby přes `without_url`), chyby ji neobsahují (testy
`validate_table_never_leaks_url`, `cache_roundtrip_and_age`), UI dostává jen `domain` (jiné zdroje prázdné).

## UI `src/calendar/`

| soubor | co dělá |
|---|---|
| `CalendarCard.tsx` | karta (Dnes/Zítra/Příští, prázdné a chybové stavy, výzva „Přidat kalendář“, patička s vypnutím jedním klikem); u schůzky „za N min“ a **Připojit** (`api.calendarJoin`); Outlook zavřený → „Načíst z Outlooku“; `openCalendarSettings` |
| `CalendarSettings.tsx` | editor v Nastavení → Osobní (kotva `f-personal.calendars` je `Section` v `settings/panels/Personal.tsx`): dlaždice zdrojů `SOURCES` (Odkaz ICS / Outlook v tomto PC; účty doplní Q), barva, název, zapnutí, Otestovat / Změnit odkaz (ICS) nebo Načíst z Outlooku, připomínka (`Remind`: minuty předem + oznámení), Odebrat s Vrátit, návod |
| `model.ts` (+ `model.test.mjs`) | barvy, `normalizeUrl` (zrcadlo Rust), `domainOf`, `timeLabel`, `withCalendar`; `sourceOf`, `providerName`, `placeLabel`, `joinable`, `startsIn`, `remindOptions`, `remindOf`, `outlookFeed` |
| `calendar.css` | jen třídy `cal-*` |

Angličtina: `src/i18n/en-calendar.ts`. Typy: `types/calendar.ts` `CalendarView`, `CalendarItem` (`join`), `CalendarJoin`,
`CalendarSource`, `CalendarDay`, `CalendarInfo` (`source`, `hint`), `CalendarTest`, `OutlookProbe`;
`api.calendarTest`, `api.calendarJoin`, `api.calendarOutlookProbe`, `api.calendarOutlookLoad`. Sekce
`calendar` v `cards/sections.ts`.
