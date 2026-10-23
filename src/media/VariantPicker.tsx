// Výběr vzhledu karty Právě hraje: malé tlačítko v rohu karty (vidět při najetí) → nabídka (společné menu
// `ui/Popover.tsx`) s miniaturami všech pěti vzhledů; klik = okamžitá změna (uloží jen `media.variant` existující cestou Nastavení:
// `get_config` → jedno pole → `save_config`), v nabídce pak „Vrátit“. Miniatury (`Thumb`) používá i
// Nastavení → Osobní. Do potvrzení snapshotem platí vybraný vzhled hned (`shownVariant`). Je to zkratka:
// kompletní vzhled karty (akcent, pozadí, hlavička, hustota + tytéž miniatury) je v panelu „Vzhled karty…“
// (`layout/CardLook.tsx`), kam vede poslední položka nabídky (`openMediaCardLook`).
import { createSignal, For, Show } from "solid-js";
import { api, type MediaLook, type MediaVariant } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { boxOf, PopItem, PopMenu } from "../ui/Popover";
import type { Box } from "../ui/place";
import { activePageId } from "../theme/apply";
import { normalizeLayout, pageOf } from "../layout/model";
import { openCardLook } from "../layout/lookState";
import { VARIANTS, variantOf } from "./timeline";

/** Lidský název vzhledu. */
export function variantName(v: MediaVariant): string {
  switch (v) {
    case "compact":
      return t("cards.media.v.compact", "Kompaktní řádek");
    case "card":
      return t("cards.media.v.card", "Karta s průběhem");
    case "cover":
      return t("cards.media.v.cover", "Velký obal");
    case "vinyl":
      return t("cards.media.v.vinyl", "Vinyl");
    case "strip":
      return t("cards.media.v.strip", "Jen v proužku");
  }
}

/** Jedna věta, co vzhled dělá (Nastavení). */
export function variantHint(v: MediaVariant): string {
  switch (v) {
    case "compact":
      return t("cards.media.vh.compact", "Jeden řádek — víc místa pro ostatní karty.");
    case "card":
      return t("cards.media.vh.card", "Obal, název, čas skladby a přetáčení.");
    case "cover":
      return t("cards.media.vh.cover", "Rozmazaný obal přes celou kartu.");
    case "vinyl":
      return t("cards.media.vh.vinyl", "Deska se točí jen při přehrávání.");
    case "strip":
      return t("cards.media.vh.strip", "Skladba v proužku nahoře, v šuplíku jen malý ovladač.");
  }
}

/** Miniatura vzhledu (jen tvary z tokenů, žádná data). `lg` = větší náhled v Nastavení. */
export function Thumb(p: { v: MediaVariant; lg?: boolean }) {
  return (
    <span
      class="med-thumb"
      classList={{ "med-thumb-lg": p.lg, "med-th-compact": p.v === "compact", "med-th-cover": p.v === "cover", "med-th-vinyl": p.v === "vinyl", "med-th-strip": p.v === "strip" }}
      aria-hidden="true"
    >
      <i class="med-th-art" />
      <i class="med-th-line" />
      <i class="med-th-bar" />
    </span>
  );
}

const [optimistic, setOptimistic] = createSignal<MediaVariant | null>(null);
let settle: number | undefined;

/** Vzhled, který karta ukáže: právě vybraný (do potvrzení snapshotem), jinak ze snapshotu. */
export const shownVariant = (look: MediaLook | null | undefined): MediaVariant => optimistic() ?? variantOf(look);

/** Uloží `media.variant`. Rozbitý config (v paměti výchozí hodnoty) neukládá — přepsal by uživatelův. */
export async function saveVariant(v: MediaVariant, cfgError: string | null): Promise<boolean> {
  if (cfgError) {
    say(t("cards.media.cfgError", "Nastavení obsahuje chybu — vzhled teď změníš jen v Nastavení."), true);
    return false;
  }
  clearTimeout(settle);
  setOptimistic(v);
  try {
    const cfg = await api.getConfig();
    if (!cfg) throw new Error(t("cards.media.noConfig", "Nastavení se nepodařilo načíst."));
    await api.saveConfig({ ...cfg, media: { ...(cfg.media ?? {}), variant: v } });
    // nový snapshot přijde do chvilky; pak už platí on
    settle = window.setTimeout(() => setOptimistic(null), 1500);
    return true;
  } catch (e) {
    setOptimistic(null);
    sayError(e);
    return false;
  }
}

/** Otevře panel „Vzhled karty…“ karty Právě hraje na zobrazené záložce (stav `layout/lookState.ts`, panel `CardLook.tsx` se načte líně). */
export async function openMediaCardLook(cfgError: string | null) {
  const cfg = await api.getConfig();
  const l = normalizeLayout(cfg?.layout);
  const page = pageOf(l, activePageId()).id;
  const card = pageOf(l, page).cards.find((c) => c.id === "media");
  openCardLook(page, "media", card?.look, cfgError);
}

const IcLooks = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true">
    <rect x="4" y="4" width="7" height="7" rx="1.5" />
    <rect x="13" y="4" width="7" height="7" rx="3.5" />
    <rect x="4" y="13" width="16" height="7" rx="1.5" />
  </svg>
);

// stav nabídky mimo komponentu: po změně vzhledu se karta překreslí (Jen v proužku má tlačítko jinde)
// a nabídka i „Vrátit“ musí zůstat otevřené
const [at, setAt] = createSignal<{ anchor: Box; keyboard: boolean } | null>(null);
const [undo, setUndo] = createSignal<MediaVariant | null>(null);
const close = () => {
  setAt(null);
  setUndo(null);
};

/** Tlačítko v rohu karty + nabídka vzhledů s miniaturami a „Vrátit“. `push` = odsadit doprava (hlavička bez názvu aplikace). */
export function VariantPicker(p: { current: MediaVariant; cfgError: string | null; push?: boolean }) {
  let btn: HTMLButtonElement | undefined;
  const toggle = (e: MouseEvent) => {
    if (at()) return close();
    const anchor = boxOf(e.currentTarget as Element);
    if (anchor) setAt({ anchor, keyboard: e.detail === 0 });
  };
  const pick = async (v: MediaVariant) => {
    if (v === p.current) return;
    const before = p.current;
    if (await saveVariant(v, p.cfgError)) setUndo(before);
  };
  const label = t("cards.media.look", "Vzhled přehrávače");
  return (
    <span class="med-pick" classList={{ "med-pick-open": !!at(), "med-pick-push": p.push }}>
      <button class="med-pick-btn" ref={btn} aria-haspopup="menu" aria-expanded={!!at()} title={label} aria-label={label} onClick={toggle}>
        <IcLooks />
      </button>
      <Show when={at()}>
        {(a) => (
          <PopMenu anchor={a().anchor} keyboard={a().keyboard} trigger={btn} onClose={close} align="end" label={label}>
            <For each={VARIANTS}>
              {(v) => (
                <PopItem radio on={v === p.current} onClick={() => void pick(v)}>
                  <Thumb v={v} />
                  <span>{variantName(v)}</span>
                </PopItem>
              )}
            </For>
            <PopItem tone="more" onClick={() => { close(); void openMediaCardLook(p.cfgError); }}>
              {t("cards.media.more", "Víc možností karty…")}
            </PopItem>
            <Show when={undo()}>
              {(u) => (
                <PopItem tone="undo" onClick={() => void saveVariant(u(), p.cfgError).then((ok) => ok && setUndo(null))}>
                  {t("cards.media.undo", "Vrátit: {name}", { name: variantName(u()) })}
                </PopItem>
              )}
            </Show>
          </PopMenu>
        )}
      </Show>
    </span>
  );
}
