// Vlastní zavřený Ledge (`strip.look = "custom"`, `strip.compose`, Rust `looks/compose.rs`, kolo 23):
// seznamy tvarů, položek a stylů, rozsahy, ořez (`normCompose` = Rust `Compose::norm`), předvolby
// „začít z hotového vzhledu“ (`composeOf`, čistá funkce look → compose) a úpravy kostek (čisté, vstup
// netknutý). Id a rozsahy hlídá Rust test `compose_tests::ts_mirror` — zápis `X = [...]` drž na jednom řádku.
// Bez importů za běhu (`compose.test.mjs`). Kolo 25: víc dílů (12, zóna 5), nové styly, barva dílu, výzdoba
// (`deco` akcent hrany, `fill` pozadí, `sep` oddělení; Rust `looks/composedeco.rs`) a 6 designových předvoleb.

export const COMPOSE_SHAPES = ["capsule", "notch", "line", "corner", "drop", "island"] as const;
export type ComposeShape = (typeof COMPOSE_SHAPES)[number];
/** Jen položky se skutečným zdrojem v Rustu (`looks::Facts`). */
export const COMPOSE_ITEMS = ["time", "date", "claude", "limit5h", "limitWeek", "music", "timer", "cpu", "memory", "gpu", "battery", "weather", "servers", "repos", "activity"] as const;
export type ComposeItem = (typeof COMPOSE_ITEMS)[number];
export const ITEM_STYLES = ["icon", "badge", "text", "ring", "graph", "dot", "iconText", "bar", "spark", "chip", "dotPct", "ring5h"] as const;
export type ItemStyle = (typeof ITEM_STYLES)[number];
// hrana a její živé efekty (kolo 28) žijí v `edgefx.ts`
import { DEFAULT_DECO, DEFAULT_LIVE, type ComposeDeco, normDeco } from "./edgefx.ts";
export { COMPOSE_EDGES, EDGE_LENS, DEFAULT_DECO, edgeColor, type ComposeEdge, type ComposeDeco } from "./edgefx.ts";
export const FILL_KINDS = ["black", "glass", "tint", "gradient", "none"] as const;
export type FillKind = (typeof FILL_KINDS)[number];
export const FILL_DIRS = ["right", "down", "diag"] as const;
export const COMPOSE_SEPS = ["none", "gap", "dot", "bar", "pills", "islands"] as const;
export type ComposeSep = (typeof COMPOSE_SEPS)[number];
export type Slot = "left" | "center" | "right";
export const SLOTS: readonly Slot[] = ["left", "center", "right"];

export const PIECE_SIZES = ["small", "normal", "large"] as const;
export type PieceSize = (typeof PIECE_SIZES)[number];
export const TEXT_WEIGHTS = ["normal", "semi", "bold"] as const;
export type TextWeight = (typeof TEXT_WEIGHTS)[number];
export const TEXT_SPACINGS = ["tight", "normal", "wide"] as const;
export type TextSpacing = (typeof TEXT_SPACINGS)[number];
/** Písmo proužku nezávislé na výšce (kolo 27): `size` násobek 0,6–1,4 (1 = dnešní), tloušťka, proložení (mezery mezi díly). */
export type ComposeText = { size: number; weight: TextWeight; spacing: TextSpacing };
export const TEXT_MIN = 0.6;
export const TEXT_MAX = 1.4;
export const DEFAULT_TEXT: ComposeText = { size: 1, weight: "normal", spacing: "normal" };
/** Násobky velikosti dílu a mezer (zrcadlo Rust `PieceSize::mult`, `TextLook::space`). */
export const SIZE_MULT: Record<PieceSize, number> = { small: 0.82, normal: 1, large: 1.25 };
export const SPACE_MULT: Record<TextSpacing, number> = { tight: 0.7, normal: 1, wide: 1.4 };

/** `color`: chybí = podle hodnoty (dnešní), `accent`, `#rrggbb`. Kolo 27: `size` (chybí = normal), `bg` kostka kolem dílu
 *  (chybí = žádná, `accent`, `#rrggbb`), `ink` barva textu (chybí = podle `color`). */
export type Piece = { item: ComposeItem; style: ItemStyle; color?: string; size?: PieceSize; bg?: string; ink?: string; onlyActive?: boolean };
/** Akcent hrany: `color` = `accent` | `claude` (podle stavu Clauda) | `#rrggbb`; `color2` = druhá barva přechodu. */
export type ComposeFill = { kind: FillKind; from: string; to: string; dir: (typeof FILL_DIRS)[number]; via: string; flow: boolean };
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
  deco: ComposeDeco;
  fill: ComposeFill;
  sep: ComposeSep;
  /** písmo s pevnou šířkou (Cascadia Mono) */
  mono: boolean;
  /** písmo nezávislé na výšce (kolo 27) */
  text: ComposeText;
};

export const MAX_PIECES = 12;
export const MAX_ZONE = 5;
export const W_MIN = 60;
export const W_MAX = 1200;
export const DEFAULT_FILL: ComposeFill = { kind: "black", from: "", to: "", dir: "right", via: "", flow: false };
const isHex = (s: unknown): s is string => typeof s === "string" && /^#[0-9a-fA-F]{6}$/.test(s.trim());
const hex = (s: unknown) => (isHex(s) ? s.trim().toLowerCase() : "");
/** Barva dílu jako Rust `norm_piece_color`: "" (podle hodnoty) | accent | #rrggbb. */
export const pieceColor = (s: unknown) => (typeof s === "string" && s.trim().toLowerCase() === "accent" ? "accent" : hex(s));
export const H_MIN = 18;
export const H_MAX = 52;
export const LINE_H_MIN = 2;
export const LINE_H_MAX = 10;

export const hRange = (shape: ComposeShape): [number, number] => (shape === "line" ? [LINE_H_MIN, LINE_H_MAX] : [H_MIN, H_MAX]);
const clamp = (v: unknown, a: number, b: number, d: number) => {
  const n = typeof v === "number" && Number.isFinite(v) ? Math.round(v) : d;
  return Math.min(b, Math.max(a, n));
};

/** „claude:icon“ / „time:chip:#ff8800“ → položka (styl chybí = ikona, barva chybí = podle hodnoty). */
function mkPiece(s: string): Piece {
  const [item, style, color] = s.split(":");
  const p: Piece = { item: item as ComposeItem, style: (style ?? "icon") as ItemStyle };
  if (color) p.color = color;
  return p;
}
function mk(shape: ComposeShape, width: number, height: number, round: number, left: string[], center: string[], right: string[], extra: Partial<StripCompose> = {}): StripCompose {
  return { shape, width, height, round, left: left.map(mkPiece), center: center.map(mkPiece), right: right.map(mkPiece), hideIdle: false, rotate: false, deco: { ...DEFAULT_DECO }, fill: { ...DEFAULT_FILL }, sep: "none", mono: false, text: { ...DEFAULT_TEXT }, ...extra };
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

/** Přidá dílům vlastní vzhled (`size` / `bg` / `ink`) podle položky — jen pro zápis předvoleb. */
function look(c: StripCompose, by: Partial<Record<ComposeItem, Pick<Piece, "size" | "bg" | "ink">>> = {}): StripCompose {
  for (const s of SLOTS) c[s] = c[s].map((p) => ({ ...p, ...(by[p.item] ?? {}) }));
  return c;
}

/** Designové předvolby kola 25 (výzdoba + styly dílů); jména v editoru (`DESIGN_NAME`). */
const DESIGN: Record<string, StripCompose> = {
  islands: mk("capsule", 420, 30, 100, ["claude:dotPct"], ["time:iconText"], ["weather:iconText", "cpu:spark"], { sep: "islands", deco: { ...DEFAULT_DECO, edge: "underline", color: "claude" } }),
  neon: mk("capsule", 340, 30, 60, ["claude:ring5h"], ["time:text"], ["cpu:bar"], { deco: { ...DEFAULT_DECO, edge: "glow", len: "full", color: "#22d3ee" } }),
  terminal: mk("capsule", 520, 26, 10, ["claude:text:#7ee787", "activity:text"], ["time:text"], ["cpu:text", "memory:text"], { sep: "bar", mono: true, deco: { ...DEFAULT_DECO, edge: "underline", len: "full", color: "#7ee787", width: 1 } }),
  chips: mk("capsule", 460, 32, 100, ["claude:chip"], ["time:chip"], ["limit5h:chip", "weather:chip:#f59e0b"], { sep: "gap" }),
  glass: mk("island", 360, 32, 100, ["claude:iconText"], ["time:text"], ["battery:iconText", "weather:iconText"], { sep: "dot", fill: { ...DEFAULT_FILL, kind: "glass" }, deco: { ...DEFAULT_DECO, edge: "outline", color: "#ffffff", width: 1 } }),
  sunset: mk("drop", 400, 34, 80, ["claude:dotPct"], ["time:text"], ["weather:iconText"], { fill: { ...DEFAULT_FILL, kind: "gradient", from: "#5b247a", to: "#e8604c" }, deco: { ...DEFAULT_DECO, edge: "gradient", len: "full", color: "#ffd166", color2: "#ef476f" } }),
  tall: look(mk("capsule", 340, 46, 70, ["claude:icon"], ["time:text"], ["limit5h:ring"], { text: { size: 0.72, weight: "semi", spacing: "wide" } })),
  cubes: look(mk("capsule", 440, 34, 100, ["claude:iconText"], ["time:text"], ["weather:text", "cpu:text"], { sep: "gap", text: { size: 0.9, weight: "semi", spacing: "normal" } }), {
    claude: { bg: "accent", ink: "#ffffff" },
    time: { size: "large" },
    weather: { bg: "#3b82f6", ink: "#ffffff", size: "small" },
    cpu: { bg: "#f59e0b", ink: "#ffffff", size: "small" },
  }),
  // kolo 28: živá hrana (římsa) — přelévání podle stavu, žhavá římsa s rozsvícením, kometa
  aurora: mk("notch", 320, 32, 70, ["claude:icon"], ["time:text"], ["limit5h:ring"], { deco: { ...DEFAULT_DECO, edge: "flow", len: "wide", color: "aurora", live: { ...DEFAULT_LIVE, on: true } } }),
  embers: mk("capsule", 380, 30, 60, ["claude:dotPct"], ["time:text"], ["cpu:spark"], { fill: { ...DEFAULT_FILL, kind: "gradient", from: "#2a0e05", via: "#160603", to: "#000000", flow: true }, deco: { ...DEFAULT_DECO, edge: "light", len: "full", color: "embers", live: { ...DEFAULT_LIVE, on: true } } }),
  comet: mk("island", 300, 30, 100, ["claude:iconText"], [], ["time:text"], { deco: { ...DEFAULT_DECO, edge: "comet", len: "full", color: "ocean", speed: 2, live: { ...DEFAULT_LIVE, on: true, work: "comet" } } }),
  hardware: mk("notch", 420, 32, 70, ["cpu:spark", "memory:spark"], ["time:text"], ["gpu:spark"], { deco: { ...DEFAULT_DECO, edge: "underline", len: "wide", color: "#22d3ee" } }),
  hwbar: mk("notch", 380, 30, 65, ["claude:dotPct"], ["time:text"], ["cpu:bar", "memory:bar", "gpu:bar"], { sep: "gap", deco: { ...DEFAULT_DECO, edge: "glow", len: "center", color: "claude" } }),
  cyber: mk("capsule", 460, 30, 40, ["claude:iconText", "limit5h:ring"], ["time:text"], ["cpu:text", "memory:text", "gpu:text"], { sep: "pills", mono: true, fill: { ...DEFAULT_FILL, kind: "gradient", from: "#0f172a", to: "#020617" }, deco: { ...DEFAULT_DECO, edge: "gradient", len: "full", color: "#38bdf8", color2: "#a855f7" } }),
  hwrings: mk("notch", 380, 32, 70, ["cpu:ring:#22d3ee", "memory:ring:#a855f7"], ["time:text"], ["gpu:ring:#10b981", "battery:ring"], { deco: { ...DEFAULT_DECO, edge: "glow", len: "wide", color: "#22d3ee" } }),
  devtools: mk("notch", 480, 30, 45, ["repos:iconText", "servers:badge", "claude:dotPct"], ["time:text"], ["cpu:bar", "memory:bar", "gpu:bar"], { sep: "bar", mono: true, deco: { ...DEFAULT_DECO, edge: "underline", len: "full", color: "#38bdf8", width: 1 } }),
  groove: mk("island", 390, 32, 100, ["music:iconText:#ec4899"], ["time:text"], ["cpu:spark", "gpu:spark"], { sep: "dot", fill: { ...DEFAULT_FILL, kind: "glass" }, deco: { ...DEFAULT_DECO, edge: "glow", len: "center", color: "#ec4899" } }),
  claudefocus: mk("capsule", 440, 32, 75, ["claude:iconText", "activity:text"], ["time:text"], ["limit5h:ring", "limitWeek:ring"], { sep: "gap", deco: { ...DEFAULT_DECO, edge: "flow", len: "full", color: "claude", live: { ...DEFAULT_LIVE, on: true } } }),
  matrix: mk("capsule", 460, 26, 15, ["claude:text:#22c55e", "servers:text:#22c55e"], ["time:text:#4ade80"], ["cpu:spark:#22c55e", "memory:spark:#22c55e", "gpu:spark:#22c55e"], { sep: "bar", mono: true, fill: { ...DEFAULT_FILL, kind: "black" }, deco: { ...DEFAULT_DECO, edge: "glow", len: "full", color: "#22c55e", width: 1 } }),
  zen: mk("island", 220, 26, 100, ["claude:dot"], ["time:text"], ["battery:icon"], { sep: "none", fill: { ...DEFAULT_FILL, kind: "glass" }, deco: { ...DEFAULT_DECO, edge: "outline", color: "#ffffff", width: 1 } }),
};
export const DESIGN_IDS = ["islands", "neon", "terminal", "chips", "glass", "sunset", "tall", "cubes", "aurora", "embers", "comet", "hardware", "hwbar", "cyber", "hwrings", "devtools", "groove", "claudefocus", "matrix", "zen"] as const;
export type DesignId = (typeof DESIGN_IDS)[number];

const clone = (c: StripCompose): StripCompose => ({ ...c, left: c.left.map((p) => ({ ...p })), center: c.center.map((p) => ({ ...p })), right: c.right.map((p) => ({ ...p })), deco: { ...c.deco, live: { ...c.deco.live } }, fill: { ...c.fill }, text: { ...c.text } });

/** Výchozí compose (= Rust `Compose::default`, předvolba Klasický). */
export const DEFAULT_COMPOSE: StripCompose = PRESETS.classic;

/** Předvolba z hotového vzhledu; neznámý / `custom` = Klasický. Vždy nová kopie. */
export function composeOf(look: string): StripCompose {
  return clone(PRESETS[look] ?? PRESETS.classic);
}
/** Designová předvolba (kolo 25); neznámá = Klasický. Vždy nová kopie. */
export function designOf(id: string): StripCompose {
  return clone(DESIGN[id] ?? PRESETS.classic);
}

/** Ořez jako Rust `Compose::norm`: rozsahy, každá položka jednou, 1–12 položek (zóna 5), barvy jen známé;
 *  neznámé hodnoty = výchozí. */
export function normCompose(raw: unknown): StripCompose {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const d = DEFAULT_COMPOSE;
  const shape = COMPOSE_SHAPES.includes(r.shape as ComposeShape) ? (r.shape as ComposeShape) : d.shape;
  const [h0, h1] = hRange(shape);
  const seen = new Set<string>();
  const take = (v: unknown): Piece[] => {
    const out: Piece[] = [];
    for (const p of Array.isArray(v) ? v : []) {
      const item = (p as Piece)?.item;
      if (!COMPOSE_ITEMS.includes(item) || seen.has(item) || seen.size >= MAX_PIECES || out.length >= MAX_ZONE) continue;
      seen.add(item);
      const st = (p as Piece).style;
      const x: Piece = { item, style: ITEM_STYLES.includes(st) ? st : "icon" };
      const col = pieceColor((p as Piece).color);
      if (col) x.color = col;
      const sz = (p as Piece).size;
      if (sz === "small" || sz === "large") x.size = sz;
      const bg = pieceColor((p as Piece).bg);
      if (bg) x.bg = bg;
      const ink = pieceColor((p as Piece).ink);
      if (ink) x.ink = ink;
      if ((p as Piece).onlyActive === true) x.onlyActive = true;
      out.push(x);
    }
    return out;
  };
  const rf = (r.fill && typeof r.fill === "object" ? r.fill : {}) as Record<string, unknown>;
  const pick = <T extends string>(list: readonly T[], v: unknown, dflt: T): T => (list.includes(v as T) ? (v as T) : dflt);
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
    deco: normDeco(r.deco),
    fill: { kind: pick(FILL_KINDS, rf.kind, "black"), from: hex(rf.from), to: hex(rf.to), dir: pick(FILL_DIRS, rf.dir, "right"), via: hex(rf.via), flow: rf.flow === true },
    sep: pick(COMPOSE_SEPS, r.sep, "none"),
    mono: r.mono === true,
    text: normText(r.text),
  };
}

/** Ořez písma jako Rust `TextLook::norm`: velikost 0,6–1,4 na dvě desetinná místa, neznámé = výchozí. */
export function normText(raw: unknown): ComposeText {
  const t = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const n = typeof t.size === "number" && Number.isFinite(t.size) ? Math.round(Math.min(TEXT_MAX, Math.max(TEXT_MIN, t.size)) * 100) / 100 : 1;
  return {
    size: n,
    weight: TEXT_WEIGHTS.includes(t.weight as TextWeight) ? (t.weight as TextWeight) : "normal",
    spacing: TEXT_SPACINGS.includes(t.spacing as TextSpacing) ? (t.spacing as TextSpacing) : "normal",
  };
}

/** Písmo proužku (částečná změna). */
export function setText(c: StripCompose, patch: Partial<ComposeText>): StripCompose {
  return { ...clone(c), text: normText({ ...c.text, ...patch }) };
}

/** Vlastní vzhled dílu (kolo 27): velikost, pozadí kostky, barva textu; prázdné / `normal` = dnešní (klíč se vynechá). */
export function setPieceLook(c: StripCompose, item: ComposeItem, patch: { size?: PieceSize; bg?: string; ink?: string; onlyActive?: boolean }): StripCompose {
  const x = clone(c);
  for (const s of SLOTS)
    x[s] = x[s].map((p) => {
      if (p.item !== item) return p;
      const { size: _s, bg: _b, ink: _i, onlyActive: _o, ...rest } = p;
      const size = patch.size ?? p.size;
      const bg = "bg" in patch ? pieceColor(patch.bg) : p.bg;
      const ink = "ink" in patch ? pieceColor(patch.ink) : p.ink;
      const onlyActive = "onlyActive" in patch ? patch.onlyActive === true : p.onlyActive === true;
      return { ...rest, ...(size === "small" || size === "large" ? { size } : {}), ...(bg ? { bg } : {}), ...(ink ? { ink } : {}), ...(onlyActive ? { onlyActive: true } : {}) };
    });
  return x;
}

/** Styly, které položka nabízí: průběh jen CPU / paměť / GPU (mají historii), „tečka + 5h“ a „prstenec 5h“ jen Claude. */
export function stylesFor(item: ComposeItem): ItemStyle[] {
  return ITEM_STYLES.filter((s) => (s === "spark" ? item === "cpu" || item === "memory" || item === "gpu" : s === "dotPct" || s === "ring5h" ? item === "claude" : true));
}
export function setPieceColor(c: StripCompose, item: ComposeItem, color: string): StripCompose {
  const x = clone(c);
  const col = pieceColor(color);
  for (const s of SLOTS)
    x[s] = x[s].map((p) => {
      if (p.item !== item) return p;
      const { color: _, ...rest } = p;
      return col ? { ...rest, color: col } : rest;
    });
  return x;
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
  if (x[slot].length >= MAX_ZONE) return c;
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
export type DrawKind = "icon" | "badge" | "text" | "dot" | "ring" | "meter" | "iconText" | "under" | "spark" | "dotPct";
export function drawKind(item: ComposeItem, style: ItemStyle, hasFrac: boolean): DrawKind {
  if (style === "ring") return hasFrac || item === "claude" ? "ring" : "icon";
  if (style === "ring5h") return item === "claude" || hasFrac ? "ring" : "icon";
  if (style === "graph") return hasFrac ? "meter" : "icon";
  if (style === "chip") return "iconText";
  if (style === "bar") return hasFrac ? "under" : "text";
  if (style === "spark") return item === "cpu" || item === "memory" || item === "gpu" ? "spark" : hasFrac ? "meter" : "icon";
  return style;
}

export function setStyle(c: StripCompose, item: ComposeItem, style: ItemStyle | "next"): StripCompose {
  const x = clone(c);
  const next = (p: Piece) => {
    const list = stylesFor(p.item);
    return list[(list.indexOf(p.style) + 1) % list.length];
  };
  for (const s of SLOTS) x[s] = x[s].map((p) => (p.item === item ? { ...p, style: style === "next" ? next(p) : style } : p));
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

/** Prohodí levou a pravou přihrádku (střed zůstane). */
export function mirrorSlots(c: StripCompose): StripCompose {
  const x = clone(c);
  [x.left, x.right] = [x.right, x.left];
  return x;
}

/** Shoda dvou vlastních vzhledů (pro zvýraznění aktivní předvolby v editoru). */
export function sameCompose(a: StripCompose, b: StripCompose): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
