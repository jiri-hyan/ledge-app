import type { ActionView, ScriptView } from "./actions";
import type { Ask, HooksStatus, LimitsView, Pending, QuestionLog, SessionView } from "./claude";
import type { NoteKind, NoteView, NotesFolderView, NotesSyncView, PinsView } from "./notes";
import type { PersonalView } from "./personal";
import type { ProcView, ProjectView, View } from "./projects";
import type { UpdateView } from "./settings";
import type { WidgetView } from "./widgets";
import type { DeskView } from "./desk";
import type { SectionKey } from "../cards/sections";
import type { Layout } from "../layout/model";

/** `drawer.sections` — viditelnost karet a jejich částí, ODVOZENÁ v Rustu z `layout` (seznam v `cards/sections.ts`). */
export type Sections = Record<SectionKey, boolean>;

/** Probíhající incident Anthropicu (`incidents.rs`). Název je cizí text (anglicky), stav a dopad jsou kódy. */
export interface Incident {
  name: string;
  status: "investigating" | "identified" | "monitoring" | "other";
  impact: "minor" | "major" | "critical" | "none";
  /** kolik dalších probíhajících incidentů je kromě tohoto */
  more: number;
}

/** `smart.*` pro UI: klávesy Alt+1…9 a šipky, přetažení na šuplík, kolečko nad záložkami (`smartconf.rs`). */
export interface SmartView {
  quickKeys: boolean;
  dropShortcut: boolean;
  tabWheel: boolean;
}

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
  /** upozornění na neuloženou práci (`config.gitAlerts`; chybí = starší jádro = vypnuto) */
  gitAlerts?: { enabled: boolean; read?: string };
  procs: ProcView[];
  pending: Pending[];
  /** otázky Clauda (AskUserQuestion) — typ v bloku „questions“ na konci */
  questions: Ask[];
  /** deník posledních otázek (prázdný = Rust pole vynechá) */
  questionLog?: QuestionLog[];
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
  /** widgety na ploše: připnuté widgety a profily (Rust `desk::win::view`) */
  desk?: DeskView;
  /** záložka Osobní */
  personal: PersonalView;
  notesSync: NotesSyncView;
  /** propojení poznámek se složkou */
  notesFolder: NotesFolderView;

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
  /** probíhající incident Anthropicu (`smart.incidents`; chybí = vypnuto nebo žádný) */
  incident?: Incident;
  /** přepínače chytřejšího šuplíku (`smart.*`; chybí = starší jádro = vypnuto) */
  smart?: SmartView;
  /** časovač (chybí jen ve starém jádře) */
  timer?: import("../timer/model").TimerView;
  /** widgety umístěné na záložce nebo v proužku (chybí = žádné) */
  widgets?: WidgetView[];
  /** kontextové návrhy: zapnuté zdroje a GitHub z paměti (chybí jen ve starém jádře) */
  suggest?: import("./suggest").SuggestView;
}
