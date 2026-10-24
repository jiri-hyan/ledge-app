// Lidské názvy z modelu rozložení (páry klíč + čeština v `model.ts`) — překlad až při vykreslení.
// Dynamická karta (widget ze skriptu) má název z hlavičky skriptu (`widgets/registry.ts`), jinak soubor.
import { t, tp } from "../i18n";
import { widgetInfo } from "../widgets/registry";
import { widgetFile } from "../widgets/model";
import { BODY, builtinName, CARD_INFO, isBuiltin, PART_INFO, type CardId, type CardSize, type LayoutPage, type PartKey, type Text } from "./model";
import type { SizeOutcome } from "./size";

export const tx = (x: Text, vars?: Record<string, string | number>): string => t(x[0], x[1], vars);
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
/** Název části karty v pořadí částí (`model.ts` `CARD_SLOTS`): skrytelné části mají svůj, `body` je hlavní obsah karty. */
export const slotName = (k: string): string => (k === BODY ? t("cards.layout.part.body", "Obsah karty") : partName(k as PartKey));
/** Název záložky (bez ikony); vestavěná bez názvu podle jazyka. */
export const pageName = (p: LayoutPage): string => {
  const b = builtinName(p);
  return b ? tx(b) : p.name.trim() || p.id;
};

/** Lidský název velikosti (Malá / Střední / Velká / Přes celou šířku). */
const SIZE_NAMES: Record<CardSize, Text> = {
  s: ["cards.layout.size.s", "Malá"],
  m: ["cards.layout.size.m", "Střední"],
  l: ["cards.layout.size.l", "Velká"],
  xl: ["cards.layout.size.xl", "Přes celou šířku"],
};
export const sizeName = (s: CardSize): string => (SIZE_NAMES[s] ? tx(SIZE_NAMES[s]) : s);

/** Nápověda, když volba velikosti nemá viditelný efekt (`size.ts` `sizeOutcome`) — nikdy tiché nic. */
export function sizeNoteText(o: SizeOutcome, want: CardSize, cols: number): string {
  switch (o.note) {
    case "cols":
      return tp("cards.layout.note.cols", cols, ["Vzhled má jen {n} sloupec, „{want}“ se sem nevejde — vybráno „{size}“.", "Vzhled má jen {n} sloupce, „{want}“ se sem nevejde — vybráno „{size}“.", "Vzhled má jen {n} sloupců, „{want}“ se sem nevejde — vybráno „{size}“."], { want: sizeName(want), size: sizeName(o.size) });
    case "alone":
      return t("cards.layout.note.alone", "Karta je v řadě sama, proto vyplní celou šířku (aby nezůstala díra). Užší bude, až vedle ní dáš jinou kartu.");
    case "fill":
      return t("cards.layout.note.fill", "Karta teď vyplňuje volné místo v řadě, takže vypadá stejně. Rozdíl uvidíš, až se řada změní.");
    default:
      return "";
  }
}
