// Čistá logika doplňků schválení Clauda (`PermExtra.tsx`): cíle „Vždy povolit“, cílový soubor, důvod
// zamítnutí a co UI pošle do `decide(id, allow, extra?)`. Pravidlo samo vybírá Rust (`claude/rules.rs`);
// UI posílá jen cíl. Bez Solidu a IPC.
import type { DecideExtra, PermDest } from "../types";

/** Pořadí v rozbalení; první = výchozí (projekt lokálně). */
export const DESTS: PermDest[] = ["localSettings", "userSettings", "session"];
export const DEFAULT_DEST: PermDest = DESTS[0];

/** Strop důvodu (znaky) — stejný jako `rules::REASON_MAX` v Rustu (ten ořízne i tak). */
export const REASON_MAX = 500;

/** Soubor, kam Claude Code pravidlo zapíše (dokumentace hooků); `session` = nikam (jen paměť session). */
export function destFile(d: PermDest): string | null {
  if (d === "localSettings") return ".claude/settings.local.json";
  if (d === "userSettings") return "~/.claude/settings.json";
  return null;
}

/** Důvod k odeslání: ořízlý, nejvýš `REASON_MAX` znaků; prázdný = `null` (obyčejné zamítnutí). */
export function reasonOf(text: string): string | null {
  const r = [...text.trim()].slice(0, REASON_MAX).join("").trimEnd();
  return r ? r : null;
}

/** Doplněk pro „Zamítnout s důvodem“; bez důvodu `undefined` = chování jako dřív. */
export function denyExtra(text: string): DecideExtra | undefined {
  const reason = reasonOf(text);
  return reason ? { reason } : undefined;
}

/** Doplněk pro „Vždy povolit“. */
export const alwaysExtra = (d: PermDest): DecideExtra => ({ always: d });

/** Klávesy v poli důvodu: Enter odešle (jen s důvodem), Shift+Enter nový řádek, Esc zavře. */
export function reasonKey(key: string, shift: boolean, text: string): "send" | "cancel" | null {
  if (key === "Escape") return "cancel";
  if (key === "Enter" && !shift && reasonOf(text)) return "send";
  return null;
}
