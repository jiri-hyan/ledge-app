// „Claude na pozadí…“ z dlaždice projektu (`BgTileButton`) nebo palety (`openBgLaunch`): formulář s úkolem
// a volitelným jménem → `claude --bg [--name N] "<úkol>"` ve složce projektu (Rust `agent_spawn`, pole
// argumentů, žádný shell). Fokus si šuplík vezme jen po tomhle výslovném kliku a po odeslání / zrušení ho
// vrátí. Formulář kreslí `BgLaunch` jednou na hlavní stránce (`MainPage.tsx`). Kontroly v `agentview.ts`.
import { createEffect, createSignal, Show } from "solid-js";
import { api, type ProjectView, type Snapshot } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { agentApi } from "./agentApi";
import { NAME_MAX, PROMPT_MAX, bgKey, nameProblem, promptProblem } from "./agentview";
import "./agentview.css";

/** id projektu s otevřeným formulářem (jeden pro celý šuplík) */
const [target, setTarget] = createSignal<string | null>(null);
/** fokus vzal formulář (vrátit jen tehdy a jen jednou) */
let took = false;

/** Otevře formulář pro projekt — volat jen z výslovného kliku / volby v paletě. */
export function openBgLaunch(projectId: string) {
  setTarget(projectId);
  if (!took) {
    took = true;
    api.focusDrawer();
  }
}

function closeBgLaunch() {
  setTarget(null);
  if (took) {
    took = false;
    api.releaseFocus();
  }
}

/** Nabízet „Claude na pozadí…“ u projektu? (most Clauda zapnutý, složka existuje) */
export const canBgLaunch = (s: Snapshot, pr: ProjectView) => s.claudeEnabled && pr.pathOk;

export const IconBgClaude = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <rect x="3" y="5" width="18" height="14" rx="2" />
    <path d="m8 10 3 2-3 2" />
    <path d="M13 15h3" />
    <path d="M18 2v2M21 3h-2" />
  </svg>
);

/** Tlačítko pod dlaždicí projektu. */
export function BgTileButton(p: { s: Snapshot; pr: ProjectView }) {
  return (
    <Show when={canBgLaunch(p.s, p.pr)}>
      <button title={t("sessions.av.bg.tile", "Claude na pozadí…")} aria-label={t("sessions.av.bg.tileFor", "Claude na pozadí v {name}", { name: p.pr.name })} onClick={() => openBgLaunch(p.pr.id)}>
        <IconBgClaude />
      </button>
    </Show>
  );
}

export default function BgLaunch(p: { s: Snapshot }) {
  const project = () => p.s.projects.find((x) => x.id === target());
  // projekt zmizel z configu (nebo už nejde spustit) s otevřeným formulářem → zavřít a vrátit fokus
  createEffect(() => {
    const pr = project();
    if (target() !== null && (!pr || !canBgLaunch(p.s, pr))) closeBgLaunch();
  });
  return <Show when={project()}>{(pr) => <BgForm pr={pr()} />}</Show>;
}

function BgForm(p: { pr: ProjectView }) {
  const [prompt, setPrompt] = createSignal("");
  const [name, setName] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [tried, setTried] = createSignal(false);
  let box: HTMLTextAreaElement | undefined;
  queueMicrotask(() => box?.focus());
  const problem = () => promptProblem(prompt()) ?? nameProblem(name());

  async function send() {
    setTried(true);
    if (busy() || problem()) return;
    setBusy(true);
    try {
      const msg = await agentApi.spawn(p.pr.id, prompt(), name().trim() || null);
      say(msg ?? t("sessions.av.bg.started", "Claude běží na pozadí."));
      closeBgLaunch();
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  function key(e: KeyboardEvent) {
    const k = bgKey(e.key, e.ctrlKey || e.metaKey);
    if (!k) return;
    e.preventDefault();
    e.stopPropagation(); // Esc jinak zavře šuplík
    if (k === "cancel") closeBgLaunch();
    else void send();
  }

  return (
    <div class="card sess-bg" data-anim role="dialog" aria-label={t("sessions.av.bg.title", "Claude na pozadí · {name}", { name: p.pr.name })} onKeyDown={key}>
      <h4><span class="ic" style="background:var(--violet)" />{t("sessions.av.bg.title", "Claude na pozadí · {name}", { name: p.pr.name })}</h4>
      <div class="sess-bg-where" title={p.pr.path ?? ""}>claude --bg · {p.pr.path ?? ""}</div>
      <textarea ref={box} class="sess-bg-prompt" rows={3} maxLength={PROMPT_MAX} value={prompt()} disabled={busy()}
        placeholder={t("sessions.av.bg.placeholder", "Co má Claude udělat? (Ctrl+Enter spustí, Esc zavře)")}
        onInput={(e) => setPrompt(e.currentTarget.value)} />
      <input class="sess-bg-name" type="text" maxLength={NAME_MAX} value={name()} disabled={busy()} spellcheck={false}
        placeholder={t("sessions.av.bg.name", "Jméno session (nepovinné)")} onInput={(e) => setName(e.currentTarget.value)} />
      <div class="sess-bg-foot">
        <Show when={tried() && problem()}><span class="sess-bg-err">{problem()}</span></Show>
        <button class="mini" disabled={busy()} onClick={closeBgLaunch}>{t("sessions.av.cancel", "Zrušit")}</button>
        <button class="mini ok" disabled={busy() || (tried() && !!problem())} onClick={send}>{busy() ? t("sessions.av.bg.starting", "Spouštím…") : t("sessions.av.bg.run", "Spustit na pozadí")}</button>
      </div>
    </div>
  );
}
