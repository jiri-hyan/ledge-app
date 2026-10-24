// Export útrat a limitů Claude (kolo 20): položky v ⋯ karty Claude „Exportovat jako CSV / Markdown…“. Jen data,
// která Ledge má z reálných zdrojů (přehled útrat z transcriptů, poslední vzorek limitů); co chybí, se
// neexportuje (`model.ts` `canExport`). Soubor vzniká v jádře (`dataexport.rs`) po dialogu „Uložit jako“; žádná síť.
import { api, type Snapshot } from "../api";
import type { CardMenuItem } from "../cards/CardMenu";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { canExport } from "./model";

export async function exportUsage(format: "csv" | "md") {
  try {
    const path = await api.exportUsage(format);
    if (path) say(t("sessions.export.saved", "Uloženo: {path}", { path }));
  } catch (e) {
    sayError(e);
  }
}

/** Volby navíc v ⋯ karty Claude. */
export function exportMenuItems(s: Snapshot): CardMenuItem[] {
  const disabled = !canExport(s);
  return [
    { label: t("sessions.export.csv", "Exportovat útratu a limity (CSV)…"), run: () => void exportUsage("csv"), disabled },
    { label: t("sessions.export.md", "Exportovat útratu a limity (Markdown)…"), run: () => void exportUsage("md"), disabled },
  ];
}
