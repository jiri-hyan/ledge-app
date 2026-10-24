// Kroky Vítej (jazyk), Vzhled (výběr hotového motivu — jen hodnota `appearance.theme`, vykresluje
// sekce Vzhled) a Hotovo (souhrn uloženého + jak šuplík otevřít podle `drawer.trigger`).
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { doneRows, estimateMinutes } from "../logic";
import { chooseLang, setWiz, steps, wiz } from "../store";
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
        <span class="ob-kicker">{t("onboarding.welcome.kicker", "Asi {n} min", { n: estimateMinutes(steps()) })}</span>
        <h2>{t("onboarding.welcome.title", "Vítej v Ledge")}</h2>
        <p>
          {t(
            "onboarding.welcome.lead",
            "Ledge je šuplík u horního okraje obrazovky: projekty a dev servery na jeden klik, aplikace, odkazy, poznámky a přehled o Claude Code. Projdeme to krok po kroku — každý krok jde přeskočit a všechno změníš později v Nastavení.",
          )}
        </p>
        <ol class="ob-plan">
          <For each={steps().slice(1, -1)}>{(s) => <li>{t(s.key, s.cs)}</li>}</For>
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
        <p>{t("onboarding.look.lead", "Vyber si motiv. Jak vypadá zavřený Ledge, co se stane po najetí myší, šířku a animace vybereš z obrázků v Nastavení → Vzhled (nebo Začínáme → Vyber si vzhled).")}</p>
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
      return t("onboarding.done.tipOff", "Otevírání myší je vypnuté — šuplík otevřeš zkratkou (Nastavení → Šuplík a karty → Otevírání a klávesy).");
    default:
      return t("onboarding.done.tipHover", "Šuplík otevřeš najetím myší na proužek nahoře uprostřed obrazovky.");
  }
}

/** Jméno motivu pro souhrn: z `THEMES` (stejně jako v Nastavení → Vzhled), jinak popisek z nabídky Rustu; neznámé = `null`. */
function themeName(v: string | null): string | null {
  if (!v) return null;
  const id = () => themeOf(v) as ThemeId; // (zápis `THEMES[id()]` jako v LookStep — vědomý dynamický klíč v i18n.test.mjs)
  if (themeOf(v)) return t(THEMES[id()].key, THEMES[id()].cs);
  return wiz.themes.find((o) => o.value === v)?.label || null;
}

export function DoneStep() {
  const rows = () => doneRows(wiz.sum, wiz.hooks?.state === "full", themeName(wiz.sum.theme));
  return (
    <div class="ob-center">
      <div class="ob-lead">
        <h3>{t("onboarding.done.title", "Hotovo — Ledge je tvůj")}</h3>
        <p>{rows().length ? openTip(wiz.trigger) : t("onboarding.done.nothing", "Hotovo — Ledge najdeš u horního okraje obrazovky.")}</p>
      </div>
      <Show when={rows().length}>
        <div class="ob-summary">
          <For each={rows()}>
            {(r) => (
              <div class="ob-sum">
                <b>{r.kind === "projects" || r.kind === "links" ? r.n : "✓"}</b>
                <span>
                  {r.kind === "projects" && t("onboarding.done.projects", "projektů a aplikací přidáno")}
                  {r.kind === "links" && t("onboarding.done.links", "odkazů")}
                  {r.kind === "theme" && t("onboarding.done.theme", "motiv {v}", { v: r.name })}
                  {r.kind === "hooks" && t("onboarding.done.hooks", "hooky Claude Code")}
                </span>
                <Show when={r.kind === "projects" ? r.names : r.kind === "links" ? t("onboarding.done.inActions", "v kartě Odkazy") : ""}>{(small) => <small>{small()}</small>}</Show>
              </div>
            )}
          </For>
        </div>
      </Show>
      <ul class="ob-bullets">
        <li>{t("onboarding.done.tipTray", "Ikonka Ledge v oznamovací oblasti (vedle hodin) šuplík otevře a Ledge tam i ukončíš.")}</li>
        <li>{t("onboarding.done.tipLayout", "Uprav si šuplík — ✎ v hlavičce: přetáhni karty, změň jejich velikost, přidej je z katalogu nebo si založ vlastní záložku.")}</li>
        <li>{t("onboarding.done.tipAgain", "Průvodce kdykoli znovu: Nastavení → Aplikace → Jazyk a průvodce. Všechno ostatní najdeš tamtéž.")}</li>
      </ul>
    </div>
  );
}
