// Angličtina pro kartu Claude (`src/ClaudeSessions.tsx`, klíče `sessions.*`).
import { registerDict } from "../i18n";

registerDict("en", {
  "sessions.inbox": "Claude is waiting for you",
  "sessions.status.waiting": "waiting for you",
  "sessions.status.working": "working",
  "sessions.status.idle": "idle",
  "sessions.status.ended": "finished",
  "sessions.count.one": "{n} session",
  "sessions.count.other": "{n} sessions",
  "sessions.collapseIdle": "Collapse idle",
  "sessions.moreIdle.one": "+{n} idle",
  "sessions.moreIdle.other": "+{n} idle",
  "sessions.empty": "No Claude Code sessions running.",
  "sessions.hooksMissing": "Missing: {list}",
  "sessions.connectHooks": "Connect hooks for approvals and subagents",
  "sessions.bridgeOff": "The Claude bridge is off in Settings — no approvals or subagents.",
  "sessions.ended": "ended",
  "sessions.agents.one": "{n} agent: {types}",
  "sessions.agents.other": "{n} agents: {types}",
  "sessions.lastAgent": "last: {type}",
  "sessions.uncommitted": "Uncommitted:\n{list}",
  "sessions.unpushed": "Session commits that aren't on any remote",
  "sessions.agentRunning": "running {time}",
  "sessions.deny": "Deny",
  "sessions.allow": "Allow",
});
