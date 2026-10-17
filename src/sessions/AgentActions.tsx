// Agent View pod řádkem session na pozadí (karta Claude, `SessionRow`): Připojit (`claude attach` v novém
// terminálu), Logy (`claude logs` bez okna, jen na klik, výstup do rozbalení), Restartovat / Zastavit /
// Odebrat ze seznamu (s potvrzením v kartě, ne modálním oknem). Akce nabízí Rust (`job.actions`) a při
// kliku je znovu ověří. Poslat zprávu do běžící session z shellu docs neuvádějí → tady není; odpovídá se
// po Připojit v terminálu. Logika v `agentview.ts`.
import { createSignal, For, Show } from "solid-js";
import type { JobAction, SessionView } from "../types";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { agentApi } from "./agentApi";
import { actionHint, actionLabel, confirmText, needsConfirm, shownActions } from "./agentview";
import "./agentview.css";

/** Pilulka „na pozadí“ do řádku session. */
export function BgPill(p: { ss: SessionView }) {
  return (
    <Show when={p.ss.background}>
      <span class="pill n" title={p.ss.job ? t("sessions.av.bgHint", "Session na pozadí (Agent View) · id {id}", { id: p.ss.job.id }) : undefined}>{t("sessions.av.bg", "na pozadí")}</span>
    </Show>
  );
}

export default function AgentActions(p: { ss: SessionView }) {
  /** akce čekající na potvrzení */
  const [asking, setAsking] = createSignal<JobAction | null>(null);
  const [busy, setBusy] = createSignal<JobAction | null>(null);
  /** výstup logů; null = sbalené */
  const [log, setLog] = createSignal<{ text: string; cut: boolean } | null>(null);
  const actions = () => shownActions(p.ss.job);

  async function run(a: JobAction, confirmed: boolean) {
    if (busy()) return;
    setAsking(null);
    setBusy(a);
    try {
      const out = await agentApi.action(p.ss.id, a, confirmed);
      if (a === "logs") setLog({ text: out?.text ?? "", cut: !!out?.cut });
      else if (a !== "attach") say(out?.text || t("sessions.av.done", "Hotovo: {action}", { action: actionLabel(a) }));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(null);
    }
  }

  function click(a: JobAction) {
    if (a === "logs" && log()) return setLog(null);
    if (needsConfirm(a)) setAsking(a);
    else void run(a, false);
  }

  return (
    <Show when={p.ss.job}>
      {(job) => (
        <div class="sess-av">
          <Show when={asking()} fallback={
            <div class="sess-av-acts">
              <For each={actions()}>
                {(a) => (
                  <button class="mini" classList={{ ok: a === "attach" }} disabled={!!busy()} title={actionHint(a, job().id)} aria-expanded={a === "logs" ? !!log() : undefined} onClick={() => click(a)}>
                    {busy() === a ? "…" : a === "logs" && log() ? t("sessions.av.logsHide", "Skrýt logy") : actionLabel(a)}
                  </button>
                )}
              </For>
            </div>
          }>
            {(a) => (
              <div class="sess-av-ask" role="alertdialog" aria-label={confirmText(a(), p.ss.name)}>
                <span class="sess-av-q">{confirmText(a(), p.ss.name)}</span>
                <button class="mini" onClick={() => setAsking(null)}>{t("sessions.av.cancel", "Zrušit")}</button>
                <button class="mini ok" onClick={() => run(a(), true)}>{actionLabel(a())}</button>
              </div>
            )}
          </Show>
          <Show when={log()}>
            {(l) => (
              <>
                <Show when={l().cut}><div class="sess-av-note">{t("sessions.av.logsCut", "Jen konec výstupu (32 KB).")}</div></Show>
                <pre class="sess-av-log">{l().text || t("sessions.av.logsEmpty", "Žádný výstup.")}</pre>
                <button class="sess-av-link" disabled={!!busy()} onClick={() => run("logs", false)}>{t("sessions.av.logsAgain", "Načíst znovu")}</button>
              </>
            )}
          </Show>
        </div>
      )}
    </Show>
  );
}
