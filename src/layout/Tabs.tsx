// Záložky v hlavičce šuplíku (N záložek z `layout.pages`). Mimo úpravy: klik = `set_mode` (Rust drží,
// na které záložce šuplík je), klávesy 1–9 obsluhuje `App`. V režimu úprav: klik přepne rozpracovanou
// záložku, dvojklik = přejmenovat (Enter / Esc), × = smazat (karty do katalogu, s Vrátit), tažení =
// pořadí, „+“ = nová záložka (název + volitelná ikona z malé sady). Záložka je i cíl tažení karty
// (`data-tab`, zvýraznění z `dropTab`; prodleva nad ní záložku přepne — `cardDrag.ts`). Mimo úpravy se
// při najetí na záložky ukáže tužka „Upravit rozložení“.
import { createSignal, For, onMount, Show } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { beginEdit, change, draft, dropTab, editing, editPage, offerUndo, setEditPage } from "./edit";
import { addPage, MAX_PAGES, movePage, normalizeLayout, PAGE_ICONS, pageNameError, pageOf, removePage, renamePage, restorePage, WORK, PERSONAL, type LayoutPage } from "./model";
import { pageName } from "./text";

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
  const [drag, setDrag] = createSignal<{ from: number; to: number } | null>(null);
  let box!: HTMLDivElement;
  let moved = false;

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

  /** tažení záložky = pořadí (od posunu o pár px; jinak je to klik) */
  function start(e: PointerEvent, i: number) {
    if (!editing() || e.button !== 0 || renaming()) return;
    const el = e.currentTarget as HTMLElement;
    const tabs = Array.from(box.querySelectorAll<HTMLElement>(".lay-tab"));
    const centers = tabs.map((x) => {
      const r = x.getBoundingClientRect();
      return r.left + r.width / 2;
    });
    const x0 = e.clientX;
    moved = false;
    el.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      if (!moved && Math.abs(ev.clientX - x0) < 5) return;
      moved = true;
      let to = 0;
      centers.forEach((c, j) => Math.abs(ev.clientX - c) < Math.abs(ev.clientX - centers[to]) && (to = j));
      setDrag({ from: i, to });
    };
    const end = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      const d = drag();
      setDrag(null);
      if (d && d.to !== d.from) change((x) => movePage(x, d.from, d.to));
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  }

  return (
    <Show when={editing() || l().pages.length > 1}>
      <div class="seg lay-tabs" classList={{ editing: editing() }} role="group" aria-label={t("cards.head.tab", "Záložka")} ref={box}>
        <For each={l().pages}>
          {(pg, i) => (
            <span class="lay-tab" classList={{ drop: dropTab() === pg.id, "drag-to": !!drag() && drag()!.to === i() && drag()!.from !== i() }}>
              <Show when={renaming() === pg.id} fallback={
                <button
                  data-tab={pg.id}
                  classList={{ on: cur() === pg.id }}
                  aria-pressed={cur() === pg.id}
                  title={editing() ? t("cards.layout.tab.editTitle", "{name} — dvojklik přejmenuje, tažení změní pořadí", { name: pageName(pg) }) : title(pg, i())}
                  onPointerDown={(e) => start(e, i())}
                  onClick={() => !moved && go(pg.id)}
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
          <span class="lay-tab lay-tab-pencil">
            <button class="lay-tab-edit" title={t("cards.layout.pencil.title", "Upravit rozložení — karty pak přetáhneš kamkoli. Rychleji: podrž kartu za hlavičku a táhni.")} aria-label={t("cards.layout.pencil", "Upravit rozložení")} onClick={() => beginEdit(p.s)}>✎</button>
          </span>
        </Show>
        <Show when={editing() && l().pages.length < MAX_PAGES}>
          <span class="lay-tab">
            <button class="lay-tab-add" title={t("cards.layout.tab.add", "Nová záložka")} aria-label={t("cards.layout.tab.add", "Nová záložka")} aria-expanded={adding()} onClick={() => setAdding(!adding())}>+</button>
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
