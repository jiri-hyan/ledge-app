// Karta „Claude aktivita“ (kolo 23, id `activity`): čistá logika bez DOM — které session ukázat, v jakém
// pořadí, jméno fáze (bez kroku: fáze je odhad). Data jsou jen z Rustu (`claude/activity.rs` → `SessionView.activity`):
// fáze je ODHAD podle posledního nástroje, soubor z `tool_input`, zpráva z hooku Stop / konce přepisu.
// Bez importů za běhu kromě typů (`activity.test.mjs`).
import type { ActivityPhase, SessionActivity } from "../types/claude.ts";

/** Co karta potřebuje ze session (podmnožina `SessionView`). */
export interface ActRow {
  id: string;
  status: "waiting" | "working" | "idle" | "ended";
  updatedAt: number;
  activity?: SessionActivity | null;
}

export const PHASES: readonly ActivityPhase[] = ["reading", "editing", "testing", "working", "done", "error"];

/** „C:\r\src\main.rs“ → „main.rs“ (Rust `activity::base_name`). */
export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** Ukázat session v kartě? Skončené ne; ostatní jen s aktivitou z hooků nebo čekající na tebe. */
export function shown(r: ActRow, waitingIds: ReadonlySet<string>): boolean {
  return r.status !== "ended" && (!!r.activity || waitingIds.has(r.id));
}

/** Pořadí: čeká na tebe > pracuje > zbytek; uvnitř podle poslední změny fáze (jinak `updatedAt`). */
export function orderActivity<T extends ActRow>(list: readonly T[], waitingIds: ReadonlySet<string>): T[] {
  const rank = (r: T) => (waitingIds.has(r.id) ? 0 : r.status === "working" ? 1 : 2);
  const at = (r: T) => r.activity?.at ?? r.updatedAt;
  return list.filter((r) => shown(r, waitingIds)).sort((a, b) => rank(a) - rank(b) || at(b) - at(a));
}

/** Fáze k zobrazení: rozpracovaná fáze u session, která už nepracuje, je zastaralá → nic. */
export function livePhase(r: ActRow): ActivityPhase | null {
  const p = r.activity?.phase;
  if (!p) return null;
  if (p === "done" || r.status === "working" || r.status === "waiting") return p;
  return null;
}

/** Hooky, bez kterých aktivita nepřijde (`PostToolUse`, `Stop`), nejsou v `~/.claude/settings.json` Ledge → karta to řekne
 *  místo „Claude teď nic nedělá“. Neznámý stav (chyba čtení) netvrdí nic. */
export function hooksMissing(h: { state: string; missing: readonly string[] } | null | undefined): boolean {
  if (!h) return false;
  return h.state === "none" || (h.state === "partial" && h.missing.some((m) => m === "PostToolUse" || m === "Stop"));
}

/** Karta `activity` schvaluje sama jen bez karty Claude (jinak je schválení v ní). */
export const activityApproves = <T extends string>(has: (id: T) => boolean): boolean => !has("claude" as T);

/** `ClaudeInbox` (osamělé schválení) se ukáže, jen když na záložce není ani karta Claude, ani `activity`. */
export const inboxShown = <T extends string>(has: (id: T) => boolean): boolean => !has("claude" as T) && !has("activity" as T);
