// Příkaz rozšíření (Rust `commands/ext.rs`). Mimo Tauri (plain vite) vrací prázdný seznam.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ExtRow } from "./model.ts";

export const extApi = {
  /** Stav + změřená RAM a CPU. Měří se jen při volání → volat po 5 s a jen s viditelným blokem. */
  status: (): Promise<ExtRow[]> => (inTauri ? invoke<ExtRow[]>("ext_status") : Promise.resolve([])),
  /** Pozadí tapety z vlastní fotky: `get` = název vybrané fotky (`null` = přechod), `pick` = dialog + zmenšená kopie, `clear` = zpět přechod. */
  wallBg: (op: "get" | "pick" | "clear"): Promise<string | null> => (inTauri ? invoke<string | null>("ext_wall_bg", { op }) : Promise.resolve(null)),
};
