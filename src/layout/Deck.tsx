// Mřížka karet jedné záložky: `.deck` (sloupce z CSS proměnné `--deck-cols` — určuje styl, vlna 6 F),
// každá karta v `.deck-item` s `data-size` (s|m|l|xl), `data-card` a volitelně `data-h` (výška v řádcích;
// obsah pak scrolluje uvnitř karty). Rozpětí počítá `size.ts` `packSpans` (řada bez prázdného místa —
// volné sloupce dostanou karty té řady), sloupce se čtou z `--deck-cols` při změně vzhledu (MutationObserver
// na `<html>`, žádné časovače).
// Režim úprav: karty se jemně rozvlní, úchyt ⠿ (pointer tažení — HTML5 drag&drop v okně Tauri nefunguje;
// i na záložku v hlavičce = přesun tam), **tah za pravý okraj = šířka, za spodní = výška, za roh = obojí**
// (živý obrys cíle s popiskem „2 × 3“, dvojklik na spodní okraj = výška podle obsahu), ⇔ / ↕ = totéž
// klikem a z klávesnice, × do katalogu (s Vrátit), Alt+šipky na úchytu = přesun; karta bez dat je vidět
// jako zástupce; dlaždice „+ Přidat kartu“ = katalog.
import { createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import { t, tp } from "../i18n";
import { change, dropTab, editPage, offerUndo, setCatalogOpen, setDropTab } from "./edit";
import {
  gridDropIndex, keyMove, moveCard, moveToPage, nextHeight, nextSize, pageOf, removeCard, restoreCard, setHeight, setSize,
  type CardHeight, type CardId, type CardSize, type Layout, type LayoutPage, type Rect,
} from "./model";
import { deckCols, heightPx, packSpans, resizeTo, spanOf, spanPx, type DeckMetrics, type Dim, type Edge } from "./size";
import { cardName, pageName } from "./text";
import "./layout.css";

type Drag = { from: number; to: number; x: number; y: number };
type Resize = { id: CardId; edge: Edge; dim: Dim; w: number; h: number };

export const sizeLabel = (s: CardSize) =>
  ({ s: t("cards.layout.size.s", "Malá"), m: t("cards.layout.size.m", "Střední"), l: t("cards.layout.size.l", "Velká"), xl: t("cards.layout.size.xl", "Přes celou šířku") })[s] ?? s;
export const heightLabel = (h: CardHeight | undefined) =>
  h === undefined ? t("cards.layout.height.autoLong", "podle obsahu") : tp("cards.layout.height.rows", h, ["{n} řádek", "{n} řádky", "{n} řádků"]);

export default function Deck(p: { page: LayoutPage; layout: Layout; editing: boolean; shown: (id: CardId) => boolean; render: (id: CardId) => JSX.Element }) {
  let box!: HTMLDivElement;
  const [drag, setDrag] = createSignal<Drag | null>(null);
  const [rs, setRs] = createSignal<Resize | null>(null);
  const [cols, setCols] = createSignal(4);
  // For podle id (řetězce), ne objektů: změna velikosti, klon rozložení ani nový snapshot kartu nepřestaví
  const ids = () => (p.editing ? p.page.cards : p.page.cards.filter((c) => p.shown(c.id))).map((c) => c.id);
  const cardOf = (id: CardId) => p.page.cards.find((c) => c.id === id);
  const sizeOf = (id: CardId) => cardOf(id)?.size ?? "m";
  const hOf = (id: CardId) => cardOf(id)?.h;
  const spans = createMemo(() => packSpans(ids().map(sizeOf), cols()));

  // sloupce určuje styl (`--deck-cols` na <html>): přečíst při startu a při změně vzhledu
  onMount(() => {
    const read = () => setCols(deckCols(getComputedStyle(box).getPropertyValue("--deck-cols")));
    read();
    const mo = new MutationObserver(read);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "data-layout"] });
    onCleanup(() => mo.disconnect());
  });

  function start(e: PointerEvent, i: number) {
    if (e.button !== 0) return;
    e.preventDefault();
    const items = Array.from(box.querySelectorAll<HTMLElement>(":scope > .deck-item"));
    const rects: Rect[] = items.map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.left, y: r.top, w: r.width, h: r.height };
    });
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const pageId = p.page.id;
    const tabAt = (x: number, y: number) => {
      const id = (document.elementFromPoint(x, y) as HTMLElement | null)?.closest<HTMLElement>("[data-tab]")?.dataset.tab ?? null;
      return id && id !== pageId ? id : null;
    };
    setDrag({ from: i, to: i, x: e.clientX, y: e.clientY });
    const move = (ev: PointerEvent) => {
      const tab = tabAt(ev.clientX, ev.clientY);
      setDropTab(tab);
      setDrag({ from: i, to: tab ? i : gridDropIndex(rects, i, ev.clientX, ev.clientY), x: ev.clientX, y: ev.clientY });
    };
    const end = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      const d = drag();
      const tab = dropTab();
      setDrag(null);
      setDropTab(null);
      if (!d) return;
      if (tab) toPage(i, tab);
      else if (d.to !== d.from) change((l) => moveCard(l, pageId, d.from, d.to));
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  }

  /** rozměry mřížky v okamžiku chycení okraje (řádek `--deck-row` změří sonda — proměnná je calc) */
  function metrics(item: HTMLElement): DeckMetrics {
    const cs = getComputedStyle(box);
    const gap = Number.parseFloat(cs.columnGap) || 12;
    const inner = box.clientWidth - (Number.parseFloat(cs.paddingLeft) || 0) - (Number.parseFloat(cs.paddingRight) || 0);
    const probe = document.createElement("div");
    probe.className = "lay-probe";
    box.appendChild(probe);
    const row = probe.getBoundingClientRect().height || 104;
    probe.remove();
    const r = item.getBoundingClientRect();
    return { cols: cols(), colW: (inner - (cols() - 1) * gap) / cols(), gap, row, left: r.left, top: r.top };
  }

  /** tah za okraj karty: živý obrys cíle, pustit = nová šířka / výška (Esc = zrušit) */
  function resize(e: PointerEvent, id: CardId, edge: Edge) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const el = e.currentTarget as HTMLElement;
    const item = el.closest<HTMLElement>(".deck-item");
    if (!item) return;
    el.setPointerCapture(e.pointerId);
    const m = metrics(item);
    const natural = item.getBoundingClientRect().height;
    const right = box.getBoundingClientRect().right - (Number.parseFloat(getComputedStyle(box).paddingRight) || 0);
    const startDim: Dim = { size: sizeOf(id), h: hOf(id) };
    const pageId = p.page.id;
    const frame = (dim: Dim): Resize => ({
      id,
      edge,
      dim,
      w: Math.min(spanPx(spanOf(dim.size, m.cols), m.colW, m.gap), right - m.left),
      h: dim.h === undefined ? natural : heightPx(dim.h, m.row, m.gap),
    });
    setRs(frame(startDim));
    const move = (ev: PointerEvent) => setRs(frame(resizeTo(m, edge, startDim, ev.clientX, ev.clientY)));
    const key = (ev: KeyboardEvent) => ev.key === "Escape" && (ev.stopPropagation(), finish(false));
    const finish = (apply: boolean) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      const r = rs();
      setRs(null);
      if (!apply || !r) return;
      if (r.dim.size !== startDim.size) change((l) => setSize(l, pageId, id, r.dim.size));
      if (r.dim.h !== startDim.h) change((l) => setHeight(l, pageId, id, r.dim.h));
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
  }

  /** přesun na jinou záložku (tažením na její název) — s Vrátit */
  function toPage(i: number, tab: string) {
    const id = p.page.cards[i]?.id;
    if (!id) return;
    const from = p.page.id;
    const prev = p.layout;
    change((l) => moveToPage(l, from, id, tab));
    offerUndo(t("cards.layout.moved", "„{name}“ přesunuta na záložku {tab}", { name: cardName(id), tab: pageName(pageOf(prev, tab)) }), () => change(() => prev));
  }

  function remove(id: CardId) {
    const r = removeCard(p.layout, p.page.id, id);
    if (!r.undo) return;
    const u = r.undo;
    change(() => r.layout);
    offerUndo(t("cards.layout.removed", "Karta „{name}“ odebrána do katalogu", { name: cardName(id) }), () => change((l) => restoreCard(l, u)));
  }

  const key = (e: KeyboardEvent, i: number) => {
    if (!e.altKey) return;
    const to = keyMove(e.key, i, p.page.cards.length);
    if (to === null) return;
    e.preventDefault();
    change((l) => moveCard(l, p.page.id, i, to));
    // úchyt přesunuté karty zpět do fokusu
    requestAnimationFrame(() => (box.children[to]?.querySelector(".lay-grip") as HTMLElement | null)?.focus());
  };

  const dimText = (d: Dim) => t("cards.layout.dim", "{w} × {h}", { w: spanOf(d.size, cols()), h: d.h ?? t("cards.layout.height.autoShort", "auto") });

  return (
    <div class="deck" classList={{ editing: p.editing, dragging: !!drag(), resizing: !!rs() }} ref={box} data-page={p.page.id}>
      <For each={ids()}>
        {(id, i) => (
          <div
            class="deck-item"
            data-size={sizeOf(id)}
            data-card={id}
            data-h={hOf(id)}
            style={{ "--span": String(spans()[i()] ?? 1), "--h": hOf(id) === undefined ? undefined : String(hOf(id)) }}
            classList={{ lifted: drag()?.from === i(), "drop-at": !!drag() && drag()!.to === i() && drag()!.from !== i() }}
          >
            <Show when={p.shown(id)} fallback={
              <div class="card lay-ghost">
                <h4>{cardName(id)}</h4>
                <div class="empty">{t("cards.layout.ghost", "Teď nemá co ukázat — objeví se sama, až bude mít data.")}</div>
              </div>
            }>
              {p.render(id)}
            </Show>
            <Show when={p.editing}>
              <div class="lay-tools">
                <button
                  class="lay-grip"
                  title={t("cards.layout.grip", "Přetáhni na jiné místo nebo na záložku (Alt+šipky)")}
                  aria-label={t("cards.layout.grip.aria", "Přesunout {name}: Alt+šipka", { name: cardName(id) })}
                  onPointerDown={(e) => start(e, i())}
                  onKeyDown={(e) => key(e, i())}
                >⠿</button>
                <button
                  class="lay-size"
                  title={t("cards.layout.sizeTitle", "Velikost: {size} — klik změní", { size: sizeLabel(sizeOf(id)) })}
                  aria-label={t("cards.layout.sizeTitle", "Velikost: {size} — klik změní", { size: sizeLabel(sizeOf(id)) })}
                  onClick={() => change((l) => setSize(l, p.page.id, id, nextSize(sizeOf(id))))}
                >⇔ <b>{sizeOf(id).toUpperCase()}</b></button>
                <button
                  class="lay-size"
                  title={t("cards.layout.heightTitle", "Výška: {h} — klik změní", { h: heightLabel(hOf(id)) })}
                  aria-label={t("cards.layout.heightTitle", "Výška: {h} — klik změní", { h: heightLabel(hOf(id)) })}
                  onClick={() => change((l) => setHeight(l, p.page.id, id, nextHeight(hOf(id))))}
                >↕ <b>{hOf(id) ?? t("cards.layout.height.autoShort", "auto")}</b></button>
                <button class="lay-remove" title={t("cards.layout.remove", "Odebrat do katalogu")} aria-label={t("cards.layout.remove", "Odebrat do katalogu")} onClick={() => remove(id)}>×</button>
              </div>
              {/* úchyty okrajů: pravý = šířka, spodní = výška (dvojklik = podle obsahu), roh = obojí */}
              <div class="lay-rs lay-rs-e" aria-hidden="true" title={t("cards.layout.rs.e", "Táhni = šířka (přichytí se na sloupce)")} onPointerDown={(e) => resize(e, id, "e")} />
              <div
                class="lay-rs lay-rs-s"
                aria-hidden="true"
                title={t("cards.layout.rs.s", "Táhni = výška · dvojklik = podle obsahu")}
                onPointerDown={(e) => resize(e, id, "s")}
                onDblClick={() => change((l) => setHeight(l, p.page.id, id, undefined))}
              />
              <div class="lay-rs lay-rs-se" aria-hidden="true" title={t("cards.layout.rs.se", "Táhni = šířka i výška")} onPointerDown={(e) => resize(e, id, "se")}>◢</div>
              <Show when={rs()?.id === id ? rs() : null}>
                {(r) => (
                  <div class="lay-target" style={{ width: `${r().w}px`, height: `${r().h}px` }} aria-hidden="true">
                    <span class="lay-dim">{dimText(r().dim)}</span>
                  </div>
                )}
              </Show>
            </Show>
          </div>
        )}
      </For>
      <Show when={p.editing && editPage() === p.page.id}>
        <button class="lay-add" onClick={() => setCatalogOpen(true)}>{t("cards.layout.add", "+ Přidat kartu")}</button>
      </Show>
      <Show when={drag()}>
        {(d) => (
          <div class="lay-float" style={{ left: `${d().x}px`, top: `${d().y}px` }} aria-hidden="true">
            {cardName(p.page.cards[d().from]?.id ?? "projects")}
            <Show when={dropTab()}>{(tab) => <span> → {pageName(pageOf(p.layout, tab()))}</span>}</Show>
          </div>
        )}
      </Show>
    </div>
  );
}
