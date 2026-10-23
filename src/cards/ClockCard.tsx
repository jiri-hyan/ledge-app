// Osobní: velké hodiny s ISO týdnem (tik `now` dodává skořápka, jen při otevřeném šuplíku) a pod nimi
// baterie, jen na stroji s baterií (`media/Battery.tsx`).
import { Show, type JSX } from "solid-js";
import { fmtDate, fmtTime, isoWeek, pad2 } from "../format";
import { t } from "../i18n";
import type { BatteryView } from "../api";
import Battery from "../media/Battery";
import { PartSlot, PartsBox } from "../layout/PartSlot";
import "./personal.css";

export default function ClockCard(p: { now: number; battery?: BatteryView | null; hideBattery?: boolean; menu?: JSX.Element }) {
  const d = () => new Date(p.now);
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--teal)" />{t("cards.clock.title", "Čas")} <span class="n">{t("cards.clock.week", "týden {n}", { n: isoWeek(d()) })}</span>{p.menu}</h4>
      {/* hodiny a baterie jsou části karty: pořadí a skrytí v úpravách (`layout/PartSlot.tsx`); baterie je skrytá i bez baterie */}
      <PartsBox card="clock">
        <PartSlot card="clock" k="body">
          <div class="big-time">{fmtTime(d())}<span class="sec">{pad2(d().getSeconds())}</span></div>
          <div class="big-date">{t("cards.clock.date", "{date} {year}", { date: fmtDate(d()), year: d().getFullYear() })}</div>
        </PartSlot>
        <Show when={p.battery || p.hideBattery}>
          <PartSlot card="clock" k="battery" hidden={p.hideBattery}>
            <Battery b={p.battery ?? null} />
          </PartSlot>
        </Show>
      </PartsBox>
    </div>
  );
}
