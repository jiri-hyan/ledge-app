// Režimy „co se stane po najetí myší“ (`drawer.hover`, Rust `hovermode.rs` `HoverMode`). Id `full|compact|island`
// jsou kontrakt (config, výběr v Nastavení → Vzhled → Po najetí). `cs` / `en` = název, `desc` = popis
// v jazyce UI (přeloží se až při čtení, ne při importu modulu).
import { t } from "../i18n.ts";

export type HoverModeId = "full" | "compact" | "island";

export interface HoverMode {
  id: HoverModeId;
  /** klíč názvu v i18n (`cards.hover.mode.<id>`) */
  key: string;
  cs: string;
  en: string;
  /** popis jednou větou v jazyce UI */
  readonly desc: string;
}

export const HOVER_MODES: readonly HoverMode[] = [
  {
    id: "full",
    key: "cards.hover.mode.full",
    cs: "Rovnou celý šuplík",
    en: "Full drawer right away",
    get desc() {
      return t("cards.hover.mode.full.desc", "Dnešní chování: najetí otevře celý šuplík.");
    },
  },
  {
    id: "compact",
    key: "cards.hover.mode.compact",
    cs: "Kompaktní náhled",
    en: "Compact preview",
    get desc() {
      return t("cards.hover.mode.compact.desc", "Malý panel s tím nejdůležitějším: schválení, session Clauda, hudba, čas a limit. Kliknutím se rozbalí celý šuplík.");
    },
  },
  {
    id: "island",
    key: "cards.hover.mode.island",
    cs: "Roztažený ostrůvek",
    en: "Expanded island",
    get desc() {
      return t("cards.hover.mode.island.desc", "Pod proužkem se roztáhne kapsle a ukáže jen to, co se právě děje, s tlačítky. Nejméně rušivé.");
    },
  },
];

/** Výchozí režim (starý config bez `drawer.hover`). */
export const DEFAULT_HOVER_MODE: HoverModeId = "full";

/** Režim z configu; neznámá hodnota = výchozí. */
export const hoverModeOf = (v: unknown): HoverModeId => (HOVER_MODES.some((m) => m.id === v) ? (v as HoverModeId) : DEFAULT_HOVER_MODE);
