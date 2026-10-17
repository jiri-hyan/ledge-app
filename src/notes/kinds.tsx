// Kategorie poznámek (`kind:`): vestavěné inline ikony (jména jako `config::NOTE_KIND_ICONS`),
// dohledání kategorie podle id (neznámé z druhého PC = šedý text) a výběr pilulkami.

import { For, Show } from "solid-js";
import type { NoteKind } from "../api";
import { t } from "../i18n";

export const DEFAULT_KIND = "info";

const st = { fill: "none", stroke: "currentColor", "stroke-width": 1.7, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;
const PATHS: Record<string, string> = {
  bulb: "M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V17h5v-1.1c0-.8.4-1.5 1-2A6 6 0 0 0 12 3",
  chat: "M4 5h16v11H9l-5 4z",
  checklist: "M4 6l1.5 1.5L8 5M4 12l1.5 1.5L8 11M4 18l1.5 1.5L8 17M11 6h9M11 12h9M11 18h9",
  note: "M6 3h9l4 4v14H6zM14 3v5h5M9 13h7M9 17h5",
  flag: "M5 21V4M5 4h11l-2 4 2 4H5",
  star: "M12 3l2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9z",
  bug: "M8 8a4 4 0 0 1 8 0v7a4 4 0 0 1-8 0zM4 13h4M16 13h4M5 7l3 2M19 7l-3 2M5 19l3-2M19 19l-3-2",
  heart: "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10",
};

export function KindIcon(p: { icon: string }) {
  return (
    <Show when={PATHS[p.icon]} fallback={<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" fill="currentColor" /></svg>}>
      {(d) => <svg viewBox="0 0 24 24" {...st}><path d={d()} /></svg>}
    </Show>
  );
}

const GRAY = "#94a3b8";

/** Kategorie podle id; neznámá (z druhého PC / smazaná z configu) = id jako text, šedá. */
export function kindOf(kinds: NoteKind[], id: string): NoteKind {
  return kinds.find((k) => k.id === id) ?? (id === DEFAULT_KIND ? { id, label: t("notes.kind.default", "Poznámka"), color: GRAY, icon: "note" } : { id, label: id, color: GRAY, icon: "" });
}

export const kindColor = (k: NoteKind) => k.color || GRAY;

/** Malá barevná ikona kategorie (řádek karty). */
export function KindBadge(p: { kinds: NoteKind[]; id: string }) {
  const k = () => kindOf(p.kinds, p.id);
  return <span class="kind-badge" style={{ color: kindColor(k()) }} title={k().label}><KindIcon icon={k().icon} /></span>;
}

/** Výběr kategorie: řada pilulek s ikonou (neznámé id z poznámky se ukáže jako další pilulka). */
export function KindPicker(p: { kinds: NoteKind[]; value: string; onChange: (id: string) => void }) {
  const list = () => (p.kinds.some((k) => k.id === p.value) ? p.kinds : [...p.kinds, kindOf(p.kinds, p.value)]);
  return (
    <div class="kind-pills" role="radiogroup" aria-label={t("notes.kind.label", "Kategorie")}>
      <For each={list()}>
        {(k) => (
          <button
            class="kind-pill"
            classList={{ on: k.id === p.value }}
            role="radio"
            aria-checked={k.id === p.value}
            style={{ "--k": kindColor(k) }}
            onClick={() => p.onChange(k.id)}
          >
            <KindIcon icon={k.icon} />
            {k.label}
          </button>
        )}
      </For>
    </div>
  );
}
