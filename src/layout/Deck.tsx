// Mřížka karet jedné záložky: `.deck` (sloupce z CSS proměnné `--deck-cols`), každá karta v `.deck-item`
// s `data-size` (s|m|l|xl) a `data-card` — rozhraní pro styly (worker F). V režimu úprav: karty se jemně
// rozvlní (CSS, vypne „omezit pohyb“), nad každou úchyt ⠿ (pointer tažení — HTML5 drag&drop v okně Tauri
// nefunguje; i na záložku v hlavičce = přesun tam), ⇔ velikost, × do katalogu (s Vrátit), Alt+šipky na
// úchytu = přesun z klávesnice; karta bez dat je vidět jako zástupce; dlaždice „+ Přidat kartu“ = katalog.
import { createSignal, For, Show, type JSX } from "solid-js";
import { t } from "../i18n";
import { change, dropTab, editPage, offerUndo, setCatalogOpen, setDropTab } from "./edit";
import { gridDropIndex, keyMove, moveCard, moveToPage, nextSize, pageOf, removeCard, restoreCard, setSize, type CardId, type Layout, type LayoutPage, type Rect } from "./model";
import { cardName, pageName } from "./text";
import "./layout.css";

type Drag = { from: number; to: number; x: number; y: number };

export default function Deck(p: { page: LayoutPage; layout: Layout; editing: boolean; shown: (id: CardId) => boolean; render: (id: CardId) => JSX.Element }) {
  let box!: HTMLDivElement;
  const [drag, setDrag] = createSignal<Drag | null>(null);
  // For podle id (řetězce), ne objektů: změna velikosti, klon rozložení ani nový snapshot kartu nepřestaví
  const ids = () => (p.editing ? p.page.cards : p.page.cards.filter((c) => p.shown(c.id))).map((c) => c.id);
  const sizeOf = (id: CardId) => p.page.cards.find((c) => c.id === id)?.size ?? "m";

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

  const sizeLabel = (s: string) =>
    ({ s: t("cards.layout.size.s", "Malá"), m: t("cards.layout.size.m", "Střední"), l: t("cards.layout.size.l", "Velká"), xl: t("cards.layout.size.xl", "Přes celou šířku") })[s] ?? s;

  return (
    <div class="deck" classList={{ editing: p.editing, dragging: !!drag() }} ref={box} data-page={p.page.id}>
      <For each={ids()}>
        {(id, i) => (
          <div class="deck-item" data-size={sizeOf(id)} data-card={id} classList={{ lifted: drag()?.from === i(), "drop-at": !!drag() && drag()!.to === i() && drag()!.from !== i() }}>
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
                <button class="lay-remove" title={t("cards.layout.remove", "Odebrat do katalogu")} aria-label={t("cards.layout.remove", "Odebrat do katalogu")} onClick={() => remove(id)}>×</button>
              </div>
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
