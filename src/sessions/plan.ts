// Čistá logika karty plánu (`PlanReview.tsx`, ExitPlanMode): co pošlou akce do `decide(id, allow, extra?)`.
// Rozhodnutí a tělo odpovědi dělá Rust (`claude/plan.rs`, `claude/planhook.rs`). Bez Solidu a IPC.
import type { DecideExtra, HooksStatus, Pending } from "../types";
import { denyExtra } from "./permission.ts";

export type PlanAction = "approve" | "approveEdits" | "inClaude" | "return";

/** Akce → argumenty `decide`; „Vrátit“ bez připomínky = `null` (tlačítko neodešle nic). */
export function planDecide(action: PlanAction, text = ""): { allow: boolean; extra?: DecideExtra } | null {
  if (action === "approve") return { allow: true };
  if (action === "approveEdits") return { allow: true, extra: { mode: "acceptEdits" } };
  if (action === "inClaude") return { allow: true, extra: { inClaude: true } };
  const extra = denyExtra(text);
  return extra ? { allow: false, extra } : null;
}

/** Vedlejší akce karty plánu: přes PermissionRequest jde nastavit režim (acceptEdits), přes PreToolUse ne
 *  (docs hooků) → tam „Vybrat režim v Claude“ (Ledge nerozhodne, Claude ukáže svůj dialog). */
export const planSecondary = (pd: Pick<Pending, "via">): PlanAction => (pd.via === "preToolUse" ? "inClaude" : "approveEdits");

/** Titulek plánu do peeku a řádku karty (Rust ho už zkrátil); prázdný → shrnutí žádosti. */
export const planTitle = (pd: Pick<Pending, "plan" | "summary">): string => pd.plan?.title || pd.summary;

/** Jména doplňkových hooků PreToolUse v `HooksStatus.missing` (`hooksetup.rs` `QUESTIONS_EVENT`, `PLAN_EVENT`). */
export const ADDON_HOOKS = ["PreToolUse", "PreToolUse(ExitPlanMode)"];

/** Chybí jen doplňky z novější verze (otázky / review plánu) → karta nabídne doplnění jedním klikem. */
export const addonsMissing = (h: Pick<HooksStatus, "state" | "missing">): boolean =>
  h.state === "partial" && h.missing.length > 0 && h.missing.every((m) => ADDON_HOOKS.includes(m));
