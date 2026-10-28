// Panel Vzhled, krok „Po najetí“ (co se stane, když najedeš myší na proužek): 3 dlaždice `drawer.hover`
// (data `hover/modes.ts` od J) s HTML náhledem (`previews/HoverPreview.tsx`; co se „děje“, se přepíná
// ve scéně Studia). V „Pokročilé“ prodleva, než se malý náhled sám rozbalí do celého šuplíku
// (`drawer.hoverExpandMs`: 0 = jen klikem, jinak 400–10 000 ms; odpočítává `hover/expand.ts`). Jak se šuplík
// otevírá (najetí / přitlačení / podržení) je Šuplík a karty → Otevírání a klávesy. Najetí na dlaždici =
// náhled ve scéně Studia (`peekHoverOn`); klik platí hned.
import { For, Show } from "solid-js";
import { lang, t } from "../../i18n";
import type { HoverModeId } from "../../types";
import { HOVER_MODES } from "../../hover/modes";
import { offerUndo } from "../ctx";
import { fieldHint, fieldLabel } from "../labels";
import HoverPreview from "../previews/HoverPreview";
import { Advanced, Slider, clearPeek, lit, peekHoverOn, previewState, type Ap } from "./apUi";

/** Výchozí a rozsah `drawer.hoverExpandMs` (zrcadlo Rust `hovermode::EXPAND_MS` a ořezu v `drawer.rs`). */
const EXPAND_DEFAULT = 1200;
const EXPAND_MIN = 400;
const EXPAND_MAX = 10000;

const modeName = (x: { cs: string; en: string }) => (lang() === "en" ? x.en : x.cs);
export const normHover = (v: unknown): HoverModeId => (HOVER_MODES.some((x) => x.id === v) ? (v as HoverModeId) : "full");

/** Jednořádkový popis (literály kvůli hlídači překladů; `desc` od J je jen česky). */
const DESC: Record<HoverModeId, () => string> = {
  full: () => t("appearance.hover.full.desc", "Najetí otevře rovnou celý šuplík — jako dosud."),
  compact: () => t("appearance.hover.compact.desc", "Malý panel s tím nejdůležitějším: Claude, hudba, čas. Klik rozbalí celý šuplík."),
  island: () => t("appearance.hover.island.desc", "Proužek se roztáhne a ukáže jen to, co se právě děje, s tlačítky. Nejméně ruší."),
};

export default function AppearanceHover(q: { ap: Ap; adv: boolean; setAdv: (v: boolean) => void }) {
  const p = q.ap.p;
  const mode = () => normHover(p.cfg.drawer?.hover);
  const setMode = (v: HoverModeId) => (p.cfg.drawer ? p.setCfg("drawer", "hover", v) : p.setCfg("drawer", { hover: v }));
  const expandMs = (): number => p.cfg.drawer?.hoverExpandMs ?? EXPAND_DEFAULT;
  const setExpand = (v: number) => (p.cfg.drawer ? p.setCfg("drawer", "hoverExpandMs", v) : p.setCfg("drawer", { hoverExpandMs: v }));
  const sec = (v: number) => (lang() === "cs" ? `${(v / 1000).toFixed(1).replace(".", ",")} s` : `${(v / 1000).toFixed(1)} s`);
  const pick = (id: HoverModeId) => {
    clearPeek();
    const prev = mode();
    if (prev === id) return;
    setMode(id);
    const x = HOVER_MODES.find((m) => m.id === id);
    offerUndo(t("appearance.hover.picked", "Po najetí: {name}", { name: x ? modeName(x) : id }), () => setMode(prev));
  };
  return (
    <>
    <div class="card ap-top" id="f-drawer.hover" classList={{ hit: lit("drawer.hover") }}>
      <div class="ap-hovers" role="radiogroup" aria-label={t("appearance.hover.title", "Co se stane po najetí myší")}>
        <For each={HOVER_MODES}>
          {(x) => (
            <button class="ap-style" classList={{ on: mode() === x.id }} role="radio" aria-checked={mode() === x.id} title={DESC[x.id]()} {...peekHoverOn(x.id)} onClick={() => pick(x.id)}>
              <HoverPreview mode={x.id} state={previewState()} a={q.ap.a()} />
              <span class="ap-name">{modeName(x)}</span>
              <small class="ap-desc">{DESC[x.id]()}</small>
            </button>
          )}
        </For>
      </div>
    </div>

    <Advanced open={q.adv} onToggle={q.setAdv} hint={t("appearance.hover.advHint", "Kdy se malý náhled sám rozbalí do celého šuplíku")}>
      <div class="opt" id="f-drawer.hoverExpandMs" classList={{ hit: lit("drawer.hoverExpandMs") }}>
        <span>
          {fieldLabel("drawer.hoverExpandMs")}
          <small>{fieldHint("drawer.hoverExpandMs")}</small>
        </span>
        <button class="st-switch" classList={{ on: expandMs() > 0 }} role="switch" aria-checked={expandMs() > 0} aria-label={fieldLabel("drawer.hoverExpandMs")}
          onClick={() => setExpand(expandMs() > 0 ? 0 : EXPAND_DEFAULT)} />
      </div>
      <Show when={expandMs() > 0}>
        <Slider label={t("appearance.hover.expandAfter", "Po kolika sekundách")} min={EXPAND_MIN} max={EXPAND_MAX} step={100}
          value={Math.min(EXPAND_MAX, Math.max(EXPAND_MIN, expandMs()))} fmt={sec} set={(v) => setExpand(Math.round(v))} />
      </Show>
    </Advanced>
    </>
  );
}
