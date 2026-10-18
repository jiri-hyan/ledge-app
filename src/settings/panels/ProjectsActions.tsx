// Panel „Projekty, akce a odkazy“: všechno, co se otevírá jedním klikem, na jednom místě — projekty
// (`Projects.tsx`, hlavička panelu s přidáním), jeden seznam akcí a odkazů (`Actions.tsx`; odkaz = akce
// typu web / složka / soubor), skripty (`actions/ScriptsSettings.tsx`, v sekci akcí) a jen když nějaké
// zbyly, starší odkazy, které se nepřevedly na akce (`LegacyLinks.tsx`).
import type { PanelProps } from "../types";
import ProjectsPanel from "./Projects";
import ActionsPanel from "./Actions";
import LegacyLinks from "./LegacyLinks";

export default function ProjectsActionsPanel(p: PanelProps) {
  return (
    <>
      <ProjectsPanel {...p} />
      <ActionsPanel {...p} />
      <LegacyLinks />
    </>
  );
}
