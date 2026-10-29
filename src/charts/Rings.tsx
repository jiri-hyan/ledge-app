// Soustředné prstence (kolo 25): až 4 podíly 0–1, vnější první; uprostřed `children`. Geometrie `dial.ts`
// (`concentric`, `ringDash`), šířka podle obalu (viewBox 100×100, žádné pevné px), kreslí se jen viditelné.
// Každý prstenec má vlastní tooltip (`title`: hodnota a čas) jako SVG `<title>`; bez čísla (null) jen stopa.
import { For, Show, type JSX } from "solid-js";
import { concentric, ringDash } from "./dial";
import { useVisible } from "./visible";
import "./charts.css";

export type RingSpec = { frac: number | null | undefined; tone?: "ok" | "warn" | "bad"; title?: string };

const SIZE = 100;
const STROKE = 8;
const GAP = 3.5;

export default function Rings(p: { rings: readonly RingSpec[]; label?: string; children?: JSX.Element }) {
  let el: HTMLDivElement | undefined;
  const vis = useVisible(() => el);
  const radii = () => concentric(p.rings.length, SIZE, STROKE, GAP);
  return (
    <div ref={el} class="chart-rings" role={p.label ? "img" : undefined} aria-label={p.label} aria-hidden={p.label ? undefined : "true"}>
      <Show when={vis()}>
        <svg viewBox={`0 0 ${SIZE} ${SIZE}`} aria-hidden="true">
          <For each={radii()}>
            {(r, i) => {
              const spec = () => p.rings[i()];
              const d = () => ringDash(r, spec().frac);
              return (
                <g class="chart-rings-g" data-tone={spec().tone ?? "ok"} data-ring={i()}>
                  <Show when={spec().title}><title>{spec().title}</title></Show>
                  <circle class="chart-track" cx={SIZE / 2} cy={SIZE / 2} r={r} stroke-width={STROKE} />
                  <Show when={d()}>{(x) => <circle class="chart-rings-arc" cx={SIZE / 2} cy={SIZE / 2} r={r} stroke-width={STROKE} stroke-linecap="round" stroke-dasharray={x().dash} transform={`rotate(-90 ${SIZE / 2} ${SIZE / 2})`} />}</Show>
                </g>
              );
            }}
          </For>
        </svg>
      </Show>
      <Show when={p.children}>
        <div class="chart-rings-in">{p.children}</div>
      </Show>
    </div>
  );
}
