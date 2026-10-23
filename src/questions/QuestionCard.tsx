// Otázka Clauda (AskUserQuestion) jako v chatu: kdo se ptá, štítek, text, možnosti jako velká tlačítka,
// „Jiné…“ s vlastním textem, víc otázek po stránkách, „Odpovědět v Claude“ a zbývající čas. Odpověď →
// příkaz `question_answer` (Rust `claude/questions.rs`), výběr je čistá logika v `model.ts`. Fokus se
// bere jen po výslovném kliku do karty / na „Jiné…“ (pak klávesy 1–n, Enter, ←/→). Peek i karta Claude.
// Viditelná karta potvrdí „zobrazeno“ (`seen.ts`) — bez toho ji pojistka jádra po `questionUnseenS` pustí do Claude.
import { createEffect, createSignal, For, on, onCleanup, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { api, inTauri, type Ask, type QuestionReply, type Snapshot } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { answered, emptyDraft, instant, keyTarget, pick, remaining, toReply, toggleOther, type Draft } from "./model";
import { useReportSeen } from "./seen";
import "./questions.css";

const lastSeg = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

const send = (id: number, answers: QuestionReply[]) => (inTauri ? invoke<void>("question_answer", { id, answers }) : Promise.resolve());
const release = (id: number) => (inTauri ? invoke<boolean>("question_release", { id }) : Promise.resolve(false));

/** `inSession` = karta leží pod řádkem session (karta Claude) — bez opakování projektu a titulku. */
export default function QuestionCard(p: { ask: Ask; s: Snapshot; inSession?: boolean }) {
  const [page, setPage] = createSignal(0);
  const [drafts, setDrafts] = createSignal<Draft[]>(p.ask.questions.map(() => emptyDraft()));
  const [busy, setBusy] = createSignal(false);
  const [armed, setArmed] = createSignal(false);
  const [now, setNow] = createSignal(Date.now());
  let otherBox: HTMLTextAreaElement | undefined;
  let root: HTMLDivElement | undefined;
  // pojistka `claude.questionUnseenS`: jádru říct, že karta je opravdu vidět (`seen.ts`)
  useReportSeen(() => root, () => p.ask.id, () => p.ask.answerable);

  // jiná otázka ve stejné komponentě (nekeyed rodič) → začít znovu
  createEffect(on(() => p.ask.id, () => {
    setPage(0);
    setDrafts(p.ask.questions.map(() => emptyDraft()));
  }, { defer: true }));
  const timer = window.setInterval(() => setNow(Date.now()), 1000);
  onCleanup(() => clearInterval(timer));

  const qs = () => p.ask.questions;
  const q = () => qs()[Math.min(page(), qs().length - 1)];
  const d = () => drafts()[page()] ?? emptyDraft();
  const setD = (nd: Draft) => setDrafts((ds) => ds.map((x, i) => (i === page() ? nd : x)));
  const last = () => page() >= qs().length - 1;
  const allAnswered = () => drafts().length === qs().length && drafts().every(answered);
  const sess = () => p.s.sessions.find((x) => x.id === p.ask.sessionId);
  const where = () => sess()?.project ?? lastSeg(p.ask.cwd);
  const left = () => remaining(p.ask.expiresAt, now());
  const live = () => p.ask.answerable && !busy();

  /** Výslovný klik do karty → šuplík dostane klávesnici (jinak Ledge fokus nikdy nebere). */
  function arm() {
    if (armed() || !p.ask.answerable) return;
    setArmed(true);
    api.focusDrawer();
  }

  async function submit(ds = drafts()) {
    if (!live() || ds.length !== qs().length || !ds.every(answered)) return;
    setBusy(true);
    try {
      await send(p.ask.id, ds.map(toReply));
      say(t("questions.sent", "Odpověď odešla Claudovi"));
      if (armed()) api.releaseFocus();
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  function choose(label: string) {
    if (!live()) return;
    const nd = pick(q(), d(), label);
    if (instant(qs())) return void submit([nd]);
    setD(nd);
    if (!q().multiSelect && !last()) setPage(page() + 1);
  }

  function other() {
    if (!live()) return;
    const nd = toggleOther(q(), d());
    setD(nd);
    if (nd.otherOn) {
      arm();
      queueMicrotask(() => otherBox?.focus());
    }
  }

  function next() {
    if (!answered(d())) return;
    if (!last()) setPage(page() + 1);
    else void submit();
  }

  /** „Odpovědět v Claude“: Ledge otázku pustí a přepne do okna session (když ho zná). */
  async function toClaude() {
    if (busy()) return;
    setBusy(true);
    try {
      if (p.ask.answerable) await release(p.ask.id);
      if (armed()) api.releaseFocus();
      if (sess()?.canOpen) await api.sessionOpen(p.ask.sessionId);
      else say(t("questions.released", "Otázka je v okně Claude — odpověz tam"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  // klávesy jen po kliknutí do karty; capture před zkratkami šuplíku (1/2 = režim)
  const key = (e: KeyboardEvent) => {
    if (!armed() || !live() || e.ctrlKey || e.altKey || e.metaKey) return;
    if (["INPUT", "TEXTAREA", "SELECT"].includes((e.target as HTMLElement).tagName)) return;
    const k = keyTarget(q(), e.key);
    if (k === "other") other();
    else if (k) choose(k.label);
    else if (e.key === "Enter") next();
    else if (e.key === "ArrowRight" && !last()) setPage(page() + 1);
    else if (e.key === "ArrowLeft" && page() > 0) setPage(page() - 1);
    else return;
    e.preventDefault();
    e.stopPropagation();
  };
  window.addEventListener("keydown", key, true);
  onCleanup(() => window.removeEventListener("keydown", key, true));

  function otherKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (instant(qs())) void submit([d()]);
      else next();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      setD({ ...d(), otherOn: false });
    }
  }

  return (
    <div ref={root} class="qcard" classList={{ ro: !p.ask.answerable, "in-sess": !!p.inSession }} onPointerDown={(e) => !(e.target as HTMLElement).closest("button, textarea") && arm()}>
      <div class="q-head">
        <span class="q-ico" aria-hidden="true">?</span>
        <span class="q-who">
          <b>{p.inSession ? t("questions.asks", "Claude se ptá") : t("questions.asksIn", "Claude se ptá · {where}", { where: where() })}</b>
          <Show when={!p.inSession && sess()?.title}>{(title) => <small>{title()}</small>}</Show>
        </span>
        <span class="q-left">
          {p.ask.answerable
            ? left() && t("questions.left", "čeká ještě {left}", { left: left() })
            : t("questions.readOnly", "odpověz v okně Claude")}
        </span>
      </div>
      <Show when={qs().length > 1}>
        <div class="q-steps">
          <For each={qs()}>
            {(x, i) => (
              <button classList={{ cur: i() === page(), done: answered(drafts()[i()] ?? emptyDraft()) }} onClick={() => setPage(i())}>
                <span>{i() + 1}</span>{x.header || x.question}
              </button>
            )}
          </For>
        </div>
      </Show>
      <div class="q-body">
        <Show when={q().header}><span class="q-tag">{q().header}</span></Show>
        <p class="q-text">{q().question}</p>
        <Show when={q().multiSelect}><small class="q-note">{t("questions.multi", "Vyber jednu nebo víc možností")}</small></Show>
        <div class="q-opts" role={q().multiSelect ? "group" : "radiogroup"}>
          <For each={q().options}>
            {(o, i) => {
              const on = () => d().labels.includes(o.label);
              return (
                <button class="q-opt" classList={{ on: on(), multi: q().multiSelect }} disabled={!live()} aria-pressed={on()} onClick={() => choose(o.label)}>
                  <span class="q-key">{q().multiSelect && on() ? "✓" : i() + 1}</span>
                  <span class="q-txt">
                    <b>{o.label}</b>
                    <Show when={o.description}><small>{o.description}</small></Show>
                    <Show when={o.preview}><pre class="q-preview">{o.preview}</pre></Show>
                  </span>
                </button>
              );
            }}
          </For>
          <Show when={p.ask.answerable}>
            <button class="q-opt other" classList={{ on: d().otherOn }} disabled={!live()} aria-pressed={d().otherOn} onClick={other}>
              <span class="q-key">{d().otherOn && q().multiSelect ? "✓" : q().options.length + 1}</span>
              <span class="q-txt"><b>{t("questions.other", "Jiné…")}</b><small>{t("questions.otherHint", "napíšu vlastní odpověď")}</small></span>
            </button>
          </Show>
        </div>
        <Show when={d().otherOn}>
          <textarea
            ref={otherBox}
            class="q-other"
            rows={2}
            placeholder={t("questions.otherPlaceholder", "Tvoje odpověď… (Enter odešle, Shift+Enter nový řádek)")}
            value={d().other}
            disabled={!live()}
            onInput={(e) => setD({ ...d(), other: e.currentTarget.value })}
            onKeyDown={otherKey}
          />
        </Show>
      </div>
      <div class="q-foot">
        <button class="mini" disabled={busy()} onClick={toClaude}>
          {p.ask.answerable ? t("questions.toClaude", "Odpovědět v Claude") : t("questions.switch", "Přepnout do Claude")}
        </button>
        <Show when={armed() && p.ask.answerable}><small class="q-keys">{t("questions.keys", "1–{n} vybrat · Enter dál", { n: q().options.length + 1 })}</small></Show>
        <span class="grow" />
        <Show when={p.ask.answerable && (!instant(qs()) || d().otherOn)}>
          <Show when={page() > 0}><button class="mini" onClick={() => setPage(page() - 1)}>{t("questions.back", "Zpět")}</button></Show>
          <Show when={last()} fallback={<button class="btn" disabled={!answered(d()) || busy()} onClick={next}>{t("questions.next", "Další")}</button>}>
            <button class="btn pri" disabled={!allAnswered() || busy()} onClick={() => void submit()}>{t("questions.send", "Odeslat")}</button>
          </Show>
        </Show>
      </div>
    </div>
  );
}
