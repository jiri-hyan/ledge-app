// Panel Vzhled, krok „Zavřený“ (jak vypadá Ledge u horní hrany, když je šuplík zavřený): galerie 12 vzhledů
// (`strip.look`, data `strip/looks.ts`) s živými HTML náhledy na „horní hraně monitoru“
// (`previews/StripLookPreview.tsx`) a přepínačem ukázkového dění, pod ní jen pozice a velikost proužku.
// Zbytek je v „Pokročilé“: tvar klasického proužku (galerie, ne řada 8 tlačítek), linka dole, krytí a celé
// „Co proužek ukazuje“ (`StripSettings.tsx`: položky, barvy, klid, vlastní text, časovač, monitor).
// Klik na dlaždici = rozpracovaný config + „Vrátit“ v liště; skutečný proužek se překreslí po Uložit.
import { For, Show } from "solid-js";
import { lang, t } from "../../i18n";
import type { StripLookId, StripStyle } from "../../types";
import { STRIP_LOOKS } from "../../strip/looks";
import { STRIP_STYLES, isHex, stripShape } from "../../theme/appearance";
import { offerUndo } from "../ctx";
import { POSITION_STEPS, SIZE_STEPS } from "../appearanceModel";
import StripLookPreview from "../previews/StripLookPreview";
import StripSettings from "./StripSettings";
import { Advanced, Seg, Slider, StateSwitch, lit, pct, previewState, type Ap } from "./apUi";

/** Výchozí krytí proužku (`strip.opacity`, config.rs `Default`). */
const STRIP_OPACITY = 0.82;

/** Název vzhledu z dat od I (`{ cs, en }`). */
const lookName = (x: { cs: string; en: string }) => (lang() === "en" ? x.en : x.cs);
export const normLook = (v: unknown): StripLookId => (STRIP_LOOKS.some((x) => x.id === v) ? (v as StripLookId) : "classic");

/** Jednořádkový popis vzhledu (literály kvůli hlídači překladů). */
const LOOK_DESC: Record<StripLookId, () => string> = {
  classic: () => t("appearance.look.classic.short", "Dnešní proužek s položkami; tvar zvolíš v Pokročilé."),
  island: () => t("appearance.look.island.short", "Černá kapsle u hrany; při dění se roztáhne a ukáže, co se děje."),
  dots: () => t("appearance.look.dots.short", "Pár barevných teček místo textu — skoro neviditelné."),
  line: () => t("appearance.look.line.short", "Tenká linka přes celou hranu; barva ukazuje stav."),
  icons: () => t("appearance.look.icons.short", "Ikony s čísly jako v liště telefonu."),
  clock: () => t("appearance.look.clock.short", "Velké hodiny a jedna stavová tečka."),
  corner: () => t("appearance.look.corner.short", "Malý odznak v rohu, ne uprostřed."),
  ghost: () => t("appearance.look.ghost.short", "V klidu není vidět nic; ukáže se, až na tebe něco čeká."),
  bar: () => t("appearance.look.bar.short", "Pruh přes celou šířku jako lišta terminálu."),
  notch: () => t("appearance.look.notch.short", "Černý výřez s oblými oušky — jako součást monitoru."),
  progress: () => t("appearance.look.progress.short", "Krátký text a lišta průběhu: limit, časovač, skladba."),
  ticker: () => t("appearance.look.ticker.short", "Jeden řádek; víc věcí se jemně střídá."),
};

const SHAPE_LABEL: Record<StripStyle, () => string> = {
  pill: () => t("appearance.strip.pill", "Pilulka"),
  line: () => t("appearance.strip.line", "Pruh"),
  dot: () => t("appearance.strip.dot", "Tečky"),
  term: () => t("appearance.strip.term", "Terminál"),
  tag: () => t("appearance.strip.tag", "Štítek"),
  glow: () => t("appearance.strip.glow", "Neon"),
  block: () => t("appearance.strip.block", "Blok"),
  drop: () => t("appearance.strip.drop", "Kapka"),
};

export default function AppearanceClosed(q: { ap: Ap; adv: boolean; setAdv: (v: boolean) => void }) {
  const p = q.ap.p;
  const a = () => q.ap.a();
  const look = () => normLook(p.cfg.strip?.look);
  const setStripCfg = (k: string, v: unknown) => (p.cfg.strip ? p.setCfg("strip", k, v) : p.setCfg("strip", { [k]: v }));
  const pick = (id: StripLookId) => {
    const prev = look();
    if (prev === id) return;
    setStripCfg("look", id);
    const x = STRIP_LOOKS.find((l) => l.id === id);
    offerUndo(t("appearance.closed.picked", "Zavřený Ledge: {name} — proužek se změní po Uložit", { name: x ? lookName(x) : id }), () => setStripCfg("look", prev));
  };
  const setStrip = (patch: Partial<ReturnType<typeof a>["strip"]>) => q.ap.set("strip", { ...a().strip, ...patch });
  const opacity = () => p.cfg.strip?.opacity ?? STRIP_OPACITY;

  return (
    <>
      <div class="card ap-top" id="f-strip.look" classList={{ hit: lit("strip.look") }}>
        <h4>{t("appearance.closed.title", "Jak vypadá zavřený Ledge")}</h4>
        <p class="ap-note">{t("appearance.closed.lead", "Vyber obrázek. Nahoře přepínej, co se zrovna děje — náhledy ukážou, jak na to vzhled zareaguje. Data jsou ukázková.")}</p>
        <StateSwitch />
        <div class="ap-looks" role="radiogroup" aria-label={t("appearance.closed.title", "Jak vypadá zavřený Ledge")}>
          <For each={STRIP_LOOKS}>
            {(x) => (
              <button class="ap-style" classList={{ on: look() === x.id }} role="radio" aria-checked={look() === x.id} title={LOOK_DESC[x.id]()} onClick={() => pick(x.id)}>
                <StripLookPreview look={x.id} state={previewState()} a={a()} opacity={opacity()} />
                <span class="ap-name">{lookName(x)}</span>
                <small class="ap-desc">{LOOK_DESC[x.id]()}</small>
              </button>
            )}
          </For>
        </div>
      </div>

      <div class="card ap-top" id="f-appearance.strip" classList={{ hit: lit("appearance.strip") }}>
        <div class="opt">
          <span>{t("appearance.strip.position", "Pozice")}<small>{t("appearance.strip.positionHint", "Proužek i šuplík u horní hrany; myš otevírá tam, kde proužek je.")}</small></span>
          <Seg label={t("appearance.strip.position", "Pozice")} value={a().strip.position} set={(v) => setStrip({ position: v })}
            options={POSITION_STEPS.map((v) => [v, ({ left: t("appearance.strip.left", "Vlevo"), center: t("appearance.strip.center", "Uprostřed"), right: t("appearance.strip.right", "Vpravo") })[v]])} />
        </div>
        <div class="opt">
          <span>{t("appearance.strip.size", "Velikost")}</span>
          <Seg label={t("appearance.strip.size", "Velikost")} value={a().strip.size} set={(v) => setStrip({ size: v })}
            options={SIZE_STEPS.map((v) => [v, ({ small: t("appearance.strip.small", "Malý"), normal: t("appearance.strip.normal", "Střední"), large: t("appearance.strip.large", "Velký") })[v]])} />
        </div>
      </div>

      <Advanced open={q.adv} onToggle={q.setAdv} hint={t("appearance.closed.advHint", "Co proužek ukazuje, barvy položek, klid, vlastní text, tvar, linka, krytí a monitor")}>
        <Show when={look() === "classic"}>
          <div class="ap-col">
            <span>{t("appearance.strip.shape", "Tvar klasického proužku")}<small>{t("appearance.strip.styleHint", "„Tečky“ ukazují jen stavy, bez textu a času.")}</small></span>
            <div class="ap-shapes" role="radiogroup" aria-label={t("appearance.strip.shape", "Tvar klasického proužku")}>
              <For each={STRIP_STYLES}>
                {(s) => (
                  <button class="ap-layout" classList={{ on: a().strip.style === s }} role="radio" aria-checked={a().strip.style === s} onClick={() => setStrip({ style: s })}>
                    <span class="ap-shape" data-shape={s} aria-hidden="true">
                      <i style={{ height: `${Math.round(stripShape(s, "normal").h * 0.55)}px`, "border-radius": `0 0 ${stripShape(s, "normal").r * 0.55}px ${stripShape(s, "normal").r * 0.55}px` }} />
                    </span>
                    <span class="ap-name">{SHAPE_LABEL[s]()}</span>
                  </button>
                )}
              </For>
            </div>
          </div>
        </Show>
        <div class="opt">
          <span>{t("appearance.strip.color", "Linka dole")}</span>
          <span class="ap-accents">
            <Seg label={t("appearance.strip.color", "Linka dole")} value={a().strip.color === "" || a().strip.color === "accent" ? a().strip.color : "own"}
              set={(v) => setStrip({ color: v === "own" ? (isHex(a().strip.color) ? a().strip.color : "#ffffff") : v })}
              options={[["", t("appearance.strip.lineGradient", "Přechod")], ["accent", t("appearance.strip.lineAccent", "Akcent")], ["own", t("appearance.strip.lineOwn", "Vlastní")]]} />
            <Show when={isHex(a().strip.color)}>
              <label class="ap-acc own on"><input type="color" value={a().strip.color} aria-label={t("appearance.strip.lineOwn", "Vlastní")} onInput={(e) => setStrip({ color: e.currentTarget.value })} /></label>
            </Show>
          </span>
        </div>
        <Slider label={t("appearance.strip.opacity", "Krytí proužku")} min={0.3} max={1} step={0.05} value={opacity()} fmt={pct} set={(v) => setStripCfg("opacity", +v.toFixed(2))} />
        <StripSettings />
      </Advanced>
    </>
  );
}
