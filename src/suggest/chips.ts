// Chipy návrhů jako reaktivní výběr (sdílí pruh `SuggestBar.tsx` a klidový ostrůvek `hover/Island.tsx`).
// `pick` se přepočítá při změně snapshotu / zavření chipu / schránky a na časových hranicích (`nextChange`),
// ne každou sekundu; stejný obsah = stejné pole (bez zbytečného překreslení).
import { createMemo, untrack } from "solid-js";
import type { Snapshot } from "../api";
import { dismissed } from "./actions";
import { currentMeetings, knownLinks, nextChange, pick, sameSuggestions, trackMeetings, type Meeting, type Suggestion } from "./model";

export function useChips(s: () => Snapshot, now: () => number, clip: () => { url: string; atMs: number } | null): () => Suggestion[] {
  /** schůzky s odkazem pamatované jen v paměti, i když je kalendář po konci skryl (poznámka ze schůzky) */
  let meetMem = new Map<string, Meeting>();
  /** příští hranice, kdy se výsledek mění sám časem (0 = hned) */
  let due = 0;
  const known = createMemo(() => knownLinks(s().actions, s().personal.links));
  // brána: sekundový tik `now` ji jen porovná; dál (a tím do `pick`) projde až na hranici
  const gate = createMemo<number>((prev) => (now() >= due || prev === undefined ? now() : prev), 0);
  return createMemo<Suggestion[]>(() => {
    gate();
    const on = s().suggest?.on;
    if (!on) return [];
    const at = untrack(now);
    const snap = s();
    meetMem = trackMeetings(meetMem, currentMeetings(snap.personal.calendar), at);
    const input = {
      on,
      tasks: snap.notesTaskCounts ?? { overdue: 0, today: 0 },
      notes: snap.notesEnabled,
      meetings: [...meetMem.values()],
      reminders: snap.reminders.map((n) => ({ path: n.path, title: n.title, remindMs: n.remindMs })),
      github: snap.suggest?.github ?? { review: [], failing: [], fetchedMs: null },
      procs: snap.procs,
      projects: snap.projects,
      clip: clip(),
      known: known(),
      claude: snap.claudeEnabled ? { sessions: snap.sessions, hiddenAt: snap.drawerHiddenAt ?? null } : undefined,
      briefs: snap.briefWaiting ?? null,
      // H5 ranní shrnutí: schůzky dnes bez celodenních a týdenní limit; rozhodnutí „je ráno“ dělá čistá `morning.ts`
      morning: {
        hiddenAt: snap.drawerHiddenAt ?? null,
        agenda: (snap.personal.calendar?.days.find((d) => d.kind === "today")?.items ?? []).filter((i) => !i.allDay).map((i) => ({ start: i.start, end: Math.max(i.end, i.start + 60_000) })),
        limits: snap.claudeEnabled ? snap.limits : null,
      },
    };
    const d = dismissed();
    due = nextChange(input, d, at) ?? Infinity;
    return pick(input, d, at);
  }, [], { equals: sameSuggestions });
}
