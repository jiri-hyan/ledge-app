// Panel Vzhled, krok „Otevřený“ (jak vypadá otevřený šuplík): galerie design stylů (`AppearanceStyles.tsx`),
// pod ní barvy (`AppearanceThemes.tsx`: motivy a vlastní motivy, akcent), vzhled záložek (`AppearancePages.tsx`)
// a monitor ve skutečném poměru (`previews/MonitorPreview.tsx` — šuplík ve skutečné šířce v px vůči
// monitoru, skutečný počet sloupců a hustota) a dvě hlavní volby: Šířka (4 předvolby) a Hustota. Zbytek je
// v „Pokročilé“: rozložení karet, vlastní šířka v px, písmo, materiál, rohy, krytí a rozmazání.
// Vše mění jen rozpracovaný config (živě na celý šuplík přes `applyAppearance` v panelu); ukládá lišta.
import { t } from "../../i18n";
import type { WidthPreset } from "../../types";
import { WIDTHS } from "../../theme/looks";
import { lbl } from "../../theme/label";
import { DENSITY_STEPS, WIDTH_STEPS } from "../appearanceModel";
import MonitorPreview from "../previews/MonitorPreview";
import AppearanceStyles from "./AppearanceStyles";
import AppearanceThemes from "./AppearanceThemes";
import AppearancePages from "./AppearancePages";
import { CustomWidth, DRAWER_WIDTH, DetailsRows, LayoutPicker } from "./AppearanceLayout";
import { Advanced, Seg, lit, type Ap } from "./apUi";

const DENSITY_LABEL = {
  comfy: () => t("appearance.density.comfy", "Vzdušná"),
  normal: () => t("appearance.density.normal", "Běžná"),
  compact: () => t("appearance.density.compact", "Hustá"),
};

export default function AppearanceOpen(q: { ap: Ap; dirty: boolean; adv: boolean; setAdv: (v: boolean) => void }) {
  const a = () => q.ap.a();
  const customW = () => q.ap.p.cfg.drawer?.width ?? DRAWER_WIDTH;
  return (
    <>
      <AppearanceStyles ap={q.ap} dirty={q.dirty} />
      <AppearanceThemes ap={q.ap} />
      <AppearancePages ap={q.ap} />
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
      <Advanced open={q.adv} onToggle={q.setAdv} hint={t("appearance.open.advHint", "Rozložení karet, vlastní šířka, písmo, rohy, krytí a rozmazání")}>
        <LayoutPicker ap={q.ap} />
        <CustomWidth ap={q.ap} />
        <DetailsRows ap={q.ap} />
      </Advanced>
    </>
  );
}
