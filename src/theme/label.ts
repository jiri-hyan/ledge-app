// Překlad páru [klíč, čeština] z tabulek vzhledu (`looks.ts`) až při vykreslení — tabulky samy t()
// nevolají (konstanta při importu by zůstala česky). Jediné místo s dynamickým klíčem pro styly.
import { t } from "../i18n";
import type { Pair } from "./looks";

export const lbl = (p: Pair) => t(p[0], p[1]);
