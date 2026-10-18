// Plovoucí panel nad celým šuplíkem (Portal do `.shell`, pevná pozice): kotví se k prvku / bodu, změří se
// a umístí přes `place.ts` `placePopup` — pod kotvu, u spodního okraje nad ni, u boku dovnitř okna, vyšší než
// okno se posouvá uvnitř. Karta s omezenou výškou ho neořízne. Přepočet jen na změnu velikosti obsahu / okna
// (ResizeObserver, `resize`) — v klidu nic neběží. Zavírání (klik mimo, Esc) řeší obsah, nebo `onDismiss`.
import { createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { Portal } from "solid-js/web";
import { placePopup, type Box, type Placed } from "./place";
import "./popover.css";

export default function Popover(p: {
  /** kotva (obdélník prvku v okně); null = střed okna */
  anchor: () => Box | null;
  align?: "start" | "end";
  label: string;
  /** volitelné zavírání klikem mimo / Esc (výběr `ignore` = prvky, na které klik mimo neplatí) */
  onDismiss?: () => void;
  ignore?: string;
  /** náhled po najetí: neklikatelný, bez fokusu */
  passive?: boolean;
  children: JSX.Element;
}) {
  let box!: HTMLDivElement;
  const [pos, setPos] = createSignal<Placed | null>(null);
  const place = () => {
    if (!box) return;
    const w = window.innerWidth;
    const h = window.innerHeight;
    const a = p.anchor() ?? { left: w / 2, right: w / 2, top: h / 3, bottom: h / 3 };
    setPos(placePopup(a, { w: box.offsetWidth, h: box.scrollHeight }, { w, h }, { align: p.align }));
  };
  onMount(() => {
    place();
    const ro = new ResizeObserver(() => place());
    ro.observe(box);
    window.addEventListener("resize", place);
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      if (!el || box.contains(el) || (p.ignore && el.closest(p.ignore))) return;
      p.onDismiss?.();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && (e.stopPropagation(), p.onDismiss?.());
    if (p.onDismiss) {
      document.addEventListener("pointerdown", down, true);
      window.addEventListener("keydown", key, true);
    }
    onCleanup(() => {
      ro.disconnect();
      window.removeEventListener("resize", place);
      document.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key, true);
    });
  });
  return (
    <Portal mount={document.querySelector(".shell") ?? document.body}>
      <div
        class="ui-pop"
        classList={{ "ui-pop-passive": !!p.passive }}
        ref={box}
        role={p.passive ? "tooltip" : "dialog"}
        aria-label={p.label}
        style={{ left: `${pos()?.x ?? 0}px`, top: `${pos()?.y ?? 0}px`, "max-height": pos() ? `${pos()!.maxH}px` : undefined, visibility: pos() ? "visible" : "hidden" }}
      >
        {p.children}
      </div>
    </Portal>
  );
}

/** Obdélník prvku v okně jako kotva (nebo null, když prvek není vidět). */
export function boxOf(el: Element | null | undefined): Box | null {
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return r.width || r.height ? { left: r.left, top: r.top, right: r.right, bottom: r.bottom } : null;
}
