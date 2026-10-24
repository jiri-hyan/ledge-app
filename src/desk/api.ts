// Most k Rustu pro widgety na ploše (`commands/desk.rs`). Ze šuplíku: připnout / odepnout / volby / profily /
// schovat vše. Z okna widgetu: start, volby, posun, ⋯ (dočasně větší okno), otevřít šuplík — Rust pozná
// widget podle labelu volajícího okna (`desk-<id>`), id se z okna neposílá.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { DeskInit, DeskLayer, DeskWidget } from "../types";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export type DeskPatch = Partial<{ layer: DeskLayer; locked: boolean; through: boolean; opacity: number; compact: boolean; hideInGame: boolean }>;

export const deskApi = {
  /** připnout druh na plochu; už připnutý se jen ukáže. Vrací id widgetu. */
  pin: (kind: string) => call<string>("desk_pin", { kind }),
  /** ze šuplíku podle id, z okna widgetu sám sebe */
  unpin: (id?: string) => call<void>("desk_unpin", { id: id ?? null }),
  set: (patch: DeskPatch, id?: string) => call<DeskWidget>("desk_set", { id: id ?? null, patch }),
  init: () => call<DeskInit>("desk_init"),
  drag: () => call<void>("desk_drag"),
  menu: (open: boolean) => call<void>("desk_menu", { open }),
  /** `waiting` = náhled se schválením v šuplíku, `settings` = Nastavení, `drawer` = celý šuplík */
  openDrawer: (target: "waiting" | "settings" | "drawer") => call<void>("desk_open_drawer", { target }),
  profileSave: (name: string) => call<string>("desk_profile_save", { name }),
  profileApply: (name: string) => call<void>("desk_profile_apply", { name }),
  profileDelete: (name: string) => call<boolean>("desk_profile_delete", { name }),
  /** `undefined` = přepnout; vrací „jsou schované“ */
  hideAll: (hide?: boolean) => call<boolean>("desk_hide_all", { hide: hide ?? null }),
};
