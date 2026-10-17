// Čistá logika palety příkazů: skládání bez diakritiky (`fold`), fuzzy skóre se zvýrazněním
// (`score`), řazení s nedávno použitými (`rank`, `pushRecent`). Bez závislostí a bez DOM — ověřuje
// ji `search.test.mjs` (`node --experimental-strip-types --test src/palette/`).

/** Text bez diakritiky, malými písmeny, se zachovanou délkou (index i = znak i originálu). */
export function fold(s: string): string {
  let out = "";
  for (const ch of s) {
    const f = ch.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
    // jeden znak → jeden znak (jinak by nesedělo zvýraznění); zvláštní případy nechat být
    out += f.length === ch.length ? f : ch.toLowerCase().length === ch.length ? ch.toLowerCase() : ch;
  }
  return out;
}

/** Rozsah zvýraznění v originálním textu: [od, do). */
export type Range = [number, number];
export type Match = { score: number; ranges: Range[] };

const isWordStart = (t: string, i: number) => i === 0 || /[\s\-_/\\.:·—(]/.test(t[i - 1]);

function pushRange(ranges: Range[], from: number, to: number) {
  const last = ranges[ranges.length - 1];
  if (last && last[1] >= from) last[1] = Math.max(last[1], to);
  else ranges.push([from, to]);
}

/** Jedno slovo dotazu v textu (oboje už složené): souvislý výskyt, jinak podposloupnost. */
function word(q: string, t: string): Match | null {
  const idx = t.indexOf(q);
  if (idx >= 0) {
    // radši výskyt na začátku slova než první výskyt uprostřed
    let at = idx;
    for (let i = idx; i >= 0; i = t.indexOf(q, i + 1)) {
      if (isWordStart(t, i)) {
        at = i;
        break;
      }
    }
    const bonus = at === 0 ? 60 : isWordStart(t, at) ? 40 : 0;
    return { score: 100 + bonus + q.length * 3 - Math.min(at, 30), ranges: [[at, at + q.length]] };
  }
  const ranges: Range[] = [];
  let score = 10;
  let ti = 0;
  let prev = -2;
  for (const ch of q) {
    // znak dotazu: přednostně začátek slova, jinak první další výskyt
    let found = -1;
    for (let i = ti; i < t.length; i++) {
      if (t[i] !== ch) continue;
      if (found < 0) found = i;
      if (isWordStart(t, i) || i === prev + 1) {
        found = i;
        break;
      }
    }
    if (found < 0) return null;
    score += found === prev + 1 ? 6 : isWordStart(t, found) ? 8 : -Math.min(found - ti, 5);
    pushRange(ranges, found, found + 1);
    prev = found;
    ti = found + 1;
  }
  return { score, ranges };
}

/** Skóre dotazu proti textu: všechna slova dotazu musí sedět (pořadí slov libovolné). `null` = ne. */
export function score(query: string, text: string): Match | null {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (!words.length) return { score: 0, ranges: [] };
  const t = fold(text);
  let total = 0;
  const ranges: Range[] = [];
  for (const w of words) {
    const m = word(w, t);
    if (!m) return null;
    total += m.score;
    ranges.push(...m.ranges);
  }
  ranges.sort((a, b) => a[0] - b[0]);
  const merged: Range[] = [];
  for (const [a, b] of ranges) pushRange(merged, a, b);
  return { score: total, ranges: merged };
}

/** Rozdělí text na úseky pro vykreslení se zvýrazněním. */
export function segments(text: string, ranges: Range[]): { text: string; hit: boolean }[] {
  const out: { text: string; hit: boolean }[] = [];
  let i = 0;
  for (const [a, b] of ranges) {
    if (a > i) out.push({ text: text.slice(i, a), hit: false });
    out.push({ text: text.slice(a, b), hit: true });
    i = b;
  }
  if (i < text.length) out.push({ text: text.slice(i), hit: false });
  return out;
}

export interface Item {
  id: string;
  title: string;
  /** druhý řádek (cesta, projekt…) — hledá se slaběji, bez zvýraznění */
  sub?: string;
  /** skrytá klíčová slova (synonyma) */
  keywords?: string;
  /** pořadí sekcí bez dotazu (menší = výš) */
  order?: number;
}
export type Ranked<T extends Item> = { item: T; score: number; ranges: Range[] };

/**
 * Výsledky pro dotaz: titul plnou vahou (se zvýrazněním), podtitul a klíčová slova slaběji.
 * Nedávno použité (`recent`, nejnovější první) dostanou bonus; bez dotazu jdou nahoru a zbytek
 * zůstane v pořadí `order`, pak vstupu.
 */
export function rank<T extends Item>(items: T[], query: string, recent: string[], limit = 50): Ranked<T>[] {
  const recentPos = new Map(recent.map((id, i) => [id, i]));
  const q = query.trim();
  const out: Ranked<T>[] = [];
  items.forEach((item, i) => {
    const r = recentPos.get(item.id);
    if (!q) {
      out.push({ item, score: r !== undefined ? 1000 - r : -(item.order ?? 0) * 1000 - i / 1000, ranges: [] });
      return;
    }
    const m = score(q, item.title);
    const extra = m ? null : score(q, `${item.sub ?? ""} ${item.keywords ?? ""}`);
    if (!m && !extra) return;
    const base = m ? m.score : (extra as Match).score * 0.6;
    out.push({ item, score: base + (r !== undefined ? Math.max(30 - r * 2, 5) : 0), ranges: m ? m.ranges : [] });
  });
  out.sort((a, b) => b.score - a.score);
  return out.slice(0, limit);
}

/** Nový seznam nedávno použitých: `id` na začátek, bez duplicit, nejvýš `max`. */
export function pushRecent(recent: string[], id: string, max = 20): string[] {
  return [id, ...recent.filter((x) => x !== id)].slice(0, max);
}
