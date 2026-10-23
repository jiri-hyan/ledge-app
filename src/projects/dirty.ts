// Upozornění „N neuloženo · M nepushnuto“ v hlavičce šuplíku (zadání týden 43, bod 1): které projekty se
// počítají (ztišené `muteDirty` ne), podpis stavu pro „Přečteno“ (zvýraznění zhasne, dokud se počty
// nezmění) a seznam rep pro rozbalení. Čistá logika bez DOM — tlačítko a menu jsou `cards/DirtyPill.tsx`.
import type { ProjectView } from "../types.ts";

export interface DirtySummary {
  /** všechna repa s neuloženou nebo nepushnutou prací (i ztišená — v menu jdou znovu zapnout) */
  repos: ProjectView[];
  /** počet neztišených rep s neuloženými soubory / nepushnutými commity */
  dirty: number;
  unpushed: number;
  /** podpis neztišeného stavu („id:dirty:ahead|…“); "" = nic k upozornění */
  sig: string;
}

const n = (v: number | undefined | null) => Math.max(0, v ?? 0);

export function dirtySummary(projects: readonly ProjectView[]): DirtySummary {
  const repos = projects.filter((x) => n(x.git?.dirty) > 0 || n(x.git?.ahead) > 0);
  const loud = repos.filter((x) => !x.muteDirty);
  return {
    repos,
    dirty: loud.filter((x) => n(x.git?.dirty) > 0).length,
    unpushed: loud.filter((x) => n(x.git?.ahead) > 0).length,
    sig: loud.map((x) => `${x.id}:${n(x.git?.dirty)}:${n(x.git?.ahead)}`).join("|"),
  };
}

/** Krátký otisk podpisu do configu (`gitAlerts.read`, FNV-1a 32 b, base36) — přežije restart, config nenafoukne. */
export function sigHash(sig: string): string {
  if (!sig) return "";
  let h = 0x811c9dc5;
  for (let i = 0; i < sig.length; i++) {
    h ^= sig.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(36);
}

/** „Přečteno“ platí, dokud se podpis nezmění (jiný počet souborů / commitů, nové repo); `read` = `sigHash`. */
export const isAcked = (sig: string, read: string | null | undefined): boolean => sig !== "" && !!read && sigHash(sig) === read;
