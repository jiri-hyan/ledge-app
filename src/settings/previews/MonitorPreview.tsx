// Náhled „monitor ve skutečném poměru“ pro krok Otevřený v panelu Vzhled: obrazovka v poměru stran
// skutečného monitoru (`window.screen` = monitor, na kterém je šuplík; neznámý → 16 : 9 bez čísel) a na ní
// šuplík ve SKUTEČNÉ šířce v px (`appearanceModel.monitorModel`, zrcadlo Rust `preset_width`), na své
// pozici u hrany, se skutečným počtem sloupců rozložení stylu. Šuplík se kreslí v plné velikosti
// (logické px, mezery a odsazení z proměnných vzhledu — hustota `--dens`, písmo, rohy, materiál) a celé
// plátno se zmenší `transform: scale` podle šířky náhledu (ResizeObserver, jen při změně velikosti).
// Změna šířky i hustoty je tak vidět ve stejném poměru jako na monitoru. Karty jsou ukázkové obdélníky.
import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { t, tp } from "../../i18n";
import type { Appearance } from "../../types";
import { scopeStyle } from "../../theme/apply";
import { monitorModel, sampleSpans } from "../appearanceModel";
import "./previews.css";

const DENSITY_NAME: Record<Appearance["density"], () => string> = {
  comfy: () => t("appearance.density.comfy", "Vzdušná"),
  normal: () => t("appearance.density.normal", "Běžná"),
  compact: () => t("appearance.density.compact", "Hustá"),
};

/** Rozměr monitoru z prohlížeče (logické px); mimo Ledge / bez obrazovky null. */
function screenSize(): { w: number; h: number } | null {
  if (typeof screen === "undefined") return null;
  return screen.width > 0 && screen.height > 0 ? { w: screen.width, h: screen.height } : null;
}

export default function MonitorPreview(q: { a: Appearance; customW: number; blur: boolean }) {
  const scr = screenSize();
  const m = () => monitorModel(q.a, q.customW, scr);
  let box!: HTMLDivElement;
  const [boxW, setBoxW] = createSignal(0);
  onMount(() => {
    setBoxW(box.clientWidth);
    if (typeof ResizeObserver !== "function") return;
    const ro = new ResizeObserver(() => setBoxW(box.clientWidth));
    ro.observe(box);
    onCleanup(() => ro.disconnect());
  });
  const k = () => (boxW() > 0 ? boxW() / m().monW : 0.2);
  const caption = () => {
    const cols = tp("appearance.mon.cols", m().cols, ["{n} sloupec", "{n} sloupce", "{n} sloupců"]);
    const dens = DENSITY_NAME[q.a.density]();
    return m().known
      ? t("appearance.mon.caption", "šuplík {w} px z {mon} px · {cols} · hustota {dens}", { w: m().drawerW, mon: m().monW, cols, dens })
      : t("appearance.mon.captionUnknown", "šuplík {w} px · {cols} · hustota {dens} (rozměr monitoru neznám)", { w: m().drawerW, cols, dens });
  };
  return (
    <figure class="apv-mon">
      <div class="apv-mon-screen" ref={box} style={{ "aspect-ratio": `${m().monW} / ${m().monH}` }}>
        <div class="apv-mon-stage theme-scope" style={{ ...scopeStyle(q.a), width: `${m().monW}px`, height: `${m().monH}px`, transform: `scale(${k()})` }} aria-hidden="true">
          <div class="apv-mon-drawer" classList={{ "apv-blur": q.blur, "apv-float": q.a.layout === "float" && !q.blur }} style={{ width: `${m().drawerW}px`, left: `${m().left}px` }}>
            <div class="apv-mon-head">
              <b>{t("appearance.sample.mode", "Práce")}</b>
              <span class="apv-mon-tabs"><i class="on" /><i /><i /></span>
            </div>
            <div class="apv-mon-deck" style={{ "--c": String(m().cols) }}>
              <For each={sampleSpans(m().cols)}>
                {(span) => (
                  <div class="apv-mon-card" style={{ "grid-column": `span ${span}` }}>
                    <i class="apv-mon-h" /><i class="apv-mon-l" /><i class="apv-mon-l" /><i class="apv-mon-l apv-short" />
                  </div>
                )}
              </For>
            </div>
          </div>
        </div>
      </div>
      <figcaption class="apv-mon-cap">
        {caption()}
        <Show when={q.a.width === "full"}><span class="apv-dim"> · {t("appearance.mon.fullNote", "přes celou šířku, bez okraje")}</span></Show>
      </figcaption>
    </figure>
  );
}
