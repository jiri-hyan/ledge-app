// Karta „Týden“ (kolo 22, id `week`; kolo 25 vzhledy `weekDesigns.ts`): výchozí „strip“ = velký měsíc a čas, pás dnů
// od pondělí (dnešek zvýrazněný, víkend barevně, tečka = den s událostí) a zbytek dneška z kalendáře. Rozložení se
// přizpůsobí šířce karty přes container queries (`board.css`): úzká = pás + čas, střední = měsíc + čas nad pásem,
// široká = pás vlevo a události dneška ve sloupci vpravo. Další vzhledy: Měsíc, Odtrhávací list, Osa dne, Ciferník,
// Program (`WeekViews.tsx`). Události jen z existujícího kalendáře (`calendar::live`, cache; stahuje se jen s kartou
// na otevřené záložce jako karta Kalendář); bez kalendáře řekne, že žádný není — „Nic na dnešek“ jen tehdy, když
// kalendář opravdu nic nemá.
import { For, Match, Show, Switch, type JSX } from "solid-js";
import type { Snapshot } from "../api";
import { fmtClock } from "../format";
import { t } from "../i18n";
import { calState, todayLeft, weekStrip } from "./model";
import { dowShort, monthShort } from "./text";
import { AgendaBody, DialBody, MonthBody, TearoffBody, TimelineBody } from "./WeekViews";
import { weekDesign } from "./weekDesigns";
import "./board.css";

export default function WeekCard(p: { s: Snapshot; now: number; menu?: JSX.Element; onSettings?: () => void; design?: string }) {
  const cal = () => p.s.personal?.calendar;
  const design = () => weekDesign(p.design);
  const d = () => new Date(p.now);
  const days = () => weekStrip(p.now, cal());
  const left = () => todayLeft(cal(), p.now, 5);
  const when = (start: number, allDay: boolean) => (allDay ? t("cards.board.week.allDay", "celý den") : fmtClock(new Date(start)));
  /** Prázdné stavy kalendáře (žádný / načítá se / nedostupný); u hotového kalendáře nic. */
  const Notes = () => (
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
    </Switch>
  );
  return (
    <div class="card bd-week" data-anim data-wk={design()}>
      <h4><span class="ic" style="background:var(--blue)" />{t("cards.board.week.title", "Týden")}{p.menu}</h4>
      <Switch>
        <Match when={design() === "strip"}>
          <div class="bd-wk-body">
            <div class="bd-wk-main">
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
            </div>
            <div class="bd-wk-side">
              <Notes />
              <Show when={calState(cal()) === "ok"}>
                <Show when={left().length > 0} fallback={<div class="bd-note bd-calm">{t("cards.board.week.nothing", "Nic na dnešek")}</div>}>
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
                </Show>
              </Show>
            </div>
          </div>
        </Match>
        <Match when={design() === "month"}><MonthBody cal={cal()} now={p.now} /><Notes /></Match>
        <Match when={design() === "tearoff"}><TearoffBody cal={cal()} now={p.now} /><Notes /></Match>
        <Match when={design() === "timeline"}><TimelineBody cal={cal()} now={p.now} /><Notes /></Match>
        <Match when={design() === "dial"}><DialBody cal={cal()} now={p.now} /><Notes /></Match>
        <Match when={design() === "agenda"}><AgendaBody cal={cal()} now={p.now} /><Notes /></Match>
      </Switch>
    </div>
  );
}
