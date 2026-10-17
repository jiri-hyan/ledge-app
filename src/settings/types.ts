// Společné rozhraní panelů Nastavení: shell (`src/Settings.tsx`) drží rozpracovaný config ve store,
// panel ho čte/mění přes `setCfg` a ukládá přes `save()` (validace a zápis řeší shell).
import type { SetStoreFunction } from "solid-js/store";
import type { Snapshot } from "../types";

export type PanelProps = {
  cfg: any;
  setCfg: SetStoreFunction<any>;
  save: () => Promise<void>;
  s: Snapshot;
};
