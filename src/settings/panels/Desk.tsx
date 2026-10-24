// Stránka „Widgety na ploše“ (sekce Na ploše, kolo 21): widgety, profily rozmístění a výchozí chování
// nových widgetů (`desk/DeskSettings.tsx` → [desk.md]). Dřív sekce stránky Šuplík a karty. Nadpis „Na ploše“
// je jen tady (jako v navigaci), sekce pod ním mají vlastní, jiné nadpisy.
import { t } from "../../i18n";
import { PanelHead } from "../fields/Row";
import type { PanelProps } from "../types";
import DeskSettings from "../../desk/DeskSettings";

export default function DeskPanel(p: PanelProps) {
  return (
    <>
      <PanelHead title={t("settings.sec.naplose", "Na ploše")} desc={t("settings.desk.desc", "Karty Ledge jako malá okna na ploše nebo na druhém monitoru — žijí i se zavřeným šuplíkem. Připneš je i přes ⋯ na kartě → Připnout na plochu.")} />
      <DeskSettings s={p.s} />
    </>
  );
}
