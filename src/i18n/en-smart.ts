// Angličtina pro „chytřejší šuplík“ (kolo 20): nový zástupce z přetaženého (`actions/DropNew.tsx`, klíče
// `actions.drop.*`), export útrat a limitů (`exports/usage.ts`, `sessions.export.*`) a řádky Nastavení
// (`settings.smart.*`; popisky voleb `settings.f.*` jsou v `settings/en.ts`).
import { registerDict } from "../i18n";

registerDict("en", {
  "actions.drop.title": "New shortcut from what you dropped",
  "actions.drop.note": "Check it and confirm. Nothing is saved or launched until you press Add.",
  "actions.drop.skipped.one": "{n} more dropped item was skipped — only the first one is suggested.",
  "actions.drop.skipped.other": "{n} more dropped items were skipped — only the first one is suggested.",
  "actions.drop.host": "There is no card on this tab that would show the shortcut — add the “{name}” card.",
  "actions.drop.added": "Shortcut “{label}” added.",
  "sessions.export.csv": "Export usage and limits (CSV)…",
  "sessions.export.md": "Export usage and limits (Markdown)…",
  "sessions.export.saved": "Saved: {path}",
});
