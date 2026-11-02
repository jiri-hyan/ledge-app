// Vzhledy karty Claude „Při práci“ a „Hotovo“ (kolo 28): čistá logika bez DOM a bez `t()` (test `claudeLive.test.mjs`).
// Data jsou jen skutečná (`SessionView`, aktivita z hooků): fáze je ODHAD z posledního nástroje, ne krok plánu;
// co chybí (zpráva, soubor, čas), vrací null a karta to nekreslí.
import type { ActivityPhase, SessionActivity } from "../types/claude.ts";

export type LiveRow = {
  id: string;
  status: "waiting" | "working" | "idle" | "ended";
  since: number | null;
  updatedAt: number;
  uncommitted: string[];
  unpushed: number;
  activity?: SessionActivity | null;
};

/** Pás fází: jen to, co umíme z posledního nástroje odhadnout. */
export const STRIP: readonly ActivityPhase[] = ["reading", "editing", "testing", "done"];

/** Pracující session, nejčerstvější první. */
export function workingList<T extends LiveRow>(list: readonly T[]): T[] {
  return list.filter((r) => r.status === "working").sort((a, b) => b.updatedAt - a.updatedAt);
}

/** Vybraná session: ta s `selected`, jinak nejčerstvější; prázdný seznam = null. */
export function pickWorking<T extends LiveRow>(list: readonly T[], selected: string | null): T | null {
  const w = workingList(list);
  return w.find((r) => r.id === selected) ?? w[0] ?? null;
}

/** Uplynulý čas „m:ss“ (od hodiny „h:mm:ss“); bez `since` null, záporný rozdíl (hodiny se rozešly) = 0:00. */
export function elapsed(since: number | null | undefined, now: number): string | null {
  if (since == null || !Number.isFinite(since)) return null;
  const s = Math.max(0, Math.floor((now - since) / 1000));
  const sec = String(s % 60).padStart(2, "0");
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}:${sec}`;
  return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}:${sec}`;
}

/** Index zvýrazněné fáze v `STRIP`; „working“ / „error“ / bez aktivity = -1 (nic nezvýrazníme, nehádáme). U pracující session
 *  je zbylé „done“ z minulého tahu zastaralé → -1. */
export function stripIndex(r: Pick<LiveRow, "status" | "activity">): number {
  const p = r.activity?.phase;
  if (!p || (p === "done" && r.status === "working")) return -1;
  return STRIP.indexOf(p);
}

/** Cesta → název souboru a jeho složka (jen poslední díl), bez celé cesty na obrazovce. */
export function fileParts(path: string | null | undefined): { name: string; dir: string | null } | null {
  if (!path) return null;
  const parts = path.split(/[\\/]/).filter(Boolean);
  const name = parts.pop();
  return name ? { name, dir: parts.pop() ?? null } : null;
}

/** Jednořádková zpráva: bílé znaky slité, nad `max` znaků zkráceno na „…“; prázdné = null. */
export function clipMessage(msg: string | null | undefined, max = 90): string | null {
  const s = (msg ?? "").replace(/\s+/g, " ").trim();
  if (!s) return null;
  const a = Array.from(s);
  return a.length <= max ? s : `${a.slice(0, Math.max(1, max - 1)).join("").trimEnd()}…`;
}

/** Kdy session dokončila tah: u fáze „done“ čas z hooku, jinak od kdy je nečinná. */
export function doneAt(r: LiveRow): number {
  return r.activity?.phase === "done" ? r.activity.at : (r.since ?? r.updatedAt);
}

/** Session po dokončeném tahu (nepracuje ani nečeká na tebe), nejnovější nahoře. Skončená jen s nedokončenou prací
 *  (jinak ji Rust nedrží). */
export function doneList<T extends LiveRow>(list: readonly T[]): T[] {
  return list
    .filter((r) => r.status === "idle" || r.status === "ended" || (r.activity?.phase === "done" && r.status !== "working" && r.status !== "waiting"))
    .sort((a, b) => doneAt(b) - doneAt(a));
}

/** Štítek nedokončené práce; nic = null. */
export function openWork(r: Pick<LiveRow, "uncommitted" | "unpushed">): { files: number; unpushed: number } | null {
  const files = r.uncommitted.length;
  return files || r.unpushed ? { files, unpushed: r.unpushed } : null;
}
