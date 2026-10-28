# Codemapa: Osobní — právě hraje, hlasitost, baterie

Tři malé zdroje živých dat pro záložku Osobní. Společné pravidlo: události, žádný polling; nic nevznikne, dokud UI neukáže záložku Osobní, a skrytá karta odběry zruší. Výjimka: položka proužku `media` (`strip.items`) nastartuje GSMTC hned při startu / uložení configu i bez záložky Osobní.

Detail (toky, invarianty, testy, co nečíst, historie): [media-detail.md](media-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `media.rs` | GSMTC (`Windows.Media.Control`, WinRT): vlákno `media` (COM MTA) drží odběr `CurrentSessionChanged` + u aktuální session `MediaPropertiesChanged` / `PlaybackInfoChanged` / … | `MediaView` `timeline` `shuffle` `repeat` `Control` `ensure` `sync` `view` `control` |
| `mediatime.rs` | průběh skladby a režimy: čtení timeline (`read`: pozice, délka, `LastUpdatedTime`, rychlost, `IsPlaybackPositionEnabled`), `modes` (náhodně/opakovat jen s `IsShuffleEnabled` / `IsRepeatEnabled`) … | `Timeline`, `Repeat`, `Modes`, `merge`, `position_at` |
| `mediaconf.rs` | config `media` (`variant` compact/card/cover/vinyl/strip, `stripTime` auto/on/off, `equalizer`, `tint` (podbarvení obalem, výchozí vyp.), `favorites` ≤ 24 `{name, uri}`) + oblíbené … | `MediaConfig` `Variant` `StripTime` `Favorite` `Source` `source_of` `validate` |
| `volume.rs` | (kolo 22: `allowed` i s kartou Rychlé přepínače `layout::TOGGLES_CARD`, [board.md](board.md)) Core Audio … | `VolumeView` `AudioDevice` `ensure` `sync` `view` `set` `mute` `mic_mute_toggle` |
| `battery.rs` | `GetSystemPowerStatus`; na stroji s baterií `PowerSettingRegisterNotification` (`DEVICE_NOTIFY_CALLBACK`, bez okna i vlákna) pro `GUID_ACDC_POWER_SOURCE` a `GUID_BATTERY_PERCENTAGE_REMAINING` → … | `Raw`, `BatteryView`, `ensure`, `sync`, `view` |
| `commands/media.rs` | Tauri: `personal_live` (UI ukázalo Osobní → `ensure` všech tří), `media_favorite_open` (klik na oblíbenou: `index` + `uri` musí sedět s configem, tvar znovu `source_of`, otevře `opener.open_url` … | "track" |
| `src/media/MediaCard.tsx` | karta Právě hraje, pět vzhledů (`Body`: `med-v-compact`, výchozí karta, `med-v-cover` s rozmazaným obalem `med-backdrop`, vinyl `med-disc`, `med-v-slim` tenký ovladač bez hlavičky) + hlasitost (jen … | `MediaCard` `PersonalLive` |
| `src/media/tint.ts` | (kolo 23: 32×32, `withContrast` ≥ 3:1 proti `--card`, `coverAccent` s cache 4 obalů; `model.ts` `outputPickerShown`) podbarvení karty barvou obalu (`media.tint`, výchozí vyp. … | `pickTint` `coverTint` |
| `src/media/VolumeBar.tsx` | ovladač hlasitosti (ztlumit, posuvník s místní hodnotou při tahu, %), tlačítko mikrofonu (jen s mikrofonem), výběr výstupu `PopMenu` (jen s víc výstupy) | `VolumeBar` |
| `src/media/FavoriteTiles.tsx` · `favorites.ts` · `src/media/api.ts` | část karty `favorites` (`PARTS`, ⋯ → Skrýt sekci): dlaždice s ikonou zdroje → `favApi.open`; „☆ Přidat, co hraje“ = inline formulář (název z GSMTC `suggestedName`, odkaz z historie schránky … | `Favorites` `saveFavorites` `favErrText` `favApi` |
| `src/media/FavoritesSettings.tsx` | Nastavení → Šuplík a karty → Obsah karet → Přehrávač → Oblíbené: přidat, přejmenovat, pořadí (`SortList`), smazat s Vrátit; mění jen návrh Nastavení | `FavoritesSettings` |
| `src/media/Player.tsx` | stavebnice: `useCover` (jedna data URL na kartu), `Cover`, `Progress` (lišta: klik/tah/šipky = `{seek}`, jen s `canSeek` … |  |
| `src/media/VariantPicker.tsx` | ikonka v rohu karty → nabídka s miniaturami (`Thumb`), uložení `media.variant` (`get_config` → `save_config`), „Vrátit“, okamžitý vzhled do snapshotu (`shownVariant`) | `VariantPicker` `Thumb` `saveVariant` |
| `src/media/MediaSettings.tsx` | Nastavení → Šuplík a karty → Obsah karet → Přehrávač: vzhled s náhledy, čas v proužku, ekvalizér (společná lišta Uložit/Vrátit) |  |
| `src/media/timeline.ts` · `src/media/shown.ts` | čistá logika (`positionAt`, `shownPosition`, `fmtDuration`, `seekTarget`, `seekKey`, `nextTickIn`, `variantOf`, `nextRepeat`) — test `timeline.test.mjs` … |  |
| `src/media/Battery.tsx` | řádek baterie pod hodinami v `cards/ClockCard.tsx` | `Battery` |
| `src/media/model.ts` | čistá logika: `remaining` (sekundy → h/min), `batteryLow`, `volumeLevel` (ikona), `throttle` (posuvník) — test `model.test.mjs` |  |
| `src/media/media.css` · `src/i18n/en-media.ts` | styly (prefix `med-`) · angličtina `cards.media.*`, `cards.sound.*`, `cards.battery.*` |  |
