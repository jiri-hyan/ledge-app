// ⋯ menu poznámky v editoru: skupina „Export“ (Markdown / HTML do souboru, kopie do schránky, Otevřít v … registrovaných aplikacích).
// Seznam se kreslí přes `Portal` s pevnou pozicí u tlačítka jako `RowMenu` (editor s omezenou výškou ho
// neořízne), zavírá se klikem mimo, posunem i Esc — Esc nedojde k editoru (nezahodí rozepsané).

import { createSignal, For, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { t } from "../i18n";
import { placePopup } from "../ui/place";
import { IconMore } from "../pin/icons";
import { exportItems } from "./exportActions";

type Pos = { x: number; y: number; maxH: number };

export default function NoteMenu(p: { path: string; body: () => string }) {
  const [pos, setPos] = createSignal<Pos | null>(null);
  let btn: HTMLButtonElement | undefined;
  let list: HTMLSpanElement | undefined;
  const outside = (e: PointerEvent) => {
    const node = e.target as Node;
    if (!btn?.contains(node) && !list?.contains(node)) close();
  };
  const scrolled = (e: Event) => {
    if (!list?.contains(e.target as Node)) close();
  };
  const esc = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  };
  const close = () => {
    setPos(null);
    document.removeEventListener("pointerdown", outside, true);
    document.removeEventListener("scroll", scrolled, true);
    document.removeEventListener("keydown", esc, true);
  };
  const place = () => {
    if (!btn || !list) return;
    const r = btn.getBoundingClientRect();
    const at = placePopup({ left: r.left, top: r.top, right: r.right, bottom: r.bottom }, { w: list.offsetWidth, h: list.scrollHeight }, { w: window.innerWidth, h: window.innerHeight }, { align: "end" });
    setPos({ x: at.x, y: at.y, maxH: at.maxH });
  };
  const toggle = () => {
    if (pos() || !btn) return close();
    const r = btn.getBoundingClientRect();
    setPos({ x: Math.max(0, r.right - 200), y: r.bottom + 4, maxH: window.innerHeight });
    queueMicrotask(place);
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("scroll", scrolled, true);
    document.addEventListener("keydown", esc, true);
  };
  onCleanup(close);
  return (
    <>
      <button class="icon-btn" classList={{ on: !!pos() }} ref={btn} aria-haspopup="menu" aria-expanded={!!pos()} title={t("notes.export.more", "Export a další volby")} aria-label={t("notes.export.more", "Export a další volby")} onClick={toggle}>
        <IconMore />
      </button>
      <Show when={pos()}>
        {(at) => (
          <Portal mount={btn?.closest(".shell") ?? document.body}>
            <span class="card-menu-list note-act-list" role="menu" ref={list} style={{ position: "fixed", left: `${at().x}px`, top: `${at().y}px`, "max-height": `${at().maxH}px`, "overflow-y": "auto" }}>
              <span class="card-menu-cap">{t("notes.export.title", "Export")}</span>
              <For each={exportItems(p.path, p.body)}>
                {(it) => (
                  <button role="menuitem" onClick={() => { close(); it.run(); }}>
                    {it.label}
                  </button>
                )}
              </For>
            </span>
          </Portal>
        )}
      </Show>
    </>
  );
}
