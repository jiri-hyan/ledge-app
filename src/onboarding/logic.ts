// Průvodce — čisté pomocné funkce (výběr, návrhy, formát): žádný stav, žádné IPC.
import { t } from "../i18n";
import type { FoundApp, FoundRepo } from "../types";

export type StepId = "welcome" | "projects" | "apps" | "claude" | "links" | "look" | "done";
export const STEPS: { id: StepId; key: string; cs: string }[] = [
  { id: "welcome", key: "onboarding.step.welcome", cs: "Vítej" },
  { id: "projects", key: "onboarding.step.projects", cs: "Projekty" },
  { id: "apps", key: "onboarding.step.apps", cs: "Aplikace" },
  { id: "claude", key: "onboarding.step.claude", cs: "Claude Code" },
  { id: "links", key: "onboarding.step.links", cs: "Odkazy" },
  { id: "look", key: "onboarding.step.look", cs: "Vzhled" },
  { id: "done", key: "onboarding.step.done", cs: "Hotovo" },
];

/** Repo jde přidat: má návrh projektu a v Ledge ještě není. */
export const importable = (r: FoundRepo) => !!r.project && !r.already;

/** Výchozí výběr: `n` naposledy měněných rep, která jde přidat (seznam už je seřazený od nejnovějšího). */
export function defaultRepoSelection(repos: FoundRepo[], n = 6): string[] {
  return repos.filter(importable).slice(0, n).map((r) => r.path);
}

/** Projekt `kind: app` ze zástupce (id dopočítá Rust z názvu). */
export function appProject(a: FoundApp): Record<string, unknown> {
  return { id: "", name: a.name, enabled: true, kind: "app", exe: a.exe, appName: a.name, args: a.args, passPort: false, openBrowser: false, links: [] };
}

/** „před 3 dny“, „před 2 měsíci“ — poslední aktivita repa; `null` = nevím. */
export function since(ms: number | null, now = Date.now()): string {
  if (ms == null) return t("onboarding.since.unknown", "neznámo kdy");
  const d = Math.floor(Math.max(0, now - ms) / 86400000);
  if (d < 1) return t("onboarding.since.today", "dnes");
  if (d < 2) return t("onboarding.since.yesterday", "včera");
  if (d < 14) return t("onboarding.since.days", "před {n} dny", { n: d });
  if (d < 60) return t("onboarding.since.weeks", "před {n} týdny", { n: Math.round(d / 7) });
  if (d < 730) return t("onboarding.since.months", "před {n} měsíci", { n: Math.round(d / 30) });
  return t("onboarding.since.years", "před {n} lety", { n: Math.round(d / 365) });
}

/** Monogram dlaždice bez loga: první písmena slov (max 2). */
export function monogram(name: string): string {
  const w = name.replace(/[^\p{L}\p{N} ]+/gu, " ").trim().split(/\s+/).filter(Boolean);
  return (w.length > 1 ? w[0][0] + w[1][0] : (w[0] ?? "?").slice(0, 2)).toUpperCase();
}

export type LinkSuggestion = { id: string; label: string; url: string; color: string };
/** Návrhy odkazů (veřejné adresy služeb, nic z autorova světa). Funkce, ne konstanta: popisky jsou
 *  v jazyce UI v okamžiku volby (uloží se do configu jako popisek akce). */
export const linkSuggestions = (): LinkSuggestion[] => [
  { id: "gmail", label: "Gmail", url: "https://mail.google.com/", color: "#ea4335" },
  { id: "outlook", label: "Outlook", url: "https://outlook.office.com/mail/", color: "#0a64d6" },
  { id: "gcal", label: t("onboarding.link.gcal", "Google Kalendář"), url: "https://calendar.google.com/", color: "#1a73e8" },
  { id: "ocal", label: t("onboarding.link.ocal", "Outlook kalendář"), url: "https://outlook.office.com/calendar/", color: "#0f6cbd" },
  { id: "github", label: "GitHub", url: "https://github.com/", color: "#8b949e" },
  { id: "drive", label: t("onboarding.link.drive", "Google Disk"), url: "https://drive.google.com/", color: "#34a853" },
  { id: "claude", label: "Claude", url: "https://claude.ai/", color: "#d97757" },
];
/** Adresa pro vlastní odkaz: jen http(s), bez mezer; holá doména dostane `https://`. `null` = neplatná. */
export function normalizeUrl(raw: string): string | null {
  const s = raw.trim();
  if (!s || /\s/.test(s)) return null;
  const withScheme = /^[a-z][a-z0-9+.-]*:/i.test(s) ? s : `https://${s}`;
  try {
    const u = new URL(withScheme);
    return (u.protocol === "https:" || u.protocol === "http:") && u.hostname.includes(".") ? u.href : null;
  } catch {
    return null;
  }
}

/** Uživatel si přeje omezený pohyb → přechody kroků bez animace. */
export const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
