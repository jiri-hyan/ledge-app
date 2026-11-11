// Stav API klíče pro zadání (jen „je / není“; klíč samotný se do UI nikdy nevrací a nikde se neukládá).
import { createSignal } from "solid-js";
import { api } from "../api";

/** undefined = zatím neznámé / jádro příkaz nemá (tlačítko neblokuje, případnou chybu ukáže odeslání). */
export const [hasKey, setHasKey] = createSignal<boolean | undefined>(undefined);
export async function refreshKey() {
  try {
    const v = await api.apikeyStatus();
    setHasKey(typeof v === "boolean" ? v : undefined);
  } catch {
    setHasKey(undefined);
  }
}
