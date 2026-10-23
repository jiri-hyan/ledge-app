// Malé menu pravého kliku na dlaždici (projekt, akce, odkaz, složka): „Upravit vzhled…“, u upraveného
// „Obnovit vzhled“, u složky „Přejmenovat / Zrušit složku“. Jedno menu na šuplík (`<TileMenu />` v MainPage),
// společné menu `ui/Popover.tsx` `PopMenu` (pevná pozice u kurzoru, u okraje se otočí / posune, šipky / Home / End / Esc);
// z ⋯ a z klávesnice (Shift+F10 / klávesa menu — událost bez souřadnic) se kotví k prvku, ne k bodu 0,0.
import { createSignal, For, Show } from "solid-js";
import { pointBox, type Box } from "../ui/place";
import { PopItem, PopMenu } from "../ui/Popover";

export type MenuItem = { text: string; run: () => void; danger?: boolean };
type Open = { anchor: Box; items: MenuItem[]; keyboard: boolean; trigger: HTMLElement | null };
const [menu, setMenu] = createSignal<Open | null>(null);

/** Pravý klik na dlaždici → menu u kurzoru (výchozí kontextové menu WebView se nepoužije). */
export function openTileMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  if (!items.length) return;
  // klik na ⋯ / klávesnice (contextmenu bez souřadnic) → kotva = prvek; pravý klik myší → bod kurzoru
  const el = (e.currentTarget ?? e.target) as HTMLElement | null;
  const keyboard = e.type === "contextmenu" && e.clientX === 0 && e.clientY === 0;
  const r = el && (keyboard || e.type === "click") ? el.getBoundingClientRect() : null;
  const anchor = r ? { left: r.left, top: r.top, right: r.right, bottom: r.bottom } : pointBox(e.clientX, e.clientY);
  setMenu({ anchor, items, keyboard: keyboard || e.detail === 0, trigger: r ? el : null });
}
export const closeTileMenu = () => setMenu(null);

export default function TileMenu() {
  return (
    <Show when={menu()} keyed>
      {(m) => (
        <PopMenu anchor={m.anchor} keyboard={m.keyboard} trigger={m.trigger} onClose={closeTileMenu} gap={2}>
          <For each={m.items}>
            {(it) => <PopItem danger={it.danger} onClick={() => { closeTileMenu(); it.run(); }}>{it.text}</PopItem>}
          </For>
        </PopMenu>
      )}
    </Show>
  );
}
