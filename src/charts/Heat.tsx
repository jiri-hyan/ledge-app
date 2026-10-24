// Pás (heat, kolo 22): buňky v úrovních 0–4 (0 = nic), jeden řádek (týden, hodiny) nebo `rows` řádků po sloupcích
// (7 = týdny jako u GitHubu). Geometrie `geom.ts` `heat`; < 2 čísla = nic. Barva úrovní = `currentColor`
// s krytím podle úrovně (`charts.css`), úroveň 0 = stopa `--line-2`. Kreslí se jen viditelné, bez animace.
import { For, Show } from "solid-js";
import { heat } from "./geom";
import { useVisible } from "./visible";
import "./charts.css";

export type HeatProps = {
  values: readonly (number | null | undefined)[];
  w?: number;
  h?: number;
  rows?: number;
  gap?: number;
  /** pevné maximum (jinak největší hodnota) */
  max?: number;
  /** popis buňky (nápověda) podle indexu — např. datum a počet */
  cellTitle?: (i: number) => string;
  label?: string;
};

export default function Heat(p: HeatProps) {
  let el: HTMLSpanElement | undefined;
  const vis = useVisible(() => el);
  const w = () => p.w ?? 120;
  const h = () => p.h ?? 12;
  const cells = () => (vis() ? heat(p.values, w(), h(), { rows: p.rows, gap: p.gap, max: p.max }) : []);
  return (
    <span
      ref={el}
      class="chart chart-heat"
      style={{ width: `${w()}px`, height: `${h()}px` }}
      role={p.label ? "img" : undefined}
      aria-label={p.label}
      aria-hidden={p.label ? undefined : "true"}
    >
      <Show when={cells().length}>
        <svg viewBox={`0 0 ${w()} ${h()}`} width={w()} height={h()} aria-hidden="true">
          <For each={cells()}>
            {(c) => (
              <rect class="chart-cell" data-l={c.level} x={c.x} y={c.y} width={c.w} height={c.h} rx={Math.min(2, c.w / 3)}>
                <Show when={p.cellTitle}>{(f) => <title>{f()(c.i)}</title>}</Show>
              </rect>
            )}
          </For>
        </svg>
      </Show>
    </span>
  );
}
