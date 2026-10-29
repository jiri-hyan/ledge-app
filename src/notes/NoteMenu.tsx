// ⋯ menu poznámky v editoru: skupina „Export“ (Markdown / HTML do souboru, kopie do schránky, Otevřít v … registrovaných aplikacích).
// Seznam je společné menu `ui/Popover.tsx` (pevná pozice u tlačítka — editor s omezenou výškou ho neořízne);
// Esc ho zavře a nedojde k editoru (nezahodí rozepsané).

import { For, Show } from "solid-js";
import { t } from "../i18n";
import { createMenu, PopCap, PopItem, PopMenu } from "../ui/Popover";
import { IconMore } from "../pin/icons";
import { exportItems } from "./exportActions";
import ColorDots from "./ColorDots";

export default function NoteMenu(p: { path: string; body: () => string; color?: string | null; inherited?: string | null; onColor?: (id: string) => void }) {
  const menu = createMenu();
  const label = t("notes.export.more", "Export a další volby");
  return (
    <>
      <button class="icon-btn" classList={{ on: menu.open() }} aria-haspopup="menu" aria-expanded={menu.open()} title={label} aria-label={label} onClick={menu.toggle}>
        <IconMore />
      </button>
      <Show when={menu.at()}>
        {(at) => (
          <PopMenu anchor={at().anchor} keyboard={at().keyboard} trigger={menu.trigger()} onClose={menu.close} align="end" label={label}>
            <PopCap>{t("notes.export.title", "Export")}</PopCap>
            <For each={exportItems(p.path, p.body)}>
              {(it) => <PopItem onClick={() => { menu.close(); it.run(); }}>{it.label}</PopItem>}
            </For>
            <Show when={p.onColor}>
              {(pick) => (
                <>
                  <PopCap>{t("notes.color.title", "Barva poznámky")}</PopCap>
                  <ColorDots color={p.color} inherited={p.inherited} onPick={(id) => { menu.close(); pick()(id); }} />
                </>
              )}
            </Show>
          </PopMenu>
        )}
      </Show>
    </>
  );
}
