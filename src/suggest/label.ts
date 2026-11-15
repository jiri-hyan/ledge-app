// Text chipu (sdílí pruh v šuplíku `SuggestBar.tsx` a klidový ostrůvek `hover/Island.tsx`).
import { t } from "../i18n";
import type { Suggestion } from "./model";
import type { MorningPart } from "./morning.ts";
import { SINCE_ORDER, type SinceItem, type SinceKind } from "./since.ts";

/** „Dnes: 2 po termínu · 3 úkolů“ — jen nenulové části */
export function todayLabel(overdue: number, today: number): string {
  const parts = [
    overdue > 0 ? t("cards.suggest.todayOverdue", "{n} po termínu", { n: overdue }) : "",
    today > 0 ? t("cards.suggest.todayTasks", "{n} úkolů", { n: today }) : "",
  ].filter(Boolean);
  return t("cards.suggest.today", "Dnes: {parts}", { parts: parts.join(" · ") });
}

/** Důvod konce session (`SessionEnd.reason`) lidsky; neznámý / `other` / prázdný = null (nic se nehádá, „spadla“ se neříká). */
export function endReasonLabel(reason: string | null | undefined): string | null {
  switch (reason) {
    case "prompt_input_exit": return t("cards.suggest.end.exit", "ukončena uživatelem");
    case "logout": return t("cards.suggest.end.logout", "odhlášení");
    case "clear": return t("cards.suggest.end.clear", "vyčištěna (/clear)");
    case "resume": return t("cards.suggest.end.resume", "přepnuta na jinou session");
    case "bypass_permissions_disabled": return t("cards.suggest.end.bypass", "vypnuto obcházení oprávnění");
    default: return null;
  }
}

const SINCE_NAMES = 2;
const sinceHead = (k: SinceKind): string => {
  switch (k) {
    case "waiting": return t("cards.suggest.since.waiting", "čeká");
    case "failed": return t("cards.suggest.since.failed", "chyba");
    case "done": return t("cards.suggest.since.done", "doběhlo");
    case "ended": return t("cards.suggest.since.ended", "skončila");
  }
};
const sinceGroup = (items: readonly SinceItem[], k: SinceKind): string => {
  const names = items.filter((i) => i.kind === k).map((i) => i.name);
  if (!names.length) return "";
  const rest = names.length - SINCE_NAMES;
  return `${sinceHead(k)}: ${names.slice(0, SINCE_NAMES).join(", ")}${rest > 0 ? ` +${rest}` : ""}`;
};

/** „Doběhlo: A, B · čeká: C“ — jen neprázdné skupiny, první velkým písmenem (pořadí `SINCE_ORDER`: čeká · chyba · doběhlo · skončila). */
export function sinceLabel(items: readonly SinceItem[]): string {
  const s = SINCE_ORDER.map((k) => sinceGroup(items, k)).filter(Boolean).join(" · ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Tooltip: řádek na session (stav; u skončené důvod konce, jen když ho hook nesl). */
export function sinceTip(items: readonly SinceItem[]): string {
  return items.map((i) => {
    const why = i.kind === "ended" ? endReasonLabel(i.reason) : null;
    return `${i.name} — ${sinceHead(i.kind)}${why ? ` (${why})` : ""}`;
  }).join("\n");
}

/** „Ráno: Schůzky: 3 (první 09:30) · po termínu: 2 · PR k revizi: 1“ — části z `morning.ts`, jen neprázdné. */
export function morningLabel(parts: readonly MorningPart[]): string {
  const one = (p: MorningPart): string => {
    switch (p.k) {
      case "meetings": return t("cards.suggest.morning.meetings", "schůzky: {n} (první {time})", { n: p.n, time: new Date(p.first).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) });
      case "overdue": return t("cards.suggest.morning.overdue", "po termínu: {n}", { n: p.n });
      case "tasks": return t("cards.suggest.morning.tasks", "úkoly na dnes: {n}", { n: p.n });
      case "reminders": return t("cards.suggest.morning.reminders", "připomínky: {n}", { n: p.n });
      case "briefs": return t("cards.suggest.morning.briefs", "zadání čeká: {n}", { n: p.n });
      case "review": return t("cards.suggest.morning.review", "PR k revizi: {n}", { n: p.n });
      case "ci": return t("cards.suggest.morning.ci", "spadlé CI: {n}", { n: p.n });
      case "week": return t("cards.suggest.morning.week", "týdenní limit Claude: {n} %", { n: p.n });
    }
  };
  return t("cards.suggest.morning", "Ráno: {parts}", { parts: parts.map(one).join(" · ") });
}

export function chipLabel(c: Suggestion): string {
  switch (c.kind) {
    case "reminder": return t("cards.suggest.reminder", "Připomínka: {title}", { title: c.title || t("cards.suggest.imageOnly", "(jen obrázek)") });
    case "meeting": return t("cards.suggest.meeting", "Schůzka: {title}", { title: c.title || t("cards.calendar.untitled", "(bez názvu)") });
    case "meetingNote": return t("cards.suggest.meetingEnded", "Schůzka skončila: {title}", { title: c.title || t("cards.calendar.untitled", "(bez názvu)") });
    case "today": return todayLabel(c.overdue ?? 0, c.today ?? 0);
    case "ghCi": return t("cards.suggest.ghCi", "Spadlo CI na tvém PR {pr}", { pr: c.title });
    case "ghReview": return t("cards.suggest.ghReview", "Čeká na tvou revizi: {pr}", { pr: c.title });
    case "dev": return (c.errors ?? 0) > 0
      ? t("cards.suggest.devErr", "Dev server: {n}× chyba — {project}", { n: c.errors ?? 0, project: c.detail })
      : t("cards.suggest.dev", "Dev server běží: {project}", { project: c.detail });
    case "claudeSince": return sinceLabel(c.since ?? []);
    case "morning": return morningLabel(c.morning ?? []);
    case "briefWaiting": return c.brief === "review"
      ? t("cards.suggest.briefReview", "Zadání ke kontrole: {title}", { title: c.title })
      : t("cards.suggest.briefWaiting", "Zadání čeká na tvou odpověď: {title}", { title: c.title });
    case "clip": return t("cards.suggest.clip", "Zkopíroval jsi odkaz: {host}", { host: c.title });
  }
}
