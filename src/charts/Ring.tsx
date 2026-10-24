// Prstenec (kolo 22): podíl 0–1 jako oblouk od 12 hodin po směru hodin + stopa celého kruhu; uprostřed volitelný
// obsah (`children`, např. „42 %“). Geometrie `geom.ts` `ring`; chybějící / nekonečné číslo = jen prázdný obal.
// Barva oblouku = `currentColor`, stopa = `--line-2`. Kreslí se jen viditelné (`useVisible`), bez animace.
import { Show, type JSX } from "solid-js";
import { ring } from "./geom";
import { useVisible } from "./visible";
import "./charts.css";

export type RingProps = {
  /** podíl 0–1 (mimo se ořízne) */
  value: number | null | undefined;
  /** průměr v px (výchozí 40) a tloušťka (výchozí 4) */
  size?: number;
  stroke?: number;
  label?: string;
  children?: JSX.Element;
};

export default function Ring(p: RingProps) {
  let el: HTMLSpanElement | undefined;
  const vis = useVisible(() => el);
  const size = () => p.size ?? 40;
  const stroke = () => p.stroke ?? 4;
  const g = () => (vis() ? ring(p.value, size(), stroke()) : null);
  const mid = () => size() / 2;
  return (
    <span
      ref={el}
      class="chart chart-ring"
      style={{ width: `${size()}px`, height: `${size()}px` }}
      role={p.label ? "img" : undefined}
      aria-label={p.label}
      aria-hidden={p.label ? undefined : "true"}
    >
      <Show when={g()}>
        {(r) => (
          <svg viewBox={`0 0 ${size()} ${size()}`} width={size()} height={size()} aria-hidden="true">
            <circle class="chart-track" cx={mid()} cy={mid()} r={r().r} stroke-width={stroke()} />
            <circle class="chart-arc" cx={mid()} cy={mid()} r={r().r} stroke-width={stroke()} stroke-dasharray={r().dash} transform={`rotate(-90 ${mid()} ${mid()})`} />
          </svg>
        )}
      </Show>
      <Show when={p.children}>
        <span class="chart-ring-in">{p.children}</span>
      </Show>
    </span>
  );
}
