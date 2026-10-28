# Codemapa: útrata Claude (odhad v cenách API)

Sekce „Útrata Claude (odhad)“ dole v kartě Claude: tokeny a odhad ceny v cenách API, ne účet (předplatné Pro/Max se po tokenech neúčtuje) — dnes / 7 dní / 30 dní, top projekty, modely a sloupce po dnech. Zdroj = lokální transcripty Claude Code `~/.claude/projects/<slug>//*.jsonl` (session i `subagents/…`). Návod a config …

Detail (toky, invarianty, testy, co nečíst, historie): [costs-detail.md](costs-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/usage/parse.rs` | řádek JSONL → záznam (součty tokenů nasycené, ne přetékající) (jen `type:"assistant"` s `message.usage`), předfiltr bajtů, klíč deduplikace, `cwd` z libovolného řádku | `parse_line`, `Rec`, `Tokens`, `maybe_usage`, `fnv`, `iso_ms`, `line_cwd` |
| `src-tauri/src/usage/price.rs` | ceník USD / 1M tokenů jako data (datum ověření + zdroj), kanonické id modelu, pásma (Haiku 5.5 > 100k, fast, `inference_geo: us`), cena | `MODELS` `VERIFIED` `SOURCE` `canonical` `lookup` `tier` `cost` `display` `TIER_*` |
| `src-tauri/src/usage/index.rs` | inkrementální index na disku: výpis souborů, čtení od offsetu po celých řádcích (řádek nad `MAX_LINE` = 8 MB se přeskočí bez načtení do paměti), dedup, agregace den × cwd × model × pásmo a (kolo 22) … | `Index` `Index::update` `Index::add` `list_files` `Found` `Window` `Stats` `FileEntry` |
| `src-tauri/src/usage/floor.rs` | (kolo 23, E3) podlaha nové relace/agenta: první usage souboru (`First`, doplňuje `index::read_file`; starý index `fill` jen čtením začátku), typ agenta z `agent-*.meta.json`, medián 7 dní + trend … | `First` `first_of` `fill` `scan_head` `agent_type` `clean_type` `is_subagent` `med` |
| `src-tauri/src/ctxsize.rs` | (kolo 23, E1) kontext živé session: poslední usage v konci transcriptu (`transcript_path` z hooku), poměr k prvnímu volání, úrovně 250 k / 350 k, cache podle mtime+délky (15 s), povolené cesty … | `Ctx` `level` `ratio` `context` `last_usage` `read_context` `for_path` `allowed` `WARN` |
| `src-tauri/src/usage/view.rs` | index → přehled (rozsahy, top projekty/modely + „ostatní“, dny, dnešek po hodinách a projektech `hours` pro kartu Claude dnes) | `build` `Summary` `Range` `Line` `Day` `HourRow` `hour_rows` `Split` `TOP` `date_of` |
| `src-tauri/src/costslive.rs` | za běhu: žádost s omezením, vlákno s nízkou prioritou, průběh, přehled v paměti, projekt z `cwd`; kolo 22: kdo přehled chce (`wanted` … | `request` `view` `CostsView` `compute` `project_name` `local_day` `local_hour` `wanted` |
| `src-tauri/src/commands/costs.rs` | Tauri příkaz | `costs_refresh` |
| lepidlo | `lib.rs` (`mod usage`, `mod costslive`, registrace `costs_refresh`), `snapshot.rs` (`Snapshot.costs`), `config/*` (`ClaudeConfig.costs`), `Cargo.toml` (`memchr`, už ve stromu přes regex) |  |
| `src/costs/model.ts` | čistá logika UI + typy zrcadla (`CostsView`) — test `model.test.mjs` | `compact` `usdDigits` `bars` `share` `progressPct` `pickRange` `floorTrend` |
| `src/sessions/ctx.ts` · `ContextBar.tsx` · `ctx.css` · `src/i18n/en-ctx.ts` | pruh kontextu pod živou session (kolo 23, E1; `ClaudeSessions.tsx` `SessionRow`, `claude.contextBar`) — test `ctx.test.mjs`; angličtina i pro podlahu (`sessions.costs.floor*`) | `ctxFill` `ctxTicks` `showRatio` `showTip` `sameCtx` `ContextBar` `api.sessionContext` |
| `src/costs/CostsSection.tsx` · `costs.css` | sekce v kartě Claude (`ClaudeSessions.tsx` dole), rozbalení a rozsah v `localStorage` (jen pohodlí) · styly `cost-*` | `CostsSection`, `usd`, `tok` |
| `src/i18n/en-costs.ts` | angličtina, klíče `sessions.costs.*` (Nastavení: `settings.f.claude.costs` v `settings/en.ts`, `settings.claude.costs` v `settings/panels/en.ts`) |  |
| Nastavení | `settings/panels/Claude.tsx` sekce „Útrata“ (přepínač `claude.costs`), `settings/labels.ts` |  |
