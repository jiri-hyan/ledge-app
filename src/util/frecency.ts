// Lokální frekvence a nedávnost (frecency) pro řazení zástupců projektů a rychlých akcí.
// Čisté skórování s poločasem rozpadu 72 h (3 dny) + reaktivní SolidJS signály uložené v localStorage.
import { createSignal } from "solid-js";

export const FRECENCY_KEY = "ledge.frecency.v1";
export const FRECENCY_SORT_KEY = "ledge.frecency.sort.v1";

export type FrecencyEntry = { count: number; lastMs: number };
export type FrecencyMap = Record<string, FrecencyEntry>;

function getStorageItem(key: string): string | null {
  try {
    if (typeof localStorage !== "undefined" && localStorage && typeof localStorage.getItem === "function") {
      return localStorage.getItem(key);
    }
  } catch {
    // ignorovat chyby storage
  }
  return null;
}

function setStorageItem(key: string, value: string): void {
  try {
    if (typeof localStorage !== "undefined" && localStorage && typeof localStorage.setItem === "function") {
      localStorage.setItem(key, value);
    }
  } catch {
    // ignorovat chyby storage
  }
}

function loadMap(): FrecencyMap {
  const raw = getStorageItem(FRECENCY_KEY);
  if (!raw) return {};
  try {
    const val = JSON.parse(raw);
    if (val && typeof val === "object" && !Array.isArray(val)) {
      return val as FrecencyMap;
    }
  } catch {
    // ignorovat chyby parsování
  }
  return {};
}

function loadSort(): Record<string, boolean> {
  const raw = getStorageItem(FRECENCY_SORT_KEY);
  if (!raw) return {};
  try {
    const val = JSON.parse(raw);
    if (val && typeof val === "object" && !Array.isArray(val)) {
      return val as Record<string, boolean>;
    }
  } catch {
    // ignorovat chyby parsování
  }
  return {};
}

/**
 * Spočte frecency skóre pro záznam:
 * Základ je počet spuštění `count`, násobený exponenciálním rozpadem s poločasem 72 h.
 * Čerstvě spuštěná položka (0 h) má násobič 3× (1 + 2 * 1), po 72 h má 2× (1 + 2 * 0.5), v nekonečnu 1×.
 */
export function scoreOf(entry: FrecencyEntry | undefined, now: number): number {
  if (!entry || entry.count <= 0) return 0;
  const hours = Math.max(0, (now - entry.lastMs) / 3_600_000);
  const decay = Math.pow(0.5, hours / 72);
  return entry.count * (1 + 2 * decay);
}

/**
 * Zaznamená jedno použití položky do mapy (čistá funkce).
 * Omezuje maximální počet záznamů na 200 (drží 200 nejnovějších podle lastMs).
 */
export function recordEntry(map: FrecencyMap, key: string, now: number): FrecencyMap {
  const prev = map[key];
  const next: FrecencyMap = {
    ...map,
    [key]: { count: (prev?.count ?? 0) + 1, lastMs: now },
  };
  const entries = Object.entries(next);
  if (entries.length > 200) {
    entries.sort((a, b) => b[1].lastMs - a[1].lastMs);
    return Object.fromEntries(entries.slice(0, 200));
  }
  return next;
}

/**
 * Stabilní řazení podle frecency skóre:
 * - Položky se scoreOf > 0 jdou na začátek, sestupně podle skóre.
 * - Položky se scoreOf === 0 si zachovávají přesné původní relativní pořadí na konci.
 * - Položky se stejným skóre > 0 si také zachovávají původní relativní pořadí.
 */
export function sortByFrecency<T>(
  items: readonly T[],
  keyOf: (item: T) => string,
  map: FrecencyMap,
  now: number,
): T[] {
  const annotated = items.map((item, index) => ({
    item,
    index,
    score: scoreOf(map[keyOf(item)], now),
  }));

  annotated.sort((a, b) => {
    const aHas = a.score > 0;
    const bHas = b.score > 0;
    if (aHas && bHas) {
      if (b.score !== a.score) {
        return b.score - a.score;
      }
      return a.index - b.index;
    }
    if (aHas) return -1;
    if (bHas) return 1;
    return a.index - b.index;
  });

  return annotated.map((x) => x.item);
}

// Reaktivní signály
const [map, setMap] = createSignal<FrecencyMap>(loadMap());
const [sortMap, setSortMap] = createSignal<Record<string, boolean>>(loadSort());

export function frecencyMap(): FrecencyMap {
  return map();
}

export function recordUse(key: string, now = Date.now()): void {
  setMap((prev) => {
    const next = recordEntry(prev, key, now);
    setStorageItem(FRECENCY_KEY, JSON.stringify(next));
    return next;
  });
}

export function isFrecencySort(scope: "proj" | "act"): boolean {
  return !!sortMap()[scope];
}

export function toggleFrecencySort(scope: "proj" | "act"): boolean {
  const nextVal = !sortMap()[scope];
  setSortMap((prev) => {
    const next = { ...prev, [scope]: nextVal };
    setStorageItem(FRECENCY_SORT_KEY, JSON.stringify(next));
    return next;
  });
  return nextVal;
}
