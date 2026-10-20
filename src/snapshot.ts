// Prázdný snapshot do startu: tvar `Snapshot` z Rustu, dokud nepřijde `get_state` / událost `state`.
import type { Snapshot } from "./types";
import { allSectionsOn } from "./cards/sections";
import { defaultLayout } from "./layout/model";

export const EMPTY_SNAPSHOT: Snapshot = {
  mode: "prace",
  personalEnabled: false,
  view: "hidden",
  cfgError: null,
  sections: allSectionsOn(),
  layout: defaultLayout(),
  claudeEnabled: true,
  projects: [],
  procs: [],
  pending: [],
  questions: [],
  sessions: [],
  hooks: { state: "full", missing: [], error: null, path: "", hint: null },
  notesEnabled: true,
  paletteCompact: false,
  notes: [],
  notesMore: 0,
  reminders: [],
  remindersCount: 0,
  noteKinds: [],
  notesKindCounts: {},
  notesRev: 0,
  notesArchived: 0,
  personal: { weather: { status: "idle", label: "", data: null, fetchedMs: null, error: null, loading: false }, links: [], notes: [], notesMore: 0, media: null, volume: null, battery: null },
  notesSync: { state: "off", lastOkMs: null, message: null, files: [], root: null },
  update: { state: "off", current: "", repo: "", checking: false, checkedMs: null, installing: false, installError: null },
};
