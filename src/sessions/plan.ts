// Čistá logika karty plánu (`PlanReview.tsx`, ExitPlanMode): co pošlou tři akce do `decide(id, allow, extra?)`.
// Rozhodnutí a tělo odpovědi dělá Rust (`claude/plan.rs`). Bez Solidu a IPC.
import type { DecideExtra, Pending } from "../types";
import { denyExtra } from "./permission.ts";

export type PlanAction = "approve" | "approveEdits" | "return";

/** Akce → argumenty `decide`; „Vrátit“ bez připomínky = `null` (tlačítko neodešle nic). */
export function planDecide(action: PlanAction, text = ""): { allow: boolean; extra?: DecideExtra } | null {
  if (action === "approve") return { allow: true };
  if (action === "approveEdits") return { allow: true, extra: { mode: "acceptEdits" } };
  const extra = denyExtra(text);
  return extra ? { allow: false, extra } : null;
}

/** Titulek plánu do peeku a řádku karty (Rust ho už zkrátil); prázdný → shrnutí žádosti. */
export const planTitle = (pd: Pick<Pending, "plan" | "summary">): string => pd.plan?.title || pd.summary;
