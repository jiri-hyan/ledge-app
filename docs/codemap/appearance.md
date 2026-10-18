# Codemapa: vzhled a animace (`appearance`)

Design styly (vlna 6), motiv, akcent, hustota, písmo, materiál, rohy, krytí pozadí, rozložení a šířka,
styl/rychlost animací, vzhled a pozice proužku. Config `appearance` (AGENTS.md →
Konfigurace). Výchozí = vzhled před zavedením sekce (styl `classic`). Pořadí a velikost karet jsou v [layout.md](layout.md). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/appearance.rs` | struktura configu, výchozí hodnoty, validace (jen nová pole), testy (i starý config z vlny 5) | `AppearanceConfig` `Theme` (`Theme::ALL`) `StripLook` `StripStyle` `StripPosition` `MotionLook` `validate` `DRAWER_WIDTH` `is_hex` |
| `src-tauri/src/appearance/look.rs` | složky stylu bez barev: písmo, materiál, rozložení, předvolba šířky (px); tvar jména stylu | `FontLook` `Material` `Layout` `WidthPreset` (`px`) `style_id_ok` `CLASSIC` |
| `src-tauri/src/appearance/stripdeco.rs` | ozdoby proužku podle `strip.style`: okraj, okraj v akcentu, tvrdý / měkký stín, záře linky, písmo, podíl a výška linky; přesah kolem těla | `strip_deco` `StripDeco` (`extents`) `StripFont` |
| `src-tauri/src/appearance/motion.rs` | pohyb okna šuplíku z `motion` (none nebo systémové „omezit pohyb“ = 0 ms, jinak 170 / rychlost); převod starého `drawer.animationMs` při `config::load` | `drawer_anim_ms` `anim_ms_now` `legacy_animation_ms` `upgrade_loaded` `BASE_SLIDE_MS` |
| `src-tauri/src/appearance/geom.rs` | čistá geometrie a barvy pro Rust: osa Ledge u hrany, šířka (předvolba stylu / `drawer.width`, ořez), tvar a paleta proužku (11 motivů), linka, kontrast a čitelná barva položky (`strip.colors`) | `anchor_c2` `span_x` `axis2_for` `drawer_width_for` `preset_width` `drawer_width` `strip_metrics` `palette` `ink` `readable` `contrast` `line_stops` `accent_pair` |
| `strip.rs` (`draw`) | kreslí podle `StripModel.{style, metrics, palette, line, axis2, pulse}` + `stripdeco` (styl `dot` = jen tečky; stíny/záře v přesahu okna kolem těla, `last` = tělo; puls = linka v rytmu `attention::pulse_level` roztažená k plné šířce) | `StripModel` `pulse_now` |
| `stripview.rs` (`refresh_strip`) | naplní vzhled modelu z configu | |
| `drawer.rs` | `set_anchor` + `place` (x z `anchor_c2`/`span_x`); šířku dostává oříznutou | `Drawer::set_anchor` |
| `threads/ui.rs` | před každým `UiCmd`: `set_anchor`, šířka = `geom::drawer_width_for` (předvolba stylu; jinak ≤ 95 % monitoru), délka animace = `motion::anim_ms_now` | |
| `threads/hover.rs` | zóna bez proužku leží na ose (`axis2_for`), jinak podle skutečného obdélníku proužku | |
| `settings.rs` (`apply_config`) | změna `appearance` → událost `appearance` do šuplíku; proužek přes `mark_dirty` | |
| `src/theme/looks.ts` | **tabulka stylů** a jejich složek jako čistá data: co styl nastaví (`StyleParts`), písma (jen systémová), materiály, rozložení (sloupce), šířky; `lookVars` = CSS proměnné `--m-*` `--h-*` `--deck-*` `--span-*` `--l-pad` `--sans` `--display` (vždy úplná sada) | `STYLES` `STYLE_IDS` `styleOf` `FONTS` `MATERIALS` `LAYOUTS` `WIDTHS` `spans` `lookVars` `CLASSIC` |
| `src/theme/styles.ts` | výběr stylu: hodnoty stylu do vzhledu / celého configu, které složky se od stylu liší | `withStyle` `configWithStyle` `styleChanges` `PARTS` |
| `src/theme/openAnim.ts` · `springs.ts` | plán otevření / zavření podle stylu animace (clip-path morf, „z výřezu“ přes `band`, odskok, rozplynutí; zavření ≤ `CLOSE_CAP_MS`) · čistá simulace pružiny | `openPlan` `closePlan` `Geo` `Plan` `CLOSE_CAP_MS` · `simulateSpring` |
| `src/theme/themes.ts` | 11 motivů jako úplné sady tokenů (`COLOR_KEYS`); `dark` = `styles.css`; `terminal`/`paper`/`neon`/`brutal`/`pastel` = barvy stylů | `THEMES` `THEME_IDS` `COLOR_KEYS` |
| `src/theme/appearance.ts` | čistá logika: proměnné (+ `lookVars`), normalizace, akcent, tvar proužku (8 stylů), presety pružin, přechod rozšíření Nastavení | `appearanceVars` `normalizeAppearance` `accentPair` `stripShape` `springPreset` `effectiveMotion` `widenMotion` `MOTION_STYLES` `STRIP_STYLES` |
| `src/theme/apply.ts` · `label.ts` | DOM: proměnné na `<html>` (staré smaže), `data-theme`/`data-motion`/`data-scheme`/`data-style`/`data-layout`/`data-material`/`data-font`, motion.ts; událost + `get_config` · překlad páru `[klíč, cs]` z tabulek | `applyAppearance` `initAppearance` `savedAppearance` `scopeStyle` · `lbl` |
| `src/motion.ts` | styl/rychlost mění jen parametry pružin a délky; `FULL_CLIP` a rádius proužku sledují vzhled; `bandClip` = mezikrok „z výřezu“ | `setMotion` `motionNow` `dur` `bandClip` `setShellRadius` `shellRadius` `setStripRadius` |
| `src/App.tsx` (`prepare`/`play`/`closing`) | přehraje `openPlan`/`closePlan` na `.shell`; handshake `prepared`/`closed` beze změny; přechod z peek = vždy morf tvaru | `geo` `settle` |
| `src/settings/panels/Appearance.tsx` · `AppearanceStyles.tsx` · `AppearanceLayout.tsx` · `AppearanceMotion.tsx` · `AppearancePreview.tsx` · `apUi.tsx` · `appearance.css` · `en-look.ts` | panel: galerie stylů (živé miniatury `.theme-scope`, „Upraveno: …“, Obnovit styl) → náhled → barvy → šířka a rozložení → animace (7 ukázek, hraje jen po kliku) → proužek → Upravit detaily (`<details>`) → jazyk; vše živě na celý šuplík, ukládá společná lišta; JEDINÉ místo pro pozadí/akryl, šířku a animace; kotvy hledání `f-<klíč z labels.ts>` | `AppearancePanel` `AppearanceStyles` `LayoutCard` `DetailsCard` `AppearanceMotion` `Ap` `Seg` `Slider` `lit` |
| `src/palette/styles.ts` | paleta Ctrl+K: „Styl: …“ → `configWithStyle` → `save_config` | `styleItems` |

## Nový design styl (jeden záznam + volitelný CSS blok)

1. Záznam do `STYLES` v `src/theme/looks.ts` (`id` = malá písmena, `name`/`desc` jako páry
   `["appearance.style.<id>", "…"]`, `parts` = motiv, hustota, rohy, krytí, tvar proužku, písmo, materiál,
   rozložení, šířka, rozmazání). Rust nic nepotřebuje (styl zná jen jménem).
2. Angličtina: `appearance.style.<id>` + `.desc` v `src/settings/panels/en-look.ts`.
3. Volitelně: vlastní barvy = nový motiv (`themes.ts` + `Theme` a `palette()` v Rustu, testy kontrastu a
   zrcadla to hlídají); ozdoba, která nejde proměnnou = blok `html[data-style="<id>"] …` na konci `styles.css`
   (jen statické — žádná animace v klidu); nový materiál / písmo / rozložení = záznam v `MATERIALS` / `FONTS` /
   `LAYOUTS` + varianta enumu v `appearance/look.rs` (test zrcadel).

Rozhraní k mřížce karet (`.deck` > `.deck-item[data-size]`, mechanika v `layout/layout.css` → [layout.md](layout.md)):
styl dává `--deck-cols` (Klasický ho nevydá = 4), `--deck-gap` a `--deck-pad` (místo na stíny krajních karet;
čte je `html[data-layout] .deck` ve `styles.css`, `deck` je v SHARED hlídače); `--span-*` jsou k dispozici,
`.deck` si rozpětí ořízne sama přes `min(n, var(--deck-cols))`.

## Toky

- **Start**: `App` → `initAppearance()` → `api.getConfig()` → `applyAppearance` (CSS proměnné inline na
  `<html>`). Do té doby platí `:root` ze `styles.css` (= motiv dark).
- **Náhled**: panel `createEffect(() => applyAppearance(rozpracovaný))`, `onCleanup` → `savedAppearance()`.
  Klik na styl = `withStyle` do rozpracovaného configu (+ `drawer.glass` jen při změně rozmazání).
- **Otevření**: `prepare` vezme `openPlan(styl, rychlost, geo)` → nastaví `plan.start` na `.shell`, `prepared`;
  `play` přehraje snímky; `closing` přehraje `closePlan` a pošle `closed`. Šířka okna: Rust
  `drawer_width_for` (předvolba stylu) — panel ji v náhledu nemění (projeví se po uložení).
- **Uložení**: společná lišta Nastavení (Uložit; panel vlastní tlačítko nemá) → `save_config` → `apply_config` → `emit("appearance")` → `commit` (uložený signál
  + aplikace) a `mark_dirty` → `refresh_strip` (styl, paleta, linka, osa).
- **Osa u hrany**: `anchor_c2` = dvojnásobek x osy podle pozice a šířky velkého šuplíku; proužek, peek
  i šuplík jsou na ní vystředěné (`span_x`) → morf (`collapsedClip` je středový) zůstává souměrný.

## Invarianty a pasti

- Výchozí hodnoty = dnešní vzhled: `center_matches_old_formula`, `metrics_default_is_today`,
  `line_stops_resolution`, test „motiv dark = tokeny ze styles.css“. Měníš `:root`? Změň i `THEMES.dark`.
- Odvozené tokeny (`--tint*`, `--r-*`, `--s*`, `--fs-*`, `--dur-*`) jsou v bloku `:root, .theme-scope` —
  var() se vyhodnocuje tam, kde je deklarace; bez `.theme-scope` by náhledy neměnily písmo ani krytí.
- Nová barva v CSS = token v `COLOR_KEYS` + hodnota ve VŠECH motivech (jinak prosákne jiný motiv).
- Mechanika morfu (handshake) se nemění — styl mění jen to, co se animuje (`openAnim.ts`). `none` = 0 ms;
  systémové „omezit pohyb“ = `none` ve frontendu (`prefers-reduced-motion`) i v Rustu (`motion::anim_ms_now`
  → `SPI_GETCLIENTAREAANIMATION`: bez morfu i bez akrylového vysunutí okna).
- Proměnné stylu (`lookVars`) se vydávají VŽDY všechny — miniatura `.theme-scope` jiného stylu by jinak
  zdědila hodnoty kořene. Výchozí hodnoty `var(--x, …)` ve `styles.css` = Klasický styl (test).
- Plovoucí panel (`layout: float`) jen bez akrylu: DWM rozmaže celé okno, odstup by prosvítal.
  `drawer.animationMs` už NENÍ: Rust bere `motion::drawer_anim_ms` (`none` = bez morfu i vysunutí).
  Starý klíč se jen čte (`0` → `none` vždy; jiná délka → rychlost, pokud `motion.speed` v souboru chybí). Zavírání při `speed 0.5` trvá ~560 ms (pojistka v `Drawer::hide` je 600 ms).
- `drawer.width` se nevaliduje (starý config se nesmí přestat načítat), jen ořízne (`drawer_width`).
- Rozmazání = `drawer.glass: acrylic` (DWM akryl, šuplík vyjíždí místo morfu); `backdrop-filter` v
  šuplíku nepoužívej (software rendering WebView2).
- Akcenty motivů jsou v Rustu (`palette().accent`) i v TS — hlídá `theme_accents_mirror_frontend`.
- Barva na položku proužku (`strip.colors`, `stripconf.rs`) jsou jen předvolby; text prochází `geom::readable`
  (kontrast ≥ 4.5:1 v každém motivu, test `stripconf::every_preset_readable_in_every_theme`). Konec časovače
  pulzuje modře (`timer::COLOR`) přes `attention::signal` — stejná pravidla jako upozornění Clauda.
- Puls proužku („Claude čeká / hotovo“, `attention.rs`) respektuje `appearance.motion.style: none` a
  systémově vypnuté animace (`SPI_GETCLIENTAREAANIMATION`) — pak jen statické bliknutí linky (`Core.flash`).
  Jiná animace proužku není; časovač běží jen po dobu pulsu.

## Testy

`cargo test appearance` (validace, starý config z vlny 5, osa na DPI a záporných monitorech, ořez a předvolby
šířky, kontrast textu proužku ve všech 11 motivech, linka, ozdoby proužku, zrcadla TS, `motion::tests` — délka
animace i se systémovým „omezit pohyb“ a převod `drawer.animationMs`, tabulka). `npm run test:ui` =
`node --test src/theme/theme.test.mjs` (kontrast textů ≥ 4.5:1 ve všech motivech, dark = CSS, výchozí hodnoty
proměnných = Klasický, starý config, tabulka stylů, výběr stylu, zrcadla enumů a šířek s Rustem, plány animací,
normalizace, pořadí karet, presety, `widenMotion`).

## Co nečíst

`strip.rs` celý (jen `draw`), `drawer.rs` mimo `place`/`set_anchor`, `themes.ts` (jen hodnoty).
