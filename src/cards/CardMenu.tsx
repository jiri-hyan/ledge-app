// ⋯ v hlavičce karty (vidět při najetí): „Skrýt kartu“ (odebere ji z této záložky do katalogu),
// „Skrýt sekci: …“ (části karty — limity, útrata, Git, subagenti v kartě Claude; hlasitost; baterie)
// a „Upravit šuplík…“ (režim úprav). Ukládá jen pole `layout` (`layout/edit.ts` `saveLayout`:
// `get_config` → změna → `save_config`); kartu schová nový snapshot. Po skrytí lišta „… skryta · Vrátit“
// na pár sekund (`HiddenNotice`, vykresluje `MainPage`) — žádný modal.
import { createSignal, For, onCleanup, Show } from "solid-js";
import { t } from "../i18n";
import { saveLayout } from "../layout/edit";
import { removeCard, restoreCard, setPartHidden, type CardId, type Layout, type PartKey } from "../layout/model";
import { cardName, partName } from "../layout/text";

const UNDO_MS = 6000;
const [hidden, setHidden] = createSignal<{ text: string; undo: (l: Layout) => Layout } | null>(null);
let undoTimer: number | undefined;

async function hide(text: string, f: (l: Layout) => { layout: Layout; undo: (l: Layout) => Layout }, cfgError: string | null) {
  const got: { undo?: (l: Layout) => Layout } = {};
  const ok = await saveLayout((l) => {
    const r = f(l);
    got.undo = r.undo;
    return r.layout;
  }, cfgError);
  if (!ok || !got.undo) return;
  clearTimeout(undoTimer);
  setHidden({ text, undo: got.undo });
  undoTimer = window.setTimeout(() => setHidden(null), UNDO_MS);
}

/** ⋯ v hlavičce karty `card` na záložce `page`; `parts` = viditelné části karty → „Skrýt sekci: …“. */
export default function CardMenu(p: { card: CardId; page: string; parts?: PartKey[]; cfgError: string | null; onEdit?: () => void }) {
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
  const hideCard = () => {
    close();
    void hide(t("cards.hide.doneCard", "Karta „{name}“ skryta", { name: cardName(p.card) }), (l) => {
      const r = removeCard(l, p.page, p.card);
      return { layout: r.layout, undo: (x) => (r.undo ? restoreCard(x, r.undo) : x) };
    }, p.cfgError);
  };
  const hidePart = (k: PartKey) => {
    close();
    void hide(t("cards.hide.doneSection", "Sekce „{name}“ skryta", { name: partName(k) }), (l) => ({ layout: setPartHidden(l, k, true), undo: (x) => setPartHidden(x, k, false) }), p.cfgError);
  };
  return (
    <span class="card-menu" classList={{ open: open() }} ref={root}>
      <button class="card-menu-btn" aria-haspopup="menu" aria-expanded={open()} title={t("cards.hide.more", "Další volby")} aria-label={t("cards.hide.more", "Další volby")} onClick={toggle}>⋯</button>
      <Show when={open()}>
        <span class="card-menu-list" role="menu">
          <button role="menuitem" onClick={hideCard}>{t("cards.hide.card", "Skrýt kartu")}</button>
          <For each={p.parts ?? []}>
            {(k) => <button role="menuitem" onClick={() => hidePart(k)}>{t("cards.hide.part", "Skrýt sekci: {name}", { name: partName(k) })}</button>}
          </For>
          <Show when={p.onEdit}>
            <button role="menuitem" onClick={() => { close(); p.onEdit?.(); }}>{t("cards.layout.editMenu", "Upravit šuplík…")}</button>
          </Show>
        </span>
      </Show>
    </span>
  );
}

/** Lišta po skrytí: „Karta X skryta · Vrátit“. Vrátit = opačná změna rozložení. */
export function HiddenNotice(p: { cfgError: string | null }) {
  const undo = async () => {
    const h = hidden();
    clearTimeout(undoTimer);
    setHidden(null);
    if (h) await saveLayout(h.undo, p.cfgError);
  };
  return (
    <Show when={hidden()}>
      {(h) => (
        <div class="card-undo" role="status">
          <span>{h().text}</span>
          <button class="mini" onClick={() => void undo()}>{t("cards.hide.undo", "Vrátit")}</button>
        </div>
      )}
    </Show>
  );
}
