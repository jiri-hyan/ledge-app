// Obsah widgetu podle druhu. Co jde, je znovupoužitá karta šuplíku nad daty z `desk-data` ve tvaru snapshotu
// (`model.ts` `deskSnapshot`): Limity = `limits/LimitsRow`, Dev servery = `cards/Running` (Otevřít / Stop přes
// tytéž příkazy jako v šuplíku — `stop` zastaví jen procesy, které spustil Ledge), Agenda = `calendar/CalendarCard`,
// skript = `widgets/WidgetCard`. „Čeká na tebe“ je vlastní a jen ukazuje: klik otevře šuplík na náhledu se
// schválením, rozhoduje se tam (widget nikdy nic neschválí). Souhlas se skriptem se taky dává jen v šuplíku.
import { For, Match, Show, Switch, type JSX } from "solid-js";
import { t } from "../i18n";
import LimitsRow from "../limits/LimitsRow";
import Running from "../cards/Running";
import CalendarCard from "../calendar/CalendarCard";
import WidgetCard from "../widgets/WidgetCard";
import WeekCard from "../board/WeekCard";
import BigClockCard from "../board/BigClockCard";
import UsageCard from "../board/UsageCard";
import type { DeskData, DeskWidget } from "../types";
import { deskApi } from "./api";
import { deskSnapshot, isScriptKind, waitMinutes } from "./model";
import { sayError } from "../ui/toast";

const open = (target: "waiting" | "settings" | "drawer") => () => void deskApi.openDrawer(target).catch(sayError);

function Waiting(p: { d: DeskData | null; compact: boolean }) {
  const rows = () => p.d?.waiting ?? [];
  const now = () => p.d?.now ?? 0;
  return (
    <Show when={p.d?.claudeEnabled !== false} fallback={<div class="empty">{t("cards.desk.wait.off", "Claude je v Nastavení vypnutý.")}</div>}>
      <Show when={rows().length} fallback={<div class="empty dk-calm">{t("cards.desk.wait.none", "Nic nečeká ✓")}</div>}>
        <Show when={!p.compact} fallback={
          <button class="dk-wait-sum" onClick={open("waiting")} title={t("cards.desk.wait.openTip", "Otevře Ledge — schvaluje se tam")}>
            {t("cards.desk.wait.count", "Čeká na tebe: {n}", { n: rows().length })}
          </button>
        }>
          <For each={rows()}>
            {(r) => (
              <button class="row dk-wait" onClick={open("waiting")} title={t("cards.desk.wait.openTip", "Otevře Ledge — schvaluje se tam")}>
                <span class="pill" classList={{ a: r.kind === "approve", b: r.kind === "question" }}>
                  {r.kind === "approve" ? t("cards.desk.wait.approve", "schválit") : t("cards.desk.wait.question", "otázka")}
                </span>
                <span class="grow">
                  <span class="name">{r.project}</span>
                  <span class="sub dk-wait-text">{r.text}</span>
                </span>
                <span class="sub">{t("cards.desk.wait.min", "{n} min", { n: waitMinutes(r.since, now()) })}</span>
              </button>
            )}
          </For>
          <div class="dk-foot">{t("cards.desk.wait.foot", "Schvaluje se jen v šuplíku Ledge.")}</div>
        </Show>
      </Show>
    </Show>
  );
}

/** Widget ze skriptu: bez souhlasu jen odkaz do šuplíku (povolit smí člověk tam, kde vidí obsah skriptu). */
function Script(p: { kind: string; d: DeskData | null; title: string }) {
  const v = () => p.d?.widgets?.find((x) => x.id === p.kind) ?? null;
  return (
    <Show when={v()?.status !== "consent"} fallback={
      <Card title={p.title}>
        <div class="empty">
          {t("cards.desk.script.consent", "Skript ještě není povolený — povol ho v Ledge na kartě widgetu.")}
          <button class="mini ok" onClick={open("drawer")}>{t("cards.desk.openLedge", "Otevřít Ledge")}</button>
        </div>
      </Card>
    }>
      <WidgetCard s={deskSnapshot(p.d)} id={p.kind} now={p.d?.now ?? 0} menu={() => null} />
    </Show>
  );
}

/** Karta s nadpisem pro widgety, které vlastní kartu nemají (limity, čekající, stavové věty). */
function Card(p: { title: string; children: JSX.Element }) {
  return (
    <div class="card dk-card">
      <h4>{p.title}</h4>
      {p.children}
    </div>
  );
}

export default function DeskBody(p: { w: DeskWidget; d: DeskData | null; title: string }) {
  const s = () => deskSnapshot(p.d);
  const now = () => p.d?.now ?? 0;
  return (
    <Switch fallback={<Card title={p.title}><div class="empty">{t("cards.desk.unknown", "Tenhle widget už Ledge nezná — odepni ho přes ⋯.")}</div></Card>}>
      <Match when={!p.d}>
        <Card title={p.title}><div class="empty">{t("cards.desk.loading", "Načítám…")}</div></Card>
      </Match>
      <Match when={p.w.kind === "limits"}>
        <Card title={p.title}>
          <Show when={p.d?.claudeEnabled !== false} fallback={<div class="empty">{t("cards.desk.wait.off", "Claude je v Nastavení vypnutý.")}</div>}>
            <LimitsRow s={s()} now={now()} onSettings={open("settings")} />
          </Show>
        </Card>
      </Match>
      <Match when={p.w.kind === "waiting"}>
        <Card title={p.title}><Waiting d={p.d} compact={p.w.compact} /></Card>
      </Match>
      <Match when={p.w.kind === "dev"}>
        <Running s={s()} now={now()} />
      </Match>
      <Match when={p.w.kind === "agenda"}>
        <Show when={p.d?.calendar?.status !== "off"} fallback={<Card title={p.title}><div class="empty">{t("cards.desk.agenda.off", "Kalendář je v šuplíku skrytý — přidej kartu Kalendář (✎ → + Přidat kartu).")}</div></Card>}>
          <CalendarCard s={s()} now={now()} onSettings={open("settings")} readOnly />
        </Show>
      </Match>
      {/* kolo 22 (`src/board/`): stejné karty jako v šuplíku nad daty z `desk-data` (čas po minutách) */}
      <Match when={p.w.kind === "week"}>
        <WeekCard s={s()} now={now()} />
      </Match>
      <Match when={p.w.kind === "clock"}>
        <BigClockCard now={now()} desk />
      </Match>
      <Match when={p.w.kind === "usage"}>
        <UsageCard s={s()} now={now()} desk />
      </Match>
      <Match when={isScriptKind(p.w.kind)}>
        <Script kind={p.w.kind} d={p.d} title={p.title} />
      </Match>
    </Switch>
  );
}
