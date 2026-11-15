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

/** Stáří údaje v celých minutách od posledního úspěšného dotazu (nikdy záporné). */
export const ageMin = (i: Incident, now: number): number => Math.max(0, Math.floor((now - i.okMs) / 60000));

/** Zobrazit řádek? Skutečný incident s neprázdným názvem a údaj ne starší než `ttlMin` (poctivost: po výpadku sítě
 *  nevydávat starý incident za aktuální; jádro skrývá totéž při přestavbě snapshotu, UI i mezi nimi). */
export const shown = (i: Incident | undefined | null, now: number): i is Incident => !!i && i.name.trim().length > 0 && ageMin(i, now) <= i.ttlMin;

/** Poznámka o stáří: ukáže se, až když je údaj starší než jeden dotazovací odstup (`ttlMin / 2`); jinak `null`. */
export const staleMin = (i: Incident, now: number): number | null => {
  const a = ageMin(i, now);
  return a > i.ttlMin / 2 ? a : null;
};
