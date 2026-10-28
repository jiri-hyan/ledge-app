# Codemapa: vzhled a animace (`appearance`)

Design styly (vlna 6, 10 stylů), motiv (11 + 3 barvy stylů + vlastní motivy), vzhled jednotlivých záložek, akcent, hustota, písmo, materiál, rohy, krytí pozadí, rozložení a šířka, styl/rychlost animací (9), vzhled a pozice proužku. Config `appearance` ([config-behavior.md](../agents/config-behavior.md) → Vzhled) …

Detail (toky, invarianty, testy, co nečíst, historie): [appearance-detail.md](appearance-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/appearance.rs` | struktura configu, výchozí hodnoty, validace (jen nová pole), testy (i starý … | `AppearanceConfig` `Theme` |
| `src-tauri/src/appearance/pages.rs` | vzhled záložek (`appearance.pages` = … | `PageLook` `CustomTheme` |
| `src-tauri/src/appearance/look.rs` | složky stylu bez barev: písmo, materiál, rozložení (+ `Notch` s pevnou šířkou … | `FontLook` `Material` `Layout` |
| `src-tauri/src/appearance/size.rs` | kolo 23, velikost otevřeného šuplíku (`appearance.size` = … | `SizeConfig` `PageSize` |
| `src-tauri/src/appearance/stripdeco.rs` | ozdoby proužku podle `strip.style`: okraj, okraj v akcentu, tvrdý / měkký stín … | `strip_deco` `StripDeco` |
| `src-tauri/src/appearance/motion.rs` | pohyb okna šuplíku z `motion` (none nebo systémové „omezit pohyb“ = 0 ms, jinak … | `drawer_anim_ms` `anim_ms_now` |
| `src-tauri/src/appearance/geom.rs` | čistá geometrie a barvy pro Rust: osa Ledge u hrany, šířka (předvolba stylu / … | `anchor_c2` `span_x` |
| `strip.rs` (`draw`) | kreslí podle `StripModel.{style, metrics, palette, line, axis2, pulse}` + … | `StripModel` `pulse_now` |
| `stripview.rs` (`refresh_strip`) | naplní vzhled modelu z configu; u `strip.look` ≠ `classic` i … | `look_facts` |
| `src-tauri/src/looks/mod.rs` | styly zavřeného Ledge (`strip.look`): `Look` (id = kontrakt s … | `Look` `Facts` `Show` `Part` |
| `src-tauri/src/looks/facts.rs` · `frame.rs` | převody stavu jádra (časovač, právě hraje, limit 5h a týden, baterie, počasí z … | `timer_fact` `music_fact` |
| `src-tauri/src/looks/capsule.rs` | kapsle kola 22 (čisté `fill` pro `minimum` = baterie + čas, `trio` = počasí \| … | `fill` `ring_color` `day_of` |
| `src-tauri/src/looks/compose.rs` · `compose_tests.rs` | vlastní zavřený Ledge (`strip.look = custom`, `strip.compose`, kolo 23) … | `Compose` `Form` `PieceItem` |
| `src-tauri/src/looks/activity_tests.rs` | položka skládačky `activity` („Claude aktivita“, kolo 23) … | `PieceItem::Activity` |
| `stripdraw.rs` | kreslí `Show` stylu (tělo, oušky výřezu přes path geometrii, tečky, ikony s … | `draw` `pick_fonts` `Fonts` |
| `src/strip/looks.ts` · `strip/en.ts` | jediný seznam stylů pro Nastavení (`STRIP_LOOKS` = `{ id, key, cs, en, desc }` … | `STRIP_LOOKS` `STRIP_LOOK_IDS` |
| `src/strip/compose.ts` · `compose.test.mjs` | vlastní vzhled pro Nastavení: `COMPOSE_SHAPES` / `COMPOSE_ITEMS` / … | `composeOf` `normCompose` |
| `drawer.rs` | `set_anchor` + `place` (x z `anchor_c2`/`span_x`); šířku dostává oříznutou | `Drawer::set_anchor` |
| `drawer/resize.rs` | kolo 23, `Drawer::resize`: otevřený Full na velikost z `appearance.size` … | `Drawer::resize` |
| `threads/ui.rs` | před každým `UiCmd`: `set_anchor`, šířka = `geom::drawer_width_for` (vlastní … |  |
| `threads/hover.rs` | zóna = `looks::frame::zone`: podle skutečného obdélníku proužku (linka / lišta … |  |
| `src-tauri/src/threads/pointer.rs` | pojistky hoveru (kolo 22): `cursor_visible` (skrytý = `flags == 0` … | `cursor_visible` |
| `settings.rs` (`apply_config`) | změna `appearance` → událost `appearance` do šuplíku; proužek přes `mark_dirty` |  |
| `src/theme/looks.ts` | tabulka stylů a jejich složek jako čistá data … | `STYLES` `STYLE_IDS` `styleOf` |
| `src/notch/NotchBar.tsx` · `model.ts` · `notch.css` · `en.ts` | široký notch (rozložení `notch`, kolo 22): lišta sekcí nad kartami záložky … | `NotchBar` `notchSections` |
| `src/theme/styles.ts` | výběr stylu: hodnoty stylu do vzhledu / celého configu, které složky se od … | `withStyle` `configWithStyle` |
| `src/theme/openAnim.ts` · `springs.ts` | plán otevření / zavření podle stylu animace (clip-path morf, „z výřezu“ přes … | `openPlan` `closePlan` `Geo` |
| `src/theme/themes.ts` | 16 motivů jako úplné sady tokenů (`COLOR_KEYS`); `dark` = `styles.css` … | `THEMES` `THEME_IDS` |
| `src/theme/contrast.ts` | kontrast textu (WCAG) — jedna implementace pro test motivů i živou kontrolu v … | `textProblems` `contrast` |
| `src/theme/customThemes.ts` | vlastní motivy (čistě): `resolveCustom` složí úplné tokeny (vrstvy z `base`, u … | `CUSTOM_KEYS` `resolveCustom` |
| `src/theme/pages.ts` | vzhled záložek (čistě): `effectiveAppearance(a, pageId)` = celek + přepis … | `effectiveAppearance` |
| `src/theme/size.ts` | kolo 23, čistá logika velikosti šuplíku (zrcadlo `appearance/size.rs`) … | `normalizeSize` `sizeFor` |
| `src/theme/appearance.ts` | čistá logika: proměnné (+ `lookVars`), normalizace (vč … | `appearanceVars` |
| `src/theme/apply.ts` · `label.ts` | DOM: proměnné na `<html>` (staré smaže) … | `applyAppearance` |
| `src/motion.ts` | styl/rychlost mění jen parametry pružin a délky … | `setMotion` `motionNow` `dur` |
| `src/App.tsx` (`prepare`/`play`/`closing`) | přehraje `openPlan`/`closePlan` na `.shell` … | `geo` `settle` |
| `src/settings/panels/Appearance.tsx` | jeden krok Vzhledu jako otázka Studia (`AppearanceStep step=closed|hover|open|motion`: Proužek / Otevírání / Vzhled šuplíku); rozbalené Doladit drží modul, skok (`hit`) ho rozbalí | `AppearanceStep` `AppearancePanel` |
| `panels/AppearanceClosed.tsx` · `StripSettings.tsx` | krok Zavřený: galerie 15 vzhledů `strip.look` + dlaždice „Vlastní“ … | `AppearanceClosed` `normLook` |
| `panels/ComposeEditor.tsx` · `compose.css` · `previews/ComposePreview.tsx` | krok Zavřený, kolo 23: dlaždice „Vlastní“ pod galerií (`AppearanceClosed` … | `ComposeEditor` |
| `panels/AppearanceHover.tsx` | krok Po najetí: 3 dlaždice `drawer.hover` (data `src/hover/modes.ts` od J) s … | `AppearanceHover` `normHover` |
| `panels/AppearanceOpen.tsx` · `AppearanceStyles.tsx` · `AppearanceLayout.tsx` | krok Otevřený: galerie stylů (živé miniatury, najetí = dočasný náhled přes … | `AppearanceOpen` |
| `panels/AppearanceOpen.tsx` `SizeTable` | kolo 23, krok Otevřený (pod Šířkou a hustotou) → „Velikost otevřeného Ledge“ … | `SizeTable` |
| `panels/AppearanceThemes.tsx` | Barvy: galerie vestavěných i vlastních motivů (vzorníky, najetí = náhled) … | `AppearanceThemes` |
| `panels/AppearancePages.tsx` | Vzhled záložek: řádek na záložku z `layout.pages` (`pageName`) … | `AppearancePages` |
| `panels/AppearanceMotion.tsx` · `apUi.tsx` · `appearance.css` · `en-look.ts` | krok Pohyb (9 ukázek, hraje jen po kliku, rychlost … | `AppearanceMotion` `Ap` `Seg` |
| `src/settings/previews/{HoverPreview,MonitorPreview,StripLookPreview}.tsx` (`StripLookPreview`, `HoverPreview`, `MonitorPreview`, `previews.css`, `en.ts`) | HTML náhledy (prefix `apv-`): zavřený Ledge na „horní hraně monitoru“ a po … | `StripLookPreview` |
| `src/layout/CardLook.tsx` (`CardLookPanel`, líný chunk; stav v `lookState.ts`) | jediný panel vzhledu karty (⋯ → Vzhled karty…) … | `CardLookPanel` `openCardLook` |
| `src/palette/styles.ts` | paleta Ctrl+K: „Styl: …“ → `configWithStyle` → `save_config` | `styleItems` |

Další soubory (podrobnosti v detailu):
`src/pin/en.ts` · `src/projects/en.ts`
