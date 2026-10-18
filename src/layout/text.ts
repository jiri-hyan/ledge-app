// Lidské názvy z modelu rozložení (páry klíč + čeština v `model.ts`) — překlad až při vykreslení.
import { t } from "../i18n";
import { builtinName, CARD_INFO, PART_INFO, type CardId, type LayoutPage, type PartKey, type Text } from "./model";

export const tx = (x: Text): string => t(x[0], x[1]);
export const cardName = (id: CardId): string => (CARD_INFO[id] ? tx(CARD_INFO[id].name) : id);
export const cardDesc = (id: CardId): string => (CARD_INFO[id] ? tx(CARD_INFO[id].desc) : "");
export const partName = (k: PartKey): string => (PART_INFO[k] ? tx(PART_INFO[k]) : k);
/** Název záložky (bez ikony); vestavěná bez názvu podle jazyka. */
export const pageName = (p: LayoutPage): string => {
  const b = builtinName(p);
  return b ? tx(b) : p.name.trim() || p.id;
};
