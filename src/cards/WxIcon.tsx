// Vlastní ikony počasí jako inline SVG (kolo 25) — žádné emoji, barvy z tokenů (`--amber`, `--blue` …) přes třídy
// v `weather.css`. Druh počasí určuje `weatherModel.wxKind`; neznámý kód (kind = null) = otazník v kroužku.
import { Match, Show, Switch } from "solid-js";
import type { WxKind } from "./weatherModel";

const CLOUD = "M7 18.5h9.5a3.5 3.5 0 0 0 .4-6.97A5 5 0 0 0 7.3 11 3.8 3.8 0 0 0 7 18.5Z";

function Sun() {
  return (
    <g class="wxi-sun">
      <circle cx="12" cy="12" r="4" fill="currentColor" stroke="none" />
      <path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4" />
    </g>
  );
}

function Moon(p: { x?: number; y?: number; s?: number }) {
  return <path class="wxi-moon" fill="currentColor" stroke="none" transform={`translate(${p.x ?? 0} ${p.y ?? 0}) scale(${p.s ?? 1})`} d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z" />;
}

/** Ikona 24×24; velikost nastaví CSS (`width`/`height` rodiče), barva z `color` + tříd `wxi-*`. */
export default function WxIcon(p: { kind: WxKind | null; day: boolean; title?: string }) {
  return (
    <svg class="wxi" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role={p.title ? "img" : undefined} aria-label={p.title} aria-hidden={p.title ? undefined : "true"}>
      <Switch fallback={<g><circle cx="12" cy="12" r="8" /><path d="M9.7 9.6a2.4 2.4 0 1 1 3.4 2.2c-.7.4-1.1.9-1.1 1.7M12 16.6v.1" /></g>}>
        <Match when={p.kind === "clear"}>
          <Show when={p.day} fallback={<Moon />}><Sun /></Show>
        </Match>
        <Match when={p.kind === "partly"}>
          <Show when={p.day} fallback={<Moon x={-3} y={-3} s={0.85} />}>
            <g transform="translate(-4 -4) scale(0.8)"><Sun /></g>
          </Show>
          <path class="wxi-cloud" d={CLOUD} fill="var(--card, #222)" transform="translate(1.5 1)" />
        </Match>
        <Match when={p.kind === "cloud"}>
          <path class="wxi-cloud" d={CLOUD} />
        </Match>
        <Match when={p.kind === "fog"}>
          <path class="wxi-cloud" d="M7 14h9.5a3.2 3.2 0 0 0 .3-6.4A4.6 4.6 0 0 0 7.4 8 3.2 3.2 0 0 0 7 14Z" />
          <path class="wxi-fog" d="M5 17.5h14M8 20.5h8" />
        </Match>
        <Match when={p.kind === "drizzle"}>
          <path class="wxi-cloud" d="M7 14.5h9.5a3.3 3.3 0 0 0 .3-6.6A4.7 4.7 0 0 0 7.4 8.4 3.3 3.3 0 0 0 7 14.5Z" />
          <path class="wxi-rain" d="M9 17.5v.5M12 18.5v.5M15 17.5v.5M10.5 20.5v.5M13.5 20.5v.5" />
        </Match>
        <Match when={p.kind === "rain"}>
          <path class="wxi-cloud" d="M7 14.5h9.5a3.3 3.3 0 0 0 .3-6.6A4.7 4.7 0 0 0 7.4 8.4 3.3 3.3 0 0 0 7 14.5Z" />
          <path class="wxi-rain" d="M8.5 17l-1 3M12.5 17l-1 3M16.5 17l-1 3" />
        </Match>
        <Match when={p.kind === "snow"}>
          <path class="wxi-cloud" d="M7 14.5h9.5a3.3 3.3 0 0 0 .3-6.6A4.7 4.7 0 0 0 7.4 8.4 3.3 3.3 0 0 0 7 14.5Z" />
          <path class="wxi-snow" d="M8.5 18v.1M12 19.5v.1M15.5 18v.1M10 21v.1M14 21v.1" stroke-width="2.4" />
        </Match>
        <Match when={p.kind === "storm"}>
          <path class="wxi-cloud" d="M7 14h9.5a3.3 3.3 0 0 0 .3-6.6A4.7 4.7 0 0 0 7.4 7.9 3.3 3.3 0 0 0 7 14Z" />
          <path class="wxi-bolt" d="M12.5 13.5 10 18h3l-1.5 3.5" />
        </Match>
      </Switch>
    </svg>
  );
}
