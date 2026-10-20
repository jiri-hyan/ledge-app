// ⋯ na řádku poznámky (karta, stránka Poznámky): Splnit · Archivovat · Smazat (v archivu Vrátit z archivu ·
// Smazat). Seznam se kreslí přes `Portal` s pevnou pozicí u tlačítka (karta s omezenou výškou ho neořízne),
// zavírá se klikem mimo, posunem i Esc. Texty akcí jsou společné s editorem a lístkem (`actionText`).

import { createSignal, For, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import type { NoteView } from "../api";
import { t } from "../i18n";
import { placePopup } from "../ui/place";
import { archiveNote, completeNote, deleteNote, reopenNote, unarchiveNote } from "./lifecycle";
import type { LifeAction } from "./lifeactions";
import { lifeActions } from "./lifeactions";
import { actionIcon, actionText } from "./lifelabels";

type Pos = { x: number; y: number; maxH: number };

/** Provede akci nad poznámkou ze šuplíku (lišta Zpět se ukáže sama). */
export function runAction(a: LifeAction, n: NoteView): Promise<boolean> {
  switch (a) {
    case "done": return completeNote(n);
    case "undone": return reopenNote(n);
    case "archive": return archiveNote(n);
    case "unarchive": return unarchiveNote(n);
    case "delete": return deleteNote(n);
  }
}

export default function RowMenu(p: { n: NoteView; archived: boolean }) {
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
  const acts = () => lifeActions(p.n.done, p.archived);
  return (
    <>
      <button class="note-act" classList={{ on: !!pos() }} ref={btn} aria-haspopup="menu" aria-expanded={!!pos()} title={t("notes.life.more", "Splnit, archivovat, smazat…")} aria-label={t("notes.life.more", "Splnit, archivovat, smazat…")} onClick={toggle}>⋯</button>
      <Show when={pos()}>
        {(at) => (
          <Portal mount={btn?.closest(".shell") ?? document.body}>
            <span class="card-menu-list note-act-list" role="menu" ref={list} style={{ position: "fixed", left: `${at().x}px`, top: `${at().y}px`, "max-height": `${at().maxH}px`, "overflow-y": "auto" }}>
              <For each={acts()}>
                {(a) => (
                  <button role="menuitem" classList={{ danger: a === "delete" }} onClick={() => { close(); void runAction(a, p.n); }}>
                    {actionIcon(a)}{actionText(a)}
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
