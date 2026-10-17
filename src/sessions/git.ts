// Čistá logika panelu Git v kartě Claude (`GitPanel.tsx`): písmeno stavu souboru, výchozí výběr ke
// commitu, jaké diffy soubor má, klávesy pole zprávy. Stav i pravidla (blokace, co jde zahodit, texty
// potvrzení) počítá Rust (`sessiongit/`); tady nic, co by rozhodovalo místo něj. Bez Solidu a IPC.
import type { GitFileView, GitView } from "../types";

/** Jedno písmeno do řádku souboru: změna v pracovní složce má přednost před připravenou. */
export function badge(f: Pick<GitFileView, "kind" | "x" | "y">): string {
  if (f.kind === "untracked") return "?";
  if (f.kind === "conflict") return "U";
  return f.y !== "." ? f.y : f.x;
}

export const staged = (f: Pick<GitFileView, "x">): boolean => f.x !== "." && f.x !== "?";
export const unstaged = (f: Pick<GitFileView, "y">): boolean => f.y !== ".";

/** Výchozí výběr ke commitu: všechny změněné soubory kromě konfliktů (ty git nezacommituje). */
export function defaultPicked(files: GitFileView[]): string[] {
  return files.filter((f) => f.kind !== "conflict").map((f) => f.path);
}

/** Přepnout soubor ve výběru (pořadí zůstane podle seznamu souborů). */
export function togglePicked(picked: string[], path: string, files: GitFileView[]): string[] {
  const set = new Set(picked);
  if (set.has(path)) set.delete(path);
  else set.add(path);
  return files.map((f) => f.path).filter((p) => set.has(p));
}

export const commitReady = (message: string, picked: string[]): boolean => message.trim().length > 0 && picked.length > 0;

/** Diffy, které soubor má: pracovní složka proti indexu (`false`), index proti HEAD (`true`). První = výchozí. */
export function diffModes(f: Pick<GitFileView, "kind" | "x" | "y">): boolean[] {
  if (f.kind === "untracked") return [false];
  const m: boolean[] = [];
  if (unstaged(f)) m.push(false);
  if (staged(f)) m.push(true);
  return m;
}

/** Klávesa v poli zprávy: Ctrl+Enter = commit, Esc = zrušit (Enter samotný dělá nový řádek). */
export function messageKey(key: string, ctrl: boolean): "send" | "cancel" | null {
  if (key === "Escape") return "cancel";
  if (key === "Enter" && ctrl) return "send";
  return null;
}

/** Kolik souborů se do panelu nevešlo. */
export const hiddenFiles = (v: Pick<GitView, "files" | "total">): number => Math.max(0, v.total - v.files.length);

/** Jde pushnout (podle plánu z Rustu)? */
export const canPush = (v: Pick<GitView, "push">): boolean => v.push.kind !== "no";
