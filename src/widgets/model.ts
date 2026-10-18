// Čistá logika widgetů ze skriptů pro UI (bez importů za běhu, test `model.test.mjs`): id karty /
// položky proužku `widget:<soubor>` (zrcadlo Rust `widgets::plan::file_of`), barvy z výstupu, interval,
// co dělá klik na položku, co nabídnout v katalogu karet a v proužku.
import type { WidgetItem, WidgetTone, WidgetView } from "../types";

export const WIDGET_PREFIX = "widget:";

/** Soubor z id: jen jméno (bez cesty a `:`), ne skrytý, nejvýš 200 znaků; jinak null. */
export function widgetFile(id: unknown): string | null {
  if (typeof id !== "string" || !id.startsWith(WIDGET_PREFIX)) return null;
  const f = id.slice(WIDGET_PREFIX.length);
  // eslint-disable-next-line no-control-regex
  const ok = f.length > 0 && f.length <= 200 && !/[\\/:\u0000-\u001f]/.test(f) && !f.startsWith(".");
  return ok ? f : null;
}
export const isWidgetId = (id: unknown): id is `widget:${string}` => widgetFile(id) !== null;

/** Předvolba barvy → CSS proměnná motivu (jako barvy proužku). */
export const TONE_VAR: Record<WidgetTone, string> = {
  accent: "var(--accent)",
  text: "var(--t0)",
  muted: "var(--t2)",
  violet: "var(--violet)",
  blue: "var(--blue)",
  teal: "var(--teal)",
  green: "var(--green)",
  amber: "var(--amber)",
  red: "var(--red)",
};
export const toneVar = (t: WidgetTone | null | undefined): string | undefined => (t ? TONE_VAR[t] : undefined);

/** Interval obnovy pro člověka: 90 → 90 s, 300 → 5 min, 3600 → 60 min, 5400 → 1,5 h (jednotka + číslo). */
export function intervalParts(s: number): { n: number; unit: "s" | "min" | "h" } {
  if (s < 120) return { n: s, unit: "s" };
  if (s < 5400) return { n: Math.round(s / 60), unit: "min" };
  return { n: Math.round((s / 3600) * 10) / 10, unit: "h" };
}

export type ItemKind = "url" | "open" | "copy" | "run" | null;
/** Co udělá klik na položku (null = nic, jen text). */
export const itemKind = (i: WidgetItem): ItemKind => (i.sep ? null : i.url ? "url" : (i.action?.type ?? null));

/** Průběh 0–1 → celá procenta (null = bez průběhu). */
export const progressPct = (p: number | null | undefined): number | null => (typeof p === "number" && Number.isFinite(p) ? Math.round(Math.min(1, Math.max(0, p)) * 100) : null);

/** Data starší než dva intervaly (karta ukáže „stará data“). */
export const isStale = (updatedMs: number | null, intervalS: number, now: number): boolean => updatedMs != null && intervalS > 0 && now - updatedMs > 2 * intervalS * 1000;

type Pages = { id: string; cards: { id: string }[] }[];

/** Widgety do katalogu karet pro záložku: ty, které jdou do karty a na téhle záložce nejsou, + kde jsou jinde. */
export function catalogWidgets(widgets: readonly WidgetView[], pages: Pages, pageId: string): { id: `widget:${string}`; title: string; desc: string | null; icon: string | null; on: string[] }[] {
  const page = pages.find((p) => p.id === pageId);
  return widgets
    .filter((w) => w.show !== "strip" && isWidgetId(w.id) && !page?.cards.some((c) => c.id === w.id))
    .map((w) => ({ id: w.id as `widget:${string}`, title: w.title, desc: w.description, icon: w.icon, on: pages.filter((p) => p.cards.some((c) => c.id === w.id)).map((p) => p.id) }));
}

/** Položky proužku z widgetů (jen ty, které do proužku smí), za vestavěnými; bez duplicit. */
export function stripWidgetIds(widgets: readonly WidgetView[], items: readonly string[]): string[] {
  const out = items.filter((id) => isWidgetId(id));
  for (const w of widgets) if (w.show !== "card" && isWidgetId(w.id) && !out.includes(w.id)) out.push(w.id);
  return out;
}

/** Položky proužku se zapnutým / vypnutým widgetem (vstup netknutý). */
export function withStripItem(items: readonly string[] | undefined, id: string, on: boolean): string[] {
  const rest = (items ?? []).filter((x) => x !== id);
  return on ? [...rest, id] : rest;
}
