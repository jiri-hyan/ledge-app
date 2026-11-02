// Stav hlavičky šuplíku (kolo 27) bez UI: uložený vzhled z `appearance.head` / `appearance.tabs`, rozpracovaná
// úprava (živý náhled přímo v hlavičce) a uložení. Panel `HeadLook.tsx` se načítá líně jen v režimu ✎.
import { createSignal } from "solid-js";
import { api } from "../api";
import type { HeadLook, TabsLook } from "../types.ts";
import { savedAppearance } from "../theme/apply";
import { sayError } from "../ui/toast";
import { isDefaultHead, isDefaultTabs, normalizeHead, normalizeTabs } from "./headModel";

export type HeadDraft = { head: HeadLook; tabs: TabsLook };
export const [headDraft, setHeadDraft] = createSignal<HeadDraft | null>(null);
let initial: HeadDraft | null = null;

const saved = (): HeadDraft => ({ head: normalizeHead(savedAppearance().head), tabs: normalizeTabs(savedAppearance().tabs) });
/** Hlavička, kterou se má kreslit: rozpracovaná, jinak uložená. */
export const headLook = (): HeadLook => headDraft()?.head ?? saved().head;
export const tabsLook = (): TabsLook => headDraft()?.tabs ?? saved().tabs;

export function openHeadLook() {
  if (headDraft()) return;
  initial = saved();
  setHeadDraft(initial);
}
export const revertHeadLook = () => initial && setHeadDraft(initial);

/** Zavřít panel; `save` zapíše jen, když se něco změnilo (výchozí vzhled = bez klíčů v configu). */
export async function closeHeadLook(save: boolean) {
  const d = headDraft();
  const was = initial;
  setHeadDraft(null);
  initial = null;
  if (!save || !d || !was || JSON.stringify(d) === JSON.stringify(was)) return;
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    const appearance = { ...(cfg.appearance ?? {}) } as Record<string, unknown>;
    if (isDefaultHead(d.head)) delete appearance.head;
    else appearance.head = normalizeHead(d.head);
    if (isDefaultTabs(d.tabs)) delete appearance.tabs;
    else appearance.tabs = normalizeTabs(d.tabs);
    await api.saveConfig({ ...cfg, appearance });
  } catch (e) {
    sayError(e);
  }
}
