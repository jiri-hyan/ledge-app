// Průvodce prvním spuštěním — stránka šuplíku `welcome` (vkládá ji App, když `wizardOpen()`).
// Hlavička s postupem, tělo kroku s plynulým přechodem (bez animace při prefers-reduced-motion),
// patička Zpět / Přeskočit krok / Uložit a pokračovat. Každý krok se ukládá hned (`applyStep`).
import { createEffect, createMemo, For, Match, on, onMount, Show, Switch } from "solid-js";
import { t } from "../i18n";
import { STEPS, reducedMotion, type StepId } from "./logic";
import { closeWizard } from "./state";
import { applyStep, finish, pending, prefetch, setWiz, wiz } from "./store";
import ProjectsStep from "./steps/Projects";
import { AssistStep } from "./steps/Assist";
import AppsStep from "./steps/Apps";
import { ClaudeStep, LinksStep } from "./steps/Extras";
import { DoneStep, LookStep, WelcomeStep } from "./steps/Basics";
import "./onboarding.css";

export default function Wizard(p: { onExit: () => void }) {
  onMount(() => {
    if (wiz.reposState === "idle") void prefetch();
  });
  // Vzhled jen když config sekci `appearance` s motivy má (jinak by výběr nic neuložil)
  const steps = createMemo(() => STEPS.filter((s) => s.id !== "look" || wiz.themesState !== "ready" || wiz.themes.length > 0));
  const idx = () => Math.min(wiz.step, steps().length - 1);
  const cur = (): StepId => steps()[idx()].id;
  const last = () => idx() === steps().length - 1;

  let body!: HTMLDivElement;
  createEffect(
    on(
      () => wiz.step,
      () => {
        if (reducedMotion()) return;
        body.animate([{ opacity: 0, transform: `translateX(${wiz.dir * 18}px)` }, { opacity: 1, transform: "none" }], { duration: 260, easing: "cubic-bezier(.2,.9,.25,1)" });
      },
      { defer: true },
    ),
  );

  async function exit() {
    if (wiz.busy) return;
    setWiz("busy", true);
    const ok = await finish();
    setWiz("busy", false);
    if (!ok) return;
    closeWizard();
    p.onExit();
  }

  async function next(save: boolean) {
    if (wiz.busy) return;
    if (last()) return exit();
    if (save && !(await applyStep(cur()))) return;
    setWiz({ dir: 1, step: idx() + 1 });
  }

  const back = (to = idx() - 1) => !wiz.busy && to >= 0 && setWiz({ dir: -1, step: to });
  const primary = () => (cur() === "welcome" ? t("onboarding.start", "Začít") : last() ? t("onboarding.finish", "Hotovo") : pending(cur()) ? t("onboarding.saveNext", "Uložit a pokračovat") : t("onboarding.next", "Další"));

  return (
    <div class="ob" data-anim>
      <header class="ob-top">
        <div class="ob-brand">
          <span class="ob-mark" aria-hidden="true" />
          <div>
            <b>{t("onboarding.title", "Nastavení Ledge")}</b>
            <small>{t("onboarding.progress", "Krok {n} z {m}", { n: idx() + 1, m: steps().length })}</small>
          </div>
        </div>
        <ol class="ob-steps">
          <For each={steps()}>
            {(s, i) => (
              <li classList={{ on: i() === idx(), done: i() < idx() }}>
                <button type="button" disabled={i() >= idx() || wiz.busy} onClick={() => back(i())} aria-current={i() === idx() ? "step" : undefined}>
                  <span class="ob-num">{i() < idx() ? "✓" : i() + 1}</span>
                  <span class="ob-step-name">{t(s.key, s.cs)}</span>
                </button>
              </li>
            )}
          </For>
        </ol>
        <button class="mini" disabled={wiz.busy} onClick={() => void exit()}>
          {last() ? t("onboarding.close", "Zavřít") : t("onboarding.skipAll", "Přeskočit průvodce")}
        </button>
      </header>
      <div class="ob-bar" aria-hidden="true">
        <i style={{ transform: `scaleX(${(idx() + 1) / steps().length})` }} />
      </div>
      <div class="ob-body" ref={body}>
        <Switch>
          <Match when={cur() === "welcome"}><WelcomeStep /></Match>
          <Match when={cur() === "assist"}><AssistStep /></Match>
          <Match when={cur() === "projects"}><ProjectsStep /></Match>
          <Match when={cur() === "apps"}><AppsStep /></Match>
          <Match when={cur() === "claude"}><ClaudeStep /></Match>
          <Match when={cur() === "links"}><LinksStep /></Match>
          <Match when={cur() === "look"}><LookStep /></Match>
          <Match when={cur() === "done"}><DoneStep /></Match>
        </Switch>
      </div>
      <footer class="ob-foot">
        <button class="btn" disabled={idx() === 0 || wiz.busy} onClick={() => back()}>
          {t("onboarding.back", "Zpět")}
        </button>
        <span class="ob-foot-hint">{t("onboarding.hint", "Všechno jde později změnit v Nastavení.")}</span>
        <Show when={!last() && cur() !== "welcome"}>
          <button class="btn ghost" disabled={wiz.busy} onClick={() => void next(false)}>
            {t("onboarding.skipStep", "Přeskočit krok")}
          </button>
        </Show>
        <button class="btn pri" disabled={wiz.busy} onClick={() => void next(true)}>
          {wiz.busy ? t("onboarding.saving", "Ukládám…") : primary()}
        </button>
      </footer>
    </div>
  );
}
