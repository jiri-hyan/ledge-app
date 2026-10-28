# ZBÝVÁ — aktivita (kolo 23), etapa B

## Hotovo (etapa A)
- Rust `claude/activity.rs`: fáze z `PostToolUse` (odhad), soubor z `tool_input`, poslední zpráva z `Stop.last_assistant_message`,
  jinak konec přepisu (64 kB, jen při `Stop`, jen s kartou `activity` na záložce). `Session.activity` → `sessions::Row.activity`.
- Položka skládačky `activity` (`looks/compose.rs` `activity_value`, `Facts.activity` z `stripview::look_facts` přes `activity::lead`).
- Karta `activity` v katalogu (výchozí nepřidaná): `src/sessions/ActivityCard.tsx`, schválení přes `PendingRow` jen bez karty Claude.
- Testy: `claude/activity_tests.rs`, `looks/activity_tests.rs`, `src/sessions/activity.test.mjs`; EN texty v `src/sessions/en-activity.ts`.

## Zbývá
1. Po sloučení s etapou B (`compose_tests.rs`): sjednotit `BASE_COMPOSE_ITEMS` + `COMPOSE_ITEMS` v `src/strip/compose.ts` do jednoho
   řádku a v `compose_tests::ts_mirror` přidat `PieceItem::Activity`; pak smazat `looks/activity_tests::ts_mirror_activity_item`.
2. `ComposePreview.tsx` (konfliktní soubor etapy B): ověřit, že po merge zůstal `activity` v `ITEM_GLYPH`, `case "activity"` a `activitySample`.
3. MCP (`mcp/look.rs`): `cards` už `activity` hlásí (bere `layout::CARDS`); popis položky `activity` v MCP nápovědě jen pokud se vejde
   do stropu `tools/list` (schéma se nerozšiřovalo).
4. Drobné testy navíc: `ActivityCard` bez karty Claude vs. s ní (`approvals`), `ClaudeInbox` se při kartě `activity` neukáže.
5. Volitelně: přesunout EN klíče `appearance.compose.*activity*` z `sessions/en-activity.ts` do `settings/previews/en.ts`.

## MAPA
`docs/codemap/sessions.md` (řádky activity.rs, ActivityCard) · `docs/codemap/appearance.md` (activity_tests.rs) ·
`docs/agents/config-layout.md` (karta `activity`, položka `activity`). Lepidlo: `lib.rs`/`settings.rs` (`set_activity_tail`),
`layout.rs` (`CARDS`, `ACTIVITY_CARD`), `src/layout/model.ts`, `src/MainPage.tsx`, `src/i18n/dicts.ts`, `ctxsize::read_tail` → `pub(crate)`.

## Neověřené (jen živě)
- Chyba: Ledge hookuje jen `PostToolUse`; nenulový exit Bash nejspíš přijde v `PostToolUseFailure` (nehookováno) a
  `tool_response` Bash pole `exit_code` možná nemá → fáze „Chyba“ se v praxi nemusí objevit. Přidání hooku = změna `hooksetup::EVENTS`
  (stav „hooky neúplné“ u stávajících uživatelů) → jen jako volba, rozhodne člověk.
- `last_assistant_message` v hooku `Stop` (u `SubagentStop` je); bez něj čte konec přepisu.
- Vzhled karty a položky v živém proužku (barvy, ořez dlouhého textu).
