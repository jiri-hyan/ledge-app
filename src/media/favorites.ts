// Oblíbené karty Právě hraje (`media.favorites`): čistá logika bez DOM a bez `t()`, test `favorites.test.mjs`.
// Ledge nemá vlastní přehrávač — dlaždice jen otevře odkaz (Spotify `spotify:…`, YouTube Music, jiný web).
// `favoriteSource` je zrcadlo Rust `mediaconf::source_of` (stejná tabulka v testech obou stran).

/** Oblíbená položka v configu (Rust `mediaconf::Favorite`). */
export interface Favorite {
  name: string;
  uri: string;
}
/** Odkud odkaz je (ikona dlaždice). */
export type FavSource = "spotify" | "ytMusic" | "web";

export const MAX_FAVORITES = 24;
export const MAX_FAVORITE_NAME = 60;
export const MAX_FAVORITE_URI = 2048;

const SPOTIFY_KINDS = ["playlist", "album", "artist", "track", "show", "episode", "user"];

/** Zdroj povoleného odkazu; null = nepovolený tvar (jiné schéma, mezera, jméno v adrese, prázdný host…). */
export function favoriteSource(uri: string): FavSource | null {
  if (!uri || [...uri].length > MAX_FAVORITE_URI || /[\s\u0000-\u001f\u007f-\u009f]/.test(uri)) return null;
  const lower = uri.toLowerCase();
  if (lower.startsWith("spotify:")) {
    const parts = lower.slice("spotify:".length).split(":");
    const [kind, id] = parts;
    return parts.length === 2 && SPOTIFY_KINDS.includes(kind) && /^[a-z0-9]{1,64}$/.test(id) ? "spotify" : null;
  }
  const m = /^https?:\/\/([^/?#]*)/.exec(lower);
  if (!m) return null;
  const authority = m[1];
  if (authority.includes("@")) return null;
  const colon = authority.lastIndexOf(":");
  let host = authority;
  if (colon >= 0) {
    if (!/^\d+$/.test(authority.slice(colon + 1))) return null;
    host = authority.slice(0, colon);
  }
  if (!host || host.startsWith(".") || host.startsWith("-") || !/^[a-z0-9.-]+$/.test(host)) return null;
  if (host === "open.spotify.com" || host === "play.spotify.com" || host === "spotify.link") return "spotify";
  if (host === "music.youtube.com") return "ytMusic";
  return "web";
}

/** Chyba rozepsané položky (text podle kódu je v `FavoriteTiles.tsx` `favErrText`), null = v pořádku. */
export type FavError = "max" | "name" | "nameLong" | "uri";

export function favoriteError(f: Favorite, list: readonly Favorite[]): FavError | null {
  if (list.length >= MAX_FAVORITES) return "max";
  if (!f.name.trim()) return "name";
  if ([...f.name.trim()].length > MAX_FAVORITE_NAME) return "nameLong";
  if (!favoriteSource(f.uri.trim())) return "uri";
  return null;
}

/** Seznam s přidanou položkou (název i odkaz oříznuté); neplatná = beze změny. */
export function withFavorite(list: readonly Favorite[], f: Favorite): Favorite[] {
  const next = { name: f.name.trim(), uri: f.uri.trim() };
  return favoriteError(next, list) ? list.slice() : [...list, next];
}

/** Seznam bez položky na `index`. */
export function withoutFavorite(list: readonly Favorite[], index: number): Favorite[] {
  return list.filter((_, i) => i !== index);
}

/** Seznam s přejmenovanou položkou; prázdný název = beze změny. */
export function renamedFavorite(list: readonly Favorite[], index: number, name: string): Favorite[] {
  const n = name.trim().slice(0, MAX_FAVORITE_NAME);
  return list.map((f, i) => (i === index && n ? { ...f, name: n } : f));
}

/** Kolikátou ze schránky (historie `clip_list`) nabídnout jako odkaz: nejnovější položka, která je odkazem
 *  na hudbu (Spotify / YouTube Music) a není zkrácený náhled. Bez historie (vypnutá) = "". */
export function musicLinkFrom(items: readonly { preview: string; atMs: number }[] | null | undefined): string {
  let best: { preview: string; atMs: number } | null = null;
  for (const it of items ?? []) {
    const url = it.preview.trim();
    const src = url.endsWith("…") ? null : favoriteSource(url);
    if ((src === "spotify" || src === "ytMusic") && (!best || it.atMs > best.atMs)) best = { preview: url, atMs: it.atMs };
  }
  return best?.preview ?? "";
}

/** Předvyplněný název z toho, co hraje: titul skladby, jinak aplikace. */
export function suggestedName(m: { title?: string; app?: string } | null | undefined): string {
  return ((m?.title || m?.app || "").trim()).slice(0, MAX_FAVORITE_NAME);
}
