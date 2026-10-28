// Velikost šuplíku tažením za okraje v režimu úprav (kolo 23): stav a tažení bez UI. Čistá logika je v
// `theme/size.ts`. Během tažení: šířku posílá Rust (`api.drawerSize(…, false)`, nejvýš jednou za snímek —
// okno se zvětší kolem osy), výšku kreslí `.content` hned (`mainHeight` → App). Po puštění `done` = Rust
// ořízne a zapíše `appearance.size`; dokud událost `appearance` nepřinese uloženou hodnotu, drží se
// poslední tažená (žádné probliknutí). Zrušit v úpravách vrátí velikost z doby `beginEdit`.
// Žádný časovač: jen pointer události a jeden requestAnimationFrame během tažení.
import { createSignal } from "solid-js";
import { api } from "../api";
import { savedAppearance } from "../theme/apply";
import { dragWidth, dragged, HEIGHT_LINES, overflowCount, sizeFor, snapTo, switchedSize, WIDTH_LINES, type Side } from "../theme/size";
import type { DrawerSize, PageSize } from "../types";

type Live = { page: string; width: number; height: number; done: boolean };
const [live, setLive] = createSignal<Live | null>(null);
let editingSize = false;
const [tab, setTab] = createSignal("");
const [dim, setDim] = createSignal("");
const [overflow, setOverflow] = createSignal(0);
export { dim, overflow };

/** MainPage hlásí záložku, která je vidět. */
export function setSizeTab(id: string) {
  const prev = tab();
  setTab(id);
  // záložka přepnutá UVNITŘ úprav s „Pamatovat pro každou záložku“: okno se animuje na její velikost
  // (mimo úpravy to dělá Rust v `set_mode`)
  const v = editingSize ? switchedSize(savedAppearance().size, prev, id) : null;
  if (v) {
    setLive(null);
    void inflight.then(() => api.drawerSize(id, v.width, v.height, false)).then((eff) => void (eff && showDim(eff, v)), () => {});
    queueMicrotask(measureOverflow);
  }
}

/** Velikost záložky teď: během / těsně po tažení tažená, jinak uložená (0 = výchozí). */
export function sizeNow(page: string): Required<PageSize> {
  const saved = sizeFor(savedAppearance().size, page);
  const l = live();
  if (!l || l.page !== page) return saved;
  if (l.done && saved.width === l.width && saved.height === l.height) return saved;
  return { width: l.width, height: l.height };
}

/** Pevná výška hlavní stránky jako CSS (`.content` v App), `undefined` = podle obsahu (dnešní). */
export function mainHeight(): string | undefined {
  const h = sizeNow(tab()).height;
  return h > 0 ? `min(${h}px, var(--drawer-max-h, 100vh))` : undefined;
}

const content = () => document.querySelector<HTMLElement>(".content");

/** Kolik karet se po zmenšení nevejde do viditelné plochy (nic se nemaže — plocha se posouvá). */
export function measureOverflow() {
  const deck = document.querySelector<HTMLElement>(".content > .deck");
  if (!deck) return setOverflow(0);
  const top = deck.getBoundingClientRect().top - deck.scrollTop;
  const bottoms = [...deck.querySelectorAll<HTMLElement>(":scope > .deck-item")].map((el) => el.getBoundingClientRect().bottom - top);
  setOverflow(overflowCount(bottoms, deck.clientHeight));
}

let frame = 0;
let pending: Live | null = null;
/** poslední živý příkaz — puštění počká, ať ho Rust nezpracuje až po uložení (šířka by zůstala „živá“) */
let inflight: Promise<void> = Promise.resolve();

function flush() {
  frame = 0;
  const p = pending;
  if (!p) return;
  pending = null;
  setLive(p);
  inflight = api.drawerSize(p.page, p.width, p.height, false).then((eff) => void (eff && showDim(eff, p)), () => {});
  measureOverflow();
}

const showDim = (eff: [number, number], p: PageSize) =>
  setDim(`${eff[0] || window.innerWidth} × ${eff[1] || p.height || Math.round(content()?.getBoundingClientRect().height ?? 0)} px`);

async function commit(page: string, v: Required<PageSize>) {
  if (frame) cancelAnimationFrame(frame);
  frame = 0;
  pending = null;
  setLive({ page, ...v, done: true });
  await inflight;
  try {
    const eff = await api.drawerSize(page, v.width, v.height, true);
    if (eff) showDim(eff, v);
  } catch {
    setLive(null);
  }
  queueMicrotask(measureOverflow);
}

export type Edge = Side | "bottom";

/** Tah za úchyt: `pos` = pozice osy (`appearance.strip.position`), Alt = bez přichycení. */
export function startResize(e: PointerEvent, edge: Edge, pos: "center" | "left" | "right") {
  if (e.button !== 0) return;
  e.preventDefault();
  const el = e.currentTarget as HTMLElement;
  el.setPointerCapture(e.pointerId);
  const page = tab();
  const start = sizeNow(page);
  const w0 = window.innerWidth;
  const h0 = content()?.getBoundingClientRect().height ?? 0;
  const [x0, y0] = [e.screenX, e.screenY];
  let last: Required<PageSize> = start;
  const move = (m: PointerEvent) => {
    const d = dragged(edge === "bottom" ? w0 : dragWidth(w0, m.screenX - x0, edge, pos), edge === "bottom" ? h0 + m.screenY - y0 : h0, m.altKey);
    // tažený rozměr se mění, druhý zůstává, jak byl nastavený (0 = výchozí zůstane výchozí)
    last = edge === "bottom" ? { width: start.width, height: d.height } : { width: d.width, height: start.height };
    pending = { page, ...last, done: false };
    if (!frame) frame = requestAnimationFrame(flush);
  };
  const up = () => {
    el.removeEventListener("pointermove", move);
    el.removeEventListener("pointerup", up);
    el.removeEventListener("pointercancel", up);
    if (last !== start) void commit(page, last);
  };
  el.addEventListener("pointermove", move);
  el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", up);
}

/** Dvojklik na úchyt = výchozí (šířka z předvolby / výška podle obsahu). */
export function resetEdge(edge: Edge) {
  const page = tab();
  const s = sizeNow(page);
  void commit(page, edge === "bottom" ? { width: s.width, height: 0 } : { width: 0, height: s.height });
}

/** Šipky na zaostřeném úchytu: o čáru mřížky, s Alt o 10 px. */
export function keyResize(e: KeyboardEvent, edge: Edge) {
  const dir = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -1, ArrowDown: 1 }[e.key];
  const vertical = e.key === "ArrowUp" || e.key === "ArrowDown";
  if (!dir || vertical !== (edge === "bottom")) return;
  e.preventDefault();
  const page = tab();
  const s = sizeNow(page);
  if (edge === "bottom") {
    const h = s.height || Math.round(content()?.getBoundingClientRect().height ?? 0);
    return void commit(page, { width: s.width, height: dragged(0, step(h, dir, HEIGHT_LINES, e.altKey), true).height });
  }
  // levý bok doprava = užší (na středové ose i u pravé hrany)
  const grow = edge === "right" ? dir : -dir;
  const w = s.width || window.innerWidth;
  void commit(page, { width: dragged(step(w, grow, WIDTH_LINES, e.altKey), 0, true).width, height: s.height });
}

/** Další / předchozí čára mřížky (s `free` o 10 px). */
function step(v: number, dir: number, lines: readonly number[], free: boolean): number {
  if (free) return v + 10 * dir;
  const next = dir > 0 ? lines.find((l) => l > v + 1) : [...lines].reverse().find((l) => l < v - 1);
  return next ?? snapTo(v, lines, true);
}

// ---------- Zrušit v úpravách ----------

let before: DrawerSize | undefined | null = null;
/** záložka, na které úpravy začaly (= záložka Rustu po Zrušit) */
let origin = "";

/** `beginEdit`: zapamatovat uloženou velikost. */
export function rememberSize() {
  setDim("");
  before = structuredClone(savedAppearance().size);
  origin = tab();
  editingSize = true;
}

/** konec úprav (`end` v edit.ts) */
export function endSizeEdit() {
  editingSize = false;
  void api.drawerSizeEnd().catch(() => {});
}

/** `cancelEdit`: tažení v úpravách vrátit (zapíše config jen, když se velikost změnila). */
export async function revertSize() {
  const b = before;
  before = null;
  // záložka Rustu zůstala na naposledy navštívené v úpravách → zpět na výchozí (s její uloženou velikostí)
  const back = () => {
    const s = sizeFor(b ?? undefined, origin);
    if (origin && origin !== tab()) void api.drawerSize(origin, s.width, s.height, false).catch(() => {});
  };
  if (b === null) return;
  if (JSON.stringify(b ?? {}) === JSON.stringify(savedAppearance().size ?? {})) return back();
  setLive(null);
  const cfg = await api.getConfig();
  if (!cfg) return;
  const appearance = { ...(cfg.appearance ?? {}) };
  if (b) appearance.size = b;
  else delete appearance.size;
  await api.saveConfig({ ...cfg, appearance });
  back();
}
