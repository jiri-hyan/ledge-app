// Hodiny v hlavičce (kolo 27): pět stylů z `appearance.head.clock` — Velké (čas a datum pod ním), Jednořádkové
// „20:02 · po 2. 11.“, S vteřinami, Analogové malé (SVG), Jen čas. Čas dodává `now` z `App` (sekundový tik existuje
// jen s otevřeným šuplíkem), takže tu žádný vlastní časovač není. Čistá část: `layout/headModel.ts`.
import { Show } from "solid-js";
import { fmtClock, fmtDate } from "../format";
import { locale } from "../i18n";
import type { ClockStyle } from "../types.ts";
import { handAngles, shortDate, withSeconds } from "../layout/headModel";

export default function HeadClock(p: { now: number; style: ClockStyle; showDate: boolean }) {
  const d = () => new Date(p.now);
  const hhmm = () => fmtClock(d());
  const short = () => shortDate(d().toLocaleDateString(locale(), { weekday: "short" }).replace(/\.$/, ""), d());
  const hands = () => handAngles(d());
  return (
    <div class="clock" data-clock={p.style}>
      <Show when={p.style === "analog"}>
        <svg class="clock-face" viewBox="0 0 40 40" width="38" height="38" aria-hidden="true">
          <circle cx="20" cy="20" r="18.5" fill="none" stroke="var(--line-2)" stroke-width="1.5" />
          <path d="M20 3.5v3M36.5 20h-3M20 36.5v-3M3.5 20h3" stroke="var(--t3)" stroke-width="1.5" stroke-linecap="round" />
          <line x1="20" y1="20" x2="20" y2="10.5" stroke="var(--t1)" stroke-width="2.4" stroke-linecap="round" transform={`rotate(${hands().h} 20 20)`} />
          <line x1="20" y1="20" x2="20" y2="6.5" stroke="var(--accent)" stroke-width="1.8" stroke-linecap="round" transform={`rotate(${hands().m} 20 20)`} />
          <circle cx="20" cy="20" r="1.8" fill="var(--t1)" />
        </svg>
      </Show>
      <Show when={p.style !== "analog"}>
        <span class="time">{p.style === "seconds" ? withSeconds(hhmm(), d()) : hhmm()}</span>
      </Show>
      <Show when={p.showDate && p.style !== "time"}>
        <span class="date">{p.style === "line" || p.style === "analog" ? short() : fmtDate(d())}</span>
      </Show>
    </div>
  );
}
