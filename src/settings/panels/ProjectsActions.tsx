// Panel „Projekty, akce a odkazy“: všechno, co se otevírá jedním klikem, na jednom místě — projekty
// (`Projects.tsx`, hlavička panelu s přidáním), jeden seznam akcí a odkazů (`Actions.tsx`; odkaz = akce
// typu web / složka / soubor) a jen když nějaké zbyly, starší odkazy, které se nepřevedly na akce
// (`LegacyLinks.tsx`). Skripty (`actions/ScriptsSettings.tsx`) a widgety ze skriptů (`widgets/WidgetsSettings.tsx`
// — widget je skript, který se spouští sám a kreslí kartu / položku proužku) jsou v „Pokročilé“: pro ty,
// kdo si skripty píšou sami.
import { t } from "../../i18n";
import { Advanced } from "../fields/Advanced";
import type { PanelProps } from "../types";
import ProjectsPanel from "./Projects";
import ActionsPanel from "./Actions";
import LegacyLinks from "./LegacyLinks";
import WidgetsSettings from "../../widgets/WidgetsSettings";
import ScriptsSettings from "../../actions/ScriptsSettings";

export default function ProjectsActionsPanel(p: PanelProps) {
  return (
    <>
      <ProjectsPanel {...p} />
      <ActionsPanel {...p} />
      <LegacyLinks />
      <Advanced id="akce" hint={t("settings.proj.adv.hint", "Vlastní skripty v paletě a widgety ze skriptů")}>
        <ScriptsSettings />
        <WidgetsSettings />
      </Advanced>
    </>
  );
}
