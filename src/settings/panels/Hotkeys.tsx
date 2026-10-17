// Panel „Zkratky“: globální zkratky zapnout/vypnout a editor „stiskni kombinaci“ pro každou akci,
// kolize hned u pole (`model.hotkeyCollisions`, stejné pravidlo jako Rust `hotkey_collision`).
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Toggle } from "../fields/controls";
import { HotkeyInput } from "../fields/Hotkey";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import { hotkeyCollisions, type HotkeyAction } from "../model";
import type { PanelProps } from "../types";

const ACTIONS: HotkeyAction[] = ["toggle", "note", "approve", "deny"];

export default function HotkeysPanel(_p: PanelProps) {
  const c = useSettings();
  const h = () => c.cfg.hotkeys;
  const clash = () => hotkeyCollisions(h(), !!c.cfg.notes?.enabled);
  const conflict = (a: HotkeyAction) => {
    const other = clash()[a];
    return other ? t("settings.hotkeys.clash", "Stejná zkratka je u „{o}“", { o: fieldLabel(`hotkeys.${other}`) }) : null;
  };
  return (
    <>
      <PanelHead id="hotkeys.enabled" title={t("settings.nav.zkratky", "Zkratky")} desc={t("settings.hotkeys.desc", "Klávesové zkratky platí v celém Windows. Bez nich se šuplík otevírá myší a poznámka tlačítkem.")}>
        <Toggle path="hotkeys.enabled" label={t("settings.f.hotkeys.enabled", "Globální klávesové zkratky")} />
      </PanelHead>
      <Show when={h().enabled} fallback={<p class="st-empty">{t("settings.hotkeys.off", "Zkratky jsou vypnuté — Ledge žádnou neregistruje, takže nepřebije zkratky jiných aplikací.")}</p>}>
        <Section desc={t("settings.hotkeys.how", "Klikni na pole a stiskni kombinaci (např. Ctrl+Alt+N). Esc zruší, Backspace zkratku smaže.")}>
          <For each={ACTIONS}>
            {(a) => (
              <Row id={`hotkeys.${a}`} dim={a === "note" && !c.cfg.notes?.enabled} hint={a === "note" && !c.cfg.notes?.enabled ? t("settings.hotkeys.noteOff", "Poznámky jsou vypnuté — tahle zkratka se neregistruje") : undefined}>
                <HotkeyInput value={h()[a] ?? ""} onChange={(v) => c.set(`hotkeys.${a}`, v)} label={fieldLabel(`hotkeys.${a}`)} conflict={conflict(a)} />
              </Row>
            )}
          </For>
        </Section>
      </Show>
    </>
  );
}
