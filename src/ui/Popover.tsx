// Plovoucí panel nad celým šuplíkem (Portal do `.shell`, pevná pozice): kotví se k prvku / bodu, změří se
// a umístí přes `place.ts` `placePopup` — pod kotvu, u spodního okraje nad ni, u boku dovnitř okna, vyšší než
// okno se posouvá uvnitř. Karta s omezenou výškou ho neořízne. Přepočet jen na změnu velikosti obsahu / okna
// (ResizeObserver, `resize`) — v klidu nic neběží. Zavírání (klik mimo, Esc) řeší obsah, nebo `onDismiss`.
//
// `PopMenu` je JEDNO menu celé aplikace (⋯ karty, ⋯ poznámky, pravý klik na zástupce, výběr vzhledu přehrávače,
// ⋯ lístku): role `menu` / `menuitem*`, šipky ↑↓ Home End (`popnav.ts` `navTarget`), Esc zavře a vrátí fokus na
// spouštěč (jen když v menu fokus byl — Ledge fokus nekrade), klik mimo (pointerdown) a posun pod menem zavře,
// u okraje šuplíku se překlopí (`place.ts`). Položky `PopItem`, oddělovač `PopSep`, nadpis skupiny `PopCap`.
// Spouštěč drží `createMenu` (otevřít / zavřít, kotva, otevření z klávesnice).
import { createSignal, onCleanup, onMount, Show, type JSX } from "solid-js";
import { Portal } from "solid-js/web";
import { menuKey, navTarget } from "./popnav";
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

const ITEMS = "button:not(:disabled)";

/** Stav menu s jedním spouštěčem: kotva při otevření, otevřeno z klávesnice (Enter / Mezerník na tlačítku). */
export function createMenu() {
  const [at, setAt] = createSignal<{ anchor: Box; keyboard: boolean } | null>(null);
  let trigger: HTMLElement | undefined;
  return {
    at,
    open: () => at() !== null,
    trigger: () => trigger,
    close: () => setAt(null),
    /** `onClick` spouštěče: zavře otevřené, jinak otevře pod ním (kotva = obdélník spouštěče). */
    toggle: (e: MouseEvent) => {
      trigger = e.currentTarget as HTMLElement;
      if (at()) return setAt(null);
      const anchor = boxOf(trigger);
      if (anchor) setAt({ anchor, keyboard: e.detail === 0 });
    },
  };
}

/** Menu u kotvy. Vykresluje se jen po dobu otevření (`<Show>` u volajícího) — posluchače žijí jen tehdy. */
export function PopMenu(p: {
  anchor: Box;
  /** spouštěč: klik na něj menu nezavírá (přepíná ho sám) a Esc na něj po použití klávesnice vrátí fokus */
  trigger?: HTMLElement | null;
  onClose: () => void;
  label?: string;
  align?: "start" | "end";
  gap?: number;
  /** otevřeno z klávesnice → fokus rovnou na první položku (myší ne) */
  keyboard?: boolean;
  /** uvnitř okna (lístek na ploše): bez Portalu a pevné pozice, panel přes tělo okna */
  fill?: boolean;
  children: JSX.Element;
}) {
  let box!: HTMLDivElement;
  const [pos, setPos] = createSignal<Placed | null>(null);
  const items = () => [...box.querySelectorAll<HTMLButtonElement>(ITEMS)];
  const place = () => {
    if (p.fill || !box) return;
    const size = { w: box.offsetWidth, h: box.scrollHeight };
    setPos(placePopup(p.anchor, size, { w: window.innerWidth, h: window.innerHeight }, { align: p.align, gap: p.gap ?? 4 }));
  };
  onMount(() => {
    place();
    const ro = new ResizeObserver(place);
    ro.observe(box);
    // z klávesnice fokus na první položku (myší ne — fokus nekrade)
    if (p.keyboard) queueMicrotask(() => items()[0]?.focus());
    const down = (e: PointerEvent) => {
      const el = e.target as Node | null;
      if (el && !box.contains(el) && !p.trigger?.contains(el)) p.onClose();
    };
    const scrolled = (e: Event) => !box.contains(e.target as Node) && p.onClose();
    const key = (e: KeyboardEvent) => {
      const ae = document.activeElement;
      const inside = box.contains(ae);
      const typing = !!ae && ae.matches("input, textarea, select, [contenteditable='true']") && inside;
      const what = menuKey(e.key, { inside, free: !ae || ae === document.body || ae === p.trigger, typing });
      if (what === "close") {
        if (e.key === "Escape") e.stopPropagation();
        // fokus se vrací jen tomu, kdo ho do menu přinesl (klávesnice); myší otevřené menu fokus nemělo
        if (inside && e.key === "Escape") p.trigger?.focus({ preventScroll: true });
        p.onClose();
      } else if (what === "nav") {
        const all = items();
        const to = navTarget(e.key, all.indexOf(ae as HTMLButtonElement), all.length);
        if (to === null) return;
        e.preventDefault();
        all[to]?.focus();
      }
    };
    document.addEventListener("pointerdown", down, true);
    if (!p.fill) {
      document.addEventListener("scroll", scrolled, true);
      window.addEventListener("resize", place);
    }
    window.addEventListener("keydown", key, true);
    onCleanup(() => {
      ro.disconnect();
      document.removeEventListener("pointerdown", down, true);
      document.removeEventListener("scroll", scrolled, true);
      window.removeEventListener("resize", place);
      window.removeEventListener("keydown", key, true);
    });
  });
  const menu = (
    <div
      class="pop"
      classList={{ "pop-fill": !!p.fill }}
      role="menu"
      aria-label={p.label}
      ref={box}
      style={p.fill ? undefined : { left: `${pos()?.x ?? 0}px`, top: `${pos()?.y ?? 0}px`, "max-height": pos() ? `${pos()!.maxH}px` : undefined, visibility: pos() ? "visible" : "hidden" }}
    >
      {p.children}
    </div>
  );
  return (
    <Show when={!p.fill} fallback={menu}>
      <Portal mount={document.querySelector(".shell") ?? document.body}>{menu}</Portal>
    </Show>
  );
}

/** Položka menu. `checked` (i false) = zaškrtávací, `radio` = výběr jedné z voleb (`on` = vybraná). */
export function PopItem(p: {
  onClick: () => void;
  children: JSX.Element;
  danger?: boolean;
  disabled?: boolean;
  checked?: boolean;
  radio?: boolean;
  on?: boolean;
  /** `more` = tlumená odkazová položka uprostřed, `undo` = akcentní „Vrátit“ */
  tone?: "more" | "undo";
  title?: string;
}) {
  const role = () => (p.radio ? "menuitemradio" : p.checked !== undefined ? "menuitemcheckbox" : "menuitem");
  return (
    <button
      type="button"
      class="pop-item"
      classList={{ "pop-check": p.checked !== undefined, "pop-checked": !!p.checked, "pop-danger": !!p.danger, "pop-on": !!p.on, "pop-more": p.tone === "more", "pop-undo": p.tone === "undo" }}
      role={role()}
      aria-checked={p.checked !== undefined ? p.checked : p.radio ? !!p.on : undefined}
      disabled={p.disabled}
      title={p.title}
      onClick={() => p.onClick()}
    >
      {p.children}
    </button>
  );
}

/** Vodorovná čára mezi skupinami. */
export const PopSep = () => <span class="pop-sep" role="separator" />;
/** Nadpis skupiny položek. */
export const PopCap = (p: { children: JSX.Element }) => <span class="pop-cap" role="presentation">{p.children}</span>;
