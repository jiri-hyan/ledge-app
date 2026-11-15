// „Claude od posledně“ (V1 Pruh ví): co se v Claude sessions stalo od chvíle, kdy se šuplík naposledy schoval
// (`Snapshot.drawerHiddenAt`), a co čeká na člověka. Čistá logika bez DOM a `t()` (test `since.test.mjs`).
// Jen reálná data hooků / registru: fáze `done` / `error` z `Session.activity` (odhad z hooků, ne z přepisu),
// `status: waiting`, `endReason` z `SessionEnd.reason`. Bez `drawerHiddenAt` se „od posledně“ nic nehádá.

export type SinceKind = "waiting" | "done" | "failed" | "ended";

/** Co z `SessionView` stačí (struktura `SessionView` jí vyhoví). */
export interface SinceSession {
  id: string;
  name: string;
  status: "waiting" | "working" | "idle" | "ended";
  ended: boolean;
  endReason?: string | null;
  updatedAt: number;
  canOpen: boolean;
  activity?: { phase: string; at: number } | null;
}

export interface SinceItem {
  id: string;
  name: string;
  kind: SinceKind;
  /** jen u `ended`: `SessionEnd.reason` */
  reason: string | null;
  canOpen: boolean;
}

/** Pořadí skupin v chipu i při výběru sezení, které klik otevře: čeká na mě → chyba → doběhlo → skončila. */
export const SINCE_ORDER: readonly SinceKind[] = ["waiting", "failed", "done", "ended"];

/** Sessions, které čekají (vždy), a které doběhly / selhaly / skončily PO `hiddenAt` (jen když ho známe). Prázdné = null. */
export function claudeSince(sessions: readonly SinceSession[], hiddenAt: number | null | undefined): SinceItem[] | null {
  const out: SinceItem[] = [];
  for (const s of sessions) {
    const item = (kind: SinceKind): SinceItem => ({ id: s.id, name: s.name, kind, reason: kind === "ended" ? s.endReason ?? null : null, canOpen: s.canOpen });
    if (s.ended) {
      if (hiddenAt != null && s.updatedAt > hiddenAt) out.push(item("ended"));
    } else if (s.status === "waiting") {
      out.push(item("waiting"));
    } else if (hiddenAt != null && s.activity && s.activity.at > hiddenAt) {
      if (s.activity.phase === "done") out.push(item("done"));
      // L233: chyba nástroje (např. `grep` exit 1) není selhání session — „selhalo“ jen když session už nepracuje
      else if (s.activity.phase === "error" && s.status !== "working") out.push(item("failed"));
    }
  }
  out.sort((a, b) => SINCE_ORDER.indexOf(a.kind) - SINCE_ORDER.indexOf(b.kind));
  return out.length ? out : null;
}

/** Session, kterou otevře klik na chip: první čekající, jinak první s chybou / doběhlá, kterou jde otevřít; jinak null. */
export function sinceTarget(items: readonly SinceItem[]): SinceItem | null {
  return items.find((i) => i.canOpen && i.kind !== "ended") ?? null;
}

/** Podpis obsahu chipu (pro `sameSuggestions`): stejné sessions ve stejných stavech = stejný chip. */
export const sinceSig = (items: readonly SinceItem[] | undefined): string => (items ?? []).map((i) => `${i.id}:${i.kind}:${i.reason ?? ""}`).join("|");
