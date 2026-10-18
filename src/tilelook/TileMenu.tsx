// Malé menu pravého kliku na dlaždici (projekt, akce, odkaz, složka): „Upravit vzhled…“, u upraveného
// „Obnovit vzhled“, u složky „Přejmenovat / Zrušit složku“. Jedno menu na šuplík (`<TileMenu />` v MainPage),
// pevná pozice u kurzoru přes Portal do `.shell` (karta s výškou ho neořízne).
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import "./tilelook.css";

export type MenuItem = { text: string; run: () => void };
const [menu, setMenu] = createSignal<{ x: number; y: number; items: MenuItem[] } | null>(null);

/** Pravý klik na dlaždici → menu u kurzoru (výchozí kontextové menu WebView se nepoužije). */
export function openTileMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  if (!items.length) return;
  // u pravého / spodního okraje okna dovnitř
  const x = Math.min(e.clientX, window.innerWidth - 180);
  const y = Math.min(e.clientY, window.innerHeight - 24 - items.length * 30);
  setMenu({ x: Math.max(4, x), y: Math.max(4, y), items });
}
export const closeTileMenu = () => setMenu(null);

export default function TileMenu() {
  return <Show when={menu()} keyed>{(m) => <Menu m={m} />}</Show>;
}

/** Posluchače jen po dobu otevření menu (v klidu nic neběží). */
function Menu(p: { m: { x: number; y: number; items: MenuItem[] } }) {
  let box: HTMLDivElement | undefined;
  onMount(() => {
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
      <div class="tl-menu" role="menu" ref={box} style={{ left: `${p.m.x}px`, top: `${p.m.y}px` }}>
        <For each={p.m.items}>
          {(it) => (
            <button role="menuitem" onClick={() => { closeTileMenu(); it.run(); }}>{it.text}</button>
          )}
        </For>
      </div>
    </Portal>
  );
}
