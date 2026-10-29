// Pozadí otevřeného šuplíku (kolo 25): vrstvy gradient → (živý canvas) → závoj hned za kartami. Nevykreslí nic,
// dokud není `appearance.backdrop` nastavené. Canvas kreslí JEN když je šuplík otevřený, okno viditelné a
// (bez „omezit pohyb“) — jinak smyčka stojí a nic není naplánované; „omezit pohyb“ = jeden statický snímek.
// Rozhodnutí je čistá funkce `drawMode` (`backdrop.ts`), smyčka `backdropLoop.ts`, kreslení `backdropLive.ts`.
import { Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import { backdropNow } from "./apply";
import { drawMode } from "./backdrop";
import { createLoop } from "./backdropLoop";
import { LIVE_FPS, STILL_WARMUP, createLive } from "./backdropLive";
import "./backdrop.css";

const reducedQuery = typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;
const [reduced, setReduced] = createSignal(reducedQuery?.matches ?? false);
const [hidden, setHidden] = createSignal(typeof document !== "undefined" && document.visibilityState === "hidden");
reducedQuery?.addEventListener("change", () => setReduced(reducedQuery.matches));
document.addEventListener("visibilitychange", () => setHidden(document.visibilityState === "hidden"));

/** `open` = šuplík je celý otevřený (`view() === "full"`). */
export default function Backdrop(p: { open: () => boolean }) {
  const bd = backdropNow;
  const live = createMemo(() => bd()?.live ?? "");
  const intensity = createMemo(() => bd()?.intensity ?? 0.6);
  const speed = createMemo(() => bd()?.speed ?? 1);
  const flame = createMemo(() => bd()?.flame ?? "classic");
  const [cv, setCv] = createSignal<HTMLCanvasElement>();
  const mode = createMemo(() => drawMode({ open: p.open(), hidden: hidden(), reduced: reduced(), live: !!live() }));
  const [size, setSize] = createSignal<[number, number]>([0, 0]);

  createEffect(() => {
    const el = cv();
    if (!el || !live()) return;
    const ro = new ResizeObserver(() => {
      const w = Math.max(2, Math.round(el.clientWidth / 2)), h = Math.max(2, Math.round(el.clientHeight / 2));
      if (w !== size()[0] || h !== size()[1]) setSize([w, h]);
    });
    ro.observe(el);
    onCleanup(() => ro.disconnect());
  });

  createEffect(() => {
    const kind = live() as Exclude<ReturnType<typeof live>, "">;
    const m = mode();
    const [w, h] = size();
    const el = cv();
    if (!el || !kind || m === "stop" || w < 2) return;
    el.width = w;
    el.height = h;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    const u = createLive(kind, ctx, w, h, { intensity: intensity(), speed: speed(), flame: flame() });
    if (m === "still") {
      for (let i = 0; i <= STILL_WARMUP[kind]; i++) u.frame(5 + i / 30, 1 / 30);
      return;
    }
    const loop = createLoop({ now: () => performance.now(), request: (cb) => requestAnimationFrame(cb), cancel: (hd) => cancelAnimationFrame(hd), frame: (t, dt) => u.frame(t, dt) }, LIVE_FPS[kind]);
    loop.start();
    onCleanup(() => loop.stop());
  });

  return (
    <Show when={bd()}>
      <div class="bd" aria-hidden="true">
        <div class="bd-g" />
        <Show when={live()}>
          <canvas ref={setCv} />
        </Show>
        <div class="bd-v" />
      </div>
    </Show>
  );
}
