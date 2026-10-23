// Karta Dnes (katalog karet, ve výchozím rozložení není): jen to, co dnes něco znamená — další schůzka, připomínky
// do konce dneška, PR čekající na mou revizi a reset 5h limitu Claude. Řádky skládá čistá `today.ts` z dat, která
// snapshot už má (kalendář, poznámky, paměť GitHubu, limity) — karta nic nestahuje. Řádek bez dat se nekreslí,
// bez jediného řádku se nekreslí ani karta (`Facts.today`); v režimu úprav ukáže, co sem patří.
import { createMemo, For, Show, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { fmtClock } from "../format";
import { t } from "../i18n";
import { intApi } from "../integrations/api";
import { providerName } from "../calendar/model";
import { leftText, resetClock } from "../limits/LimitsRow";
import { pctText } from "../limits/model";
import type { NotesNav } from "../nav";
import { sayError } from "../ui/toast";
import { sameRows, todayRows, type TodayInput, type TodayRow } from "./today";
import "./today.css";

/** Vstup `todayRows` ze snapshotu (jen čtení, žádný dotaz). */
export const todayInput = (s: Snapshot): TodayInput => ({
  calendar: s.personal.calendar,
  notes: s.notes,
  reminders: s.reminders,
  github: s.suggest?.github,
  limits: s.limits,
});

/** Řádky karty; stejný obsah = stejné pole (sekundový tik karta nepřekreslí). */
export const useTodayRows = (s: () => Snapshot, now: () => number, enabled: () => boolean = () => true) =>
  createMemo<TodayRow[]>(() => (enabled() ? todayRows(todayInput(s()), now()) : []), [], { equals: sameRows });

const fail = (e: unknown) => sayError(e);

export default function TodayCard(p: { s: Snapshot; now: number; nav: NotesNav; menu?: JSX.Element }) {
  const rows = useTodayRows(() => p.s, () => p.now);
  const noteOf = (path: string) => [...p.s.reminders, ...p.s.notes].find((n) => n.path === path);
  const Row = (q: { r: TodayRow }): JSX.Element => {
    const r = q.r;
    switch (r.kind) {
      case "meeting":
        return (
          <div class="row td-row">
            <span class="td-time" classList={{ "td-now": r.ongoing }}>{fmtClock(new Date(r.start))}</span>
            <div class="grow">
              <div class="name">{r.title || t("cards.calendar.untitled", "(bez názvu)")}</div>
              <div class="sub">{r.ongoing ? t("cards.today.meetingNow", "právě probíhá") : t("cards.today.meetingIn", "další schůzka · {left}", { left: leftText(r.start, p.now) })}</div>
            </div>
            <Show when={r.join}>
              {(j) => (
                <button class="mini ok" title={t("cards.calendar.joinTitle", "Připojit se přes {app}", { app: providerName(j().provider) })} onClick={() => void api.calendarJoin(j().url).catch(fail)}>
                  {t("cards.calendar.join", "Připojit")}
                </button>
              )}
            </Show>
          </div>
        );
      case "reminder":
        return (
          <button class="row td-row td-click" title={t("cards.today.openNote", "Otevřít poznámku")} onClick={() => { const n = noteOf(r.path); if (n) p.nav.onOpen(n); }}>
            <span class="td-time" classList={{ "td-late": r.overdue }}>{fmtClock(new Date(r.remindMs))}</span>
            <div class="grow">
              <div class="name">{r.title || t("cards.suggest.imageOnly", "(jen obrázek)")}</div>
              <div class="sub">{r.overdue ? t("cards.today.overdue", "připomínka po termínu") : t("cards.today.reminder", "připomínka")}</div>
            </div>
          </button>
        );
      case "reminderMore":
        return <div class="sub td-more">{t("cards.today.moreReminders", "+{n} dalších připomínek", { n: r.n })}</div>;
      case "review":
        return (
          <button class="row td-row td-click" title={[r.title, r.url].join("\n")} onClick={() => void intApi.githubOpen(r.url).catch(fail)}>
            <span class="td-time">PR</span>
            <div class="grow">
              <div class="name">{r.title}</div>
              <div class="sub">{t("cards.today.review", "čeká na tvou revizi · {pr}", { pr: `${r.repo}#${r.number}` })}</div>
            </div>
            <Show when={r.more > 0}><span class="pill n">+{r.more}</span></Show>
          </button>
        );
      case "limit":
        return (
          <div class="row td-row">
            <span class="td-time">5h</span>
            <div class="grow">
              <div class="name">{t("cards.today.limit", "Limit Claude: {pct} %", { pct: pctText(r.pct) })}</div>
              <div class="sub">{t("cards.today.limitReset", "reset v {clock} · {left}", { clock: resetClock(r.resetsAt, p.now), left: leftText(r.resetsAt, p.now) })}</div>
            </div>
          </div>
        );
    }
  };
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--amber)" />{t("cards.today.title", "Dnes")}{p.menu}</h4>
      <Show when={rows().length > 0} fallback={<div class="empty">{t("cards.today.empty", "Dnes nic — sem se ukáže schůzka, připomínky, PR a limit")}</div>}>
        <For each={rows()}>{(r) => <Row r={r} />}</For>
      </Show>
    </div>
  );
}
