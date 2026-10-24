// Chytré hledání v Nastavení (kolo 21) — čistá funkce bez DOM: název, popis, klíčová slova, synonyma
// cs ↔ en („barva“ najde „color“ i „motiv“), klíč configu (`drawer.keepWarmS`, i jen `keepwarm`) a drobný
// překlep (jedna záměna / chybějící písmeno u slov od 5 znaků). Všechna slova dotazu musí sedět.
// Pořadí: začátek názvu → celý dotaz v názvu → klíč configu → popis / klíčová slova → synonymum → překlep.
// Test: `searchModel.test.mjs` (tabulka dotaz → očekávaný výsledek).
import { norm, type SearchItem } from "./model.ts";

export type SmartItem = SearchItem & {
  /** klíč configu (cesta), když volba nějaký má */
  key?: string;
  /** volba je v „Pokročilé“ (štítek ve výsledku) */
  adv?: boolean;
  /** položka jen pro hledání podle klíče configu (prefix z `placement.ts`) — slova ji nenajdou */
  keyOnly?: boolean;
};

/** Skupiny slov, která znamenají totéž (bez diakritiky, malá písmena). Slovo dotazu najde i ostatní ze skupiny. */
export const SYNONYMS: readonly (readonly string[])[] = [
  ["barva", "barvy", "color", "colour", "motiv", "theme", "akcent", "accent"],
  ["tmavy", "dark", "noc", "night"],
  ["svetly", "light"],
  ["zkratka", "zkratky", "klavesa", "klavesy", "klavesnice", "hotkey", "shortcut", "keyboard"],
  ["animace", "animation", "pohyb", "motion"],
  ["pismo", "font", "typography"],
  ["sirka", "width", "siroky", "wide"],
  ["pruhlednost", "pruhledny", "opacity", "transparency", "kryti"],
  ["jazyk", "language", "lang", "cestina", "anglictina", "english"],
  ["zaloha", "backup", "export", "import", "obnova", "restore"],
  ["aktualizace", "update", "verze", "version", "release"],
  ["pocasi", "weather", "mesto", "city"],
  ["kalendar", "calendar", "schuzka", "meeting"],
  ["hudba", "music", "prehravac", "player", "spotify"],
  ["schranka", "clipboard"],
  ["poznamka", "poznamky", "note", "notes", "pripominka", "reminder"],
  ["listek", "listky", "pin", "sticky"],
  ["projekt", "projekty", "project", "repo"],
  ["odkaz", "odkazy", "link", "links", "web"],
  ["akce", "action", "actions", "prikaz", "command"],
  ["hra", "hry", "game", "gaming", "fullscreen"],
  ["autostart", "spousteni", "startup", "boot"],
  ["pamet", "memory", "ram"],
  ["widget", "widgety", "plocha", "desktop"],
  ["oznameni", "notifikace", "notification", "toast", "upozorneni"],
  ["zvuk", "sound", "pipnuti", "beep"],
  ["limit", "limity", "kvota", "quota"],
  ["paleta", "palette", "hledani", "search"],
  ["prouzek", "strip", "lista", "notch", "island"],
  ["suplik", "drawer"],
  ["karta", "karty", "card", "cards"],
  ["zalozka", "zalozky", "tab", "tabs"],
  ["mys", "mouse", "najeti", "hover"],
  ["schvalovani", "approval", "povolit", "approve", "permission"],
  ["heslo", "tajemstvi", "secret", "token", "password"],
];

/** `drawer.keepWarmS` → „drawer.keepwarms drawer keep warm s“ (celý klíč i jeho části). */
export function keyWords(key: string): string {
  const parts = key.split(".").flatMap((p) => p.replace(/([a-z0-9])([A-Z])/g, "$1 $2").split(" "));
  return norm(`${key} ${parts.join(" ")}`);
}

/** Levenshtein ≤ 1 (záměna, vložení, vypuštění jednoho znaku). */
export function nearlyEqual(a: string, b: string): boolean {
  if (a === b) return true;
  if (Math.abs(a.length - b.length) > 1) return false;
  let i = 0;
  let j = 0;
  let edits = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      i++;
      j++;
      continue;
    }
    if (++edits > 1) return false;
    if (a.length > b.length) i++;
    else if (b.length > a.length) j++;
    else {
      i++;
      j++;
    }
  }
  return edits + (a.length - i) + (b.length - j) <= 1;
}

/** Ostatní slova ze skupin synonym, do kterých slovo dotazu patří (i jako začátek, od 3 znaků). */
export function synonymsOf(w: string, groups = SYNONYMS): string[] {
  const out = new Set<string>();
  for (const g of groups) {
    if (g.some((m) => m === w || (w.length >= 3 && m.startsWith(w)))) for (const m of g) if (m !== w) out.add(m);
  }
  return [...out];
}

/** Jak dobře sedí jedno slovo (menší = lépe; null = nesedí). */
function wordScore(w: string, it: { label: string; text: string; key: string; tokens: string[]; keyOnly: boolean }): number | null {
  // jen klíč (prefix z `placement.ts`): až za vším ostatním; `drawer.hoverdelayms` najde i prefix `drawer`
  if (it.keyOnly) return it.key.includes(w) || w.startsWith(`${it.key.split(" ")[0]}.`) ? 6 : null;
  if (it.key.includes(w)) return it.label.includes(w) ? 1 : 2;
  if (it.label.includes(w)) return 1;
  if (it.text.includes(w)) return 3;
  const syn = synonymsOf(w);
  if (syn.some((s) => it.label.includes(s) || it.text.includes(s))) return 4;
  if (w.length >= 5 && it.tokens.some((tk) => tk.length >= 4 && nearlyEqual(w, tk))) return 5;
  return null;
}

/** Výsledky hledání seřazené podle shody (stabilně — při stejné shodě pořadí katalogu). */
export function smartSearch<T extends SmartItem>(items: readonly T[], q: string): T[] {
  const words = norm(q).split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const scored: { it: T; score: number; i: number }[] = [];
  items.forEach((it, i) => {
    const label = norm(it.label);
    const text = `${norm(it.hint)} ${norm(it.kw)}`;
    const key = it.key ? keyWords(it.key) : "";
    const prep = { label, text, key, tokens: `${label} ${text}`.split(/[^a-z0-9]+/).filter(Boolean), keyOnly: !!it.keyOnly };
    let worst = 0;
    for (const w of words) {
      const s = wordScore(w, prep);
      if (s === null) return;
      worst = Math.max(worst, s);
    }
    const score = label.startsWith(words[0]) && worst <= 1 ? 0 : worst;
    scored.push({ it, score, i });
  });
  return scored.sort((a, b) => a.score - b.score || a.i - b.i).map((x) => x.it);
}
