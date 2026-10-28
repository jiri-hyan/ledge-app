// Ledge Studio — „Platí hned“ jako čistý model (kolo 24): co se mezi dvěma verzemi configu změnilo (cesty
// a hodnoty), časová osa relace (slučování rychlých změn téže volby, zásobník Vrátit, vrácení jedné položky
// i „vše do stavu při otevření“), lidský popisek („Vzhled proužku: Klasický → Ostrůvek“) a kdy smí
// autosave zapisovat. Bez DOM, bez `t()` — popisky voleb a hodnot dodá volající. Test `history.test.mjs`.

export type Change = { path: string; before: unknown; after: unknown };
export type Entry = { id: number; at: number; changes: Change[] };
export type History = { entries: Entry[]; next: number };

/** Rychlé změny TÝCHŽ voleb do této doby od poslední se slijí do jedné položky (táhnutí posuvníku, psaní). */
export const MERGE_MS = 2500;
/** Nejvýš tolik položek časové osy (nejstarší vypadne; „vše do stavu při otevření“ jde přes původní snímek). */
export const MAX_ENTRIES = 60;
/** Autosave: ticho po poslední změně, než se config ověří a zapíše. */
export const SAVE_DELAY_MS = 400;

const isObj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);

/** Stabilní JSON (seřazené klíče) — rovnost verzí bez ohledu na pořadí klíčů. */
export function stable(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(stable).join(",")}]`;
  if (isObj(v)) return `{${Object.keys(v).sort().filter((k) => v[k] !== undefined).map((k) => `${JSON.stringify(k)}:${stable(v[k])}`).join(",")}}`;
  return JSON.stringify(v ?? null);
}
export const same = (a: unknown, b: unknown) => stable(a) === stable(b);

/** Změněné cesty: do objektů se sestupuje, pole a hodnoty se berou celé (`layout.pages`, `projects`). */
export function diffPaths(a: unknown, b: unknown, prefix = ""): Change[] {
  if (isObj(a) && isObj(b)) {
    const out: Change[] = [];
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
    for (const k of keys) out.push(...diffPaths(a[k], b[k], prefix ? `${prefix}.${k}` : k));
    return out;
  }
  if (same(a, b)) return [];
  return [{ path: prefix, before: a, after: b }];
}

/** Nastaví hodnotu na cestě (kopie; chybějící objekty založí; `undefined` klíč smaže). */
export function setAt<T>(root: T, path: string, v: unknown): T {
  const keys = path.split(".");
  const out: any = structuredClone(root ?? {});
  let o = out;
  for (const k of keys.slice(0, -1)) {
    if (!isObj(o[k])) o[k] = {};
    o = o[k];
  }
  const last = keys[keys.length - 1];
  if (v === undefined) delete o[last];
  else o[last] = structuredClone(v);
  return out;
}

export const emptyHistory = (): History => ({ entries: [], next: 1 });

const pathsKey = (c: readonly Change[]) => c.map((x) => x.path).sort().join("|");

/** Zapsaná změna (`before` → `after` celého configu) do časové osy:
 *  - změna, která přesně vrací poslední položku (ručně zpět), ji smaže;
 *  - tytéž volby do `MERGE_MS` od poslední = jedna položka (původní „před“, nové „po“);
 *  - jinak nová položka nahoře. Beze změny = beze změny. */
export function record(h: History, before: unknown, after: unknown, now: number): History {
  const changes = diffPaths(before, after);
  if (!changes.length) return h;
  const top = h.entries[h.entries.length - 1];
  if (top && pathsKey(top.changes) === pathsKey(changes)) {
    const merged = changes.map((c) => ({ path: c.path, before: top.changes.find((x) => x.path === c.path)!.before, after: c.after }));
    const reverted = merged.every((c) => same(c.before, c.after));
    if (reverted) return { ...h, entries: h.entries.slice(0, -1) };
    if (now - top.at <= MERGE_MS) return { ...h, entries: [...h.entries.slice(0, -1), { ...top, at: now, changes: merged }] };
  }
  const entries = [...h.entries, { id: h.next, at: now, changes }];
  return { entries: entries.slice(-MAX_ENTRIES), next: h.next + 1 };
}

/** Config s vrácenou položkou `id` (jen její volby, ostatní změny zůstanou) a historie bez ní. Neznámé id = null. */
export function undoEntry(h: History, id: number, current: unknown): { cfg: unknown; history: History } | null {
  const e = h.entries.find((x) => x.id === id);
  if (!e) return null;
  let cfg = current;
  for (const c of e.changes) cfg = setAt(cfg, c.path, c.before);
  return { cfg, history: { ...h, entries: h.entries.filter((x) => x.id !== id) } };
}

/** Ctrl+Z / „Vrátit“: poslední položka. Prázdná historie = null. */
export function undoLast(h: History, current: unknown) {
  const top = h.entries[h.entries.length - 1];
  return top ? undoEntry(h, top.id, current) : null;
}

/** Popisek hodnoty pro člověka: přepínač, číslo, text (zkrácený), seznam, objekt. `named` = popisek volby
 *  z UI (`"classic"` → „Klasický“), když ho volající zná. */
export type ValueWords = { on: string; off: string; empty: string; items: (n: number) => string; edited: string };
export function valueText(v: unknown, w: ValueWords, named?: (v: string) => string | null): string {
  if (v === true) return w.on;
  if (v === false) return w.off;
  if (v === null || v === undefined || v === "") return w.empty;
  if (typeof v === "number") return String(Math.round(v * 100) / 100);
  if (typeof v === "string") {
    const n = named?.(v);
    if (n) return n;
    return v.length > 28 ? `${v.slice(0, 27)}…` : v;
  }
  if (Array.isArray(v)) return w.items(v.length);
  return w.edited;
}

/** „Vzhled proužku: Klasický → Ostrůvek“; víc voleb najednou = „Vzhled proužku a 2 další“. */
export function entryText(
  e: Entry,
  labelOf: (path: string) => string,
  w: ValueWords & { more: (n: number) => string },
  named?: (path: string, v: string) => string | null,
): string {
  const first = e.changes[0];
  const label = labelOf(first.path);
  if (e.changes.length > 1) {
    const labels = [...new Set(e.changes.map((c) => labelOf(c.path)))];
    return labels.length === 1 ? label : `${label} ${w.more(labels.length - 1)}`;
  }
  const nm = named ? (v: string) => named(first.path, v) : undefined;
  const a = valueText(first.before, w, nm);
  const b = valueText(first.after, w, nm);
  return a === w.edited && b === w.edited ? label : `${label}: ${a} → ${b}`;
}

/** Nejbližší popsaná volba nad cestou (`appearance.strip.size` → `appearance.strip`); nic = poslední část cesty. */
export function nearestLabel(path: string, has: (id: string) => boolean): string {
  const parts = path.split(".");
  for (let i = parts.length; i > 0; i--) {
    const id = parts.slice(0, i).join(".");
    if (has(id)) return id;
  }
  return path;
}

/** Smí autosave zapsat? Ne během psaní do textového pole (zapíše se při Enter / opuštění), ne s chybou
 *  ověření (neplatné se neukládá, hláška je u pole), ne během konfliktu a ne bez změny proti uloženému. */
export function canSave(s: { changed: boolean; typing: boolean; invalid: boolean; conflict: boolean; saving: boolean }): boolean {
  return s.changed && !s.typing && !s.invalid && !s.conflict && !s.saving;
}

/** Je prvek textové pole, které se zapisuje až při Enter / opuštění? (tag + typ inputu) */
export function isTextEntry(tag: string, type: string | null | undefined): boolean {
  if (tag === "TEXTAREA") return true;
  if (tag !== "INPUT") return false;
  return ["text", "number", "search", "url", "email", "password", "tel", ""].includes((type ?? "text").toLowerCase());
}
