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
| `view.rs` | `any_ongoing` (probíhá teď časovaná událost? pro klid pulsu GitHubu, `live::meeting_now`); výběr karty: Dnes (bez skončených) / Zítra / první další den, řazení, strop řádků, čas další změny; `Item.join` | `pick`, `Day`, `Item`, `Picked` |
| `conf.rs` | config `personal.calendars` (`source`, `account`, `remindMin`, `remindToast`), adresa jako tajemství (webcal→https, jen https, doména, otisk), validace podle zdroje | `CalendarFeed`, `normalize_url`, `domain`, `key`, `validate`, `DEFAULT_REMIND_MIN` |
| `source.rs` | **zdroj jako abstrakce**: druhy, okno, sdílený typ událostí, klíč stavu, cíl načtení, dispatch, kontrakt adaptéru účtů | `SourceKind`, `Event` (= `Occ`), `Window`, `Adapter`, `adapter`, `Target`, `key`, `targets`, `fetch`, `finish`, `unavailable` |
| `outlook.rs` | zdroj `outlook-local`: detekce bez spuštění, tenká vrstva IDispatch, okno přes `Restrict`, převod položek (čistý) | `installed`, `running`, `events`, `RawItem`, `to_occ`, `naive_from_ole`, `restrict_filter`, `missing`, `closed` |
| `people.rs` | (L85) účastníci: `Occ.people` (`Crowd` = max `MAX_PEOPLE` 8 + `more`), `person` / `resp` (declined → pryč), `from_names` (Outlook); odmítnutí a místnosti se nekreslí | `Crowd`, `Person`, `Resp`, `person`, `resp`, `from_names`, `MAX_PEOPLE` |
| `meeting.rs` | odkaz Připojit: vytažení odkazů z textu, rozbalení Safe Links / `google.com/url`, rozpoznání Teams/Meet/Zoom/Webex/Jitsi, `msteams:` jen bezpečně | `find`, `classify`, `offer`, `rank_of`, `RANK_*`, `Join`, `Provider`, `app_url`, `launch_uri` |
| `remind.rs` | připomínka schůzky: okno [začátek − předstih, začátek + 2 min), budík, text proužku, toast s Připojit, jednou na schůzku | `active`, `next_alarm`, `fresh`, `strip_text`, `toast_text`, `next_deadline`, `tick`, `strip_now`, `GRACE_MS` |
| `span.rs` | (L85, kolo 33) délka schůzky v zavřeném proužku: čisté `meeting_span` (zrcadlo TS `meetingSpan`, stejná tabulka hranic), `pick` (běžící dřív, pak nejbližší do 10 min), `tail` „zbývá 12 min“ / „za 8 min“; bez časovače (minutová hodina emit vlákna) | `meeting_span` `Span` `pick` `tail` `min_label` `STRIP_BEFORE_MIN` |
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
(`CalInfo.account`; čip v kartě vede do Nastavení → Obsah karet → Kalendáře → Pokročilé; bez připomínky). Řádek `graph`/`google` má přednost.

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
Zapojení = jedna větev v `source::adapter` + dlaždice v `CalendarSettings.tsx` `SOURCES` (jen ICS a Outlook v PC; účty se přihlašují v Pokročilé pod kalendáři).

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
Při vstupu do okna jednou `attention::signal` (bliknutí + puls) a s `remindToast` (výchozí zapnuto; `remind::toast_wanted` = jen se schůzkou s odkazem) toast s tlačítkem Připojit
(aktivace protokolem, Ledge nebere fokus). Mrtvý Ledge (fullscreen hra / herní režim) nic; toast ne při
„nerušit“ / prezentaci / otevřeném šuplíku. Budík = `next_alarm` (začátek okna, začátek, konec „začíná“);
data z paměti — po startu `ensure_cache` načte cache z disku i se skrytou kartou; stahování kvůli připomínce (zavřená karta) řeší jen `bgfetch.rs` (≥ 15 min);
čerstvost jinak = poslední otevření karty / načtení z Outlooku.
Délka v proužku (L85): mimo okno připomínky `strip_now` → `span_text` — běžící schůzka „📅 Název · zbývá 12 min“, nadcházející do 10 min „… · za 8 min“ (`span::pick`); jen u kalendářů s `remindMin > 0` (data z paměti), celodenní nic. Žádný nový budík: text se přepočítá s minutovou hodinou emit vlákna (`next_wake`), `refresh_strip` běží po každém probuzení.

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
| `CalendarCard.tsx` | karta (Dnes/Zítra/Příští, prázdné a chybové stavy, výzva „Přidat kalendář“, patička s vypnutím jedním klikem); u schůzky „za 8 min · 30 min“ / „běží 35 min · zbývá 12 min“ (`meetingSpan`, z hodin aplikace), iniciálové avatary účastníků (max 4 + „+N“, tooltip se jmény) a **Připojit** (`api.calendarJoin`); Outlook zavřený → „Načíst z Outlooku“; `openCalendarSettings` |
| `CalendarSettings.tsx` | editor v Nastavení → Šuplík a karty → Obsah karet (kotva `f-personal.calendars` je `Section` v `settings/panels/Personal.tsx`): dlaždice zdrojů `SOURCES` (Odkaz ICS / Outlook v tomto PC; účty doplní Q), barva, název, zapnutí, Otestovat / Změnit odkaz (ICS) nebo Načíst z Outlooku, připomínka (`Remind`: minuty předem + oznámení), Odebrat s Vrátit, návod |
| `model.ts` (+ `model.test.mjs`) | barvy, `normalizeUrl` (zrcadlo Rust), `domainOf`, `timeLabel`, `withCalendar`; `sourceOf`, `providerName`, `placeLabel`, `joinable`, `meetingSpan`, `minLabel`, `initials`, `avatarColor`, `avatarRow` (L85: délka schůzky a avatary účastníků), `remindOptions`, `remindOf`, `outlookFeed` |
| `calendar.css` | jen třídy `cal-*` |

Angličtina: `src/i18n/en-calendar.ts`. Typy: `types/calendar.ts` `CalendarView`, `CalendarItem` (`join`), `CalendarJoin`,
`CalendarSource`, `CalendarDay`, `CalendarInfo` (`source`, `hint`), `CalendarTest`, `OutlookProbe`;
`api.calendarTest`, `api.calendarJoin`, `api.calendarOutlookProbe`, `api.calendarOutlookLoad`. Sekce
`calendar` v `cards/sections.ts`.

## Účastníci a délka schůzky (kolo 33, L85)

Bez nového oprávnění, klíče ani volby. `Occ.people` (`people::Crowd`) → `view::Item.people` → `CalendarItem.people` (`src/types/calendar.ts`).
Zdroj bez účastníků = pole se vůbec nesériuje a karta avatary nekreslí (žádné divadlo); stará cache bez pole se načte.

| zdroj | účastníci | co zdroj vrací / nevrací |
|---|---|---|
| ICS | ano (`ATTENDEE`: `CN`, e-mail, `PARTSTAT`) | bez DECLINED, `CUTYPE` RESOURCE/ROOM a `ROLE=NON-PARTICIPANT`; feed bez `ATTENDEE` (soukromé události s jedním účastníkem, tajné adresy některých služeb) = nic |
| Graph | ano (`$select=…attendees`: jméno, adresa, `status.response`) | bez declined a `type=resource` |
| Google | ano (`fields=attendees(self,responseStatus,displayName,email,resource)`) | bez declined a `resource`; bez `displayName` jen e-mail |
| Outlook COM | jen jména (`RequiredAttendees` + `OptionalAttendees`), bez odpovědí a e-mailů | **jen při `IsTrusted`** (ochrana objektového modelu, jako `Body`); jinak žádné; neověřeno na reálném Outlooku |

Délka: čistá `meetingSpan(it, now)` v `model.ts` — před schůzkou do 60 min pilulka „za 8 min · 30 min“ (nahradila `startsIn` / „za N min“ do 15 min), během ní řádek „běží 35 min · zbývá 12 min“; celodenní, nulová a skončená = nic. Počítá se z `p.now` (hodiny aplikace) při kreslení, žádný nový časovač.
Iniciály `initials(name, email)` (CZ diakritika, „Příjmení, Jméno“, e-mail = první písmeno) a barva `avatarColor` (hash e-mailu / jména → `CAL_COLORS`), `avatarRow` (max 4 + „+N“, tooltip se jmény, dedup).

## Zápis „Do kalendáře“ (kolo 33, L74)

Bez nového oprávnění a bez nové volby. Položka v ⋯ menu poznámky (`src/notes/NoteMenu.tsx`, props `title`, `remind`) se nabídne jen když poznámka má `remind` nebo první otevřený bod `- [ ]` má `⏰` (`pickCalEvent`; remind má přednost). Konstanty 30 min / upozornění 10 min (`CAL_DUR_MIN`, `CAL_REMIND_MIN`).
Příkaz `calendar_add({req})` (`commands/calendar.rs`, jen šuplík — `capabilities/default.json`; volá se jen přímo z `calendar/api.ts`, wrapper v `api.ts` smazán): `{title, startIso, durMin?, remindMin?, allDay?, uid, target: "ics"|"connector"}` → `{target, id, duplicate}`.
- `ics`: soubor `%TEMP%Ledge-icsledge-<hash>.ics` (starší než 24 h se smažou při dalším zápisu), otevře výchozí aplikací a schová šuplík — **otevře okno cizí aplikace, volat jen na výslovný klik**. Čas do UTC (`DTSTART:…Z`), celodenní `VALUE=DATE` (DTEND = další den, bez VALARM), `TRIGGER:-PT<n>M`, UID = `<hash>@ledge` ze semene `cesta|zdroj|název` BEZ času (odložení `⏰` = aktualizace téže události; `SEQUENCE` = minuty od epochy z `DTSTAMP`).
- `connector`: `client::call("create_event")` se stejným klíčem a adresou jako odeslání úkolů (`commands/connector.rs` `endpoint`, `need_key`); před odesláním náhled v `CalAddDialog`. Argumenty: `name` (povinné, ≤ 500), `date` `YYYY-MM-DD` | `startDateTime` + `endDateTime` (ISO s offsetem, kontroluje `isValidDeadline`), `description`, `clientRequestId` (8–128; semeno = uid + čas: nový čas = nová událost); odpověď `{ok, eventId, name, status[, deduplicated]}`. Chybu „klíč nesmí zapisovat události“ ohlásí Ledge podle odpovědi serveru; upozornění se neposílá; typ, priorita, stav = výchozí.
Testy: Rust `calendar/write.rs` (tabulky času, uid, escapování, zalomení UTF-8, přes půlnoc, celodenní, argumenty propojené aplikace), `src/calendar/write.test.mjs`. Neověřeno: živé volání `create_event` proti skutečné propojené aplikaci (testy nikdy nevolají skutečný server) a otevření `.ics` v konkrétním kalendáři.
