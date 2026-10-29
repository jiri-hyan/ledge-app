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

## Testy

`limits::tests` (čas, oba formáty vstupu, čerstvější, přepis, stav, proužek, připomínka),
`limitsrc::tests` (API odpověď, round-trip cache, historie aplikace v1/v2, doplnění resetu, cesty, token, odstup),
`limitsfetch::tests` (pevná URL), `statusline::tests` (`record` nad tempdir, kandidáti Git Bash, shell, průchod bajtů + termín přes Git Bash —
přeskočí bez Git Bash), `statuslinesetup::tests` (příkaz pro oba shelly, merge/unmerge přesně, chybějící /
cizí / neobjektová hodnota, rozbité vstupy, soubory nad tempdir). UI: `src/limits/model.test.mjs`, `src/limits/sources.test.mjs`, `src/layout/hero.test.mjs` (důvody).
Žádný test nesahá na skutečné `~/.claude` ani `%LOCALAPPDATA%\Ledge`.
