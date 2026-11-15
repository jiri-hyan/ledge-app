# Codemapa: živá hrana zavřeného Ledge (římsa)

Hrana pod vlastním zavřeným Ledge (`strip.look = custom`, `strip.compose.deco` / `fill`, kolo 28): stálá linka /
záře / obrys a živé efekty (přelévání, čárky, kometa, rozsvícení, dech, obrys přechodem, naplnění), barevné sady
bez duhy a „Podle stavu Clauda“. Pravidla: výchozí = vzhled před kolem 28; snímky (25 fps, vlastní časovač `FX_TIMER` vlákna
proužku) jen během animace — Claude pracuje / čeká, jednorázové naplnění, „Vyzkoušet“, nebo výslovné `always`;
ve hře (proužek skrytý), při nerušit / prezentaci (`fx_calm`) a bez animací (`animate`) statický snímek. Svit
pod tělem má rezervu okna (průhledná, click-through, hover zóna ji nebere). Širší souvislosti: [appearance.md](appearance.md),
detail řádků v [appearance-detail.md](appearance-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/looks/edgefx.rs` | čistá logika: `EdgeLive` (stav → efekt), `live_state` (čeká > hotovo > limit ≥ `LIMIT_AT` (= `capsule::WARN_PCT`, 75 %) > pracuje), `apply` (tón stavu, „Vyzkoušet“), `SETS` / `colors`, `span` / `span_x` (střed 40 % beze změny), fáze `flow_phase` / `breath` / `comet` / `sweep` / `dashes`, `frames`, `reserve`; tabulkové testy + `ts_mirror` | `EdgeLive` `LiveState` `Fx` `apply` `frames` `reserve` `live_state` |
| `src-tauri/src/stripfx.rs` | kreslení D2D: `under` (svit pod tělem), `line` (efekty a přechod 3 barvami), `fill` (pozadí 3 barvami / přelévané); `prepare` před snímkem (hodiny naplnění, „Vyzkoušet“) → časovač v `strip.rs`; IPC `edge_try` | `prepare` `Clock` `Try` `under` `line` `fill` `edge_try` |
| `src-tauri/src/looks/composedeco.rs` | config `Deco` (nové `color3`, `pct`, `speed`, `power`, `always`, `live`) a `Fill` (`via`, `flow`), `Paint` (`span`, `c3`, `fx`), `restyle` | `Deco` `Fill` `Paint` `paint` `restyle` |
| `src/strip/edgefx.ts` · `edgefx.test.mjs` | zrcadlo pro Nastavení: `COMPOSE_EDGES`, `EDGE_LENS`, `LIVE_STATES`, `EDGE_SETS`, `SET_COLORS`, `normDeco`, `liveEdge`, `stateTone`, `spanInset` | `normDeco` `liveEdge` `stateTone` |
| `src/settings/panels/ComposeEdge.tsx` · `panels/en-edge.ts` | editor hrany uvnitř `ComposeDeco` + sekce „Podle stavu Clauda“ (kotva `strip.compose.deco.live`) s „Vyzkoušet“ (`api.edgeTry`); angličtina | `EdgeBlock` `EDGE_NAME` |
| `src/settings/previews/EdgeFx.tsx` | HTML náhled hrany (`edgeLook`, `EdgeLayer`; CSS blok kola 28 v `previews.css`, hýbe se jen `[data-run]`, reduced-motion = statické) | `edgeLook` `EdgeLayer` `liveOf` |

## Okna a styly „nekrade fokus“ (kolo 34, L160)

Jednotný styl bez aktivace žije v `src-tauri/src/winstyle.rs`: tabulka `Win` + `profile(Win)` (invariant: **každé okno Ledge s HWND má řádek**, test `profile_table`; nové okno = nová varianta), `no_activate_ex` při vzniku přes Win32, `apply_profile(hwnd, Win)` po `build` oken z tao, `no_activate(hwnd, on)` za běhu. Změnu zobrazení
(monitor, DPI, hlavní panel) chytá wndproc okna proužku (top-level; message-only okna broadcasty nedostanou) → `state::display_event`, viz
[core-detail.md](core-detail.md) (Změna zobrazení).

| okno | místo vzniku | styl bez aktivace |
|---|---|---|
| proužek | `strip.rs` `CreateWindowExW` | `winstyle::no_activate_ex()` + LAYERED, TRANSPARENT, TOPMOST |
| šuplík | `drawer.rs` (WebView2), `init` / `focus` / `place` | `winstyle::no_activate(hwnd, true / false)` — odebírá se při výslovném kliku / zkratce |
| cíl přetažení přihrádky | `shelf/drop.rs` | `winstyle::no_activate_ex()` + LAYERED, TOPMOST |
| desk (widgety) | `desk/win.rs` `open` | `apply_profile(Win::Desk)` hned po `build` (nikdy se neaktivuje) |
| lístek | `pins/win.rs` `open` / `activate` / `watch` | `apply_profile(Win::Pin)` po `build`; výslovný klik (`pointerdown` → `pin_activate`) nebo nový lístek = `no_activate(false)` + `SetForegroundWindow`; `Focused(false)` → zase zamčeno |
| watcher / okno pozadí rozšíření | `ext/wall/mod.rs` | vlastní `WS_EX_TOOLWINDOW | NOACTIVATE` při `CreateWindowExW` (ext proces, `Win::UserOpened` se netýká) |
| message-only (`cliphist/win.rs`, `rawpush.rs`) | `HWND_MESSAGE` | bez viditelnosti, profil netřeba |
| PDF náhled, webpanel, Studio | `notes/pdf.rs`, `webpanel.rs`, `studio.rs` | `Win::UserOpened`: záměrně aktivní (výslovná akce) |
