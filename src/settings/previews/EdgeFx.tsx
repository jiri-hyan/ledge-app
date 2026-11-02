// HTML náhled živé hrany vlastního zavřeného Ledge (kolo 28, `strip.compose.deco`, Rust `stripfx`): proměnné
// barev / délky / rychlosti a vrstva hrany (`.apv-edge` s `data-fx`). Hýbe se jen to, co by se hýbalo i v proužku
// (Claude pracuje / čeká, `always`, jednorázové naplnění, „Vyzkoušet“); `prefers-reduced-motion` a motion „none“
// = statický snímek (previews.css, blok kola 28). Data jsou ukázková.
import { Show, type JSX } from "solid-js";
import type { PreviewState } from "../appearanceModel";
import { type ComposeDeco, type LiveState, MOVING, SET_COLORS, type EdgeSet, liveEdge, spanInset, stateTone } from "../../strip/edgefx";

/** Stav náhledu („Co se děje“) → stav živé hrany (hudba / časovač = klid). */
export const liveOf = (s: PreviewState): LiveState => (s === "work" || s === "wait" || s === "done" ? s : "idle");

const TONE = { run: "var(--blue)", wait: "var(--amber)", ok: "var(--green)", off: "var(--t3)", acc: "var(--accent)" };

/** Barvy hrany jako Rust `edgefx::colors` (+ tón stavu jako `apply`). */
function colors(d: ComposeDeco, s: LiveState, tried: boolean): [string, string, string] {
  const tone = stateTone(d, s, tried);
  if (tone) {
    const c = s === "limit" ? TONE.wait : TONE[tone];
    return [c, `color-mix(in srgb, ${c} 45%, #fff)`, `color-mix(in srgb, ${c} 45%, #fff)`];
  }
  const set = SET_COLORS[d.color as EdgeSet];
  if (set) return set;
  const claude = s === "work" ? TONE.run : s === "wait" ? TONE.wait : s === "done" ? TONE.ok : TONE.off;
  const e1 = d.color === "claude" ? claude : d.color === "accent" ? TONE.acc : d.color;
  const e2 = d.color2 || `color-mix(in srgb, ${e1} 45%, #fff)`;
  return [e1, e2, d.color3 || e2];
}

/** Co se v náhledu kreslí: efekt, jestli se hýbe, proměnné CSS. `tried` = „Vyzkoušet“ (stav i bez živé hrany). */
export function edgeLook(d: ComposeDeco, s: LiveState, tried = false): { fx: string; run: boolean; vars: JSX.CSSProperties } {
  const fx = liveEdge(d, s, tried);
  const busy = s === "work" || s === "wait";
  const run = fx === "sweep" || (MOVING.includes(fx) && (busy || d.always || tried));
  const [e1, e2, e3] = colors(d, s, tried);
  const k = [1.8, 1.35, 1, 0.72, 0.5][d.speed - 1] ?? 1;
  const pk = [0.7, 1, 1.4][d.power - 1] ?? 1;
  const vars = {
    "--e1": e1,
    "--e2": e2,
    "--e3": e3,
    "--ew": `${d.width}px`,
    "--ein": `${spanInset(d)}%`,
    "--esp": `${(3.2 * k).toFixed(2)}s`,
    "--epk": String(pk),
    "--efrac": s === "limit" && fx === "dashes" ? "80%" : "100%",
  } as JSX.CSSProperties;
  return { fx, run, vars };
}

/** Vrstva hrany uvnitř těla náhledu (`.apv-sx`); záře / rozsvícení kreslí stín těla podle `data-edge`. */
export function EdgeLayer(q: { fx: string; run: boolean }) {
  return (
    <Show when={q.fx !== "none" && q.fx !== "outline" && q.fx !== "glow"}>
      <i class="apv-edge" data-fx={q.fx} data-run={q.run ? "" : undefined} aria-hidden="true" />
    </Show>
  );
}
