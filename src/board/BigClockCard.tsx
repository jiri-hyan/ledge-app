// Karta „Velké hodiny“ (kolo 22, id `bigclock`, obr. 5 a 6): čas jako hrdina (obří tabulkové číslice, hodiny nad
// minutami), pod ním den a datum. Čas je systémový (`now` dodává skořápka jen při otevřeném šuplíku; na ploše po
// minutách — proto tam bez sekund). Vedle karty Čas, která má týden a baterii, je tohle čistě typografická volba.
import { Show, type JSX } from "solid-js";
import { fmtDate, isoWeek, pad2 } from "../format";
import { t } from "../i18n";
import "./board.css";

export default function BigClockCard(p: { now: number; menu?: JSX.Element; desk?: boolean }) {
  const d = () => new Date(p.now);
  return (
    <div class="card bd-clock" data-anim>
      <h4><span class="ic" style="background:var(--teal)" />{t("cards.board.clock.title", "Hodiny")} <span class="n">{t("cards.clock.week", "týden {n}", { n: isoWeek(d()) })}</span>{p.menu}</h4>
      <div class="bd-digits" aria-label={`${pad2(d().getHours())}:${pad2(d().getMinutes())}`}>
        <span class="bd-hh">{pad2(d().getHours())}</span>
        <span class="bd-mm">{pad2(d().getMinutes())}</span>
        <Show when={!p.desk}><span class="bd-ss">{pad2(d().getSeconds())}</span></Show>
      </div>
      <div class="bd-cdate">{t("cards.clock.date", "{date} {year}", { date: fmtDate(d()), year: d().getFullYear() })}</div>
    </div>
  );
}
