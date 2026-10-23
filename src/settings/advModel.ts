// Čistá logika sbalitelné části „Pokročilé“ v panelech Nastavení (bez DOM a Solid; `fields/Advanced.tsx`):
// zapamatované rozbalení po panelech (localStorage jako JSON `{ "<id>": true }`) — nečitelná nebo cizí
// hodnota = všechno sbalené, nikdy výjimka. Skok z hledání na volbu uvnitř rozbalí oddíl bez zápisu do paměti.

/** Klíč v localStorage (jedna mapa pro všechny panely). */
export const ADV_KEY = "ledge.settings.advanced";

/** Uložený JSON → mapa `id → true` (jen platné záznamy; cokoli jiného se zahodí). */
export function parseAdvOpen(raw: string | null | undefined): Record<string, true> {
  if (!raw) return {};
  try {
    const v: unknown = JSON.parse(raw);
    if (!v || typeof v !== "object" || Array.isArray(v)) return {};
    const out: Record<string, true> = {};
    for (const [k, on] of Object.entries(v)) if (on === true && k.length > 0 && k.length <= 64) out[k] = true;
    return out;
  } catch {
    return {};
  }
}

/** Mapa po změně jednoho oddílu (sbalený se z mapy vypouští, ať neroste). */
export function withAdv(map: Record<string, true>, id: string, open: boolean): Record<string, true> {
  const out = { ...map };
  if (open) out[id] = true;
  else delete out[id];
  return out;
}
