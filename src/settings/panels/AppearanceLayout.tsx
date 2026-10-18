// Panel Vzhled, část „Šířka a rozložení“ (předvolby šířky + vlastní px, rozložení hlavní stránky jako
// malé schéma sloupců) a rozbalovací „Upravit detaily“ (písmo, materiál, hustota, velikost písma, rohy,
// krytí a rozmazání pozadí) — jednotlivé složky stylu, které jde přebít. Hodnoty stylů: `theme/looks.ts`.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import type { FontLook, LayoutId, Material, WidthPreset } from "../../types";
import { FONTS, LAYOUTS, MATERIALS, WIDTHS } from "../../theme/looks";
import { lbl } from "../../theme/label";
import { Seg, Slider, lit, pct, type Ap } from "./apUi";

/** Výchozí `drawer.width` (config.rs `Default`). */
export const DRAWER_WIDTH = 1080;
const keys = <T extends string>(o: Record<T, unknown>) => Object.keys(o) as T[];

export function LayoutCard(q: { ap: Ap }) {
  const a = () => q.ap.a();
  const width = () => q.ap.p.cfg.drawer?.width ?? DRAWER_WIDTH;
  return (
    <div class="card" id="f-appearance.layout" classList={{ hit: lit("appearance.layout") }}>
      <h4>{t("appearance.layout.title", "Šířka a rozložení")}</h4>
      <div class="opt" id="f-drawer.width" classList={{ hit: lit("drawer.width") }}>
        <span>{t("appearance.width", "Šířka šuplíku")}<small>{a().width === "full" ? t("appearance.width.fullHint", "Přes celou šířku monitoru.") : t("appearance.width.hint", "Na menším monitoru se ořízne na 95 % šířky.")}</small></span>
        <Seg<WidthPreset> label={t("appearance.width", "Šířka šuplíku")} value={a().width} set={(v) => q.ap.set("width", v)}
          options={keys(WIDTHS).map((w) => [w, lbl(WIDTHS[w].name)])} />
      </div>
      <Show when={a().width === "custom"}>
        <Slider label={t("appearance.width.custom", "Vlastní šířka")} min={720} max={2400} step={20} value={width()} fmt={(v) => `${v} px`}
          set={(v) => q.ap.p.setCfg("drawer", "width", v)} />
      </Show>
      <div class="ap-layouts" role="radiogroup" aria-label={t("appearance.layout.pick", "Rozložení karet")}>
        <For each={keys(LAYOUTS)}>
          {(id: LayoutId) => (
            <button class="ap-layout" classList={{ on: a().layout === id }} role="radio" aria-checked={a().layout === id} onClick={() => q.ap.set("layout", id)}>
              <span class="ap-lschema" data-layout={id} style={{ "--mc": String(LAYOUTS[id].cols ?? 4) }} aria-hidden="true">
                <For each={Array.from({ length: LAYOUTS[id].cols ?? 4 })}>{() => <i />}</For>
              </span>
              <span class="ap-name">{lbl(LAYOUTS[id].name)}</span>
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

export function DetailsCard(q: { ap: Ap }) {
  const a = () => q.ap.a();
  return (
    <details class="card ap-details" id="f-appearance.details" classList={{ hit: lit("appearance.details") || lit("drawer.glass") }} open={lit("appearance.details") || lit("drawer.glass")}>
      <summary><h4>{t("appearance.details.title", "Upravit detaily")}</h4><small>{t("appearance.details.hint", "Písmo, materiál, hustota, rohy a pozadí — přebijí hodnoty stylu.")}</small></summary>
      <div class="opt">
        <span>{t("appearance.font.title", "Písmo")}<small>{t("appearance.font.hint", "Jen písma, která má Windows.")}</small></span>
        <Seg<FontLook> label={t("appearance.font.title", "Písmo")} value={a().font} set={(v) => q.ap.set("font", v)} options={keys(FONTS).map((f) => [f, lbl(FONTS[f].name)])} />
      </div>
      <div class="opt">
        <span>{t("appearance.material.title", "Materiál")}</span>
        <span class="ap-chips" role="radiogroup" aria-label={t("appearance.material.title", "Materiál")}>
          <For each={keys(MATERIALS)}>
            {(m: Material) => (
              <button class="mini" classList={{ "ap-chip-on": a().material === m }} role="radio" aria-checked={a().material === m} onClick={() => q.ap.set("material", m)}>{lbl(MATERIALS[m].name)}</button>
            )}
          </For>
        </span>
      </div>
      <div class="opt">
        <span>{t("appearance.density", "Hustota")}</span>
        <Seg label={t("appearance.density", "Hustota")} value={a().density} set={(v) => q.ap.set("density", v)}
          options={[["compact", t("appearance.density.compact", "Úsporná")], ["normal", t("appearance.density.normal", "Běžná")], ["comfy", t("appearance.density.comfy", "Vzdušná")]]} />
      </div>
      <Slider label={t("appearance.font", "Velikost písma")} min={0.85} max={1.25} step={0.05} value={a().fontScale} fmt={pct} set={(v) => q.ap.set("fontScale", +v.toFixed(2))} />
      <div class="opt">
        <span>{t("appearance.radius", "Rohy")}</span>
        <Seg label={t("appearance.radius", "Rohy")} value={a().radius} set={(v) => q.ap.set("radius", v)}
          options={[["sharp", t("appearance.radius.sharp", "Ostré")], ["normal", t("appearance.radius.normal", "Běžné")], ["round", t("appearance.radius.round", "Kulaté")]]} />
      </div>
      <div id="f-drawer.glass">
        <Slider label={t("appearance.glass.opacity", "Krytí pozadí")} hint={t("appearance.glass.opacityHint", "Méně = víc prosvítá plocha za šuplíkem.")}
          min={0.25} max={1} step={0.05} value={a().glass.opacity} fmt={pct} set={(v) => q.ap.set("glass", { opacity: +v.toFixed(2) })} />
        <div class="opt">
          <span>{t("appearance.glass.blur", "Rozmazat plochu za šuplíkem")}<small>{t("appearance.glass.blurHint2", "Akryl Windows 11 — dražší na výkon; šuplík pak místo morfu z proužku jen vyjede shora a plovoucí panel ztratí odstup.")}</small></span>
          <button class="switch" classList={{ on: q.ap.blur() }} role="switch" aria-checked={q.ap.blur()} aria-label={t("appearance.glass.blur", "Rozmazat plochu za šuplíkem")}
            onClick={() => q.ap.setBlur(!q.ap.blur())} />
        </div>
      </div>
    </details>
  );
}
