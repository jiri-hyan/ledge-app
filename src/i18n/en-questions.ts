// Angličtina pro kartu otázky Clauda (`src/questions/*`) a výzvy v `ClaudeSessions.tsx` (klíče `questions.*`).
// Texty panelu `settings/panels/Questions.tsx` (`questions.panel.*`) jsou v `settings/panels/en.ts`.
// Labely voleb a „Jiné“ v odpovědi pro Claude Code jsou DATA otázky (Rust `claude/questions.rs`) — nepřekládat.
import { registerDict } from "../i18n";

registerDict("en", {
  "questions.addHook": "Add Claude's questions (answer them in Ledge)",
  "questions.sent": "Answer sent to Claude",
  "questions.released": "The question is in the Claude window — answer it there",
  "questions.asks": "Claude asks",
  "questions.asksIn": "Claude asks · {where}",
  "questions.left": "{left} more waiting",
  "questions.readOnly": "answer in the Claude window",
  "questions.multi": "Pick one or more options",
  "questions.other": "Other…",
  "questions.otherHint": "I'll write my own answer",
  "questions.otherPlaceholder": "Your answer… (Enter sends, Shift+Enter new line)",
  "questions.toClaude": "Answer in Claude",
  "questions.switch": "Switch to Claude",
  "questions.keys": "1–{n} pick · Enter next",
  "questions.back": "Back",
  "questions.next": "Next",
  "questions.send": "Send",
});
