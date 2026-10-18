// Katalog Nastavení: panely z `registry.ts` (pořadí, názvy) s komponentami a seznam všeho, co jde
// najít (panely + volby z `labels.ts`). Panel = složení sekcí z `panels/*` (jedna volba = jedno místo,
// hlídá `settings.test.mjs`).
import type { Component } from "solid-js";
import { lang, t } from "../i18n";
import { PANEL_META, type PanelMeta } from "./registry";
import type { SearchItem } from "./model";
import type { PanelProps } from "./types";
import { FIELDS, fieldAnchor, fieldCs, fieldHint, fieldKw, fieldLabel } from "./labels";
import StartPanel from "./panels/Start";
import DrawerPanel from "./panels/Drawer";
import AppearancePanel from "./panels/Appearance";
import ProjectsActionsPanel from "./panels/ProjectsActions";
import ClaudePanel from "./panels/Claude";
import PersonalPanel from "./panels/Personal";
import NotesPanel from "./panels/Notes";
import BehaviorPanel from "./panels/Behavior";
import AppPanel from "./panels/AppPanel";

export type PanelEntry = PanelMeta & { component: Component<PanelProps> };

const COMPONENTS: Record<string, Component<PanelProps>> = {
  zaciname: StartPanel,
  suplik: DrawerPanel,
  vzhled: AppearancePanel,
  akce: ProjectsActionsPanel,
  claude: ClaudePanel,
  osobni: PersonalPanel,
  poznamky: NotesPanel,
  chovani: BehaviorPanel,
  aplikace: AppPanel,
};

/** Pořadí v postranní navigaci. */
export const PANELS: PanelEntry[] = PANEL_META.map((m) => ({ ...m, component: COMPONENTS[m.id] }));

export const panelLabel = (p: PanelMeta) => t(p.key, p.cs);

/** V jiném jazyce než češtině se hledá i v české podobě (návod, starší zvyk, smíšené UI). */
const csToo = (cs: string) => (lang() === "cs" ? "" : ` ${cs}`);

/** Vše, co jde najít: panely (název + klíčová slova) a volby — v textu aktuálního jazyka (čte `lang()`,
 *  takže memo v `Settings` se po přepnutí jazyka přepočítá). */
export function searchItems(): SearchItem[] {
  const panels = PANELS.map((p) => ({ panel: p.id, anchor: "", label: panelLabel(p), hint: "", kw: t(p.kw[0], p.kw[1]) + csToo(`${p.cs} ${p.kw[1]}`) }));
  const fields = Object.entries(FIELDS).map(([id, f]) => ({ panel: f.panel, anchor: fieldAnchor(id), label: fieldLabel(id), hint: fieldHint(id), kw: fieldKw(id) + csToo(fieldCs(id)) }));
  return [...panels, ...fields];
}
