// Formátování času a čísel pro karty podle jazyka UI (čisté funkce nad `t` / `locale()`, bez stavu).
// Čas je v obou jazycích 24h („07:05“) — velké hodiny přidávají sekundy hned za minuty.
import { locale, t } from "./i18n.ts";

/** Názvy dnů od neděle (`Date.getDay()`). */
export const dayNames = () => [
  t("format.day.sun", "neděle"),
  t("format.day.mon", "pondělí"),
  t("format.day.tue", "úterý"),
  t("format.day.wed", "středa"),
  t("format.day.thu", "čtvrtek"),
  t("format.day.fri", "pátek"),
  t("format.day.sat", "sobota"),
];
/** Měsíce ve 2. pádu („16. října“). */
const monthsGen = () => [
  t("format.month.jan", "ledna"),
  t("format.month.feb", "února"),
  t("format.month.mar", "března"),
  t("format.month.apr", "dubna"),
  t("format.month.may", "května"),
  t("format.month.jun", "června"),
  t("format.month.jul", "července"),
  t("format.month.aug", "srpna"),
  t("format.month.sep", "září"),
  t("format.month.oct", "října"),
  t("format.month.nov", "listopadu"),
  t("format.month.dec", "prosince"),
];

export const pad2 = (n: number) => String(n).padStart(2, "0");
/** „7:05“ → „07:05“ (24 h). */
export const fmtTime = (d: Date) => `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
/** „pátek 16. října“ / „Friday, October 16“ */
export const fmtDate = (d: Date) => t("format.date", "{dow} {d}. {month}", { dow: dayNames()[d.getDay()], d: d.getDate(), month: monthsGen()[d.getMonth()] });
export const num = (v: number, digits = 0) => v.toLocaleString(locale(), { maximumFractionDigits: digits });

/** ISO týden (pondělí = začátek, týden 1 obsahuje první čtvrtek roku). */
export function isoWeek(d: Date): number {
  const u = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  u.setUTCDate(u.getUTCDate() + 4 - (u.getUTCDay() || 7));
  return Math.ceil(((u.getTime() - Date.UTC(u.getUTCFullYear(), 0, 1)) / 86400000 + 1) / 7);
}

export function ago(ms: number, now = Date.now()): string {
  const s = Math.max(0, Math.round((now - ms) / 1000));
  if (s < 45) return t("format.now", "teď");
  const m = Math.round(s / 60);
  if (m < 60) return t("format.agoMin", "před {n} min", { n: m });
  const h = Math.round(m / 60);
  return t("format.agoH", "před {n} h", { n: h });
}

export function uptime(ms: number, now = Date.now()): string {
  const s = Math.max(0, Math.round((now - ms) / 1000));
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}
