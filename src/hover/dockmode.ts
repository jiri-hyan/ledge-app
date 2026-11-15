// Dok vs. ostrůvek: obě kreslí `view: "island"`, rozliší je `ViewMsg.dock` (L130, jádro ho bere z `drawer.hover`).
// Režim se drží v signálu, ať se nekreslí ostrůvek, který by za chvíli skočil na dok. Plus stav 📌 (`dockPinned`, L137):
// drží se tady, ne v komponentě — příchod události (schválení / otázka) dok přepne na ostrůvek a `DockRow` se odmontuje,
// jádro přitom dál drží `Opener::Pinned` (každé `view` nese `pinned` z jádra). Plus příznak „zkratka chce poznámku“ — událost `dock-note` z jádra
// (Ctrl+Alt+N) chodí po `show`, dok ji může stihnout přečíst až po vykreslení.
import { createSignal } from "solid-js";
import { on, type ViewMsg } from "../api";
import { dockOfView, dockPinAfter } from "./model";

const [dock, setDock] = createSignal<boolean | null>(null);
/** true = dok, false = ostrůvek, null = ještě nevíme (nekreslit nic) */
export const dockMode = dock;

const [pin, setPin] = createSignal(false);
/** 📌 v docku: odpovídá `Opener::Pinned` v jádře (přežije odmontování `DockRow` při události) */
export const dockPinned = pin;
export const dockPinDo = (ev: Parameters<typeof dockPinAfter>[0]) => setPin(dockPinAfter(ev));

const [want, setWant] = createSignal(false);
export const dockNoteWanted = want;
export const wantDockNote = setWant;

let viewListening = false;
/** Jednou za běh: `dock` z každého `view` (bez čtení configu); každé `view` = nové `show` / skrytí; upevnění drží, jen když ho jádro ohlásí (`ViewMsg.pinned`). */
export function listenDockView(): void {
  if (viewListening) return;
  viewListening = true;
  void on<ViewMsg>("view", (m) => {
    const d = dockOfView(m);
    if (d !== null) setDock(d);
    dockPinDo({ type: "view", pinned: m.pinned });
  }).catch(() => {
    viewListening = false;
  });
}

let listening = false;
/** Jednou za běh: poslech `dock-note` (jen okno šuplíku, `SmallView` ho tam jediné vykresluje). */
export function listenDockNote(): void {
  if (listening) return;
  listening = true;
  void on<boolean>("dock-note", () => {
    setDock(true); // zkratka v režimu dock = dok jistě
    setWant(true);
  }).catch(() => {
    listening = false;
  });
}
