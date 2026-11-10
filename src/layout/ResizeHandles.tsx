// Úchyty velikosti šuplíku v režimu úprav (kolo 23): boky = šířka (na středové ose symetricky, u hrany jen
// volný bok; široký notch roztažitelný), spodek = výška. Vzhled jako úchyty karet (`.lay-rs`: linka akcentu,
// žádná trvalá animace); rozměr v px v bublině `.lay-dim` jen během / po tažení, „nevejde se N karet“
// pod spodním úchytem. Nahoře interaktivní panel pro rozměr zavřeného notche a rychlý reset.
// Logika: `drawerSize.ts` + `theme/size.ts`. Kreslí MainPage jen v úpravách.
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { layoutNow, savedAppearance } from "../theme/apply";
import { widthSides } from "../theme/size";
import {
  dim,
  keyResize,
  openDimText,
  overflow,
  resetDrawerSize,
  resetEdge,
  resetStripSize,
  startResize,
  startStripResize,
  stripDim,
  type Edge,
} from "./drawerSize";
import "./drawerSize.css";

export default function ResizeHandles() {
  const pos = () => savedAppearance().strip.position;
  const sides = () => widthSides(pos(), layoutNow());
  const handle = (edge: Edge) => (
    <div class="dsz" classList={{ "dsz-left": edge === "left", "dsz-right": edge === "right", "dsz-bottom": edge === "bottom" }} role="slider" tabindex="0"
      aria-orientation={edge === "bottom" ? "vertical" : "horizontal"}
      aria-label={edge === "bottom" ? t("cards.size.height", "Výška šuplíku — táhni, šipky, dvojklik = výchozí") : t("cards.size.width", "Šířka šuplíku — táhni, šipky, dvojklik = výchozí")}
      title={t("cards.size.title", "Táhni = velikost šuplíku (Alt = bez přichycení k mřížce), dvojklik = výchozí")}
      onPointerDown={(e) => startResize(e, edge, pos())}
      onDblClick={() => resetEdge(edge)}
      onKeyDown={(e) => keyResize(e, edge)} />
  );
  return (
    <>
      <div class="dsz-panel">
        <div class="dsz-closed-wrap">
          <div
            class="dsz-closed"
            style={{
              width: `${Math.min(560, stripDim().width)}px`,
              height: `${stripDim().height}px`,
            }}
          >
            <span>
              {t("cards.size.closedLabel", "Zavřený notch")}: {stripDim().width} × {stripDim().height} px
            </span>
            <div
              class="dsz-ch-r"
              role="slider"
              tabindex="0"
              aria-orientation="horizontal"
              aria-label={t("cards.size.closedWidthHint", "Táhni = šířka zavřeného notche, dvojklik = 300 px")}
              title={t("cards.size.closedWidthHint", "Táhni = šířka zavřeného notche, dvojklik = 300 px")}
              onPointerDown={(e) => startStripResize(e, "width")}
              onDblClick={resetStripSize}
            />
            <div
              class="dsz-ch-b"
              role="slider"
              tabindex="0"
              aria-orientation="vertical"
              aria-label={t("cards.size.closedHeightHint", "Táhni = výška zavřeného notche, dvojklik = 30 px")}
              title={t("cards.size.closedHeightHint", "Táhni = výška zavřeného notche, dvojklik = 30 px")}
              onPointerDown={(e) => startStripResize(e, "height")}
              onDblClick={resetStripSize}
            />
            <div
              class="dsz-ch-c"
              role="slider"
              tabindex="0"
              aria-label={t("cards.size.closedBothHint", "Táhni = šířka i výška zavřeného notche")}
              title={t("cards.size.closedBothHint", "Táhni = šířka i výška zavřeného notche")}
              onPointerDown={(e) => startStripResize(e, "both")}
              onDblClick={resetStripSize}
            />
          </div>
        </div>
        <div class="dsz-bar">
          <span>
            {t("cards.size.closedLabel", "Zavřený notch")}: {stripDim().width} × {stripDim().height} px
          </span>
          <button type="button" class="mini" onClick={resetStripSize}>
            {t("cards.size.closedReset", "Výchozí notch")}
          </button>
          <span>
            {t("cards.size.openLabel", "Otevřený šuplík")}: {openDimText()}
          </span>
          <button type="button" class="mini" onClick={resetDrawerSize}>
            {t("cards.size.openReset", "Výchozí šuplík")}
          </button>
        </div>
      </div>
      <For each={sides()}>{(s) => handle(s)}</For>
      {handle("bottom")}
      <Show when={dim() || overflow() > 0}>
        <div class="dsz-info" role="status">
          <Show when={dim()}><span class="lay-dim">{dim()}</span></Show>
          <Show when={overflow() > 0}>
            <span class="lay-dim">{t("cards.size.overflow", "Karet mimo pohled: {n} — posuň dolů, nic se nesmazalo", { n: overflow() })}</span>
          </Show>
        </div>
      </Show>
    </>
  );
}
