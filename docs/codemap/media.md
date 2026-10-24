# Codemapa: Osobní — právě hraje, hlasitost, baterie

Tři malé zdroje živých dat pro záložku Osobní. Společné pravidlo: **události, žádný polling**; nic
nevznikne, dokud UI neukáže záložku Osobní, a skrytá karta odběry zruší. Výjimka: položka proužku `media`
(`strip.items`) nastartuje GSMTC hned při startu / uložení configu i bez záložky Osobní.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `media.rs` | GSMTC (`Windows.Media.Control`, WinRT): vlákno `media` (COM MTA) drží odběr `CurrentSessionChanged` + u aktuální session `MediaPropertiesChanged` / `PlaybackInfoChanged` / `TimelinePropertiesChanged`, handler jen pošle zprávu; vlákno čte vlastnosti s termínem 3 s (makro `wait!` nad `.when`), klid 150 ms pro nárazové události. Zpráva jen o timeline (`Msg::Timeline` → `publish_timeline`) čte synchronně jen timeline, bez dotazu na skladbu. Ovládání (`Control`: toggle/next/prev + `Seek(ms)`, `Shuffle(bool)`, `Repeat`; `control` odmítne, co zdroj neumí), obal (Storage.Streams → `iconimg::to_png` → data URL, cache 1 kus). Čisté: `app_name` (AUMID → název), `status_of`, `cover_key`, `needed` (karta NEBO proužek), `strip_wants` (položka `media` nebo vzhled `strip`) | `MediaView` (+ `timeline`, `shuffle`, `repeat`), `Control`, `ensure`, `sync`, `view`, `control`, `cover`, `strip_now`, `next_deadline_live`, `allowed`, `needed`, `strip_wants` |
| `mediatime.rs` | průběh skladby a režimy: čtení timeline (`read`: pozice, délka, `LastUpdatedTime`, rychlost, `IsPlaybackPositionEnabled`), `modes` (náhodně/opakovat jen s `IsShuffleEnabled` / `IsRepeatEnabled`), `seek` / `set_shuffle` / `set_repeat`. Čisté: `from_ticks` (délka < 1 s = žádná timeline), `position_at`, `same_course` + `merge` (hlášení potvrzující průběh se zahodí; pauza bez obnovené timeline se zmrazí, pokračování se starým hlášením platí od teď), `seek_ticks`, `next_minute`, `strip_time` („1/4 min“) | `Timeline`, `Repeat`, `Modes`, `merge`, `position_at` |
| `mediaconf.rs` | config `media` (`variant` compact/card/cover/vinyl/strip, `stripTime` auto/on/off, `equalizer`, `tint` (podbarvení obalem, výchozí vyp.), `favorites` ≤ 24 `{name, uri}`) + oblíbené: `source_of` (povolený tvar odkazu: `spotify:<druh>:<id>` nebo http(s) bez jména v adrese → Spotify / YouTube Music / web), `validate` (cesta `media.favorites.N.uri`); a segment proužku: `strip_text`, `strip_segment` („♪ Název · Interpret[ · 1/4 min]“, ozdoba `EQ_GLYPH` u `strip`), `with_strip_item` (vzhled `strip` doplní položku `media`), `strip_has_item`, `shows_time`, `shows_eq`, `next_deadline` (budík `emit` na změnu minuty) | `MediaConfig`, `Variant`, `StripTime`, `Favorite`, `Source`, `source_of`, `validate`, `MAX_FAVORITES` |
| `volume.rs` | (kolo 22: `allowed` i s kartou Rychlé přepínače `layout::TOGGLES_CARD`, [board.md](board.md)) Core Audio: vlákno `volume` (COM MTA), `IMMDeviceEnumerator` → výchozí výstup (`eRender`/`eMultimedia`) → `IAudioEndpointVolume`; `IAudioEndpointVolumeCallback` (změna zvenku → paměť) a `IMMNotificationClient` (jiné výchozí zařízení → `Rebind`). Z nahromaděných `Set` platí poslední. Název zařízení z `PKEY_Device_FriendlyName`. Kolo 18: seznam aktivních výstupů (`devices`), přepnutí výchozího přes `IPolicyConfig` (jen id, které jádro nabídlo — `known_device`; selhání = Nastavení zvuku Windows), výchozí mikrofon s vlastním callbackem `MicCb` (`mic_muted`, `None` = bez mikrofonu; změna výchozího vstupu = `Rebind`). Čisté: `to_pct`, `to_scalar`, `known_device` | `VolumeView`, `AudioDevice`, `ensure`, `sync`, `view`, `set`, `mute`, `mic_mute_toggle`, `set_default_output` |
| `battery.rs` | `GetSystemPowerStatus`; na stroji s baterií `PowerSettingRegisterNotification` (`DEVICE_NOTIFY_CALLBACK`, bez okna i vlákna) pro `GUID_ACDC_POWER_SOURCE` a `GUID_BATTERY_PERCENTAGE_REMAINING` → callback přečte stav znovu. Desktop (`BatteryFlag` 128) nic neregistruje. Čisté: `interpret` (`Raw` → `BatteryView`), `has_battery` | `Raw`, `BatteryView`, `ensure`, `sync`, `view` |
| `commands/media.rs` | Tauri: `personal_live` (UI ukázalo Osobní → `ensure` všech tří), `media_favorite_open` (klik na oblíbenou: `index` + `uri` musí sedět s configem, tvar znovu `source_of`, otevře `opener.open_url` — stejně jako ostatní odkazy, žádný nový ShellExecute), `media_control` (`"toggle"` / `{"seek": ms}` / `{"shuffle": bool}` / `{"repeat": "none"|"track"|"list"}`), `media_cover` (async), `volume_set`, `volume_mute` | |
| `src/media/MediaCard.tsx` | karta Právě hraje, pět vzhledů (`Body`: `med-v-compact`, výchozí karta, `med-v-cover` s rozmazaným obalem `med-backdrop`, vinyl `med-disc`, `med-v-slim` tenký ovladač bez hlavičky) + hlasitost (jen hlasitost = karta „Zvuk“), `PersonalLive` | `MediaCard` `PersonalLive` |
| `src/media/tint.ts` | podbarvení karty barvou obalu (`media.tint`, výchozí vyp.; od kola 22 je barva obalu i jediný `--accent` karty — průběh, ekvalizér, režimy, `media.css` `.med-tinted`): čistá `pickTint` nad 8×8 RGBA (test `tint.test.mjs`), `coverTint` přes canvas jednou na obal | `pickTint` `coverTint` |
| `src/media/VolumeBar.tsx` | ovladač hlasitosti (ztlumit, posuvník s místní hodnotou při tahu, %), tlačítko mikrofonu (jen s mikrofonem), výběr výstupu `PopMenu` (jen s víc výstupy) | `VolumeBar` |
| `src/media/FavoriteTiles.tsx` · `favorites.ts` · `src/media/api.ts` | část karty `favorites` (`PARTS`, ⋯ → Skrýt sekci): dlaždice s ikonou zdroje → `favApi.open`; „☆ Přidat, co hraje“ = inline formulář (název z GSMTC `suggestedName`, odkaz z historie schránky `musicLinkFrom`, ne z hlavy — GSMTC odkaz nedává), `saveFavorites` (`get_config` → `save_config`). Čistá logika: `favoriteSource` (zrcadlo `source_of`, stejná tabulka v testech obou stran), `favoriteError`, `withFavorite`, `renamedFavorite`, `withoutFavorite` — test `favorites.test.mjs` | `Favorites` `saveFavorites` `favErrText` `favApi` |
| `src/media/FavoritesSettings.tsx` | Nastavení → Šuplík a karty → Obsah karet → Přehrávač → Oblíbené: přidat, přejmenovat, pořadí (`SortList`), smazat s Vrátit; mění jen návrh Nastavení | `FavoritesSettings` |
| `src/media/Player.tsx` | stavebnice: `useCover` (jedna data URL na kartu), `Cover`, `Progress` (lišta: klik/tah/šipky = `{seek}`, jen s `canSeek`; drží cíl do nové timeline), `TimeText`, `Controls` (+ náhodně/opakovat jen když je zdroj hlásí), `Equalizer` (ozdoba) — tik na celé sekundy jen při přehrávání s otevřeným šuplíkem | |
| `src/media/VariantPicker.tsx` | ikonka v rohu karty → nabídka s miniaturami (`Thumb`), uložení `media.variant` (`get_config` → `save_config`), „Vrátit“, okamžitý vzhled do snapshotu (`shownVariant`) | `VariantPicker` `Thumb` `saveVariant` |
| `src/media/MediaSettings.tsx` | Nastavení → Šuplík a karty → Obsah karet → Přehrávač: vzhled s náhledy, čas v proužku, ekvalizér (společná lišta Uložit/Vrátit) | |
| `src/media/timeline.ts` · `src/media/shown.ts` | čistá logika (`positionAt`, `shownPosition`, `fmtDuration`, `seekTarget`, `seekKey`, `nextTickIn`, `variantOf`, `nextRepeat`) — test `timeline.test.mjs`; `drawerShown` (události `view`/`closing` + `visibilitychange`) | |
| `src/media/Battery.tsx` | řádek baterie pod hodinami v `cards/ClockCard.tsx` | `Battery` |
| `src/media/model.ts` | čistá logika: `remaining` (sekundy → h/min), `batteryLow`, `volumeLevel` (ikona), `throttle` (posuvník) — test `model.test.mjs` | |
| `src/media/media.css` · `src/i18n/en-media.ts` | styly (prefix `med-`) · angličtina `cards.media.*`, `cards.sound.*`, `cards.battery.*` | |

## Toky

- **Start:** `MainPage` (Osobní) → `<PersonalLive />` → `api.personalLive()` → `media::ensure` / `volume::ensure` /
  `battery::ensure` (každý zkontroluje `allowed(cfg)` = `modes.personalEnabled` && svůj `drawer.sections.*`).
  `wanted` si pamatuje, že UI už chtělo — zapnutí karty v Nastavení pak spustí odběr hned (`sync`).
  GSMTC běží, když `needed(cfg, wanted)` = (`wanted` && `allowed`) || `strip_wants` (zapnutý proužek s položkou
  `media`); `lib.rs` volá `media::sync` při startu (bez položky nic nevznikne).
- **Config:** `settings::apply_config` při změně `drawer.sections` / `modes` → `sync(&new)` všech tří, při změně
  `strip_wants` ještě `media::sync` (nepotřebné = `Stop`, zrušení odběrů, `view` = `None`).
- **Proužek:** `stripview::refresh_strip` položka `Media` → `media::strip_now` (jen paměť); událost GSMTC →
  `publish` → `mark_dirty` → vlákno `emit` → `refresh_strip`. `stripconf::is_event`: `media` = událost, jen když
  má segment (hraje).
- **Data:** callback / vlákno přepíše statický `VIEW` a při změně `mark_dirty` → `emit` → snapshot
  `personal.{media, volume, battery}` (jen paměť, filtrované přepínači v `snapshot::personal_view`).
- **Viditelnost karty:** karta `media` v modelu rozložení (`layout/model.ts`, mapa [layout.md](layout.md));
  `MainPage.tsx` ji ukáže, když je co ukázat (části `nowPlaying` = skladba, `sound` = hlasitost, `favorites` = oblíbené); části v menu ⋯:
  `sound` (Právě hraje), `battery` (Čas) — `PARTS`.

## Průběh skladby a vzhledy

- Rust drží jen poslední hlášenou pozici + čas hlášení (`Timeline`), UI dopočítává (`positionAt`) a tiká
  (`nextTickIn`, setTimeout na další celou sekundu) **jen** při přehrávání s otevřeným šuplíkem (`shown.ts`).
  Deska (`med-disc-spin`) a ekvalizér (`med-eq-live`) stejně; omezení pohybu je vypne globálně.
- Přehrávač, který timeline hlásí průběžně, nic nebudí: `mediatime::merge` hlášení v toleranci 1,5 s zahodí
  (`VIEW` se nezmění → žádné `mark_dirty`).
- Proužek: čas jen po minutách (`media.stripTime`, budík `media::next_deadline_live` v `threads/emit.rs` —
  ≤ 1 probuzení za minutu při přehrávání), ozdoba ekvalizéru je statický znak (animace by překreslovala
  nativní proužek). Proužek je click-through a najetí otevře šuplík — „malý ovladač“ vzhledu `strip` je proto
  tenký řádek v šuplíku (`med-v-slim`), ne okno nad proužkem.
- Vzhled se mění jen tokeny ze `styles.css` (design styly); pevné barvy nemá nic kromě obalu alba.

## Pasti

- Callbacky Core Audio a GSMTC běží na cizích vláknech: jen paměť / zpráva do kanálu, žádné volání
  enumerátoru ani blokující dotaz uvnitř.
- `TryGetMediaPropertiesAsync` umí u zaseklé aplikace viset — proto `wait!` s termínem, ne `.join()`.
- Obal se čte jen na žádost UI (otevřený šuplík), klíč musí sedět na aktuální `coverKey`.
- Paměť: GSMTC a Core Audio načtou knihovny Windows až při prvním zobrazení Osobní — změř `scripts/perf.ps1`.

## Testy

Rust: `mediaconf` — i `favorite_source_table` (tabulka sdílená s `favorites.test.mjs`), `favorites_validation_table`, `favorites_serde_old_config_and_roundtrip`; `media` (`app_name`, `status_of`, `needed` karta/proužek, `cover_key`, `allowed` + výchozí `true` starého
configu), `mediatime` (`from_ticks`, `position_at`, `merge`, `seek_ticks`, `next_minute`, `strip_time`), `mediaconf`
(serde starého configu, `shows_time`/`shows_eq`/`with_strip_item`, `strip_text`, `strip_segment`, `next_deadline`), `stripconf` (`media` v serde, `is_event`, `compose`),
`volume` (`to_pct`/`to_scalar`), `battery` (`interpret`, `has_battery`). UI: `src/media/model.test.mjs`, `src/media/timeline.test.mjs`,
`src/cards/sections.test.mjs` (karta Právě hraje / Zvuk). Živé chování (události, ovládání) jen ručně.
