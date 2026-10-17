// Seznam s řazením přetažením za úchyt (pointer události — HTML5 drag&drop v okně Tauri chytá
// systémové přetahování souborů) a z klávesnice (úchyt ve fokusu + Alt+↑/↓). Cílový index počítá
// čistá `model.dropIndex`; ostatní řádky ustupují posunem, při reduced-motion bez animace.
import { createSignal, For, type JSX } from "solid-js";
import { t } from "../../i18n";
import { dropIndex } from "../model";

export function SortList<T>(p: { items: T[]; onMove: (from: number, to: number) => void; label: string; children: (item: T, index: () => number, handle: () => JSX.Element) => JSX.Element; class?: string }) {
  let box!: HTMLDivElement;
  const [drag, setDrag] = createSignal<{ from: number; to: number; dy: number; heights: number[] } | null>(null);

  function start(e: PointerEvent, i: number) {
    if (e.button !== 0) return;
    e.preventDefault();
    const rows = Array.from(box.children) as HTMLElement[];
    const gap = rows.length > 1 ? rows[1].getBoundingClientRect().top - rows[0].getBoundingClientRect().bottom : 0;
    const heights = rows.map((r) => r.getBoundingClientRect().height + gap);
    const y0 = e.clientY;
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    setDrag({ from: i, to: i, dy: 0, heights });
    const move = (ev: PointerEvent) => {
      const dy = ev.clientY - y0;
      setDrag({ from: i, to: dropIndex(heights, i, dy), dy, heights });
    };
    const end = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      const d = drag();
      setDrag(null);
      if (d && d.to !== d.from) p.onMove(d.from, d.to);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  }

  /** posun řádku `i` během tažení */
  const shift = (i: number): string | undefined => {
    const d = drag();
    if (!d) return undefined;
    if (i === d.from) return `translateY(${d.dy}px)`;
    const h = d.heights[d.from];
    if (d.from < d.to && i > d.from && i <= d.to) return `translateY(${-h}px)`;
    if (d.from > d.to && i < d.from && i >= d.to) return `translateY(${h}px)`;
    return "translateY(0)";
  };

  const key = (e: KeyboardEvent, i: number) => {
    if (!e.altKey || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    e.preventDefault();
    const to = e.key === "ArrowUp" ? i - 1 : i + 1;
    if (to < 0 || to >= p.items.length) return;
    p.onMove(i, to);
    // úchyt přesunutého řádku zpět do fokusu
    requestAnimationFrame(() => (box.children[to]?.querySelector(".st-grip") as HTMLElement | null)?.focus());
  };

  return (
    <div class={`st-sort ${p.class ?? ""}`} classList={{ dragging: !!drag() }} ref={box} role="list" aria-label={p.label}>
      <For each={p.items}>
        {(item, i) => (
          <div class="st-sort-item" role="listitem" classList={{ lifted: drag()?.from === i() }} style={{ transform: shift(i()) }}>
            {p.children(item, i, () => (
              <button
                class="st-grip"
                title={t("settings.ui.grip", "Přetáhni pro změnu pořadí (nebo Alt+↑/↓)")}
                aria-label={t("settings.ui.grip.aria", "Změnit pořadí: Alt+šipka nahoru nebo dolů")}
                onPointerDown={(e) => start(e, i())}
                onKeyDown={(e) => key(e, i())}
              >
                <svg viewBox="0 0 24 24"><circle cx="9" cy="6" r="1.4" /><circle cx="15" cy="6" r="1.4" /><circle cx="9" cy="12" r="1.4" /><circle cx="15" cy="12" r="1.4" /><circle cx="9" cy="18" r="1.4" /><circle cx="15" cy="18" r="1.4" /></svg>
              </button>
            ))}
          </div>
        )}
      </For>
    </div>
  );
}
