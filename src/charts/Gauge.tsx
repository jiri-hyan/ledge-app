// Budík (kolo 25): půlkruh 0–100 % s ručičkou, volitelnou ryskou plánu a obsahem uprostřed dole (`children`).
// Geometrie `dial.ts`; šířka podle obalu (viewBox, žádné pevné px), kreslí se jen viditelné (`useVisible`).
// Ručička se přesouvá jen `transform` při změně hodnoty (v `prefers-reduced-motion` skokem), nic neběží v klidu.
// `title` = tooltip celého budíku (hodnota a čas); bez čísla (`frac` null) je jen stopa a ručička stojí vlevo.
import { Show, type JSX } from "solid-js";
import { dialDeg, dialPath, frac01, needleEnd } from "./dial";
import { useVisible } from "./visible";
import "./charts.css";

export type GaugeProps = {
  /** podíl 0–1; null = bez údaje */
  frac: number | null | undefined;
  /** ryska plánu 0–1 (kde bys měl být podle času) */
  plan?: number | null;
  tone?: "ok" | "warn" | "bad";
  title?: string;
  label?: string;
  children?: JSX.Element;
};

const CX = 50;
const CY = 50;
const R = 40;

export default function Gauge(p: GaugeProps) {
  let el: HTMLDivElement | undefined;
  const vis = useVisible(() => el);
  const f = () => frac01(p.frac);
  const planAt = () => {
    const v = frac01(p.plan);
    return v === null ? null : { a: needleEnd(CX, CY, R - 7, v), b: needleEnd(CX, CY, R + 5, v) };
  };
  return (
    <div ref={el} class="chart-gauge" data-tone={p.tone ?? "ok"} title={p.title} role={p.label ? "img" : undefined} aria-label={p.label ?? p.title} aria-hidden={p.label || p.title ? undefined : "true"}>
      <Show when={vis()}>
        <svg viewBox="0 0 100 58" aria-hidden="true">
          <path class="chart-track" d={dialPath(CX, CY, R, 0, 1)} stroke-width="8" stroke-linecap="round" />
          <Show when={f() !== null && f()! > 0}>
            <path class="chart-gauge-arc" d={dialPath(CX, CY, R, 0, f()!)} stroke-width="8" stroke-linecap="round" />
          </Show>
          <Show when={planAt()}>{(pl) => <line class="chart-gauge-plan" x1={pl().a.x} y1={pl().a.y} x2={pl().b.x} y2={pl().b.y} />}</Show>
          <Show when={f() !== null}>
            <g class="chart-gauge-needle" style={{ transform: `rotate(${dialDeg(f()!)}deg)`, "transform-origin": `${CX}px ${CY}px` }}>
              {/* s číslem uprostřed (children) ručička začíná nad ním, ať přes text nevede (kolo 25) */}
              <line x1={CX} y1={p.children ? CY - 19 : CY} x2={CX} y2={CY - (p.children ? R - 3 : R - 12)} />
              <Show when={!p.children}><circle cx={CX} cy={CY} r="3" /></Show>
            </g>
          </Show>
        </svg>
      </Show>
      <Show when={p.children}>
        <div class="chart-gauge-in">{p.children}</div>
      </Show>
    </div>
  );
}
