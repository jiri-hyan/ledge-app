// Panel Vzhled, krok Otevřený → „Pokročilé“: rozložení karet (malé schéma sloupců), vlastní šířka v px
// a jednotlivé složky stylu, které jde přebít (písmo, materiál, velikost písma, rohy, krytí a rozmazání
// pozadí). Šířka předvolbou a hustota jsou hlavní volby kroku (`AppearanceOpen.tsx`). Hodnoty stylů:
// `theme/looks.ts`.
import { For } from "solid-js";
import { t } from "../../i18n";
import type { FontLook, LayoutId, Material } from "../../types";
import { FONTS, LAYOUTS, MATERIALS } from "../../theme/looks";
import { lbl } from "../../theme/label";
import { Seg, Slider, lit, pct, type Ap } from "./apUi";

/** Výchozí `drawer.width` (config.rs `Default`). */
export const DRAWER_WIDTH = 1080;
const keys = <T extends string>(o: Record<T, unknown>) => Object.keys(o) as T[];

/** Rozložení hlavní stránky jako galerie schémat (sloupce = `--deck-cols` stylu). */
export function LayoutPicker(q: { ap: Ap }) {
  const a = () => q.ap.a();
  return (
    <div class="ap-col">
      <span>{t("appearance.layout.pick", "Rozložení karet")}<small>{t("appearance.layout.pickHint", "Kolik sloupců má mřížka karet — náhled monitoru nahoře se změní hned.")}</small></span>
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

/** Vlastní šířka v px (`appearance.width: custom` + `drawer.width`). */
export function CustomWidth(q: { ap: Ap }) {
  const width = () => q.ap.p.cfg.drawer?.width ?? DRAWER_WIDTH;
  const on = () => q.ap.a().width === "custom";
  return (
    <div class="opt" id="f-drawer.width" classList={{ hit: lit("drawer.width") }}>
      <span>{t("appearance.width.custom", "Vlastní šířka")}<small>{on() ? t("appearance.width.hint", "Na menším monitoru se ořízne na 95 % šířky.") : t("appearance.width.customOff", "Teď platí předvolba z kroku nahoře — zapni, když chceš přesné px.")}</small></span>
      <span class="ap-range">
        <button class="st-switch" classList={{ on: on() }} role="switch" aria-checked={on()} aria-label={t("appearance.width.custom", "Vlastní šířka")}
          onClick={() => q.ap.set("width", on() ? "medium" : "custom")} />
        <input type="range" min={720} max={2400} step={20} value={width()} disabled={!on()} aria-label={t("appearance.width.custom", "Vlastní šířka")}
          onInput={(e) => q.ap.p.setCfg("drawer", "width", +e.currentTarget.value)} />
        <output>{width()} px</output>
      </span>
    </div>
  );
}

/** Složky stylu, které jde přebít: písmo, materiál, velikost písma, rohy, pozadí. */
export function DetailsRows(q: { ap: Ap }) {
  const a = () => q.ap.a();
  return (
    <div id="f-appearance.details" classList={{ hit: lit("appearance.details") }}>
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
      <Slider label={t("appearance.font", "Velikost písma")} min={0.85} max={1.25} step={0.05} value={a().fontScale} fmt={pct} set={(v) => q.ap.set("fontScale", +v.toFixed(2))} />
      <div class="opt">
        <span>{t("appearance.radius", "Rohy")}</span>
        <Seg label={t("appearance.radius", "Rohy")} value={a().radius} set={(v) => q.ap.set("radius", v)}
          options={[["sharp", t("appearance.radius.sharp", "Ostré")], ["normal", t("appearance.radius.normal", "Běžné")], ["round", t("appearance.radius.round", "Kulaté")]]} />
      </div>
      <div id="f-drawer.glass" classList={{ hit: lit("drawer.glass") }}>
        <Slider label={t("appearance.glass.opacity", "Krytí pozadí")} hint={t("appearance.glass.opacityHint", "Méně = víc prosvítá plocha za šuplíkem.")}
          min={0.25} max={1} step={0.05} value={a().glass.opacity} fmt={pct} set={(v) => q.ap.set("glass", { opacity: +v.toFixed(2) })} />
        <div class="opt">
          <span>{t("appearance.glass.blur", "Rozmazat plochu za šuplíkem")}<small>{t("appearance.glass.blurHint2", "Akryl Windows 11 — dražší na výkon; šuplík pak místo morfu z proužku jen vyjede shora a plovoucí panel ztratí odstup.")}</small></span>
          <button class="switch" classList={{ on: q.ap.blur() }} role="switch" aria-checked={q.ap.blur()} aria-label={t("appearance.glass.blur", "Rozmazat plochu za šuplíkem")}
            onClick={() => q.ap.setBlur(!q.ap.blur())} />
        </div>
      </div>
    </div>
  );
}
