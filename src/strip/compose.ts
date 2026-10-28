// Vlastní zavřený Ledge (`strip.look = "custom"`, `strip.compose`, Rust `looks/compose.rs`, kolo 23):
// seznamy tvarů, položek a stylů, rozsahy, ořez (`normCompose` = Rust `Compose::norm`), předvolby
// „začít z hotového vzhledu“ (`composeOf`, čistá funkce look → compose) a úpravy kostek (čisté, vstup
// netknutý). Id a rozsahy hlídá Rust test `compose_tests::ts_mirror` — zápis `X = [...]` drž na jednom řádku.
// Bez importů za běhu (`compose.test.mjs`).

export const COMPOSE_SHAPES = ["capsule", "notch", "line", "corner", "drop", "island"] as const;
export type ComposeShape = (typeof COMPOSE_SHAPES)[number];
/** Jen položky se skutečným zdrojem v Rustu (`looks::Facts`). */
export const COMPOSE_ITEMS = ["time", "date", "claude", "limit5h", "limitWeek", "music", "timer", "cpu", "memory", "battery", "weather", "servers", "repos", "activity"] as const;
export type ComposeItem = (typeof COMPOSE_ITEMS)[number];
export const ITEM_STYLES = ["icon", "badge", "text", "ring", "graph", "dot"] as const;
export type ItemStyle = (typeof ITEM_STYLES)[number];
export type Slot = "left" | "center" | "right";
export const SLOTS: readonly Slot[] = ["left", "center", "right"];

export type Piece = { item: ComposeItem; style: ItemStyle };
export type StripCompose = {
  shape: ComposeShape;
  width: number;
  height: number;
  /** zaoblení 0–100 % */
  round: number;
  left: Piece[];
  center: Piece[];
  right: Piece[];
  hideIdle: boolean;
  rotate: boolean;
};

export const MAX_PIECES = 5;
export const W_MIN = 60;
export const W_MAX = 620;
export const H_MIN = 18;
export const H_MAX = 52;
export const LINE_H_MIN = 2;
export const LINE_H_MAX = 10;

export const hRange = (shape: ComposeShape): [number, number] => (shape === "line" ? [LINE_H_MIN, LINE_H_MAX] : [H_MIN, H_MAX]);
const clamp = (v: unknown, a: number, b: number, d: number) => {
  const n = typeof v === "number" && Number.isFinite(v) ? Math.round(v) : d;
  return Math.min(b, Math.max(a, n));
};

/** „claude:icon“ → položka (styl chybí = ikona). */
function mkPiece(s: string): Piece {
  const [item, style] = s.split(":");
  return { item: item as ComposeItem, style: (style ?? "icon") as ItemStyle };
}
function mk(shape: ComposeShape, width: number, height: number, round: number, left: string[], center: string[], right: string[], extra: Partial<StripCompose> = {}): StripCompose {
  return { shape, width, height, round, left: left.map(mkPiece), center: center.map(mkPiece), right: right.map(mkPiece), hideIdle: false, rotate: false, ...extra };
}

/** Ekvivalent každého z 15 hotových vzhledů (jen položky se skutečným zdrojem; mikrofon, pošta apod. ne). */
const PRESETS: Record<string, StripCompose> = {
  classic: mk("capsule", 300, 30, 55, ["claude:icon"], ["time:text"], ["limit5h:ring"]),
  island: mk("island", 150, 30, 100, [], ["claude:text"], []),
  dots: mk("capsule", 84, 18, 100, [], ["claude:dot", "servers:dot", "repos:dot", "music:dot"], []),
  line: mk("line", 620, 3, 0, ["claude:dot"], ["limit5h:dot"], ["music:dot"]),
  icons: mk("capsule", 190, 30, 55, [], ["claude:badge", "servers:badge", "repos:badge", "music:badge"], []),
  clock: mk("capsule", 150, 34, 50, [], ["time:text"], ["claude:dot"]),
  corner: mk("corner", 150, 28, 60, ["claude:icon"], [], ["time:text"]),
  ghost: mk("island", 220, 30, 100, [], ["claude:text"], [], { hideIdle: true }),
  bar: mk("capsule", 620, 24, 0, ["claude:text"], ["time:text"], ["limit5h:text"]),
  notch: mk("notch", 240, 32, 70, ["claude:icon"], [], ["time:text"]),
  progress: mk("capsule", 210, 40, 40, [], ["limit5h:graph"], []),
  ticker: mk("island", 230, 28, 100, [], ["claude:text", "limit5h:text", "music:text", "timer:text"], [], { rotate: true }),
  minimum: mk("capsule", 112, 26, 100, ["battery:icon"], [], ["time:text"]),
  trio: mk("capsule", 280, 32, 60, ["weather:text"], ["time:text"], ["cpu:text"]),
  rings: mk("capsule", 250, 36, 60, ["limit5h:ring"], ["time:text"], ["limitWeek:ring"]),
};

const clone = (c: StripCompose): StripCompose => ({ ...c, left: c.left.map((p) => ({ ...p })), center: c.center.map((p) => ({ ...p })), right: c.right.map((p) => ({ ...p })) });

/** Výchozí compose (= Rust `Compose::default`, předvolba Klasický). */
export const DEFAULT_COMPOSE: StripCompose = PRESETS.classic;

/** Předvolba z hotového vzhledu; neznámý / `custom` = Klasický. Vždy nová kopie. */
export function composeOf(look: string): StripCompose {
  return clone(PRESETS[look] ?? PRESETS.classic);
}

/** Ořez jako Rust `Compose::norm`: rozsahy, každá položka jednou, 1–5 položek; neznámé hodnoty = výchozí. */
export function normCompose(raw: unknown): StripCompose {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const d = DEFAULT_COMPOSE;
  const shape = COMPOSE_SHAPES.includes(r.shape as ComposeShape) ? (r.shape as ComposeShape) : d.shape;
  const [h0, h1] = hRange(shape);
  const seen = new Set<string>();
  const take = (v: unknown): Piece[] =>
    (Array.isArray(v) ? v : []).flatMap((p) => {
      const item = (p as Piece)?.item;
      if (!COMPOSE_ITEMS.includes(item) || seen.has(item) || seen.size >= MAX_PIECES) return [];
      seen.add(item);
      const st = (p as Piece).style;
      return [{ item, style: ITEM_STYLES.includes(st) ? st : "icon" }];
    });
  const has = (k: string) => k in r;
  const left = has("left") ? take(r.left) : take(d.left);
  const center = has("center") ? take(r.center) : take(d.center);
  const right = has("right") ? take(r.right) : take(d.right);
  if (!left.length && !center.length && !right.length) center.push({ item: "claude", style: "icon" });
  return {
    shape,
    width: clamp(r.width, W_MIN, W_MAX, d.width),
    height: clamp(r.height, h0, h1, d.height),
    round: clamp(r.round, 0, 100, d.round),
    left,
    center,
    right,
    hideIdle: r.hideIdle === true,
    rotate: r.rotate === true,
  };
}

export const total = (c: StripCompose) => c.left.length + c.center.length + c.right.length;
export const hasItem = (c: StripCompose, item: ComposeItem) => SLOTS.some((s) => c[s].some((p) => p.item === item));
/** Kde položka je (slot, pořadí); `null` = není. */
export function whereIs(c: StripCompose, item: ComposeItem): [Slot, number] | null {
  for (const s of SLOTS) {
    const i = c[s].findIndex((p) => p.item === item);
    if (i >= 0) return [s, i];
  }
  return null;
}

/** Přidá / přesune položku do slotu na pozici `at` (konec, když chybí). Plno (5) = beze změny. */
export function putPiece(c: StripCompose, item: ComposeItem, slot: Slot, at?: number): StripCompose {
  const x = clone(c);
  const was = whereIs(x, item);
  let style: ItemStyle = "icon";
  if (was) {
    style = x[was[0]][was[1]].style;
    x[was[0]].splice(was[1], 1);
  } else if (total(x) >= MAX_PIECES) {
    return c;
  }
  const pos = at === undefined ? x[slot].length : Math.max(0, Math.min(at, x[slot].length));
  x[slot].splice(pos, 0, { item, style });
  return x;
}

/** Odebere položku; poslední (1) zůstane. */
export function removePiece(c: StripCompose, item: ComposeItem): StripCompose {
  if (total(c) <= 1 || !hasItem(c, item)) return c;
  const x = clone(c);
  for (const s of SLOTS) x[s] = x[s].filter((p) => p.item !== item);
  return x;
}

/** Posune položku o krok doleva / doprava (přes hranice slotů). */
export function shiftPiece(c: StripCompose, item: ComposeItem, dir: -1 | 1): StripCompose {
  const flat = SLOTS.flatMap((s) => c[s].map((p) => ({ s, p })));
  const i = flat.findIndex((e) => e.p.item === item);
  if (i < 0) return c;
  const cur = flat[i];
  const [s, idx] = whereIs(c, item)!;
  const si = SLOTS.indexOf(s);
  // uvnitř slotu prohodí se sousedem, na kraji přejde do vedlejšího slotu
  if ((dir < 0 && idx > 0) || (dir > 0 && idx < c[s].length - 1)) return putPiece(c, cur.p.item, s, idx + dir);
  const ns = SLOTS[si + dir];
  if (!ns) return c;
  return putPiece(c, cur.p.item, ns, dir > 0 ? 0 : c[ns].length);
}

/** Co se skutečně nakreslí: prstenec / graf bez podílu spadne na ikonu (jen Claude má prázdný prstenec; aktivita je odhad, podíl nemá).
 *  Zrcadlí Rust `compose::parts`. */
export type DrawKind = "icon" | "badge" | "text" | "dot" | "ring" | "meter";
export function drawKind(item: ComposeItem, style: ItemStyle, hasFrac: boolean): DrawKind {
  if (style === "ring") return hasFrac || item === "claude" ? "ring" : "icon";
  if (style === "graph") return hasFrac ? "meter" : "icon";
  return style;
}

/** Další styl položky (klik na kostku). */
export function nextStyle(s: ItemStyle): ItemStyle {
  return ITEM_STYLES[(ITEM_STYLES.indexOf(s) + 1) % ITEM_STYLES.length];
}
export function setStyle(c: StripCompose, item: ComposeItem, style: ItemStyle | "next"): StripCompose {
  const x = clone(c);
  for (const s of SLOTS) x[s] = x[s].map((p) => (p.item === item ? { ...p, style: style === "next" ? nextStyle(p.style) : style } : p));
  return x;
}
export function setAllStyles(c: StripCompose, style: ItemStyle): StripCompose {
  const x = clone(c);
  for (const s of SLOTS) x[s] = x[s].map((p) => ({ ...p, style }));
  return x;
}

/** Změna tvaru: výška se ořízne do rozsahu tvaru (linka 2–10). */
export function withShape(c: StripCompose, shape: ComposeShape): StripCompose {
  const [h0, h1] = hRange(shape);
  return { ...clone(c), shape, height: Math.min(h1, Math.max(h0, c.height)) };
}
