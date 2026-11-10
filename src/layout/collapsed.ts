// Sbalené karty šuplíku na 1 řádek (hlavičku).
// Ukládá se do localStorage pod klíčem COLLAPSED_KEY.
import { createSignal } from "solid-js";

export const COLLAPSED_KEY = "ledge.collapsedCards.v1";

export const cardCollapseKey = (pageId: string, cardId: string): string => `${pageId}:${cardId}`;

export function toggleSet(list: readonly string[], key: string): string[] {
  return list.includes(key) ? list.filter((k) => k !== key) : [...list, key];
}

function loadCollapsed(): string[] {
  try {
    if (typeof localStorage === "undefined") return [];
    const raw = localStorage.getItem(COLLAPSED_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) && parsed.every((x) => typeof x === "string") ? parsed : [];
  } catch {
    return [];
  }
}

function saveCollapsed(list: readonly string[]): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify(list));
    }
  } catch {}
}

const [collapsedList, setCollapsedList] = createSignal<string[]>(loadCollapsed());

export const isCardCollapsed = (pageId: string, cardId: string): boolean =>
  collapsedList().includes(cardCollapseKey(pageId, cardId));

export function toggleCardCollapsed(pageId: string, cardId: string): boolean {
  const key = cardCollapseKey(pageId, cardId);
  const next = toggleSet(collapsedList(), key);
  setCollapsedList(next);
  saveCollapsed(next);
  return next.includes(key);
}

export function setCardCollapsed(pageId: string, cardId: string, collapsed: boolean): void {
  const key = cardCollapseKey(pageId, cardId);
  const cur = collapsedList();
  const has = cur.includes(key);
  if (collapsed === has) return;
  const next = collapsed ? [...cur, key] : cur.filter((k) => k !== key);
  setCollapsedList(next);
  saveCollapsed(next);
}
