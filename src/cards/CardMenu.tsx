// ⋯ v hlavičce karty (vidět při najetí): rychlé volby „Velikost: S/M/L/XL“ a „Výška: Auto/Nízká/Vysoká“,
// volby navíc z `children` (karta Projekty: dlaždice, popisek, git, které projekty), „Skrýt kartu“
// (odebere ji z této záložky do katalogu), „Skrýt sekci: …“ (části karty — limity, útrata, Git,
// subagenti v kartě Claude; hlasitost; baterie), „Vzhled karty…“ (vlna 8 P: akcent, pozadí, hlavička,
// hustota, varianta — panel `layout/CardLook.tsx` pod mřížkou) a „Upravit šuplík…“ (režim úprav). Ukládá jen pole
// `layout` (`layout/edit.ts` `saveLayout`: `get_config` → změna → `save_config`); kartu překreslí nový
// snapshot. Po skrytí lišta „… skryta · Vrátit“ na pár sekund (`HiddenNotice`, vykresluje `MainPage`) —
// žádný modal. Seznam se kreslí přes `Portal` s pevnou pozicí u tlačítka: karta s omezenou výškou
// (obsah scrolluje) ho neořízne a u spodního okraje šuplíku se otevře nahoru. `extra` = vlastní volby
// karty nahoře (widget: Obnovit teď, Upravit skript, Zobrazit výstup, V proužku).
// Vlna 9: Velikost nabízí jen velikosti, které při počtu sloupců vypadají jinak (`size.ts` `usableSizes`);
// volba bez viditelného efektu se ohlásí v liště (`sizeOutcome` → „Karta je v řadě sama…“) a vybere se
// nejbližší možná — nikdy tiché nic. Každá změna velikosti / výšky / stropu má „Vrátit“; v režimu úprav jde
// do draftu (`tweak`). „Řádků“ = strop karet Claude, Repa, Běží. Pozice seznamu z `ui/place.ts`.
import { createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import { Portal } from "solid-js/web";
import { t } from "../i18n";
import { change, editing, offerUndo, saveLayout } from "../layout/edit";
import { QUICK_HEIGHTS, QUICK_LIMITS, removeCard, restoreCard, setHeight, setLimit, setPartHidden, setSize, type CardHeight, type CardId, type CardSize, type Layout, type PartKey } from "../layout/model";
import { fitSize, usableSizes, type SizeOutcome } from "../layout/size";
import { colsNow } from "../layout/cols";
import { cardName, partName, sizeName, sizeNoteText, tx } from "../layout/text";
import { placePopup } from "../ui/place";
import { openCardLook } from "../layout/CardLook";
import type { CardLook } from "../layout/look";
import "./cardmenu.css";

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

/** Skrytí s lištou „… · Vrátit“ (i pro „Odebrat“ dlaždici z karty Projekty, vlna 9 Z). */
export const hideWithUndo = hide;

/** Změna z ⋯ s lištou „… · Vrátit“: v režimu úprav do draftu (Zpět v liště úprav), jinak do configu. */
function tweak(text: string, f: (l: Layout) => { layout: Layout; undo: (l: Layout) => Layout }, cfgError: string | null) {
  if (editing()) {
    change((l) => f(l).layout);
    offerUndo(text);
    return;
  }
  void hide(text, f, cfgError);
}

type Pos = { x: number; y: number; maxH: number };

/** Vlastní volba karty v ⋯ (text už přeložený). */
export type CardMenuItem = { label: string; run: () => void; disabled?: boolean };

/** Řádek rychlé volby v menu: popisek + až 4 tlačítka (víc voleb = jinde, ne segment). */
export function MenuChoice<T>(p: { label: string; items: readonly { v: T; text: string; title?: string }[]; value: T; onPick: (v: T) => void }) {
  return (
    <span class="card-menu-choice" role="group" aria-label={p.label}>
      <span class="card-menu-cap">{p.label}</span>
      <span class="card-menu-seg">
        <For each={p.items}>
          {(it) => (
            <button role="menuitemradio" aria-checked={it.v === p.value} classList={{ on: it.v === p.value }} title={it.title} onClick={() => it.v !== p.value && p.onPick(it.v)}>
              {it.text}
            </button>
          )}
        </For>
      </span>
    </span>
  );
}

/** ⋯ v hlavičce karty `card` na záložce `page`; `parts` = viditelné části karty → „Skrýt sekci: …“;
 *  `size` / `h` = dnešní šířka a výška (rychlé volby); `children` = volby navíc (karta Projekty). */
export default function CardMenu(p: {
  card: CardId;
  page: string;
  parts?: PartKey[];
  size?: CardSize;
  h?: CardHeight;
  /** strop řádků (jen karty, které ho mají) */
  limit?: number;
  /** co udělá volba velikosti na téhle záložce (MainPage zná pořadí karet) */
  outcome?: (want: CardSize) => SizeOutcome;
  /** uložený vzhled karty (`layout…cards[].look`) — výchozí stav panelu Vzhled karty */
  look?: CardLook;
  cfgError: string | null;
  onEdit?: () => void;
  children?: JSX.Element;
  extra?: CardMenuItem[];
}) {
  const [pos, setPos] = createSignal<Pos | null>(null);
  const open = () => pos() !== null;
  let root: HTMLSpanElement | undefined;
  let list: HTMLSpanElement | undefined;
  const outside = (e: PointerEvent) => {
    const n = e.target as Node;
    if (!root?.contains(n) && !list?.contains(n)) close();
  };
  /** posun mřížky / karty pod pevným menu = zavřít (ne posun seznamu projektů v menu) */
  const scrolled = (e: Event) => {
    if (!list?.contains(e.target as Node)) close();
  };
  const close = () => {
    setPos(null);
    document.removeEventListener("pointerdown", outside, true);
    document.removeEventListener("scroll", scrolled, true);
  };
  const [cols, setCols] = createSignal(4);
  /** umístění přes `ui/place.ts`: pod ⋯ zarovnané vpravo, u spodního okraje nad něj, vždy celé v okně */
  const place = () => {
    if (!root || !list) return;
    const r = root.getBoundingClientRect();
    const at = placePopup({ left: r.left, top: r.top, right: r.right, bottom: r.bottom }, { w: list.offsetWidth, h: list.scrollHeight }, { w: window.innerWidth, h: window.innerHeight }, { align: "end" });
    setPos({ x: at.x, y: at.y, maxH: at.maxH });
  };
  const toggle = () => {
    if (open() || !root) return close();
    setCols(colsNow());
    const r = root.getBoundingClientRect();
    setPos({ x: Math.max(0, r.right - 220), y: r.bottom + 4, maxH: window.innerHeight });
    queueMicrotask(place);
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("scroll", scrolled, true);
  };
  onCleanup(close);
  const hideCard = () => {
    close();
    tweak(t("cards.hide.doneCard", "Karta „{name}“ skryta", { name: cardName(p.card) }), (l) => {
      const r = removeCard(l, p.page, p.card);
      return { layout: r.layout, undo: (x) => (r.undo ? restoreCard(x, r.undo) : x) };
    }, p.cfgError);
  };
  const hidePart = (k: PartKey) => {
    close();
    tweak(t("cards.hide.doneSection", "Sekce „{name}“ skryta", { name: partName(k) }), (l) => ({ layout: setPartHidden(l, k, true), undo: (x) => setPartHidden(x, k, false) }), p.cfgError);
  };
  // jen velikosti, které při dnešním počtu sloupců vypadají jinak (ve 2 sloupcích S a celá šířka)
  const sizeItems = () => usableSizes(cols()).map((s) => ({ v: s, text: s.toUpperCase(), title: sizeName(s) }));
  const heightItems = () => QUICK_HEIGHTS.map((q) => ({ v: q.h, text: tx(q.name) }));
  // vlastní výška mimo rychlé volby (1 nebo 3 řádky z úprav) = žádné tlačítko není zvýrazněné
  const quickH = () => (QUICK_HEIGHTS.some((q) => q.h === p.h) ? p.h : null);
  const limitItems = () =>
    QUICK_LIMITS.map((n) => ({
      v: n,
      text: n ? String(n) : t("cards.menu.limitAll", "Vše"),
      title: n ? t("cards.menu.limitTitle", "Nejvýš {n} řádků, zbytek pod „Zobrazit další“", { n }) : t("cards.menu.limitAllTitle", "Bez stropu — všechny řádky"),
    }));
  const name = () => cardName(p.card);
  const pickSize = (v: CardSize) => {
    close();
    const o = p.outcome?.(v) ?? { size: fitSize(v, cols()), span: 0, note: null };
    const prev = p.size;
    const text = [t("cards.menu.sizeDone", "„{name}“: {size}", { name: name(), size: sizeName(o.size) }), sizeNoteText(o, v, cols())].filter(Boolean).join(" ");
    tweak(text, (l) => ({ layout: setSize(l, p.page, p.card, o.size), undo: (x) => (prev ? setSize(x, p.page, p.card, prev) : x) }), p.cfgError);
  };
  const pickHeight = (v: CardHeight | undefined) => {
    close();
    const prev = p.h;
    const label = QUICK_HEIGHTS.find((q) => q.h === v);
    tweak(t("cards.menu.heightDone", "„{name}“: výška {h}", { name: name(), h: label ? tx(label.name) : String(v) }), (l) => ({ layout: setHeight(l, p.page, p.card, v), undo: (x) => setHeight(x, p.page, p.card, prev) }), p.cfgError);
  };
  const pickLimit = (n: number) => {
    close();
    const prev = p.limit;
    const text = n ? t("cards.menu.limitDone", "„{name}“: nejvýš {n} řádků, zbytek pod „Zobrazit další“", { name: name(), n }) : t("cards.menu.limitAllDone", "„{name}“: všechny řádky", { name: name() });
    tweak(text, (l) => ({ layout: setLimit(l, p.page, p.card, n), undo: (x) => setLimit(x, p.page, p.card, prev) }), p.cfgError);
  };
  return (
    <span class="card-menu" classList={{ open: open() }} ref={root} data-cmenu>
      <button class="card-menu-btn" aria-haspopup="menu" aria-expanded={open()} title={t("cards.hide.more", "Další volby")} aria-label={t("cards.hide.more", "Další volby")} onClick={toggle}>⋯</button>
      <Show when={pos()}>
        {(at) => (
          <Portal mount={root?.closest(".shell") ?? document.body}>
            <span class="card-menu-list" role="menu" ref={list} style={{ position: "fixed", left: `${at().x}px`, top: `${at().y}px`, "max-height": `${at().maxH}px`, "overflow-y": "auto" }}>
              <For each={p.extra ?? []}>
                {(x) => <button role="menuitem" disabled={x.disabled} onClick={() => { close(); x.run(); }}>{x.label}</button>}
              </For>
              <Show when={(p.extra ?? []).length > 0}><span class="card-menu-sep" /></Show>
              <Show when={p.size}>
                {(sz) => <MenuChoice label={t("cards.menu.size", "Velikost")} items={sizeItems()} value={fitSize(sz(), cols())} onPick={pickSize} />}
              </Show>
              <MenuChoice label={t("cards.menu.height", "Výška")} items={heightItems()} value={quickH()} onPick={(v) => pickHeight(v ?? undefined)} />
              <Show when={p.limit !== undefined}>
                <MenuChoice label={t("cards.menu.limit", "Řádků")} items={limitItems()} value={p.limit !== undefined && QUICK_LIMITS.includes(p.limit) ? p.limit : -1} onPick={pickLimit} />
              </Show>
              {p.children}
              <span class="card-menu-sep" />
              <button role="menuitem" onClick={() => { close(); openCardLook(p.page, p.card, p.look, p.cfgError); }}>{t("cards.cardlook.menu", "Vzhled karty…")}</button>
              <button role="menuitem" onClick={hideCard}>{t("cards.hide.card", "Skrýt kartu")}</button>
              <For each={p.parts ?? []}>
                {(k) => <button role="menuitem" onClick={() => hidePart(k)}>{t("cards.hide.part", "Skrýt sekci: {name}", { name: partName(k) })}</button>}
              </For>
              <Show when={p.onEdit}>
                <button role="menuitem" onClick={() => { close(); p.onEdit?.(); }}>{t("cards.layout.editMenu", "Upravit šuplík…")}</button>
              </Show>
            </span>
          </Portal>
        )}
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
