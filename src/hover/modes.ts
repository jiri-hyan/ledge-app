// Režimy „co se stane po najetí myší“ (`drawer.hover`, Rust `hovermode.rs` `HoverMode`). Id `full|island|dock`
// jsou kontrakt (config, výběr v Nastavení → Vzhled → Po najetí). `cs` / `en` = název; popis řádku v Nastavení má
// `AppearanceHover.tsx` `DESC` (jediné místo, dřív tu byl druhý přepis `desc`).

export type HoverModeId = "full" | "island" | "dock";

export interface HoverMode {
  id: HoverModeId;
  /** klíč názvu v i18n (`cards.hover.mode.<id>`) */
  key: string;
  cs: string;
  en: string;
}

export const HOVER_MODES: readonly HoverMode[] = [
  {
    id: "full",
    key: "cards.hover.mode.full",
    cs: "Rovnou celý šuplík",
    en: "Full drawer right away",
  },
  {
    id: "island",
    key: "cards.hover.mode.island",
    cs: "Roztažený ostrůvek",
    en: "Expanded island",
  },
  {
    id: "dock",
    key: "cards.hover.mode.dock",
    cs: "Dok",
    en: "Dock",
  },
];

/** Výchozí režim (starý config bez `drawer.hover`). */
export const DEFAULT_HOVER_MODE: HoverModeId = "full";
