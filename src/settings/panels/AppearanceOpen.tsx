// Panel Vzhled, krok „Otevřený“ (jak vypadá otevřený šuplík): galerie design stylů (`AppearanceStyles.tsx`),
// pod ní monitor ve skutečném poměru (`previews/MonitorPreview.tsx` — šuplík ve skutečné šířce v px vůči
// monitoru, skutečný počet sloupců a hustota) a dvě hlavní volby: Šířka (4 předvolby) a Hustota. Zbytek je
// v „Pokročilé“: barvy a akcent, rozložení karet, vlastní šířka v px, písmo, materiál, rohy, krytí a rozmazání.
// Vše mění jen rozpracovaný config (živě na celý šuplík přes `applyAppearance` v panelu); ukládá lišta.
import { For } from "solid-js";
import { t } from "../../i18n";
import type { WidthPreset } from "../../types";
import { THEMES, THEME_IDS } from "../../theme/themes";
import { WIDTHS } from "../../theme/looks";
import { isHex } from "../../theme/appearance";
import { scopeStyle } from "../../theme/apply";
import { lbl } from "../../theme/label";
import { DENSITY_STEPS, WIDTH_STEPS } from "../appearanceModel";
import MonitorPreview from "../previews/MonitorPreview";
import AppearanceStyles from "./AppearanceStyles";
import { CustomWidth, DRAWER_WIDTH, DetailsRows, LayoutPicker } from "./AppearanceLayout";
import { Advanced, Seg, lit, type Ap } from "./apUi";

const ACCENTS: [string, string, string][] = [
  ["#8b5cf6", "appearance.accent.violet", "Fialová"],
  ["#3b82f6", "appearance.accent.blue", "Modrá"],
  ["#14b8a6", "appearance.accent.teal", "Tyrkysová"],
  ["#22c55e", "appearance.accent.green", "Zelená"],
  ["#f59e0b", "appearance.accent.amber", "Jantarová"],
  ["#ef4444", "appearance.accent.red", "Červená"],
  ["#ec4899", "appearance.accent.pink", "Růžová"],
];

const DENSITY_LABEL = {
  comfy: () => t("appearance.density.comfy", "Vzdušná"),
  normal: () => t("appearance.density.normal", "Běžná"),
  compact: () => t("appearance.density.compact", "Hustá"),
};

function Colors(q: { ap: Ap }) {
  const a = () => q.ap.a();
  const set = q.ap.set;
  return (
    <div id="f-appearance.theme" classList={{ hit: lit("appearance.theme") }}>
      <h4>{t("appearance.theme.colors", "Barvy")}</h4>
      <div class="ap-themes" role="radiogroup" aria-label={t("appearance.theme.title", "Motiv")}>
        <For each={THEME_IDS}>
          {(id) => (
            <button class="ap-tile" classList={{ on: a().theme === id }} role="radio" aria-checked={a().theme === id} onClick={() => set("theme", id)}>
              <span class="ap-swatch theme-scope" style={scopeStyle({ ...a(), theme: id, glass: { opacity: 1 } })}>
                <i class="l1" /><i class="l2" /><i class="l3" /><i class="acc" />
              </span>
              <span class="ap-name">{t(THEMES[id].key, THEMES[id].cs)}</span>
            </button>
          )}
        </For>
      </div>
      <div class="opt" id="f-appearance.accent" classList={{ hit: lit("appearance.accent") }}>
        <span>{t("appearance.accent.title", "Akcent")}<small>{t("appearance.accent.hint", "Linka, záře a fokus. Výchozí = barva motivu.")}</small></span>
        <span class="ap-accents">
          <button class="ap-acc def" classList={{ on: !a().accent }} title={t("appearance.accent.default", "Podle motivu")} aria-label={t("appearance.accent.default", "Podle motivu")} onClick={() => set("accent", "")} />
          <For each={ACCENTS}>
            {([hex, key, cs]) => (
              <button class="ap-acc" classList={{ on: a().accent === hex }} style={{ background: hex }} title={t(key, cs)} aria-label={t(key, cs)} onClick={() => set("accent", hex)} />
            )}
          </For>
          <label class="ap-acc own" classList={{ on: !!a().accent && !ACCENTS.some(([h]) => h === a().accent) }} title={t("appearance.accent.custom", "Vlastní barva")}>
            <input type="color" value={a().accent || "#a78bfa"} aria-label={t("appearance.accent.custom", "Vlastní barva")} onInput={(e) => isHex(e.currentTarget.value) && set("accent", e.currentTarget.value)} />
          </label>
        </span>
      </div>
    </div>
  );
}

export default function AppearanceOpen(q: { ap: Ap; dirty: boolean; adv: boolean; setAdv: (v: boolean) => void }) {
  const a = () => q.ap.a();
  const customW = () => q.ap.p.cfg.drawer?.width ?? DRAWER_WIDTH;
  return (
    <>
      <AppearanceStyles ap={q.ap} dirty={q.dirty} />
      <div class="card ap-top" id="f-appearance.layout" classList={{ hit: lit("appearance.layout") }}>
        <h4>{t("appearance.open.size", "Šířka a hustota na tvém monitoru")}</h4>
        <MonitorPreview a={a()} customW={customW()} blur={q.ap.blur()} />
        <div class="opt">
          <span>{t("appearance.open.width", "Šířka")}<small>{a().width === "custom"
            ? t("appearance.open.widthCustom", "Teď vlastní šířka {px} px (Pokročilé).", { px: customW() })
            : t("appearance.open.widthHint", "Kolik místa šuplík zabere na tvém monitoru.")}</small></span>
          <Seg<WidthPreset> label={t("appearance.open.width", "Šířka")} value={a().width} set={(v) => q.ap.set("width", v)}
            options={WIDTH_STEPS.map((w) => [w, lbl(WIDTHS[w].name)])} />
        </div>
        <div class="opt" id="f-appearance.density" classList={{ hit: lit("appearance.density") }}>
          <span>{t("appearance.density", "Hustota")}<small>{t("appearance.density.hint", "Víc karet na obrazovce, nebo víc vzduchu.")}</small></span>
          <Seg label={t("appearance.density", "Hustota")} value={a().density} set={(v) => q.ap.set("density", v)}
            options={DENSITY_STEPS.map((d) => [d, DENSITY_LABEL[d]()])} />
        </div>
      </div>
      <Advanced open={q.adv} onToggle={q.setAdv} hint={t("appearance.open.advHint", "Barvy a akcent, rozložení karet, vlastní šířka, písmo, rohy, krytí a rozmazání")}>
        <Colors ap={q.ap} />
        <LayoutPicker ap={q.ap} />
        <CustomWidth ap={q.ap} />
        <DetailsRows ap={q.ap} />
      </Advanced>
    </>
  );
}
