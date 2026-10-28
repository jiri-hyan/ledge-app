// Angličtina pro pruh kontextu session (`src/sessions/ContextBar.tsx`, klíče `sessions.ctx.*`) a podlahu nové relace
// v útratě (`src/costs/CostsSection.tsx`, klíče `sessions.costs.floor*`).
import { registerDict } from "../i18n";

registerDict("en", {
  "sessions.ctx.aria": "Session context: {n} tokens",
  "sessions.ctx.tip": "Context of the last call: {n} tokens (input + cache write + cache read). Marks: 250 k and 350 k.",
  "sessions.ctx.each": "every turn now reads ~{n} tokens",
  "sessions.ctx.ratio": "{r}× more than at the start",
  "sessions.ctx.advice": "A big context makes every turn pricier — consider a handoff and a fresh session",
  "sessions.costs.floorProjects": "Start of a new session (first call)",
  "sessions.costs.floorAgents": "Start of an agent by type (first call)",
  "sessions.costs.floorTip": "Median context of the first API call of each session / agent in the last 7 days; the trend compares it with the 7 days before (shown from 3 files in both). A bloated CLAUDE.md, MCP servers or skills show up here.",
  "sessions.costs.floorRowTip": "median {m} tokens in {n} files; previous 7 days: {p}",
  "sessions.costs.floorPrevNone": "no files",
  "sessions.costs.floorPrev": "median {m} in {n} files",
  "sessions.costs.floorEmpty": "No new session or agent in the last 7 days.",
  "sessions.costs.floorFlat": "no change",
  "sessions.costs.floorUp": "▲ +{v}",
  "sessions.costs.floorDown": "▼ −{v}",
  "sessions.costs.floorCount": "{n}×",
});
