// Sloupce (kolo 22): hodnoty po dnech / hodinách zdola nahoru, zvýrazněný sloupec (`hi`, např. dnešek).
// Geometrie `geom.ts` `bars`; < 2 čísla = nic. Barva = `currentColor` (zvýrazněný plně, ostatní tlumeně).
// Kreslí se jen viditelné (`useVisible`), bez animace.
import { For, Show } from "solid-js";
import { bars } from "./geom";
import { useVisible } from "./visible";
import "./charts.css";

export type BarsProps = {
  values: readonly (number | null | undefined)[];
  w?: number;
  h?: number;
  gap?: number;
  /** pevné maximum osy; jinak největší hodnota */
  max?: number;
  /** index zvýrazněného sloupce (záporný = od konce: -1 = poslední, výchozí) */
  hi?: number;
  /** roztáhnout na šířku obalu (100 %) */
  fluid?: boolean;
  /** nápověda sloupce podle indexu (datum a hodnota) */
  title?: (i: number) => string;
  label?: string;
};

export default function Bars(p: BarsProps) {
  let el: HTMLSpanElement | undefined;
  const vis = useVisible(() => el);
  const w = () => p.w ?? 96;
  const h = () => p.h ?? 28;
  const rects = () => (vis() ? bars(p.values, w(), h(), { gap: p.gap, max: p.max, minH: 1 }) : []);
  const hi = () => (p.hi === undefined ? rects().length - 1 : p.hi < 0 ? rects().length + p.hi : p.hi);
  return (
    <span
      ref={el}
      class="chart chart-bars"
      classList={{ "chart-fluid": !!p.fluid }}
      style={{ width: p.fluid ? undefined : `${w()}px`, height: `${h()}px` }}
      role={p.label ? "img" : undefined}
      aria-label={p.label}
      aria-hidden={p.label ? undefined : "true"}
    >
      <Show when={rects().length}>
        <svg viewBox={`0 0 ${w()} ${h()}`} width={p.fluid ? "100%" : w()} height={h()} preserveAspectRatio={p.fluid ? "none" : undefined} aria-hidden="true">
          <For each={rects()}>
            {(r, i) => (
              <rect class="chart-bar" classList={{ "chart-hi": i() === hi() }} x={r.x} y={r.y} width={r.w} height={r.h} rx={Math.min(2, r.w / 2)}>
                <Show when={p.title}>{(f) => <title>{f()(i())}</title>}</Show>
              </rect>
            )}
          </For>
        </svg>
      </Show>
    </span>
  );
}
