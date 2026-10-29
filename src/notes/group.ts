// Seskupení seznamu poznámek v kartě (volba vzhledu karty „Seskupit podle“): nic / projekt / kategorie /
// datum (Dnes · Tento týden · Starší) / připnuté nahoře. Čistá funkce bez DOM a bez `t()` — popisky skupin
// dává volající (`ctx`), ať je test tabulkový. Pořadí poznámek uvnitř skupiny zůstává, jak přišlo.
import type { NoteView } from "../types.ts";

export const GROUP_BY = ["none", "project", "kind", "date", "pinned"] as const;
export type GroupBy = (typeof GROUP_BY)[number];
/** Styl oddělení: čára s nadpisem · štítek · samostatné karty. */
export const GROUP_STYLES = ["line", "tag", "cards"] as const;
export type GroupStyle = (typeof GROUP_STYLES)[number];

export type Group = { key: string; label: string; items: NoteView[] };
export type GroupCtx = {
  now: number;
  /** projekt → název (null = bez projektu) */
  project: (id: string | null) => string;
  /** kategorie → název */
  kind: (id: string) => string;
  pinned: (n: NoteView) => boolean;
  /** popisky časových skupin a „Připnuté“ / „Ostatní“ */
  text: { today: string; week: string; older: string; pinned: string; rest: string };
};

/** Obranné čtení volby z configu (neznámé = bez seskupení / čára). */
export const groupByOf = (x: unknown): GroupBy => ((GROUP_BY as readonly unknown[]).includes(x) ? (x as GroupBy) : "none");
export const groupStyleOf = (x: unknown): GroupStyle => ((GROUP_STYLES as readonly unknown[]).includes(x) ? (x as GroupStyle) : "line");

const dayStart = (ms: number) => {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
};
/** Pondělí 00:00 týdne, do kterého `ms` patří. */
const weekStart = (ms: number) => {
  const d = new Date(dayStart(ms));
  d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
  return d.getTime();
};

/** Do které časové skupiny patří vytvoření `ms`: dnes (i budoucí) · tento týden (od pondělí) · starší. */
export function dateBucket(ms: number, now: number): "today" | "week" | "older" {
  if (ms >= dayStart(now)) return "today";
  return ms >= weekStart(now) ? "week" : "older";
}

/** Rozdělí poznámky do skupin; prázdné skupiny nevznikají. `none` = jedna skupina bez názvu. */
export function groupNotes(items: readonly NoteView[], by: GroupBy, ctx: GroupCtx): Group[] {
  if (by === "none") return items.length ? [{ key: "all", label: "", items: [...items] }] : [];
  const out = new Map<string, Group>();
  const put = (key: string, label: string, n: NoteView) => {
    const g = out.get(key);
    if (g) g.items.push(n);
    else out.set(key, { key, label, items: [n] });
  };
  for (const n of items) {
    if (by === "project") put(`p:${n.project ?? ""}`, ctx.project(n.project), n);
    else if (by === "kind") put(`k:${n.kind}`, ctx.kind(n.kind), n);
    else if (by === "pinned") put(ctx.pinned(n) ? "pin" : "rest", ctx.pinned(n) ? ctx.text.pinned : ctx.text.rest, n);
    else {
      const b = dateBucket(n.createdMs, ctx.now);
      put(b, ctx.text[b], n);
    }
  }
  const all = [...out.values()];
  const order = by === "date" ? ["today", "week", "older"] : by === "pinned" ? ["pin", "rest"] : null;
  return order ? all.sort((a, b) => order.indexOf(a.key) - order.indexOf(b.key)) : all;
}

/** Názvy voleb pro panel Vzhled karty: páry [klíč, čeština] (angličtina `i18n/en-notes.ts`). */
export const GROUP_BY_NAMES: Record<GroupBy, [string, string]> = {
  none: ["notes.group.by.none", "Nic"],
  project: ["notes.group.by.project", "Projekt"],
  kind: ["notes.group.by.kind", "Kategorie"],
  date: ["notes.group.by.date", "Datum (Dnes · Tento týden · Starší)"],
  pinned: ["notes.group.by.pinned", "Připnuté nahoře"],
};
export const GROUP_STYLE_NAMES: Record<GroupStyle, [string, string]> = {
  line: ["notes.group.style.line", "Čára s nadpisem"],
  tag: ["notes.group.style.tag", "Štítek"],
  cards: ["notes.group.style.cards", "Samostatné karty"],
};
