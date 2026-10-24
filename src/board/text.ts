// Krátké názvy dnů a měsíců pro karty kola 22 (pás týdne, velké hodiny) a štítek původu čísla.
import { t } from "../i18n";
import type { Origin } from "./model";

/** Zkratky dnů od neděle (`Date.getDay()`). */
export const dowShort = () => [
  t("cards.board.dow.sun", "Ne"),
  t("cards.board.dow.mon", "Po"),
  t("cards.board.dow.tue", "Út"),
  t("cards.board.dow.wed", "St"),
  t("cards.board.dow.thu", "Čt"),
  t("cards.board.dow.fri", "Pá"),
  t("cards.board.dow.sat", "So"),
];

/** Zkratky měsíců (1. pád). */
export const monthShort = () => [
  t("cards.board.mon.jan", "led"),
  t("cards.board.mon.feb", "úno"),
  t("cards.board.mon.mar", "bře"),
  t("cards.board.mon.apr", "dub"),
  t("cards.board.mon.may", "kvě"),
  t("cards.board.mon.jun", "čvn"),
  t("cards.board.mon.jul", "čvc"),
  t("cards.board.mon.aug", "srp"),
  t("cards.board.mon.sep", "zář"),
  t("cards.board.mon.oct", "říj"),
  t("cards.board.mon.nov", "lis"),
  t("cards.board.mon.dec", "pro"),
];

/** Štítek původu čísla: krátký text a vysvětlení do bubliny. */
export function originText(o: Origin): { text: string; tip: string } {
  return o === "official"
    ? { text: t("cards.board.origin.official", "oficiální"), tip: t("cards.board.origin.officialTip", "Číslo hlásí přímo zdroj (Windows / Claude Code), Ledge ho jen ukazuje.") }
    : { text: t("cards.board.origin.derived", "odhad"), tip: t("cards.board.origin.derivedTip", "Odvozené číslo: Ledge ho spočítal z lokálních transcriptů Claude Code podle ceníku API — není to účet.") };
}
