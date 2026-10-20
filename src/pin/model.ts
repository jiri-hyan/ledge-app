// Čistá logika lístků na plochu (bez DOM a Tauri, testy `model.test.mjs`): přidání bodu / textu na
// konec těla poznámky, barvy podle průhledností (text vždy čitelný), porovnání cest a stav připnutí
// ze snapshotu šuplíku (`Snapshot.pins`, Rust `pins::win::view`).

/** Průhlednost textu nejníž sem (%) — stejně jako Rust `pins::TEXT_MIN`. */
export const TEXT_MIN = 50;
/** Pauza v psaní (ms), po které se rozepsaný text uloží do poznámky. */
export const SAVE_DELAY = 600;
export type SaveState = "idle" | "pending" | "saving" | "saved";

/** Barva pozadí a textu lístku (RGB) — tmavé sklo, světlý text. */
export const BG_RGB = "24, 26, 32";
export const FG_RGB = "244, 245, 248";

const TASK = /^\s*[-*+] \[[ xX]\]/;
export const isTaskLine = (l: string) => TASK.test(l);

/** Bod checklistu z napsaného textu (víc řádků = jeden bod). Prázdný text = "". */
export function taskLine(text: string): string {
  const t = text.replace(/\s+/g, " ").trim();
  return t ? `- [ ] ${t}` : "";
}

/** Přidá řádek(y) na konec těla. Bod za bodem jde hned pod něj (jeden seznam), cokoli jiného
 *  odděluje prázdný řádek (nový odstavec / nový seznam). Prázdné přidání tělo nezmění. */
export function appendLine(body: string, line: string): string {
  const add = line.replace(/\r\n?/g, "\n").replace(/\s+$/, "").replace(/^\n+/, "");
  if (!add.trim()) return body;
  const base = body.replace(/\s+$/, "");
  if (!base) return add;
  const last = base.slice(base.lastIndexOf("\n") + 1);
  const tight = isTaskLine(last) && isTaskLine(add.split("\n")[0]);
  return `${base}${tight ? "\n" : "\n\n"}${add}`;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Number.isFinite(v) ? v : hi));
const r2 = (v: number) => Math.round(v * 100) / 100;

/** Barvy lístku: `bg` 0–100 % neprůhlednost pozadí, `text` `TEXT_MIN`–100 % neprůhlednost textu.
 *  Při průhledném pozadí dostane text stín — přečte se i přes světlou stránku. */
export function pinColors(bg: number, text: number) {
  const a = clamp(bg, 0, 100) / 100;
  const t = clamp(text, TEXT_MIN, 100) / 100;
  return {
    background: `rgba(${BG_RGB}, ${r2(a)})`,
    bar: `rgba(${BG_RGB}, ${r2(Math.min(1, a + 0.1))})`,
    color: `rgba(${FG_RGB}, ${r2(t)})`,
    shadow: a < 0.45 ? "0 1px 2px rgba(0, 0, 0, 0.9), 0 0 3px rgba(0, 0, 0, 0.8)" : "none",
  };
}

/** Cesta pro porovnání (jako Rust `pins::same_note`): malá písmena, zpětná lomítka. */
export const normPath = (p: string) => p.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
export const samePath = (a: string, b: string) => normPath(a) === normPath(b);

export type PinRef = { path: string; through: boolean };
export type PinsLike = { open: PinRef[]; key: string } | undefined | null;

export const isPinned = (pins: PinsLike, path: string) => !!pins?.open.some((p) => samePath(p.path, path));
export const anyThrough = (pins: PinsLike) => !!pins?.open.some((p) => p.through);
