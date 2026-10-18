// Co UI ví o widgetech mimo snapshot: názvy pro katalog, Nastavení a „karta odebrána“ (`layout/text.ts`)
// a poslední seznam ze složky (`widgets_list`). Plní ho snapshot (umístěné widgety) i seznam — žádné I/O.
import { createSignal } from "solid-js";
import type { WidgetsListing, WidgetView } from "../types";
import { widgetsApi } from "./api";

const [known, setKnown] = createSignal<Record<string, Pick<WidgetView, "title" | "icon" | "description">>>({});
const [listing, setListing] = createSignal<WidgetsListing | null>(null);
export { listing as widgetsListing };

/** Zapamatuje názvy widgetů (snapshot i seznam); beze změny nic nepřepíše. */
export function rememberWidgets(list: readonly WidgetView[] | undefined) {
  if (!list?.length) return;
  const cur = known();
  if (list.every((w) => cur[w.id]?.title === w.title && cur[w.id]?.icon === w.icon && cur[w.id]?.description === w.description)) return;
  const next = { ...cur };
  for (const w of list) next[w.id] = { title: w.title, icon: w.icon, description: w.description };
  setKnown(next);
}

export const widgetInfo = (id: string) => known()[id];

/** Sken složky widgetů (otevření katalogu, Nastavení, proužku v Nastavení). */
export async function loadWidgets(): Promise<WidgetsListing | null> {
  const l = (await widgetsApi.list()) ?? null;
  setListing(l);
  rememberWidgets(l?.widgets);
  return l;
}
