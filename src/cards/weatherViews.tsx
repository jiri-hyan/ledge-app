// Vzhledy karty Počasí 1/2 (kolo 25): společné pomocné díly, Živá obloha, Teploměr dne a Plakát. Každý dostane jen
// data z Rustu (`Weather`), co chybí, vynechá — nic se nedopočítává. Pohyb jen CSS (transform / opacity), běží
// pod `.wx-live` (šuplík otevřený), viz `weather.css`.
import { For, Show } from "solid-js";
import { num } from "../format";
import { t } from "../i18n";
import type { Weather } from "../types/personal";
import { tempIn } from "./temp";
import { rangePos, skyId, tempGradient, windIndex, wxKind } from "./weatherModel";
import WxIcon from "./WxIcon";

export type VP = { d: Weather; f: boolean | undefined };

/** Teplota s jednotkovým kroužkem: „8°“. */
export const deg = (c: number, f: boolean | undefined) => `${num(tempIn(c, f))}°`;

const DIRS = () => [
  t("cards.weather.dir.0", "S"), t("cards.weather.dir.1", "SV"), t("cards.weather.dir.2", "V"), t("cards.weather.dir.3", "JV"),
  t("cards.weather.dir.4", "J"), t("cards.weather.dir.5", "JZ"), t("cards.weather.dir.6", "Z"), t("cards.weather.dir.7", "SZ"),
];

/** „pocitově 3° · vítr 11 km/h Z“ — jen co zdroj poslal. */
export function Extras(p: VP) {
  return (
    <Show when={p.d.feels != null || p.d.wind != null}>
      <div class="sub wx-extra">
        <Show when={p.d.feels != null}><span>{t("cards.weather.feels", "pocitově {t}", { t: deg(p.d.feels!, p.f) })}</span></Show>
        <Show when={p.d.wind != null}>
          <span>{t("cards.weather.wind", "vítr {v} km/h", { v: num(p.d.wind!) })}{p.d.windDir != null ? ` ${DIRS()[windIndex(p.d.windDir)]}` : ""}</span>
        </Show>
      </div>
    </Show>
  );
}

/** Dnešní min / max / srážky jako jeden řádek (jako klasické). */
export function Range(p: VP) {
  return (
    <div class="sub">
      <Show when={p.d.min != null && p.d.max != null} fallback={t("cards.weather.noToday", "dnešek bez údajů")}>{deg(p.d.min!, p.f)} / {deg(p.d.max!, p.f)}</Show>
      <Show when={p.d.precipSum != null}> · {num(p.d.precipSum!, 1)} mm</Show>
      <Show when={p.d.precipProb != null}>{t("cards.weather.precipProb", " · srážky {p} %", { p: num(p.d.precipProb!) })}</Show>
    </div>
  );
}

const range = (n: number) => Array.from({ length: n }, (_, i) => i);

/** Pozadí Živé oblohy: gradient podle `data-sky` + mraky / kapky / vločky / hvězdy. */
export function SkyBg(p: { code: number; isDay: boolean }) {
  const kind = () => wxKind(p.code) ?? "clear";
  const wet = () => ["drizzle", "rain", "storm"].includes(kind());
  return (
    <div class="wx-skybg" data-sky={skyId(p.code, p.isDay)} aria-hidden="true">
      <Show when={["partly", "cloud", "fog", "drizzle", "rain", "snow", "storm"].includes(kind())}>
        <For each={range(3)}>{(i) => <i class="wx-cloud" style={{ "--i": String(i) }} />}</For>
      </Show>
      <Show when={wet()}><For each={range(14)}>{(i) => <i class="wx-drop" style={{ "--i": String(i) }} />}</For></Show>
      <Show when={kind() === "snow"}><For each={range(12)}>{(i) => <i class="wx-flake" style={{ "--i": String(i) }} />}</For></Show>
      <Show when={!p.isDay && (kind() === "clear" || kind() === "partly")}><For each={range(10)}>{(i) => <i class="wx-star" style={{ "--i": String(i) }} />}</For></Show>
    </div>
  );
}

export function SkyBody(p: VP) {
  return (
    <div class="wx-sky">
      <span class="wx-ico-lg"><WxIcon kind={wxKind(p.d.code)} day={p.d.isDay} /></span>
      <div class="grow">
        <div class="wx-big">{deg(p.d.temp, p.f)}</div>
        <div class="name">{p.d.desc}</div>
        <Range d={p.d} f={p.f} />
        <Extras d={p.d} f={p.f} />
      </div>
    </div>
  );
}

/** Teploměr dne: pruh min–max se značkou „teď“. Bez min / max jen velká teplota a řekne to. */
export function ThermoBody(p: VP) {
  const lo = () => (p.d.min != null && p.d.max != null ? Math.min(p.d.min, p.d.temp) : null);
  const hi = () => (p.d.min != null && p.d.max != null ? Math.max(p.d.max, p.d.temp) : null);
  const pos = () => rangePos(p.d.temp, lo()!, hi()!) * 100;
  return (
    <div class="wx-thermo">
      <div class="wx-th-head">
        <span class="wx-ico-sm"><WxIcon kind={wxKind(p.d.code)} day={p.d.isDay} /></span>
        <span class="wx-big">{deg(p.d.temp, p.f)}</span>
        <span class="name">{p.d.desc}</span>
      </div>
      <Show when={lo() != null} fallback={<div class="empty">{t("cards.weather.noToday", "dnešek bez údajů")}</div>}>
        <div class="wx-th-bar" role="img" aria-label={t("cards.weather.thermoAria", "od {a} do {b}, teď {n}", { a: deg(lo()!, p.f), b: deg(hi()!, p.f), n: deg(p.d.temp, p.f) })}>
          <div class="wx-th-track" style={{ background: tempGradient(lo()!, hi()!) }} />
          <div class="wx-th-mark" style={{ left: `${pos()}%` }}><i /><span>{t("cards.weather.now", "teď")}</span></div>
        </div>
        <div class="wx-th-ends"><span>{deg(lo()!, p.f)}</span><span>{deg(hi()!, p.f)}</span></div>
      </Show>
      <Extras d={p.d} f={p.f} />
    </div>
  );
}

/** Plakát: obří teplota, popis jako titulek. */
export function PosterBody(p: VP) {
  return (
    <div class="wx-poster">
      <div class="wx-p-temp">{num(tempIn(p.d.temp, p.f))}<span>°</span></div>
      <div class="wx-p-word">{p.d.desc}</div>
      <Range d={p.d} f={p.f} />
      <Extras d={p.d} f={p.f} />
    </div>
  );
}
