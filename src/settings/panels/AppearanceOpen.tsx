// Panel Vzhled, krok „Otevřený“ (jak vypadá otevřený šuplík): galerie design stylů (`AppearanceStyles.tsx`),
// pod ní barvy (`AppearanceThemes.tsx`: motivy a vlastní motivy, akcent), vzhled záložek (`AppearancePages.tsx`),
// karty 2026 a bento šablony (`layout/CardsGallery.tsx`, kolo 22)
// a monitor ve skutečném poměru (`previews/MonitorPreview.tsx` — šuplík ve skutečné šířce v px vůči
// monitoru, skutečný počet sloupců a hustota) a dvě hlavní volby: Šířka (4 předvolby) a Hustota. Zbytek je
// v „Pokročilé“: rozložení karet, vlastní šířka v px, písmo, materiál, rohy, krytí a rozmazání.
// Vše mění jen rozpracovaný config (živě na celý šuplík přes `applyAppearance` v panelu); ukládá lišta.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import type { PageSize, WidthPreset } from "../../types";
import { HEIGHT_RANGE, WIDTH_RANGE, setSize, sizeFor } from "../../theme/size";
import { normalizeLayout } from "../../layout/model";
import { pageName } from "../../layout/text";
import { WIDTHS } from "../../theme/looks";
import { lbl } from "../../theme/label";
import { DENSITY_STEPS, WIDTH_STEPS } from "../appearanceModel";
import MonitorPreview from "../previews/MonitorPreview";
import AppearanceStyles from "./AppearanceStyles";
import AppearanceThemes from "./AppearanceThemes";
import AppearancePages from "./AppearancePages";
import CardsGallery from "../../layout/CardsGallery";
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
      <CardsGallery />
      <div class="card ap-top" id="f-appearance.layout" classList={{ hit: lit("appearance.layout") }}>
        <h4>{t("appearance.open.size", "Šířka (rychle) a hustota na tvém monitoru")}</h4>
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
      <SizeTable ap={q.ap} />
      <Advanced open={q.adv} onToggle={q.setAdv} hint={t("appearance.open.advHint", "Rozložení karet, vlastní šířka, písmo, rohy, krytí a rozmazání")}>
        <LayoutPicker ap={q.ap} />
        <CustomWidth ap={q.ap} />
        <DetailsRows ap={q.ap} />
      </Advanced>
    </>
  );
}

/** Velikost otevřeného Ledge (kolo 23, `appearance.size`): vysvětlení tažení v úpravách, volba „Pamatovat
 *  velikost pro každou záložku“ (výchozí vypnuto) a tabulka přesných čísel (prázdné = výchozí: šířka z
 *  předvolby, výška podle obsahu). Mění jen draft Nastavení. Logika: `theme/size.ts`. */
function SizeTable(q: { ap: Ap }) {
  const size = () => q.ap.a().size;
  const perPage = () => !!size()?.perPage;
  const rows = () => (perPage() ? normalizeLayout(q.ap.p.cfg.layout).pages.map((p) => ({ id: p.id, name: pageName(p) })) : [{ id: "", name: t("appearance.size.all", "Všechny záložky") }]);
  const put = (id: string, v: PageSize) => q.ap.set("size", setSize(size(), id, v));
  const num = (v: string) => (v.trim() === "" ? 0 : Math.round(+v) || 0);
  return (
    <div class="card ap-top" id="f-appearance.size" classList={{ hit: lit("appearance.size") }}>
      <h4>{t("appearance.size.title", "Přesná velikost v px")}</h4>
      <p class="ap-note">{t("appearance.size.hint", "V úpravách šuplíku (✎) můžeš táhnout za boky (šířka, od středu na obě strany) a za spodek (výška). Přichytává se to k mřížce, s Alt volně; dvojklik na úchyt vrátí výchozí. Prázdné = výchozí: šířka podle předvolby výše, výška podle obsahu. Zadané číslo má přednost před předvolbou i před vlastní šířkou v Doladit.")}</p>
      <div class="opt" id="f-appearance.size.perPage" classList={{ hit: lit("appearance.size.perPage") }}>
        <span>{t("appearance.size.perPage", "Pamatovat velikost pro každou záložku")}<small>{t("appearance.size.perPageHint", "Přepnutí záložky šuplík plynule zvětší nebo zmenší. Vypnuto = jedna velikost pro všechny.")}</small></span>
        <button class="switch" classList={{ on: perPage() }} role="switch" aria-checked={perPage()} aria-label={t("appearance.size.perPage", "Pamatovat velikost pro každou záložku")}
          onClick={() => q.ap.set("size", { ...(size() ?? {}), perPage: !perPage() })} />
      </div>
      <table class="ap-size">
        <thead><tr>
          <th>{t("appearance.size.tab", "Záložka")}</th>
          <th>{t("appearance.size.width", "Šířka (px)")}</th>
          <th>{t("appearance.size.height", "Výška (px)")}</th>
          <th />
        </tr></thead>
        <tbody>
          <For each={rows()}>
            {(r) => {
              const v = () => sizeFor(size(), r.id);
              return (
                <tr>
                  <td>{r.name}</td>
                  <td><input type="number" min={WIDTH_RANGE[0]} max={WIDTH_RANGE[1]} step={10} value={v().width || ""} placeholder={t("appearance.size.auto", "výchozí")}
                    aria-label={`${r.name}: ${t("appearance.size.width", "Šířka (px)")}`} onChange={(e) => put(r.id, { width: num(e.currentTarget.value), height: v().height })} /></td>
                  <td><input type="number" min={HEIGHT_RANGE[0]} max={HEIGHT_RANGE[1]} step={10} value={v().height || ""} placeholder={t("appearance.size.content", "podle obsahu")}
                    aria-label={`${r.name}: ${t("appearance.size.height", "Výška (px)")}`} onChange={(e) => put(r.id, { width: v().width, height: num(e.currentTarget.value) })} /></td>
                  <td><Show when={v().width || v().height}>
                    <button class="mini" onClick={() => put(r.id, {})}>{t("appearance.size.reset", "Výchozí")}</button>
                  </Show></td>
                </tr>
              );
            }}
          </For>
        </tbody>
      </table>
      <p class="ap-note">{t("appearance.size.range", "Šířka {w0}–{w1} px (nejvýš 95 % monitoru), výška {h0}–{h1} px (1–5 řad karet, nejvýš 80 % výšky monitoru). Co se nevejde, se posouvá — nic se nemaže.", { w0: WIDTH_RANGE[0], w1: WIDTH_RANGE[1], h0: HEIGHT_RANGE[0], h1: HEIGHT_RANGE[1] })}</p>
    </div>
  );
}
