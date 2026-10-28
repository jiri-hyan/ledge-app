// Angličtina živé aktivity Clauda (kolo 23): karta „Claude aktivita“ (`sessions/ActivityCard.tsx`) a položka
// skládačky (`settings/panels/ComposeEditor.tsx`, náhled `settings/previews/ComposePreview.tsx`). Načítá `i18n/dicts.ts`.
import { registerDict } from "../i18n";

registerDict("en", {
  "cards.layout.card.activity": "Claude activity",
  "cards.layout.desc.activity": "What Claude is doing in each session right now: phase (estimated from the last tool), file, last message and approvals",
  "cards.activity.title": "Claude activity",
  "cards.activity.estimate": "estimated from the last tool",
  "cards.activity.noHooks": "Ledge has no hooks installed — activity will not show.",
  "cards.activity.inClaude": "Waiting for you — the approval is in the Claude card.",
  "cards.activity.empty": "Claude isn't doing anything right now.",
  "cards.activity.foot": "The phase is estimated from the last tool; file and message come from Claude Code hooks.",
  "cards.activity.phase.reading": "Reading",
  "cards.activity.phase.editing": "Editing",
  "cards.activity.phase.testing": "Tests",
  "cards.activity.phase.working": "Working",
  "cards.activity.phase.done": "Done",
  "cards.activity.phase.error": "Error",
  "appearance.compose.item.activity": "Claude activity",
  "appearance.compose.hint.activity": "The phase (reading, editing, tests, done) is estimated from the last tool; a pending approval comes first.",
  "appearance.compose.sampleActivity": "Editing · main.rs",
});
