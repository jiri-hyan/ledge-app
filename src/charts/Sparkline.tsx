// Sparkline (kolo 22): čára posledních hodnot + jemná plocha pod ní + tečka posledního bodu. Bez os a mřížky
// (mini graf 24–32 px vedle čísla). Geometrie `geom.ts` `sparkline`; < 2 čísla = nic (prázdný obal drží místo).
// Barva = `currentColor` obalu (nastav `color` / `--chart-c`), kreslí se jen viditelné (`useVisible`).
import { Show } from "solid-js";
import { sparkline } from "./geom";
import { useVisible } from "./visible";
import "./charts.css";

export type SparklineProps = {
  values: readonly (number | null | undefined)[];
  /** šířka a výška v px (výchozí 96 × 28) */
  w?: number;
  h?: number;
  /** pevný rozsah osy (např. 0–100 pro procenta); jinak z dat */
  min?: number;
  max?: number;
  /** plocha pod čarou (výchozí ano) */
  area?: boolean;
  /** tečka posledního bodu (výchozí ano) */
  dot?: boolean;
  /** roztáhnout na šířku obalu (100 %); tečka se pak nekreslí (zkreslila by se) */
  fluid?: boolean;
  /** popis pro čtečky; bez něj je graf ozdoba (`aria-hidden`) */
  label?: string;
};

export default function Sparkline(p: SparklineProps) {
  let el: HTMLSpanElement | undefined;
  const vis = useVisible(() => el);
  const w = () => p.w ?? 96;
  const h = () => p.h ?? 28;
  const g = () => (vis() ? sparkline(p.values, w(), h(), { min: p.min, max: p.max }) : null);
  return (
    <span
      ref={el}
      class="chart chart-spark"
      classList={{ "chart-fluid": !!p.fluid }}
      style={{ width: p.fluid ? undefined : `${w()}px`, height: `${h()}px` }}
      role={p.label ? "img" : undefined}
      aria-label={p.label}
      aria-hidden={p.label ? undefined : "true"}
    >
      <Show when={g()}>
        {(s) => (
          <svg viewBox={`0 0 ${w()} ${h()}`} width={p.fluid ? "100%" : w()} height={h()} preserveAspectRatio="none" aria-hidden="true">
            <Show when={p.area !== false}>
              <path class="chart-area" d={s().area} />
            </Show>
            <path class="chart-line" d={s().line} vector-effect="non-scaling-stroke" />
            <Show when={p.dot !== false && !p.fluid}>
              <circle class="chart-dot" cx={s().last.x} cy={s().last.y} r={2} />
            </Show>
          </svg>
        )}
      </Show>
    </span>
  );
}
