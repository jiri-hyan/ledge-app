// Výzdoba vlastního zavřeného Ledge v editoru Kostky (kolo 25, `strip.compose.deco` / `fill` / `sep` / `mono`):
// 11 designových předvoleb (jeden klik, Vrátit přes zásobník editoru), hrana (`ComposeEdge.tsx`, kolo 28: živé
// efekty a „Podle stavu Clauda“), pozadí těla (přechod až 3 barvami, volitelně přelévaný), oddělení dílů
// a nástroje dílu (styl jen z `stylesFor`, barva podle hodnoty / akcent / vlastní). Úpravy přes `commit` editoru.
import { For, Show, createSignal } from "solid-js";
import { t } from "../../i18n";
import type { Appearance } from "../../types";
import {
  COMPOSE_SEPS, DESIGN_IDS, FILL_DIRS, FILL_KINDS,
  type ComposeItem, type ComposeSep, type DesignId, type FillKind, type ItemStyle, type Piece, type StripCompose,
  designOf, sameCompose, setPieceColor, setStyle, stylesFor,
} from "../../strip/compose";
import StripLookPreview from "../previews/StripLookPreview";
import type { PreviewState } from "../appearanceModel";
import { Seg, lit } from "./apUi";
import { EdgeBlock } from "./ComposeEdge";

export const DESIGN_NAME: Record<DesignId, () => string> = {
  islands: () => t("appearance.compose.design.islands", "Ostrůvky"),
  neon: () => t("appearance.compose.design.neon", "Neonová linka"),
  terminal: () => t("appearance.compose.design.terminal", "Terminál"),
  chips: () => t("appearance.compose.design.chips", "Kapsle"),
  glass: () => t("appearance.compose.design.glass", "Sklo"),
  sunset: () => t("appearance.compose.design.sunset", "Západ slunce"),
  tall: () => t("appearance.compose.design.tall", "Vysoký výřez, drobné písmo"),
  cubes: () => t("appearance.compose.design.cubes", "Barevné kostky"),
  aurora: () => t("appearance.compose.design.aurora", "Polární záře"),
  embers: () => t("appearance.compose.design.embers", "Žhavá římsa"),
  comet: () => t("appearance.compose.design.comet", "Kometa"),
  hardware: () => t("appearance.compose.design.hardware", "Výkon: CPU · RAM · GPU"),
  hwbar: () => t("appearance.compose.design.hwbar", "Výřez: Claude + CPU · RAM · GPU"),
  cyber: () => t("appearance.compose.design.cyber", "Cyber HUD"),
  hwrings: () => t("appearance.compose.design.hwrings", "Prstence: CPU · RAM · GPU"),
  devtools: () => t("appearance.compose.design.devtools", "Vývojářský kokpit"),
  groove: () => t("appearance.compose.design.groove", "Hudba a výkon"),
  claudefocus: () => t("appearance.compose.design.claudefocus", "Claude Focus"),
  matrix: () => t("appearance.compose.design.matrix", "Matrix HUD"),
  zen: () => t("appearance.compose.design.zen", "Čistý zen"),
};

type DesignCat = "all" | "hw" | "claude" | "fx" | "clean";
const DESIGN_CATS: readonly DesignCat[] = ["all", "hw", "claude", "fx", "clean"];
const CAT_NAME: Record<DesignCat, () => string> = {
  all: () => t("appearance.compose.cat.all", "Vše"),
  hw: () => t("appearance.compose.cat.hw", "CPU · RAM · GPU"),
  claude: () => t("appearance.compose.cat.claude", "Claude a vývoj"),
  fx: () => t("appearance.compose.cat.fx", "Živé efekty"),
  clean: () => t("appearance.compose.cat.clean", "Čisté a klidné"),
};
const CAT_IDS: Record<Exclude<DesignCat, "all">, readonly DesignId[]> = {
  hw: ["hardware", "hwbar", "hwrings", "cyber", "devtools", "matrix", "groove"],
  claude: ["claudefocus", "devtools", "terminal", "islands", "hwbar", "cyber"],
  fx: ["aurora", "embers", "comet", "neon", "sunset", "matrix"],
  clean: ["glass", "chips", "tall", "cubes", "zen", "groove"],
};
const FILL_NAME: Record<FillKind, () => string> = {
  black: () => t("appearance.compose.fill.black", "Černá"),
  glass: () => t("appearance.compose.fill.glass", "Sklo"),
  tint: () => t("appearance.compose.fill.tint", "Tón akcentu"),
  gradient: () => t("appearance.compose.fill.gradient", "Přechod"),
  none: () => t("appearance.compose.fill.none", "Bez pozadí"),
};
const SEP_NAME: Record<ComposeSep, () => string> = {
  none: () => t("appearance.compose.sep.none", "Mezera"),
  gap: () => t("appearance.compose.sep.gap", "Velká mezera"),
  dot: () => t("appearance.compose.sep.dot", "Tečka ·"),
  bar: () => t("appearance.compose.sep.bar", "Čárka |"),
  pills: () => t("appearance.compose.sep.pills", "Kapsle"),
  islands: () => t("appearance.compose.sep.islands", "Ostrůvky"),
};
const DIR_NAME: Record<(typeof FILL_DIRS)[number], () => string> = {
  right: () => t("appearance.compose.dir.right", "Zleva doprava"),
  down: () => t("appearance.compose.dir.down", "Shora dolů"),
  diag: () => t("appearance.compose.dir.diag", "Šikmo"),
};

type Q = { c: StripCompose; commit: (c: StripCompose, key?: string) => void };

/** Rozbalovací výběr (víc než 4 volby). */
function Pick<T extends string>(q: { label: string; value: T; options: [T, string][]; set: (v: T) => void }) {
  return (
    <select aria-label={q.label} value={q.value} onChange={(e) => q.set(e.currentTarget.value as T)}>
      <For each={q.options}>{([v, l]) => <option value={v}>{l}</option>}</For>
    </select>
  );
}

/** 20 designových předvoleb s náhledem a filtrem kategorií; klik = celý vzhled (Vrátit vrátí předchozí). */
export function ComposeDesigns(q: Q & { state: PreviewState; a: Appearance; opacity: number }) {
  const [cat, setCat] = createSignal<DesignCat>("all");
  const list = () => {
    const k = cat();
    return k === "all" ? DESIGN_IDS : CAT_IDS[k];
  };
  return (
    <section class="cx-blk">
      <div class="cx-bh">
        <span>{t("appearance.compose.designs", "Hotové designy")}<small>{t("appearance.compose.designsHint", "Jeden klik přepíše celý vlastní vzhled; Vrátit ho vrátí.")}</small></span>
        <span class="cx-cats" role="tablist" aria-label={t("appearance.compose.designs", "Hotové designy")}>
          <For each={DESIGN_CATS}>
            {(k) => (
              <button type="button" role="tab" aria-selected={cat() === k} class="cx-cat" classList={{ on: cat() === k }} onClick={() => setCat(k)}>
                {CAT_NAME[k]()}
              </button>
            )}
          </For>
        </span>
      </div>
      <div class="cx-designs" role="list" aria-label={t("appearance.compose.designs", "Hotové designy")}>
        <For each={list()}>
          {(id) => {
            const active = () => sameCompose(q.c, designOf(id));
            return (
              <button class="ap-style cx-pre" classList={{ on: active() }} type="button" role="listitem" aria-pressed={active()} onClick={() => q.commit(designOf(id))}>
                <StripLookPreview look="custom" compose={designOf(id)} state={q.state} a={q.a} opacity={q.opacity} fitWidth={232} />
                <span class="ap-name">{DESIGN_NAME[id]()}</span>
              </button>
            );
          }}
        </For>
      </div>
    </section>
  );
}

/** Rám a akcent, pozadí těla, oddělení dílů a písmo. */
export function ComposeDecoBlock(q: Q) {
  const f = () => q.c.fill;
  const setFill = (patch: Partial<StripCompose["fill"]>, key = "") => q.commit({ ...q.c, fill: { ...f(), ...patch } }, key);
  return (
    <section class="cx-blk" id="f-strip.compose.deco" classList={{ hit: lit("strip.compose.deco") }}>
      <div class="cx-bh">
        <span>{t("appearance.compose.deco", "Rám a akcent")}<small>{t("appearance.compose.decoHint2", "Linka, záře nebo živý efekt hrany. Nažhavení při najetí myší se kreslí nad ní.")}</small></span>
      </div>
      <EdgeBlock c={q.c} commit={q.commit} />

      <div class="opt" id="f-strip.compose.fill" classList={{ hit: lit("strip.compose.fill") }}><span>{t("appearance.compose.fill", "Pozadí")}<small>{t("appearance.compose.fillHint", "Bez pozadí = jen obsah s jemným stínem, ať je čitelný.")}</small></span>
        <Pick label={t("appearance.compose.fill", "Pozadí")} value={f().kind} options={FILL_KINDS.map((v) => [v, FILL_NAME[v]()])} set={(v) => setFill(v === "gradient" && !f().from ? { kind: v, from: "#1e3a8a", to: "#000000" } : { kind: v })} /></div>
      <Show when={f().kind === "gradient"}>
        <div class="opt"><span>{t("appearance.compose.fillColors", "Barvy přechodu")}</span>
          <span class="cx-col">
            <input type="color" value={f().from || "#1e3a8a"} aria-label={t("appearance.compose.fillFrom", "První barva")} onInput={(e) => setFill({ from: e.currentTarget.value }, "fillFrom")} />
            <input type="color" value={f().to || "#000000"} aria-label={t("appearance.compose.fillTo", "Druhá barva")} onInput={(e) => setFill({ to: e.currentTarget.value }, "fillTo")} />
            <Show when={f().via} fallback={<button type="button" class="mini" onClick={() => setFill({ via: "#3b0764" })}>{t("appearance.compose.fillViaAdd", "+ třetí barva")}</button>}>
              <input type="color" value={f().via} aria-label={t("appearance.compose.fillVia", "Prostřední barva")} onInput={(e) => setFill({ via: e.currentTarget.value }, "fillVia")} />
              <button type="button" class="mini" onClick={() => setFill({ via: "" })}>{t("appearance.compose.edgeColor3Off", "Jen dvě")}</button>
            </Show>
          </span></div>
        <label class="opt"><span>{t("appearance.compose.fillFlow", "Pomalu přelévat")}<small>{t("appearance.compose.fillFlowHint", "Přechod se hýbe spolu s hranou: když Claude pracuje nebo čeká (nebo pořád, je-li hrana zapnutá „i v klidu“).")}</small></span>
          <input type="checkbox" checked={f().flow} onChange={(e) => setFill({ flow: e.currentTarget.checked })} /></label>
        <div class="opt"><span>{t("appearance.compose.fillDir", "Směr")}</span>
          <Seg label={t("appearance.compose.fillDir", "Směr")} value={f().dir} options={FILL_DIRS.map((v) => [v, DIR_NAME[v]()])} set={(v) => setFill({ dir: v })} /></div>
      </Show>

      <div class="opt" id="f-strip.compose.sep" classList={{ hit: lit("strip.compose.sep") }}><span>{t("appearance.compose.sep", "Oddělit části")}<small>{t("appearance.compose.sepHint", "Ostrůvky rozdělí tělo na tři kapsle: vlevo, uprostřed, vpravo.")}</small></span>
        <Pick label={t("appearance.compose.sep", "Oddělit části")} value={q.c.sep} options={COMPOSE_SEPS.map((v) => [v, SEP_NAME[v]()])} set={(v) => q.commit({ ...q.c, sep: v })} /></div>
      <label class="opt" id="f-strip.compose.mono" classList={{ hit: lit("strip.compose.mono") }}><span>{t("appearance.compose.mono", "Písmo s pevnou šířkou")}<small>{t("appearance.compose.monoHint", "Jako terminál (Cascadia Mono).")}</small></span>
        <input type="checkbox" checked={q.c.mono} onChange={(e) => q.commit({ ...q.c, mono: e.currentTarget.checked })} /></label>
    </section>
  );
}

/** Nástroje dílu pod kostkou: styl (jen co položka umí) a barva. */
export function PieceTools(q: Q & { p: Piece; styleName: Record<ItemStyle, () => string>; itemName: string }) {
  const item = (): ComposeItem => q.p.item;
  const col = () => q.p.color ?? "";
  return (
    <span class="cx-pt">
      <select aria-label={t("appearance.compose.pieceStyle", "Styl: {name}", { name: q.itemName })} value={q.p.style} onChange={(e) => q.commit(setStyle(q.c, item(), e.currentTarget.value as ItemStyle))}>
        <For each={stylesFor(item())}>{(s) => <option value={s}>{q.styleName[s]()}</option>}</For>
      </select>
      <select aria-label={t("appearance.compose.pieceColor", "Barva: {name}", { name: q.itemName })} value={col().startsWith("#") ? "#" : col()}
        onChange={(e) => q.commit(setPieceColor(q.c, item(), e.currentTarget.value === "#" ? "#ff8800" : e.currentTarget.value))}>
        <option value="">{t("appearance.compose.colorAuto", "Podle hodnoty")}</option>
        <option value="accent">{t("appearance.compose.colorAccent", "Akcent")}</option>
        <option value="#">{t("appearance.compose.colorCustom", "Vlastní")}</option>
      </select>
      <Show when={col().startsWith("#")}>
        <input type="color" value={col()} aria-label={t("appearance.compose.pieceColor", "Barva: {name}", { name: q.itemName })} onInput={(e) => q.commit(setPieceColor(q.c, item(), e.currentTarget.value), `color-${item()}`)} />
      </Show>
    </span>
  );
}
