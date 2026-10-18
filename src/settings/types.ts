// Společné rozhraní panelů Nastavení: shell (`src/Settings.tsx`) drží rozpracovaný config ve store,
// panel ho čte/mění přes `setCfg`; ukládá JEN lišta shellu (Uložit / Zahodit, validace a zápis) — panel
// sám neukládá (jedno chování ve všech panelech). Co jde až po uložení: `ctx.ts` `afterSave`.
import type { SetStoreFunction } from "solid-js/store";
import type { Snapshot } from "../types";

export type PanelProps = {
  cfg: any;
  setCfg: SetStoreFunction<any>;
  s: Snapshot;
};
