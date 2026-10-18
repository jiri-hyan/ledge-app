// Lidské názvy z modelu rozložení (páry klíč + čeština v `model.ts`) — překlad až při vykreslení.
// Dynamická karta (widget ze skriptu) má název z hlavičky skriptu (`widgets/registry.ts`), jinak soubor.
import { t } from "../i18n";
import { widgetInfo } from "../widgets/registry";
import { widgetFile } from "../widgets/model";
import { builtinName, CARD_INFO, isBuiltin, PART_INFO, type CardId, type LayoutPage, type PartKey, type Text } from "./model";

export const tx = (x: Text): string => t(x[0], x[1]);
export const cardName = (id: CardId): string => {
  if (isBuiltin(id)) return tx(CARD_INFO[id].name);
  const w = widgetInfo(id);
  return w ? `${w.icon ? `${w.icon} ` : ""}${w.title}` : (widgetFile(id) ?? id);
};
export const cardDesc = (id: CardId): string => {
  if (isBuiltin(id)) return tx(CARD_INFO[id].desc);
  return widgetInfo(id)?.description ?? t("cards.layout.desc.widget", "Widget ze skriptu ve složce widgetů");
};
export const partName = (k: PartKey): string => (PART_INFO[k] ? tx(PART_INFO[k]) : k);
/** Název záložky (bez ikony); vestavěná bez názvu podle jazyka. */
export const pageName = (p: LayoutPage): string => {
  const b = builtinName(p);
  return b ? tx(b) : p.name.trim() || p.id;
};
