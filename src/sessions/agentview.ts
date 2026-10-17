// Čistá logika Agent View v kartě Claude (`AgentActions.tsx`, `BgLaunch.tsx`): popisky stavu úlohy a
// důvodu čekání, pořadí a potvrzení akcí, kontrola úkolu pro „Claude na pozadí…“. Které akce jsou
// povolené, rozhoduje Rust (`agentview::actions` → `job.actions`) a při kliku to ověří znovu. Bez Solidu a IPC.
import { t } from "../i18n.ts";
import type { JobAction, JobView, SessionView, WaitFor } from "../types.ts";

/** Strop úkolu — stejný jako `agentview::PROMPT_MAX` v Rustu (ten ho ověří i tak). */
export const PROMPT_MAX = 4000;
/** Strop jména — `agentview::NAME_MAX`. */
export const NAME_MAX = 80;

/** Na co session čeká (Agent View `waitingFor`), přeloženě. */
export function waitText(w: WaitFor): string {
  switch (w) {
    case "permission": return t("sessions.av.wait.permission", "čeká: oprávnění");
    case "input": return t("sessions.av.wait.input", "čeká: vstup");
    case "sandbox": return t("sessions.av.wait.sandbox", "čeká: sandbox");
    case "worker": return t("sessions.av.wait.worker", "čeká: worker");
    case "dialog": return t("sessions.av.wait.dialog", "čeká: dialog");
  }
}

/** Pilulka stavu místo obecné (`STATUS` v kartě), když Agent View ví víc: [třída barvy, text] nebo null.
 *  Třídy jen a/n/r (stejné jako `STATUS`, hlídá `cssClasses.test.mjs`). */
export function statusPill(ss: Pick<SessionView, "status" | "waitingFor" | "job">): ["a" | "n" | "r", string] | null {
  if (ss.status === "waiting" && ss.waitingFor) return ["a", waitText(ss.waitingFor)];
  if (ss.status !== "ended") return null;
  switch (ss.job?.state) {
    case "failed": return ["r", t("sessions.av.state.failed", "selhala")];
    case "stopped": return ["n", t("sessions.av.state.stopped", "zastavena")];
    case "done": return ["n", t("sessions.av.state.done", "hotovo")];
    default: return null;
  }
}

/** Pořadí tlačítek (jen ty, které Rust nabídl). */
const ORDER: JobAction[] = ["attach", "logs", "respawn", "stop", "remove"];
export const shownActions = (job: JobView | null | undefined): JobAction[] => (job ? ORDER.filter((a) => job.actions.includes(a)) : []);

/** Mění stav session → inline potvrzení (Rust bez `confirmed` nic nespustí). */
export const needsConfirm = (a: JobAction) => a === "stop" || a === "remove" || a === "respawn";

export function actionLabel(a: JobAction): string {
  switch (a) {
    case "attach": return t("sessions.av.attach", "Připojit");
    case "logs": return t("sessions.av.logs", "Logy");
    case "stop": return t("sessions.av.stop", "Zastavit");
    case "remove": return t("sessions.av.remove", "Odebrat ze seznamu");
    case "respawn": return t("sessions.av.respawn", "Restartovat");
  }
}

export function actionHint(a: JobAction, id: string): string {
  switch (a) {
    case "attach": return t("sessions.av.attachHint", "Otevře session v novém terminálu (claude attach {id})", { id });
    case "logs": return t("sessions.av.logsHint", "Poslední výstup session (claude logs {id})", { id });
    case "stop": return t("sessions.av.stopHint", "Zastaví session; konverzace zůstane (claude stop {id})", { id });
    case "remove": return t("sessions.av.removeHint", "Smaže session ze seznamu i její worktree; přepis zůstane pro claude --resume (claude rm {id})", { id });
    case "respawn": return t("sessions.av.respawnHint", "Restartuje session s aktuální verzí Claude (claude respawn {id})", { id });
  }
}

/** Otázka potvrzení (jen u `needsConfirm`). */
export function confirmText(a: JobAction, name: string): string {
  switch (a) {
    case "stop": return t("sessions.av.confirmStop", "Zastavit „{name}“?", { name });
    case "remove": return t("sessions.av.confirmRemove", "Odebrat „{name}“ ze seznamu? Smaže i worktree session.", { name });
    case "respawn": return t("sessions.av.confirmRespawn", "Restartovat „{name}“?", { name });
    default: return "";
  }
}

/** Jde úkol odeslat? Stejná pravidla jako `agentview::clean_prompt` (Rust je ověří znovu). */
export function promptProblem(prompt: string): string | null {
  const s = prompt.trim();
  if (!s) return t("sessions.av.bg.empty", "Napiš, co má Claude udělat.");
  if (s.startsWith("-")) return t("sessions.av.bg.dash", "Úkol nesmí začínat pomlčkou.");
  if ([...s].length > PROMPT_MAX) return t("sessions.av.bg.long", "Úkol je moc dlouhý (nejvýš {n} znaků).", { n: PROMPT_MAX });
  return null;
}

export function nameProblem(name: string): string | null {
  const s = name.trim();
  if (s.startsWith("-")) return t("sessions.av.bg.nameDash", "Jméno nesmí začínat pomlčkou.");
  if ([...s].length > NAME_MAX) return t("sessions.av.bg.nameLong", "Jméno je moc dlouhé (nejvýš {n} znaků).", { n: NAME_MAX });
  return null;
}

/** Klávesy pole úkolu: Ctrl+Enter odešle, Esc zruší, jinak nic. */
export function bgKey(key: string, ctrl: boolean): "send" | "cancel" | null {
  if (key === "Escape") return "cancel";
  if (key === "Enter" && ctrl) return "send";
  return null;
}
