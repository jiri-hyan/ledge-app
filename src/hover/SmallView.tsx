// Malé pohledy skořápky (`App.tsx`): peek (schválení / připomínka) a náhledy po najetí — ostrůvek / dok (`drawer.hover`). Který se ukáže, rozhoduje jádro (`hovermode.rs`) a posílá ho ve `view`.
import { Match, Switch } from "solid-js";
import type { Snapshot, View } from "../api";
import Peek from "../cards/Peek";
import Dock from "./Dock";
import Island from "./Island";
import { dockMode, listenDockNote, listenDockView } from "./dockmode";

// poslech už při načtení modulu (před prvním `view`, který SmallView vůbec vykreslí), ne až v komponentě
listenDockView();

export default function SmallView(p: { view: View; s: Snapshot; now: number; expandMs: number }) {
  // ostrůvek a dok jdou obě jako `view: "island"` — rozliší je `ViewMsg.dock` (L130, bez čtení configu)
  listenDockNote();
  return (
    <Switch fallback={<Peek s={p.s} />}>
      <Match when={p.view === "island" && dockMode() === true}><Dock s={p.s} now={p.now} expandMs={p.expandMs} /></Match>
      <Match when={p.view === "island" && dockMode() === false}><Island s={p.s} now={p.now} expandMs={p.expandMs} /></Match>
      <Match when={p.view === "island"}><div class="isl isl-dock" /></Match>
    </Switch>
  );
}
