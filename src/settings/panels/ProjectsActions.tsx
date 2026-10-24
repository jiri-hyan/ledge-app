// Stránka „Projekty, odkazy a akce“ (kolo 21): všechno, co se otevírá jedním klikem, v sbalovacích
// skupinách (`fields/Group.tsx`) — nejčastější nahoře a otevřené: Projekty (`Projects.tsx`, hlavička stránky
// s přidáním) a Odkazy a akce (`Actions.tsx`; odkaz = akce typu web / složka / soubor); pod nimi sbalené
// Upozornění na neuloženou práci (`GitAlerts`) a Kde hledat repa (`Roots`); jen když nějaké zbyly, starší
// odkazy, které se nepřevedly na akce (`LegacyLinks.tsx`). Skripty (`actions/ScriptsSettings.tsx`) a widgety
// ze skriptů (`widgets/WidgetsSettings.tsx` — widget je skript, který se spouští sám a kreslí kartu / položku
// proužku) jsou v „Pokročilé“: pro ty, kdo si skripty píšou sami.
import { t } from "../../i18n";
import { Advanced } from "../fields/Advanced";
import { Group } from "../fields/Group";
import type { PanelProps } from "../types";
import ProjectsPanel, { GitAlerts, Roots } from "./Projects";
import ActionsPanel from "./Actions";
import LegacyLinks from "./LegacyLinks";
import WidgetsSettings from "../../widgets/WidgetsSettings";
import ScriptsSettings from "../../actions/ScriptsSettings";

export default function ProjectsActionsPanel(p: PanelProps) {
  const roots = () => (p.cfg.roots ?? []).length;
  return (
    <>
      <ProjectsPanel {...p} />
      <ActionsPanel {...p} />
      <LegacyLinks />
      <Group gid="akce.git" title={t("settings.proj.gGit", "Upozornění na neuloženou práci")} desc={t("settings.proj.gGitDesc", "Kdy ti Ledge připomene necommitnuté nebo nepushnuté změny v repech projektů.")} open={false}>
        <GitAlerts />
      </Group>
      <Group gid="akce.roots" title={t("settings.proj.gRoots", "Složky s repy")} desc={t("settings.proj.gRootsDesc", "Složky s naklonovanými repy — odtud Ledge najde projekty na každém počítači a sem klonuje nové.")} open={false} badge={roots() ? String(roots()) : undefined}>
        <Roots />
      </Group>
      <Advanced id="akce" hint={t("settings.proj.adv.hint", "Vlastní skripty v paletě a widgety ze skriptů")}>
        <ScriptsSettings />
        <WidgetsSettings />
      </Advanced>
    </>
  );
}
