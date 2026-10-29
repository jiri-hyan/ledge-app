# ZBÝVÁ — limity Claude bez terminálu (kolo 25, `k25/limity`)

## Hotovo (etapa A)
- Průzkum (fakta): aplikace Claude (MSIX, `%LOCALAPPDATA%\Claude-Data\claude_<hash>\`) si limity sama ptá
  (`claude.ai/api/organizations/<org>/usage`, à 15 min, jen když to má zapnuté) a připisuje je do
  `plan-usage-history.json` (`{"version":2,"samples":[{"t":ms,"org":…,"u":{"fh":%,"sd":%}}]}`) — jen procenta, bez resetu.
  Plná odpověď drží jen v paměti. Claude Code `/usage` = `GET https://api.anthropic.com/api/oauth/usage`,
  `Authorization: Bearer <accessToken>`, `anthropic-beta: oauth-2025-04-20`; odpověď `five_hour`/`seven_day`
  `{utilization, resets_at ISO}`. `~/.claude.json` `cachedUsageUtilization` se píše jen po `/usage` (staré, nepoužito).
- `limitsrc.rs` (čisté + testy), `limitsfetch.rs` (síť, vzor `incidents.rs`), `limits.rs` (`Source::Desktop/Api`, `via`),
  `limitslive.rs` (třetí zdroj, `best` = nejčerstvější + `fill_resets`, `LimitsView.desktop/fetch`).
- Volby `claude.limitsDesktop`, `claude.limitsFetch` (výchozí vypnuto, test se starým configem, `demo-config.json`).
- UI: `src/limits/sources.ts` (+ test), `LimitsSources.tsx` v `LimitsRow` (důvod + zapnutí jedním klikem),
  `hero.ts` `limitsWhy` (nové důvody, test), Nastavení → Claude dva přepínače s větou, en texty, docs, codemapa
  (rozdělena na `limits.md` + `limits-detail.md`).

## Zbývá (etapa B)
- `src/cards/ClaudeDesign.tsx` `LimitsEmpty` (vzhledy karty): přidat `<LimitsSources l={lv()} />` pod důvod — tento
  worker na `src/cards/**` nesahal (paralelní worker); dnes tam jde jen nový text důvodu.
- Studio (`src/studio/parts.ts` / `labels.ts`): řádky pro `claude.limitsDesktop` / `claude.limitsFetch`, jestli tam
  limity mají být (dnes jsou jen v Nastavení → Claude).
- Volitelně: interval dotazu jako volba (`EVERY_MS` je pevně 10 min; text v UI ho uvádí).

## MAPA
`docs/codemap/limits.md` → `limitsrc.rs`, `limitsfetch.rs`, `limitslive.rs` (`best`, `desktop_path`, `reload`),
`src/limits/LimitsSources.tsx`, `src/layout/hero.ts` (`limitsWhy`). Lepidlo: `lib.rs` (mod), `threads/emit.rs`,
`threads/ui.rs`, `commands/drawer.rs` (`limitsfetch::maybe_refresh`), `settings.rs` (`limitsfetch::on_config`).

## Neověřené (živě)
- Skutečný dotaz na `/api/oauth/usage` (v testech žádná síť). Na tomhle PC má `~/.claude/.credentials.json`
  `expiresAt = 0` a mtime 15. 9. — buď dlouhodobý token, nebo staré přihlášení; Ledge to zkusí a při 401 ukáže
  „přihlášení vypršelo“. Aplikace Claude svému Claude Code předává přihlášení sama, tenhle soubor neobnovuje.
- Kadence `plan-usage-history.json`: do 25. 10. vzorek à 15 min, od té doby ~1–2× denně (aplikace nově podmiňuje
  dotaz otevřením panelu využití / tray v posledních N h) — čerstvost tedy závisí na aplikaci Claude.
