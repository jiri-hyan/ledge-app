// Převody jednotek v paletě (Ctrl+K): „<číslo> <jednotka> <spojka> <jednotka>“ — `10 km na mi`,
// `5 ft in cm`, `100 °F na °C`, `3 GB v MB`, `60 km/h na m/s`. Spojky cs na / v / ve / do, en to / in,
// i `->`, `→`, `=`. Délka, hmotnost, teplota, objem, plocha, data (B…TB po 1000, KiB…TiB po 1024,
// bity), čas a rychlost. Měny ne (chtěly by síť). Jména jednotek bez ohledu na velikost písmen a
// diakritiku (`tyden` = `týden`); objem v amerických jednotkách (gal, qt, pt, fl oz, cup). Čistá
// logika bez DOM — `units.test.mjs`.
import { tIn, type Lang } from "../i18n.ts";
import { formatNum } from "./calc.ts";

export type Dim = "length" | "mass" | "temp" | "volume" | "area" | "data" | "time" | "speed";

/**
 * Jednotka: `base = (hodnota + off) · f` v základní jednotce rozměru (m, kg, K, l, m², B, s, m/s).
 * `sym` je značka (a anglický tvar); `key` + `cs` = značka, která se v češtině píše jinak (týd., r.).
 */
export type Unit = { sym: string; key?: string; cs?: string; dim: Dim; f: number; off?: number; names: string };

const IN = 0.0254;
const FT = 0.3048;
const YD = 0.9144;
const MI = 1609.344;
const GAL = 3.785411784;
const DAY = 86400;

export const UNITS: readonly Unit[] = [
  // délka (m)
  { sym: "mm", dim: "length", f: 1e-3, names: "milimetr milimetru milimetry millimeter millimeters millimetre" },
  { sym: "cm", dim: "length", f: 1e-2, names: "centimetr centimetru centimetry centimeter centimeters centimetre" },
  { sym: "dm", dim: "length", f: 0.1, names: "decimetr decimetru decimetry" },
  { sym: "m", dim: "length", f: 1, names: "metr metru metry meter meters metre metres" },
  { sym: "km", dim: "length", f: 1e3, names: "kilometr kilometru kilometry kilometer kilometers kilometre" },
  { sym: "in", dim: "length", f: IN, names: "inch inches palec palce palcu \"" },
  { sym: "ft", dim: "length", f: FT, names: "foot feet stopa stopy stop '" },
  { sym: "yd", dim: "length", f: YD, names: "yard yards yardu yardy" },
  { sym: "mi", dim: "length", f: MI, names: "mile miles mil mili" },
  { sym: "nmi", dim: "length", f: 1852, names: "nautical" },
  // hmotnost (kg)
  { sym: "mg", dim: "mass", f: 1e-6, names: "miligram miligramu miligramy milligram milligrams" },
  { sym: "g", dim: "mass", f: 1e-3, names: "gram gramu gramy grams" },
  { sym: "dkg", dim: "mass", f: 1e-2, names: "dag dekagram dekagramu dekagramy deka" },
  { sym: "kg", dim: "mass", f: 1, names: "kilo kila kilogram kilogramu kilogramy kilograms" },
  { sym: "t", dim: "mass", f: 1e3, names: "tuna tuny tun tonne tonnes" },
  { sym: "oz", dim: "mass", f: 0.028349523125, names: "ounce ounces unce" },
  { sym: "lb", dim: "mass", f: 0.45359237, names: "lbs pound pounds libra libry liber" },
  // teplota (K)
  { sym: "°C", dim: "temp", f: 1, off: 273.15, names: "c ℃ celsius" },
  { sym: "°F", dim: "temp", f: 5 / 9, off: 459.67, names: "f ℉ fahrenheit" },
  { sym: "K", dim: "temp", f: 1, names: "kelvin kelvinu kelviny" },
  // objem (l)
  { sym: "ml", dim: "volume", f: 1e-3, names: "mililitr mililitru mililitry milliliter milliliters millilitre" },
  { sym: "cl", dim: "volume", f: 1e-2, names: "centilitr" },
  { sym: "dl", dim: "volume", f: 0.1, names: "decilitr decilitru decilitry deciliter" },
  { sym: "l", dim: "volume", f: 1, names: "litr litru litry liter liters litre litres" },
  { sym: "hl", dim: "volume", f: 100, names: "hektolitr hektolitru hektolitry" },
  { sym: "m³", dim: "volume", f: 1e3, names: "m3 kubik kubiku kubiky" },
  { sym: "cm³", dim: "volume", f: 1e-3, names: "cm3 ccm cc" },
  { sym: "gal", dim: "volume", f: GAL, names: "gallon gallons galon galonu galony" },
  { sym: "qt", dim: "volume", f: GAL / 4, names: "quart quarts" },
  { sym: "pt", dim: "volume", f: GAL / 8, names: "pint pints pinta pinty" },
  { sym: "cup", dim: "volume", f: GAL / 16, names: "cups" },
  { sym: "fl oz", dim: "volume", f: GAL / 128, names: "fl.oz" },
  // plocha (m²)
  { sym: "mm²", dim: "area", f: 1e-6, names: "mm2" },
  { sym: "cm²", dim: "area", f: 1e-4, names: "cm2" },
  { sym: "m²", dim: "area", f: 1, names: "m2 sqm" },
  { sym: "km²", dim: "area", f: 1e6, names: "km2" },
  { sym: "ha", dim: "area", f: 1e4, names: "hektar hektaru hektary hectare hectares" },
  { sym: "ac", dim: "area", f: 4046.8564224, names: "acre acres akr akru akry" },
  { sym: "in²", dim: "area", f: IN * IN, names: "in2 sqin" },
  { sym: "ft²", dim: "area", f: FT * FT, names: "ft2 sqft" },
  { sym: "yd²", dim: "area", f: YD * YD, names: "yd2 sqyd" },
  { sym: "mi²", dim: "area", f: MI * MI, names: "mi2 sqmi" },
  // data (B)
  { sym: "bit", dim: "data", f: 1 / 8, names: "bits bity bitu" },
  { sym: "kbit", dim: "data", f: 1e3 / 8, names: "kbits kilobit kilobity" },
  { sym: "Mbit", dim: "data", f: 1e6 / 8, names: "mbits megabit megabity" },
  { sym: "Gbit", dim: "data", f: 1e9 / 8, names: "gbits gigabit gigabity" },
  { sym: "B", dim: "data", f: 1, names: "byte bytes bajt bajty bajtu" },
  { sym: "kB", dim: "data", f: 1e3, names: "kilobyte kilobytes kilobajt kilobajty kilobajtu" },
  { sym: "MB", dim: "data", f: 1e6, names: "megabyte megabytes megabajt megabajty megabajtu mega" },
  { sym: "GB", dim: "data", f: 1e9, names: "gigabyte gigabytes gigabajt gigabajty gigabajtu giga" },
  { sym: "TB", dim: "data", f: 1e12, names: "terabyte terabytes terabajt terabajty terabajtu tera" },
  { sym: "PB", dim: "data", f: 1e15, names: "petabyte petabytes petabajt" },
  { sym: "KiB", dim: "data", f: 1024, names: "kibibyte kibibytes" },
  { sym: "MiB", dim: "data", f: 1024 ** 2, names: "mebibyte mebibytes" },
  { sym: "GiB", dim: "data", f: 1024 ** 3, names: "gibibyte gibibytes" },
  { sym: "TiB", dim: "data", f: 1024 ** 4, names: "tebibyte tebibytes" },
  // čas (s)
  { sym: "ms", dim: "time", f: 1e-3, names: "milisekunda milisekundy milisekund millisecond milliseconds" },
  { sym: "s", dim: "time", f: 1, names: "sec secs sekunda sekundy sekund second seconds" },
  { sym: "min", dim: "time", f: 60, names: "mins minuta minuty minut minute minutes" },
  { sym: "h", dim: "time", f: 3600, names: "hod hodina hodiny hodin hr hrs hour hours" },
  { sym: "d", dim: "time", f: DAY, names: "den dny dni dnu day days" },
  { sym: "wk", key: "palette.unit.week", cs: "týd.", dim: "time", f: 7 * DAY, names: "tyd tyden tydny tydnu week weeks" },
  // měsíc a rok jako průměr gregoriánského kalendáře (365,2425 dne)
  { sym: "mo", key: "palette.unit.month", cs: "měs.", dim: "time", f: (365.2425 * DAY) / 12, names: "mes mesic mesice mesicu month months" },
  { sym: "yr", key: "palette.unit.year", cs: "r.", dim: "time", f: 365.2425 * DAY, names: "r rok roky roku let yrs year years" },
  // rychlost (m/s)
  { sym: "m/s", dim: "speed", f: 1, names: "mps" },
  { sym: "km/h", dim: "speed", f: 1 / 3.6, names: "kmh kph km/hod" },
  { sym: "mph", dim: "speed", f: MI / 3600, names: "mi/h" },
  { sym: "ft/s", dim: "speed", f: FT, names: "fps" },
  { sym: "kn", dim: "speed", f: 1852 / 3600, names: "knot knots uzel uzly uzlu" },
];

/** Klíč jména: malými písmeny, bez diakritiky (symboly jako ° ² ³ zůstanou). */
export const unitKey = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

let index: Map<string, Unit> | null = null;
/** Jméno / symbol → jednotka (líně, jednou). Duplicitu hlídá test. */
export function unitIndex(): Map<string, Unit> {
  if (index) return index;
  index = new Map();
  for (const u of UNITS) for (const n of [u.sym.replace(/\s+/g, ""), ...u.names.split(" ")]) index.set(unitKey(n), u);
  return index;
}

const CONJ = new Set(["na", "v", "ve", "do", "to", "in", "->", "→", "=", "=>"]);

export type Conversion = { value: number; from: Unit; result: number; to: Unit };

/** Dotaz → převod, nebo `null` (není ve tvaru převodu, neznámá jednotka, různé rozměry). */
export function convert(query: string): Conversion | null {
  const s = query.trim().replace(/\s+/g, " ").replace(/^([+\-−]?)/, (m) => (m === "−" ? "-" : m));
  const m = /^([+-]?(?:\d+(?:[.,]\d+)?|[.,]\d+)) ?(\S.*)$/.exec(s.replace(/fl oz/gi, "floz"));
  if (!m) return null;
  const parts = m[2].split(" ");
  if (parts.length !== 3 || !CONJ.has(unitKey(parts[1]))) return null;
  const idx = unitIndex();
  const from = idx.get(unitKey(parts[0]));
  const to = idx.get(unitKey(parts[2]));
  if (!from || !to || from.dim !== to.dim) return null;
  const value = Number(m[1].replace(",", "."));
  const base = (value + (from.off ?? 0)) * from.f;
  const result = base / to.f - (to.off ?? 0);
  return Number.isFinite(result) ? { value, from, result, to } : null;
}

/** Značka jednotky v jazyce UI. */
export const unitSym = (u: Unit, lang: Lang) => (u.key && u.cs ? tIn(lang, u.key, u.cs) : u.sym);

/** „10 km = 6,21371192 mi“ (zobrazení) a jen číslo výsledku do schránky. */
export function conversionText(c: Conversion, lang: Lang): { title: string; copy: string } {
  const res = formatNum(c.result, lang, { sig: 9, group: true });
  return {
    title: `${formatNum(c.value, lang, { sig: 9, group: true })} ${unitSym(c.from, lang)} = ${res} ${unitSym(c.to, lang)}`,
    copy: formatNum(c.result, lang, { sig: 9 }),
  };
}
