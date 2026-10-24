// Widgety na ploše (Rust `desk/`): zrcadlo serde struktur — `DeskWidget` (řádek `desk.json`), `DeskView`
// (snapshot šuplíku), `DeskData` (událost `desk-data` do okna widgetu), `DeskInit` (`desk_init`).
import type { CalendarView } from "./calendar";
import type { CostsView } from "../costs/model";
import type { LimitsView } from "./claude";
import type { ProcView } from "./projects";
import type { WidgetView } from "./widgets";

export type DeskLayer = "top" | "normal" | "desktop";

export interface DeskWidget {
  id: string;
  /** `limits` · `waiting` · `dev` · `agenda` · `week` · `clock` · `usage` · `widget:<soubor>` */
  kind: string;
  x: number | null;
  y: number | null;
  w: number;
  h: number;
  cw: number;
  ch: number;
  compact: boolean;
  layer: DeskLayer;
  locked: boolean;
  through: boolean;
  opacity: number;
  hideInGame: boolean;
}

export interface DeskView {
  widgets: DeskWidget[];
  profiles: string[];
  active: string | null;
  hiddenAll: boolean;
  max: number;
}

export interface DeskWaitRow {
  id: number;
  kind: "approve" | "question";
  project: string;
  text: string;
  since: number;
}

export interface DeskData {
  /** čas zaokrouhlený na minutu */
  now: number;
  cfgError: string | null;
  claudeEnabled: boolean;
  limits?: LimitsView;
  working: boolean;
  waiting?: DeskWaitRow[];
  procs?: ProcView[];
  projects?: { id: string; name: string }[];
  calendar?: CalendarView;
  widgets?: WidgetView[];
  /** kolo 22: útrata Claude (widget Claude dnes) */
  costs?: CostsView;
}

export interface DeskInit {
  widget: DeskWidget;
  lang: "cs" | "en";
  data: DeskData;
}
