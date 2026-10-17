// Panel Nastavení „Průvodce“: stav průvodce prvním spuštěním a ruční spuštění (vždy od začátku).
// Samotný průvodce je stránka šuplíku `welcome` (`src/onboarding/Wizard.tsx`); tady jen tlačítko.
import { t } from "../../i18n";
import { startWizard } from "../../onboarding/state";
import type { PanelProps } from "../types";

export default function OnboardingPanel(p: PanelProps) {
  const done = () => p.cfg?.onboarding?.done !== false;
  return (
    <div class="card wide">
      <h4>{t("settings.welcome.title", "Průvodce prvním spuštěním")}</h4>
      <div class="opt">
        <span>
          {done() ? t("settings.welcome.done", "Proběhl nebo byl přeskočen") : t("settings.welcome.pending", "Ještě neproběhl — ukáže se při dalším otevření šuplíku")}
          <small>
            {t(
              "settings.welcome.lead",
              "Projde s tebou jazyk, projekty na disku, aplikace ze Start menu, Claude Code, odkazy a vzhled. Nic nemaže — jen přidává; co už v Ledge je, přeskočí.",
            )}
          </small>
        </span>
        <button class="mini ok" onClick={() => startWizard()}>
          {t("settings.welcome.run", "Spustit průvodce")}
        </button>
      </div>
    </div>
  );
}
