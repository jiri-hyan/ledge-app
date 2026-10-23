# Codemapa: vzhled a animace (`appearance`)

Design styly (vlna 6, 10 stylů), motiv (11 + 3 barvy stylů + vlastní motivy), vzhled jednotlivých záložek, akcent, hustota,
písmo, materiál, rohy, krytí pozadí, rozložení a šířka, styl/rychlost animací (9), vzhled a pozice proužku. Config `appearance` ([config-behavior.md](../agents/config-behavior.md) →
Vzhled). Výchozí = vzhled před zavedením sekce (styl `classic`). Pořadí a velikost karet jsou v [layout.md](layout.md). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/appearance.rs` | struktura configu, výchozí hodnoty, validace (jen nová pole), testy (i starý config z vlny 5) | `AppearanceConfig` `Theme` (`Theme::ALL`, 14) `MotionStyle` `StripLook` `StripStyle` `StripPosition` `MotionLook` `validate` `DRAWER_WIDTH` `is_hex` |
| `src-tauri/src/appearance/pages.rs` | **vzhled záložek** (`appearance.pages` = `{ id záložky: { style?, theme?, accent? } }`) a **vlastní motivy** (`customThemes` ≤ 20: `id`, `name`, `base`, 11 barev `EDITABLE`; `customTheme` = vybraný pro celek): validace (hex, unikátní id ≠ vestavěný motiv, kontrast textu ≥ 4.5:1 na `bg` i `raised`, odkazy na existující motiv), `light` z pozadí, `palette_for` = paleta nativního proužku z vlastního motivu | `PageLook` `CustomTheme` `EDITABLE` `TEXT_KEYS` `validate` `theme_problems` `palette_for` `active_custom` |
| `src-tauri/src/appearance/look.rs` | složky stylu bez barev: písmo, materiál, rozložení, předvolba šířky (px); tvar jména stylu | `FontLook` `Material` `Layout` `WidthPreset` (`px`) `style_id_ok` `CLASSIC` |
| `src-tauri/src/appearance/stripdeco.rs` | ozdoby proužku podle `strip.style`: okraj, okraj v akcentu, tvrdý / měkký stín, záře linky, písmo, podíl a výška linky; přesah kolem těla | `strip_deco` `StripDeco` (`extents`) `StripFont` |
| `src-tauri/src/appearance/motion.rs` | pohyb okna šuplíku z `motion` (none nebo systémové „omezit pohyb“ = 0 ms, jinak 170 / rychlost); převod starého `drawer.animationMs` při `config::load` | `drawer_anim_ms` `anim_ms_now` `legacy_animation_ms` `upgrade_loaded` `BASE_SLIDE_MS` |
| `src-tauri/src/appearance/geom.rs` | čistá geometrie a barvy pro Rust: osa Ledge u hrany, šířka (předvolba stylu / `drawer.width`, ořez), tvar a paleta proužku (14 motivů), linka, kontrast a čitelná barva položky (`strip.colors`) | `anchor_c2` `span_x` `axis2_for` `drawer_width_for` `preset_width` `drawer_width` `strip_metrics` `palette` `ink` `readable` `contrast` `line_stops` `accent_pair` |
| `strip.rs` (`draw`) | kreslí podle `StripModel.{style, metrics, palette, line, axis2, pulse}` + `stripdeco` (styl `dot` = jen tečky; stíny/záře v přesahu okna kolem těla, `last` = tělo; puls = linka v rytmu `attention::pulse_level` roztažená k plné šířce) | `StripModel` `pulse_now` |
| `stripview.rs` (`refresh_strip`) | naplní vzhled modelu z configu; u `strip.look` ≠ `classic` i `StripModel.{look, show, animate, position}` (`look_facts` → `looks::show`) | `look_facts` |
| `src-tauri/src/looks/mod.rs` | **styly zavřeného Ledge** (`strip.look`): `Look` (id = kontrakt s `src/strip/looks.ts`), stav (`events`, priorita čeká > hotovo > časovač > pracuje > hudba > klid), „stav → co styl ukáže“ (`show` → `Show { parts, right, progress, line, rotate, hidden, idle }`), Pokročilé `without` (`strip.lookHide`), ikony `Glyph` (kódy Segoe Fluent Icons / MDL2 + náhrady) | `Look` `Facts` `Show` `Part` `Glyph` `show` `events` `state_of` `without` `DONE_MS` `ROTATE_MS` |
| `src-tauri/src/looks/facts.rs` · `frame.rs` | převody stavu jádra (časovač, právě hraje, limit 5h — jen čerstvá data) · rozměry stylu (`spec`), šířka těla, okno a tělo v něm (`place`: celá šířka, roh podle pozice, výřez s oušky, jinak na ose), hover zóna (`zone`), tvar morfu (`morph_body`), šířka ostrůvku (`WidthAnim`, 240 ms jen při změně obsahu) | `timer_fact` `music_fact` `limit_fact` · `Spec` `spec` `body_width` `place` `Placed` `zone` `morph_body` `WidthAnim` |
| `stripdraw.rs` | kreslí `Show` stylu (tělo, oušky výřezu přes path geometrii, tečky, ikony s odznaky, lišta průběhu, úseky linky, puls = obrys v barvě události, nažhavení = linka akcentu); písmo ikon / mono / display zjistí jednou `pick_fonts` | `draw` `pick_fonts` `Fonts` |
| `src/strip/looks.ts` · `strip/en.ts` | jediný seznam stylů pro Nastavení (`STRIP_LOOKS` = `{ id, key, cs, en, desc }`, `lookOf`), Pokročilé `LOOK_HIDEABLE` + `withLookShown`; angličtina `appearance.look.*` | `STRIP_LOOKS` `STRIP_LOOK_IDS` `StripLookId` `lookOf` `LOOK_HIDEABLE` `withLookShown` |
| `drawer.rs` | `set_anchor` + `place` (x z `anchor_c2`/`span_x`); šířku dostává oříznutou | `Drawer::set_anchor` |
| `threads/ui.rs` | před každým `UiCmd`: `set_anchor`, šířka = `geom::drawer_width_for` (předvolba stylu; jinak ≤ 95 % monitoru), délka animace = `motion::anim_ms_now` | |
| `threads/hover.rs` | zóna = `looks::frame::zone`: podle skutečného obdélníku proužku (linka / lišta celá šířka, roh, …), oříznutá na monitor, aspoň 4 px; bez proužku (neviditelný styl v klidu) pruh na ose (`axis2_for`) | |
| `settings.rs` (`apply_config`) | změna `appearance` → událost `appearance` do šuplíku; proužek přes `mark_dirty` | |
| `src/theme/looks.ts` | **tabulka stylů** a jejich složek jako čistá data: co styl nastaví (`StyleParts`), písma (jen systémová), materiály, rozložení (sloupce), šířky; `lookVars` = CSS proměnné `--m-*` `--h-*` `--deck-*` `--span-*` `--l-pad` `--sans` `--display` (vždy úplná sada) | `STYLES` `STYLE_IDS` `styleOf` `FONTS` `MATERIALS` `LAYOUTS` `WIDTHS` `spans` `lookVars` `CLASSIC` |
| `src/theme/styles.ts` | výběr stylu: hodnoty stylu do vzhledu / celého configu, které složky se od stylu liší | `withStyle` `configWithStyle` `styleChanges` `PARTS` |
| `src/theme/openAnim.ts` · `springs.ts` | plán otevření / zavření podle stylu animace (clip-path morf, „z výřezu“ přes `band`, odskok, **pád** = jeden odraz, **rozvinutí** = tvar bez překmitu + `Plan.reveal` karty po 25 ms (`motion.ts` `revealContent`), rozplynutí; zavření ≤ `CLOSE_CAP_MS`) · čistá simulace pružiny | `openPlan` `closePlan` `Geo` `Plan` `CLOSE_CAP_MS` · `simulateSpring` |
| `src/theme/themes.ts` | 16 motivů jako úplné sady tokenů (`COLOR_KEYS`); `dark` = `styles.css`; `terminal`/`paper`/`neon`/`brutal`/`pastel`/`aurora`/`mono`/`retro`/`oled`/`warm` = barvy stylů (poslední dva z kola 18, stejně jako styl Fluent, materiály `acrylic`/`tinted`, rozložení `panel` a animace `liquid`) | `THEMES` `THEME_IDS` `COLOR_KEYS` |
| `src/theme/contrast.ts` | kontrast textu (WCAG) — **jedna implementace** pro test motivů i živou kontrolu v editoru vlastního motivu: `parseColor`, `contrast`, `surfaces`, `textProblems` | `textProblems` `contrast` `parseColor` `MIN_CONTRAST` |
| `src/theme/customThemes.ts` | vlastní motivy (čistě): `resolveCustom` složí úplné tokeny (vrstvy z `base`, u opačného tónu z `dark`/`light`, odvozené barvy dotažené na ≥ 4.5:1), `customProblems` (živá kontrola, blokuje uložení), normalizace, `addCustomTheme`/`updateCustomTheme`, export / import JSON (`ledgeTheme: 1`), `themeOf` = tokeny + tón pro `Appearance` | `CUSTOM_KEYS` `resolveCustom` `customProblems` `themeOf` `normalizeCustomThemes` `importTheme` `exportTheme` |
| `src/theme/pages.ts` | vzhled záložek (čistě): `effectiveAppearance(a, pageId)` = celek + přepis záložky (styl → jeho složky bez proužku a šířky; motiv vestavěný / vlastní; akcent), `setPageLook`, `dropThemeRefs`, `normalizePages` | `effectiveAppearance` `setPageLook` `dropThemeRefs` `normalizePages` |
| `src/theme/appearance.ts` | čistá logika: proměnné (+ `lookVars`), normalizace (vč. `pages`, `customThemes`), akcent, tvar proužku (8 stylů), presety pružin, přechod rozšíření Nastavení | `appearanceVars` `normalizeAppearance` `accentPair` `stripShape` `springPreset` `effectiveMotion` `widenMotion` `MOTION_STYLES` `STRIP_STYLES` |
| `src/theme/apply.ts` · `label.ts` | DOM: proměnné na `<html>` (staré smaže), `data-theme`/`data-motion`/`data-scheme`/`data-style`/`data-layout`/`data-material`/`data-font`, motion.ts; událost + `get_config`; **aktivní záložka** (`setActivePage` volá `MainPage`; přepnutí na záložku s vlastním vzhledem = `applyAppearance` s `effectiveAppearance` + `html[data-look-shift]` ~450 ms, jen barvy) · překlad páru `[klíč, cs]` z tabulek | `applyAppearance` `initAppearance` `savedAppearance` `scopeStyle` `setActivePage` `activePageId` · `lbl` |
| `src/motion.ts` | styl/rychlost mění jen parametry pružin a délky; `FULL_CLIP` a rádius proužku sledují vzhled; `bandClip` = mezikrok „z výřezu“ | `setMotion` `motionNow` `dur` `bandClip` `setShellRadius` `shellRadius` `setStripRadius` |
| `src/App.tsx` (`prepare`/`play`/`closing`) | přehraje `openPlan`/`closePlan` na `.shell`; handshake `prepared`/`closed` beze změny; přechod z peek = vždy morf tvaru | `geo` `settle` |
| `src/settings/panels/Appearance.tsx` | panel ve 4 krocích (záložky `.ap-step`): **Zavřený · Po najetí · Otevřený · Pohyb** + jazyk pod nimi; krok a rozbalené Pokročilé drží modul (přežijí přepnutí panelu), skok z hledání (`hit`) přepne krok a rozbalí Pokročilé (`settings/appearanceModel.ts`); živý náhled `applyAppearance(draft)` na celý šuplík | `AppearancePanel` `lastStep` `advOpen` |
| `panels/AppearanceClosed.tsx` · `StripSettings.tsx` | krok Zavřený: galerie 12 vzhledů `strip.look` (data `src/strip/looks.ts` od I) s HTML náhledy a přepínačem „Co se děje“, pod ní pozice a velikost proužku; Pokročilé = tvar klasického proužku (galerie 8 tvarů), linka, krytí a „Co proužek ukazuje“ (`StripSettings.tsx`). Klik = draft + „Vrátit“ (`offerUndo`); proužek se překreslí po Uložit | `AppearanceClosed` `normLook` `LOOK_DESC` |
| `panels/AppearanceHover.tsx` | krok Po najetí: 3 dlaždice `drawer.hover` (data `src/hover/modes.ts` od J) s náhledem | `AppearanceHover` `normHover` |
| `panels/AppearanceOpen.tsx` · `AppearanceStyles.tsx` · `AppearanceLayout.tsx` | krok Otevřený: galerie stylů (živé miniatury, **najetí = dočasný náhled** přes `apUi` `peekOn`, „Upraveno: …“, Obnovit styl) → **Barvy** → **Vzhled záložek** → monitor ve skutečném poměru + Šířka (4 předvolby) + Hustota; Pokročilé = rozložení karet, vlastní šířka px, písmo, materiál, velikost písma, rohy, krytí a rozmazání | `AppearanceOpen` `AppearanceStyles` `LayoutPicker` `CustomWidth` `DetailsRows` |
| `panels/AppearanceThemes.tsx` | **Barvy**: galerie vestavěných i vlastních motivů (vzorníky, najetí = náhled), akcent; vlastní motivy: „Uložit aktuální barvy…“, editor 11 barev s živým poměrem kontrastu u textů (uložení s < 4.5:1 zablokované, pracovní kopie `work` jde do vzhledu až „Uložit motiv“), přejmenovat, smazat (`dropThemeRefs`), export do schránky, import vložením JSON | `AppearanceThemes` `currentColors` |
| `panels/AppearancePages.tsx` | **Vzhled záložek**: řádek na záložku z `layout.pages` (`pageName`) — Styl / Motiv (vestavěné + vlastní) / Akcent, „Jako celek“ maže přepis (`setPageLook`) | `AppearancePages` |
| `panels/AppearanceMotion.tsx` · `apUi.tsx` · `appearance.css` · `en-look.ts` | krok Pohyb (9 ukázek, hraje jen po kliku, rychlost; rozvinutí ukáže i postupné naskakování) · společné prvky (`Seg` ≤ 4 volby, `Slider`, `Advanced`, `StateSwitch`, `lit`, `peekOn`/`peek`, `themeDraft`) | `AppearanceMotion` `Ap` `Seg` `Slider` `Advanced` `StateSwitch` `previewState` |
| `src/settings/previews/*` (`StripLookPreview`, `HoverPreview`, `MonitorPreview`, `previews.css`, `en.ts`) | HTML náhledy (prefix `apv-`): zavřený Ledge na „horní hraně monitoru“ a po najetí (ukázková data podle stavu, `.theme-scope` rozpracovaného vzhledu, žádná nekonečná animace); monitor v poměru `window.screen` (jinak 16 : 9 bez čísel) se šuplíkem ve skutečných px, na své pozici, se skutečnými sloupci a hustotou — kreslí se v plné velikosti a zmenší `transform: scale` (ResizeObserver) | `StripLookPreview` `HoverPreview` `MonitorPreview` |
| `src/layout/CardLook.tsx` (`CardLookPanel`, líný chunk; stav v `lookState.ts`) | **jediný panel vzhledu karty** (⋯ → Vzhled karty…): akcent, pozadí, hustota, hlavička; u `media` výběr varianty (miniatury `media/VariantPicker.tsx` `Thumb`, ukládá `media.variant` přes `saveVariant` — jeden zdroj s ikonkou v rohu karty a Nastavením → Osobní), u `projects` / `actions` výběr dlaždice → panel `tilelook/*` `openLook`. Staré vstupy (ikonka přehrávače „Víc možností karty…“, Nastavení → Přehrávač, pravý klik na dlaždici) jsou zkratky ke stejnému zdroji | `CardLookPanel` `openCardLook` `MediaVariants` `TileLooks` `openMediaCardLook` |
| `src/palette/styles.ts` | paleta Ctrl+K: „Styl: …“ → `configWithStyle` → `save_config` | `styleItems` |

## Nový design styl (jeden záznam + volitelný CSS blok)

1. Záznam do `STYLES` v `src/theme/looks.ts` (`id` = malá písmena, `name`/`desc` jako páry
   `["appearance.style.<id>", "…"]`, `parts` = motiv, hustota, rohy, krytí, tvar proužku, písmo, materiál,
   rozložení, šířka, rozmazání). Rust styl zná jen jménem; zrcadlo tabulky pro MCP je `src-tauri/src/mcp/look.rs`
   `STYLES` (test). Každý styl má jinou sestavu motiv + rozložení + materiál + písmo (test), rozložení se smí opakovat.
2. Angličtina: `appearance.style.<id>` + `.desc` v `src/settings/panels/en-look.ts`.
3. Volitelně: vlastní barvy = nový motiv (`themes.ts` + `Theme` a `palette()` v Rustu, testy kontrastu a
   zrcadla to hlídají; `accent: "…", "accent-2": "…"` v `themes.ts` musí sedět s `palette().accent`); ozdoba, která nejde proměnnou = blok `html[data-style="<id>"] …` na konci `styles.css`
   (jen statické — žádná animace, `filter` ani `backdrop-filter`, hlídá `theme.test.mjs`); nový materiál / písmo / rozložení = záznam v `MATERIALS` / `FONTS` /
   `LAYOUTS` + varianta enumu v `appearance/look.rs` (test zrcadel).

Vzhled JEDNÉ karty (akcent, pozadí, hlavička, hustota — vlna 8 P) přebíjí styl jen na `.deck-item[data-cl-*]`
(`layout.css`, [layout.md](layout.md)); karta bez `look` kreslí přesně styl. Token `--black` (`:root`, mimo motivy jako
`--white`) = černá kresba na světlém podkladu zástupce s vlastní barvou.

Rozhraní k mřížce karet (`.deck` > `.deck-item[data-size]`, mechanika v `layout/layout.css` → [layout.md](layout.md)):
styl dává `--deck-cols` (Klasický ho nevydá = 4), `--deck-gap` a `--deck-pad` (místo na stíny krajních karet;
čte je `html[data-layout] .deck` ve `styles.css`, `deck` je v SHARED hlídače); `--span-*` jsou k dispozici,
`.deck` si rozpětí ořízne sama přes `min(n, var(--deck-cols))`.

## Styly zavřeného Ledge (`strip.look`, vlna 7)

- `classic` = dnešní proužek (`strip.items`, `appearance.strip.style`), kreslí `strip::draw`. Ostatní styly obsah
  skládají samy z `Facts` (`looks::show`), `items`/`quiet`/`colors`/`custom`/`opacity` nepoužijí; doladění =
  `strip.lookHide` (co neukazovat). `appearance.strip.size` styly zvětší (`metrics.k`), `position` = osa (roh u `corner`).
- Nový styl: varianta `Look` + `ALL`/`id`/`name` + větev v `show` a `frame::spec` (případně `place`), řádek
  v tabulce `looks::tests::table_11_looks_x_6_states`, záznam v `src/strip/looks.ts` + `strip/en.ts` (zrcadla to hlídají).
- Výkon: kreslí se jen při změně modelu; jediné časovače = puls, ~240 ms změny šířky ostrůvku, 5 s střídání
  `ticker` s víc událostmi. „Hotovo“ vyprší s minutovým budíkem `emit` (`DONE_MS`), průběh skladby / časovače
  se přepočítá po minutách — po sekundách nic.
- Okno zůstává click-through a bez fokusu (stejné okno jako klasický proužek); `ghost` v klidu = `visible: false`,
  hover zóna pak leží na ose. Tah souborů (přihrádka) se kreslí ve tvaru stylu (`shelf_show`).

## Toky

- **Start**: `App` → `initAppearance()` → `api.getConfig()` → `applyAppearance` (CSS proměnné inline na
  `<html>`). Do té doby platí `:root` ze `styles.css` (= motiv dark).
- **Náhled**: panel `createEffect(() => applyAppearance(rozpracovaný))`, `onCleanup` → `savedAppearance()`.
  Klik na styl = `withStyle` do rozpracovaného configu (+ `drawer.glass` jen při změně rozmazání). Šířku okna
  a nativní proužek (`strip.look`) náhled v šuplíku nezmění — šířku ukáže monitor v kroku Otevřený, vzhled
  proužku galerie; skutečné okno / proužek se změní po Uložit.
- **Otevření**: `prepare` vezme `openPlan(styl, rychlost, geo)` → nastaví `plan.start` na `.shell`, `prepared`;
  `play` přehraje snímky; `closing` přehraje `closePlan` a pošle `closed`. Šířka okna: Rust
  `drawer_width_for` (předvolba stylu) — panel ji v náhledu nemění (projeví se po uložení).
- **Záložky**: `MainPage` → `setActivePage(page().id)` (při odchodu z hlavní stránky `""`); `applyAppearance` aplikuje
  `effectiveAppearance(normalize(raw), activePage)`, takže náhled v Nastavení (aktivní záložka `""`) vždy ukazuje celek.
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

## Vzhled záložek a vlastní motivy (kolo 13)

- Config: `appearance.pages`, `customThemes`, `customTheme` se zapisují jen neprázdné (`skip_serializing_if`), starý config je
  nemá. Rust validuje odkazy (`pages.*.theme` = vestavěný nebo existující vlastní, `customTheme` existuje) — smazání vlastního
  motivu proto v UI maže i odkazy (`dropThemeRefs`).
- Vlastní motiv je **vestavěný `base` + 11 barev**: vrstvy karet / linek se berou z `base`, jen když má stejný tón (světlý ⇔
  `lum(bg) > 0.18`, `LIGHT_LUM` v TS i Rustu), jinak z `dark` / `light`. Text se kontroluje na kartách (nad černou i bílou
  plochou), `bg` a `raised`; Rust je slabší pojistka (jen `bg` a `raised`) — editor je přísnější, takže co projde editorem, projde Rustem.
- `themeOf(a)` je jediné místo, které z `Appearance` dělá tokeny a tón; `applyAppearance`, `appearanceVars` a miniatury jdou přes něj.
  Swatche motivů musí předat `customTheme: ""`, jinak by ukázaly vybraný vlastní motiv.
- Paleta nativního proužku: `palette_for(&appearance)`; u vlastního motivu horní / dolní barva z `bg`, akcenty z `accent` / `accent-2`.
  Vzhled záložky se na proužek **neprojeví** (okno je jedno pro všechny záložky).
- Přechod při přepnutí záložky: `html[data-look-shift]` ~450 ms, jen `background-color`/`border-color`/`color`/`fill` (žádný
  `transform`, `filter` ani stín); při `motion: none` a systémovém „omezit pohyb“ se nenastaví.

## Testy

`cargo test looks` (11 stylů × 6 stavů, priorita a střídání, připomínka, jen reálná data, Pokročilé, angličtina,
převody faktů, config starý/nový, zrcadlo `looks.ts`, vlákno `media` pro styly s hudbou, umístění každého stylu na
monitoru vč. záporných a DPI, hover zóny, šířka ostrůvku); `node --test src/strip/looks.test.mjs` (id = Rust,
slovník, Pokročilé). `cargo test appearance` (validace, starý config z vlny 5, osa na DPI a záporných monitorech, ořez a předvolby
šířky, kontrast textu proužku ve všech 11 motivech, linka, ozdoby proužku, zrcadla TS, `motion::tests` — délka
animace i se systémovým „omezit pohyb“ a převod `drawer.animationMs`, tabulka). `npm run test:ui` =
`node --test src/theme/theme.test.mjs` (kontrast textů ≥ 4.5:1 ve všech motivech, dark = CSS, výchozí hodnoty
proměnných = Klasický, starý config, tabulka stylů, výběr stylu, zrcadla enumů a šířek s Rustem, plány animací,
normalizace, pořadí karet, presety, `widenMotion`), `node --test src/theme/pages.test.mjs` (kontrast = tabulka z Rustu, vlastní
motivy: tokeny, tón, normalizace, export / import, vzhled záložek, zrcadlo `appearance/pages.rs`); `cargo test appearance::pages`
(validace, limity, kontrast, `light`, `palette_for`, schéma).

## Co nečíst

`strip.rs` celý (jen `draw`), `drawer.rs` mimo `place`/`set_anchor`, `themes.ts` (jen hodnoty).
