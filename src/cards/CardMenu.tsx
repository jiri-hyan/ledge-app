// Skrytí karty / sekce jedním klikem: nenápadné ⋯ v hlavičce karty (vidět při najetí) → „Skrýt kartu“
// (karta Claude navíc „Skrýt sekci: …“ pro limity, útratu, Git, subagenty). Uloží jen pole
// `drawer.sections.<k>` existující cestou Nastavení (`get_config` → změna jednoho pole → `save_config`);
// kartu pak schová nový snapshot. Po skrytí lišta
// „… skryta · Vrátit“ na pár sekund (`HiddenNotice`, vykresluje `MainPage`) — žádný modal.
import { createSignal, For, onCleanup, Show } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { isPart, sectionItem, withSection, type SectionKey } from "./sections";

const UNDO_MS = 6000;
const [hidden, setHidden] = createSignal<SectionKey | null>(null);
let undoTimer: number | undefined;

/** Změní jedno pole `drawer.sections` a uloží config. Při chybě configu (rozbitý soubor = v paměti výchozí
 *  hodnoty) neukládá — uživatelův config by se přepsal; to patří do Nastavení. */
export async function setSection(k: SectionKey, on: boolean, cfgError: string | null): Promise<boolean> {
  if (cfgError) {
    say(t("cards.hide.cfgError", "Config má chybu — karty teď skryješ jen v Nastavení."), true);
    return false;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) return false;
    await api.saveConfig(withSection(cfg, k, on));
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

async function hide(k: SectionKey, cfgError: string | null) {
  if (!(await setSection(k, false, cfgError))) return;
  clearTimeout(undoTimer);
  setHidden(k);
  undoTimer = window.setTimeout(() => setHidden(null), UNDO_MS);
}

/** ⋯ v hlavičce karty. `k` = přepínač karty v `drawer.sections`; `parts` = zapnuté sekce uvnitř karty
 *  (karta Claude: limity, útrata, Git, subagenti) → položky „Skrýt sekci: …“. */
export default function CardMenu(p: { k: SectionKey; parts?: SectionKey[]; cfgError: string | null }) {
  const [open, setOpen] = createSignal(false);
  let root: HTMLSpanElement | undefined;
  const outside = (e: PointerEvent) => {
    if (root && !root.contains(e.target as Node)) close();
  };
  const close = () => {
    setOpen(false);
    document.removeEventListener("pointerdown", outside, true);
  };
  const toggle = () => {
    if (open()) return close();
    setOpen(true);
    document.addEventListener("pointerdown", outside, true);
  };
  onCleanup(() => document.removeEventListener("pointerdown", outside, true));
  const pick = (k: SectionKey) => {
    close();
    void hide(k, p.cfgError);
  };
  return (
    <span class="card-menu" classList={{ open: open() }} ref={root}>
      <button class="card-menu-btn" aria-haspopup="menu" aria-expanded={open()} title={t("cards.hide.more", "Další volby")} aria-label={t("cards.hide.more", "Další volby")} onClick={toggle}>⋯</button>
      <Show when={open()}>
        <span class="card-menu-list" role="menu">
          <button role="menuitem" onClick={() => pick(p.k)}>{t("cards.hide.card", "Skrýt kartu")}</button>
          <For each={p.parts ?? []}>
            {(k) => <button role="menuitem" onClick={() => pick(k)}>{t("cards.hide.part", "Skrýt sekci: {name}", { name: sectionName(k) })}</button>}
          </For>
        </span>
      </Show>
    </span>
  );
}

const sectionName = (k: SectionKey) => {
  const it = sectionItem(k);
  return it ? t(it.key, it.cs) : k;
};

/** Lišta po skrytí: „Karta X skryta · Vrátit“. Vrátit = totéž pole zpět na `true`. */
export function HiddenNotice(p: { cfgError: string | null }) {
  const undo = async (k: SectionKey) => {
    clearTimeout(undoTimer);
    setHidden(null);
    await setSection(k, true, p.cfgError);
  };
  return (
    <Show when={hidden()}>
      {(k) => (
        <div class="card-undo" role="status">
          <span>{isPart(k()) ? t("cards.hide.doneSection", "Sekce „{name}“ skryta", { name: sectionName(k()) }) : t("cards.hide.doneCard", "Karta „{name}“ skryta", { name: sectionName(k()) })}</span>
          <button class="mini" onClick={() => void undo(k())}>{t("cards.hide.undo", "Vrátit")}</button>
        </div>
      )}
    </Show>
  );
}
