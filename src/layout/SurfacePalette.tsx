// Výběr barevné sady povrchu karty (kolo 28): řádek vzorků v panelu vzhledu karty, jen u povrchů Přechod, Přelévání,
// Záře hrany a Proužky. První vzorek = podle akcentu karty (výchozí, pole `palette` chybí).
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { isColorSurface, PALETTE_COLORS, PALETTE_NAMES, PALETTES, type CardPalette } from "./surfaces";
import { tx } from "./text";
import type { CardLook } from "./look";
import "./surfaces.css";

export default function SurfacePalette(p: { look: CardLook; onPick: (v: CardPalette | undefined) => void }) {
  return (
    <Show when={isColorSurface(p.look.surface)}>
      <div class="tl-ed-row" role="group" aria-label={t("cards.cardlook.palette", "Barevná sada")}>
        <span class="tl-ed-cap">{t("cards.cardlook.palette", "Barevná sada")}</span>
        <div class="sf-pal">
          <For each={PALETTES}>
            {(k) => {
              const c = k === "card" ? undefined : PALETTE_COLORS[k];
              const on = () => (p.look.palette ?? "card") === k;
              return (
                <button
                  class="sf-sw"
                  classList={{ on: on() }}
                  style={{ "--sf-pa": c?.a ?? "color-mix(in srgb, var(--accent) 60%, #0c0c12)", "--sf-pb": c?.e ?? "var(--accent)" }}
                  title={tx(PALETTE_NAMES[k])}
                  aria-label={tx(PALETTE_NAMES[k])}
                  aria-pressed={on()}
                  onClick={() => p.onPick(k === "card" ? undefined : k)}
                />
              );
            }}
          </For>
        </div>
      </div>
    </Show>
  );
}
