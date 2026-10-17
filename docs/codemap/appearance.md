# Codemapa: vzhled a animace (`appearance`)

Motiv, akcent, hustota, písmo, rohy, krytí pozadí, styl/rychlost animací, vzhled a pozice proužku,
pořadí karet. Config `appearance` (AGENTS.md → Konfigurace). Výchozí = vzhled před zavedením sekce.
Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/appearance.rs` | struktura configu, výchozí hodnoty, validace (jen nová pole), testy | `AppearanceConfig` `Theme` `StripLook` `StripPosition` `MotionLook` `validate` `CARD_IDS` `DRAWER_WIDTH` `is_hex` |
| `src-tauri/src/appearance/motion.rs` | pohyb okna šuplíku z `motion` (none = 0 ms, jinak 170 / rychlost); převod starého `drawer.animationMs` při `config::load` | `drawer_anim_ms` `legacy_animation_ms` `upgrade_loaded` `BASE_SLIDE_MS` |
| `src-tauri/src/appearance/geom.rs` | čistá geometrie a barvy pro Rust: osa Ledge u hrany, ořez šířky, tvar a paleta proužku, linka | `anchor_c2` `span_x` `axis2_for` `drawer_width` `strip_metrics` `palette` `ink` `line_stops` `accent_pair` |
| `strip.rs` (`draw`) | kreslí podle `StripModel.{style, metrics, palette, line, axis2, pulse}` (styl `dot` = jen tečky; puls = linka v rytmu `attention::pulse_level` roztažená k plné šířce) | `StripModel` `pulse_now` |
| `stripview.rs` (`refresh_strip`) | naplní vzhled modelu z configu | |
| `drawer.rs` | `set_anchor` + `place` (x z `anchor_c2`/`span_x`); šířku dostává oříznutou | `Drawer::set_anchor` |
| `threads/ui.rs` | před každým `UiCmd`: `set_anchor`, šířka = `geom::drawer_width` (≤ 95 % monitoru), délka animace = `motion::drawer_anim_ms` | |
| `threads/hover.rs` | zóna bez proužku leží na ose (`axis2_for`), jinak podle skutečného obdélníku proužku | |
| `settings.rs` (`apply_config`) | změna `appearance` → událost `appearance` do šuplíku; proužek přes `mark_dirty` | |
| `src/theme/themes.ts` | 6 motivů jako úplné sady tokenů (`COLOR_KEYS`); `dark` = `styles.css` | `THEMES` `THEME_IDS` `COLOR_KEYS` |
| `src/theme/appearance.ts` | čistá logika: proměnné, normalizace, akcent, pořadí karet, tvar proužku, presety pružin, přechod rozšíření Nastavení | `appearanceVars` `normalizeAppearance` `accentPair` `cardOrder` `stripShape` `springPreset` `effectiveMotion` `widenMotion` |
| `src/theme/apply.ts` | DOM: proměnné na `<html>`, `data-theme`/`data-motion`, motion.ts; událost + `get_config` | `applyAppearance` `initAppearance` `savedAppearance` `scopeStyle` |
| `src/motion.ts` | styl/rychlost mění jen parametry pružin a délky; `FULL_CLIP` a rádius proužku sledují vzhled | `setMotion` `motionNow` `dur` `setShellRadius` `shellRadius` `setStripRadius` |
| `src/settings/panels/Appearance.tsx` · `AppearancePreview.tsx` · `appearance.css` | panel s náhledem (živě na celý šuplík, odchod bez uložení = zpět); JEDINÉ místo pro pozadí/akryl, šířku a animace; kotvy hledání `f-<klíč z labels.ts>` | `AppearancePanel` `Seg` `Slider` `lit` |
| `src/MainPage.tsx` | pořadí karet Práce z `savedAppearance().cards` | `card` |

## Toky

- **Start**: `App` → `initAppearance()` → `api.getConfig()` → `applyAppearance` (CSS proměnné inline na
  `<html>`). Do té doby platí `:root` ze `styles.css` (= motiv dark).
- **Náhled**: panel `createEffect(() => applyAppearance(rozpracovaný))`, `onCleanup` → `savedAppearance()`.
- **Uložení**: `p.save()` → `save_config` → `apply_config` → `emit("appearance")` → `commit` (uložený signál
  + aplikace) a `mark_dirty` → `refresh_strip` (styl, paleta, linka, osa).
- **Osa u hrany**: `anchor_c2` = dvojnásobek x osy podle pozice a šířky velkého šuplíku; proužek, peek
  i šuplík jsou na ní vystředěné (`span_x`) → morf (`collapsedClip` je středový) zůstává souměrný.

## Invarianty a pasti

- Výchozí hodnoty = dnešní vzhled: `center_matches_old_formula`, `metrics_default_is_today`,
  `line_stops_resolution`, test „motiv dark = tokeny ze styles.css“. Měníš `:root`? Změň i `THEMES.dark`.
- Odvozené tokeny (`--tint*`, `--r-*`, `--s*`, `--fs-*`, `--dur-*`) jsou v bloku `:root, .theme-scope` —
  var() se vyhodnocuje tam, kde je deklarace; bez `.theme-scope` by náhledy neměnily písmo ani krytí.
- Nová barva v CSS = token v `COLOR_KEYS` + hodnota ve VŠECH motivech (jinak prosákne jiný motiv).
- Mechanika morfu se nemění — jen parametry (`springPreset`, `dur`). `none` = 0 ms; systémové
  „omezit pohyb“ = `none` (ve frontendu; Rust o něm neví → akrylové vysunutí okna ho nerespektuje).
  `drawer.animationMs` už NENÍ: Rust bere `motion::drawer_anim_ms` (`none` = bez morfu i vysunutí).
  Starý klíč se jen čte (`0` → `none` vždy; jiná délka → rychlost, pokud `motion.speed` v souboru chybí). Zavírání při `speed 0.5` trvá ~560 ms (pojistka v `Drawer::hide` je 600 ms).
- `drawer.width` se nevaliduje (starý config se nesmí přestat načítat), jen ořízne (`drawer_width`).
- Rozmazání = `drawer.glass: acrylic` (DWM akryl, šuplík vyjíždí místo morfu); `backdrop-filter` v
  šuplíku nepoužívej (software rendering WebView2).
- Akcenty motivů jsou v Rustu (`palette().accent`) i v TS — hlídá `theme_accents_mirror_frontend`.
- Puls proužku („Claude čeká / hotovo“, `attention.rs`) respektuje `appearance.motion.style: none` a
  systémově vypnuté animace (`SPI_GETCLIENTAREAANIMATION`) — pak jen statické bliknutí linky (`Core.flash`).
  Jiná animace proužku není; časovač běží jen po dobu pulsu.

## Testy

`cargo test appearance` (validace, osa na DPI a záporných monitorech, ořez šířky, kontrast textu proužku,
linka, zrcadla TS, `motion::tests` — délka animace a převod `drawer.animationMs`, tabulka). `npm run test:ui` = `node --test src/theme/theme.test.mjs` (kontrast textů ≥ 4.5:1
ve všech motivech, dark = CSS, normalizace, pořadí karet, presety, `widenMotion`).

## Co nečíst

`strip.rs` celý (jen `draw`), `drawer.rs` mimo `place`/`set_anchor`, `themes.ts` (jen hodnoty).
