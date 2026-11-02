// Sekce lišty notche „Dny a čas“ (kolo 27) ve vzhledech z `designs.ts`: opravený Pás týdne (výchozí — datum a
// čas nahoře v jedné řadě, pod nimi týden, dnešek decentně akcentem, víkend tlumeně), List kalendáře, Velký čas,
// Týden s tečkami schůzek (jen dny, které kalendář v paměti posílá; bez kalendáře bez teček) a Analogové hodiny
// (SVG; vteřinová ručička jen jako volba — čas tiká z App jen s otevřeným šuplíkem, žádný vlastní časovač).
import { For, Match, Show, Switch } from "solid-js";
import { t } from "../i18n";
import { dayNames, fmtClock, fmtDate } from "../format";
import type { CalendarView } from "../types/calendar";
import { weekStrip as busyWeek } from "../board/model";
import { weekStrip } from "./model";
import { dotsOf, handAngles, nextMeeting, solidOf } from "./look";
import { todayLeft } from "../board/model";
import { calColor, hm } from "../calendar/model";
import { MeetLine } from "./SecInfo";
import type { BarSection } from "./bar";

export const MONTHS = () => [
  t("cards.notch.month.jan", "led"), t("cards.notch.month.feb", "úno"), t("cards.notch.month.mar", "bře"), t("cards.notch.month.apr", "dub"),
  t("cards.notch.month.may", "kvě"), t("cards.notch.month.jun", "čvn"), t("cards.notch.month.jul", "čvc"), t("cards.notch.month.aug", "srp"),
  t("cards.notch.month.sep", "zář"), t("cards.notch.month.oct", "říj"), t("cards.notch.month.nov", "lis"), t("cards.notch.month.dec", "pro"),
];
/** Písmena dnů od neděle (`Date.getDay()`). */
const DOW = () => [
  t("cards.notch.dow.sun", "Ne"), t("cards.notch.dow.mon", "Po"), t("cards.notch.dow.tue", "Út"), t("cards.notch.dow.wed", "St"),
  t("cards.notch.dow.thu", "Čt"), t("cards.notch.dow.fri", "Pá"), t("cards.notch.dow.sat", "So"),
];

/** Řada dnů týdne; `busy` (vzhled s tečkami) = den s událostí v kalendáři. */
function Week(p: { now: number; cal?: CalendarView | null; dots?: boolean }) {
  const days = () => {
    const d = new Date(p.now);
    if (!p.dots) return weekStrip(d).map((w) => ({ ...w, busy: 0 }));
    return busyWeek(p.now, p.cal).map((w) => ({ date: w.day, dow: w.dow, today: w.today, weekend: w.weekend, busy: dotsOf(p.cal, w.date) }));
  };
  return (
    <div class="nb-week">
      <For each={days()}>
        {(w) => (
          <span class="nb-day" classList={{ "nb-today": w.today, "nb-weekend": w.weekend, "nb-dotted": p.dots }}>
            <i>{DOW()[w.dow]}</i>
            <b>{w.date}</b>
            <Show when={p.dots}><span class="nb-dots" aria-hidden="true"><For each={Array.from({ length: w.busy }, (_, i) => i)}>{() => <u />}</For></span></Show>
          </span>
        )}
      </For>
    </div>
  );
}

function Analog(p: { now: number; seconds: boolean; color?: string }) {
  const a = () => handAngles(new Date(p.now));
  const hand = (deg: number, len: number, w: number) => (
    <line class="nb-hand" x1="50" y1="50" x2="50" y2={50 - len} stroke-width={w} stroke-linecap="round" transform={`rotate(${deg} 50 50)`} />
  );
  return (
    <svg class="nb-analog" viewBox="0 0 100 100" role="img" aria-label={fmtClock(new Date(p.now))}>
      <circle cx="50" cy="50" r="46" class="nb-face" />
      <For each={Array.from({ length: 12 }, (_, i) => i)}>
        {(i) => <line x1="50" y1="8" x2="50" y2={i % 3 ? 12 : 15} class="nb-tick" transform={`rotate(${i * 30} 50 50)`} />}
      </For>
      {hand(a().h, 24, 4.5)}
      {hand(a().m, 36, 3)}
      <Show when={p.seconds}>
        <line x1="50" y1="58" x2="50" y2="10" stroke-width="1.2" style={{ stroke: solidOf(p.color) }} transform={`rotate(${a().s} 50 50)`} />
      </Show>
      <circle cx="50" cy="50" r="3" style={{ fill: solidOf(p.color) }} />
    </svg>
  );
}

export default function Days(p: { sec: BarSection; design: string; now: number; cal?: CalendarView | null }) {
  const d = () => new Date(p.now);
  const accent = () => (p.sec.color ? { "--nb-c": solidOf(p.sec.color) } : {});
  return (
    <section class="nb-sec nb-days" data-design={p.design} style={accent()} aria-label={t("cards.clock.title", "Čas")}>
      <Switch>
        <Match when={p.design === "sheet"}>
          <div class="nb-sheet">
            <i>{dayNames()[d().getDay()]}</i>
            <b>{d().getDate()}</b>
            <small>{MONTHS()[d().getMonth()]} · {fmtClock(d())}</small>
          </div>
          <div class="nb-agenda">
            <For each={todayLeft(p.cal, p.now, 3)} fallback={<small class="nb-empty">{p.cal?.days.length ? t("cards.notch.dayFree", "Dnes už nic v kalendáři.") : ""}</small>}>
              {(it) => (
                <div class="nb-ag" style={{ "--nb-cal": calColor(p.cal?.calendars[it.cal]?.color, it.cal) }}>
                  <b class="nb-title">{it.title}</b>
                  <small classList={{ "nb-agnow": it.ongoing || (it.start <= p.now && !it.allDay) }}>
                    {it.allDay ? t("cards.board.week.allDay", "celý den") : it.ongoing || it.start <= p.now ? t("cards.notch.now", "Právě teď") : `${hm(it.start)}–${hm(it.end)}`}
                  </small>
                </div>
              )}
            </For>
          </div>
        </Match>
        <Match when={p.design === "clock"}>
          <div class="nb-bigtime">{fmtClock(d())}</div>
          <div class="nb-date">{fmtDate(d())}</div>
        </Match>
        <Match when={p.design === "analog"}>
          <Analog now={p.now} seconds={!!p.sec.seconds} color={p.sec.color} />
          <div class="nb-dcol">
            <div class="nb-date">{fmtDate(d())}</div>
            <div class="nb-time">{fmtClock(d())}</div>
          </div>
        </Match>
        <Match when={true}>
          <div class="nb-dhead">
            <span class="nb-date">{fmtDate(d())}</span>
            <span class="nb-time">{fmtClock(d())}</span>
          </div>
          <Week now={p.now} cal={p.cal} dots={p.design === "dots"} />
          <Show when={p.design === "dots" ? nextMeeting(p.cal, p.now) : null}>{(it) => <MeetLine it={it()} cal={p.cal} now={p.now} compact />}</Show>
        </Match>
      </Switch>
    </section>
  );
}
