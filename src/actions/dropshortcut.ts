// Přetažení na otevřený šuplík = nový zástupce (kolo 20, `smart.dropShortcut`). ČISTÁ logika: z návrhu, který
// pošle jádro (`shelf/shortcut.rs` `Payload`, událost `drop-shortcut`), vznikne rozepsaná akce pro editor
// (`ActionEditor`), a pozná se, jestli ji na záložce někde uvidíš. Nic se nespouští ani neukládá samo — člověk
// akci v editoru potvrdí („Přidat“). Příkaz (`command`) z puštěného nevznikne nikdy.
import type { ActionCfg, ActionKind } from "../types";
import { labelFrom, slugId } from "./model.ts";

export type DropKind = "url" | "folder" | "file" | "app";

/** Co pošle jádro (zrcadlo `shelf::shortcut::Payload`). */
export interface DropPayload {
  kind: DropKind;
  target: string;
  /** kolik dalších puštěných položek se přeskočilo */
  skipped: number;
}

const KINDS: readonly DropKind[] = ["url", "folder", "file", "app"];

/** Návrh z jádra → bezpečný tvar, nebo `null` (neznámý typ, prázdný cíl, ne objekt). */
export function readPayload(x: unknown): DropPayload | null {
  const o = x as Partial<DropPayload> | null;
  if (!o || typeof o !== "object") return null;
  if (!KINDS.includes(o.kind as DropKind) || typeof o.target !== "string" || !o.target.trim()) return null;
  const skipped = Number.isInteger(o.skipped) && (o.skipped as number) > 0 ? (o.skipped as number) : 0;
  return { kind: o.kind as DropKind, target: o.target.trim(), skipped };
}

/** Rozepsaná akce pro editor: popisek z cíle, jednoznačné id, aplikace se před každým spuštěním ptá. */
export function draftFrom(p: DropPayload, takenIds: string[]): ActionCfg {
  const label = labelFrom(p.target) || p.target.slice(0, 40);
  const a: ActionCfg = { id: slugId(label, takenIds), label, kind: p.kind as ActionKind, target: p.target, showOn: "both" };
  // .exe z puštěného: výchozí je ptát se před spuštěním (člověk to v editoru může vypnout)
  if (p.kind === "app") a.confirm = true;
  return a;
}

/** Karty, ve kterých se akce typu `kind` ukáže: odkazy (web, složka, soubor) v Odkazech i Rychlých akcích,
 *  aplikace jen v Rychlých akcích. */
export function hostCards(kind: ActionKind): readonly string[] {
  return kind === "app" ? ["actions"] : ["links", "actions"];
}

/** Uvidí se nová akce na záložce s kartami `pageCards`? Když ne, kterou kartu nabídnout přidat. */
export function visibility(pageCards: readonly string[], kind: ActionKind): { visible: true } | { visible: false; add: "links" | "actions" } {
  const hosts = hostCards(kind);
  if (pageCards.some((c) => hosts.includes(c))) return { visible: true };
  return { visible: false, add: kind === "app" ? "actions" : "links" };
}
