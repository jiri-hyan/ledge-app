# Codemapa: limity Claude 5h / týden

Karta Claude (a volitelně proužek) ukazuje spotřebu 5h a týdenního okna Claude s odpočtem resetu;
v čase resetu okna, které dosáhlo 100 %, vyjede krátký peek. Zdroj = stavový řádek Claude Code přes
průchozí wrapper Ledge, záložně `~/.claude/usage-log/latest.json`. Návod: [agents/sessions.md](../agents/sessions.md) → „Limity Claude 5h / týden“.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/limits.rs` | čistá logika: vzorek ze vstupu stavového řádku i ze souboru, čas (s/ms/ISO), výběr čerstvějšího, stáří, odpočet, text proužku, připomínka (arm/due) | `Sample`, `Window`, `Kind`, `Source`, `from_status_input`, `from_file`, `to_file`, `should_write`, `fresher`, `state`, `State`, `strip_text`, `top_fresh`, `Armed`, `arm`, `due`, `next_reset`, `STALE_MS`, `REMIND_GRACE_MS` |
| `src-tauri/src/statusline.rs` | wrapper `ledge.exe --statusline` (volá `main.rs` před Tauri): zápis vzorku, Git Bash / PowerShell jako Claude Code, průchod stdout byte po bytu | `main`, `record`, `sample_path`, `bash_candidates`, `git_bash`, `shell_command`, `run_original`, `ORIGINAL_TIMEOUT` |
| `src-tauri/src/statuslinesetup.rs` | merge/unmerge/status klíče `statusLine` nad textem (`hooksetup::J`), uložený originál, zápis | `our_command`, `is_ours_cmd`, `merge`, `Merged`, `unmerge`, `status`, `Stored`, `original_command`, `store_path`, `load_stored`, `write_atomic`, `status_at`, `install_at`, `remove_at`, `StatusLineStatus` |
| `src-tauri/src/limitslive.rs` | za běhu: cache obou souborů (mtime+délka), stav zapojení, watcher, připomínka (peek + FLASH), segment proužku | `tick`, `next_deadline`, `view`, `LimitsView`, `strip_segments`, `start_watch`, `invalidate`, `NOTICE_MS` |
| `src-tauri/src/commands/limits.rs` | Tauri příkazy Nastavení | `statusline_status`, `statusline_install`, `statusline_remove` |
| lepidlo | `main.rs` (`--statusline`), `lib.rs` (`statusline_main`, `limitslive::start_watch`, registrace), `snapshot.rs` (`Snapshot.limits`), `threads/emit.rs` (`tick` + `next_deadline` v `deadlines`), `stripview.rs` (`StripItem::Limits`), `config/*` (`StripItem::Limits`, `ClaudeConfig.limits_reminder`), `hooksetup.rs` (`J::get/get_mut/str/obj`, `parse`, `render`, `read` jsou `pub(crate)`) | |
| `src/limits/model.ts` | čistá logika UI (stav okna, odpočet, pruh, barva) — test `model.test.mjs` | `winState`, `left`, `bar`, `level`, `pctText`, `otherDay` |
| `src/limits/LimitsRow.tsx` · `LimitsPeek.tsx` · `StatusLineSetup.tsx` · `limits.css` | řádek v kartě Claude (pod hlavičkou) · peek resetu (v `cards/Peek.tsx` před `ReminderPeek`) · zapojení v `settings/panels/Claude.tsx` (kotva `claude.statusLine`, přepínač `claude.limitsReminder`) · styly `lim-*` | `LimitsRow`, `leftText`, `resetClock`, `LimitsPeek`, `StatusLineSetup` |
| `src/i18n/en-limits.ts` | angličtina, klíče `sessions.limits.*` (Nastavení: `settings.f.claude.statusLine`, `.limitsReminder` v `settings/en.ts`) | |

## Tok dat

Claude Code → `ledge.exe --statusline` (stdin JSON) → `statusline::record` → `%LOCALAPPDATA%\Ledge\ratelimits.json`
→ watcher `limits-watch` (`CHANGED` + `mark_dirty`) → emit vlákno `limitslive::tick` (stat, čtení změněných,
`limits::arm`/`due`) → `view()` do `Snapshot.limits` → `LimitsRow`. Wrapper pak spustí původní příkaz
z `statusline-original.json` a jeho stdout vrátí Claude Code beze změny.

## Invarianty a pasti

- **Žádná síť, žádné tokeny:** jen dokumentovaný vstup stavového řádku (`rate_limits`, jen Pro/Max, až po
  první odpovědi API, okna mohou chybět; po `resets_at` je Claude Code zahodí) a cizí `latest.json`.
- `--statusline` se zpracuje v `main` PŘED `ledge_lib::run()` — žádné okno, žádný single-instance plugin.
  GUI subsystém (release) píše do roury od Claude Code normálně (ověřeno malým GUI exe: bash roura i
  `spawnSync` z node). Příkaz wrapperu míří na `current_exe()` při zapojení (exe, které `--statusline` umí).
- Wrapper nikdy nespadne a nespustí sám sebe (`original_command` odmítne `is_ours_cmd`). Bez původního
  příkazu nevypíše nic. Spawn jen přes `procman::Job::spawn` (pozastavený → v jobu → rozběh) + CREATE_NO_WINDOW.
- Klid: žádné pollování. `tick` stat-uje jen po události watcheru nebo nejvýš 1× za 30 s při probuzení, které
  nastalo stejně (minuta hodin); budí se navíc jen v `next_deadline` (reset okna na 100 %, konec peeku).
  `view`/`strip_segments` čtou jen paměť.
- Nic vymyšleného: chybějící okno = „—“; starý vzorek (> `STALE_MS`) = „staré (před X min)“, proužek ho
  nehlásí; po `resetsAt` žádná procenta, jen „obnoveno“. Procenta dolů (99,6 → 99).
- Připomínka: jen okno ≥ 100 % s `resetsAt` v budoucnu (zapamatované v `Armed`, pozdější vzorek bez okna ho
  nesmaže); ve fullscreen / vypnuto propadne, reset starší `REMIND_GRACE_MS` tiše. Peek bez fokusu, zavře se
  po `NOTICE_MS`, jen když peek nedrží schválení / otázka / připomínka poznámky.
- `statusLine`: cizí hodnota se uloží celá a odpojení ji vrátí na totéž místo; rozbitý `statusline-original.json`
  = chyba (nehádat), chybějící = klíč se smaže. Rozbitý settings.json = chyba, nikdy přepsání.
- Příkaz wrapperu závisí na shellu zjištěném při zapojení (cesta s mezerou: `"…"` pro Git Bash, `& '…'` pro
  PowerShell); po instalaci / odinstalaci Git Bash na PC s mezerou v cestě je potřeba „Zapojit“ znovu.

## Testy

`limits::tests` (čas, oba formáty vstupu, čerstvější, přepis, stav, proužek, připomínka),
`statusline::tests` (`record` nad tempdir, kandidáti Git Bash, shell, průchod bajtů + termín přes Git Bash —
přeskočí bez Git Bash), `statuslinesetup::tests` (příkaz pro oba shelly, merge/unmerge přesně, chybějící /
cizí / neobjektová hodnota, rozbité vstupy, soubory nad tempdir). UI: `src/limits/model.test.mjs`.
Žádný test nesahá na skutečné `~/.claude` ani `%LOCALAPPDATA%\Ledge`.
