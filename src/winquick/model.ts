// Čistá logika karty Rychlé dlaždice Windows (kolo 23, Rust `winquick.rs`): katalog dlaždic, přidání / odebrání / pořadí,
// stav rádia. Bez DOM a bez volání jádra. Test `model.test.mjs`.

/** Dlaždice, které karta umí (zrcadlo `winquick::TILE_IDS`; pořadí = výchozí podle užitku). */
export const TILE_IDS = ["mic", "out", "term", "dir", "theme", "wifi", "bt", "set", "shot"] as const;
export type TileId = (typeof TILE_IDS)[number];

export const TILE_ICON: Record<TileId, string> = {
  mic: "🎙️", out: "🎧", term: "⌨️", dir: "📁", theme: "🌓", wifi: "📶", bt: "🔷", set: "⚙️", shot: "🖼️",
};

export interface RadioInfo { kind: "wifi" | "bluetooth"; name: string; on: boolean }

export function isTile(id: string): id is TileId {
  return (TILE_IDS as readonly string[]).includes(id);
}

/** Jen známé dlaždice, bez duplicit, v zadaném pořadí. */
export function cleanTiles(tiles: readonly string[]): TileId[] {
  const out: TileId[] = [];
  for (const t of tiles) if (isTile(t) && !out.includes(t)) out.push(t);
  return out;
}

/** Dlaždice, které na kartě chybí (nabídka „Přidat“). */
export function missingTiles(tiles: readonly string[]): TileId[] {
  return TILE_IDS.filter((t) => !tiles.includes(t));
}

export function addTile(tiles: readonly string[], id: string): TileId[] {
  return cleanTiles([...tiles, id]);
}

export function removeTile(tiles: readonly string[], id: string): TileId[] {
  return cleanTiles(tiles.filter((t) => t !== id));
}

/** Posun o `delta` míst (−1 doleva, +1 doprava); na okraji beze změny. */
export function moveTile(tiles: readonly string[], id: string, delta: number): TileId[] {
  const a = cleanTiles(tiles);
  const i = a.indexOf(id as TileId);
  const j = i + delta;
  if (i < 0 || j < 0 || j >= a.length) return a;
  [a[i], a[j]] = [a[j], a[i]];
  return a;
}

/** První rádio daného druhu; žádné = `null` (dlaždice ukáže „není k dispozici“). */
export function radioOf(radios: readonly RadioInfo[], kind: "wifi" | "bt"): RadioInfo | null {
  const k = kind === "wifi" ? "wifi" : "bluetooth";
  return radios.find((r) => r.kind === k) ?? null;
}

/** Připnutá složka: bez duplicit (bez ohledu na velikost písmen a koncový zpětný lomítko), nejvýš `max`. */
export function pinFolder(folders: readonly string[], path: string, max = 8): string[] {
  const norm = (p: string) => p.replace(/[\\/]+$/, "").toLowerCase();
  const p = path.trim();
  if (!p || folders.some((f) => norm(f) === norm(p)) || folders.length >= max) return [...folders];
  return [...folders, p];
}

/** Poslední část cesty pro popisek složky. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export interface ShotView { name: string; path: string; count: number; thumb: string | null }

/** Odpověď `wq_state` (Rust `commands/winquick.rs` `WqState`). */
export interface WqState {
  tiles: string[];
  outputSwitch: boolean;
  pinned: string[];
  folders: { key: string; path: string }[];
  /** true = světlý motiv Windows */
  light: boolean;
  radios: RadioInfo[];
  shot: ShotView | null;
}
