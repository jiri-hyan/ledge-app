// Průběh skladby a vzhledy karty Právě hraje: čistá logika (bez DOM a bez `t()`), test `timeline.test.mjs`.
// Rust (`mediatime.rs`) posílá jen poslední hlášenou pozici s časem hlášení; tady se z ní dopočítá
// pozice „teď“ — jen když je karta vidět a hraje (jinak nic netiká). Zdroj bez timeline = null → lišta
// se neukáže, nic se nevymýšlí.

/** Zrcadlo Rust `mediatime::Timeline` (ms; `updatedMs` = unixový čas hlášení). */
export interface MediaTimeline {
  positionMs: number;
  durationMs: number;
  updatedMs: number;
  rate: number;
  canSeek: boolean;
}
export type MediaRepeat = "none" | "track" | "list";
/** `media.variant` v configu (Rust `mediaconf::Variant`). */
export type MediaVariant = "compact" | "card" | "cover" | "vinyl" | "strip";
export type MediaStripTime = "auto" | "on" | "off";
/** Sekce `media` v configu (snapshot `personal.mediaLook`). */
export interface MediaLook {
  variant: MediaVariant;
  stripTime: MediaStripTime;
  equalizer: boolean;
  /** oblíbené (Rust `mediaconf::Favorite`); prázdné se neposílá */
  favorites?: import("./favorites").Favorite[];
}

/** Pořadí vzhledů v přepínači (cyklí se tímhle směrem). */
export const VARIANTS: readonly MediaVariant[] = ["compact", "card", "cover", "vinyl", "strip"];
export const DEFAULT_VARIANT: MediaVariant = "card";

/** Vzhled ze snapshotu; chybí (staré jádro) / neznámý = výchozí karta s průběhem. */
export function variantOf(look: { variant?: unknown } | null | undefined): MediaVariant {
  const v = look?.variant;
  return VARIANTS.includes(v as MediaVariant) ? (v as MediaVariant) : DEFAULT_VARIANT;
}

/** Další vzhled v pořadí `VARIANTS` (dokola). */
export const nextVariant = (v: MediaVariant): MediaVariant => VARIANTS[(VARIANTS.indexOf(v) + 1) % VARIANTS.length];

/** Opakování dokola: vypnuto → celý seznam → jedna skladba → vypnuto. */
export const nextRepeat = (r: MediaRepeat): MediaRepeat => (r === "none" ? "list" : r === "list" ? "track" : "none");

/**
 * Pozice v čase `now`: hraje = poslední pozice + uplynulý čas × rychlost, pauza = stojí; ořez na
 * 0…délka. (Zrcadlo Rust `mediatime::position_at`.)
 */
export function positionAt(tl: MediaTimeline, now: number, playing: boolean): number {
  const run = playing ? Math.max(0, now - tl.updatedMs) * (Number.isFinite(tl.rate) && tl.rate > 0 ? tl.rate : 1) : 0;
  return Math.min(tl.durationMs, Math.max(0, Math.floor(tl.positionMs + run)));
}

/**
 * Pozice po kliknutí do lišty: dokud jádro nepošle novou timeline (nebo do `holdMs`), ukazuje se cíl
 * přetočení — jinak by lišta na chvíli skočila zpět.
 */
export function shownPosition(tl: MediaTimeline, now: number, playing: boolean, seek: { ms: number; at: number } | null, holdMs = 2500): number {
  if (seek && tl.updatedMs < seek.at && now - seek.at < holdMs) {
    const run = playing ? Math.max(0, now - seek.at) * (tl.rate > 0 ? tl.rate : 1) : 0;
    return Math.min(tl.durationMs, Math.max(0, Math.floor(seek.ms + run)));
  }
  return positionAt(tl, now, playing);
}

/** Čas „1:42“, od hodiny „1:02:05“; záporné / neplatné = 0:00. */
export function fmtTime(ms: number): string {
  const s = Math.floor((Number.isFinite(ms) ? Math.max(0, ms) : 0) / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Podíl průběhu 0…1 (bez délky = 0). */
export const fraction = (pos: number, dur: number): number => (dur > 0 ? Math.min(1, Math.max(0, pos / dur)) : 0);

/** Klik / tah po liště: x v pixelech (od levého okraje lišty) → cílová pozice v ms, ořez na lištu. */
export function seekTarget(x: number, width: number, durationMs: number): number {
  if (!(width > 0) || !(durationMs > 0)) return 0;
  return Math.round(fraction(x, width) * durationMs);
}

/** Šipky na liště (klávesnice): ±5 s, Home/End na začátek/konec; jiná klávesa = null. */
export function seekKey(key: string, pos: number, durationMs: number): number | null {
  const step = 5000;
  const to = key === "ArrowRight" || key === "ArrowUp" ? pos + step : key === "ArrowLeft" || key === "ArrowDown" ? pos - step : key === "Home" ? 0 : key === "End" ? durationMs : null;
  return to == null ? null : Math.min(durationMs, Math.max(0, to));
}

/**
 * Za kolik ms se změní zobrazená sekunda (další tik časovače karty); nehraje = null (nic netiká).
 * Rychlost > 1 = dřív.
 */
export function nextTickIn(pos: number, rate: number, playing: boolean): number | null {
  if (!playing) return null;
  const r = Number.isFinite(rate) && rate > 0 ? rate : 1;
  return Math.max(50, Math.ceil((1000 - (pos % 1000)) / r) + 5);
}

/** Smí se něco hýbat (deska, ekvalizér, tikání času)? Hraje a šuplík je vidět. */
export const live = (playing: boolean, shown: boolean): boolean => playing && shown;
