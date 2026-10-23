// Most k jádru pro oblíbené karty Právě hraje (`src-tauri/src/commands/media.rs`). Vlastní soubor, ať se
// nesahá do sdíleného `api.ts`. Mimo Tauri no-op.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const favApi = {
  /** Otevře odkaz oblíbené položky `index` (jádro ověří, že `uri` pořád sedí s configem a že je povolený). */
  open: (index: number, uri: string) => call("media_favorite_open", { index, uri }),
};
