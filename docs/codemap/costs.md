# Codemapa: útrata Claude (odhad v cenách API)

Sekce „Útrata Claude (odhad)“ dole v kartě Claude: tokeny a odhad ceny **v cenách API, ne účet**
(předplatné Pro/Max se po tokenech neúčtuje) — dnes / 7 dní / 30 dní, top projekty, modely a sloupce po
dnech. Zdroj = lokální transcripty Claude Code `~/.claude/projects/<slug>/**/*.jsonl` (session i
`subagents/…`). Návod a config: [agents/sessions.md](../agents/sessions.md) → „Útrata Claude (odhad)“.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/usage/parse.rs` | řádek JSONL → záznam (součty tokenů nasycené, ne přetékající) (jen `type:"assistant"` s `message.usage`), předfiltr bajtů, klíč deduplikace, `cwd` z libovolného řádku | `parse_line`, `Rec`, `Tokens`, `maybe_usage`, `fnv`, `iso_ms`, `line_cwd` |
| `src-tauri/src/usage/price.rs` | ceník USD / 1M tokenů jako data (datum ověření + zdroj), kanonické id modelu, pásma (Haiku 5.5 > 100k, fast, `inference_geo: us`), cena | `MODELS`, `VERIFIED`, `SOURCE`, `canonical`, `lookup`, `tier`, `cost`, `display`, `TIER_*`, `LONG_PROMPT` |
| `src-tauri/src/usage/index.rs` | inkrementální index na disku: výpis souborů, čtení od offsetu po celých řádcích (řádek nad `MAX_LINE` = 8 MB se přeskočí bez načtení do paměti), dedup, agregace den × cwd × model × pásmo a (kolo 22) dnešek + včerejšek po místních hodinách × cwd (`hours`, verze 2 = jednou přepočet), uložení | `Index`, `Index::update`, `Index::add`, `list_files`, `Found`, `Window`, `Stats`, `FileEntry`, `Key`, `HourKey`, `HOUR_DAYS`, `MAX_LINE`, `Sum`, `load`, `save`, `encode_seen`, `decode_seen`, `VERSION` |
| `src-tauri/src/usage/floor.rs` | (kolo 23, E3) podlaha nové relace/agenta: první usage souboru (`First`, doplňuje `index::read_file`; starý index `fill` jen čtením začátku), typ agenta z `agent-*.meta.json`, medián 7 dní + trend, `Floor` do `Summary.floor` | `First`, `first_of`, `fill`, `scan_head`, `agent_type`, `clean_type`, `is_subagent`, `med`, `build`, `Floor`, `FloorRow`, `MIN_TREND`, `ROWS` |
| `src-tauri/src/ctxsize.rs` | (kolo 23, E1) kontext živé session: poslední usage v konci transcriptu (`transcript_path` z hooku), poměr k prvnímu volání, úrovně 250 k / 350 k, cache podle mtime+délky (15 s), povolené cesty; příkaz `session_context` (`commands/sessions.rs`) | `Ctx`, `level`, `ratio`, `context`, `last_usage`, `read_context`, `for_path`, `allowed`, `WARN`, `HIGH`, `THROTTLE_MS` |
| `src-tauri/src/usage/view.rs` | index → přehled (rozsahy, top projekty/modely + „ostatní“, dny, dnešek po hodinách a projektech `hours` pro kartu Claude dnes) | `build`, `Summary`, `Range`, `Line`, `Day`, `HourRow`, `hour_rows`, `Split`, `TOP`, `date_of` |
| `src-tauri/src/costslive.rs` | za běhu: žádost s omezením, vlákno s nízkou prioritou, průběh, přehled v paměti, projekt z `cwd`; kolo 22: kdo přehled chce (`wanted`: sekce v kartě Claude nebo karta Claude dnes `USAGE_CARD`), místní hodina pro časovou osu, widget na ploše (`request_desk` 1× / `DESK_EVERY`, `desk_view`) — [board.md](board.md) | `request`, `view`, `CostsView`, `compute`, `project_name`, `local_day`, `local_hour`, `wanted`, `request_desk`, `desk_view`, `THROTTLE`, `DESK_EVERY` |
| `src-tauri/src/commands/costs.rs` | Tauri příkaz | `costs_refresh` |
| lepidlo | `lib.rs` (`mod usage`, `mod costslive`, registrace `costs_refresh`), `snapshot.rs` (`Snapshot.costs`), `config/*` (`ClaudeConfig.costs`), `Cargo.toml` (`memchr`, už ve stromu přes regex) | |
| `src/costs/model.ts` | čistá logika UI + typy zrcadla (`CostsView`) — test `model.test.mjs` | `compact`, `usdDigits`, `bars`, `share`, `progressPct`, `pickRange`, `floorTrend` (kolo 23: podlaha) |
| `src/sessions/ctx.ts` · `ContextBar.tsx` · `ctx.css` · `src/i18n/en-ctx.ts` | pruh kontextu pod živou session (kolo 23, E1; `ClaudeSessions.tsx` `SessionRow`, `claude.contextBar`) — test `ctx.test.mjs`; angličtina i pro podlahu (`sessions.costs.floor*`) | `ctxFill`, `ctxTicks`, `showRatio`, `showTip`, `sameCtx`, `ContextBar`, `api.sessionContext` |
| `src/costs/CostsSection.tsx` · `costs.css` | sekce v kartě Claude (`ClaudeSessions.tsx` dole), rozbalení a rozsah v `localStorage` (jen pohodlí) · styly `cost-*` | `CostsSection`, `usd`, `tok` |
| `src/i18n/en-costs.ts` | angličtina, klíče `sessions.costs.*` (Nastavení: `settings.f.claude.costs` v `settings/en.ts`, `settings.claude.costs` v `settings/panels/en.ts`) | |
| Nastavení | `settings/panels/Claude.tsx` sekce „Útrata“ (přepínač `claude.costs`), `settings/labels.ts` | |

## Tok dat

Rozbalení sekce / otevření šuplíku s rozbalenou sekcí (`view === "full"`) → `api.costsRefresh()` →
`costslive::request` (vypnuto / běží / < 60 s od posledního = nic) → vlákno `costs`
(`THREAD_MODE_BACKGROUND_BEGIN`) → `index::load` (`%LOCALAPPDATA%\Ledge\usage-index.json` + `.seen`) →
`list_files` (jen výpis složek) → `Index::update` (průběh → `mark_dirty`) → `save` → `view::build` s projektem
podle `cwd` → `Live.summary` → `Snapshot.costs` → `CostsSection`. Vlákno pak skončí.

## Invarianty a pasti

- **Žádný sken v klidu:** přepočet jen na žádost UI s rozbalenou sekcí v otevřeném šuplíku, nejvýš 1× za
  `THROTTLE` (60 s), nikdy dva naráz; vypnuté `claude.costs` = nic se nečte; skrytá sekce/karta (`drawer.sections.claudeCosts`
  / `.claude`, `costslive::enabled`) = nic se nečte, jen když přehled neukazuje i karta `usage` na záložce
  nebo widget na ploše (`costslive::wanted`). V paměti zůstane
  jen hotový `Summary` (pár kB); index, množina viděných i výpis souborů se po běhu zahodí.
- **Inkrement:** soubor s mtime před místní půlnocí začátku okna se neotevře (stat z výpisu složky) a vypadne
  z indexu; nezměněný (délka + mtime) se neotevře; změněný se čte od `FileEntry.off` jen do délky z výpisu a
  jen celé řádky — poslední bez `\n` (Claude Code ho píše) se přečte příště. Soubor kratší než offset nebo jiné
  okno = celý index od nuly (`Stats.rebuilt`). Smazaný soubor jen vypadne z indexu, jeho dny zůstanou.
- **Deduplikace:** jedna odpověď API se v JSONL opakuje (každý blok obsahu = řádek se stejnou usage; resume/fork
  kopíruje historii do nového souboru) → klíč FNV-1a z `requestId` + `message.id`, množina přes všechny soubory
  v okně (`Index.seen`, 12 B / odpověď na disku). Počítá se první výskyt (jako `token-audit.py` ve WorkSpace).
  Bez dedupu ~4× přepočet.
- **Den = místní půlnoc** podle `timestamp` záznamu (`costslive::local_day`); `cwd` záznamu, jinak poslední
  `cwd` souboru. Projekt: `notes::project_for_cwd` (hranice komponenty), worktree → hlavní checkout
  (`resolve::main_checkout`), jinak název složky; mapuje se při každém přepočtu (změna configu se projeví).
- **Nic vymyšleného:** model mimo `MODELS` (přesná shoda kanonického id) = tokeny ano, cena `None` („—“) a tokeny
  „bez ceny“ zvlášť; fast mode u modelu bez fast ceny taky `None`. Během přepočtu bez dat „počítám… N %“, ne nuly;
  den bez odpovědí je naměřená 0. Ceny měň jen v `price.rs` (+ `VERIFIED`, test `costs_table`).
- Rust ani testy nesahají na skutečné `~/.claude` ani `%LOCALAPPDATA%\Ledge` — testy indexu běží nad tempdir
  s mtime podstrčeným ve `Found`.

## Podlaha a kontext (kolo 23)

- **První usage** je položka `FileEntry.first` (+ `first_done`, `agent`; `serde(default)` → starý index se načte bez
  přepočtu). Nový/změněný soubor ji zapíše při běžném čtení (`read_file`, žádný druhý průchod); položka bez
  `first_done` (starý index) se doplní `floor::fill` = čtení jen začátku souboru po `FileEntry.off`. První usage se
  bere i mimo okno a mimo dedup (resume kopie = původní první volání), den se filtruje až v `floor::build`.
- Subagent bez `.meta.json` se do žádné tabulky nepočítá (typ neznáme); soubory subagentů nejdou do projektů.
- **Kontext session** čte `ctxsize` jen na žádost UI (`session_context`) z `Session.transcript` (hook `transcript_path`);
  cesta jen `<.claude>/projects/**/*.jsonl`. Řádky `isSidechain:true` se přeskakují. Zdroj stavového řádku
  (`context_window`) se NEpoužil: wrapper zapisuje jen `rate_limits` a přepis je nutný stejně pro poměr k začátku.

## Testy

`usage::parse::tests` (řádky, nuly, chybějící pole, klíč, `cwd`), `usage::price::tests` (kanonická id, ceník
známý/neznámý, cache 5m/1h, pásma Haiku/fast/us), `usage::index::tests` (dedup 3× + resume kopie + subagent,
inkrement od offsetu, useknutý řádek, zkrácený soubor, staré soubory bez otevření, místní půlnoc, záloha `cwd`,
změna okna, uložení/načtení, rozbité soubory), `usage::view::tests` (rozsahy, top + ostatní, dny),
`costslive::tests` (projekt podle cwd, den/půlnoc), `usage::floor::tests` (medián, cesty subagentů, skupiny/období/trend, první průchod, doplnění starého indexu bez přepočtu, soubor bez usage), `ctxsize::tests` (úrovně, poměr, poslední usage, cesty, čtení z disku + cache). UI: `src/costs/model.test.mjs`, `src/sessions/ctx.test.mjs`.

## Měření (2026-10-17, tento stroj, release, mimo Ledge nad kopií modulů `usage/*`)

- `~/.claude/projects`: 5 953 souborů `*.jsonl`, z toho 2 547 starších než okno (neotevřeny), 3 406 v okně = 6,87 GB.
- **První plný běh:** 69,7 s zdi, ale jen 6,3 s CPU (user 2,7 + kernel 3,6) — brzdí čtení disku (stroj byl
  vytížený jinými běhy); špička paměti 20,5 MB pracovní sady procesu (z velké části buffer nejdelšího řádku),
  po běhu se uvolní. 172 097 odpovědí + 168 632 duplicit (~2×). UI mezitím ukazuje „počítám… N %“.
- **Inkrement hned potom:** 0,55 s zdi, 0,22 s CPU (465 ms výpis složek, 6 otevřených souborů, 0,8 MB).
- Index na disku: `usage-index.json` 947 kB + `usage-index.seen` 2 065 kB.
