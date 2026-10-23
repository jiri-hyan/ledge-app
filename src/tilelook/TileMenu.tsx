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
/** typ události stisku na ⋯ (posluchače kliku mimo drží `PopMenu`, tady se jen pozná, odkud menu přišlo) */
const POINTER_DOWN = "pointer" + "down";

export function openTileMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  if (!items.length) return;
  // klik / stisk na ⋯ / klávesnice (contextmenu bez souřadnic) → kotva = prvek; pravý klik myší → bod kurzoru.
  // ⋯ zástupce otevírá už na pointerdown (dřív, než tah nebo klik dlaždice spustí aplikaci); druhý stisk zavře.
  const el = (e.currentTarget ?? e.target) as HTMLElement | null;
  const press = e.type === POINTER_DOWN;
  if (press && el && menu()?.trigger === el) return closeTileMenu();
  const keyboard = e.type === "contextmenu" && e.clientX === 0 && e.clientY === 0;
  const r = el && (keyboard || e.type === "click" || press) ? el.getBoundingClientRect() : null;
  const anchor = r ? { left: r.left, top: r.top, right: r.right, bottom: r.bottom } : pointBox(e.clientX, e.clientY);
  // PointerEvent má detail vždy 0 — klávesnice je jen klik bez počtu kliknutí (Enter / Mezerník)
  setMenu({ anchor, items, keyboard: keyboard || (e.type === "click" && e.detail === 0), trigger: r ? el : null });
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
