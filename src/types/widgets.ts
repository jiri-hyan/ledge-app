// --- widgets ---
/** Barva z výstupu widgetu (předvolba, Rust `widgets::output::Tone`). */
export type WidgetTone = "accent" | "text" | "muted" | "violet" | "blue" | "teal" | "green" | "amber" | "red";
/** Akce položky widgetu — jen tyhle (Rust `widgets::output::Act`). */
export type WidgetAct = { type: "open"; path: string } | { type: "copy"; text: string } | { type: "run"; script: string; arg: string | null };
export interface WidgetItem {
  label: string;
  value?: string;
  icon?: string;
  color?: WidgetTone;
  url?: string;
  action?: WidgetAct;
  /** oddělovač */
  sep?: boolean;
}
/** Co widget vypsal (Rust `widgets::output::WidgetData`). */
export interface WidgetData {
  title: string | null;
  text: string | null;
  items: WidgetItem[];
  badge: string | null;
  /** 0–1 */
  progress: number | null;
  strip: string | null;
  color: WidgetTone | null;
  /** co z výstupu nešlo použít */
  notes?: string[];
}
export type WidgetStatus = "off" | "missing" | "problem" | "consent" | "loading" | "error" | "ok" | "idle";
export type WidgetKind = "powershell" | "cmd" | "python" | "node" | "exe";
/** Widget pro UI (Rust `widgets::live::WidgetView`). */
export interface WidgetView {
  /** `widget:<soubor>` — id karty i položky proužku */
  id: string;
  file: string;
  title: string;
  icon: string | null;
  description: string | null;
  show: "card" | "strip" | "both";
  intervalS: number;
  kind: WidgetKind | null;
  status: WidgetStatus;
  problem: string | null;
  consent: { changed: boolean; note: string; confirmText: string; preview: string } | null;
  data: WidgetData | null;
  error: string | null;
  updatedMs: number | null;
  running: boolean;
  hasOutput: boolean;
  /** je položkou proužku (`strip.items`) */
  inStrip: boolean;
}
/** `widgets_list`: co je ve složce widgetů (katalog, Nastavení). */
export interface WidgetsListing {
  enabled: boolean;
  dir: string;
  exists: boolean;
  widgets: WidgetView[];
  skipped: number;
  error: string | null;
}
/** Příklad z galerie (Rust `widgets::templates::TemplateView`). */
export interface WidgetTemplate {
  file: string;
  title: string;
  description: string | null;
  icon: string | null;
  kind: WidgetKind;
  /** kam sahá po síti (null = nikam) */
  net: string | null;
  installed: boolean;
  body: string;
}
