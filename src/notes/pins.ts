// Lístky na plochu ze šuplíku: připnout / odepnout poznámku (řádek karty, editor, paleta), nový lístek
// a průchozí režim u všech (⋯ karty Poznámky, paleta). Stav připnutí je ve snapshotu (`Snapshot.pins`,
// Rust `pins::win::view`); okno lístku samo je `src/pin/`.
import { api, type Snapshot } from "../api";
import { anyThrough, isPinned } from "../pin/model";
import type { CardMenuItem } from "../cards/CardMenu";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";

export { anyThrough, isPinned };

export async function togglePin(path: string) {
  try {
    const on = await api.pinToggle(path);
    if (on === undefined) return;
    say(on ? t("notes.pin.pinned", "Připnuto na plochu — lístek je vpravo nahoře, tahem za lištu ho posuneš.") : t("notes.pin.unpinned", "Odepnuto z plochy — poznámka zůstala."));
  } catch (e) {
    sayError(e);
  }
}

export async function newPin() {
  try {
    await api.pinNew();
  } catch (e) {
    sayError(e);
  }
}

/** Průchozí režim u všech lístků: je-li některý průchozí, vypne ho všem, jinak zapne. */
export async function throughAll() {
  try {
    const on = await api.pinThroughAll();
    if (on === undefined) return;
    if (on === null) return say(t("notes.pin.none", "Na ploše není žádný lístek."));
    say(on ? t("notes.pin.throughOn", "Lístky jsou průchozí — myš jde na stránku pod nimi.") : t("notes.pin.throughOff", "Průchozí režim vypnutý — lístky jdou zase ovládat myší."));
  } catch (e) {
    sayError(e);
  }
}

/** Popisek přepínače průchozího režimu (podle toho, co udělá). */
export function throughLabel(s: Snapshot): string {
  const key = s.pins?.key ? ` (${s.pins.key})` : "";
  return anyThrough(s.pins) ? t("notes.pin.throughAllOff", "Lístky: vypnout průchozí režim{key}", { key }) : t("notes.pin.throughAllOn", "Lístky: zapnout průchozí režim{key}", { key });
}

/** Volby navíc v ⋯ karty Poznámky. */
export function pinMenuItems(s: Snapshot): CardMenuItem[] {
  if (!s.notesEnabled) return [];
  const out: CardMenuItem[] = [{ label: t("notes.pin.newMenu", "Nový lístek na plochu"), run: () => void newPin() }];
  if (s.pins?.open.length) out.push({ label: throughLabel(s), run: () => void throughAll() });
  return out;
}
