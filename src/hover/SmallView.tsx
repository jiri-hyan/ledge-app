// Malé pohledy skořápky (`App.tsx`): peek (schválení / připomínka) a náhledy po najetí — kompaktní náhled
// a ostrůvek (`drawer.hover`). Který se ukáže, rozhoduje jádro (`hovermode.rs`) a posílá ho ve `view`.
import { Match, Switch } from "solid-js";
import type { Snapshot, View } from "../api";
import Peek from "../cards/Peek";
import Compact from "./Compact";
import Island from "./Island";

export default function SmallView(p: { view: View; s: Snapshot; now: number; expandMs: number }) {
  return (
    <Switch fallback={<Peek s={p.s} />}>
      <Match when={p.view === "compact"}><Compact s={p.s} now={p.now} expandMs={p.expandMs} /></Match>
      <Match when={p.view === "island"}><Island s={p.s} now={p.now} expandMs={p.expandMs} /></Match>
    </Switch>
  );
}
