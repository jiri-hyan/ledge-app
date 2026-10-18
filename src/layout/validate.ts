// Validace rozložení — zrcadlo `layout::validate` v Rustu (test `validate.test.mjs` čte stejnou tabulku
// `src-tauri/src/fixtures/layout-validate.json` jako Rust test `validation_mirror_with_ts`). Vrací pole
// první chyby ve STEJNÉM pořadí kontrol jako Rust (`layout.pages.0.cards.1.id`), nebo null.
// Hotovo v režimu úprav se podle ní zeptá dřív, než config pošle (nic se neztratí, úpravy zůstanou otevřené).
import { CARD_IDS, HEIGHTS, MAX_LIMIT, MAX_NAME, MAX_PAGES, PARTS, PERSONAL, WORK, isDynamic } from "./model.ts";

const MAX_ICON = 4;
const idOk = (id: string) => id.length >= 1 && id.length <= 24 && /^[a-z0-9-]+$/.test(id);
const chars = (s: string) => [...s].length;
const str = (x: unknown) => (typeof x === "string" ? x : "");

/** Pole první chyby rozložení (jako Rust `FieldError.field`), nebo null = platné. */
export function layoutProblem(raw: unknown): string | null {
  const l = (raw ?? {}) as { pages?: unknown; hiddenParts?: unknown };
  const pages = Array.isArray(l.pages) ? l.pages : [];
  if (!pages.length || pages.length > MAX_PAGES) return "layout.pages";
  const ids = new Set<string>();
  for (const [i, p] of pages.entries()) {
    const id = str(p?.id);
    const name = str(p?.name);
    if (!idOk(id) || ids.has(id)) return `layout.pages.${i}.id`;
    ids.add(id);
    if (chars(name) > MAX_NAME) return `layout.pages.${i}.name`;
    if (id !== WORK && id !== PERSONAL && !name.trim()) return `layout.pages.${i}.name`;
    if (chars(str(p?.icon)) > MAX_ICON) return `layout.pages.${i}.icon`;
    const seen = new Set<string>();
    for (const [j, c] of (Array.isArray(p?.cards) ? p.cards : []).entries()) {
      const cid = str(c?.id);
      if ((!(CARD_IDS as readonly string[]).includes(cid) && !isDynamic(cid)) || seen.has(cid)) return `layout.pages.${i}.cards.${j}.id`;
      seen.add(cid);
      if (c?.h != null && !(HEIGHTS as readonly unknown[]).includes(c.h)) return `layout.pages.${i}.cards.${j}.h`;
      if (c?.limit != null && !(Number.isInteger(c.limit) && c.limit >= 0 && c.limit <= MAX_LIMIT)) return `layout.pages.${i}.cards.${j}.limit`;
    }
  }
  for (const [i, h] of (Array.isArray(l.hiddenParts) ? l.hiddenParts : []).entries()) if (!(typeof h === "string" && h in PARTS)) return `layout.hiddenParts.${i}`;
  return null;
}
