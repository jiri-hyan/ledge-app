// Tečky barvy poznámky v menu: první = bez vlastní barvy (dědí barvu projektu, je-li), pak 8 pojmenovaných barev.
import { For } from "solid-js";
import { t } from "../i18n";
import { colorName } from "../pin/StyleGallery";
import { NOTE_COLORS, paletteId } from "./meta";

export default function ColorDots(p: { color: string | null | undefined; /** barva projektu, kterou poznámka dědí */ inherited?: string | null; onPick: (id: string) => void }) {
  const cur = () => paletteId(p.color);
  return (
    <div class="note-colors" role="radiogroup" aria-label={t("notes.color.title", "Barva poznámky")}>
      <button type="button" class="note-color note-color-none" classList={{ on: !cur() }} role="radio" aria-checked={!cur()} title={p.inherited ? t("notes.color.byProject", "Podle projektu") : t("notes.color.none", "Bez barvy")} style={p.inherited ? { "--c": p.inherited } : undefined} onClick={() => p.onPick("")} />
      <For each={NOTE_COLORS}>
        {(c) => <button type="button" class="note-color" classList={{ on: cur() === c.id }} role="radio" aria-checked={cur() === c.id} title={colorName(c.id)} aria-label={colorName(c.id)} style={{ "--c": c.hex }} onClick={() => p.onPick(c.id)} />}
      </For>
    </div>
  );
}
