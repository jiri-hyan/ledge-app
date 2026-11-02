// Živá hrana vlastního zavřeného Ledge (kolo 28, `strip.compose.deco`) — zrcadlo Rust `looks::edgefx`
// a `composedeco::Deco` (id hlídá test `edgefx::ts_mirror`): efekty hrany, délky linky, barevné sady (bez duhy),
// stavy živé hrany a ořez configu. Náhled (`previews/EdgeFx.tsx`) i editor (`panels/ComposeEdge.tsx`) čtou odsud.

export const COMPOSE_EDGES = ["none", "underline", "outline", "glow", "gradient", "flow", "dashes", "comet", "light", "breath", "outlineGradient", "sweep"] as const;
export type ComposeEdge = (typeof COMPOSE_EDGES)[number];
export const EDGE_LENS = ["center", "wide", "full", "custom"] as const;
export type EdgeLen = (typeof EDGE_LENS)[number];
export const LIVE_STATES = ["idle", "work", "wait", "done", "limit"] as const;
export type LiveState = (typeof LIVE_STATES)[number];
export const EDGE_SETS = ["aurora", "sunset", "ocean", "embers", "clay"] as const;
export type EdgeSet = (typeof EDGE_SETS)[number];
/** Barvy sad podél hrany (zrcadlo `edgefx::SETS`). */
export const SET_COLORS: Record<EdgeSet, [string, string, string]> = {
  aurora: ["#34d399", "#22d3ee", "#a78bfa"],
  sunset: ["#ffd166", "#f97316", "#e11d48"],
  ocean: ["#38bdf8", "#2dd4bf", "#6366f1"],
  embers: ["#fbbf24", "#f97316", "#dc2626"],
  clay: ["#d97757", "#f0b38f", "#b85a3a"],
};
/** Jednorázové naplnění „hotovo“ a délka „Vyzkoušet“ (ms). */
export const SWEEP_MS = 1200;
export const TRY_MS = 4000;
/** Efekty, které se hýbou (jen když Claude pracuje / čeká, nebo `always`). */
export const MOVING: readonly ComposeEdge[] = ["flow", "dashes", "comet", "breath"];
/** Efekty pro živou hranu (stálá hrana nabízí všechny kromě jednorázového naplnění). */
export const BASE_EDGES = COMPOSE_EDGES.filter((e) => e !== "sweep");

export type ComposeLive = { on: boolean; work: ComposeEdge; wait: ComposeEdge; done: ComposeEdge; limit: ComposeEdge };
export const DEFAULT_LIVE: ComposeLive = { on: false, work: "flow", wait: "breath", done: "sweep", limit: "dashes" };
export type ComposeDeco = {
  edge: ComposeEdge;
  len: EdgeLen;
  color: string;
  color2: string;
  width: number;
  color3: string;
  pct: number;
  speed: number;
  power: number;
  always: boolean;
  live: ComposeLive;
};
export const DEFAULT_DECO: ComposeDeco = { edge: "none", len: "center", color: "accent", color2: "", width: 2, color3: "", pct: 40, speed: 3, power: 2, always: false, live: { ...DEFAULT_LIVE } };

const isHex = (s: unknown): s is string => typeof s === "string" && /^#[0-9a-fA-F]{6}$/.test(s.trim());
export const hex = (s: unknown) => (isHex(s) ? s.trim().toLowerCase() : "");
/** Barva hrany jako Rust `norm_edge_color`: accent | claude | sada | #rrggbb. */
export const edgeColor = (s: unknown) => {
  const v = typeof s === "string" ? s.trim().toLowerCase() : "";
  return v === "claude" || (EDGE_SETS as readonly string[]).includes(v) ? v : hex(s) || "accent";
};
const clamp = (v: unknown, a: number, b: number, d: number) => {
  const n = typeof v === "number" && Number.isFinite(v) ? Math.round(v) : d;
  return Math.min(b, Math.max(a, n));
};
const pick = <T extends string>(list: readonly T[], v: unknown, dflt: T): T => (list.includes(v as T) ? (v as T) : dflt);

/** Ořez `deco` jako Rust `Deco::norm` (chybějící klíče = výchozí = vzhled před kolem 28). */
export function normDeco(raw: unknown): ComposeDeco {
  const rd = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const rl = (rd.live && typeof rd.live === "object" ? rd.live : {}) as Record<string, unknown>;
  const L = DEFAULT_LIVE;
  return {
    edge: pick(COMPOSE_EDGES, rd.edge, "none"),
    len: pick(EDGE_LENS, rd.len, "center"),
    color: "color" in rd ? edgeColor(rd.color) : "accent",
    color2: hex(rd.color2),
    width: clamp(rd.width, 1, 3, 2),
    color3: hex(rd.color3),
    pct: clamp(rd.pct, 10, 100, 40),
    speed: clamp(rd.speed, 1, 5, 3),
    power: clamp(rd.power, 1, 3, 2),
    always: rd.always === true,
    live: { on: rl.on === true, work: pick(COMPOSE_EDGES, rl.work, L.work), wait: pick(COMPOSE_EDGES, rl.wait, L.wait), done: pick(COMPOSE_EDGES, rl.done, L.done), limit: pick(COMPOSE_EDGES, rl.limit, L.limit) },
  };
}

/** Efekt, který se teď kreslí (jako Rust `edgefx::apply`): živá hrana podle stavu (nebo `tried`), jinak stálá. */
export function liveEdge(d: ComposeDeco, s: LiveState, tried = false): ComposeEdge {
  if (!d.live.on && !tried) return d.edge;
  const e = s === "idle" ? "none" : d.live[s];
  return e === "none" ? d.edge : e;
}

/** Barva stavu pro živou hranu (čeká jantar, hotovo zelená, limit jantar); `null` = barvy hrany. */
export function stateTone(d: ComposeDeco, s: LiveState, tried = false): "wait" | "ok" | null {
  if ((!d.live.on && !tried) || s === "idle" || d.live[s] === "none") return null;
  return s === "wait" || s === "limit" ? "wait" : s === "done" ? "ok" : null;
}

/** Okraje linky v % šířky (vlevo = vpravo): střed 40 % → 30 %, široká 80 % → 10 %, celá → 8 % (zaoblení). */
export function spanInset(d: ComposeDeco): number {
  switch (d.len) {
    case "wide": return 10;
    case "full": return 8;
    case "custom": return d.pct >= 100 ? 8 : Math.max(8, (100 - d.pct) / 2);
    default: return 30;
  }
}
