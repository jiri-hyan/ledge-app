// Mřížka karet jedné záložky: `.deck` (sloupce z CSS proměnné `--deck-cols` — určuje styl, vlna 6 F),
// každá karta v `.deck-item` s `data-size` (s|m|l|xl), `data-card` a volitelně `data-h` (výška v řádcích;
// obsah pak scrolluje uvnitř karty). Rozpětí počítá `size.ts` `packSpans` (řada bez prázdného místa —
// volné sloupce dostane největší karta řady), sloupce se čtou z `--deck-cols` při změně vzhledu
// (MutationObserver na `<html>`, žádné časovače).
// Režim úprav (vlna 9 — „rozložení, které vždy poslechne“): klidný přerušovaný obrys (bez kývání), úchyt ⠿
// = pointer tažení (HTML5 drag&drop v okně Tauri nefunguje) s ŽIVÝM náhledem: tažená karta je hned na místě
// dopadu jako přerušovaný zástupce, ostatní se krátce posunou (FLIP přes Web Animations, `motionNow`),
// u horního / spodního okraje se mřížka posouvá sama, Esc / ztráta ukazatele = zrušit (karta zůstane),
// puštění daleko mimo karty = nic, na ouško záložky = přesun tam (kde už karta je, se neztratí — nápověda).
// Tah za pravý okraj = šířka, za spodní = výška, za roh = obojí: karta se mění ŽIVĚ, přichytí se na sloupce
// a řádky a ukazuje „2 × 3“; dvojklik na spodní okraj = výška podle obsahu. ⇔ / ↕ = totéž klikem (jen
// velikosti, které vypadají jinak). Klávesy na úchytu: šipky přesun, + / − šířka, PageUp / PageDown výška.
// Volba bez viditelného efektu se ohlásí v liště (`sizeOutcome` → `sizeNoteText`). × do katalogu (s Vrátit).
// Vzhled karty (vlna 8 P, `look.ts`): `data-cl-bg` / `data-cl-head` / `data-cl-dens` / `data-cl-accent` a
// `--accent` na `.deck-item` (živý náhled z panelu `CardLook.tsx`); karta bez vzhledu = žádný atribut.
import { batch, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import { t, tp } from "../i18n";
import { motionNow } from "../motion";
import { change, dropTab, editPage, offerUndo, setCatalogOpen, setDropTab } from "./edit";
import {
  gridDropIndex, moveCard, nextHeight, moveToPage, pageMoveBlock, pageOf, removeCard, restoreCard, setHeight, setSize,
  type CardHeight, type CardId, type CardSize, type Layout, type LayoutPage, type Rect,
} from "./model";
import { deckCols, nextSizeFor, packSpans, resizeTo, sizeOutcome, stepHeight, stepSize, type DeckMetrics, type Dim, type Edge } from "./size";
import { autoScrollStep, dropAction, keyAction, previewOrder, type DragEnd } from "./drag";
import { cardName, pageName, sizeName, sizeNoteText } from "./text";
import { cardLookAttrs } from "./look";
import { lookFor } from "./CardLook";
import "./layout.css";

type Drag = { id: CardId; from: number; to: number; x: number; y: number };
type Resize = { id: CardId; edge: Edge; dim: Dim };

/** Kolik px musí ukazatel ujet, než se z kliknutí na úchyt stane tah (klik = fokus pro klávesy). */
const DRAG_START = 4;

export const sizeLabel = (s: CardSize) => sizeName(s);
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
  // během tahu za okraj se karta mění živě (náhled), uloží se až po puštění
  const sizeNow = (id: CardId) => (rs()?.id === id ? rs()!.dim.size : sizeOf(id));
  const hNow = (id: CardId) => (rs()?.id === id ? rs()!.dim.h : hOf(id));
  const cl = (id: CardId) => cardLookAttrs(lookFor(p.page.id, id, cardOf(id)?.look));
  // náhled pořadí během tahu: DOM se nepřeskládá (CSS `order`), takže ukazatel ani fokus o úchyt nepřijdou
  const view = createMemo(() => {
    const d = drag();
    const base = ids();
    return d && !dropTab() ? previewOrder(base, d.from, d.to) : base;
  });
  const spans = createMemo(() => {
    const v = view();
    const sp = packSpans(v.map(sizeNow), cols());
    return new Map(v.map((id, k) => [id, sp[k] ?? 1]));
  });

  // sloupce určuje styl (`--deck-cols` na <html>): přečíst při startu a při změně vzhledu
  onMount(() => {
    const read = () => setCols(deckCols(getComputedStyle(box).getPropertyValue("--deck-cols")));
    read();
    const mo = new MutationObserver(read);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "data-layout"] });
    onCleanup(() => mo.disconnect());
  });

  const items = () => Array.from(box.querySelectorAll<HTMLElement>(":scope > .deck-item"));
  /** změna pořadí / velikosti s krátkým posunem ostatních karet (bez kývání; „bez animací“ = hned) */
  function flip(update: () => void) {
    const m = motionNow();
    if (m.style === "none") return update();
    const els = items();
    const first = els.map((el) => el.getBoundingClientRect());
    update();
    const ms = Math.round(160 / m.speed);
    els.forEach((el, k) => {
      const b = el.getBoundingClientRect();
      const dx = first[k].left - b.left;
      const dy = first[k].top - b.top;
      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) return;
      el.animate?.([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }], { duration: ms, easing: "cubic-bezier(.2,.7,.2,1)" });
    });
  }

  /** nápověda v liště úprav, když volba velikosti nemá viditelný efekt */
  function note(sizes: CardSize[], i: number, want: CardSize) {
    const o = sizeOutcome(sizes, i, want, cols());
    const text = sizeNoteText(o, want, cols());
    if (text) offerUndo(text);
    return o.size;
  }

  function start(e: PointerEvent, i: number) {
    if (e.button !== 0) return;
    const id = ids()[i];
    if (!id) return;
    // rozměry karet v původním pořadí (cíl tahu = místo karty; náhled ho nemění → žádné přeskakování)
    const rects: Rect[] = items().map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.left, y: r.top, w: r.width, h: r.height };
    });
    const pageId = p.page.id;
    const x0 = e.clientX;
    const y0 = e.clientY;
    const scroll0 = box.scrollTop;
    let last = { x: x0, y: y0 };
    let raf = 0;
    const tabAt = (x: number, y: number) => {
      const tab = (document.elementFromPoint(x, y) as HTMLElement | null)?.closest<HTMLElement>("[data-tab]")?.dataset.tab ?? null;
      return tab && tab !== pageId ? tab : null;
    };
    const retarget = () => {
      const d = drag();
      if (!d) return;
      const tab = tabAt(last.x, last.y);
      const to = tab ? d.from : gridDropIndex(rects, i, last.x, last.y + (box.scrollTop - scroll0));
      if (tab !== dropTab()) setDropTab(tab);
      if (to !== d.to) flip(() => setDrag({ ...d, to, x: last.x, y: last.y }));
      else setDrag({ ...d, x: last.x, y: last.y });
    };
    // posun mřížky u horního / spodního okraje — smyčka běží jen během tahu
    const tick = () => {
      const r = box.getBoundingClientRect();
      const step = autoScrollStep(last.y, r.top, r.bottom);
      if (step) {
        box.scrollTop += step;
        retarget();
      }
      raf = requestAnimationFrame(tick);
    };
    const move = (ev: PointerEvent) => {
      last = { x: ev.clientX, y: ev.clientY };
      if (!drag()) {
        if (Math.hypot(ev.clientX - x0, ev.clientY - y0) < DRAG_START) return;
        setDrag({ id, from: i, to: i, x: ev.clientX, y: ev.clientY });
        raf = requestAnimationFrame(tick);
      }
      retarget();
    };
    const finish = (how: DragEnd) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      cancelAnimationFrame(raf);
      const d = drag();
      const a = dropAction(d, dropTab(), how);
      const apply = () =>
        batch(() => {
          if (a.kind === "move") change((l) => moveCard(l, pageId, a.from, a.to));
          setDrag(null);
          setDropTab(null);
        });
      // zrušeno po posunu náhledu: karty se plynule vrátí na svá místa; puštění = náhled už je výsledek
      if (a.kind === "none" && d && d.to !== d.from) flip(apply);
      else apply();
      if (a.kind === "page") toPage(id, a.tab);
    };
    const up = () => finish("up");
    const cancel = () => finish("cancel");
    // Esc zruší jen tah, ne celé úpravy (dřív Esc během tahu zavřel režim úprav)
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      ev.stopPropagation();
      finish("escape");
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
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

  /** tah za okraj karty: karta se mění živě (přichytí se na sloupce / řádky), pustit = uložit do draftu, Esc = zrušit */
  function resize(e: PointerEvent, id: CardId, edge: Edge) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const el = e.currentTarget as HTMLElement;
    const item = el.closest<HTMLElement>(".deck-item");
    if (!item) return;
    el.setPointerCapture(e.pointerId);
    // rozměry z okamžiku chycení — náhled kartu mění, výpočet z pevného bodu nekmitá
    const m = metrics(item);
    const startDim: Dim = { size: sizeOf(id), h: hOf(id) };
    const pageId = p.page.id;
    setRs({ id, edge, dim: startDim });
    const move = (ev: PointerEvent) => {
      const dim = resizeTo(m, edge, startDim, ev.clientX, ev.clientY);
      const cur = rs();
      if (cur && (cur.dim.size !== dim.size || cur.dim.h !== dim.h)) flip(() => setRs({ ...cur, dim }));
    };
    const key = (ev: KeyboardEvent) => ev.key === "Escape" && (ev.preventDefault(), ev.stopPropagation(), finish(false));
    const finish = (apply: boolean) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      const r = rs();
      if (!apply || !r || (r.dim.size === startDim.size && r.dim.h === startDim.h)) {
        if (r && apply === false) flip(() => setRs(null));
        else setRs(null);
        return;
      }
      const sizes = ids().map(sizeOf);
      const size = r.dim.size !== startDim.size ? note(sizes, ids().indexOf(id), r.dim.size) : startDim.size;
      batch(() => {
        change((l) => setHeight(setSize(l, pageId, id, size), pageId, id, r.dim.h));
        setRs(null);
      });
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
  }

  /** přesun na jinou záložku (tažením na její název) — s Vrátit; kde už karta je, nic se nezmění */
  function toPage(id: CardId, tab: string) {
    const from = p.page.id;
    const prev = p.layout;
    const target = pageName(pageOf(prev, tab));
    if (pageMoveBlock(prev, from, id, tab) === "there") {
      offerUndo(t("cards.layout.already", "„{name}“ už na záložce {tab} je — na jedné záložce smí být jen jednou, takže zůstává tady.", { name: cardName(id), tab: target }));
      return;
    }
    change((l) => moveToPage(l, from, id, tab));
    offerUndo(t("cards.layout.moved", "„{name}“ přesunuta na záložku {tab}", { name: cardName(id), tab: target }), () => change(() => prev));
  }

  function remove(id: CardId) {
    const r = removeCard(p.layout, p.page.id, id);
    if (!r.undo) return;
    const u = r.undo;
    flip(() => change(() => r.layout));
    offerUndo(t("cards.layout.removed", "Karta „{name}“ odebrána do katalogu", { name: cardName(id) }), () => change((l) => restoreCard(l, u)));
  }

  /** šířka klikem / klávesou: jen přes velikosti, které vypadají jinak; bez efektu = nápověda */
  function sizeTo(id: CardId, want: CardSize) {
    const list = ids();
    const size = note(list.map(sizeOf), list.indexOf(id), want);
    if (size !== sizeOf(id)) flip(() => change((l) => setSize(l, p.page.id, id, size)));
  }
  function heightTo(id: CardId, h: CardHeight | undefined) {
    if (h !== hOf(id)) flip(() => change((l) => setHeight(l, p.page.id, id, h)));
  }

  /** klávesy na úchytu: šipky přesun, + / − šířka, PageUp / PageDown výška (`drag.ts` `keyAction`) */
  const key = (e: KeyboardEvent, i: number) => {
    const a = keyAction(e);
    const id = ids()[i];
    if (!a || !id) return;
    e.preventDefault();
    e.stopPropagation();
    if ("move" in a) {
      const to = i + a.move;
      if (to < 0 || to >= p.page.cards.length) return;
      flip(() => change((l) => moveCard(l, p.page.id, i, to)));
    } else if ("size" in a) sizeTo(id, stepSize(sizeOf(id), a.size, cols()));
    else heightTo(id, stepHeight(hOf(id), a.height));
    // úchyt karty zpět do fokusu (DOM se po přesunu přeskládá)
    requestAnimationFrame(() => box.querySelector<HTMLElement>(`.deck-item[data-card="${CSS.escape(id)}"] .lay-grip`)?.focus());
  };

  const dimText = (id: CardId, d: Dim) => t("cards.layout.dim", "{w} × {h}", { w: spans().get(id) ?? 1, h: d.h ?? t("cards.layout.height.autoShort", "auto") });
  const dragName = () => {
    const d = drag();
    return d ? cardName(d.id) : "";
  };
  const dragBlocked = () => {
    const d = drag();
    const tab = dropTab();
    return !!d && !!tab && pageMoveBlock(p.layout, p.page.id, d.id, tab) === "there";
  };

  return (
    <div class="deck" classList={{ editing: p.editing, dragging: !!drag(), resizing: !!rs() }} ref={box} data-page={p.page.id}>
      <For each={ids()}>
        {(id, i) => (
          <div
            class="deck-item"
            data-size={sizeNow(id)}
            data-card={id}
            data-h={hNow(id)}
            data-cl-bg={cl(id).attrs["data-cl-bg"]}
            data-cl-head={cl(id).attrs["data-cl-head"]}
            data-cl-dens={cl(id).attrs["data-cl-dens"]}
            data-cl-accent={cl(id).attrs["data-cl-accent"]}
            style={{
              "--span": String(spans().get(id) ?? 1),
              "--h": hNow(id) === undefined ? undefined : String(hNow(id)),
              "--accent": cl(id).style["--accent"],
              order: String(view().indexOf(id)),
            }}
            classList={{ lifted: drag()?.id === id && !dropTab() }}
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
                  title={t("cards.layout.grip2", "Přetáhni na jiné místo nebo na záložku (Esc zruší). Klávesy: šipky přesun, + / − šířka, PageUp / PageDown výška")}
                  aria-label={t("cards.layout.grip.aria2", "Přesunout {name}: šipky, šířka + a −, výška PageUp a PageDown", { name: cardName(id) })}
                  onPointerDown={(e) => start(e, i())}
                  onKeyDown={(e) => key(e, i())}
                >⠿</button>
                <button
                  class="lay-size"
                  title={t("cards.layout.sizeTitle", "Velikost: {size} — klik změní", { size: sizeLabel(sizeOf(id)) })}
                  aria-label={t("cards.layout.sizeTitle", "Velikost: {size} — klik změní", { size: sizeLabel(sizeOf(id)) })}
                  onClick={() => sizeTo(id, nextSizeFor(sizeOf(id), cols()))}
                >⇔ <b>{sizeOf(id).toUpperCase()}</b></button>
                <button
                  class="lay-size"
                  title={t("cards.layout.heightTitle", "Výška: {h} — klik změní", { h: heightLabel(hOf(id)) })}
                  aria-label={t("cards.layout.heightTitle", "Výška: {h} — klik změní", { h: heightLabel(hOf(id)) })}
                  onClick={() => heightTo(id, nextHeight(hOf(id)))}
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
                onDblClick={() => heightTo(id, undefined)}
              />
              <div class="lay-rs lay-rs-se" aria-hidden="true" title={t("cards.layout.rs.se", "Táhni = šířka i výška")} onPointerDown={(e) => resize(e, id, "se")}>◢</div>
              <Show when={rs()?.id === id ? rs() : null}>
                {(r) => (
                  <div class="lay-target" aria-hidden="true">
                    <span class="lay-dim">{dimText(id, r().dim)}</span>
                  </div>
                )}
              </Show>
            </Show>
          </div>
        )}
      </For>
      <Show when={p.editing && editPage() === p.page.id}>
        <button class="lay-add" style={{ order: "9999" }} onClick={() => setCatalogOpen(true)}>{t("cards.layout.add", "+ Přidat kartu")}</button>
      </Show>
      <Show when={drag()}>
        {(d) => (
          <div class="lay-float" classList={{ blocked: dragBlocked() }} style={{ left: `${d().x}px`, top: `${d().y}px` }} aria-hidden="true">
            {dragName()}
            <Show when={dropTab()}>
              {(tab) => <span> → {pageName(pageOf(p.layout, tab()))}{dragBlocked() ? ` (${t("cards.layout.alreadyShort", "už tam je")})` : ""}</span>}
            </Show>
          </div>
        )}
      </Show>
    </div>
  );
}
