// Peek = malý šuplík (bez fokusu): první čekající schválení Clauda (Povolit / Zamítnout jen klikem
// člověka), jinak otázka Clauda (`QuestionCard` — odpověď kliknutím), jinak připomínky po termínu.
import { Show } from "solid-js";
import { api, type Snapshot } from "../api";
import { IconShield } from "../icons";
import { ReminderPeek } from "../notes/NotesCard";
import QuestionCard from "../questions/QuestionCard";
import { t } from "../i18n";
import "./peek.css";

export default function Peek(p: { s: Snapshot }) {
  const first = () => p.s.pending[0];
  const more = () => p.s.pending.length + p.s.questions.length - 1;
  const where = () => {
    const pd = first();
    if (!pd) return "";
    const ss = p.s.sessions.find((x) => x.id === pd.sessionId);
    return ss?.project ?? pd.cwd.split(/[\\/]/).pop();
  };
  const question = () => (first() ? undefined : p.s.questions[0]);
  return (
    <Show when={first()} fallback={
      <Show when={question()} keyed fallback={<ReminderPeek s={p.s} />}>
        {(a) => <div class="qpeek" data-anim><QuestionCard ask={a} s={p.s} /></div>}
      </Show>
    }>
      {(pd) => (
        <div class="peek" data-anim>
          <div class="ico"><IconShield /></div>
          <div class="tx">
            <b>{t("cards.peek.waiting", "Claude čeká · {where}", { where: where() ?? "" })}{more() > 0 ? ` (+${more()})` : ""}</b>
            <div class="code" title={pd().summary}>{pd().tool}: {pd().summary}</div>
          </div>
          <button class="btn" onClick={() => api.decide(pd().id, false)}>{t("cards.peek.deny", "Zamítnout")}</button>
          <button class="btn pri" onClick={() => api.decide(pd().id, true)}>{t("cards.peek.allow", "Povolit")}</button>
        </div>
      )}
    </Show>
  );
}
