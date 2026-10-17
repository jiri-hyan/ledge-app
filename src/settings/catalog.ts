// Katalog Nastavení: panely v pořadí navigace (vlastní + `EXTERNAL_PANELS` z registru) a seznam
// všeho, co jde najít (panely + volby z `labels.ts`).
import type { Component } from "solid-js";
import { lang, t } from "../i18n";
import { EXTERNAL_PANELS, type PanelEntry } from "./registry";
import type { SearchItem } from "./model";
import type { PanelProps } from "./types";
import { FIELDS, fieldAnchor, fieldCs, fieldHint, fieldKw, fieldLabel } from "./labels";
import ProjectsPanel from "./panels/Projects";
import DrawerPanel from "./panels/Drawer";
import ClaudePanel from "./panels/Claude";
import NotesPanel from "./panels/Notes";
import PersonalPanel from "./panels/Personal";
import HotkeysPanel from "./panels/Hotkeys";
import BackupPanel from "./panels/Backup";
import AboutPanel from "./panels/About";

type Pair = readonly [string, string];
const own = (id: string, name: Pair, kw: Pair, component: Component<PanelProps>): PanelEntry => ({ id, key: name[0], cs: name[1], kw, component });

const ext = (id: string) => EXTERNAL_PANELS.find((p) => p.id === id);

/** Pořadí v postranní navigaci. Panel z registru, který chybí, se přeskočí. */
export const PANELS: PanelEntry[] = [
  ext("vitejte"),
  own("projekty", ["settings.nav.projekty", "Projekty"], ["settings.nav.projekty.kw", "projekt dlaždice repo složka"], ProjectsPanel),
  own("suplik", ["settings.nav.suplik", "Šuplík a proužek"], ["settings.nav.suplik.kw", "chování otevírání proužek monitor prohlížeč"], DrawerPanel),
  ext("vzhled"),
  ext("akce"),
  own("claude", ["settings.nav.claude", "Claude Code"], ["settings.nav.claude.kw", "claude hooky schvalování commit"], ClaudePanel),
  ext("otazky"),
  own("poznamky", ["settings.nav.poznamky", "Poznámky"], ["settings.nav.poznamky.kw", "poznámky kategorie synchronizace git"], NotesPanel),
  own("osobni", ["settings.nav.osobni", "Osobní"], ["settings.nav.osobni.kw", "osobní počasí odkazy záložka"], PersonalPanel),
  own("zkratky", ["settings.nav.zkratky", "Zkratky"], ["settings.nav.zkratky.kw", "klávesové zkratky klávesnice"], HotkeysPanel),
  own("zaloha", ["settings.nav.zaloha", "Záloha a obnova"], ["settings.nav.zaloha.kw", "export import záloha výchozí reset config.json"], BackupPanel),
  own("oaplikaci", ["settings.nav.oaplikaci", "Aktualizace a o aplikaci"], ["settings.nav.oaplikaci.kw", "verze aktualizace log o aplikaci"], AboutPanel),
  // panely z registru, které katalog ještě nezná (přidané později), na konec
  ...EXTERNAL_PANELS.filter((p) => !["vitejte", "vzhled", "akce", "otazky"].includes(p.id)),
].filter((p): p is PanelEntry => !!p);

export const panelLabel = (p: PanelEntry) => t(p.key, p.cs);

/** V jiném jazyce než češtině se hledá i v české podobě (návod, starší zvyk, smíšené UI). */
const csToo = (cs: string) => (lang() === "cs" ? "" : ` ${cs}`);

/** Vše, co jde najít: panely (název + klíčová slova) a volby — v textu aktuálního jazyka (čte `lang()`,
 *  takže memo v `Settings` se po přepnutí jazyka přepočítá). */
export function searchItems(): SearchItem[] {
  const panels = PANELS.map((p) => ({ panel: p.id, anchor: "", label: panelLabel(p), hint: "", kw: t(p.kw[0], p.kw[1]) + csToo(`${p.cs} ${p.kw[1]}`) }));
  const fields = Object.entries(FIELDS).map(([id, f]) => ({ panel: f.panel, anchor: fieldAnchor(id), label: fieldLabel(id), hint: fieldHint(id), kw: fieldKw(id) + csToo(fieldCs(id)) }));
  return [...panels, ...fields];
}
