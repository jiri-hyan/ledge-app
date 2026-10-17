# Codemapa: Osobní — právě hraje, hlasitost, baterie

Tři malé zdroje živých dat pro záložku Osobní. Společné pravidlo: **události, žádný polling**; nic
nevznikne, dokud UI neukáže záložku Osobní, a skrytá karta odběry zruší. Výjimka: položka proužku `media`
(`strip.items`) nastartuje GSMTC hned při startu / uložení configu i bez záložky Osobní.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `media.rs` | GSMTC (`Windows.Media.Control`, WinRT): vlákno `media` (COM MTA) drží odběr `CurrentSessionChanged` + u aktuální session `MediaPropertiesChanged` / `PlaybackInfoChanged` (timeline NE), handler jen pošle zprávu; vlákno čte vlastnosti s termínem 3 s (makro `wait!` nad `.when`), klid 150 ms pro nárazové události. Ovládání, obal (Storage.Streams → `iconimg::to_png` → data URL, cache 1 kus). Čisté: `app_name` (AUMID → název), `status_of`, `cover_key`, `strip_text` / `strip_segment` (segment proužku „♪ …“ jen při přehrávání), `needed` (karta NEBO proužek), `strip_wants` | `MediaView`, `Control`, `ensure`, `sync`, `view`, `control`, `cover`, `strip_now`, `allowed`, `needed`, `strip_wants` |
| `volume.rs` | Core Audio: vlákno `volume` (COM MTA), `IMMDeviceEnumerator` → výchozí výstup (`eRender`/`eMultimedia`) → `IAudioEndpointVolume`; `IAudioEndpointVolumeCallback` (změna zvenku → paměť) a `IMMNotificationClient` (jiné výchozí zařízení → `Rebind`). Z nahromaděných `Set` platí poslední. Název zařízení z `PKEY_Device_FriendlyName`. Čisté: `to_pct`, `to_scalar` | `VolumeView`, `ensure`, `sync`, `view`, `set`, `mute` |
| `battery.rs` | `GetSystemPowerStatus`; na stroji s baterií `PowerSettingRegisterNotification` (`DEVICE_NOTIFY_CALLBACK`, bez okna i vlákna) pro `GUID_ACDC_POWER_SOURCE` a `GUID_BATTERY_PERCENTAGE_REMAINING` → callback přečte stav znovu. Desktop (`BatteryFlag` 128) nic neregistruje. Čisté: `interpret` (`Raw` → `BatteryView`), `has_battery` | `Raw`, `BatteryView`, `ensure`, `sync`, `view` |
| `commands/media.rs` | Tauri: `personal_live` (UI ukázalo Osobní → `ensure` všech tří), `media_control`, `media_cover` (async), `volume_set`, `volume_mute` | |
| `src/media/MediaCard.tsx` | karta Právě hraje (+ hlasitost; jen hlasitost = karta „Zvuk“), `PersonalLive` (volá `personal_live` při mountu Osobní), `Cover` (obal na `coverKey`) | `MediaCard` `PersonalLive` |
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
- **Viditelnost karty:** `cards/sections.ts` `visibleCards` (fakt `media` = je co ukázat), klíč karty
  `mediaCardKey` (`nowPlaying`, se skrytou skladbou `sound`); části v menu ⋯: `sound` (Právě hraje),
  `battery` (Čas) — `PERSONAL_PARTS`.

## Pasti

- Callbacky Core Audio a GSMTC běží na cizích vláknech: jen paměť / zpráva do kanálu, žádné volání
  enumerátoru ani blokující dotaz uvnitř.
- `TryGetMediaPropertiesAsync` umí u zaseklé aplikace viset — proto `wait!` s termínem, ne `.join()`.
- Obal se čte jen na žádost UI (otevřený šuplík), klíč musí sedět na aktuální `coverKey`.
- Paměť: GSMTC a Core Audio načtou knihovny Windows až při prvním zobrazení Osobní — změř `scripts/perf.ps1`.

## Testy

Rust: `media` (`app_name`, `status_of`, `strip_text`, `strip_segment` hraje/pauza/nic, `needed` karta/proužek,
`cover_key`, `allowed` + výchozí `true` starého configu), `stripconf` (`media` v serde, `is_event`, `compose`),
`volume` (`to_pct`/`to_scalar`), `battery` (`interpret`, `has_battery`). UI: `src/media/model.test.mjs`,
`src/cards/sections.test.mjs` (karta Právě hraje / Zvuk). Živé chování (události, ovládání) jen ručně.
