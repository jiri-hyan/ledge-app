// Čistá logika pozadí otevřeného šuplíku (kolo 25, bez DOM, testuje `backdrop.test.mjs`):
//  - normalizace `appearance.backdrop` (neplatné = výchozí, tj. žádné pozadí = dnešní vzhled),
//  - interpolace zastávek (`colorAt`) a CSS gradientu (`gradientCss`: linear / radial / conic / mesh),
//  - ztmavovací závoj pod kartami podle kontrastu (`veilFor`, reuse `contrast.ts` `textProblems`),
//  - akcent z barev pozadí (`accentFromGradient`) a doladění akcentu na čitelnost (`fitAccent`),
//  - proměnné pro `appearanceVars` (`backdropVars`) a vlastní gradienty (přidat / změnit / smazat).
// Hotové gradienty jsou v `backdropPresets.ts`, kreslení živých pozadí v `backdropLive.ts`.
import type { Backdrop, CardsMode, CustomGradient, FlameColor, GradKind, GradStop, LiveKind } from "../types.ts";
import { contrast, over, parseColor, textProblems, toHex, type Rgba } from "./contrast.ts";
import { FLAME_COLORS, LIVE_PRESETS, asGradient, presetById } from "./backdropPresets.ts";

export const MAX_CUSTOM_GRADIENTS = 20;
export const STOPS_MIN = 2;
export const STOPS_MAX = 5;
export const MAX_GRAD_NAME = 24;
export const GRAD_KINDS: readonly GradKind[] = ["linear", "radial", "conic", "mesh"];
export const LIVE_KINDS: readonly LiveKind[] = ["fire", "aurora", "lava", "stars", "rain", "grain"];
export const FLAMES: readonly FlameColor[] = ["classic", "blue", "green", "violet"];

export const DEFAULT_BACKDROP: Backdrop = { id: "", live: "", flame: "classic", intensity: 0.6, speed: 1, cards: "auto", custom: [] };
export const CARDS_MODES: readonly CardsMode[] = ["auto", "solid", "tint", "glass"];

const isHex6 = (s: unknown): s is string => typeof s === "string" && /^#[0-9a-fA-F]{6}$/.test(s);
const num = (v: unknown, lo: number, hi: number, d: number) => (typeof v === "number" && v >= lo && v <= hi ? v : d);
const pick = <T extends string>(v: unknown, ok: readonly T[], d: T): T => (ok.includes(v as T) ? (v as T) : d);
export const customGradId = (s: unknown): s is string => typeof s === "string" && /^[a-z0-9-]{1,32}$/.test(s);

export type Gradient = Pick<CustomGradient, "kind" | "angle" | "stops">;

/** Zastávky seřazené podle polohy (stabilně), poloha oříznutá na 0–100. */
export const sortStops = (stops: readonly GradStop[]): GradStop[] =>
  stops.map((s, i) => ({ s, i })).sort((a, b) => a.s.p - b.s.p || a.i - b.i).map(({ s }) => ({ c: s.c.toLowerCase(), p: Math.max(0, Math.min(100, s.p)) }));

/** Platný vlastní gradient nebo `null` (jiný tvar, málo / moc zastávek, vadná barva). */
export function normalizeGradient(r: any): CustomGradient | null {
  if (!r || !customGradId(r.id) || !GRAD_KINDS.includes(r.kind) || !Array.isArray(r.stops)) return null;
  if (r.stops.length < STOPS_MIN || r.stops.length > STOPS_MAX || r.stops.some((s: any) => !isHex6(s?.c) || !(typeof s.p === "number" && s.p >= 0 && s.p <= 100))) return null;
  const name = typeof r.name === "string" && r.name.trim() ? r.name.trim().slice(0, MAX_GRAD_NAME) : r.id;
  return { id: r.id, name, kind: r.kind, angle: num(r.angle, 0, 360, 0), stops: sortStops(r.stops) };
}

/** Config z Rustu (nebo rozpracovaný v panelu) → platné pozadí; neznámý `id` = bez gradientu. */
export function normalizeBackdrop(raw: any): Backdrop {
  const d = DEFAULT_BACKDROP;
  const custom: CustomGradient[] = [];
  for (const g of Array.isArray(raw?.custom) ? raw.custom : []) {
    const n = normalizeGradient(g);
    if (n && custom.length < MAX_CUSTOM_GRADIENTS && !custom.some((c) => c.id === n.id) && !presetById(n.id)) custom.push(n);
  }
  const id = typeof raw?.id === "string" && (presetById(raw.id) || custom.some((c) => c.id === raw.id)) ? raw.id : "";
  return {
    id,
    live: pick(raw?.live, LIVE_KINDS, "") as Backdrop["live"],
    flame: pick(raw?.flame, FLAMES, d.flame),
    intensity: num(raw?.intensity, 0.1, 1, d.intensity),
    speed: num(raw?.speed, 0.3, 2, d.speed),
    cards: pick(raw?.cards, CARDS_MODES, d.cards),
    custom,
  };
}

/** Výchozí pozadí (= nic nenastaveno) — pole se pak z configu vynechá. */
export const isDefaultBackdrop = (b: Backdrop): boolean =>
  !b.id && !b.live && b.flame === "classic" && b.intensity === 0.6 && b.speed === 1 && b.cards === "auto" && b.custom.length === 0;

/** Gradient podle `id` (hotový nebo vlastní); `null` = bez gradientu. */
export function gradientOf(b: Backdrop): Gradient | null {
  if (!b.id) return null;
  const c = b.custom.find((x) => x.id === b.id);
  if (c) return c;
  const p = presetById(b.id);
  return p ? asGradient(p) : null;
}

// ---------- interpolace a CSS ----------
const hexRgb = (h: string): [number, number, number] => {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
};
export const mixHex = (a: string, b: string, t: number): string => {
  const [x, y] = [hexRgb(a), hexRgb(b)];
  return toHex(x.map((v, i) => v + (y[i] - v) * t));
};

/** Barva gradientu v poloze `p` (0–100): lineární míchání sousedních zastávek, mimo krajní = krajní barva. */
export function colorAt(stops: readonly GradStop[], p: number): string {
  const s = sortStops(stops);
  if (!s.length) return "#000000";
  if (p <= s[0].p) return s[0].c;
  for (let i = 1; i < s.length; i++) {
    if (p <= s[i].p) {
      const span = s[i].p - s[i - 1].p;
      return span <= 0 ? s[i].c : mixHex(s[i - 1].c, s[i].c, (p - s[i - 1].p) / span);
    }
  }
  return s[s.length - 1].c;
}

/** Polohy skvrn mesh gradientu (v %): rozptýlené po ploše, úhel je otáčí kolem středu. */
const MESH_AT: readonly (readonly [number, number])[] = [[14, 18], [86, 14], [78, 88], [18, 84], [50, 50]];
export function meshPoints(n: number, angle: number): [number, number][] {
  const a = (angle * Math.PI) / 180;
  const [sin, cos] = [Math.sin(a), Math.cos(a)];
  return MESH_AT.slice(0, n).map(([x, y]) => {
    const [dx, dy] = [x - 50, y - 50];
    return [Math.round(50 + dx * cos - dy * sin), Math.round(50 + dx * sin + dy * cos)];
  });
}

const stopList = (stops: readonly GradStop[]) => sortStops(stops).map((s) => `${s.c} ${Math.round(s.p * 10) / 10}%`).join(", ");

/** Základ mesh gradientu: průměr barev zastávek ztmavený (skvrny se kladou na něj). */
export function meshBase(stops: readonly GradStop[]): string {
  const rgb = stops.map((s) => hexRgb(s.c));
  const avg = [0, 1, 2].map((i) => rgb.reduce((t, c) => t + c[i], 0) / Math.max(1, rgb.length));
  return toHex(avg.map((v) => v * 0.35));
}

/** CSS `background-image` pro gradient (u mesh víc vrstev oddělených čárkou, vrchní první). */
export function gradientCss(g: Gradient): string {
  const stops = sortStops(g.stops);
  if (!stops.length) return "none";
  if (g.kind === "radial") {
    const a = (g.angle * Math.PI) / 180;
    return `radial-gradient(ellipse 140% 140% at ${Math.round(50 + 45 * Math.sin(a))}% ${Math.round(50 - 45 * Math.cos(a))}%, ${stopList(stops)})`;
  }
  if (g.kind === "conic") {
    const closed = stops[stops.length - 1].p < 100 ? [...stops, { c: stops[0].c, p: 100 }] : stops;
    return `conic-gradient(from ${Math.round(g.angle)}deg at 50% 40%, ${stopList(closed)})`;
  }
  if (g.kind === "mesh") {
    const pts = meshPoints(stops.length, g.angle);
    const layers = stops.map((s, i) => {
      const r = Math.round(38 + s.p * 0.5);
      return `radial-gradient(ellipse ${r}% ${Math.round(r * 1.1)}% at ${pts[i][0]}% ${pts[i][1]}%, ${s.c} 0%, transparent 100%)`;
    });
    return [...layers, `linear-gradient(0deg, ${meshBase(stops)}, ${meshBase(stops)})`].join(", ");
  }
  return `linear-gradient(${Math.round(g.angle)}deg, ${stopList(stops)})`;
}

// ---------- závoj podle kontrastu ----------
/** Barvy, které gradient může na ploše mít: zastávky a středy mezi nimi (u mesh i jejich průměr s hlubokým základem). */
export function sampleColors(g: Gradient): string[] {
  const s = sortStops(g.stops);
  const out = s.map((x) => x.c);
  for (let i = 1; i < s.length; i++) out.push(mixHex(s[i - 1].c, s[i].c, 0.5));
  if (g.kind === "mesh") out.push(meshBase(s));
  return out;
}

export const VEIL_MIN = 0.1;
export const VEIL_MAX = 0.92;
const VEIL_STEP = 0.02;

/** Nejslabší závoj barvy motivu (`vars.bg`), při kterém jsou všechny texty čitelné (≥ 4.5:1) na kartách
 *  i t1 / t2 přímo na ploše — pro každou barvu, kterou gradient (a živé pozadí `tones`) na ploše má.
 *  Nejhůř vyjde-li i 0.92, vrací 0.92 (víc už gradient nezůstane vidět). */
export function veilFor(colors: readonly string[], vars: Record<string, string>, tones: readonly string[] = []): number {
  const bg = parseColor(vars.bg);
  const samples = [...colors, ...tones].map((c) => parseColor(c));
  const ok = (alpha: number) =>
    samples.every((s) => {
      const shell = over([bg[0], bg[1], bg[2], alpha], s);
      const v = { ...vars, bg: `${Math.round(shell[0])} ${Math.round(shell[1])} ${Math.round(shell[2])}` };
      return textProblems(v).length === 0 && contrast(parseColor(vars.t1), shell) >= 4.5 && contrast(parseColor(vars.t2), shell) >= 4.5;
    });
  for (let i = 0; VEIL_MIN + i * VEIL_STEP < VEIL_MAX; i++) {
    const a = Math.round((VEIL_MIN + i * VEIL_STEP) * 100) / 100;
    if (ok(a)) return a;
  }
  return VEIL_MAX;
}

/** Odstín živého pozadí, který se při výpočtu závoje přidá k barvám plochy (zeslabený podle síly). */
export function liveTone(b: Backdrop, base: string): string[] {
  if (!b.live) return [];
  const lp = LIVE_PRESETS.find((l) => l.id === b.live);
  const tone = b.live === "fire" ? FLAME_COLORS[b.flame][1] : (lp?.tone ?? base);
  return [mixHex(base, tone, 0.45 * b.intensity)];
}

// ---------- akcent z pozadí ----------
const rgbHsl = (hex: string): [number, number, number] => {
  const [r, g, b] = hexRgb(hex).map((v) => v / 255);
  const max = Math.max(r, g, b), min = Math.min(r, g, b), l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, s, l];
};
const hslHex = (h: number, s: number, l: number): string => {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return toHex([f(0) * 255, f(8) * 255, f(4) * 255]);
};

/** Dvě akcentní barvy z gradientu: nejsytější zastávky, rozjasněné do čitelného pásma (tmavé pozadí → světlejší
 *  akcent). Šedý gradient (žádná sytost) → `null` (zůstane akcent motivu). Druhá barva = nejvzdálenější odstín. */
export function accentFromGradient(g: Gradient): [string, string] | null {
  const cand = sortStops(g.stops).map((s) => ({ hex: s.c, hsl: rgbHsl(s.c) })).filter((x) => x.hsl[1] >= 0.2);
  if (!cand.length) return null;
  cand.sort((a, b) => b.hsl[1] * (1 - Math.abs(b.hsl[2] - 0.4)) - a.hsl[1] * (1 - Math.abs(a.hsl[2] - 0.4)));
  const first = cand[0];
  const hueDist = (x: number, y: number) => Math.min(Math.abs(x - y), 360 - Math.abs(x - y));
  const second = cand.slice(1).sort((a, b) => hueDist(b.hsl[0], first.hsl[0]) - hueDist(a.hsl[0], first.hsl[0]))[0] ?? first;
  const lift = (h: [number, number, number]) => hslHex(h[0], Math.max(h[1], 0.55), Math.max(h[2], 0.58));
  return [lift(first.hsl), second === first ? hslHex((first.hsl[0] + 40) % 360, Math.max(first.hsl[1], 0.55), 0.62) : lift(second.hsl)];
}

/** Barva akcentu pro text na kartách motivu: nejhorší poměr kontrastu proti plochám karet. */
export function accentContrast(hex: string, vars: Record<string, string>): number {
  const bg = parseColor(vars.bg);
  const planes: Rgba[] = [];
  for (const desk of [[0, 0, 0, 1], [255, 255, 255, 1]] as Rgba[]) {
    const shell = over([bg[0], bg[1], bg[2], 0.985], desk);
    for (const k of ["card", "card-2"]) planes.push(over(parseColor(vars[k]), shell));
  }
  return Math.min(...planes.map((p) => contrast(parseColor(hex), p)));
}

/** Akcent doladěný tak, aby jako text na kartách motivu dal ≥ `min`:1 (světlé motivy ztmaví, tmavé zesvětlí).
 *  Odstín a sytost zůstávají; výchozí vzhled se tím nemění (volá se jen u nových voleb akcentu). */
export function fitAccent(hex: string, vars: Record<string, string>, min = 4.5): string {
  if (accentContrast(hex, vars) >= min) return hex.toLowerCase();
  const [h, s, l] = rgbHsl(hex);
  const light = lum01(vars) > 0.5;
  for (let i = 1; i <= 40; i++) {
    const nl = Math.max(0.05, Math.min(0.95, l + (light ? -1 : 1) * i * 0.015));
    const c = hslHex(h, s, nl);
    if (accentContrast(c, vars) >= min) return c;
  }
  return hex.toLowerCase();
}
const lum01 = (vars: Record<string, string>): number => {
  const [r, g, b] = parseColor(vars.bg);
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
};

/** Text na akcentu (tlačítko, výplň): černá nebo bílá podle vyššího kontrastu. */
export const onAccent = (hex: string): string => (contrast(parseColor(hex), [255, 255, 255, 1]) >= contrast(parseColor(hex), [0, 0, 0, 1]) ? "#ffffff" : "#000000");

// ---------- proměnné ----------
/** CSS proměnné pozadí (`bd-img`, `bd-veil`); prázdné = bez gradientu. `vars` = tokeny motivu pro závoj. */
export function backdropVars(b: Backdrop, vars: Record<string, string>): Record<string, string> {
  const g = gradientOf(b);
  if (!g && !b.live) return {};
  const base = g ? sampleColors(g) : [toHex(parseColor(vars.bg))];
  const out: Record<string, string> = { "bd-veil": String(veilFor(base, vars, liveTone(b, base[0]))) };
  if (g) out["bd-img"] = gradientCss(g);
  return out;
}

// ---------- karty nad pozadím ----------
/** Skutečný vzhled karet: `auto` = průsvitné jen když je pozadí zvolené; bez pozadí vždy `solid` (nic se nemění). */
export function cardMode(b: Backdrop | null | undefined): "solid" | "tint" | "glass" {
  if (!b || (!b.id && !b.live)) return "solid";
  return b.cards === "auto" ? "tint" : b.cards;
}

export const CARD_ALPHA_MIN = 0.55;
export const CARD_ALPHA_MAX = 0.7;
/** Krytí průsvitných karet 0.55–0.70: čím víc závoje pozadí potřebuje, tím krytější karta. */
export const cardAlpha = (veil: number): number =>
  Math.round((CARD_ALPHA_MIN + ((Math.max(VEIL_MIN, Math.min(VEIL_MAX, veil)) - VEIL_MIN) / (VEIL_MAX - VEIL_MIN)) * (CARD_ALPHA_MAX - CARD_ALPHA_MIN)) * 100) / 100;

/** Nejhorší kontrast textu `t1` / `t2` na průsvitné kartě nad barvou pozadí `sample` (pozadí → závoj → karta). */
export function cardContrast(vars: Record<string, string>, sample: string, veil: number, alpha = cardAlpha(veil)): number {
  const bg = parseColor(vars.bg);
  const shell = over([bg[0], bg[1], bg[2], veil], parseColor(sample));
  const card = over([bg[0], bg[1], bg[2], alpha], shell);
  return Math.min(contrast(parseColor(vars.t1), card), contrast(parseColor(vars.t2), card));
}

/** Proměnné průsvitných karet (`m-card-bg`, `k-base`, okraj, rozmazání) — prázdné pro `solid`. */
export function cardVars(b: Backdrop | null | undefined, vars: Record<string, string>): Record<string, string> {
  const mode = cardMode(b);
  if (mode === "solid") return {};
  const a = cardAlpha(Number(vars["bd-veil"]) || VEIL_MIN);
  const fill = `rgb(${vars.bg} / ${a})`;
  const out: Record<string, string> = { "m-card-bg": fill, "k-base": fill, "bd-card": fill };
  if ((vars["m-card-bd"] ?? "").startsWith("1px solid")) out["m-card-bd"] = "1px solid color-mix(in srgb, var(--t1) 14%, transparent)";
  return out;
}

// ---------- vlastní gradienty ----------
export function newGradId(existing: readonly string[], name: string): string {
  const slug = name.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 16).replace(/-+$/, "");
  const stem = `my-${slug || "gradient"}`;
  let id = stem;
  for (let i = 2; existing.includes(id) || presetById(id); i++) id = `${stem}-${i}`;
  return id;
}

/** Přidá vlastní gradient; `null` = už je jich 20. */
export function addGradient(list: readonly CustomGradient[], name: string, g: Gradient): { list: CustomGradient[]; id: string } | null {
  if (list.length >= MAX_CUSTOM_GRADIENTS) return null;
  const clean = name.trim().slice(0, MAX_GRAD_NAME) || "Gradient";
  const id = newGradId(list.map((c) => c.id), clean);
  const n = normalizeGradient({ id, name: clean, ...g });
  return n ? { list: [...list, n], id } : null;
}

/** Nový gradient pro editor: výchozí = půlnoc → modrá. */
export const blankGradient = (): Gradient => ({ kind: "linear", angle: 160, stops: [{ c: "#1b2a5a", p: 0 }, { c: "#0a0e1c", p: 100 }] });

/** Přidá zastávku doprostřed největší mezery (barva = interpolace v tom místě); max 5. */
export function addStop(stops: readonly GradStop[]): GradStop[] {
  const s = sortStops(stops);
  if (s.length >= STOPS_MAX) return s;
  let at = 0, gap = -1;
  for (let i = 1; i < s.length; i++) if (s[i].p - s[i - 1].p > gap) ((gap = s[i].p - s[i - 1].p), (at = i));
  const p = Math.round((s[at - 1].p + s[at].p) / 2);
  return sortStops([...s, { c: colorAt(s, p), p }]);
}

/** Odebere zastávku `i` (po řazení); min 2. */
export const removeStop = (stops: readonly GradStop[], i: number): GradStop[] => {
  const s = sortStops(stops);
  return s.length <= STOPS_MIN ? s : s.filter((_, k) => k !== i);
};

/** Živé pozadí se smí kreslit jen otevřené a viditelné; `reduced` = jeden statický snímek. */
export type DrawState = { open: boolean; hidden: boolean; reduced: boolean; live: boolean };
export type DrawMode = "run" | "still" | "stop";
export function drawMode(s: DrawState): DrawMode {
  if (!s.live || !s.open || s.hidden) return "stop";
  return s.reduced ? "still" : "run";
}

/** Změna vlastního gradientu v seznamu (neplatný výsledek se ignoruje, zastávky se seřadí). */
export function updateGradient(list: readonly CustomGradient[], id: string, patch: { name?: string; kind?: GradKind; angle?: number; stops?: GradStop[] }): CustomGradient[] {
  return list.map((g) => {
    if (g.id !== id) return g;
    const next = normalizeGradient({ ...g, ...patch, name: patch.name === undefined ? g.name : patch.name.trim().slice(0, MAX_GRAD_NAME) || g.name });
    return next ?? g;
  });
}

/** Smaže vlastní gradient; `id` vybraného pozadí se vrátí na prázdné, když šlo o něj. */
export function removeGradient(b: Backdrop, id: string): Backdrop {
  return { ...b, custom: b.custom.filter((g) => g.id !== id), id: b.id === id ? "" : b.id };
}

/** Přesun zastávky `i` na polohu `p` (0–100, celé číslo); vrací seřazené zastávky a nový index přesunuté. */
export function moveStop(stops: readonly GradStop[], i: number, p: number): { stops: GradStop[]; index: number } {
  const moved = stops.map((s, k) => (k === i ? { ...s, p: Math.max(0, Math.min(100, Math.round(p))) } : s));
  const marked = moved.map((s, k) => ({ s, k })).sort((a, b) => a.s.p - b.s.p || (a.k === i ? 1 : 0) - (b.k === i ? 1 : 0) || a.k - b.k);
  return { stops: marked.map(({ s }) => ({ c: s.c.toLowerCase(), p: s.p })), index: marked.findIndex(({ k }) => k === i) };
}

/** Poloha 0–100 % z vodorovné souřadnice ukazatele nad lištou gradientu. */
export const posFromPointer = (x: number, left: number, width: number): number => (width <= 0 ? 0 : Math.max(0, Math.min(100, ((x - left) / width) * 100)));
