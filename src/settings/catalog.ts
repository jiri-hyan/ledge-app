// Katalog Nastavení: stránky z `registry.ts` (názvy, kw) s komponentami a seznam všeho, co jde najít
// (stránky + volby z `labels.ts` + prefixy klíčů configu z `placement.ts`). Stránka = složení sekcí
// z `panels/*` (jedna volba = jedno místo, hlídá `settings.test.mjs`); do částí Ledge Studia je třídí `studio/parts.ts`.
import type { Component } from "solid-js";
import { lang, t } from "../i18n";
import { PANEL_META, type PanelMeta } from "./registry";
import type { SmartItem } from "./searchModel";
import type { PanelProps } from "./types";
import { PLACEMENT, isAdvancedField } from "./placement";
import { isAdvanced } from "./appearanceModel";
import { FIELDS, fieldAnchor, fieldCs, fieldHint, fieldKw, fieldLabel } from "./labels";
import StartPanel from "./panels/Start";
import DrawerPanel from "./panels/Drawer";
import AppearancePanel from "./panels/Appearance";
import ProjectsActionsPanel from "./panels/ProjectsActions";
import ClaudePanel from "./panels/Claude";
import PersonalPanel from "./panels/Personal";
import NotesPanel from "./panels/Notes";
import BehaviorPanel from "./panels/Behavior";
import DeskPanel from "./panels/Desk";
import StartupPanel from "./panels/Startup";
import AppPanel from "./panels/AppPanel";

/** `component` chybí jen u `doporucene` — vede na domov Studia („Tvůj Ledge“). */
export type PanelEntry = PanelMeta & { component?: Component<PanelProps> };

const COMPONENTS: Record<string, Component<PanelProps>> = {
  zaciname: StartPanel,
  suplik: DrawerPanel,
  vzhled: AppearancePanel,
  akce: ProjectsActionsPanel,
  claude: ClaudePanel,
  osobni: PersonalPanel,
  poznamky: NotesPanel,
  chovani: BehaviorPanel,
  naplose: DeskPanel,
  spousteni: StartupPanel,
  aplikace: AppPanel,
};

/** Všechny stránky (Studio je skládá do částí podle `studio/parts.ts`). */
export const PANELS: PanelEntry[] = PANEL_META.map((m) => ({ ...m, component: COMPONENTS[m.id] }));

export const panelLabel = (p: PanelMeta) => t(p.key, p.cs);

/** V jiném jazyce než češtině se hledá i v české podobě (návod, starší zvyk, smíšené UI). */
const csToo = (cs: string) => (lang() === "cs" ? "" : ` ${cs}`);

/** Vše, co jde najít: stránky (název + klíčová slova), volby (+ klíč configu a štítek Pokročilé) a prefixy
 *  klíčů configu, které nemají vlastní volbu (najde je jen klíč) — v textu aktuálního jazyka (čte `lang()`,
 *  takže memo v `Settings` se po přepnutí jazyka přepočítá). */
export function searchItems(): SmartItem[] {
  const panels: SmartItem[] = PANELS.map((p) => ({ panel: p.id, anchor: "", label: panelLabel(p), hint: "", kw: t(p.kw[0], p.kw[1]) + csToo(`${p.cs} ${p.kw[1]}`) }));
  const fields: SmartItem[] = Object.entries(FIELDS).map(([id, f]) => ({
    panel: f.panel,
    anchor: fieldAnchor(id),
    label: fieldLabel(id),
    hint: fieldHint(id),
    kw: fieldKw(id) + csToo(fieldCs(id)),
    key: id,
    adv: f.panel === "vzhled" ? isAdvanced(fieldAnchor(id)) : isAdvancedField(id),
  }));
  const keys: SmartItem[] = PLACEMENT.filter(([k]) => !FIELDS[k]).map(([k, page, level]) => {
    const p = PANELS.find((x) => x.id === page);
    return { panel: page, anchor: "", label: p ? panelLabel(p) : page, hint: k, kw: "", key: k, adv: level === "advanced", keyOnly: true };
  });
  return [...panels, ...fields, ...keys];
}
