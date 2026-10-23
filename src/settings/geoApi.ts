// Most k jádru pro hledání města (`src-tauri/src/commands/geo.rs` → `weather.rs` `search_places`).
// Mimo Tauri no-op (prázdný seznam). Volat jen na psaní v poli Město, nikdy v klidu.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { Place } from "./cityModel";

export const geoSearch = (query: string): Promise<Place[]> => (inTauri ? invoke<Place[]>("geo_search", { query }) : Promise.resolve([]));
