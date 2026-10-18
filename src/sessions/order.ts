// Pořadí session v kartě Claude bez DOM (test `layout/cap.test.mjs`): kdo čeká na tebe (schválení,
// otázka Clauda nebo stav „čeká“) je vždy první a strop karty ho nikdy neschová; pak pracující, pak
// nečinné podle poslední aktivity (nejnovější nahoře), nakonec skončené. Souhrn schovaných pro „+3 nečinné“.
import type { SessionView } from "../types";

type S = Pick<SessionView, "id" | "status" | "updatedAt">;
const RANK: Record<SessionView["status"], number> = { waiting: 0, working: 1, idle: 2, ended: 3 };

/** Čeká na tebe: stav `waiting` nebo k ní patří schválení / otázka. */
export const needsYou = (x: S, asked: (id: string) => boolean): boolean => x.status === "waiting" || asked(x.id);

export function orderSessions<T extends S>(list: readonly T[], asked: (id: string) => boolean): T[] {
  const rank = (x: T) => (needsYou(x, asked) ? 0 : RANK[x.status]);
  return [...list].sort((a, b) => rank(a) - rank(b) || b.updatedAt - a.updatedAt);
}

/** Kolik schovaných session je v jakém stavu (čekající se neschovávají). */
export function hiddenSummary(hidden: readonly S[]): { working: number; idle: number; ended: number } {
  const out = { working: 0, idle: 0, ended: 0 };
  for (const x of hidden) if (x.status !== "waiting") out[x.status]++;
  return out;
}
