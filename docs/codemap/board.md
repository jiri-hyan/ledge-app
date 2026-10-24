# Codemapa: nové widgety kola 22 (`board`)

Čtyři karty z katalogu (výchozí rozložení beze změny, starý config se nemění): **Claude dnes** (`usage`), **Týden**
(`week`), **Velké hodiny** (`bigclock`), **Rychlé přepínače** (`toggles`). Každé číslo má zdroj, který Ledge už čte,
a štítek původu (vzor Codenotch): `odhad` = odvozené Ledgem (útrata z transcriptů podle ceníku API), `oficiální` =
hlásí ho přímo zdroj (Windows). Co zdroj nemá, je prázdné. Žádná nová síť, žádný nový časovač v klidu. Tři z nich jdou
i na plochu (`desk/**`: druhy `week`, `clock`, `usage`). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/board/model.ts` | čistá logika (test `model.test.mjs`): místní datum, útrata (posledních n dní, součet s cenou jen když ji den má, výšky sloupců, stav karty), časová osa (řádky ze `summary.hours`, úroveň buňky 0–4, rozsah hodin, přehled ze dneška), pás týdne od pondělí (dnešek, víkend, den s událostí), stav kalendáře a zbytek dneška, krok hlasitosti | `localDate` `lastDays` `sumDays` `barHeights` `usageState` `isToday` `hourRows` `heat` `hoursMax` `hourSpan` `weekStrip` `calState` `todayLeft` `stepVolume` `Origin` |
| `src/board/text.ts` | zkratky dnů a měsíců (i18n), text štítku původu | `dowShort` `monthShort` `originText` |
| `src/board/UsageCard.tsx` | karta Claude dnes: hrdinské číslo útraty dnes (+ tokeny), 7 dní, sloupce 7 dní (sdílený `src/charts/Bars`), časová osa „Co Claude dnes dělal“ (projekty × hodiny), stavy vypnuto / počítám / chyba; v šuplíku při `view === "full"` `api.costsRefresh()` (Rust 1× / 60 s), na ploše nic nevolá | `UsageCard` `OriginTag` |
| `src/board/WeekCard.tsx` | karta Týden: velký měsíc + čas, pás 7 dnů, pod ním zbytek dneška z kalendáře; bez kalendáře „Bez kalendáře…“ + Připojit, s kalendářem bez událostí „Nic na dnešek“ | `WeekCard` |
| `src/board/BigClockCard.tsx` | karta Velké hodiny: hodiny nad minutami obřími tabulkovými číslicemi, sekundy jen v šuplíku (na ploše čas po minutách), datum a ISO týden | `BigClockCard` |
| `src/board/TogglesCard.tsx` | karta Rychlé přepínače: dlaždice Zvuk (ztlumit), Mikrofon (jen s mikrofonem), Baterie (jen stroj s baterií, jen ukazuje), hlasitost − / + po 5 % (`api.volumeSet` / `volumeMute` / `volumeMicMute`). Jas záměrně ne (bez práv správce ho Windows spolehlivě nastavit nedovolí) | `TogglesCard` |
| `src/board/board.css` · `src/board/en.ts` | styly (prefix `bd-`, jen tokeny) · angličtina `cards.board.*`, katalog `cards.layout.card/desc.{usage,week,bigclock,toggles}`, plocha `cards.desk.kind.{week,clock,usage}` | |

## Zdroje dat (Rust)

- **Claude dnes:** `usage/index.rs` drží navíc hodinovou agregaci `hours` (`HourKey` den × místní hodina × `cwd`, jen
  `HOUR_DAYS` = dnešek a včerejšek; `Window.hour_of` = `costslive::local_hour`), index `VERSION` 2 → jednou přepočet od
  nuly (starý index hodiny nemá). `usage/view.rs` `hour_rows` → `Summary.hours` (`HourRow`: projekt, 24 čísel, top
  `TOP` + „ostatní“). `costslive::enabled` = `wanted(útrata, sekce v kartě Claude, karta `USAGE_CARD` na záložce)`;
  widget na ploše: `request_desk` (nejvýš 1× / `DESK_EVERY` = 15 min) a `desk_view` (stačí zapnutá útrata).
- **Týden:** kalendář z cache `calendar::live` — `layout::WEEK_CARD` se počítá do `section_on("calendar")` a
  `page_has_calendar` (stahuje se jen s kartou na otevřené záložce, jako karta Kalendář; žádné nové volání).
- **Rychlé přepínače:** `volume::allowed` a `battery::allowed` berou i `layout::TOGGLES_CARD`; `settings::apply_config`
  je přesynchronizuje i beze změny `drawer.sections`; snapshot filtruje `volume` / `battery` podle `allowed`
  (karta Právě hraje / Čas hlídá svou část sama). Odběr startuje `PersonalLive` (MainPage, i s kartou `toggles`).
- **Velké hodiny:** systémový čas (`now` ze skořápky; tik jen při otevřeném šuplíku).

Lepidlo (cizí soubory, ucelené bloky): `layout.rs` (`CARDS` +4, `WEEK_CARD`, `TOGGLES_CARD`), `src/layout/model.ts`
(`CARD_IDS` +4, `CARD_INFO`), `src/layout/model.test.mjs` (katalog), `src/MainPage.tsx` (větve `card`, `PersonalLive`),
`src/costs/model.ts` (`CostHourRow`, `summary.hours`), `dataexport.rs` (test `Summary`), `src/i18n/dicts.ts`.
Plocha: [desk.md](desk.md) (`Kind::Week|Clock|Usage`, `DeskData.costs`, `views.tsx`, `kindsOfCard`).

## Co je jinde (nezdvojeno)

- Systém s mini grafy = existující karta Systém (`integrations/SystemCard.tsx`, historie `sysmon` jen při vzorkování
  viditelnou kartou) — [integrations.md](integrations.md).
- Přehrávač s obalem a barvou z obalu = karta Právě hraje (vzhledy `media.variant`, barva obalu `media/tint.ts`
  `pickTint`, volba `media.tint`; kolo 22: barva obalu je zároveň jediný akcent karty) — [media.md](media.md).
- Spouštěč aplikací = karta Odkazy a akce (skupiny, vzhledy) a import Launcheru — [actions.md](actions.md).
- Počasí s předpovědí: `weather.rs` stahuje jen dnešek (`forecast_days=1`); předpověď by měnila požadavek — nedělá se.

## Testy

UI `src/board/model.test.mjs` (tabulky výše), `src/desk/model.test.mjs` (`kindsOfCard`, zrcadlo `Kind::parse`),
`src/layout/model.test.mjs` (zrcadlo `CARDS`, katalog). Rust `usage::index::tests::hours_today_and_yesterday_only`,
`old_index_version_is_rebuilt`, `usage::view::tests::hours_rows_today_by_project`, `costslive::tests::wanted_table`,
`local_hour_in_range`, `volume::tests::allowed_by_config` (karta přepínačů), `desk::tests::kinds_parse_and_key`.
