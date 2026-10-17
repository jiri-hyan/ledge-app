// Review plánu (ExitPlanMode) v kartě schválení (`PendingRow` i `cards/Peek.tsx`): plán jako Markdown
// (bezpečný renderer poznámek `notes/Markdown.tsx` — HTML escapuje Rust, žádné cizí raw HTML), sbalitelný,
// výška omezená (scroll uvnitř), a „Schválit a povolit úpravy“ (allow + setMode acceptEdits pro session).
// „Schválit“ / „Zamítnout“ jsou hlavní tlačítka karty, „Vrátit s připomínkou…“ je pole důvodu v `PermExtra`.
// Jen klik člověka — žádná nová klávesa. Logika v `plan.ts`, rozhodnutí v Rustu (`claude/plan.rs`).
import { createSignal, Show } from "solid-js";
import { api, type Pending, type PlanView } from "../api";
import Markdown from "../notes/Markdown";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { planDecide } from "./plan";
import "../sessions.css";

export default function PlanReview(p: { pd: Pending; plan: PlanView; open?: boolean }) {
  const [open, setOpen] = createSignal(p.open ?? true);
  const [busy, setBusy] = createSignal(false);

  async function approveEdits() {
    const d = planDecide("approveEdits");
    if (!d || busy()) return;
    setBusy(true);
    try {
      if (!(await api.decide(p.pd.id, d.allow, d.extra))) say(t("sessions.perm.gone", "Žádost už nečeká — vyřídila se jinde nebo vypršela."), true);
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="sess-plan">
      <button class="sess-plan-head" aria-expanded={open()} onClick={() => setOpen(!open())} title={p.plan.file ?? undefined}>
        {open() ? t("sessions.plan.hide", "Sbalit plán") : t("sessions.plan.show", "Ukázat plán")} {open() ? "▴" : "▾"}
      </button>
      <Show when={open()}>
        <div class="sess-plan-body">
          <Markdown body={p.plan.markdown} images={[]} onToggle={() => {}} />
        </div>
        <Show when={p.plan.cut}>
          <small class="sess-diff-note">{t("sessions.plan.cut", "Plán je delší — v Ledge zkrácený, celý je v okně Claude.")}</small>
        </Show>
      </Show>
      <div class="sess-perm-line">
        <button class="mini" disabled={busy()} onClick={approveEdits}
          title={t("sessions.plan.editsHint", "Schválí plán a přepne tuto session do režimu „přijímat úpravy“ — Claude pak upravuje soubory bez ptaní")}>
          {t("sessions.plan.approveEdits", "Schválit a povolit úpravy")}
        </button>
      </div>
    </div>
  );
}
