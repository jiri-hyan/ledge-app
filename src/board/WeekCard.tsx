// Karta „Týden“ (kolo 22, id `week`, obr. 1 a 7): velký měsíc a čas, pás dnů od pondělí (dnešek zvýrazněný,
// víkend barevně, tečka = den s událostí) a pod ním zbytek dneška z kalendáře. Události jen z existujícího
// kalendáře (`calendar::live`, cache; stahuje se jen s kartou na otevřené záložce jako karta Kalendář); bez
// kalendáře řekne, že žádný není — „Nic na dnešek“ jen tehdy, když kalendář opravdu nic nemá.
import { For, Match, Show, Switch, type JSX } from "solid-js";
import type { Snapshot } from "../api";
import { fmtClock } from "../format";
import { t } from "../i18n";
import { calState, todayLeft, weekStrip } from "./model";
import { dowShort, monthShort } from "./text";
import "./board.css";

export default function WeekCard(p: { s: Snapshot; now: number; menu?: JSX.Element; onSettings?: () => void }) {
  const cal = () => p.s.personal?.calendar;
  const d = () => new Date(p.now);
  const days = () => weekStrip(p.now, cal());
  const left = () => todayLeft(cal(), p.now);
  const when = (start: number, allDay: boolean) => (allDay ? t("cards.board.week.allDay", "celý den") : fmtClock(new Date(start)));
  return (
    <div class="card bd-week" data-anim>
      <h4><span class="ic" style="background:var(--blue)" />{t("cards.board.week.title", "Týden")}{p.menu}</h4>
      <div class="bd-wk-top">
        <span class="bd-month">{monthShort()[d().getMonth()]}</span>
        <span class="bd-wk-time">{fmtClock(d())}</span>
      </div>
      <div class="bd-strip" role="list">
        <For each={days()}>
          {(x) => (
            <span class="bd-day" role="listitem" classList={{ "bd-today": x.today, "bd-weekend": x.weekend }} aria-current={x.today ? "date" : undefined}>
              <span class="bd-dow">{dowShort()[x.dow]}</span>
              <span class="bd-dnum">{x.day}</span>
              <i class="bd-dot" classList={{ on: x.busy }} />
            </span>
          )}
        </For>
      </div>
      <Switch>
        <Match when={calState(cal()) === "none"}>
          <div class="bd-note">
            {t("cards.board.week.noCal", "Bez kalendáře — události se tu ukážou po připojení.")}
            <Show when={p.onSettings}><button class="mini" onClick={() => p.onSettings?.()}>{t("cards.board.week.connect", "Připojit")}</button></Show>
          </div>
        </Match>
        <Match when={calState(cal()) === "loading"}>
          <div class="bd-note">{t("cards.board.week.loading", "Načítám kalendář…")}</div>
        </Match>
        <Match when={calState(cal()) === "unavailable"}>
          <div class="bd-note">{t("cards.board.week.unavailable", "Kalendář teď není dostupný.")}</div>
        </Match>
        <Match when={left().length === 0}>
          <div class="bd-note bd-calm">{t("cards.board.week.nothing", "Nic na dnešek")}</div>
        </Match>
        <Match when={left().length > 0}>
          <div class="bd-evs">
            <For each={left()}>
              {(it) => (
                <div class="bd-ev" classList={{ on: it.ongoing }}>
                  <span class="bd-ev-t">{when(it.start, it.allDay)}</span>
                  <span class="bd-ev-n" title={it.location || it.title}>{it.title || t("cards.board.week.untitled", "(bez názvu)")}</span>
                </div>
              )}
            </For>
          </div>
        </Match>
      </Switch>
    </div>
  );
}
