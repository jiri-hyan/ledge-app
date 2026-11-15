// Záložka „Úkoly“ (`_tasks`) v kartě Poznámky i na stránce Poznámky: Po termínu · Dnes (schůzky + body podle času) ·
// Čeká na tebe (`[~]`) · Brzy · Bez termínu. Data z `note_tasks` (paměť jádra, bez I/O) při změně `notesRev`; sekce
// skládá čistá `agenda.ts`. Prázdná sekce se nekreslí, úplně nic = „Žádné otevřené úkoly“ (žádné divadlo).
import { createMemo, createResource, For, Show, type JSX } from "solid-js";
import { api, type NoteView, type Snapshot } from "../api";
import { providerName } from "../calendar/model";
import { fmtClock } from "../format";
import { t, tp } from "../i18n";
import { createMenu, PopItem, PopMenu } from "../ui/Popover";
import { sayError } from "../ui/toast";
import { quiet } from "../uiErrors";
import type { TaskRow } from "../types";
import { buildAgenda, dayDiff, isOverdue, snoozeIso, type AgendaItem } from "./agenda";
import { projName } from "./store";
import { checkTask, openTask, snoozeTask } from "./tasksact";
import "./tasks.css";

const MIN = 60_000;
const dm = (ms: number) => `${new Date(ms).getDate()}. ${new Date(ms).getMonth() + 1}.`;

/** Chip času bodu: po termínu (jantar) · dnes · zítra · datum. Celodenní bez hodin. */
function whenChip(r: TaskRow, now: number): { text: string; tone: "late" | "now" | "" } | null {
  if (r.dueMs == null) return null;
  const clock = r.allDay ? "" : fmtClock(new Date(r.dueMs));
  const d = dayDiff(r.dueMs, now);
  if (isOverdue(r, now)) {
    const text = d === 0 ? t("notes.tasks.since", "od {time}", { time: clock }) : d === -1 ? t("notes.tasks.yesterday", "včera {time}", { time: clock }).trim() : dm(r.dueMs);
    return { text, tone: "late" };
  }
  if (d === 0) return { text: clock || t("notes.tasks.allDay", "dnes"), tone: "now" };
  return { text: d === 1 ? t("notes.tasks.tomorrow", "zítra {time}", { time: clock }).trim() : `${dm(r.dueMs)} ${clock}`.trim(), tone: "" };
}

function Snooze(p: { r: TaskRow; now: number; onDone: () => void }) {
  const menu = createMenu();
  const label = t("notes.tasks.snooze", "Odložit");
  const go = (iso: string) => {
    menu.close();
    void snoozeTask(p.r, iso).then((ok) => ok && p.onDone());
  };
  return (
    <>
      <button class="note-act" classList={{ on: menu.open() }} aria-haspopup="menu" aria-expanded={menu.open()} title={label} aria-label={label} onClick={menu.toggle}>⋯</button>
      <Show when={menu.at()}>
        {(at) => (
          <PopMenu anchor={at().anchor} keyboard={at().keyboard} trigger={menu.trigger()} onClose={menu.close} align="end" label={label}>
            <PopItem onClick={() => go(snoozeIso(p.r, "tomorrow", p.now))}>{t("notes.tasks.snoozeTomorrow", "Odložit na zítra")}</PopItem>
            <PopItem onClick={() => go(snoozeIso(p.r, "monday", p.now))}>{t("notes.tasks.snoozeMonday", "Odložit na pondělí")}</PopItem>
            <Show when={p.r.dueMs != null}>
              <PopItem onClick={() => go("")}>{t("notes.tasks.unschedule", "Odebrat termín")}</PopItem>
            </Show>
          </PopMenu>
        )}
      </Show>
    </>
  );
}

export default function TasksView(p: { s: Snapshot; now: number; onOpen: (n: NoteView) => void }) {
  const [res, { refetch }] = createResource(() => p.s.notesRev, () => quiet(api.noteTasks({ limit: 500 }), null, "noteTasks"));
  // po minutách: hranice po termínu / dnes se tikem sekund nepřepočítává
  const minute = () => Math.floor(p.now / MIN) * MIN;
  const ag = createMemo(() => buildAgenda(res.latest?.rows ?? [], p.s.personal?.calendar, minute()));

  const Row = (q: { r: TaskRow }): JSX.Element => {
    const chip = () => whenChip(q.r, minute());
    return (
      <div class="tk-row" classList={{ "tk-wait": q.r.state === "proposed" }}>
        <button class="tk-check" role="checkbox" aria-checked="false" aria-label={t("notes.tasks.check", "Odškrtnout")} title={q.r.state === "proposed" ? t("notes.tasks.checkWait", "Navržený bod potvrdíš v poznámce") : t("notes.tasks.check", "Odškrtnout")} disabled={q.r.state === "proposed"} onClick={() => void checkTask(q.r).then((ok) => ok && void refetch())} />
        <button class="tk-body" title={t("notes.tasks.open", "Otevřít poznámku na tomto řádku")} onClick={() => void openTask(q.r, p.onOpen)}>
          <span class="tk-text">{q.r.prio > 0 ? <b class="tk-prio" title={t("notes.tasks.prio", "Priorita")}>{"!".repeat(q.r.prio)}</b> : null}{q.r.text}</span>
          <span class="tk-sub">{q.r.noteTitle}{q.r.project ? ` · ${projName(p.s, q.r.project)}` : ""}</span>
        </button>
        <Show when={chip()}>{(c) => <span class="tk-chip" data-tone={c().tone}>{c().text}</span>}</Show>
        <Show when={q.r.state !== "proposed"}><Snooze r={q.r} now={minute()} onDone={() => void refetch()} /></Show>
      </div>
    );
  };

  const Meeting = (q: { m: Extract<AgendaItem, { kind: "meeting" }> }): JSX.Element => (
    <div class="tk-row">
      <span class="tk-time" classList={{ "tk-now": q.m.ongoing }}>{q.m.allDay ? t("notes.tasks.allDay", "dnes") : fmtClock(new Date(q.m.start))}</span>
      <div class="tk-body tk-static">
        <span class="tk-text">{q.m.title || t("cards.calendar.untitled", "(bez názvu)")}</span>
        <span class="tk-sub">{q.m.ongoing ? t("notes.tasks.meetingNow", "právě probíhá") : t("notes.tasks.meeting", "schůzka")}</span>
      </div>
      <Show when={q.m.join}>
        {(j) => (
          <button class="mini ok" title={t("cards.calendar.joinTitle", "Připojit se přes {app}", { app: providerName(j().provider) })} onClick={() => void api.calendarJoin(j().url).catch(sayError)}>
            {t("cards.calendar.join", "Připojit")}
          </button>
        )}
      </Show>
    </div>
  );

  const Section = (q: { label: string; n: number; tone?: string; more?: number; tip?: string; children: JSX.Element }): JSX.Element => (
    <section class="tk-sec" data-tone={q.tone ?? ""}>
      <div class="note-group-head" title={q.tip}><span class="note-group-label">{q.label}</span><span class="note-group-n">{q.n}</span></div>
      {q.children}
      <Show when={(q.more ?? 0) > 0}><div class="tk-more">{tp("notes.tasks.more", q.more!, "+{n} dalších")}</div></Show>
    </section>
  );

  return (
    <div class="tk">
      <Show when={ag().overdue.length}>
        <Section label={t("notes.tasks.overdue", "Po termínu")} n={ag().overdue.length} tone="late"><For each={ag().overdue}>{(r) => <Row r={r} />}</For></Section>
      </Show>
      <Show when={ag().today.length}>
        <Section label={t("notes.tasks.today", "Dnes")} n={ag().today.length}>
          <For each={ag().today}>{(i) => (i.kind === "meeting" ? <Meeting m={i} /> : <Row r={i.row} />)}</For>
        </Section>
      </Show>
      <Show when={ag().waiting.length}>
        <Section label={t("notes.tasks.waiting", "Čeká na tebe")} n={ag().waiting.length} tip={t("notes.tasks.waitingTip", "Body [~], které navrhl Claude ke kontrole")}>
          <For each={ag().waiting}>{(r) => <Row r={r} />}</For>
        </Section>
      </Show>
      <Show when={ag().soon.length}>
        <Section label={t("notes.tasks.soon", "Brzy")} n={ag().soon.length + ag().soonMore} more={ag().soonMore}><For each={ag().soon}>{(r) => <Row r={r} />}</For></Section>
      </Show>
      <Show when={ag().undated.length}>
        <Section label={t("notes.tasks.undated", "Bez termínu")} n={ag().undated.length + ag().undatedMore} more={ag().undatedMore}><For each={ag().undated}>{(r) => <Row r={r} />}</For></Section>
      </Show>
      <Show when={ag().empty}>
        <div class="empty">{res.loading && !res.latest ? t("notes.page.loading", "Načítám…") : t("notes.tasks.empty", "Žádné otevřené úkoly")}</div>
      </Show>
    </div>
  );
}
