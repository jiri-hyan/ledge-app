// Karty středního panelu z docku (L147, `drawer.dockCards`): zrcadlo Rust `hovermode::dock_cards` — jen vestavěné
// karty (`CARD_IDS`), bez opakování, nejvýš `DOCK_CARDS_MAX`, po ověření prázdné = výchozí. Čistá logika (test `midCards.test.mjs`).
import { CARD_IDS, mergedOf, type BuiltinCardId } from "../layout/model.ts";

export const DOCK_CARDS_MAX = 3;
/** Výchozí tři: poznámky (a úkoly v nich), kalendář, Claude. */
export const DOCK_CARDS_DEFAULT: readonly BuiltinCardId[] = ["notes", "calendar", "claude"];

/** Ověřený seznam karet středního panelu z libovolného vstupu (config, `ViewMsg.dockCards`). */
export function dockCardsOf(raw: unknown): BuiltinCardId[] {
  const out: BuiltinCardId[] = [];
  if (Array.isArray(raw)) {
    for (const raw1 of raw) {
      if (out.length === DOCK_CARDS_MAX) break;
      // sloučená karta (kolo 34: `week` → `calendar` …) se převede na nástupce; vzhled (`design`) dok nenese
      const id = mergedOf(raw1)?.to ?? raw1;
      if (typeof id === "string" && (CARD_IDS as readonly string[]).includes(id) && !out.includes(id as BuiltinCardId)) out.push(id as BuiltinCardId);
    }
  }
  return out.length ? out : [...DOCK_CARDS_DEFAULT];
}

/** Karta `id` po přepnutí v editoru (zapnout / vypnout) — držet max `DOCK_CARDS_MAX`, poslední vypnutá nenechá prázdno (vrátí se výchozí). */
export function toggleDockCard(cur: readonly string[], id: BuiltinCardId): BuiltinCardId[] {
  const now = dockCardsOf(cur);
  if (now.includes(id)) return now.length > 1 ? now.filter((x) => x !== id) : now;
  return now.length >= DOCK_CARDS_MAX ? now : [...now, id];
}
