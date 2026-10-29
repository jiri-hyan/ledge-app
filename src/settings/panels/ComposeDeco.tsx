// Výzdoba vlastního zavřeného Ledge v editoru Kostky (kolo 25, `strip.compose.deco` / `fill` / `sep` / `mono`):
// 6 designových předvoleb (jeden klik, Vrátit přes zásobník editoru), rám a akcent hrany, pozadí těla, oddělení
// dílů a nástroje dílu (styl jen z `stylesFor`, barva podle hodnoty / akcent / vlastní). Čisté úpravy jdou přes
// `commit` editoru; nic se nehýbe, nic se nestahuje.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import type { Appearance } from "../../types";
import {
  COMPOSE_EDGES, COMPOSE_SEPS, DESIGN_IDS, FILL_DIRS, FILL_KINDS,
  type ComposeEdge, type ComposeItem, type ComposeSep, type DesignId, type FillKind, type ItemStyle, type Piece, type StripCompose,
  designOf, setPieceColor, setStyle, stylesFor,
} from "../../strip/compose";
import StripLookPreview from "../previews/StripLookPreview";
import type { PreviewState } from "../appearanceModel";
import { Seg, Slider } from "./apUi";

export const DESIGN_NAME: Record<DesignId, () => string> = {
  islands: () => t("appearance.compose.design.islands", "Ostrůvky"),
  neon: () => t("appearance.compose.design.neon", "Neonová linka"),
  terminal: () => t("appearance.compose.design.terminal", "Terminál"),
  chips: () => t("appearance.compose.design.chips", "Kapsle"),
  glass: () => t("appearance.compose.design.glass", "Sklo"),
  sunset: () => t("appearance.compose.design.sunset", "Západ slunce"),
};
const EDGE_NAME: Record<ComposeEdge, () => string> = {
  none: () => t("appearance.compose.edge.none", "Žádný"),
  underline: () => t("appearance.compose.edge.underline", "Linka dole"),
  outline: () => t("appearance.compose.edge.outline", "Obrys"),
  glow: () => t("appearance.compose.edge.glow", "Záře"),
  gradient: () => t("appearance.compose.edge.gradient", "Přechod"),
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

/** Barva: výběr režimu + vlastní hex (`<input type=color>`). `modes` = pevné volby (accent, claude…). */
function ColorPick(q: { label: string; value: string; modes: [string, string][]; custom: string; set: (v: string) => void }) {
  const isHex = () => q.value.startsWith("#");
  return (
    <div class="opt">
      <span>{q.label}</span>
      <span class="cx-col">
        <Seg label={q.label} value={isHex() ? "#" : q.value} options={[...q.modes, ["#", q.custom]]} set={(v) => q.set(v === "#" ? (isHex() ? q.value : "#ff8800") : v)} />
        <Show when={isHex()}>
          <input type="color" value={q.value} aria-label={q.custom} onInput={(e) => q.set(e.currentTarget.value)} />
        </Show>
      </span>
    </div>
  );
}

/** 6 designových předvoleb s náhledem; klik = celý vzhled (Vrátit vrátí předchozí). */
export function ComposeDesigns(q: Q & { state: PreviewState; a: Appearance; opacity: number }) {
  return (
    <section class="cx-blk">
      <div class="cx-bh">
        <span>{t("appearance.compose.designs", "Hotové designy")}<small>{t("appearance.compose.designsHint", "Jeden klik přepíše celý vlastní vzhled; Vrátit ho vrátí.")}</small></span>
      </div>
      <div class="cx-designs" role="list" aria-label={t("appearance.compose.designs", "Hotové designy")}>
        <For each={DESIGN_IDS}>
          {(id) => (
            <button class="ap-style cx-pre" type="button" role="listitem" onClick={() => q.commit(designOf(id))}>
              <StripLookPreview look="custom" compose={designOf(id)} state={q.state} a={q.a} opacity={q.opacity} />
              <span class="ap-name">{DESIGN_NAME[id]()}</span>
            </button>
          )}
        </For>
      </div>
    </section>
  );
}

/** Rám a akcent, pozadí těla, oddělení dílů a písmo. */
export function ComposeDecoBlock(q: Q) {
  const d = () => q.c.deco;
  const f = () => q.c.fill;
  const setDeco = (patch: Partial<StripCompose["deco"]>, key = "") => q.commit({ ...q.c, deco: { ...d(), ...patch } }, key);
  const setFill = (patch: Partial<StripCompose["fill"]>, key = "") => q.commit({ ...q.c, fill: { ...f(), ...patch } }, key);
  const custom = t("appearance.compose.colorCustom", "Vlastní");
  return (
    <section class="cx-blk">
      <div class="cx-bh">
        <span>{t("appearance.compose.deco", "Rám a akcent")}<small>{t("appearance.compose.decoHint", "Stálá linka nebo záře v barvě, kterou si vybereš. Nažhavení při najetí myší se kreslí nad ní.")}</small></span>
      </div>
      <div class="opt"><span>{t("appearance.compose.edge", "Akcent hrany")}</span>
        <Pick label={t("appearance.compose.edge", "Akcent hrany")} value={d().edge} options={COMPOSE_EDGES.map((v) => [v, EDGE_NAME[v]()])} set={(v) => setDeco({ edge: v })} /></div>
      <Show when={d().edge !== "none"}>
        <Show when={d().edge !== "outline"}>
          <div class="opt"><span>{t("appearance.compose.len", "Délka linky")}</span>
            <Seg label={t("appearance.compose.len", "Délka linky")} value={d().len} options={[["center", t("appearance.compose.len.center", "Střed 40 %")], ["full", t("appearance.compose.len.full", "Celá šířka")]]} set={(v) => setDeco({ len: v })} /></div>
        </Show>
        <ColorPick label={t("appearance.compose.edgeColor", "Barva")} value={d().color} custom={custom}
          modes={[["accent", t("appearance.compose.colorAccent", "Akcent")], ["claude", t("appearance.compose.colorClaude", "Podle Clauda")]]} set={(v) => setDeco({ color: v }, "edgeColor")} />
        <Show when={d().edge === "gradient"}>
          <div class="opt"><span>{t("appearance.compose.edgeColor2", "Druhá barva")}</span>
            <input type="color" value={d().color2 || "#ffffff"} aria-label={t("appearance.compose.edgeColor2", "Druhá barva")} onInput={(e) => setDeco({ color2: e.currentTarget.value }, "edgeColor2")} /></div>
        </Show>
        <Slider label={t("appearance.compose.edgeWidth", "Tloušťka linky")} min={1} max={3} step={1} value={d().width} fmt={(v) => `${v} px`} set={(v) => setDeco({ width: v }, "edgeWidth")} />
      </Show>

      <div class="opt"><span>{t("appearance.compose.fill", "Pozadí")}<small>{t("appearance.compose.fillHint", "Bez pozadí = jen obsah s jemným stínem, ať je čitelný.")}</small></span>
        <Pick label={t("appearance.compose.fill", "Pozadí")} value={f().kind} options={FILL_KINDS.map((v) => [v, FILL_NAME[v]()])} set={(v) => setFill(v === "gradient" && !f().from ? { kind: v, from: "#1e3a8a", to: "#000000" } : { kind: v })} /></div>
      <Show when={f().kind === "gradient"}>
        <div class="opt"><span>{t("appearance.compose.fillColors", "Barvy přechodu")}</span>
          <span class="cx-col">
            <input type="color" value={f().from || "#1e3a8a"} aria-label={t("appearance.compose.fillFrom", "První barva")} onInput={(e) => setFill({ from: e.currentTarget.value }, "fillFrom")} />
            <input type="color" value={f().to || "#000000"} aria-label={t("appearance.compose.fillTo", "Druhá barva")} onInput={(e) => setFill({ to: e.currentTarget.value }, "fillTo")} />
          </span></div>
        <div class="opt"><span>{t("appearance.compose.fillDir", "Směr")}</span>
          <Seg label={t("appearance.compose.fillDir", "Směr")} value={f().dir} options={FILL_DIRS.map((v) => [v, DIR_NAME[v]()])} set={(v) => setFill({ dir: v })} /></div>
      </Show>

      <div class="opt"><span>{t("appearance.compose.sep", "Oddělit části")}<small>{t("appearance.compose.sepHint", "Ostrůvky rozdělí tělo na tři kapsle: vlevo, uprostřed, vpravo.")}</small></span>
        <Pick label={t("appearance.compose.sep", "Oddělit části")} value={q.c.sep} options={COMPOSE_SEPS.map((v) => [v, SEP_NAME[v]()])} set={(v) => q.commit({ ...q.c, sep: v })} /></div>
      <label class="opt"><span>{t("appearance.compose.mono", "Písmo s pevnou šířkou")}<small>{t("appearance.compose.monoHint", "Jako terminál (Cascadia Mono).")}</small></span>
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
