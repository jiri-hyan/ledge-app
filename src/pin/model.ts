// Čistá logika lístků na plochu (bez DOM a Tauri, testy `model.test.mjs`): přidání bodu / textu na
// konec těla poznámky, barvy podle průhledností (text vždy čitelný), porovnání cest a stav připnutí
// ze snapshotu šuplíku (`Snapshot.pins`, Rust `pins::win::view`).
import { normPath, samePath } from "../util/path.ts";
import { SUGGEST_MAX_CHARS, looksTimed, toLocalValue, type Suggest } from "../notes/suggest.ts";

/** Průhlednost textu nejníž sem (%) — stejně jako Rust `pins::TEXT_MIN`. */
export const TEXT_MIN = 50;
/** Pauza v psaní (ms), po které se rozepsaný text uloží do poznámky. */
export const SAVE_DELAY = 600;
export type SaveState = "idle" | "pending" | "saving" | "saved";

/** Barva pozadí a textu lístku (RGB) — tmavé sklo, světlý text. */
export const BG_RGB = "24, 26, 32";
export const FG_RGB = "244, 245, 248";

const TASK = /^\s*[-*+] \[[ xX]\]/;
const NUMBERED = /^(\s*)(\d+)([.)])\s/;
/** Řádek seznamu, který se za sebe lepí bez prázdného řádku: bod checklistu `- [ ]` nebo číslovaná položka `1.` / `1)`. */
export const isTaskLine = (l: string) => TASK.test(l) || NUMBERED.test(l);

const flat = (text: string) => text.replace(/\s+/g, " ").trim();

/** Bod checklistu z napsaného textu (víc řádků = jeden bod). Prázdný text = "". */
export function taskLine(text: string): string {
  const t = flat(text);
  return t ? `- [ ] ${t}` : "";
}

/** Řádek pro „+ bod“ za konec `body`: končí-li tělo číslovanou položkou, pokračuje číslováním (`N+1. text`,
 *  stejné odsazení a oddělovač) — šéf píše čísla; jinak bod checklistu. Prázdný text = "". */
export function pointLine(body: string, text: string): string {
  const t = flat(text);
  if (!t) return "";
  const base = body.replace(/\s+$/, "");
  const last = base.slice(base.lastIndexOf("\n") + 1);
  const m = NUMBERED.exec(last);
  return m ? `${m[1]}${Number(m[2]) + 1}${m[3]} ${t}` : `- [ ] ${t}`;
}

/** Přidá řádek(y) na konec těla. Položka za položkou (bod, číslo) jde hned pod ni (jeden seznam), cokoli
 *  jiného odděluje prázdný řádek (nový odstavec / nový seznam). Prázdné přidání tělo nezmění. */
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

// Porovnání cest (jako Rust `pins::same_note`) je sdílené v `util/path.ts`; odsud se jen re-exportuje.
export { normPath, samePath };

export type PinRef = { path: string; through: boolean };
export type PinsLike = { open: PinRef[]; key: string } | undefined | null;

export const isPinned = (pins: PinsLike, path: string) => !!pins?.open.some((p) => samePath(p.path, path));
export const anyThrough = (pins: PinsLike) => !!pins?.open.some((p) => p.through);

/** Začátek textu řádku Markdownu (odsazení, odrážka / číslo / úkol / nadpis / citace). */
const LEAD = /^(?:\s*(?:[-*+] \[[ xX]\] |[-*+] |\d+[.)] |#{1,6} |> ?))*\s*/;
const INLINE = "*_`~";

/** Řádek bez značek Markdownu + pro každý znak jeho pozice v původním řádku. */
function plainOf(line: string): { plain: string; at: number[] } {
  const at: number[] = [];
  let plain = "";
  for (let i = LEAD.exec(line)![0].length; i < line.length; i++) {
    if (INLINE.includes(line[i])) continue;
    plain += line[i];
    at.push(i);
  }
  return { plain, at };
}

/** Kam v těle poznámky dát kurzor při dvojkliku do vykresleného textu: `nodeText` je text uzlu pod
 *  kurzorem myši, `offset` pozice v něm. Najde řádek těla, který ten text obsahuje (bez značek Markdownu),
 *  a vrátí pozici v těle; `null` = nenašel (volající dá kurzor na konec). Best-effort: odkazy a tabulky
 *  nemapuje přesně, shodné řádky vezme první. */
export function caretFromText(body: string, nodeText: string, offset: number): number | null {
  const full = nodeText.replace(/\s+/g, " ");
  const needle = full.trim();
  if (!needle) return null;
  const lead = full.length - full.trimStart().length;
  const pre = nodeText.slice(0, Math.max(0, offset)).replace(/\s+/g, " ").length;
  const pos = Math.min(needle.length, Math.max(0, pre - lead));
  let start = 0;
  for (const line of body.split("\n")) {
    const { plain, at } = plainOf(line);
    const hit = plain.replace(/\s+/g, " ").indexOf(needle);
    if (hit >= 0 && plain.length === plain.replace(/\s+/g, " ").length) {
      const i = hit + pos;
      return start + (i < at.length ? at[i] : at.length ? at[at.length - 1] + 1 : line.length);
    }
    start += line.length + 1;
  }
  return null;
}

/**
 * Text, na který se po přidání bodu zeptat jádra (`note_suggest_remind`), nebo null: jen bod, který vypadá časově
 * („zítra v 10 zavolat X“), a jen poznámka BEZ připomínky (`remindMs`). Bez volání, dokud nikdo bod nepřidá.
 */
export function remindAskText(addedText: string, remindMs: number | null | undefined): string | null {
  const t = flat(addedText);
  return remindMs == null && looksTimed(t) ? t.slice(0, SUGGEST_MAX_CHARS) : null;
}

/** Chip „⏰ Připomenout …?“ ukázat jen s použitelným návrhem a dokud poznámka nemá připomínku. */
export function pinSuggestVisible(s: Suggest | null, remindMs: number | null | undefined): Suggest | null {
  return s && remindMs == null && toLocalValue(s.iso) ? s : null;
}
