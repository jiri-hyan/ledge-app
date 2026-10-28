// Čistá logika přidávání projektů v Nastavení (bez DOM; test `projectAdd.test.mjs`): duplicita podle složky
// i git remote, rozpracovaná id pro návrh, kontrola odkazů projektu (zrcadlo `config/validate.rs`)
// a rozhodnutí, co je vložená cesta/adresa (projekt, nebo akce podle `guessKind`).
import { guessKind } from "../actions/model.ts";
import type { ActionKind } from "../types";

const low = (v: unknown) => (typeof v === "string" ? v.trim().toLowerCase() : "");
const pathKey = (v: unknown) => low(v).replace(/[\\]/g, "/").replace(/\/+$/, "");

/** Remote ve srovnatelném tvaru: `https://host/o/r.git`, `git@host:o/r` i `o/r` → `o/r`. */
export function remoteKey(v: unknown): string {
  let s = low(v).replace(/\/+$/, "").replace(/\.git$/, "");
  s = s.replace(/^[a-z+]+:\/\/(?:[^@/]+@)?[^/]+\//, "").replace(/^[^@/]+@[^:/]+:/, "");
  return s;
}

/** Index projektu, který je stejný (složka nebo remote), nebo -1. Prázdné hodnoty se neshodují. */
export function findDuplicate(list: readonly any[], cand: { path?: string | null; remote?: string | null }): number {
  const p = pathKey(cand.path);
  const r = remoteKey(cand.remote);
  return list.findIndex((x) => (p && pathKey(x?.path) === p) || (r && remoteKey(x?.remote) === r));
}

/** Id projektů v rozpracovaném návrhu (jádro k nim nevybere stejné id). */
export const draftIds = (list: readonly any[]): string[] => list.map((x) => x?.id).filter((i): i is string => typeof i === "string" && !!i);

/** Co vyhodit z nabídky rep: složky a remoty projektů v návrhu. */
export function draftKnown(list: readonly any[]): { paths: string[]; remotes: string[] } {
  return {
    paths: list.map((x) => x?.path).filter((v): v is string => typeof v === "string" && !!v),
    remotes: list.map((x) => x?.remote).filter((v): v is string => typeof v === "string" && !!v),
  };
}

export type LinkProblem = { field: "label" | "url"; key: string; cs: string };

/** Totéž co jádro u odkazu projektu: popisek nesmí být prázdný, adresa jen http/https. */
export function linkProblem(l: { label?: string; url?: string }): LinkProblem | null {
  if (!(l.label ?? "").trim()) return { field: "label", key: "settings.proj.link.errLabel", cs: "Doplň popisek odkazu." };
  if (!/^https?:\/\/\S+$/i.test((l.url ?? "").trim())) return { field: "url", key: "settings.proj.link.errUrl", cs: "Adresa musí začínat http:// nebo https://." };
  return null;
}

/** Mění editor id projektu, který už je uložený? (osiří poznámky, vzhled dlaždice a logo) */
export function idChanged(saved: readonly any[], startId: string, nowId: string): boolean {
  return startId !== nowId && saved.some((x) => x?.id === startId);
}

/** Vložený text → co z něj udělat: složka = projekt, ostatní = akce daného typu. Prázdné = nic. */
export function addTarget(text: string): { as: "project" | "action"; kind: ActionKind } | null {
  const s = text.trim();
  if (!s) return null;
  const kind = guessKind(s);
  return { as: kind === "folder" ? "project" : "action", kind };
}
