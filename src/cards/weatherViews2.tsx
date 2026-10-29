// Vzhledy karty Počasí 2/2 (kolo 25): Křivka (hodinová teplota + srážky, hover), Pět dní (pruhy min–max jako
// v telefonu) a Oblouk slunce (originální). Data jen z `Weather` (hodiny, dny, východ / západ); co chybí, ukáže
// se prázdným textem. Žádný časovač — hover je jen signál z pointer událostí.
import { For, Show, createSignal } from "solid-js";
import { num } from "../format";
import { t } from "../i18n";
import { dowShort } from "../board/text";
import { Extras, deg, type VP } from "./weatherViews";
import { clockOf, curvePoints, dayRows, hoursFrom, nearestIndex, rangePos, sunPos, tempGradient, wxKind } from "./weatherModel";
import WxIcon from "./WxIcon";

const HOURS_SHOWN = 24;
const W = 240;
const H = 84;
/** Teplotní křivka zabírá horní část grafu, sloupky srážek spodní. */
const TOP = 8;
const BOT = 50;

export function CurveBody(p: VP) {
  const hours = () => hoursFrom(p.d.hours, p.d.observed, HOURS_SHOWN);
  const pts = () => curvePoints(hours().map((h) => h.temp!));
  const [hover, setHover] = createSignal<number | null>(null);
  const xy = (i: number) => {
    const q = pts()![i];
    return { x: q.x * W, y: TOP + q.y * (BOT - TOP) };
  };
  const line = () => pts()!.map((_, i) => `${i ? "L" : "M"}${xy(i).x.toFixed(1)} ${xy(i).y.toFixed(1)}`).join(" ");
  const area = () => `${line()} L${W} ${BOT + 6} L0 ${BOT + 6} Z`;
  const bars = () => hours().map((h, i) => ({ x: pts()![i].x * W, h: ((h.precipProb ?? 0) / 100) * (H - BOT - 8) }));
  const onMove = (e: PointerEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (r.width > 0) setHover(nearestIndex((e.clientX - r.left) / r.width, hours().length));
  };
  const cur = () => (hover() != null ? hours()[hover()!] : null);
  return (
    <div class="wx-curve">
      <div class="wx-cv-head">
        <span class="wx-ico-sm"><WxIcon kind={wxKind(p.d.code)} day={p.d.isDay} /></span>
        <span class="wx-big">{deg(p.d.temp, p.f)}</span>
        <span class="name">{p.d.desc}</span>
      </div>
      <Show when={pts()} fallback={<div class="empty">{t("cards.weather.noHours", "Hodinová předpověď není k dispozici.")}</div>}>
        <div class="wx-cv-chart" onPointerMove={onMove} onPointerLeave={() => setHover(null)} role="img" aria-label={t("cards.weather.curveAria", "Teplota v příštích hodinách")}>
          <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden="true">
            <For each={bars()}>{(b) => <rect class="wx-cv-bar" x={b.x - 3} y={H - b.h} width="6" height={b.h} />}</For>
            <path class="wx-cv-area" d={area()} />
            <path class="wx-cv-line" d={line()} vector-effect="non-scaling-stroke" />
          </svg>
          <Show when={hover() != null}>
            <i class="wx-cv-cursor" style={{ left: `${pts()![hover()!].x * 100}%` }} />
            <i class="wx-cv-dot" style={{ left: `${pts()![hover()!].x * 100}%`, top: `${(xy(hover()!).y / H) * 100}%` }} />
            <div class="wx-cv-tip" style={{ left: `${Math.min(82, Math.max(18, pts()![hover()!].x * 100))}%` }}>
              {+cur()!.time.slice(11, 13)}:00 · {deg(cur()!.temp!, p.f)}
              <Show when={cur()!.precipProb != null}> · {t("cards.weather.precipShort", "srážky {p} %", { p: num(cur()!.precipProb!) })}</Show>
            </div>
          </Show>
        </div>
        <div class="wx-cv-axis">
          <For each={hours()}>
            {(h, i) => (
              <Show when={i() % 6 === 0}>
                <span style={{ left: `${pts()![i()].x * 100}%` }}>{+h.time.slice(11, 13)}</span>
              </Show>
            )}
          </For>
        </div>
      </Show>
      <Extras d={p.d} f={p.f} />
    </div>
  );
}

export function WeekBody(p: VP) {
  const rows = () => dayRows(p.d);
  const lo = () => Math.min(...rows().map((r) => r.min));
  const hi = () => Math.max(...rows().map((r) => r.max));
  const dow = (date: string) => {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(date);
    return m ? dowShort()[new Date(Date.UTC(+m[1], +m[2] - 1, +m[3])).getUTCDay()] : "";
  };
  return (
    <Show when={rows().length > 0} fallback={<div class="empty">{t("cards.weather.noDays", "Předpověď na další dny není k dispozici.")}</div>}>
      <div class="wx-days">
        <For each={rows()}>
          {(r) => (
            <div class="wx-day" classList={{ today: r.today }}>
              <span class="wx-day-n">{r.today ? t("cards.weather.today", "Dnes") : dow(r.date)}</span>
              <span class="wx-ico-xs"><WxIcon kind={wxKind(r.code)} day={true} /></span>
              <span class="wx-day-lo">{deg(r.min, p.f)}</span>
              <span class="wx-day-track">
                <i style={{ left: `${rangePos(r.min, lo(), hi()) * 100}%`, width: `${Math.max(4, (rangePos(r.max, lo(), hi()) - rangePos(r.min, lo(), hi())) * 100)}%`, background: tempGradient(r.min, r.max) }} />
                <Show when={r.today}><b style={{ left: `${rangePos(p.d.temp, lo(), hi()) * 100}%` }} title={t("cards.weather.now", "teď")} /></Show>
              </span>
              <span class="wx-day-hi">{deg(r.max, p.f)}</span>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}

const CX = 100;
const CY = 92;
const R = 80;

export function ArcBody(p: VP) {
  const sun = () => sunPos(p.d.sunrise, p.d.sunset, p.d.observed);
  const at = (pr: number) => ({ x: CX - R * Math.cos(Math.PI * pr), y: CY - R * Math.sin(Math.PI * pr) });
  const dot = () => at(sun()!.p);
  return (
    <div class="wx-arc">
      <Show when={sun()} fallback={<div class="empty">{t("cards.weather.noSun", "Východ a západ slunce nejsou k dispozici.")}</div>}>
        <svg viewBox="0 0 200 104" aria-hidden="true">
          <path class="wx-arc-base" d={`M${CX - R} ${CY} A${R} ${R} 0 0 1 ${CX + R} ${CY}`} pathLength="100" />
          <path class="wx-arc-done" d={`M${CX - R} ${CY} A${R} ${R} 0 0 1 ${CX + R} ${CY}`} pathLength="100" stroke-dasharray={`${sun()!.p * 100} 101`} />
          <line class="wx-arc-hz" x1="8" y1={CY} x2="192" y2={CY} />
          <circle class="wx-arc-glow" cx={dot().x} cy={dot().y} r="11" classList={{ off: sun()!.phase !== "day" }} />
          <circle class="wx-arc-sun" cx={dot().x} cy={dot().y} r="6" classList={{ off: sun()!.phase !== "day" }} />
        </svg>
        <div class="wx-arc-times">
          <span>{t("cards.weather.sunrise", "východ {at}", { at: clockOf(p.d.sunrise) })}</span>
          <span>{t("cards.weather.sunset", "západ {at}", { at: clockOf(p.d.sunset) })}</span>
        </div>
      </Show>
      <div class="wx-arc-now">
        <span class="wx-big">{deg(p.d.temp, p.f)}</span>
        <span class="wx-ico-sm"><WxIcon kind={wxKind(p.d.code)} day={p.d.isDay} /></span>
        <span class="name">{p.d.desc}</span>
      </div>
      <Show when={sun() && sun()!.phase !== "day"}>
        <div class="sub">{sun()!.phase === "before" ? t("cards.weather.sunBefore", "Slunce vyjde v {at}", { at: clockOf(p.d.sunrise) }) : t("cards.weather.sunAfter", "Slunce zapadlo v {at}", { at: clockOf(p.d.sunset) })}</div>
      </Show>
      <Show when={p.d.min != null && p.d.max != null}>
        <div class="sub">{deg(p.d.min!, p.f)} / {deg(p.d.max!, p.f)}</div>
      </Show>
      <Extras d={p.d} f={p.f} />
    </div>
  );
}
