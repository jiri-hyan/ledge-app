// Galerie design stylů nahoře v panelu Vzhled: každý styl jako živá miniatura (proužek + šuplík ve
// sloupcích jeho rozložení, s jeho barvami, písmem a materiálem přes `scopeStyle`). Klik = náhled na
// celém šuplíku (panel aplikuje rozpracovaný vzhled); uložit / zahodit / vrátit = společná lišta
// Nastavení. Pod galerií: které složky jsou oproti stylu upravené a „Obnovit styl“.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { LAYOUTS, STYLES, styleOf, type StyleDef } from "../../theme/looks";
import { PARTS, styleChanges, withStyle, type PartId } from "../../theme/styles";
import { stripShape } from "../../theme/appearance";
import { scopeStyle } from "../../theme/apply";
import { lbl } from "../../theme/label";
import { lit, type Ap } from "./apUi";

const PART_LABEL: Record<PartId, () => string> = {
  colors: () => t("appearance.part.colors", "barvy"),
  font: () => t("appearance.part.font", "písmo"),
  material: () => t("appearance.part.material", "materiál"),
  density: () => t("appearance.part.density", "hustota"),
  shape: () => t("appearance.part.shape", "rohy"),
  strip: () => t("appearance.part.strip", "proužek"),
  layout: () => t("appearance.part.layout", "rozložení"),
};
/** podíl šířky miniatury šuplíku podle předvolby šířky */
const MINI_W: Record<string, string> = { custom: "64%", narrow: "48%", medium: "64%", wide: "82%", full: "100%" };

function Mini(q: { s: StyleDef }) {
  const look = () => withStyle({}, q.s.id);
  const cols = () => Math.min(5, LAYOUTS[q.s.parts.layout].cols ?? 4);
  const sh = () => stripShape(q.s.parts.strip, "normal");
  return (
    <span class="ap-mini theme-scope" style={scopeStyle(look())} data-layout={q.s.parts.layout} aria-hidden="true">
      <i class="ap-mstrip" classList={{ "ap-glow": q.s.parts.strip === "glow", "ap-block": q.s.parts.strip === "block", "ap-drop": q.s.parts.strip === "drop" }}
        style={{ height: `${Math.round(sh().h * 0.42)}px`, "border-radius": `0 0 ${sh().r * 0.42}px ${sh().r * 0.42}px` }} />
      <span class="ap-mdrawer" style={{ width: MINI_W[q.s.parts.width] }}>
        <b class="ap-mt">14:32</b>
        <span class="ap-mgrid" style={{ "--mc": String(cols()) }}>
          <span class="ap-mcard wide"><i class="ap-mh" /><i class="ap-ml" /></span>
          <For each={Array.from({ length: cols() })}>{() => <span class="ap-mcard"><i class="ap-mh" /><i class="ap-ml" /><i class="ap-ml short" /></span>}</For>
        </span>
      </span>
    </span>
  );
}

export default function AppearanceStyles(q: { ap: Ap; dirty: boolean }) {
  const a = () => q.ap.a();
  const changed = () => styleChanges(a(), q.ap.blur());
  const pick = (id: string) => {
    q.ap.setAll(withStyle(a(), id));
    q.ap.setBlur(styleOf(id).parts.blur);
  };
  return (
    <div class="card ap-top" id="f-appearance.style" classList={{ hit: lit("appearance.style") }}>
      <h4>{t("appearance.style.title", "Design styl")}</h4>
      <div class="ap-styles" role="radiogroup" aria-label={t("appearance.style.title", "Design styl")}>
        <For each={STYLES}>
          {(s) => (
            <button class="ap-style" classList={{ on: a().style === s.id }} role="radio" aria-checked={a().style === s.id} title={lbl(s.desc)} onClick={() => pick(s.id)}>
              <Mini s={s} />
              <span class="ap-name">{lbl(s.name)}</span>
              <small class="ap-desc">{lbl(s.desc)}</small>
            </button>
          )}
        </For>
      </div>
      <div class="ap-bar">
        <Show when={q.dirty} fallback={<span class="ap-state">{t("appearance.state.saved", "Odpovídá uloženému vzhledu.")}</span>}>
          <span class="ap-state warn">{t("appearance.state.dirty", "Náhled — ulož nebo zahoď dole v liště.")}</span>
        </Show>
        <Show when={changed().length > 0}>
          <span class="ap-changed">
            {t("appearance.style.changed", "Upraveno: {parts}", { parts: PARTS.filter((x) => changed().includes(x)).map((x) => PART_LABEL[x]()).join(", ") })}
          </span>
          <button class="mini" onClick={() => pick(a().style)}>{t("appearance.style.reset", "Obnovit styl")}</button>
        </Show>
      </div>
    </div>
  );
}
