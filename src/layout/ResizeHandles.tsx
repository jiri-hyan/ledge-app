// Úchyty velikosti šuplíku v režimu úprav (kolo 23): boky = šířka (na středové ose symetricky, u hrany jen
// volný bok; široký notch bez boků), spodek = výška. Vzhled jako úchyty karet (`.lay-rs`: linka akcentu,
// žádná trvalá animace); rozměr v px v bublině `.lay-dim` jen během / po tažení, „nevejde se N karet“
// pod spodním úchytem. Logika: `drawerSize.ts` + `theme/size.ts`. Kreslí MainPage jen v úpravách.
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { layoutNow, savedAppearance } from "../theme/apply";
import { widthSides } from "../theme/size";
import { dim, keyResize, overflow, resetEdge, startResize, type Edge } from "./drawerSize";
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
