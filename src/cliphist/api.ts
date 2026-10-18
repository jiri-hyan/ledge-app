// Most k jádru pro historii schránky (`src-tauri/src/commands/cliphist.rs`). Mimo Tauri no-op.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ClipList } from "./model";

/** aplikace pro `clipboard.excludeApps` (`cliphist::apps::AppChoice`) */
export interface AppChoice {
  exe: string;
  label: string;
  known: boolean;
}

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const clipApi = {
  list: () => call<ClipList>("clip_list"),
  /** zapíše položku do schránky (jádro to označí jako vlastní zápis — nový záznam nevznikne) */
  paste: (id: number) => call("clip_paste", { id }),
  pin: (id: number, on: boolean) => call("clip_pin", { id, on }),
  remove: (id: number) => call("clip_remove", { id }),
  /** smaže nepřipnuté */
  clear: () => call("clip_clear"),
  /** zapne historii (uloží `clipboard.enabled: true`) — prázdný stav v paletě */
  enable: () => call("clip_enable"),
  /** názvy k uloženým exe; `running` = i běžící programy (klik „Přidat běžící program“) */
  apps: (exes: string[], running: boolean) => call<{ listed: AppChoice[]; running: AppChoice[] }>("clip_apps", { exes, running }),
};
