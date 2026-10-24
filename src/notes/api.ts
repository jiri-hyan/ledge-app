// Most k jádru pro Lístky 2.0 (Rust `commands/notelinks.rs`, `commands/pins.rs`): barva a štítky poznámky,
// vazby `[[…]]`, otevření poznámky jako lístku, vzhled pro všechny lístky, „Schovat do…“. Mimo Tauri no-op
// (vzor `shelf/api.ts`).
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { SaveOutcome } from "../types";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const notesApi = {
  /** barva (`""` = výchozí) a / nebo štítky (celý seznam frontmatteru) s otiskem; jinde změněná = `changed` */
  setMeta: (path: string, expectedHash: string, patch: { color?: string; tags?: string[] }) => call<SaveOutcome>("note_set_meta", { path, expectedHash, patch }),
  /** štítky nehotových poznámek s počty (nejčastější první) */
  tags: () => call<{ tag: string; count: number }[]>("note_tags"),
  /** cesta poznámky pro `[[name]]`, `null` = neexistuje */
  linkResolve: (name: string) => call<string | null>("note_link_resolve", { name }),
  linkSuggest: (query: string, limit?: number) => call<{ title: string; path: string }[]>("note_link_suggest", { query, limit }),
  /** nová obecná poznámka s titulkem `name` (nebo existující se stejným názvem); vrací cestu */
  linkCreate: (name: string) => call<string>("note_link_create", { name }),
  /** poznámka jako lístek na plochu (bez fokusu); už otevřený nechá */
  pinOpen: (path: string) => call("pin_open", { path }),
  /** z okna lístku: jeho vzhled dostanou všechny lístky i každý nový; vrací počet připnutých */
  pinLookAll: () => call<number>("pin_look_all"),
  /** z okna lístku: schovat do `until` (lokální `YYYY-MM-DDTHH:MM`) */
  pinHide: (until: string) => call("pin_hide", { until }),
};
