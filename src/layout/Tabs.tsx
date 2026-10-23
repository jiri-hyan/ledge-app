// Záložky v hlavičce šuplíku (N záložek z `layout.pages`). Mimo úpravy: klik = `set_mode` (Rust drží,
// na které záložce šuplík je), klávesy 1–9 obsluhuje `App`. V režimu úprav: klik přepne rozpracovanou
// záložku, dvojklik = přejmenovat (Enter / Esc), × = smazat (karty do katalogu, s Vrátit), „+“ = nová záložka
// (název + volitelná ikona z malé sady). Pořadí záložek se mění tahem stejným motorem jako u karet
// (`tabDrag.ts`: duch, mezera, FLIP, Esc zruší; v úpravách tahem, mimo ně podržením ~0,4 s, které úpravy
// zapne); z klávesnice Alt + ← →. Záložka je i cíl tažení karty (`data-tab`, zvýraznění z `dropTab`; prodleva
// nad ní záložku přepne — `cardDrag.ts`); karta puštěná na „+“ (`data-newtab`) vytvoří „Novou záložku“ a ta se
// hned přejmenovává (`renameAsk`). Mimo úpravy se při najetí na záložky ukáže tužka „Upravit rozložení“.
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { beginEdit, change, draft, dropTab, editing, editPage, offerUndo, renameAsk, setEditPage, setRenameAsk } from "./edit";
import { addPage, MAX_PAGES, movePage, normalizeLayout, PAGE_ICONS, pageNameError, pageOf, removePage, renamePage, restorePage, WORK, PERSONAL, type LayoutPage } from "./model";
import { pageName } from "./text";
import { navTarget } from "../ui/popnav";
import { NEW_TAB } from "./dnd";
import { previewOrder } from "./drag";
import { flipEls } from "./flip";
import { createTabDrag } from "./tabDrag";
import "./dnd.css";

/** Pole pro název záložky: fokus po otevření (otevírá se jen výslovným klikem), Enter / Esc. */
function NameInput(p: { value: string; placeholder: string; onDone: (v: string | null) => void }) {
  let el!: HTMLInputElement;
  onMount(() => {
    el.focus();
    el.select();
  });
  return (
    <input
      ref={el}
      type="text"
      class="lay-tab-input"
      value={p.value}
      placeholder={p.placeholder}
      maxLength={32}
      aria-label={t("cards.layout.tab.name", "Název záložky")}
      onKeyDown={(e) => {
        if (e.key === "Enter") p.onDone(e.currentTarget.value);
        if (e.key === "Escape") {
          e.stopPropagation();
          p.onDone(null);
        }
      }}
      onBlur={(e) => p.onDone(e.currentTarget.value)}
    />
  );
}

export default function Tabs(p: { s: Snapshot }) {
  const l = () => draft() ?? normalizeLayout(p.s.layout);
  const cur = () => (editing() ? editPage() : pageOf(l(), p.s.mode).id);
  const [renaming, setRenaming] = createSignal<string | null>(null);
  const [adding, setAdding] = createSignal(false);
  const [icon, setIcon] = createSignal("");
  let box!: HTMLDivElement;
  const td = createTabDrag({
    box: () => box,
    editing,
    begin: () => beginEdit(p.s),
    busy: () => renaming() !== null,
    flip: (update) => flipEls(Array.from(box.querySelectorAll<HTMLElement>(":scope > .lay-tab[data-page]")), update),
    commit: (from, to) => movedTab(from, to),
  });
  const drag = td.drag;
  onCleanup(() => td.abort());
  /** pořadí záložek během tahu (náhled přes CSS `order`; DOM se nepřeskládá, takže fokus ani ukazatel nepřijdou o prvek) */
  const view = createMemo(() => {
    const ids = l().pages.map((x) => x.id);
    const d = drag();
    return d ? previewOrder(ids, d.from, d.to) : ids;
  });
  // karta puštěná na „+“ vytvořila záložku: její název se hned přepisuje
  createEffect(() => {
    const id = renameAsk();
    if (!id) return;
    setRenameAsk(null);
    if (l().pages.some((x) => x.id === id)) setRenaming(id);
  });

  const go = (id: string) => (editing() ? setEditPage(id) : void api.setMode(id));
  const title = (pg: LayoutPage, i: number) => `${pageName(pg)} (${i + 1})`;

  function rename(id: string, v: string | null) {
    setRenaming(null);
    if (v === null) return;
    const pg = l().pages.find((x) => x.id === id);
    const builtin = id === WORK || id === PERSONAL;
    const err = pageNameError(v, builtin);
    if (err || !pg) return;
    // vestavěná se svým výchozím názvem = bez vlastního (přeloží se podle jazyka)
    const plain = builtin && v.trim() === pageName({ ...pg, name: "" });
    change((x) => renamePage(x, id, plain ? "" : v));
  }

  function add(name: string | null) {
    if (name === null) return setAdding(false);
    const r = addPage(l(), name, icon());
    if (!r) return;
    change(() => r.layout);
    setEditPage(r.id);
    setAdding(false);
    setIcon("");
  }

  function remove(id: string) {
    const pg = l().pages.find((x) => x.id === id);
    const r = removePage(l(), id);
    if (!r.undo || !pg) return;
    const u = r.undo;
    change(() => r.layout);
    if (editPage() === id) setEditPage(r.layout.pages[0].id);
    offerUndo(t("cards.layout.tab.removed", "Záložka „{name}“ smazána — její karty jsou v katalogu", { name: pageName(pg) }), () => change((x) => restorePage(x, u)));
  }

  /** pořadí záložek se změnilo (tahem nebo Alt + šipkou) — s Vrátit */
  function movedTab(from: number, to: number) {
    const pg = l().pages[from];
    if (!pg) return;
    change((x) => movePage(x, from, to));
    offerUndo(t("cards.layout.tab.moved", "Záložka „{name}“ přesunuta", { name: pageName(pg) }), () => change((x) => movePage(x, to, from)));
  }
  /** Alt + ← → na záložce v úpravách: o místo vedle; fokus zůstane na záložce */
  function key(e: KeyboardEvent, id: string, i: number) {
    // bez Alt: šipky / Home / End přesouvají fokus mezi záložkami (výběr dělá Enter / mezerník — role tablist)
    if (!e.altKey && !e.ctrlKey && !e.metaKey) {
      const nav = ({ ArrowLeft: "ArrowUp", ArrowRight: "ArrowDown", Home: "Home", End: "End" } as Record<string, string>)[e.key];
      if (!nav) return;
      const tabs = Array.from(box.querySelectorAll<HTMLElement>(":scope > .lay-tab > button[data-tab]"));
      const to = navTarget(nav, tabs.findIndex((x) => x.dataset.tab === id), tabs.length);
      if (to === null) return;
      e.preventDefault();
      tabs[to]?.focus();
      return;
    }
    if (!editing() || !e.altKey || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
    const to = i + (e.key === "ArrowLeft" ? -1 : 1);
    if (to < 0 || to >= l().pages.length) return;
    e.preventDefault();
    e.stopPropagation();
    flipEls(Array.from(box.querySelectorAll<HTMLElement>(":scope > .lay-tab[data-page]")), () => movedTab(i, to));
    requestAnimationFrame(() => box.querySelector<HTMLElement>(`[data-tab="${CSS.escape(id)}"]`)?.focus());
  }

  return (
    <Show when={editing() || l().pages.length > 1}>
      <div class="seg lay-tabs" classList={{ editing: editing(), dragging: !!drag() }} role="tablist" aria-label={t("cards.head.tab", "Záložka")} ref={box}>
        <For each={l().pages}>
          {(pg, i) => (
            <span class="lay-tab" data-page={pg.id} style={{ order: String(view().indexOf(pg.id)) }} classList={{ drop: dropTab() === pg.id, "lp-gap": drag()?.id === pg.id }}>
              <Show when={renaming() === pg.id} fallback={
                <button
                  data-tab={pg.id}
                  classList={{ on: cur() === pg.id }}
                  role="tab"
                  aria-selected={cur() === pg.id}
                  title={editing() ? t("cards.layout.tab.editTitle", "{name} — dvojklik přejmenuje, tažení změní pořadí (Alt + ← →)", { name: pageName(pg) }) : title(pg, i())}
                  onPointerDown={(e) => td.down(e, pg.id, i())}
                  onClick={() => go(pg.id)}
                  onKeyDown={(e) => key(e, pg.id, i())}
                  onDblClick={() => editing() && setRenaming(pg.id)}
                >
                  <Show when={pg.icon}><span class="lay-tab-icon">{pg.icon}</span></Show>
                  {pageName(pg)}
                </button>
              }>
                <NameInput value={pg.name || pageName(pg)} placeholder={pageName(pg)} onDone={(v) => rename(pg.id, v)} />
              </Show>
              <Show when={editing() && l().pages.length > 1 && renaming() !== pg.id}>
                <button class="lay-tab-x" title={t("cards.layout.tab.remove", "Smazat záložku (karty se vrátí do katalogu)")} aria-label={t("cards.layout.tab.removeAria", "Smazat záložku {name}", { name: pageName(pg) })} onClick={() => remove(pg.id)}>×</button>
              </Show>
            </span>
          )}
        </For>
        {/* objevitelnost: při najetí na záložky se objeví tužka = rovnou úpravy (tažení karet, velikosti, záložky) */}
        <Show when={!editing()}>
          <span class="lay-tab lay-tab-pencil" style={{ order: "100" }}>
            <button class="lay-tab-edit" title={t("cards.layout.pencil.title", "Upravit rozložení — karty pak přetáhneš kamkoli. Rychleji: podrž kartu za hlavičku a táhni.")} aria-label={t("cards.layout.pencil", "Upravit rozložení")} onClick={() => beginEdit(p.s)}>✎</button>
          </span>
        </Show>
        <Show when={editing() && l().pages.length < MAX_PAGES}>
          <span class="lay-tab" style={{ order: "101" }} classList={{ "lp-newdrop": dropTab() === NEW_TAB }}>
            <button class="lay-tab-add" data-newtab="1" title={t("cards.layout.tab.add", "Nová záložka")} aria-label={t("cards.layout.tab.add", "Nová záložka")} aria-expanded={adding()} onClick={() => setAdding(!adding())}>+</button>
          </span>
        </Show>
        <Show when={adding()}>
          <div class="lay-newtab" role="dialog" aria-label={t("cards.layout.tab.add", "Nová záložka")}>
            <NameInput value="" placeholder={t("cards.layout.tab.placeholder", "Název, třeba Hry")} onDone={add} />
            <div class="lay-icons" role="group" aria-label={t("cards.layout.tab.icon", "Ikona")}>
              <For each={PAGE_ICONS}>
                {(ic) => (
                  <button classList={{ on: icon() === ic }} aria-pressed={icon() === ic} onPointerDown={(e) => e.preventDefault()} onClick={() => setIcon(ic)} title={ic ? ic : t("cards.layout.tab.noIcon", "Bez ikony")}>
                    {ic || "∅"}
                  </button>
                )}
              </For>
            </div>
          </div>
        </Show>
      </div>
    </Show>
  );
}
