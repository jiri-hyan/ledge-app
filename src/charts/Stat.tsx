// „Číslo jako hrdina“ (kolo 22): velké číslo (2,6 × tělo, tabulkové číslice), jednotka, popisek pod ním, volitelně
// druhý řádek, šipka trendu a sparkline. Jen zobrazuje, co dostane — nic nepočítá ani nevymýšlí; chybějící
// číslo (`value` null) = pomlčka „—“ a tlumený popisek (poctivý prázdný stav).
import { Show } from "solid-js";
import Sparkline from "./Sparkline";
import { trend } from "./geom";
import "./charts.css";

export type StatProps = {
  /** hlavní číslo už naformátované („42“, „1,23“); null = nic */
  value: string | null;
  /** jednotka za číslem („%“, „$“, „°C“) */
  unit?: string;
  /** popisek pod číslem („Limit 5 h“) */
  label: string;
  /** druhý, tlumený řádek („reset za 2 h“) */
  sub?: string;
  /** řada pro sparkline a šipku trendu (< 2 čísla = bez grafu i šipky) */
  series?: readonly (number | null | undefined)[];
  /** pevný rozsah sparklinu (procenta 0–100) */
  min?: number;
  max?: number;
  /** velikost: „lg“ (výchozí, 2,6 × tělo) nebo „md“ (2 × tělo) */
  size?: "md" | "lg";
};

export default function Stat(p: StatProps) {
  const dir = () => trend(p.series ?? [])?.dir ?? null;
  const arrow = (d: number) => (d > 0 ? "↑" : d < 0 ? "↓" : "→");
  return (
    <div class="chart-stat" data-size={p.size ?? "lg"} classList={{ "chart-stat-empty": p.value === null }}>
      <div class="chart-stat-top">
        <span class="chart-stat-num">
          {p.value ?? "—"}
          <Show when={p.unit && p.value !== null}>
            <small class="chart-stat-unit">{p.unit}</small>
          </Show>
        </span>
        <Show when={p.value !== null ? dir() : null}>
          {(d) => <span class="chart-stat-trend" data-dir={String(d())} aria-hidden="true">{arrow(d())}</span>}
        </Show>
      </div>
      <span class="chart-stat-label">{p.label}</span>
      <Show when={p.sub}>
        <span class="chart-stat-sub">{p.sub}</span>
      </Show>
      <Show when={p.value !== null ? p.series : null}>
        {(s) => (
          <span class="chart-stat-spark">
            <Sparkline values={s()} h={28} fluid min={p.min} max={p.max} />
          </span>
        )}
      </Show>
    </div>
  );
}
