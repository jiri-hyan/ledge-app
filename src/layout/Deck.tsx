// Mřížka karet jedné záložky: `.deck` (sloupce z CSS proměnné `--deck-cols` — určuje styl, vlna 6 F),
// každá karta v `.deck-item` s `data-size` (s|m|l|xl), `data-card` a volitelně `data-h` (výška v řádcích;
// obsah pak scrolluje uvnitř karty). Rozpětí počítá `size.ts` `packSpans` (řada bez prázdného místa —
// volné sloupce dostane největší karta řady), sloupce se čtou z `--deck-cols` při změně vzhledu
// (MutationObserver na `<html>`, žádné časovače).
// Režim úprav (vlna 9 — „rozložení, které vždy poslechne“; kolo 10 — přirozený drag & drop): klidný
// přerušovaný obrys (bez kývání). Kartu chytíš kdekoli na prázdném místě (nebo za ⠿), mimo úpravy ji zvedne
// podržení hlavičky (~400 ms, rovnou zapne úpravy). Gesto řídí `cardDrag.ts` (pointer události — HTML5
// drag&drop v okně Tauri nefunguje), logika je v `dnd.ts` / `drag.ts`. Živý náhled: poloprůhledný „duch“ pod
// kurzorem, na místě dopadu mezera velikosti karty (`.lifted`, CSS `order`), ostatní se plynule odsunou (FLIP
// přes Web Animations, `motionNow` + reduced-motion), u okraje se mřížka posouvá sama, dole zóna „Skrýt
// kartu“, ouško záložky = přesun na její konec, prodleva nad ouškem záložku přepne a tah pokračuje v její
// mřížce (karta se vloží na určené místo). Esc / ztráta ukazatele = zrušit (karta zůstane), puštění daleko mimo
// karty = nic (kde už karta je, se neztratí — nápověda). Po dopadu lišta „… · Vrátit“.
// Tah za pravý okraj = šířka, za spodní = výška, za roh = obojí: karta se mění ŽIVĚ, přichytí se na sloupce
// a řádky a ukazuje „2 × 3“; dvojklik na spodní okraj = výška podle obsahu. ⇔ / ↕ = totéž klikem (jen
// velikosti, které vypadají jinak). Klávesy na úchytu: šipky přesun, + / − šířka, PageUp / PageDown výška.
// Volba bez viditelného efektu se ohlásí v liště (`sizeOutcome` → `sizeNoteText`). × do katalogu (s Vrátit).
// Vzhled karty (vlna 8 P, `look.ts`): `data-cl-bg` / `data-cl-head` / `data-cl-dens` / `data-cl-accent` a
// `--accent` na `.deck-item` (živý náhled z panelu `CardLook.tsx`); karta bez vzhledu = žádný atribut.
import { batch, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import { t, tp } from "../i18n";
import { motionNow } from "../motion";
import { change, dropTab, editPage, offerUndo, setCatalogOpen } from "./edit";
import {
  moveCard, moveToPageAt, nextHeight, pageMoveBlock, pageOf, removeCard, restoreCard, setHeight, setSize,
  type CardHeight, type CardId, type CardSize, type Layout, type LayoutPage,
} from "./model";
import { deckCols, nextSizeFor, packSpans, resizeTo, sizeOutcome, stepHeight, stepSize, type DeckMetrics, type Dim, type Edge } from "./size";
import { keyAction, previewOrder, type DropAction } from "./drag";
import { flipDeltas, flipPlan, insertAt } from "./dnd";
import { createCardDrag, type Drag } from "./cardDrag";
import { cardName, pageName, sizeName, sizeNoteText } from "./text";
import { cardLookAttrs } from "./look";
import { lookFor } from "./CardLook";
import "./layout.css";

type Resize = { id: CardId; edge: Edge; dim: Dim };

const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

export const sizeLabel = (s: CardSize) => sizeName(s);
export const heightLabel = (h: CardHeight | undefined) =>
  h === undefined ? t("cards.layout.height.autoLong", "podle obsahu") : tp("cards.layout.height.rows", h, ["{n} řádek", "{n} řádky", "{n} řádků"]);

export default function Deck(p: { page: LayoutPage; layout: Layout; editing: boolean; shown: (id: CardId) => boolean; render: (id: CardId) => JSX.Element; begin?: () => void }) {
  let box!: HTMLDivElement;
  let zoneEl: HTMLDivElement | undefined;
  const [rs, setRs] = createSignal<Resize | null>(null);
  const [cols, setCols] = createSignal(4);
  // For podle id (řetězce), ne objektů: změna velikosti, klon rozložení ani nový snapshot kartu nepřestaví
  const ids = () => (p.editing ? p.page.cards : p.page.cards.filter((c) => p.shown(c.id))).map((c) => c.id);
  const cardOf = (id: CardId) => p.page.cards.find((c) => c.id === id);
  const blocked = (d: Drag, page: string) => pageMoveBlock(p.layout, d.home, d.id, page) === "there";
  const cd = createCardDrag({
    box: () => box,
    zone: () => zoneEl,
    editing: () => p.editing,
    page: () => p.page,
    ids,
    cardOf,
    begin: () => p.begin?.(),
    blocked,
    flip: (update) => flip(update),
    commit: (a, d) => commit(a, d),
  });
  const drag = cd.drag;
  const dragging = createMemo(() => !!drag());
  onCleanup(() => cd.abort());
  // tažená karta, která na zobrazené záložce ještě není (tah pokračuje v mřížce jiné záložky), si nese velikost sama
  const dragged = (id: CardId) => (!cardOf(id) && drag()?.id === id ? drag() : null);
  const sizeOf = (id: CardId) => cardOf(id)?.size ?? dragged(id)?.size ?? "m";
  const hOf = (id: CardId) => cardOf(id)?.h ?? dragged(id)?.h;
  // během tahu za okraj se karta mění živě (náhled), uloží se až po puštění
  const sizeNow = (id: CardId) => (rs()?.id === id ? rs()!.dim.size : sizeOf(id));
  const hNow = (id: CardId) => (rs()?.id === id ? rs()!.dim.h : hOf(id));
  const cl = (id: CardId) => cardLookAttrs(lookFor(p.page.id, id, cardOf(id)?.look));
  // náhled pořadí během tahu: DOM se nepřeskládá (CSS `order`), takže ukazatel ani fokus o úchyt nepřijdou;
  // nad ouškem záložky / zónou Skrýt se karty vrátí na původní místa (tažená je „v ruce“)
  const parked = () => !!dropTab() || cd.zoneOn();
  const view = createMemo(() => {
    const d = drag();
    const base = ids();
    if (!d || parked()) return base;
    if (d.home === p.page.id) return previewOrder(base, d.from, d.to);
    return blocked(d, p.page.id) ? base : insertAt(base, d.to, d.id);
  });
  /** tažená karta patřící jiné záložce, než kterou teď vidíš: v mřížce jí patří zástupná mezera */
  const slotOn = createMemo(() => {
    const d = drag();
    return !!d && !parked() && d.home !== p.page.id && !blocked(d, p.page.id);
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

  const items = () => Array.from(box.querySelectorAll<HTMLElement>(":scope > .deck-item:not(.lay-fly)"));
  /** změna pořadí / velikosti s krátkým posunem ostatních karet (FLIP transformem; bez kývání; „bez animací“
   *  a systémové omezení pohybu = hned). Rozběhlý posun se zruší a nový začne z místa, kde karta právě je. */
  function flip(update: () => void) {
    const m = motionNow();
    const plan = flipPlan(m.style, m.speed, reducedMotion());
    if (!plan) return update();
    const els = items();
    const at = (el: HTMLElement) => {
      const r = el.getBoundingClientRect();
      return { x: r.left, y: r.top };
    };
    const first = els.map(at);
    for (const el of els) el.getAnimations?.().forEach((a) => a.id === "lay-flip" && a.cancel());
    update();
    flipDeltas(first, els.map(at)).forEach((d, k) => {
      if (!d || !els[k].isConnected) return;
      const a = els[k].animate?.([{ transform: `translate(${d.dx}px, ${d.dy}px)` }, { transform: "none" }], { duration: plan.ms, easing: plan.easing });
      if (a) a.id = "lay-flip";
    });
  }

  /** nápověda v liště úprav, když volba velikosti nemá viditelný efekt */
  function note(sizes: CardSize[], i: number, want: CardSize) {
    const o = sizeOutcome(sizes, i, want, cols());
    const text = sizeNoteText(o, want, cols());
    if (text) offerUndo(text);
    return o.size;
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

  /** přesun na konec jiné záložky (puštění na její ouško) — s Vrátit; kde už karta je, nic se nezmění */
  function toPage(id: CardId, from: string, tab: string, index = Number.MAX_SAFE_INTEGER) {
    const prev = p.layout;
    const target = pageName(pageOf(prev, tab));
    if (pageMoveBlock(prev, from, id, tab) === "there") {
      offerUndo(t("cards.layout.already", "„{name}“ už na záložce {tab} je — na jedné záložce smí být jen jednou, takže zůstává tady.", { name: cardName(id), tab: target }));
      return;
    }
    change((l) => moveToPageAt(l, from, id, tab, index));
    offerUndo(t("cards.layout.moved", "„{name}“ přesunuta na záložku {tab}", { name: cardName(id), tab: target }), () => change(() => prev));
  }

  /** konec tahu: co se stane (`drag.ts` `dropAction`) a krátká lišta „… · Vrátit“ */
  function commit(a: DropAction, d: Drag) {
    switch (a.kind) {
      case "move":
        change((l) => moveCard(l, d.home, a.from, a.to));
        offerUndo(t("cards.layout.dragMoved", "Karta „{name}“ přesunuta", { name: cardName(d.id) }), () => change((l) => moveCard(l, d.home, a.to, a.from)));
        break;
      case "page":
        toPage(d.id, d.home, a.tab);
        break;
      case "insert":
        toPage(d.id, d.home, a.page, a.index);
        break;
      case "blocked":
        offerUndo(t("cards.layout.already", "„{name}“ už na záložce {tab} je — na jedné záložce smí být jen jednou, takže zůstává tady.", { name: cardName(d.id), tab: pageName(pageOf(p.layout, a.page)) }));
        break;
      case "hide":
        remove(d.id, d.home);
        break;
    }
  }

  function remove(id: CardId, pageId = p.page.id) {
    const r = removeCard(p.layout, pageId, id);
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
  /** štítek u kurzoru: kam se karta pustí, když to není obyčejné místo v mřížce */
  const flyLabel = () => {
    const d = drag();
    if (!d) return null;
    if (cd.zoneOn()) return { text: t("cards.layout.zone", "Skrýt kartu"), blocked: false };
    const tab = dropTab();
    const there = (page: string) => blocked(d, page);
    if (tab) return { text: `→ ${pageName(pageOf(p.layout, tab))}${there(tab) ? ` (${t("cards.layout.alreadyShort", "už tam je")})` : ""}`, blocked: there(tab) };
    if (d.home !== p.page.id && there(p.page.id)) return { text: t("cards.layout.alreadyHere", "už tu je — zůstane na původní záložce"), blocked: true };
    return null;
  };
  const slotStyle = () => {
    const d = drag();
    if (!d) return {};
    return {
      "--span": String(spans().get(d.id) ?? 1),
      "--h": d.h === undefined ? undefined : String(d.h),
      "min-height": d.h === undefined ? `${d.px}px` : undefined,
      order: String(view().indexOf(d.id)),
    };
  };

  return (
    <div class="deck" classList={{ editing: p.editing, dragging: dragging(), resizing: !!rs() }} ref={box} data-page={p.page.id}>
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
            classList={{ lifted: drag()?.id === id && drag()?.home === p.page.id }}
            onPointerDown={(e) => cd.down(e, id)}
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
                  title={t("cards.layout.grip3", "Chyť kartu (i kdekoli na ní) a přetáhni ji na jiné místo, na záložku nebo dolů na „Skrýt kartu“ (Esc zruší). Klávesy: šipky přesun, + / − šířka, PageUp / PageDown výška")}
                  aria-label={t("cards.layout.grip.aria2", "Přesunout {name}: šipky, šířka + a −, výška PageUp a PageDown", { name: cardName(id) })}
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
      {/* tah pokračuje na jiné záložce: mezera velikosti karty na místě, kam by dopadla */}
      <Show when={slotOn()}>
        <div class="deck-item lifted lay-slot" data-size={drag()?.size} data-card={drag()?.id} data-h={drag()?.h} style={slotStyle()} aria-hidden="true" />
      </Show>
      <Show when={p.editing && editPage() === p.page.id}>
        <button class="lay-add" style={{ order: "9999" }} onClick={() => setCatalogOpen(true)}>{t("cards.layout.add", "+ Přidat kartu")}</button>
      </Show>
      <Show when={dragging()}>
        <div class="lay-zone" classList={{ on: cd.zoneOn() }} ref={zoneEl} aria-hidden="true">
          <b>×</b> {t("cards.layout.zone", "Skrýt kartu")} <small>{t("cards.layout.zone.hint", "do katalogu · jde vrátit")}</small>
        </div>
      </Show>
      <Show when={flyLabel()}>
        {(l) => (
          <div class="lay-float" classList={{ blocked: l().blocked }} style={{ left: `${cd.ptr().x}px`, top: `${cd.ptr().y}px` }} aria-hidden="true">
            {l().text}
          </div>
        )}
      </Show>
    </div>
  );
}
