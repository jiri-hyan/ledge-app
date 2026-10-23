# Codemapa: historie schránky (`cliphist`)

Historie zkopírovaného textu v paletě (`Ctrl+K`, prefix `v:` nebo příkaz „Historie schránky…“).
Soukromí napřed: **výchozí vypnuto**, jen text (`CF_UNICODETEXT`), jen paměť, hesla ze správců hesel
a prohlížeče se neukládají. Config `clipboard.*` (Nastavení → Osobní → Historie schránky,
zapnutí i přímo z palety). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/cliphist/mod.rs` | config, validace, čistá historie (dedup, strop, připnuté, náhled), soubor připnutých, stav za běhu, operace pro příkazy | `ClipboardConfig` `validate` `History` (`push` `trim` `pin` `remove` `clear_unpinned` `view` `from_pins`) `Entry` `EntryView` `Pin` `preview` `load_pins` `save_pins_to` `sync` `on_text` `on_lock` `list` `paste` `pin` `remove` `clear` `MAX_PINS` `MAX_TOTAL_BYTES` |
| `src-tauri/src/cliphist/filter.rs` | čistá rozhodnutí „neukládat“: značky formátů, zdrojová aplikace, prázdné / bílé / > 64 KB, tvary tokenů, vlastní zápis | `Fmt` `Skip` `formats_verdict` `pre_verdict` `text_verdict` `looks_secret` `app_excluded` `exe_key` `utf16_text` `Own` `is_own` `MAX_TEXT_BYTES` `DWORD_FORMATS` |
| `src-tauri/src/cliphist/win.rs` | tenká WinAPI: vlákno `cliphist`, okno `HWND_MESSAGE`, `AddClipboardFormatListener`, `WTSRegisterSessionNotification`; čtení formátů, DWORD, textu, vlastníka | `start` `stop` `seq` `run` `wndproc` `on_update` `formats` `read_text` `owner_exe` |
| `src-tauri/src/cliphist/apps.rs` | „Přidat běžící program“ (jen na klik): okna (EnumWindows) + správci hesel v oznamovací oblasti (Toolhelp); čisté názvy | `KNOWN` `known_label` `choice` `app_list` `running` `AppChoice` |
| `src-tauri/src/cliphist/tests.rs` | historie, náhled, připnuté v tempdiru, config, validace | |
| `src-tauri/src/commands/cliphist.rs` | Tauri: `clip_list` (jen náhledy), `clip_paste`, `clip_pin`, `clip_remove`, `clip_clear`, `clip_enable` (z palety), `clip_apps` | |
| `src/cliphist/model.ts` | čistá logika palety: `CLIP_PREFIX`, `clipQuery`, `clipRows` (vypnuto + Zapnout / prázdno / položky / Vymazat / Nastavení), `clipSub`, `clampSel` — test `model.test.mjs` | |
| `src/cliphist/api.ts` · `ClipSettings.tsx` · `cliphist.css` | most k jádru · sekce v panelu Osobní (lidsky popsané volby, vyloučené aplikace s lidskými názvy, výběr běžícího programu, „Přidat běžné správce hesel“, Vymazat) · styly `ch-*` | `clipApi` `ClipSettings` |
| `src/palette/Palette.tsx` · `more.ts` · `items.ts` | režim `v:` (seznam místo běžných výsledků), Enter / Ctrl+P / Ctrl+Del, patička se zkratkami · `clipCommand`, `clipItems` · `Section` `"clip"`, `PItem.clip` | `chooseClip` `clipKey` |
| `src/i18n/en-cliphist.ts` | angličtina `palette.clip.*`, `settings.clip.*`, `settings.f.clipboard.*` | |

Lepidlo: `config/*` (`Config.clipboard` + `cliphist::validate`), `settings.rs` (změna `clipboard` →
`cliphist::sync`), `lib.rs` (start `cliphist::sync`, `generate_handler!`), `Cargo.toml` (feature
`Win32_System_RemoteDesktop`), `settings/labels.ts` (`clipboard.*`), `panels/Personal.tsx` (`<ClipSettings />`),
`settings/settings.test.mjs` (panel `suplik` vykresluje i `ClipSettings.tsx`), `i18n/dicts.ts`.

## Toky

- **Zapnutí**: Nastavení nebo paleta (`clip_enable` uloží config) → `apply_config` → `sync` → `Live`
  (připnuté z disku, je-li `savePins`) + `win::start` (vlákno, okno, listener; čeká ≤ 2 s na hwnd).
  Vypnutí → `win::stop` (`WM_CLOSE` → `WM_DESTROY`: `RemoveClipboardFormatListener`,
  `WTSUnRegisterSessionNotification`, konec vlákna) + `LIVE = None` (historie z paměti pryč).
  `savePins: false` → soubor připnutých se smaže.
- **Kopírování**: `WM_CLIPBOARDUPDATE` → `on_update`: `OpenClipboard` (8× / 15 ms) → `formats` (+ jména,
  DWORD) → `owner_exe` → `pre_verdict` → JEN když projde, `read_text` (≤ 64 KB + 1 jednotka) →
  `CloseClipboard` → `on_text`: `is_own` (vlastní zápis = konec) → `text_verdict` → `History::push` →
  událost `cliphist`.
- **Paleta**: dotaz `v:…` → `clip_list` (při vstupu do režimu a po `cliphist`) → `clipRows` → Enter =
  `clip_paste(id)` → `shelf::clip::set_text` + `Own { text, seq }` → zavřít paletu, hláška „Ve schránce“.
  Ctrl+P = `clip_pin` (výběr jde s položkou), Ctrl+Delete = `clip_remove`, řádek „Vymazat historii“ =
  `clip_clear`, vypnuto = řádek „Enter ji zapne“ (`clip_enable`) + „Nastavení historie schránky…“.
- **Zamčení**: `WM_WTSSESSION_CHANGE` + `WTS_SESSION_LOCK` → `on_lock` → `clearOnLock` → nepřipnuté pryč.

## Invarianty a pasti

- **Co se nikdy neuloží** (`filter.rs`): formát `ExcludeClipboardContentFromMonitorProcessing` nebo
  `Clipboard Viewer Ignore` (přítomnost), `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard` = 0
  (nečitelná hodnota = 0), vlastník schránky v `excludeApps`, ne-text, prázdné, bílé znaky, > 64 KB (celý
  text pryč, nic se neořezává), se `skipSecrets` tvary tokenů. Text se čte AŽ po formátech a aplikaci.
- `CanUploadToCloudClipboard = 0` bereme přísněji (i lokální historie) — vědomé rozhodnutí soukromí.
- Vlastní zápis (`paste`): `Own.seq` = pořadové číslo schránky po zápisu; než ho známe, rozhoduje shoda
  textu. Vložená položka zůstává na svém místě (nový záznam nevznikne).
- Paměť: nejvýš `max` nepřipnutých + `MAX_PINS` připnutých a `MAX_TOTAL_BYTES` (2 MB) celkem; UI dostává
  jen náhled (≤ 160 znaků), celý text zůstává v Rustu.
- Shift+Enter „vložit do předchozího okna“ záměrně NENÍ: znamenalo by SendInput Ctrl+V do okna, kterému
  `focusreturn` jen zkusí vrátit fokus — když se to nepovede, text by se vložil jinam (terminál…).
- `clip_enable` uloží config z jádra; rozpracovaný (neuložený) koncept v otevřeném Nastavení ho při
  uložení přepíše tím, co je v konceptu.
- Testy nesahají na skutečnou schránku ani `%LOCALAPPDATA%` (soubor připnutých přes cestu v tempdiru).

## Výkon (idle < 0,1 % CPU, < 25 MB private)

- Vypnuto (výchozí): žádné vlákno, okno ani paměť. Zapnuto: jedno vlákno spí v `GetMessageW`, žádný
  časovač; práce jen při změně schránky. Změř `scripts/perf.ps1` (s funkcí zapnutou: +1 vlákno).
- `wtsapi32.dll` se načte staticky (import `WTSRegisterSessionNotification`).

## Testy

`cargo test cliphist` — `filter::tests` (tabulka formátů, `pre_verdict`, `exe_key`, text/tajemství,
UTF-16, vlastní zápis), `tests` (push/dedup/strop, připnuté, limit připnutých, strop paměti, náhled,
soubor připnutých v tempdiru, config výchozí vypnuto, validace), `apps::tests`. UI:
`src/cliphist/model.test.mjs`. WinAPI (listener, formáty, vlastník, zamčení) jen živě.

## Co nečíst

`shelf/` (jen `clip::set_text`), `ports.rs` (jen `process_image`), `focusreturn.rs`.
