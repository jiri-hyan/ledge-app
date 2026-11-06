// Čisté pomocné funkce pro hlášení chyb UI do logu (`uiErrors.ts`) — bez závislostí, aby šly testovat v node.

export const MAX_DISTINCT = 30;
export const seen = new Set<string>();

/** Zpráva k zápisu: `Název: text` z prvního řádku (stack se neposílá), jinak `String(e)`; prázdná = nic. */
export function errorLine(e: unknown, where?: string): string {
  let msg: string;
  if (e instanceof Error) msg = `${e.name}: ${e.message}`;
  else if (typeof e === "string") msg = e;
  else {
    try {
      msg = JSON.stringify(e) ?? String(e);
    } catch {
      msg = String(e);
    }
  }
  const first = msg.split(/\r?\n/)[0].trim();
  if (!first) return "";
  return where ? `${first} (${where})` : first;
}

/** Hlásit už neviděnou chybu? (po `MAX_DISTINCT` různých dál jen ty, co už znám — tj. nic nového) */
export function fresh(line: string, set: Set<string> = seen): boolean {
  if (!line || set.has(line) || set.size >= MAX_DISTINCT) return false;
  set.add(line);
  return true;
}

/** `p`, a když selže, zapíše chybu přes `log` a vrátí `fallback` (tichý pád by schoval chybu před logem). */
export async function quietOn<T, F = T>(log: (e: unknown, where: string) => void, p: Promise<T>, fallback: F, where: string): Promise<T | F> {
  try {
    return await p;
  } catch (e) {
    log(e, where);
    return fallback;
  }
}
