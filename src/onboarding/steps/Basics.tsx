// Kroky Vítej (jazyk), Vzhled (výběr hotového motivu — jen hodnota `appearance.theme`, vykresluje
// sekce Vzhled) a Hotovo (souhrn uloženého + jak šuplík otevřít podle `drawer.trigger`).
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { STEPS } from "../logic";
import { chooseLang, setWiz, wiz } from "../store";
import { THEMES, type ThemeId } from "../../theme/themes";
import { savedAppearance, scopeStyle } from "../../theme/apply";

/** Motiv z nabídky Rustu, který UI zná → stejný název a náhled jako v Nastavení → Vzhled. */
const themeOf = (v: string): ThemeId | undefined => (v in THEMES ? (v as ThemeId) : undefined);

export function WelcomeStep() {
  const langs = [
    { id: "cs" as const, name: "Čeština", sample: "Ahoj! Nastavíme si Ledge." },
    { id: "en" as const, name: "English", sample: "Hi! Let's set up Ledge." },
  ];
  return (
    <div class="ob-hero">
      <div>
        <span class="ob-kicker">{t("onboarding.welcome.kicker", "Asi 2 minuty")}</span>
        <h2>{t("onboarding.welcome.title", "Vítej v Ledge")}</h2>
        <p>
          {t(
            "onboarding.welcome.lead",
            "Ledge je šuplík u horního okraje obrazovky: projekty a dev servery na jeden klik, aplikace, odkazy, poznámky a přehled o Claude Code. Projdeme to krok po kroku — každý krok jde přeskočit a všechno změníš později v Nastavení.",
          )}
        </p>
        <ol class="ob-plan">
          <For each={STEPS.slice(1, -1)}>{(s) => <li>{t(s.key, s.cs)}</li>}</For>
        </ol>
      </div>
      <div class="ob-langs" role="radiogroup" aria-label={t("onboarding.welcome.lang", "Jazyk")}>
        <h5>{t("onboarding.welcome.lang", "Jazyk")}</h5>
        <For each={langs}>
          {(l) => (
            <button type="button" role="radio" class="ob-lang" classList={{ on: wiz.lang === l.id }} aria-checked={wiz.lang === l.id} onClick={() => chooseLang(l.id)}>
              <b>{l.name}</b>
              <small>{l.sample}</small>
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

export function LookStep() {
  return (
    <div class="ob-center">
      <div class="ob-lead">
        <h3>{t("onboarding.look.title", "Vzhled")}</h3>
        <p>{t("onboarding.look.lead", "Vyber si motiv. Barvy, animace a další drobnosti doladíš v Nastavení → Vzhled a animace.")}</p>
      </div>
      <div class="ob-themes">
        <For each={wiz.themes}>
          {(o) => (
            <button type="button" class="ob-theme" classList={{ on: wiz.theme === o.value }} aria-pressed={wiz.theme === o.value} onClick={() => setWiz("theme", o.value)}>
              <Show when={themeOf(o.value)} fallback={<b>{o.label || o.value}</b>}>
                {(id) => (
                  <>
                    <span class="ap-swatch theme-scope" style={scopeStyle({ ...savedAppearance(), theme: id(), accent: "", glass: { opacity: 1 } })}>
                      <i class="l1" /><i class="l2" /><i class="l3" /><i class="acc" />
                    </span>
                    <b>{t(THEMES[id()].key, THEMES[id()].cs)}</b>
                  </>
                )}
              </Show>
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

function openTip(trigger: string): string {
  switch (trigger) {
    case "edge":
      return t("onboarding.done.tipEdge", "Šuplík otevřeš přitlačením myši k hornímu okraji obrazovky (uprostřed, u proužku).");
    case "dwell":
      return t("onboarding.done.tipDwell", "Šuplík otevřeš tak, že myš chvíli podržíš na proužku nahoře uprostřed.");
    case "off":
      return t("onboarding.done.tipOff", "Otevírání myší je vypnuté — šuplík otevřeš zkratkou (Nastavení → Zkratky).");
    default:
      return t("onboarding.done.tipHover", "Šuplík otevřeš najetím myší na proužek nahoře uprostřed obrazovky.");
  }
}

export function DoneStep() {
  const s = () => wiz.sum;
  const linksWhere = () => (s().linksTo === "actions" ? t("onboarding.done.inActions", "v rychlých akcích") : t("onboarding.done.inPersonal", "na záložce Osobní"));
  return (
    <div class="ob-center">
      <div class="ob-lead">
        <h3>{t("onboarding.done.title", "Hotovo — Ledge je tvůj")}</h3>
        <p>{openTip(wiz.trigger)}</p>
      </div>
      <div class="ob-summary">
        <div class="ob-sum">
          <b>{s().projects.length}</b>
          <span>{t("onboarding.done.projects", "projektů a aplikací přidáno")}</span>
          <Show when={s().projects.length}>
            <small>{s().projects.slice(0, 6).join(", ") + (s().projects.length > 6 ? "…" : "")}</small>
          </Show>
        </div>
        <div class="ob-sum">
          <b>{s().links}</b>
          <span>{t("onboarding.done.links", "odkazů")}</span>
          <Show when={s().links}>
            <small>{linksWhere()}</small>
          </Show>
        </div>
        <div class="ob-sum">
          <b>{s().hooks || wiz.hooks?.state === "full" ? "✓" : "—"}</b>
          <span>{t("onboarding.done.hooks", "hooky Claude Code")}</span>
        </div>
        <Show when={s().theme}>
          <div class="ob-sum">
            <b>{s().themeSaved === false ? "—" : "✓"}</b>
            <span>{t("onboarding.done.theme", "motiv {v}", { v: s().theme ?? "" })}</span>
            <Show when={s().themeSaved === false}>
              <small>{t("onboarding.done.themeLater", "tahle verze motivy ještě neumí")}</small>
            </Show>
          </div>
        </Show>
      </div>
      <ul class="ob-bullets">
        <li>{t("onboarding.done.tipBrowser", "V prohlížeči nebo maximalizovaném okně přitlač myš k hornímu okraji — šuplík vyjede i přes ně.")}</li>
        <li>{t("onboarding.done.tipAgain", "Průvodce kdykoli znovu: Nastavení → Průvodce. Všechno ostatní najdeš tamtéž.")}</li>
      </ul>
    </div>
  );
}
