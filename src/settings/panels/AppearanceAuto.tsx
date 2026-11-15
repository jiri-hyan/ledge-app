// Studio → Vzhled → Barvy, část „Podle Windows“ (kolo 34, `theme/auto.ts`, config `appearance.auto`): přepínač,
// dvojice motivů (světlý / tmavý — výběr jen z motivů daného tónu) a akcent Windows. Změnu Windows zachytí Rust
// událostí (registr, žádný polling); tady se jen ukládá volba. Při zapnutí se vybraný motiv / vlastní motiv ignoruje.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { DEFAULT_AUTO } from "../../theme/auto";
import { lbl } from "../../theme/label";
import { THEMES, THEME_IDS } from "../../theme/themes";
import type { AutoLook, ThemeId } from "../../types";
import { Toggle } from "../fields/controls";
import { type Ap } from "./apUi";

export default function AppearanceAuto(q: { ap: Ap }) {
  const auto = () => q.ap.a().auto ?? DEFAULT_AUTO;
  const put = (p: Partial<AutoLook>) => q.ap.set("auto", { ...auto(), ...p });
  const pick = (light: boolean, key: "light" | "dark", label: string) => (
    <label class="opt">
      <span>{label}</span>
      <select aria-label={label} value={auto()[key]} onChange={(e) => put({ [key]: e.currentTarget.value as ThemeId })}>
        <For each={THEME_IDS.filter((id) => THEMES[id].light === light)}>{(id) => <option value={id}>{lbl([THEMES[id].key, THEMES[id].cs])}</option>}</For>
      </select>
    </label>
  );
  return (
    <div id="f-appearance.auto">
      <div class="opt">
        <span>{t("appearance.auto.title", "Podle Windows")}<small>{t("appearance.auto.hint", "Světlý nebo tmavý motiv se přepne podle nastavení Windows (hned při změně, bez sledování na pozadí).")}</small></span>
        <Toggle label={t("appearance.auto.title", "Podle Windows")} value={auto().on} onChange={(v) => put({ on: v })} />
      </div>
      <Show when={auto().on}>
        {pick(true, "light", t("appearance.auto.light", "Motiv při světlém Windows"))}
        {pick(false, "dark", t("appearance.auto.dark", "Motiv při tmavém Windows"))}
      </Show>
      <div class="opt">
        <span>{t("appearance.auto.accent", "Akcent Windows")}<small>{t("appearance.auto.accent.hint", "Akcentová barva z Windows (doladěná na čitelnost) místo akcentu motivu.")}</small></span>
        <Toggle label={t("appearance.auto.accent", "Akcent Windows")} value={auto().accent} onChange={(v) => put({ accent: v })} />
      </div>
    </div>
  );
}
