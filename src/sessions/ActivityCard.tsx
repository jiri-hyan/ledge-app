// Karta „Claude aktivita“ (kolo 23, id `activity`, jen z katalogu): ovládací centrum — každá session s fází
// (ODHAD podle posledního nástroje, v UI označený), upravovaným souborem, poslední zprávou Clauda a pod ní
// schválení / otázky přes EXISTUJÍCÍ cestu (`PendingRow` → `api.decide`, `QuestionCard`). Povolit smí jen
// člověk kliknutím. Data jen z `Snapshot.sessions[].activity` (Rust `claude/activity.rs`); bez dat prázdné.
// Když je na záložce i karta Claude, schválení jsou tam (žádné dvojí tlačítko Povolit) a tady jen odkaz.
import { For, Show, type JSX } from "solid-js";
import type { Snapshot, SessionView } from "../api";
import { ago } from "../format";
import { t, tp } from "../i18n";
import { PendingRow } from "../ClaudeSessions";
import QuestionCard from "../questions/QuestionCard";
import { baseName, hooksMissing, livePhase, orderActivity } from "./activity";
import type { ActivityPhase } from "../types/claude";
import "./activity.css";

export default function ActivityCard(p: { s: Snapshot; now: number; approvals: boolean; menu?: JSX.Element }) {
  const waitingIds = () => new Set([...p.s.pending.map((x) => x.sessionId), ...p.s.questions.map((x) => x.sessionId)]);
  const rows = () => orderActivity(p.s.sessions, waitingIds());
  const known = () => new Set(rows().map((r) => r.id));
  const orphanPending = () => p.s.pending.filter((x) => !known().has(x.sessionId));
  const orphanAsks = () => p.s.questions.filter((x) => !known().has(x.sessionId));
  const phaseName = (ph: ActivityPhase): string =>
    ({
      reading: () => t("cards.activity.phase.reading", "Čtení"),
      editing: () => t("cards.activity.phase.editing", "Úpravy"),
      testing: () => t("cards.activity.phase.testing", "Testy"),
      working: () => t("cards.activity.phase.working", "Pracuje"),
      done: () => t("cards.activity.phase.done", "Hotovo"),
      error: () => t("cards.activity.phase.error", "Chyba"),
    })[ph]();
  const estimate = () => t("cards.activity.estimate", "odhad podle posledního nástroje");

  const Row = (q: { r: SessionView }) => {
    const a = () => q.r.activity ?? null;
    const ph = () => livePhase(q.r);
    const pend = () => p.s.pending.filter((x) => x.sessionId === q.r.id);
    const asks = () => p.s.questions.filter((x) => x.sessionId === q.r.id);
    return (
      <div class="clact-row" data-wait={pend().length + asks().length > 0 ? "" : undefined}>
        <div class="row">
          <div class="grow">
            <div class="clact-name">{q.r.project ?? q.r.name}</div>
            <Show when={a()?.file}>{(f) => <div class="clact-file" title={f()}>{baseName(f())}</div>}</Show>
          </div>
          <Show when={ph()}>
            {(x) => (
              <span class="pill" classList={{ b: x() === "reading" || x() === "testing", v: x() === "editing", n: x() === "working", g: x() === "done", r: x() === "error" }} title={estimate()}>
                {phaseName(x())}
              </span>
            )}
          </Show>
          <Show when={a()}>{(x) => <span class="clact-ago">{ago(x().at, p.now)}</span>}</Show>
        </div>
        <Show when={a()?.message}>{(m) => <div class="clact-msg" title={m()}>{m()}</div>}</Show>
        <Show when={p.approvals} fallback={<Show when={pend().length + asks().length > 0}><div class="clact-note">{t("cards.activity.inClaude", "Čeká na tebe — schválení je v kartě Claude.")}</div></Show>}>
          <For each={pend()}>{(pd) => <PendingRow pd={pd} />}</For>
          <For each={asks()}>{(x) => <QuestionCard ask={x} s={p.s} inSession />}</For>
        </Show>
      </div>
    );
  };

  return (
    <div class="card" data-anim>
      <h4>
        <span class="ic" style="background:var(--violet)" />
        {t("cards.activity.title", "Claude aktivita")} <span class="n">{rows().length ? tp("sessions.count", rows().length, "{n} session") : ""}</span>
        {p.menu}
      </h4>
      <Show when={rows().length + (p.approvals ? orphanPending().length + orphanAsks().length : 0) > 0} fallback={
          <div class="clact-empty">
            {hooksMissing(p.s.hooks)
              ? t("cards.activity.noHooks", "Ledge nemá nainstalované hooky — aktivita se nezobrazí.")
              : t("cards.activity.empty", "Claude teď nic nedělá.")}
          </div>
        }>
        <For each={rows()}>{(r) => <Row r={r} />}</For>
        <Show when={p.approvals}>
          <For each={orphanPending()}>{(pd) => <PendingRow pd={pd} />}</For>
          <For each={orphanAsks()}>{(x) => <QuestionCard ask={x} s={p.s} />}</For>
        </Show>
        <div class="clact-foot">{t("cards.activity.foot", "Fáze je odhad podle posledního nástroje; soubor a zpráva jsou z hooků Claude Code.")}</div>
      </Show>
    </div>
  );
}
