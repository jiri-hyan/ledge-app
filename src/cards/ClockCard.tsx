// Osobní: velké hodiny s ISO týdnem (tik `now` dodává skořápka, jen při otevřeném šuplíku).
import { fmtDate, fmtTime, isoWeek, pad2 } from "../format";
import { t } from "../i18n";
import "./personal.css";

export default function ClockCard(p: { now: number }) {
  const d = () => new Date(p.now);
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--teal)" />{t("cards.clock.title", "Čas")} <span class="n">{t("cards.clock.week", "týden {n}", { n: isoWeek(d()) })}</span></h4>
      <div class="big-time">{fmtTime(d())}<span class="sec">{pad2(d().getSeconds())}</span></div>
      <div class="big-date">{t("cards.clock.date", "{date} {year}", { date: fmtDate(d()), year: d().getFullYear() })}</div>
    </div>
  );
}
