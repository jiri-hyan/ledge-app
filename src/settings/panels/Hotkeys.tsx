// Sekce „Klávesové zkratky“ (stránka Otevírání a klávesy; dřív samostatný panel Zkratky): globální zkratky zapnout/vypnout
// a editor „stiskni kombinaci“ pro každou akci, kolize hned u pole (`model.hotkeyCollisions`, stejné
// pravidlo jako Rust `hotkey_collision`).
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Toggle } from "../fields/controls";
import { HotkeyInput } from "../fields/Hotkey";
import { Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import { hotkeyCollisions, type HotkeyAction } from "../model";

const ACTIONS: HotkeyAction[] = ["toggle", "note", "approve", "deny", "gameMode"];

export function HotkeysSection() {
  const c = useSettings();
  const h = () => c.cfg.hotkeys;
  const clash = () => hotkeyCollisions(h(), !!c.cfg.notes?.enabled);
  const conflict = (a: HotkeyAction) => {
    const other = clash()[a];
    return other ? t("settings.hotkeys.clash", "Stejná zkratka je u „{o}“", { o: fieldLabel(`hotkeys.${other}`) }) : null;
  };
  return (
    <Section
      id="hotkeys.enabled"
      title={t("settings.hotkeys.title", "Klávesové zkratky")}
      desc={t("settings.hotkeys.desc", "Klávesové zkratky platí v celém Windows. Bez nich se šuplík otevírá myší a poznámka tlačítkem.")}
      aside={<Toggle path="hotkeys.enabled" label={fieldLabel("hotkeys.enabled")} />}
    >
      <Show when={h().enabled} fallback={<p class="st-empty">{t("settings.hotkeys.off", "Zkratky jsou vypnuté — Ledge žádnou neregistruje, takže nepřebije zkratky jiných aplikací.")}</p>}>
        <p class="st-note">{t("settings.hotkeys.how", "Klikni na pole a stiskni kombinaci (např. Ctrl+Alt+N). Esc zruší, Backspace zkratku smaže.")}</p>
        <For each={ACTIONS}>
          {(a) => (
            <Row id={`hotkeys.${a}`} dim={a === "note" && !c.cfg.notes?.enabled} hint={a === "note" && !c.cfg.notes?.enabled ? t("settings.hotkeys.noteOff", "Poznámky jsou vypnuté — tahle zkratka se neregistruje") : undefined}>
              <HotkeyInput value={h()[a] ?? ""} onChange={(v) => c.set(`hotkeys.${a}`, v)} label={fieldLabel(`hotkeys.${a}`)} conflict={conflict(a)} />
            </Row>
          )}
        </For>
      </Show>
      {/* lístky na plochu: registruje se jen s lístkem na ploše, i s vypnutými zkratkami (cesta z průchozího režimu) */}
      <Row id="hotkeys.notePassthrough" dim={!c.cfg.notes?.enabled} hint={!c.cfg.notes?.enabled ? t("settings.hotkeys.noteOff", "Poznámky jsou vypnuté — tahle zkratka se neregistruje") : undefined}>
        <HotkeyInput value={h().notePassthrough ?? ""} onChange={(v) => c.set("hotkeys.notePassthrough", v)} label={fieldLabel("hotkeys.notePassthrough")} conflict={h().enabled ? conflict("notePassthrough") : null} />
      </Row>
      {/* rychlý nový lístek: výchozí bez zkratky, platí i s vypnutými zkratkami, ve hře nic */}
      <Row id="hotkeys.notePin" dim={!c.cfg.notes?.enabled} hint={!c.cfg.notes?.enabled ? t("settings.hotkeys.noteOff", "Poznámky jsou vypnuté — tahle zkratka se neregistruje") : undefined}>
        <HotkeyInput value={h().notePin ?? ""} onChange={(v) => c.set("hotkeys.notePin", v)} label={fieldLabel("hotkeys.notePin")} conflict={h().enabled ? conflict("notePin") : null} />
      </Row>
    </Section>
  );
}
