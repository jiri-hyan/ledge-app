// Čistá logika pole „Město“ (Nastavení → Obsah karet → Počasí; bez DOM a Solid): kdy se smí ptát na síť
// (zrcadlo Rust `weather::clean_query`), jak se návrh jmenuje a co z něj vyplní místo pro počasí.

/** Návrh města z jádra (Rust `weather::Place`). */
export type Place = { name: string; region: string | null; country: string | null; lat: number; lon: number };

/** Prodleva po posledním stisku klávesy, než se zeptáme (Open-Meteo nemá být bombardováno při psaní). */
export const GEO_DEBOUNCE_MS = 450;
/** Kratší dotaz se neposílá (Rust `GEO_MIN_CHARS`). */
export const GEO_MIN_CHARS = 2;
/** Nejdelší název místa v configu (`personal.weather.label`). */
export const LABEL_MAX = 40;

/** Dotaz po ořezu mezer, nebo null, když je moc krátký (nic se nestahuje). */
export function geoQuery(raw: string): string | null {
  const q = raw.trim();
  return [...q].length >= GEO_MIN_CHARS ? q : null;
}

/** „Praha — Hlavní město Praha, Česko“ (kraj a země jen když jsou; stejný kraj jako název se neopakuje). */
export function placeLabel(p: Place): string {
  const tail = [p.region && p.region !== p.name ? p.region : null, p.country].filter(Boolean).join(", ");
  return tail ? `${p.name} — ${tail}` : p.name;
}

/** Hodnota `personal.weather` po výběru návrhu: souřadnice a název (ořezaný na `LABEL_MAX` znaků). */
export function placeToWeather(p: Place): { lat: number; lon: number; label: string } {
  return { lat: p.lat, lon: p.lon, label: [...p.name].slice(0, LABEL_MAX).join("") };
}
