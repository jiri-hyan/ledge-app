// Čistá logika scény tapety `wall` (E3): formát dne/data/času, zarovnání minutového časovače, interpolace
// průběhu skladby z MediaView, kontrast textu nad pozadím. Bez DOM a bez importů — `scripts/wall-build.mjs`
// tenhle soubor (po odstranění typů a `export`) vloží do `src-tauri/src/ext/wall/wall.html`; testy: model.test.mjs.
// Jen „erasable“ TypeScript (žádné enum/namespace), jinak build selže.

export type Lang = "cs" | "en";

const DAYS = {
  cs: ["NEDĚLE", "PONDĚLÍ", "ÚTERÝ", "STŘEDA", "ČTVRTEK", "PÁTEK", "SOBOTA"],
  en: ["SUNDAY", "MONDAY", "TUESDAY", "WEDNESDAY", "THURSDAY", "FRIDAY", "SATURDAY"],
};
// cs: druhý pád („15. PROSINCE 2024“), en: základní tvar („15 DECEMBER, 2024“ podle návrhu)
const MONTHS = {
  cs: ["LEDNA", "ÚNORA", "BŘEZNA", "DUBNA", "KVĚTNA", "ČERVNA", "ČERVENCE", "SRPNA", "ZÁŘÍ", "ŘÍJNA", "LISTOPADU", "PROSINCE"],
  en: ["JANUARY", "FEBRUARY", "MARCH", "APRIL", "MAY", "JUNE", "JULY", "AUGUST", "SEPTEMBER", "OCTOBER", "NOVEMBER", "DECEMBER"],
};

export function normLang(l: unknown): Lang {
  return l === "en" ? "en" : "cs";
}

/** „NEDĚLE“ / „SUNDAY“ (místní čas). */
export function formatDay(d: Date, lang: Lang): string {
  return DAYS[lang][d.getDay()];
}

/** cs „15. PROSINCE 2024“, en „15 DECEMBER, 2024“. */
export function formatDate(d: Date, lang: Lang): string {
  const m = MONTHS[lang][d.getMonth()];
  return lang === "en" ? `${d.getDate()} ${m}, ${d.getFullYear()}` : `${d.getDate()}. ${m} ${d.getFullYear()}`;
}

/** „17:32“ (24 h v obou jazycích). */
export function formatTime(d: Date): string {
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** Za kolik ms se probudit: na další celou minutu (+ rezerva, aby časovač nepřišel o setinu dřív). */
export function msToNextMinute(nowMs: number, slackMs = 50): number {
  return 60000 - (((nowMs % 60000) + 60000) % 60000) + slackMs;
}

export interface Timeline {
  positionMs: number;
  durationMs: number;
  updatedMs: number;
  rate: number;
  canSeek?: boolean;
}

export interface Bar {
  /** počáteční vyplnění 0–100 */
  startPct: number;
  /** za jak dlouho má CSS animace dojet na 100 %; 0 = stojí (pauza / konec) */
  animMs: number;
}

/**
 * Průběh z MediaView: pozice platila v `updatedMs`, hraje-li, dopočítáme od něj. Bez tiku každou sekundu —
 * stránka nastaví `startPct` a spustí jednu lineární CSS animaci na `animMs`. `null` = lištu nekreslit.
 */
export function barPlan(tl: Timeline | null | undefined, status: string, nowMs: number): Bar | null {
  if (!tl || !(tl.durationMs > 0)) return null;
  const dur = tl.durationMs;
  const rate = tl.rate > 0 ? tl.rate : 1;
  const playing = status === "playing";
  let pos = tl.positionMs;
  if (playing) pos += Math.max(0, nowMs - tl.updatedMs) * rate;
  pos = Math.min(Math.max(pos, 0), dur);
  return { startPct: (pos / dur) * 100, animMs: playing ? Math.round((dur - pos) / rate) : 0 };
}

export type Rgb = [number, number, number];

function lin(c: number): number {
  const s = c / 255;
  return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

/** Relativní jas (WCAG). */
export function luminance([r, g, b]: Rgb): number {
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** Poměr kontrastu dvou barev (1–21). */
export function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

export interface Ink {
  /** barva textu */
  rgb: Rgb;
  /** kontrast textu s pozadím */
  ratio: number;
  /** 0–1: síla stínu pod textem, když kontrast nestačí (cíl 4,5) */
  scrim: number;
}

/** Barva textu nad pozadím: bílá, nebo téměř černá — podle většího kontrastu; slabý kontrast doplní scrim. */
export function pickInk(bg: Rgb): Ink {
  const white: Rgb = [255, 255, 255];
  const dark: Rgb = [16, 20, 28];
  const cw = contrast(white, bg);
  const cd = contrast(dark, bg);
  const [rgb, ratio] = cw >= cd ? [white, cw] : [dark, cd];
  return { rgb, ratio, scrim: ratio >= 4.5 ? 0 : Math.min(1, (4.5 - ratio) / 3) };
}

/** Průměr barev (pozadí scény = pár barevných zarážek). */
export function avgRgb(stops: Rgb[]): Rgb {
  const n = stops.length || 1;
  const s = stops.reduce((a, c) => [a[0] + c[0], a[1] + c[1], a[2] + c[2]], [0, 0, 0]);
  return [Math.round(s[0] / n), Math.round(s[1] / n), Math.round(s[2] / n)];
}

/** Pozadí scény: statický přechod (bez animace — mlha stála 29 % jádra), ladí s obrázky F8: les v mlze. */
export const BG_STOPS: Rgb[] = [[34, 58, 48], [20, 36, 32], [10, 18, 18]];

export function bgCss(stops: Rgb[]): string {
  const c = (r: Rgb) => `rgb(${r[0]},${r[1]},${r[2]})`;
  return `radial-gradient(90% 70% at 50% 28%, ${c(stops[0])} 0%, ${c(stops[1])} 55%, ${c(stops[2])} 100%)`;
}

export interface MediaLike {
  title?: string;
  artist?: string;
  status?: string;
  canToggle?: boolean;
  canPrev?: boolean;
  canNext?: boolean;
  timeline?: Timeline | null;
}

export interface PlayerView {
  title: string;
  artist: string;
  playing: boolean;
  canToggle: boolean;
  canPrev: boolean;
  canNext: boolean;
}

/** Přehrávač se kreslí jen s hudbou (název a stav ≠ stopped); jinak `null` — nic vymyšleného. */
export function playerView(m: MediaLike | null | undefined): PlayerView | null {
  if (!m || !m.title || !m.status || m.status === "stopped") return null;
  return {
    title: m.title,
    artist: m.artist || "",
    playing: m.status === "playing",
    canToggle: m.canToggle !== false,
    canPrev: !!m.canPrev,
    canNext: !!m.canNext,
  };
}

// ---------- pozadí z vlastní fotky (kolo 34) ----------

/** Nejslabší text scény (čas) má `opacity` .7 — kontrast se počítá pro něj, ne pro plnou bílou. */
export const TEXT_MIN_OPACITY = 0.7;
/** Cílový kontrast textu nad fotkou (WCAG AA). */
export const TARGET_RATIO = 4.5;
/** Ztmavení i u tmavé fotky (sklo a text drží pohromadě) a strop (víc už je černá plocha). */
export const PHOTO_BASE_DIM = 0.2;
export const PHOTO_MAX_DIM = 0.85;

/** Barva ztmavená černou vrstvou o průhlednosti `a` (0–1). */
export function dimmed(c: Rgb, a: number): Rgb {
  const k = 1 - a;
  return [c[0] * k, c[1] * k, c[2] * k];
}

/** Kontrast bílého textu s krytím `opacity` nad pozadím `bg` (výsledná barva textu = smíšení s pozadím). */
export function textRatio(bg: Rgb, opacity = TEXT_MIN_OPACITY): number {
  const mix = (v: number) => v * (1 - opacity) + 255 * opacity;
  return contrast([mix(bg[0]), mix(bg[1]), mix(bg[2])], bg);
}

/** Pixely `ImageData` (RGBA) → barvy mřížky. Průhledné pixely (a < 128) se přeskočí. */
export function rgbSamples(data: ArrayLike<number>): Rgb[] {
  const out: Rgb[] = [];
  for (let i = 0; i + 3 < data.length; i += 4) if (data[i + 3] >= 128) out.push([data[i], data[i + 1], data[i + 2]]);
  return out;
}

/** Barva, na kterou se ladí kontrast: 85. percentil jasu vzorků (ne průměr — jasná mračna pod textem by prošla). */
export function repColor(samples: Rgb[]): Rgb {
  if (!samples.length) return [0, 0, 0];
  const sorted = samples.slice().sort((a, b) => luminance(a) - luminance(b));
  return sorted[Math.floor(0.85 * (sorted.length - 1))];
}

export interface PhotoPlan {
  /** síla černé vrstvy přes fotku 0–1 */
  dim: number;
  /** kontrast nejslabšího textu po ztmavení */
  ratio: number;
}

/** Nejmenší ztmavení (≥ základní), při kterém má i nejslabší text kontrast ≥ `target`; strop `PHOTO_MAX_DIM`. */
export function photoPlan(samples: Rgb[], target = TARGET_RATIO): PhotoPlan {
  const rep = repColor(samples);
  const at = (a: number) => textRatio(dimmed(rep, a));
  if (at(PHOTO_BASE_DIM) >= target) return { dim: PHOTO_BASE_DIM, ratio: at(PHOTO_BASE_DIM) };
  if (at(PHOTO_MAX_DIM) < target) return { dim: PHOTO_MAX_DIM, ratio: at(PHOTO_MAX_DIM) };
  let lo = PHOTO_BASE_DIM;
  let hi = PHOTO_MAX_DIM;
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    if (at(mid) >= target) hi = mid;
    else lo = mid;
  }
  return { dim: hi, ratio: at(hi) };
}

export type CoverAct = "keep" | "clear" | "request";

/** Obal alba: co s ním při nové hodnotě `coverKey` z MediaView. Bez klíče se obal smaže, stejný klíč se nenačítá znovu. */
export function coverPlan(prev: string | null, next: string | null | undefined): CoverAct {
  if (!next) return "clear";
  return next === prev ? "keep" : "request";
}

/** Je `u` obrázkový `data:` URL daného typu a rozumné délky? (stránka nikdy nenačítá cizí adresu) */
export function isImageUrl(u: unknown, mime: string, maxLen: number): u is string {
  return typeof u === "string" && u.length <= maxLen && u.startsWith("data:image/" + mime + ";base64,");
}
