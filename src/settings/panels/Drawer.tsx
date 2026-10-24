// Panel „Šuplík a karty“: CO šuplík ukazuje — záložky a karty (`layout/LayoutEditor.tsx`: záložky,
// karty, velikost, části karet, „Upravit přímo v šuplíku“ → [layout.md]), přihrádka na soubory a widgety
// na ploše (`desk/DeskSettings.tsx` → [desk.md]).
// Jak šuplík vypadá (i celý proužek) je v panelu Vzhled, jak se otevírá a chová v panelu Chování.
import { t } from "../../i18n";
import { Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import type { PanelProps } from "../types";
import LayoutEditor from "../../layout/LayoutEditor";
import DeskSettings from "../../desk/DeskSettings";

export default function DrawerPanel(p: PanelProps) {
  return (
    <>
      <PanelHead title={t("settings.nav.suplik", "Šuplík a karty")} desc={t("settings.drawer.desc", "Co šuplík ukazuje: záložky, karty na nich, jejich pořadí a velikost. Rychleji to jde přímo v šuplíku přes ✎.")} />
      <LayoutEditor />
      <Section title={t("settings.drawer.shelf", "Přihrádka")}>
        <Row id="shelf.enabled"><Toggle path="shelf.enabled" label={fieldLabel("shelf.enabled")} /></Row>
      </Section>
      <DeskSettings s={p.s} />
    </>
  );
}
