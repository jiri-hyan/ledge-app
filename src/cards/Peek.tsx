// Peek = malý šuplík (bez fokusu): první čekající schválení Clauda (Povolit / Zamítnout jen klikem
// člověka; pod tím sbalený náhled diffu / plánu a „Vždy povolit“ a „Zamítnout s důvodem…“ z `sessions/PermExtra.tsx`),
// jinak otázka Clauda (`QuestionCard` — odpověď kliknutím), jinak připomínka resetu limitu Claude (`LimitsPeek`), jinak připomínky po termínu.
// `Approval` (schválení / otázka) a `DecideButtons` znovu používá náhled po najetí (`src/hover/*`) — jedno UI
// pro schválení ve všech režimech `drawer.hover`.
import { Show } from "solid-js";
import { api, type Pending, type Snapshot } from "../api";
import { IconShield } from "../icons";
import { ReminderPeek } from "../notes/NotesCard";
import QuestionCard from "../questions/QuestionCard";
import PermExtra from "../sessions/PermExtra";
import LimitsPeek from "../limits/LimitsPeek";
import DiffPreview from "../sessions/DiffPreview";
import PlanReview from "../sessions/PlanReview";
import { planTitle } from "../sessions/plan";
import { t } from "../i18n";
import "./peek.css";

/** Čeká na člověka něco (schválení nebo otázka)? */
export const hasApproval = (s: Snapshot) => s.pending.length > 0 || s.questions.length > 0;

/** Odkud žádost je: projekt session, jinak poslední složka cwd. */
export function pendingWhere(s: Snapshot, pd: Pending): string {
  const ss = s.sessions.find((x) => x.id === pd.sessionId);
  return ss?.project ?? pd.cwd.split(/[\\/]/).pop() ?? "";
}

/** Zamítnout / Povolit (u plánu Schválit) — jen klik člověka. */
export function DecideButtons(p: { pd: Pending }) {
  return (
    <>
      <button class="btn" onClick={() => api.decide(p.pd.id, false)}>{t("cards.peek.deny", "Zamítnout")}</button>
      <button class="btn pri" onClick={() => api.decide(p.pd.id, true)}>{p.pd.plan ? t("cards.peek.approve", "Schválit") : t("cards.peek.allow", "Povolit")}</button>
    </>
  );
}

/** První čekající schválení (s náhledem diffu / plánu), jinak otázka Clauda; nic = nic. `stack` = tlačítka
 *  pod textem (úzký kompaktní náhled, `hover/hover.css` `.hov-stack`). */
export function Approval(p: { s: Snapshot; stack?: boolean }) {
  const first = () => p.s.pending[0];
  const more = () => p.s.pending.length + p.s.questions.length - 1;
  const question = () => (first() ? undefined : p.s.questions[0]);
  return (
    <Show when={first()} fallback={
      <Show when={question()} keyed>
        {(a) => <div class="qpeek" data-anim><QuestionCard ask={a} s={p.s} /></div>}
      </Show>
    }>
      {(pd) => (
        <div class="peek" classList={{ "hov-stack": !!p.stack }} data-anim>
          <div class="ico"><IconShield /></div>
          <div class="tx">
            <b>{t("cards.peek.waiting", "Claude čeká · {where}", { where: pendingWhere(p.s, pd()) })}{more() > 0 ? ` (+${more()})` : ""}</b>
            <Show when={pd().plan} fallback={<div class="code" title={pd().summary}>{pd().tool}: {pd().summary}</div>}>
              <div class="code" title={pd().summary}>{t("cards.peek.plan", "Plán: {title}", { title: planTitle(pd()) })}</div>
            </Show>
            <Show when={pd().preview}>{(pv) => <DiffPreview pv={pv()} />}</Show>
            <Show when={pd().plan}>{(pl) => <PlanReview pd={pd()} plan={pl()} open={false} />}</Show>
            <PermExtra pd={pd()} />
          </div>
          <DecideButtons pd={pd()} />
        </div>
      )}
    </Show>
  );
}

export default function Peek(p: { s: Snapshot }) {
  return (
    <Show when={hasApproval(p.s)} fallback={<LimitsPeek s={p.s} fallback={<ReminderPeek s={p.s} />} />}>
      <Approval s={p.s} />
    </Show>
  );
}
