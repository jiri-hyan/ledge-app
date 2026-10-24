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

// ---------- skupiny voleb (`fields/Group.tsx`, kolo 21) ----------
// Sbalovací skupina má VÝCHOZÍ stav (třeba „otevřená, když je karta v šuplíku“), takže paměť drží obojí:
// `id → true | false` jen pro skupiny, které uživatel sám přepnul. Nečitelná hodnota = výchozí stavy.

/** Klíč v localStorage pro skupiny (oddělený od „Pokročilé“). */
export const GROUP_KEY = "ledge.settings.groups";

/** Uložený JSON → mapa `id → otevřeno` (jen platné záznamy; nejvýš 200, ať paměť neroste). */
export function parseGroups(raw: string | null | undefined): Record<string, boolean> {
  if (!raw) return {};
  try {
    const v: unknown = JSON.parse(raw);
    if (!v || typeof v !== "object" || Array.isArray(v)) return {};
    const out: Record<string, boolean> = {};
    for (const [k, on] of Object.entries(v).slice(0, 200)) if (typeof on === "boolean" && k.length > 0 && k.length <= 64) out[k] = on;
    return out;
  } catch {
    return {};
  }
}

/** Je skupina otevřená? Uživatelova volba vyhrává nad výchozím stavem. */
export const groupOpen = (map: Record<string, boolean>, id: string, def: boolean): boolean => (id in map ? map[id] : def);

/** Mapa po přepnutí skupiny; když se volba shoduje s výchozím stavem, záznam se vypustí. */
export function withGroup(map: Record<string, boolean>, id: string, open: boolean, def: boolean): Record<string, boolean> {
  const out = { ...map };
  if (open === def) delete out[id];
  else out[id] = open;
  return out;
}
