// Panel Vzhled, krok „Po najetí“ (co se stane, když najedeš myší na proužek): 3 dlaždice `drawer.hover`
// (data `hover/modes.ts` od J) s HTML náhledem (`previews/HoverPreview.tsx`) a stejným přepínačem
// ukázkového dění jako krok Zavřený. Jak se šuplík otevírá (najetí / přitlačení / podržení) je chování —
// panel Chování. Klik = rozpracovaný config + „Vrátit“ v liště; platí po Uložit.
import { For } from "solid-js";
import { lang, t } from "../../i18n";
import type { HoverModeId } from "../../types";
import { HOVER_MODES } from "../../hover/modes";
import { offerUndo } from "../ctx";
import HoverPreview from "../previews/HoverPreview";
import { StateSwitch, lit, previewState, type Ap } from "./apUi";

const modeName = (x: { cs: string; en: string }) => (lang() === "en" ? x.en : x.cs);
export const normHover = (v: unknown): HoverModeId => (HOVER_MODES.some((x) => x.id === v) ? (v as HoverModeId) : "full");

/** Jednořádkový popis (literály kvůli hlídači překladů; `desc` od J je jen česky). */
const DESC: Record<HoverModeId, () => string> = {
  full: () => t("appearance.hover.full.desc", "Najetí otevře rovnou celý šuplík — jako dosud."),
  compact: () => t("appearance.hover.compact.desc", "Malý panel s tím nejdůležitějším: Claude, hudba, čas. Klik rozbalí celý šuplík."),
  island: () => t("appearance.hover.island.desc", "Proužek se roztáhne a ukáže jen to, co se právě děje, s tlačítky. Nejméně ruší."),
};

export default function AppearanceHover(q: { ap: Ap }) {
  const p = q.ap.p;
  const mode = () => normHover(p.cfg.drawer?.hover);
  const setMode = (v: HoverModeId) => (p.cfg.drawer ? p.setCfg("drawer", "hover", v) : p.setCfg("drawer", { hover: v }));
  const pick = (id: HoverModeId) => {
    const prev = mode();
    if (prev === id) return;
    setMode(id);
    const x = HOVER_MODES.find((m) => m.id === id);
    offerUndo(t("appearance.hover.picked", "Po najetí: {name} — platí po Uložit", { name: x ? modeName(x) : id }), () => setMode(prev));
  };
  return (
    <div class="card ap-top" id="f-drawer.hover" classList={{ hit: lit("drawer.hover") }}>
      <h4>{t("appearance.hover.title", "Co se stane po najetí myší")}</h4>
      <p class="ap-note">{t("appearance.hover.lead", "Mezikrok mezi zavřeným proužkem a celým šuplíkem. Klik nebo delší najetí pak vždy otevře celý šuplík.")}</p>
      <StateSwitch />
      <div class="ap-hovers" role="radiogroup" aria-label={t("appearance.hover.title", "Co se stane po najetí myší")}>
        <For each={HOVER_MODES}>
          {(x) => (
            <button class="ap-style" classList={{ on: mode() === x.id }} role="radio" aria-checked={mode() === x.id} title={DESC[x.id]()} onClick={() => pick(x.id)}>
              <HoverPreview mode={x.id} state={previewState()} a={q.ap.a()} />
              <span class="ap-name">{modeName(x)}</span>
              <small class="ap-desc">{DESC[x.id]()}</small>
            </button>
          )}
        </For>
      </div>
    </div>
  );
}
