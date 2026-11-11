// Zadání pro Claude Code v editoru poznámky: čip stavu + náklad, Poslat / Pokračovat / Zastavit, otázky s polem
// pro odpověď, body ke kontrole `[~]` (Potvrdit / Vrátit), sbalitelný Log. Data jen z těla a polí `NoteView`;
// zápisy dělá jádro (`note_answer`, `note_task_set`, `brief_*`) a editor se po nich obnoví sám (`notesRev`).

import { createSignal, For, onMount, Show } from "solid-js";
import { api, type BriefStatus } from "../api";
import { t } from "../i18n";
import { say } from "../ui/toast";
import { hasKey, refreshKey } from "./apikey";
import { briefAction, briefError, costText, logLines, parseQuestions, parseTasks, type BriefTask, type Question, type SendBlock } from "./brief";

export const statusText = (s: BriefStatus): string =>
  ({
    draft: t("notes.brief.status.draft", "Koncept"),
    ready: t("notes.brief.status.ready", "Připraveno"),
    running: t("notes.brief.status.running", "Běží"),
    waiting: t("notes.brief.status.waiting", "Čeká na odpověď"),
    review: t("notes.brief.status.review", "Ke kontrole"),
    done: t("notes.brief.status.done", "Hotovo"),
  })[s];

const blockText = (b: SendBlock): string =>
  ({
    project: t("notes.brief.block.project", "Vyber projekt, ve kterém má session běžet."),
    goal: t("notes.brief.block.goal", "Napiš na první řádek, o co jde."),
    key: t("notes.brief.block.key", "Chybí API klíč — ulož ho v Nastavení → Claude."),
    state: t("notes.brief.block.state", "V tomhle stavu se zadání poslat nedá."),
  })[b];

/** Čip stavu a náklad (jen z reálných polí; bez nich nic) — sdílí editor i řádek karty. */
export function BriefChip(p: { n: { status?: BriefStatus | null; cost?: number | null; budget?: number | null } }) {
  return (
    <>
      <Show when={p.n.status}>{(s) => <span class="pill brief-chip" classList={{ g: s() === "done", n: s() === "running" || s() === "waiting" || s() === "review" }} title={t("notes.brief.chip", "Stav zadání")}>{statusText(s())}</span>}</Show>
      <Show when={costText(p.n.cost, p.n.budget)}>{(c) => <span class="pill brief-cost" title={t("notes.brief.cost", "Náklad session / limit (z API klíče)")}>{c()}</span>}</Show>
    </>
  );
}

export interface BriefNote { path: string; status?: BriefStatus | null; cost?: number | null; budget?: number | null; project: string | null }

export default function BriefPanel(p: { n: BriefNote; body: string; dirty: boolean }) {
  const [busy, setBusy] = createSignal(false);
  const [answers, setAnswers] = createSignal<Record<number, string>>({});
  onMount(refreshKey);
  const tasks = () => parseTasks(p.body);
  const review = () => tasks().filter((x) => x.state === "review");
  const questions = () => parseQuestions(p.body);
  const log = () => logLines(p.body);
  const action = () => briefAction({ status: p.n.status, project: p.n.project, body: p.body }, hasKey());

  /** Zápis do souboru dělá jádro — rozepsaná úprava by se s ním minula, proto nejdřív uložit. */
  async function run(f: () => Promise<unknown>, ok?: string) {
    if (busy()) return;
    if (p.dirty) return say(t("notes.brief.saveFirst", "Nejdřív ulož úpravu (Ctrl+Enter)."), true);
    setBusy(true);
    try {
      await f();
      if (ok) say(ok);
    } catch (e) {
      say(briefError(e, t("notes.brief.oldCore", "Tuhle část Ledge ještě neumí — aktualizuj aplikaci.")), true);
    } finally {
      setBusy(false);
    }
  }
  const send = () => run(() => api.briefSend(p.n.path), t("notes.brief.sent", "Zadání odesláno orchestrátorovi."));
  const answer = (q: Question) => {
    const a = (answers()[q.index] ?? "").trim();
    if (a) void run(() => api.noteAnswer(p.n.path, q.index, a)).then(() => setAnswers({ ...answers(), [q.index]: "" }));
  };
  const setState = (task: BriefTask, state: "open" | "done") => run(() => api.noteTaskSet(p.n.path, task.line, task.text, state, state === "done" ? task.evidence : null));

  return (
    <div class="brief">
      <div class="brief-head">
        <BriefChip n={p.n} />
        <span class="grow" />
        {(() => {
          const a = action();
          if (a.kind === "send")
            return <button class="btn pri" disabled={busy() || a.block !== null} title={a.block ? blockText(a.block) : undefined} onClick={send}>{t("notes.brief.send", "Poslat orchestrátorovi")}</button>;
          if (a.kind === "resume")
            return <button class="btn pri" disabled={busy() || a.block !== null} title={a.block ? t("notes.brief.block.answers", "Nejdřív odpověz na všechny otázky.") : undefined} onClick={() => run(() => api.briefResume(p.n.path))}>{t("notes.brief.resume", "Pokračovat")}</button>;
          if (a.kind === "stop") return <button class="btn" disabled={busy()} onClick={() => run(() => api.briefStop(p.n.path))}>{t("notes.brief.stop", "Zastavit")}</button>;
          return null;
        })()}
      </div>
      {(() => {
        const a = action();
        return a.kind === "send" && a.block ? <div class="hint brief-why">{blockText(a.block)}</div> : null;
      })()}
      <Show when={questions().length > 0}>
        <div class="brief-sec">{t("notes.brief.questions", "Otázky")}</div>
        <For each={questions()}>
          {(q) => (
            <div class="brief-q">
              <div class="brief-qt">❓ {q.text}</div>
              <Show when={q.answer !== null} fallback={
                <div class="brief-qa">
                  <input
                    class="brief-in"
                    placeholder={t("notes.brief.answer", "Tvoje odpověď…")}
                    value={answers()[q.index] ?? ""}
                    onInput={(e) => setAnswers({ ...answers(), [q.index]: e.currentTarget.value })}
                    onKeyDown={(e) => e.key === "Enter" && !e.isComposing && (e.preventDefault(), answer(q))}
                  />
                  <button class="mini ok" disabled={busy() || !(answers()[q.index] ?? "").trim()} onClick={() => answer(q)}>{t("notes.brief.answerBtn", "Odpovědět")}</button>
                </div>
              }>
                <div class="brief-ans">↳ {q.answer}</div>
              </Show>
            </div>
          )}
        </For>
      </Show>
      <Show when={review().length > 0}>
        <div class="brief-sec">{t("notes.brief.review", "Ke kontrole ({n})", { n: review().length })}</div>
        <For each={review()}>
          {(task) => (
            <div class="brief-t">
              <span class="brief-tilde" title={t("notes.brief.proposed", "Navrženo jako hotové — čeká na tebe")}>◔</span>
              <span class="grow">
                <span class="brief-tt">{task.text}</span>
                <Show when={task.evidence}>{(ev) => <small class="brief-ev">{t("notes.brief.evidence", "důkaz: {e}", { e: ev() })}</small>}</Show>
              </span>
              <button class="mini ok" disabled={busy()} onClick={() => setState(task, "done")}>{t("notes.brief.confirm", "Potvrdit")}</button>
              <button class="mini" disabled={busy()} onClick={() => setState(task, "open")}>{t("notes.brief.return", "Vrátit")}</button>
            </div>
          )}
        </For>
      </Show>
      <Show when={log().length > 0}>
        <details class="brief-log">
          <summary>{t("notes.brief.log", "Log ({n})", { n: log().length })}</summary>
          <pre>{log().join("\n")}</pre>
        </details>
      </Show>
    </div>
  );
}
