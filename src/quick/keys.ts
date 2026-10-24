// Klávesnice v otevřeném šuplíku (kolo 20, `smart.quickKeys`): Alt+1…9 spustí N-tého zástupce na záložce,
// šipky mezi zástupci přesouvají fokus, Enter spustí (nativně, tlačítko). ČISTÁ logika — `keyPlan` rozhodne
// z popisu události a stavu, `neighbor` najde souseda podle rozmístění; DOM a události řeší `quick.ts`.
// Pravidla: jen s volbou zapnutou, jen na hlavní stránce, ne v úpravách šuplíku, ne s paletou, ne v polích
// pro psaní. Nikdy nic globálního — událost přijde jen oknu šuplíku, které má fokus od uživatele.

export type Rect = { x: number; y: number; w: number; h: number };

export type QKey = { key: string; altKey: boolean; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean };

export type QCtx = {
  enabled: boolean;
  /** stránka šuplíku (`main` = karty záložky) */
  page: string;
  editing: boolean;
  paletteOpen: boolean;
  /** cíl události je pole pro psaní (input, textarea, select, contenteditable) */
  editable: boolean;
  /** cíl události je `body` (nic konkrétního nemá fokus) */
  onBody: boolean;
  /** zástupci na záložce v pořadí na obrazovce */
  rects: Rect[];
  /** index zástupce, který má fokus; -1 = žádný */
  focused: number;
};

export type QPlan = { do: "launch"; index: number } | { do: "focus"; index: number } | null;

/** Nejvýš 9 zástupců dostane číslo (Alt+1…9). */
export const MAX_NUMBERED = 9;

const ARROWS = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] as const;
type Arrow = (typeof ARROWS)[number];

const cx = (r: Rect) => r.x + r.w / 2;
const cy = (r: Rect) => r.y + r.h / 2;

/** Soused v dané šipce. Vlevo / vpravo = předchozí / další v pořadí na obrazovce (bez zalamování), nahoru /
 *  dolů = nejbližší řada nad / pod a v ní nejbližší sloupec. `-1` = není kam (zůstane se). */
export function neighbor(rects: Rect[], from: number, dir: Arrow): number {
  if (from < 0 || from >= rects.length) return -1;
  if (dir === "ArrowLeft") return from > 0 ? from - 1 : -1;
  if (dir === "ArrowRight") return from < rects.length - 1 ? from + 1 : -1;
  const cur = rects[from];
  const sign = dir === "ArrowDown" ? 1 : -1;
  const tol = Math.max(cur.h, 1) / 2;
  const cand = rects
    .map((r, i) => ({ i, dy: (cy(r) - cy(cur)) * sign, dx: Math.abs(cx(r) - cx(cur)) }))
    .filter((c) => c.i !== from && c.dy > tol);
  if (!cand.length) return -1;
  const nearest = Math.min(...cand.map((c) => c.dy));
  const row = cand.filter((c) => c.dy - nearest <= tol);
  row.sort((a, b) => a.dx - b.dx || a.i - b.i);
  return row[0].i;
}

/** Co s klávesou. `null` = nechat události její obvyklý osud. */
export function keyPlan(k: QKey, c: QCtx): QPlan {
  if (!c.enabled || c.page !== "main" || c.editing || c.paletteOpen || c.editable || c.rects.length === 0) return null;
  if (k.ctrlKey || k.metaKey) return null;
  // Alt+1…9
  if (k.altKey && !k.shiftKey) {
    const n = /^[1-9]$/.test(k.key) ? Number(k.key) : 0;
    return n >= 1 && n <= MAX_NUMBERED && n <= c.rects.length ? { do: "launch", index: n - 1 } : null;
  }
  if (k.altKey || k.shiftKey) return null;
  // šipky a Home / End: jen když fokus nikde jinde není (body) nebo je na zástupci
  if (c.focused < 0 && !c.onBody) return null;
  if (k.key === "Home" && c.focused >= 0) return { do: "focus", index: 0 };
  if (k.key === "End" && c.focused >= 0) return { do: "focus", index: c.rects.length - 1 };
  if (!(ARROWS as readonly string[]).includes(k.key)) return null;
  if (c.focused < 0) return { do: "focus", index: 0 };
  const to = neighbor(c.rects, c.focused, k.key as Arrow);
  return to < 0 ? null : { do: "focus", index: to };
}

/** Čísla k zobrazení při držení Alt: index zástupce → číslo (1–9), víc než devíti se číslo nedává. */
export function badges(count: number): number[] {
  return Array.from({ length: Math.min(count, MAX_NUMBERED) }, (_, i) => i + 1);
}
