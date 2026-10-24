// Sbalovací skupina voleb (kolo 21): karta s lidským nadpisem a jednou větou, co v ní je — pro dlouhé
// stránky (Projekty a akce, Obsah karet, Poznámky a lístky). NENÍ to „Pokročilé“: v režimu Základní je
// vidět vždy, jen může být sbalená. Nejčastější skupiny jsou výchozí otevřené (`open`), méně časté sbalené.
// Přepnutí si pamatuje úložiště prohlížeče po `gid` (`advModel.ts` `GROUP_KEY`, try/catch — bez něj platí
// výchozí stavy); skok z hledání nebo odjinud na volbu uvnitř (`hit`, i kotva skupiny `anchor`) skupinu
// rozbalí, do paměti to nezapíše. Obsah zůstává připojený i sbalený (`hidden`) — stav editorů se neztratí.
import { createEffect, createSignal, on, Show, type JSX } from "solid-js";
import { hit } from "../ctx";
import { GROUP_KEY, groupOpen, parseGroups, withGroup } from "../advModel";

const read = () => {
  try {
    return parseGroups(localStorage.getItem(GROUP_KEY));
  } catch {
    return {};
  }
};
const [stored, setStored] = createSignal<Record<string, boolean>>(read());

function remember(id: string, open: boolean, def: boolean) {
  const next = withGroup(stored(), id, open, def);
  setStored(next);
  try {
    localStorage.setItem(GROUP_KEY, JSON.stringify(next));
  } catch {
    // paměť je jen pohodlí
  }
}

/** `gid` = klíč paměti, `anchor` = kotva hledání `f-<anchor>` celé skupiny, `open` = výchozí stav,
 *  `badge` = krátký údaj vedle nadpisu (počet), `aside` = tlačítko v hlavičce (nesbalí skupinu). */
export function Group(p: { gid: string; title: string; desc?: string; anchor?: string; open?: boolean; badge?: string; aside?: JSX.Element; children: JSX.Element }) {
  let box!: HTMLElement;
  const def = () => p.open ?? true;
  const [forced, setForced] = createSignal(false);
  const open = () => forced() || groupOpen(stored(), p.gid, def());
  createEffect(
    on(hit, (h) => {
      const id = (h ?? "").replace(/[^\w.-]/g, "");
      if (id && (id === p.anchor || box?.querySelector(`[id="f-${id}"]`))) setForced(true);
    }),
  );
  const toggle = () => {
    const now = !open();
    setForced(false);
    remember(p.gid, now, def());
  };
  const bodyId = `grp-${p.gid.replace(/[^\w-]/g, "-")}`;
  return (
    <section ref={box} class="st-grp" classList={{ shut: !open(), hit: !!p.anchor && hit() === p.anchor }} id={p.anchor ? `f-${p.anchor}` : undefined}>
      <div class="st-grp-head">
        <button class="st-grp-tog" aria-expanded={open()} aria-controls={bodyId} onClick={toggle}>
          <span class="st-grp-arrow" aria-hidden="true">{open() ? "▾" : "▸"}</span>
          <span class="st-grp-txt">
            <b>{p.title}<Show when={p.badge}><em class="st-grp-badge">{p.badge}</em></Show></b>
            <Show when={p.desc}><small>{p.desc}</small></Show>
          </span>
        </button>
        <Show when={p.aside}><span class="st-grp-aside">{p.aside}</span></Show>
      </div>
      <div class="st-grp-body" id={bodyId} hidden={!open()}>{p.children}</div>
    </section>
  );
}
