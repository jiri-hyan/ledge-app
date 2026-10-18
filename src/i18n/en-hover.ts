// Angličtina pro náhled po najetí (`src/hover/*`: režimy `drawer.hover`, kompaktní náhled, ostrůvek;
// klíče `cards.hover.*`). Názvy režimů musí sedět s `en` v `hover/modes.ts` (hlídá `hover/model.test.mjs`).
import { registerDict } from "../i18n.ts";

registerDict("en", {
  "cards.hover.mode.full": "Full drawer right away",
  "cards.hover.mode.full.desc": "Today's behaviour: hovering opens the whole drawer.",
  "cards.hover.mode.compact": "Compact preview",
  "cards.hover.mode.compact.desc": "A small panel with what matters: approvals, Claude sessions, music, time and limit. Click to open the whole drawer.",
  "cards.hover.mode.island": "Expanded island",
  "cards.hover.mode.island.desc": "A capsule expands under the strip and shows just what is happening right now, with buttons. The least distracting.",
  "cards.hover.limitHint": "Claude's 5-hour limit",
  "cards.hover.limit5h": "5h {pct} %",
  "cards.hover.unknownTrack": "Unknown track",
  "cards.hover.calm": "Nothing going on.",
  "cards.hover.more": "▾ whole drawer",
  "cards.hover.island.plan": "wants a plan approved",
  "cards.hover.island.showPlan": "Show plan",
  "cards.hover.island.asks": "Claude is asking",
  "cards.hover.island.answer": "Answer",
  "cards.hover.island.done": "Done: {name}",
  "cards.hover.island.open": "Open",
  "cards.hover.island.stopwatch": "Stopwatch {time}",
  "cards.hover.island.left": "{time} left",
  "cards.hover.island.paused": "Paused {time}",
  "cards.hover.island.working": "Claude is working: {name}",
  "cards.hover.island.moreWorking.one": "+{n} more",
  "cards.hover.island.moreWorking.other": "+{n} more",
});
