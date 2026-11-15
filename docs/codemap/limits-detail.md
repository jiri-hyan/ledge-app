# Codemapa: limity Claude — detail

Jádro mapy (soubory a symboly): [limits.md](limits.md).

## Tok dat

Claude Code → `ledge.exe --statusline` (stdin JSON) → `statusline::record` → `%LOCALAPPDATA%\Ledge\ratelimits.json`
→ watcher `limits-watch` (`CHANGED` + `mark_dirty`) → emit vlákno `limitslive::tick` (stat, čtení změněných,
`limits::arm`/`due`) → `view()` do `Snapshot.limits` → `LimitsRow`. Wrapper pak spustí původní příkaz
z `statusline-original.json` a jeho stdout vrátí Claude Code beze změny.

## Invarianty a pasti

- **Síť a token jen s volbou `claude.limitsFetch`** (výchozí vypnuto): pevná URL `https://api.anthropic.com/api/oauth/usage`,
  bez přesměrování, ≥ 10 min odstup, ne při čerstvém vzorku ze stavového řádku, ne ve hře, jen s viditelnou kartou /
  limity v proužku. Token se čte při každém dotazu (jen `accessToken`), nikdy refresh / zápis / log; 401 nebo
  vypršelý = stav `expired` a pokus nejdřív za 30 min nebo po změně `.credentials.json`.
- **Aplikace Claude** (`limitsDesktop`): `plan-usage-history.json` nese jen procenta (aplikace si sama ptá
  `claude.ai/.../usage` à 15 min, když to má zapnuté); reset se převezme jen z okna, které v čase vzorku běželo
  (`fill_resets`); okno starší než jeho délka se zahodí. Bez watcheru (stat při `STAT_EVERY`).
- **Jinak žádná síť, žádné tokeny:** jen dokumentovaný vstup stavového řádku (`rate_limits`, jen Pro/Max, až po
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

## Tabulka prahů (jediná pravda, kolo 34 N1)

Barvy a stavy limitu Clauda (a zátěže CPU/RAM/GPU) mají JEDNU tabulku: **warn 75 %, bad 90 %**. Rust
`looks/capsule.rs` `WARN_PCT` / `FULL_PCT`, TS zrcadlo `src/charts/thresholds.ts` (`WARN_PCT`, `FULL_PCT`, `toneOf`);
`thresholds.test.mjs` čte Rust a ověřuje, že všechna místa tabulku používají. Nové místo s „procentem limitu“
importuje konstantu, nikdy nepíše číslo.

| Místo | Práh | Soubor |
|---|---|---|
| Barva pruhu karty / hrdiny / hover (`level`) | warn 75 · bad 90 | `limits/model.ts` |
| Zvýraznění času resetu 5h při práci (`HINT_FROM_PCT`) | 75 | `limits/model.ts` |
| Prstenec, barva dílu proužku, `heat` | 75 → 90 (spojitě) | `looks/capsule.rs` |
| Tečka segmentu proužku limitů | 75 · 90 | `limitslive.rs` `strip_segments` |
| Čas resetu v textu proužku | od 90 | `limits.rs` `strip_text` |
| Živá hrana „limit“ (`LIMIT_AT`) | 75 | `looks/edgefx.rs` |
| „Jen při aktivitě“ (`is_active` limit 5h / týden, RAM) | 75 | `looks/compose.rs` |
| Ostrůvek „limit blízko“ (`limitNear`) | 90 (nebo ETA ≤ 30 min) | `hover/priority.ts` |
| Zátěž CPU / RAM / GPU (`loadOf`, `sysmon::level`) | 75 · 90 | `integrations/format.ts`, `sysmon/calc.rs` |

Mimo tabulku záměrně: oznámení `ALERT_LEVELS` 80 / 90 (jednorázová událost za okno, text v Nastavení „80 / 90 %“,
`limits.rs`) — není to barva, ale spoušť upozornění. Zaokrouhlení procent limitu je vždy dolů (viz `pctText`).

## Testy

`limits::tests` (čas, oba formáty vstupu, čerstvější, přepis, stav, proužek, připomínka),
`limitsrc::tests` (API odpověď, round-trip cache, historie aplikace v1/v2, doplnění resetu, cesty, token, odstup),
`limitsfetch::tests` (pevná URL), `statusline::tests` (`record` nad tempdir, kandidáti Git Bash, shell, průchod bajtů + termín přes Git Bash —
přeskočí bez Git Bash), `statuslinesetup::tests` (příkaz pro oba shelly, merge/unmerge přesně, chybějící /
cizí / neobjektová hodnota, rozbité vstupy, soubory nad tempdir). UI: `src/limits/model.test.mjs`, `src/limits/sources.test.mjs`, `src/layout/hero.test.mjs` (důvody).
Žádný test nesahá na skutečné `~/.claude` ani `%LOCALAPPDATA%\Ledge`.

- Upozornění 80/90 % (L237): okno bez `resetsAt` (zdroj Desktop) se odjistí, když `pct` klesne o víc než `REARM_DROP` (5 b.) pod ohlášený práh (`alerts_due`, test `alerts_rearm_without_reset_time`).

## Offline, backoff a probuzení (`netstate.rs`, kolo 34, L220 / L222 / L224)
- Jedna politika pro zdroje ze sítě: `Policy {every, retry}` + `needs_fetch(policy, busy, last_ok, last_attempt, fails, now)` (nahradilo 3 kopie `needs_fetch` v `weather.rs`, `calendar/live.rs`, `accounts/mail.rs`; `calsync` používá `calendar::live::POLICY`). Odstup po pokusu: `retry`, po 2. neúspěchu 2×, 4×… strop 15 min (`backoff_ms`); `next_at` je tentýž výpočet jako termín (`mail::next_deadline`), takže emit vlákno se neprobouzí dřív, než by se zdroj ptal.
- `Fails` (čítač neúspěchů za sebou, ve stavu zdroje: `WeatherState`, `FeedState`, mail/calsync `St`, `limitsfetch::State`) platí jen v jedné epoše. Epocha roste (`netstate::epoch`) při probuzení / skoku hodin (`note_tick(now)` z emit smyčky: mezera mezi průchody > 3 min nebo čas zpět) a při prvním úspěchu po neúspěchu kteréhokoli zdroje (`Fails::ok` = síť je zpět). Starší epocha = 0 neúspěchů → všechny zdroje se vrátí k základnímu odstupu. Žádný nový časovač ani polling.
- Skok hodin zpět: čas pokusu / stažení z budoucnosti (> 5 s) je neznámý, ne „ještě ne“ (`netstate::past`, `limitsrc::due`).
- Limity: `Outcome::Offline` (connect / timeout při `send`, timeout těla) = 2 → 4 → 8 → 15 min místo slepých 15 min (`Outcome::next_after_ms(every, fails)`); karta dál ukazuje `fetch: "error"` + stáří dat.
- Nenapojeno: čtení `Retry-After` z odpovědí (parametr `retry_after` v `next_attempt` je a je testovaný, volající ho zatím předávají `None`).
