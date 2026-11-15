// Angličtina: widgetkit (`*.kit.json`, `src/widgetkit/*`) — poznámky resolveru a hlášky kliků.
import { registerDict } from "../i18n";

registerDict("en", {
  "cards.widgetkit.act.copied": "Copied to the clipboard.",
  "cards.widgetkit.act.missing": "Action \"{id}\" is not available (deleted or renamed).",
  "cards.widgetkit.act.unavailable": "This window cannot do that action — open the drawer.",
  "cards.widgetkit.act.open": "Widgetkit cannot open paths yet — use a button with an action.",
  "cards.widgetkit.hint.action": "Run the action",
  "cards.widgetkit.hint.drawer": "Open the drawer",
  "cards.widgetkit.note.broken": "broken source",
  "cards.widgetkit.note.btn": "Button {n} skipped: {why}",
  "cards.widgetkit.note.empty": "The row has neither text nor a bar.",
  "cards.widgetkit.note.format": "Unknown format \"{f}\" ({src}).",
  "cards.widgetkit.note.noAct": "The button has no valid action.",
  "cards.widgetkit.note.row": "Row {n} skipped: {why}",
  "cards.widgetkit.note.source": "Unknown source \"{src}\".",
  "cards.widgetkit.note.syntax": "Bad template \"{text}\" (missing or extra brace).",
});
