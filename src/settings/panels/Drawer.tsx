// Stránka „Záložky a karty“ (sekce Šuplík a karty): CO šuplík ukazuje — záložky a karty (`layout/LayoutEditor.tsx`:
// záložky, karty, velikost, části karet, „Upravit přímo v šuplíku“ → [layout.md]) a přihrádka na soubory.
// Widgety na ploše mají od kola 21 vlastní stránku Na ploše (`panels/Desk.tsx`).
// Jak šuplík vypadá (i celý proužek) je v panelu Vzhled, jak se otevírá na stránce Otevírání a klávesy.
import { t } from "../../i18n";
import { Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import type { PanelProps } from "../types";
import LayoutEditor from "../../layout/LayoutEditor";

export default function DrawerPanel(_p: PanelProps) {
  return (
    <>
      <PanelHead title={t("settings.nav.suplik", "Záložky a karty")} desc={t("settings.drawer.desc", "Co šuplík ukazuje: záložky, karty na nich, jejich pořadí a velikost. Rychleji to jde přímo v šuplíku přes ✎.")} />
      <LayoutEditor />
      <Section title={t("settings.drawer.shelf", "Přihrádka")}>
        <Row id="shelf.enabled"><Toggle path="shelf.enabled" label={fieldLabel("shelf.enabled")} /></Row>
      </Section>
    </>
  );
}
