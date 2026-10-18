// Most k Rustu pro widgety ze skriptů (`commands/widgets.rs`): seznam (sken při otevření katalogu /
// Nastavení), souhlas s přesně tím textem, který člověk viděl (obsahuje otisk obsahu), „Obnovit teď“,
// „Zobrazit výstup“, „Upravit skript“, složka, galerie příkladů a klik na položku.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { WidgetsListing, WidgetTemplate } from "../types";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const widgetsApi = {
  list: () => call<WidgetsListing>("widgets_list"),
  consent: (id: string, confirmed: string) => call<void>("widget_consent", { id, confirmed }),
  refresh: (id: string) => call<void>("widget_refresh", { id }),
  output: (id: string) => call<string | null>("widget_output", { id }),
  edit: (id: string) => call<void>("widget_edit", { id }),
  /** založí složku (výslovný klik) a otevře ji; vrací cestu */
  openDir: () => call<string>("widgets_open_dir"),
  templates: () => call<WidgetTemplate[]>("widget_templates"),
  /** zkopíruje příklad do složky (nic nespouští); vrací id nového widgetu */
  install: (file: string) => call<string>("widget_install", { file }),
  /** klik na položku; vrací hlášku (zkopírováno / výsledek skriptu) nebo null */
  click: (id: string, index: number, label: string) => call<string | null>("widget_click", { id, index, label }),
};
