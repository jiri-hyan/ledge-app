// Panel „Záloha a aplikace“: záloha, import a obnova výchozích (`Backup.tsx`) a o aplikaci —
// verze, aktualizace, soubory Ledge (`About.tsx`). Dřív dva panely („Záloha a obnova“, „Aktualizace
// a o aplikaci“); jejich staré id vedou sem (`registry.ts` `PANEL_ALIASES`).
import { t } from "../../i18n";
import { Advanced } from "../fields/Advanced";
import { PanelHead } from "../fields/Row";
import type { PanelProps } from "../types";
import BackupPanel, { BackupAdvanced } from "./Backup";
import AboutPanel, { AboutAdvanced } from "./About";

export default function AppPanel(p: PanelProps) {
  return (
    <>
      <PanelHead title={t("settings.nav.aplikace", "Záloha a aplikace")} desc={t("settings.backup.desc", "Přenes nastavení na jiný počítač, vrať se k výchozímu, nebo se podívej, jakou verzi Ledge máš.")} />
      <BackupPanel {...p} />
      <AboutPanel {...p} />
      <Advanced id="aplikace" hint={t("settings.app.adv.hint", "Ruční úprava config.json, repo s vydáními, soubory a log")}>
        <BackupAdvanced />
        <AboutAdvanced />
      </Advanced>
    </>
  );
}
