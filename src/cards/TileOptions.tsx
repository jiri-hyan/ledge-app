// Volby karty Projekty (zástupci) v ⋯ menu karty: velikost dlaždic (Malé ikony / Dlaždice / Velké),
// popisek pod ikonou, větev a stav gitu, které projekty jsou na téhle záložce. Každá volba se hned uloží
// (`saveLayout` → jen pole `layout`, volby jsou na kartě té záložky — `layout.pages[].cards[].tiles`).
// Tytéž volby v Nastavení → Šuplík a karty (`layout/LayoutEditor.tsx`).
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { saveLayout } from "../layout/edit";
import { setTiles, showProject, TILE_SIZES, type TileOpts, type TileSize } from "../layout/model";
import { MenuChoice } from "./CardMenu";

export const tileSizeName = (s: TileSize): string =>
  s === "small" ? t("cards.tiles.small", "Malé ikony") : s === "large" ? t("cards.tiles.large", "Velké") : t("cards.tiles.tile", "Dlaždice");

export default function TileOptions(p: { page: string; tiles: TileOpts; projects: readonly { id: string; name: string }[]; cfgError: string | null }) {
  const save = (patch: Partial<TileOpts>) => void saveLayout((l) => setTiles(l, p.page, patch), p.cfgError);
  const shown = (id: string) => !p.tiles.hidden.includes(id);
  return (
    <>
      <span class="card-menu-sep" />
      <MenuChoice label={t("cards.tiles.size", "Dlaždice")} items={TILE_SIZES.map((s) => ({ v: s, text: tileSizeName(s) }))} value={p.tiles.size} onPick={(v) => save({ size: v })} />
      <button role="menuitemcheckbox" aria-checked={p.tiles.label} class="card-menu-check" classList={{ on: p.tiles.label }} onClick={() => save({ label: !p.tiles.label })}>
        {t("cards.tiles.label", "Název pod ikonou")}
      </button>
      <button role="menuitemcheckbox" aria-checked={p.tiles.git} class="card-menu-check" classList={{ on: p.tiles.git }} onClick={() => save({ git: !p.tiles.git })}>
        {t("cards.tiles.git", "Větev a stav gitu")}
      </button>
      <Show when={p.projects.length > 1}>
        <span class="card-menu-cap">{t("cards.tiles.which", "Projekty na téhle záložce")}</span>
        <span class="card-menu-scroll">
          <For each={p.projects}>
            {(pr) => (
              <button
                role="menuitemcheckbox"
                aria-checked={shown(pr.id)}
                class="card-menu-check"
                classList={{ on: shown(pr.id) }}
                onClick={() => void saveLayout((l) => showProject(l, p.page, pr.id, !shown(pr.id)), p.cfgError)}
              >
                {pr.name}
              </button>
            )}
          </For>
        </span>
      </Show>
    </>
  );
}
