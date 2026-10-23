// Stav úprav vzhledu zástupce v šuplíku: který zástupce se právě upravuje (jeden panel naráz), živý
// náhled (dlaždice ho kreslí místo uloženého vzhledu — `lookOf`), uložení jen pole `look` toho projektu /
// akce (`get_config` → `withTileLook` → `save_config`, jako ⋯ na kartě) a obrázky z cache podle klíče.
import { createSignal } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { normalizeTileLook, sameLook, withTileLook, type LookKind, type TileLook } from "./model";

export type LookTarget = { kind: LookKind; id: string; name: string; initial: TileLook | undefined };

const [target, setTarget] = createSignal<LookTarget | null>(null);
const [draft, setDraft] = createSignal<TileLook | undefined>(undefined);
/** fokus si vzal panel (psaní do pole) → při zavření ho vrátit */
let tookFocus = false;

export { target as lookTarget, draft as lookDraft };

/** Vzhled, který má dlaždice kreslit: rozpracovaný (živý náhled), jinak uložený. */
export const lookOf = (kind: LookKind, id: string, saved: TileLook | undefined): TileLook | undefined => {
  const tg = target();
  return tg && tg.kind === kind && tg.id === id ? draft() : saved;
};

/** Otevřít panel „Upravit vzhled…“ (jen výslovný klik). Jiný otevřený se nejdřív uloží. */
export function openLook(tg: Omit<LookTarget, "initial"> & { initial: TileLook | undefined }, cfgError: string | null) {
  if (cfgError) {
    say(t("cards.tile.cfgError", "Nastavení obsahuje chybu — vzhled teď změníš jen v Nastavení."), true);
    return;
  }
  const cur = target();
  if (cur && (cur.kind !== tg.kind || cur.id !== tg.id)) void closeLook(true);
  const initial = normalizeTileLook(tg.initial);
  setTarget({ ...tg, initial });
  setDraft(initial);
}

export const setLook = (l: TileLook | undefined) => setDraft(normalizeTileLook(l));

/** Psaní do pole panelu (emoji, popisek, hledání ikon) potřebuje klávesnici — jen po kliku do pole. */
export function wantKeys() {
  if (tookFocus) return;
  tookFocus = true;
  void api.focusDrawer();
}

/** Uloží vzhled jednoho zástupce (jen pole `look` v configu). */
export async function saveTileLook(kind: LookKind, id: string, look: TileLook | undefined): Promise<boolean> {
  try {
    const cfg = await api.getConfig();
    if (!cfg) return false;
    await api.saveConfig(withTileLook(cfg, kind, id, look));
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Zavřít panel: `save` = uložit rozpracované (Hotovo, klik mimo), jinak zahodit (Esc). `keepFocus` = režim
 *  úprav šuplíku drží fokus sám. */
export async function closeLook(save: boolean, keepFocus = false) {
  const tg = target();
  const l = draft();
  setTarget(null);
  setDraft(undefined);
  if (tookFocus && !keepFocus) void api.releaseFocus();
  tookFocus = false;
  if (save && tg && !sameLook(l, tg.initial)) await saveTileLook(tg.kind, tg.id, l);
}

// ---------- vlastní obrázky (klíč → data URL, jednou na klíč) ----------

const [urls, setUrls] = createSignal<Record<string, string>>({});
const asked = new Set<string>();

/** Data URL obrázku z cache; `undefined` = ještě se načítá nebo chybí (dlaždice pak ukáže Auto). */
export function lookImageSrc(key: string): string | undefined {
  if (!asked.has(key)) {
    asked.add(key);
    api.lookImage(key).then((u) => u && setUrls((m) => ({ ...m, [key]: u })), () => asked.delete(key));
  }
  return urls()[key];
}

/** Výběr obrázku (systémový dialog) → klíč v cache; zrušení = null. */
export async function pickLookImage(): Promise<string | null> {
  try {
    const path = await api.pickPath("image", t("cards.tile.pickImage", "Obrázek pro zástupce"));
    if (!path) return null;
    return (await api.lookImageAdd(path)) ?? null;
  } catch (e) {
    sayError(e);
    return null;
  }
}
