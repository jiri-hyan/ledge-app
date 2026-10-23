import type { ActionView, ScriptView } from "./actions";
import type { Ask, HooksStatus, LimitsView, Pending, SessionView } from "./claude";
import type { NoteKind, NoteView, NotesSyncView, PinsView } from "./notes";
import type { PersonalView } from "./personal";
import type { ProcView, ProjectView, View } from "./projects";
import type { UpdateView } from "./settings";
import type { WidgetView } from "./widgets";
import type { SectionKey } from "../cards/sections";
import type { Layout } from "../layout/model";

/** `drawer.sections` — viditelnost karet a jejich částí, ODVOZENÁ v Rustu z `layout` (seznam v `cards/sections.ts`). */
export type Sections = Record<SectionKey, boolean>;

export interface Snapshot {
  /** id záložky, která je vidět (`layout.pages[].id`; vestavěné `prace` / `osobni`) */
  mode: string;
  /** zastaralé — vždy true, záložky řídí `layout` */
  personalEnabled: boolean;
  view: View;
  cfgError: string | null;
  sections: Sections;
  /** záložky a karty na nich (`config.layout`, `layout/model.ts`) */
  layout: Layout;
  claudeEnabled: boolean;
  projects: ProjectView[];
  procs: ProcView[];
  pending: Pending[];
  /** otázky Clauda (AskUserQuestion) — typ v bloku „questions“ na konci */
  questions: Ask[];
  sessions: SessionView[];
  hooks: HooksStatus;
  notesEnabled: boolean;
  /** kompaktní paleta: jen pole, výsledky až při psaní (`palette.compact`, `palette/compact.ts`) */
  paletteCompact: boolean;
  /** karta Poznámky: nehotové připomínky do konce dneška, pak nejnovější — nejvýš 8 */
  notes: NoteView[];
  /** kolik poznámek se do karty nevešlo (zbytek přes noteList) */
  notesMore: number;
  /** první (max 3) připomínky po termínu (peek) */
  reminders: NoteView[];
  remindersCount: number;
  /** kategorie z configu (pořadí záložek) a počty nehotových podle `kind` */
  noteKinds: NoteKind[];
  notesKindCounts: Record<string, number>;
  /** roste s každým přenačtením poznámek */
  notesRev: number;
  /** kolik poznámek je v archivu (`_archiv`) */
  notesArchived: number;
  /** lístky na plochu: připnuté poznámky (a jestli průchozí) a zkratka průchozího režimu (Rust `pins::win::view`) */
  pins?: PinsView;
  /** záložka Osobní */
  personal: PersonalView;
  notesSync: NotesSyncView;

  update: UpdateView;
  /** rychlé akce (chybí jen ve starém jádře) */
  actions?: ActionView[];
  /** skripty jako dlaždice karty (jen se `scripts.inActions`; jinak chybí) */
  scripts?: ScriptView[];
  /** stránky Nastavení Windows v paletě (`palette.winSettings`; chybí = staré jádro → zapnuto) */
  paletteWinSettings?: boolean;
  /** limity Claude (chybí jen ve starém jádře) */
  limits?: LimitsView;
  /** útrata Claude (chybí jen ve starém jádře) */
  costs?: import("../costs/model").CostsView;
  /** časovač (chybí jen ve starém jádře) */
  timer?: import("../timer/model").TimerView;
  /** widgety umístěné na záložce nebo v proužku (chybí = žádné) */
  widgets?: WidgetView[];
  /** kontextové návrhy: zapnuté zdroje a GitHub z paměti (chybí jen ve starém jádře) */
  suggest?: import("./suggest").SuggestView;
}
