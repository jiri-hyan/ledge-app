// Malé menu pravého kliku na dlaždici (projekt, akce, odkaz, složka): „Upravit vzhled…“, u upraveného
// „Obnovit vzhled“, u složky „Přejmenovat / Zrušit složku“. Jedno menu na šuplík (`<TileMenu />` v MainPage),
// pevná pozice u kurzoru přes Portal do `.shell` (karta s výškou ho neořízne). Vlna 9 Z: menu se změří a umístí
// přes `ui/place.ts` `placePopup` (u okraje se otočí / posune, vždy celé vidět i v úzkém okně); z ⋯ a z klávesnice
// (Shift+F10 / klávesa menu — událost bez souřadnic) se kotví k prvku, ne k bodu 0,0. Šipky / Home / End.
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { placePopup, pointBox, type Box } from "../ui/place";
import { Portal } from "solid-js/web";
import "./tilelook.css";

export type MenuItem = { text: string; run: () => void; danger?: boolean };
type Open = { anchor: Box; items: MenuItem[]; keyboard: boolean };
const [menu, setMenu] = createSignal<Open | null>(null);

/** Pravý klik na dlaždici → menu u kurzoru (výchozí kontextové menu WebView se nepoužije). */
export function openTileMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  if (!items.length) return;
  // klik na ⋯ / klávesnice (contextmenu bez souřadnic) → kotva = prvek; pravý klik myší → bod kurzoru
  const el = (e.currentTarget ?? e.target) as Element | null;
  const keyboard = e.type === "contextmenu" && e.clientX === 0 && e.clientY === 0;
  const r = el && (keyboard || e.type === "click") ? el.getBoundingClientRect() : null;
  const anchor = r ? { left: r.left, top: r.top, right: r.right, bottom: r.bottom } : pointBox(e.clientX, e.clientY);
  setMenu({ anchor, items, keyboard: keyboard || e.detail === 0 });
}
export const closeTileMenu = () => setMenu(null);

export default function TileMenu() {
  return <Show when={menu()} keyed>{(m) => <Menu m={m} />}</Show>;
}

/** Posluchače jen po dobu otevření menu (v klidu nic neběží). */
function Menu(p: { m: Open }) {
  let box: HTMLDivElement | undefined;
  const [pos, setPos] = createSignal<{ x: number; y: number; maxH: number } | null>(null);
  const buttons = () => [...(box?.querySelectorAll<HTMLButtonElement>("button") ?? [])];
  const nav = (e: KeyboardEvent) => {
    const all = buttons();
    const i = all.indexOf(document.activeElement as HTMLButtonElement);
    const go = (j: number) => (e.preventDefault(), all[(j + all.length) % all.length]?.focus());
    if (e.key === "ArrowDown") go(i + 1);
    else if (e.key === "ArrowUp") go(i < 0 ? all.length - 1 : i - 1);
    else if (e.key === "Home") go(0);
    else if (e.key === "End") go(all.length - 1);
  };
  onMount(() => {
    if (box) setPos(placePopup(p.m.anchor, { w: box.offsetWidth, h: box.scrollHeight }, { w: window.innerWidth, h: window.innerHeight }, { gap: 2 }));
    // z klávesnice fokus na první položku (myší ne — fokus nekrade)
    if (p.m.keyboard) queueMicrotask(() => buttons()[0]?.focus());
    const down = (e: PointerEvent) => !box?.contains(e.target as Node) && closeTileMenu();
    const key = (e: KeyboardEvent) => e.key === "Escape" && (e.stopPropagation(), closeTileMenu());
    document.addEventListener("pointerdown", down, true);
    document.addEventListener("scroll", closeTileMenu, true);
    window.addEventListener("keydown", key, true);
    onCleanup(() => {
      document.removeEventListener("pointerdown", down, true);
      document.removeEventListener("scroll", closeTileMenu, true);
      window.removeEventListener("keydown", key, true);
    });
  });
  return (
    <Portal mount={document.querySelector(".shell") ?? document.body}>
      <div class="tl-menu" role="menu" ref={box} onKeyDown={nav} style={{ left: `${pos()?.x ?? 0}px`, top: `${pos()?.y ?? 0}px`, "max-height": pos() ? `${pos()!.maxH}px` : undefined, visibility: pos() ? "visible" : "hidden" }}>
        <For each={p.m.items}>
          {(it) => (
            <button role="menuitem" classList={{ "tl-menu-danger": !!it.danger }} onClick={() => { closeTileMenu(); it.run(); }}>{it.text}</button>
          )}
        </For>
      </div>
    </Portal>
  );
}
