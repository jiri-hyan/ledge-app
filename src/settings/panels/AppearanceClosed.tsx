// Panel Vzhled, krok „Zavřený“ (jak vypadá Ledge u horní hrany, když je šuplík zavřený): galerie 15 vzhledů
// (`strip.look`, data `strip/looks.ts`) s živými HTML náhledy na „horní hraně monitoru“
// (`previews/StripLookPreview.tsx`), pod ní pozice a velikost
// proužku a „Co proužek ukazuje“ (`StripSettings.tsx`: položky, barvy, klid, vlastní text, časovač, monitor —
// od kola 21 v Základním, není to technika). V „Pokročilé“ zbyl jen tvar klasického proužku (galerie, ne řada
// 8 tlačítek), linka dole a krytí. Najetí na dlaždici = náhled ve scéně Studia (`peekLookOn`);
// klik platí hned, „Vrátit“ je v nabídce vrácení. Kolo 22: pod galerií
// „Tvar kapsle“ (`strip.lookShape`, 4 volby) jen u stylů, které tvar mají (`SHAPEABLE_LOOKS`). Kolo 23: pod galerií
// dlaždice „Vlastní“ (`strip.look = custom`, jen výslovným klikem) → editor `ComposeEditor.tsx` (`strip.compose`).
import { For, Show } from "solid-js";
import { lang, t } from "../../i18n";
import type { StripLookId, StripStyle } from "../../types";
import { LOOK_SHAPES, SHAPEABLE_LOOKS, STRIP_LOOKS, normShape, type LookShape } from "../../strip/looks";
import { STRIP_STYLES, isHex, stripShape } from "../../theme/appearance";
import { offerUndo } from "../ctx";
import { POSITION_STEPS, SIZE_STEPS } from "../appearanceModel";
import StripLookPreview from "../previews/StripLookPreview";
import { STRIP_OPACITY } from "../strip";
import StripSettings from "./StripSettings";
import ComposeEditor from "./ComposeEditor";
import { DEFAULT_COMPOSE, composeOf, normCompose } from "../../strip/compose";
import { Advanced, Seg, Slider, clearPeek, lit, pct, peekLookOn, previewState, type Ap } from "./apUi";


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
  minimum: () => t("appearance.look.minimum.short", "Jen baterie a čas v malé kapsli."),
  trio: () => t("appearance.look.trio.short", "Počasí | čas a datum | CPU, paměť a baterie."),
  rings: () => t("appearance.look.rings.short", "Prstence limitů Clauda a čas; točí se, jen když pracuje."),
  custom: () => t("appearance.look.custom.short", "Poskládej si tvar, rozměry a položky sám."),
};

const SHAPE_NAME: Record<LookShape, () => string> = {
  round: () => t("appearance.lookShape.round", "Oblá"),
  sharp: () => t("appearance.lookShape.sharp", "Ostré rohy"),
  long: () => t("appearance.lookShape.long", "Prodloužená"),
  island: () => t("appearance.lookShape.island", "Jako Mac"),
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
    clearPeek();
    const prev = look();
    if (prev === id) return;
    // Vlastní poprvé (compose nikdo neupravoval) začne z dosavadního vzhledu
    const prevCompose = p.cfg.strip?.compose;
    const fresh = JSON.stringify(normCompose(prevCompose)) === JSON.stringify(DEFAULT_COMPOSE);
    if (id === "custom" && fresh && prev !== "classic") setStripCfg("compose", composeOf(prev));
    setStripCfg("look", id);
    const x = STRIP_LOOKS.find((l) => l.id === id);
    offerUndo(t("appearance.closed.picked", "Zavřený Ledge: {name}", { name: x ? lookName(x) : id }), () => {
      setStripCfg("look", prev);
      if (prevCompose !== undefined) setStripCfg("compose", prevCompose);
    });
  };
  const setStrip = (patch: Partial<ReturnType<typeof a>["strip"]>) => q.ap.set("strip", { ...a().strip, ...patch });
  const shape = () => normShape(p.cfg.strip?.lookShape);
  const pickShape = (v: LookShape) => {
    const prev = shape();
    if (prev === v) return;
    setStripCfg("lookShape", v);
    offerUndo(t("appearance.lookShape.picked", "Tvar kapsle: {name}", { name: SHAPE_NAME[v]() }), () => setStripCfg("lookShape", prev));
  };
  const opacity = () => p.cfg.strip?.opacity ?? STRIP_OPACITY;

  return (
    <>
      <div class="card ap-top" id="f-strip.look" classList={{ hit: lit("strip.look") }}>
                <div class="ap-looks" role="radiogroup" aria-label={t("appearance.closed.title", "Jak vypadá zavřený Ledge")}>
          <For each={STRIP_LOOKS.filter((x) => x.id !== "custom")}>
            {(x) => (
              <button class="ap-style" classList={{ on: look() === x.id }} role="radio" aria-checked={look() === x.id} title={LOOK_DESC[x.id]()} {...peekLookOn(x.id)} onClick={() => pick(x.id)}>
                <StripLookPreview look={x.id} state={previewState()} a={a()} opacity={opacity()} shape={shape()} />
                <span class="ap-name">{lookName(x)}</span>
                <small class="ap-desc">{LOOK_DESC[x.id]()}</small>
              </button>
            )}
          </For>
        </div>
        <button class="ap-style cx-tile" classList={{ on: look() === "custom" }} role="radio" aria-checked={look() === "custom"} title={LOOK_DESC.custom()} {...peekLookOn("custom")} onClick={() => pick("custom")}>
          <StripLookPreview look="custom" compose={p.cfg.strip?.compose} state={previewState()} a={a()} opacity={opacity()} />
          <span class="ap-name">{t("appearance.compose.tile", "Vlastní — poskládat si vlastní")}</span>
          <small class="ap-desc">{LOOK_DESC.custom()}</small>
        </button>
        <Show when={SHAPEABLE_LOOKS.includes(look())}>
          <div class="opt" id="f-strip.lookShape" classList={{ hit: lit("strip.lookShape") }}>
            <span>{t("appearance.lookShape.title", "Tvar kapsle")}<small>{t("appearance.lookShape.hint", "„Jako Mac“ visí kousek pod hranou a je oblá ze všech stran.")}</small></span>
            <Seg<LookShape> label={t("appearance.lookShape.title", "Tvar kapsle")} value={shape()} set={pickShape} options={LOOK_SHAPES.map((v) => [v, SHAPE_NAME[v]()])} />
          </div>
        </Show>
      </div>

      <Show when={look() === "custom"}>
        <ComposeEditor value={p.cfg.strip?.compose} set={(c) => setStripCfg("compose", c)} a={a()} opacity={opacity()} state={previewState()} />
      </Show>

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

      <StripSettings />

      <Advanced open={q.adv} onToggle={q.setAdv} hint={t("appearance.closed.advHint", "Tvar klasického proužku, linka dole a krytí")}>
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
        <Show when={look() === "classic"}>
          <Slider label={t("appearance.strip.opacity", "Krytí klasického proužku")} min={0.3} max={1} step={0.05} value={opacity()} fmt={pct} set={(v) => setStripCfg("opacity", +v.toFixed(2))} />
        </Show>
      </Advanced>
    </>
  );
}
