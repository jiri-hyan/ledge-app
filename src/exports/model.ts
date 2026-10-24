// Export útrat a limitů (kolo 20) — čistá část: je co exportovat? Zrcadlo `dataexport::has_data` v Rustu
// (jádro to ověří znovu; tohle jen zašedí položku menu). Bez importu Tauri, aby šla testovat v Node.

/** Přehled útrat (spočítaný) nebo aspoň jedno okno limitů. */
export function canExport(s: { costs?: { summary?: unknown | null } | null; limits?: { fiveHour?: unknown | null; sevenDay?: unknown | null } | null }): boolean {
  return !!s.costs?.summary || !!s.limits?.fiveHour || !!s.limits?.sevenDay;
}
