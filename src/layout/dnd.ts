// Přetahování karet jako přirozený drag & drop — čisté funkce bez DOM (test `dnd.test.mjs`). Řadič gesta
// `cardDrag.ts` jen měří DOM a volá je: podržení na hlavičce (práh času a pohybu, co je hlavička a co ovládací
// prvek), cíl vložení na jiné záložce, přepnutí záložky po prodlevě nad ouškem, zásah zóny „Skrýt kartu“,
// převod bodu ukazatele do souřadnic mřížky, poloha „ducha“ pod kurzorem a výpočet FLIP posunů.
// HTML5 drag&drop v okně Tauri nefunguje — celé je to na pointer událostech.
import { DROP_MARGIN, rectGap, type Rect } from "./model.ts";

/** Jak dlouho se drží hlavička karty, než se z toho stane tah (ms). */
export const HOLD_MS = 400;
/** O kolik px se smí ukazatel za dobu podržení pohnout (třesení ruky), než je to výběr textu / klik. */
export const HOLD_SLOP = 6;
/** Jak dlouho se musí tažená karta držet nad ouškem jiné záložky, než se záložka přepne (ms). */
export const DWELL_MS = 600;
/** Kolik px musí ukazatel ujet od úchytu / prázdného místa karty v úpravách, než začne tah. */
export const GRAB_PX = 4;

// ---------- podržení (mimo úpravy) ----------

export type HoldStep = "wait" | "start" | "cancel";
/** Podržení hlavičky: po `HOLD_MS` začíná tah (i kdyby časovač přišel pozdě a ukazatel se už hýbal); dřív
 *  větší posun než `HOLD_SLOP` = obyčejný klik / výběr textu / posun, podržení se ruší. */
export function holdStep(distPx: number, elapsedMs: number): HoldStep {
  if (elapsedMs >= HOLD_MS) return "start";
  return distPx > HOLD_SLOP ? "cancel" : "wait";
}

/** Hlavička karty = nadpis první dítě karty v `.deck-item` (mimo ni se podržení neuplatní — výběr textu,
 *  scroll i klik v těle karty zůstávají beze změny). */
export const HEADER_SELECTOR = ".deck-item > :first-child > h4";
/** Ovládací prvky uvnitř hlavičky: na nich se tah nezačíná (⋯ menu, tlačítka, odkazy, pole). */
export const CONTROL_SELECTOR = "button, a, input, select, textarea, label, summary, [role='button'], [data-cmenu], [contenteditable='true'], .mini";
/** Ovládací prvky karty v úpravách: úchyty okrajů, ⇔ ↕ ×. Úchyt ⠿ tah začíná sám jako zbytek karty. */
export const EDIT_CONTROL_SELECTOR = ".lay-size, .lay-remove, .lay-rs";

export type HoldProbe = { button: number; editing: boolean; header: boolean; control: boolean };
/** Smí podržení začít? Jen levé tlačítko, mimo úpravy (v nich se chytá kdekoli), na hlavičce a ne na ovládacím prvku. */
export const canHold = (p: HoldProbe): boolean => p.button === 0 && !p.editing && p.header && !p.control;

// ---------- cíl na jiné záložce ----------

/** Kam do karet cizí záložky (rámečky `rects` v pořadí záložky) vložit tahanou kartu: index 0…n. Karta, nad
 *  kterou ukazatel je (v mezeře nejbližší), dá „před“ nebo „za“ podle poloviny (široká karta přes celou řadu
 *  podle svislé, jinak podle vodorovné). Daleko mimo karty (víc než `DROP_MARGIN`) = `prev` (nic se nehýbe);
 *  prázdná záložka = 0. */
export function insertIndex(rects: readonly Rect[], prev: number, x: number, y: number, margin = DROP_MARGIN): number {
  if (!rects.length) return 0;
  const left = Math.min(...rects.map((r) => r.x)) - margin;
  const top = Math.min(...rects.map((r) => r.y)) - margin;
  const right = Math.max(...rects.map((r) => r.x + r.w)) + margin;
  const bottom = Math.max(...rects.map((r) => r.y + r.h)) + margin;
  if (x < left || x > right || y < top || y > bottom) return Math.max(0, Math.min(prev, rects.length));
  let best = 0;
  let bestD = Infinity;
  rects.forEach((r, i) => {
    const d = rectGap(r, x, y);
    if (d < bestD) {
      bestD = d;
      best = i;
    }
  });
  const r = rects[best];
  const rowWide = r.w >= (right - left - 2 * margin) * 0.9;
  const after = rowWide ? y >= r.y + r.h / 2 : x >= r.x + r.w / 2;
  return best + (after ? 1 : 0);
}

/** Seznam s `x` vloženým na místo `i` (mimo rozsah = na okraj); vstup zůstane netknutý. */
export function insertAt<T>(list: readonly T[], i: number, x: T): T[] {
  const out = [...list];
  out.splice(Math.max(0, Math.min(i, out.length)), 0, x);
  return out;
}

// ---------- přepnutí záložky po prodlevě ----------

export type Dwell = { tab: string | null; since: number };
export const NO_DWELL: Dwell = { tab: null, since: 0 };
/** Prodleva nad ouškem záložky: po `ms` se záložka přepne (`fire`), pak se počítá znovu. Odjetí = od začátku. */
export function dwellStep(d: Dwell, tab: string | null, now: number, ms = DWELL_MS): { dwell: Dwell; fire: string | null } {
  if (!tab) return { dwell: NO_DWELL, fire: null };
  if (tab !== d.tab) return { dwell: { tab, since: now }, fire: null };
  if (now - d.since >= ms) return { dwell: NO_DWELL, fire: tab };
  return { dwell: d, fire: null };
}

// ---------- zóna „Skrýt kartu“ ----------

export type Box = { left: number; top: number; right: number; bottom: number };
/** Je bod v obdélníku (s rezervou `pad` px na všech stranách)? */
export const inBox = (b: Box, x: number, y: number, pad = 0): boolean => x >= b.left - pad && x <= b.right + pad && y >= b.top - pad && y <= b.bottom + pad;

// ---------- souřadnice mřížky a „duch“ ----------

export type ScrollBox = { left: number; top: number; clientLeft: number; clientTop: number; scrollLeft: number; scrollTop: number };
/** Bod okna (px) → souřadnice obsahu posuvné mřížky (stejné jako `offsetLeft` / `offsetTop` karet: nezávislé
 *  na posunu mřížky i na rozběhlých animacích karet). */
export const contentPoint = (b: ScrollBox, x: number, y: number): { x: number; y: number } => ({
  x: x - b.left - b.clientLeft + b.scrollLeft,
  y: y - b.top - b.clientTop + b.scrollTop,
});

/** Kde karta v okamžiku chycení: odsazení ukazatele od jejího levého horního rohu (uvnitř karty). */
export function grabOffset(r: { left: number; top: number; width: number; height: number }, x: number, y: number): { dx: number; dy: number } {
  return { dx: Math.max(0, Math.min(r.width, x - r.left)), dy: Math.max(0, Math.min(r.height, y - r.top)) };
}
/** Levý horní roh „ducha“ pod kurzorem. */
export const flyPos = (grab: { dx: number; dy: number }, x: number, y: number): { x: number; y: number } => ({ x: x - grab.dx, y: y - grab.dy });

// ---------- FLIP ----------

export type FlipBox = { x: number; y: number };
/** Posun každé karty mezi dvěma měřeními (před → po); pod 1 px nebo bez protějšku = bez animace. */
export function flipDeltas(first: readonly FlipBox[], last: readonly FlipBox[]): ({ dx: number; dy: number } | null)[] {
  return first.map((f, k) => {
    const l = last[k];
    if (!l) return null;
    const dx = f.x - l.x;
    const dy = f.y - l.y;
    return Math.abs(dx) < 1 && Math.abs(dy) < 1 ? null : { dx, dy };
  });
}
/** Délka a křivka plynulého odsunu karet podle nastavení pohybu Ledge; „bez animací“ i systémové omezení
 *  pohybu = `null` (karty se přesunou hned). */
export function flipPlan(style: string, speed: number, reduced: boolean): { ms: number; easing: string } | null {
  if (style === "none" || reduced) return null;
  return { ms: Math.max(1, Math.round(160 / (speed > 0 ? speed : 1))), easing: "cubic-bezier(.2,.7,.2,1)" };
}

// ---------- přistání ducha ----------

export type LandBox = { x: number; y: number; w: number; h: number };
/** Transformace ducha (`left: 0; top: 0; transform-origin: 0 0`) na začátku a na konci přistání: duch o rozměrech
 *  `ghost` letí z bodu `from` (levý horní roh) na mezeru `to` a přizpůsobí se jí velikostí — karta, která
 *  v cíli vyjde širší nebo nižší, na ni tak plynule „dosedne“. Stejné rozměry = čistý posun bez `scale`. */
export function landTransforms(from: { x: number; y: number }, ghost: { w: number; h: number }, to: LandBox): { from: string; to: string } {
  const sx = ghost.w > 0 ? to.w / ghost.w : 1;
  const sy = ghost.h > 0 ? to.h / ghost.h : 1;
  const same = Math.abs(sx - 1) < 0.005 && Math.abs(sy - 1) < 0.005;
  return {
    from: `translate(${Math.round(from.x)}px, ${Math.round(from.y)}px)`,
    to: `translate(${Math.round(to.x)}px, ${Math.round(to.y)}px)${same ? "" : ` scale(${+sx.toFixed(4)}, ${+sy.toFixed(4)})`}`,
  };
}
/** Obdélník prvku ve viewportu z jeho `offset*` v rámci hostitele (`offsetParent`): nezávislý na rozběhlých
 *  animacích (FLIP) prvku i posunu hostitele. */
export function offsetRect(host: { left: number; top: number; clientLeft: number; clientTop: number; scrollLeft: number; scrollTop: number }, el: { offsetLeft: number; offsetTop: number; offsetWidth: number; offsetHeight: number }): LandBox {
  return { x: host.left + host.clientLeft - host.scrollLeft + el.offsetLeft, y: host.top + host.clientTop - host.scrollTop + el.offsetTop, w: el.offsetWidth, h: el.offsetHeight };
}

// ---------- záložky ----------

/** Zástupná „záložka“ tlačítka + (nová záložka): `dropTab` je při tahu nad ním rovno tomuhle. */
export const NEW_TAB = "+new";
/** Index nejbližšího středu (osa X) — kam v řadě záložek spadne tažená záložka. */
export function nearestCenter(centers: readonly number[], x: number): number {
  let best = 0;
  centers.forEach((c, j) => {
    if (Math.abs(x - c) < Math.abs(x - centers[best])) best = j;
  });
  return best;
}

// ---------- popisky zón při tahu ----------

export type DropHint =
  | { kind: "hide" }
  | { kind: "hidePart" }
  | { kind: "tab"; tab: string; there: boolean }
  | { kind: "newTab" }
  | { kind: "alreadyHere" }
  | null;
/** Co má říkat štítek u kurzoru při tahu karty: zóna Skrýt, ouško záložky (a že tam karta už je), tlačítko
 *  nové záložky, mřížka cizí záložky, kde už karta je. Obyčejné místo v mřížce = žádný štítek. */
export function dropHint(i: { zone: boolean; tab: string | null; tabHas: boolean; foreignHas: boolean }): DropHint {
  if (i.zone) return { kind: "hide" };
  if (i.tab === NEW_TAB) return { kind: "newTab" };
  if (i.tab) return { kind: "tab", tab: i.tab, there: i.tabHas };
  return i.foreignHas ? { kind: "alreadyHere" } : null;
}
