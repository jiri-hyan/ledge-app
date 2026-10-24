// Stav incidentů Anthropicu v kartě limitů (kolo 20, `smart.incidents`): ČISTÁ logika popisků. Data posílá jádro
// (`incidents.rs` → `Snapshot.incident`, jen když je volba zapnutá a nějaký incident právě probíhá); bez incidentu
// se nekreslí nic. Název incidentu je cizí text (anglicky) — vykresluje se jen jako text. Stav a dopad jsou kódy,
// jejich lidské názvy se překládají až při vykreslení (`statusText`).
import type { Incident } from "../types";
import { t } from "../i18n.ts";

/** Stránka stavu, kam odkaz vede (stejná, jejíž API se jádro ptá). */
export const STATUS_URL = "https://status.anthropic.com";

/** Lidský popisek stavu v jazyce UI (literální klíče kvůli hlídači překladů); neznámý kód = „probíhá“. */
export function statusText(code: string): string {
  switch (code) {
    case "investigating":
      return t("sessions.incident.st.investigating", "vyšetřuje se");
    case "identified":
      return t("sessions.incident.st.identified", "příčina nalezena");
    case "monitoring":
      return t("sessions.incident.st.monitoring", "sleduje se");
    default:
      return t("sessions.incident.st.other", "probíhá");
  }
}

/** Barevná úroveň řádku z dopadu: critical / major = červená, minor = jantarová, jinak jen informace. */
export function severity(impact: string): "bad" | "warn" | "info" {
  return impact === "critical" || impact === "major" ? "bad" : impact === "minor" ? "warn" : "info";
}

/** Zobrazit řádek? Jen pro skutečný incident s neprázdným názvem. */
export const shown = (i: Incident | undefined | null): i is Incident => !!i && i.name.trim().length > 0;
