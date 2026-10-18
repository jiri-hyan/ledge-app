// Výběr vzhledu karty Právě hraje: malé tlačítko v rohu karty (vidět při najetí) → nabídka s miniaturami
// všech pěti vzhledů; klik = okamžitá změna (uloží jen `media.variant` existující cestou Nastavení:
// `get_config` → jedno pole → `save_config`), v nabídce pak „Vrátit“. Miniatury (`Thumb`) používá i
// Nastavení → Osobní. Do potvrzení snapshotem platí vybraný vzhled hned (`shownVariant`).
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, type MediaLook, type MediaVariant } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
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
    say(t("cards.media.cfgError", "Config má chybu — vzhled teď změníš jen v Nastavení."), true);
    return false;
  }
  clearTimeout(settle);
  setOptimistic(v);
  try {
    const cfg = await api.getConfig();
    if (!cfg) throw new Error(t("cards.media.noConfig", "Config se nepodařilo načíst."));
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

const IcLooks = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true">
    <rect x="4" y="4" width="7" height="7" rx="1.5" />
    <rect x="13" y="4" width="7" height="7" rx="3.5" />
    <rect x="4" y="13" width="16" height="7" rx="1.5" />
  </svg>
);

// stav nabídky mimo komponentu: po změně vzhledu se karta překreslí (Jen v proužku má tlačítko jinde)
// a nabídka i „Vrátit“ musí zůstat otevřené
const [open, setOpen] = createSignal(false);
const [undo, setUndo] = createSignal<MediaVariant | null>(null);

/** Tlačítko v rohu karty + nabídka vzhledů s miniaturami a „Vrátit“. `push` = odsadit doprava (hlavička bez názvu aplikace). */
export function VariantPicker(p: { current: MediaVariant; cfgError: string | null; push?: boolean }) {
  let root: HTMLSpanElement | undefined;
  const outside = (e: PointerEvent) => {
    if (root && !root.contains(e.target as Node)) close();
  };
  const close = () => {
    setOpen(false);
    setUndo(null);
    document.removeEventListener("pointerdown", outside, true);
  };
  const toggle = () => {
    if (open()) return close();
    setOpen(true);
    document.addEventListener("pointerdown", outside, true);
  };
  onMount(() => open() && document.addEventListener("pointerdown", outside, true));
  onCleanup(() => document.removeEventListener("pointerdown", outside, true));
  const pick = async (v: MediaVariant) => {
    if (v === p.current) return;
    const before = p.current;
    if (await saveVariant(v, p.cfgError)) setUndo(before);
  };
  return (
    <span class="med-pick" classList={{ "med-pick-open": open(), "med-pick-push": p.push }} ref={root}>
      <button class="med-pick-btn" aria-haspopup="menu" aria-expanded={open()} title={t("cards.media.look", "Vzhled přehrávače")} aria-label={t("cards.media.look", "Vzhled přehrávače")} onClick={toggle}>
        <IcLooks />
      </button>
      <Show when={open()}>
        <span class="med-pick-list" role="menu">
          <For each={VARIANTS}>
            {(v) => (
              <button role="menuitemradio" aria-checked={v === p.current} classList={{ "med-pick-on": v === p.current }} onClick={() => void pick(v)}>
                <Thumb v={v} />
                <span>{variantName(v)}</span>
              </button>
            )}
          </For>
          <Show when={undo()}>
            {(u) => (
              <button class="med-pick-undo" onClick={() => void saveVariant(u(), p.cfgError).then((ok) => ok && setUndo(null))}>
                {t("cards.media.undo", "Vrátit: {name}", { name: variantName(u()) })}
              </button>
            )}
          </Show>
        </span>
      </Show>
    </span>
  );
}
