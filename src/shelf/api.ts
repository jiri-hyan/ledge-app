// Most k jádru pro přihrádku (`src-tauri/src/commands/shelf.rs`). Mimo Tauri no-op (vzor `actions/api.ts`).
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ClipFormat, ShelfList } from "./model";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const shelfApi = {
  list: () => call<ShelfList>("shelf_list"),
  /** data URL ikony ze shellu; `null` = shell ikonu nedal (chybějící soubor) */
  icon: (path: string) => call<string | null>("shelf_icon", { path }),
  /** `paths` prázdné = celá přihrádka; vrací počet cest */
  copy: (paths: string[], format: ClipFormat) => call<number>("shelf_copy", { paths, format }),
  open: (path: string) => call("shelf_open", { path }),
  reveal: (path: string) => call("shelf_reveal", { path }),
  remove: (path: string) => call<boolean>("shelf_remove", { path }),
  clear: () => call("shelf_clear"),
  dragOut: (paths: string[]) => call("shelf_drag_out", { paths }),
  /** ze schránky: soubory jako odkazy, obrázek / text jako nový soubor v `shelf-drops`; vrací počet */
  paste: () => call<number>("shelf_paste"),
  /** text souboru z přihrádky (≤ 256 kB, UTF-8) do schránky */
  copyContent: (path: string) => call("shelf_copy_content", { path }),
  /** zmenšená kopie obrázku (PNG, delší strana ≤ 1024 px) do přihrádky; vrací její cestu */
  resize: (path: string) => call<string>("shelf_resize_image", { path, maxPx: 1024 }),
};
