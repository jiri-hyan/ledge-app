// Vzhledy karty Týden (kolo 25) kromě výchozího pásu dnů: Měsíc, Odtrhávací list, Osa dne, Ciferník, Program.
// Data jen z kalendáře v paměti (`CalendarView`); bez kalendáře těla ukážou jen to, co umí z hodin (datum), a pod
// nimi karta řekne proč (stejná poznámka s „Připojit“ jako u pásu). Žádný časovač — překresluje tik karty.
import { For, Show } from "solid-js";
import { fmtClock, dayNames } from "../format";
import { t, tp } from "../i18n";
import type { CalendarItem, CalendarView } from "../types/calendar";
import { dowShort, monthShort } from "./text";
import { allDayToday, countdown, dialAngle, dayMinutes, monthGrid, nextEvents, nowPos, polar, remainingToday, timedToday, timelineBlocks, wedgePath, type Countdown } from "./weekModel";

export type WP = { cal: CalendarView | null | undefined; now: number };

const untitled = () => t("cards.board.week.untitled", "(bez názvu)");
const range = (n: number, f = 0) => Array.from({ length: n }, (_, i) => i + f);

/** „za 25 min“ / „za 2 h“ / „za 3 dny“ / „právě teď“. */
export function cdText(c: Countdown): string {
  switch (c.kind) {
    case "now": return t("cards.board.week.cdNow", "právě teď");
    case "min": return t("cards.board.week.cdMin", "za {n} min", { n: c.n });
    case "hour": return t("cards.board.week.cdHour", "za {n} h", { n: c.n });
    case "day": return tp("cards.board.week.cdDay", c.n, ["za {n} den", "za {n} dny", "za {n} dní"]);
  }
}

/** „zbývá 2 události“ / „dnes už nic“. */
export const leftText = (n: number) =>
  n === 0 ? t("cards.board.week.nothingLeft", "dnes už nic") : tp("cards.board.week.left", n, ["zbývá {n} událost", "zbývají {n} události", "zbývá {n} událostí"]);

export function MonthBody(p: WP) {
  const grid = () => monthGrid(p.now, p.cal);
  const d = () => new Date(p.now);
  return (
    <div class="bd-mon">
      <div class="bd-mon-head"><span class="bd-month">{monthShort()[d().getMonth()]}</span><span class="bd-mon-year">{d().getFullYear()}</span></div>
      <div class="bd-mon-grid" role="grid">
        <For each={[1, 2, 3, 4, 5, 6, 0]}>{(i) => <span class="bd-mon-dow" classList={{ "bd-weekend": i === 0 || i === 6 }}>{dowShort()[i]}</span>}</For>
        <For each={grid().flat()}>
          {(c) => (
            <span class="bd-mon-cell" role="gridcell" classList={{ out: !c.inMonth, "bd-today": c.today, "bd-weekend": c.weekend }} aria-current={c.today ? "date" : undefined}>
              {c.day}
              <i class="bd-dot" classList={{ on: c.busy }} />
            </span>
          )}
        </For>
      </div>
    </div>
  );
}

export function TearoffBody(p: WP) {
  const d = () => new Date(p.now);
  const n = () => remainingToday(p.cal, p.now);
  return (
    <div class="bd-tear">
      <div class="bd-tear-top">{monthShort()[d().getMonth()]} {d().getFullYear()}</div>
      <div class="bd-tear-day">{d().getDate()}</div>
      <div class="bd-tear-dow">{dayNames()[d().getDay()]}</div>
      <Show when={p.cal && p.cal.status !== "off" && p.cal.status !== "unset"}>
        <div class="bd-tear-left">{leftText(n())}</div>
      </Show>
    </div>
  );
}

const HOUR_MARKS = [7, 10, 13, 16, 19, 22];

export function TimelineBody(p: WP) {
  const tl = () => timelineBlocks(timedToday(p.cal), 7, 22);
  const pos = () => nowPos(p.now, 7, 22);
  const lane = () => `${100 / tl().lanes}%`;
  return (
    <div class="bd-tl">
      <div class="bd-tl-axis" style={{ "--lanes": String(tl().lanes) }}>
        <For each={HOUR_MARKS}>{(h) => <i class="bd-tl-tick" style={{ left: `${((h - 7) / 15) * 100}%` }} />}</For>
        <For each={tl().blocks}>
          {(b) => (
            <span class="bd-tl-blk" classList={{ on: b.item.ongoing }} style={{ left: `${b.left}%`, width: `${b.width}%`, top: `${b.lane * (100 / tl().lanes)}%`, height: `calc(${lane()} - 2px)` }} title={`${fmtClock(new Date(b.item.start))} ${b.item.title}`}>
              <span>{b.item.title || untitled()}</span>
            </span>
          )}
        </For>
        <Show when={pos() != null}><i class="bd-tl-now" style={{ left: `${pos()}%` }} /></Show>
      </div>
      <div class="bd-tl-labels"><For each={HOUR_MARKS}>{(h) => <span style={{ left: `${((h - 7) / 15) * 100}%` }}>{h}</span>}</For></div>
      <Show when={tl().blocks.length > 0}>
        <ul class="bd-tl-list">
          <For each={tl().blocks.slice(0, 5)}>
            {(b) => <li classList={{ on: b.item.ongoing }}><time>{fmtClock(new Date(b.item.start))}</time><span>{b.item.title || untitled()}</span></li>}
          </For>
        </ul>
      </Show>
      <Show when={tl().outside > 0}><div class="bd-note">{tp("cards.board.week.outside", tl().outside, ["+{n} událost mimo osu", "+{n} události mimo osu", "+{n} událostí mimo osu"])}</div></Show>
      <Show when={allDayToday(p.cal).length > 0}><div class="bd-note">{t("cards.board.week.allDay", "celý den")}: {allDayToday(p.cal).map((x) => x.title || untitled()).join(", ")}</div></Show>
    </div>
  );
}

export function DialBody(p: WP) {
  const items = () => timedToday(p.cal);
  const hand = () => polar(60, 60, 44, dialAngle(dayMinutes(p.now)));
  return (
    <div class="bd-dial">
      <svg viewBox="0 0 120 120" role="img" aria-label={t("cards.board.week.dialAria", "Ciferník dne s událostmi")}>
        <circle class="bd-dial-ring" cx="60" cy="60" r="48" />
        <For each={range(24)}>
          {(h) => {
            const a = polar(60, 60, 49, dialAngle(h * 60));
            const b = polar(60, 60, h % 6 === 0 ? 42 : 45, dialAngle(h * 60));
            return <line class="bd-dial-tick" x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
          }}
        </For>
        <For each={[0, 6, 12, 18]}>{(h) => { const q = polar(60, 60, 36, dialAngle(h * 60)); return <text class="bd-dial-num" x={q.x} y={q.y} text-anchor="middle" dominant-baseline="central">{h}</text>; }}</For>
        <For each={items()}>
          {(it: CalendarItem) => {
            const s = dayMinutes(it.start);
            const e = Math.min(1440, s + Math.round((it.end - it.start) / 60_000));
            return <path class="bd-dial-wedge" classList={{ on: it.ongoing }} d={wedgePath(60, 60, 49, 56, dialAngle(s), dialAngle(e))}><title>{`${fmtClock(new Date(it.start))} ${it.title}`}</title></path>;
          }}
        </For>
        <line class="bd-dial-hand" x1="60" y1="60" x2={hand().x} y2={hand().y} />
        <circle class="bd-dial-pin" cx="60" cy="60" r="2.5" />
      </svg>
      <div class="bd-dial-side">
        <div class="bd-wk-time">{fmtClock(new Date(p.now))}</div>
        <div class="bd-note">{leftText(remainingToday(p.cal, p.now))}</div>
      </div>
    </div>
  );
}

export function AgendaBody(p: WP) {
  const list = () => nextEvents(p.cal, p.now, 4);
  const cd = (it: CalendarItem) => (it.allDay ? t("cards.board.week.allDay", "celý den") : cdText(countdown(it.start, p.now)));
  return (
    <Show when={list().length > 0} fallback={<Show when={p.cal && p.cal.status === "ok"}><div class="bd-note bd-calm">{t("cards.board.week.nothing", "Nic na dnešek")}</div></Show>}>
      <div class="bd-ag">
        <For each={list()}>
          {(it) => (
            <div class="bd-ag-row" classList={{ on: it.ongoing }}>
              <span class="bd-ag-cd">{cd(it)}</span>
              <span class="bd-ag-n" title={it.location || it.title}>{it.title || untitled()}</span>
              <Show when={!it.allDay}><span class="bd-ag-t">{fmtClock(new Date(it.start))}</span></Show>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}
