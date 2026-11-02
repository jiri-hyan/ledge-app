// Řada štítků v hlavičce (kolo 27): VŽDY jedna řada, žádné zalamování, nic nevyleze z kontejneru (`min-width: 0`,
// `overflow: hidden`), takže štítky nikdy nepřekryjí záložky. Styl „Auto“ (výchozí) zhušťuje postupně: oddělené pilulky →
// sloučená pilulka → sloučená se zkrácenými texty → tečky s čísly → ikonky; až když se nevejdou ani ikonky, schová
// nejméně důležité štítky pod „+N“ (Claude čeká > pád > pracuje > nedokončené > neuloženo > běží; `data-prio`).
// Ruční styl se chová jako dřív (sloučená se zkracuje místo schování). Stupeň je čistá funkce místa a obsahu
// (`headModel.ts` `pickLevel` / `hideByPriority`), proto mezi styly nic nekmitá. Měří se jen ResizeObserverem (šířka řady)
// a MutationObserverem (obsah) sloučeně do jednoho snímku; žádný časovač.
import { createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { t } from "../i18n";
import type { ChipStyle } from "../types.ts";
import { chipLevels, CHIP_PRIO, hideByPriority, pickLevel } from "../layout/headModel";

export default function ChipsRow(p: { style: ChipStyle; children: JSX.Element }) {
  let wrap!: HTMLDivElement;
  let row!: HTMLDivElement;
  let more!: HTMLButtonElement;
  const [hidden, setHidden] = createSignal(0);
  const [open, setOpen] = createSignal(false);
  let raf = 0;
  const kids = () => Array.from(row.children) as HTMLElement[];
  const gapOf = () => parseFloat(getComputedStyle(row).columnGap) || 0;
  const measure = () => {
    raf = 0;
    const ks = kids();
    ks.forEach((k) => k.classList.remove("hl-hide"));
    const levels = chipLevels(p.style);
    const apply = (l: { style: string; shrink: number }) => {
      row.dataset.style = l.style;
      row.dataset.shrink = String(l.shrink);
    };
    if (open() || !ks.length) {
      apply(levels[0]);
      return setHidden(0);
    }
    const avail = wrap.clientWidth;
    if (!avail) return;
    const { level, fits } = pickLevel(levels, (l) => {
      apply(l);
      return { widths: ks.map((k) => k.offsetWidth), gap: gapOf() };
    }, avail);
    apply(level);
    if (fits) return setHidden(0);
    more.style.display = "inline-flex";
    const moreW = more.offsetWidth;
    more.style.display = "";
    const prios = ks.map((k) => Number(k.dataset.prio ?? CHIP_PRIO.other));
    const hide = hideByPriority(ks.map((k) => k.offsetWidth), prios, gapOf(), avail, moreW);
    ks.forEach((k, i) => k.classList.toggle("hl-hide", hide.has(i)));
    setHidden(hide.size);
  };
  const schedule = () => {
    if (!raf) raf = requestAnimationFrame(measure);
  };
  const toggle = (v: boolean) => {
    setOpen(v);
    measure();
  };
  createEffect(() => {
    p.style;
    if (row) schedule();
  });
  onMount(() => {
    const ro = new ResizeObserver(schedule);
    ro.observe(wrap);
    const mo = new MutationObserver(schedule);
    mo.observe(row, { childList: true, subtree: true, characterData: true });
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      if (open() && el && !wrap.contains(el) && !el.closest(".pz-drop")) toggle(false);
    };
    const key = (e: KeyboardEvent) => open() && e.key === "Escape" && (e.stopPropagation(), toggle(false));
    document.addEventListener("pointerdown", down, true);
    window.addEventListener("keydown", key, true);
    schedule();
    onCleanup(() => {
      ro.disconnect();
      mo.disconnect();
      cancelAnimationFrame(raf);
      document.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key, true);
    });
  });
  return (
    <div class="hl-chips-wrap" ref={wrap} classList={{ open: open() }}>
      <div class="chips hl-chips" ref={row} data-style={chipLevels(p.style)[0].style}>{p.children}</div>
      <button
        ref={more}
        class="chip dim hl-more"
        classList={{ "hl-more-on": hidden() > 0 || open() }}
        aria-expanded={open()}
        title={open() ? t("cards.head.moreClose", "Sbalit štítky") : t("cards.head.more", "Další štítky")}
        onClick={() => toggle(!open())}
      >
        {open() ? "▴" : `+${hidden()}`}
      </button>
    </div>
  );
}
