// Čistá logika Nastavení (bez DOM a Solid): hledání bez diakritiky, cesty k polím configu, mapování
// chyby validace na panel, přesun v seznamu, zkratky (zrcadlo Rust `config::normalize_hotkey`).

/** Malá písmena bez diakritiky — „Šuplík“ najde i „suplik“. */
export const norm = (s: string) => s.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();

export type SearchItem = { panel: string; anchor: string; label: string; hint: string; kw: string };

/** Položky, které obsahují všechna slova dotazu; nejdřív shoda na začátku názvu, pak v názvu, pak jinde. */
export function search(items: SearchItem[], q: string): SearchItem[] {
  const words = norm(q).split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const scored: { it: SearchItem; score: number }[] = [];
  for (const it of items) {
    const label = norm(it.label);
    const all = `${label} ${norm(it.hint)} ${norm(it.kw)}`;
    if (!words.every((w) => all.includes(w))) continue;
    const score = label.startsWith(words[0]) ? 0 : words.every((w) => label.includes(w)) ? 1 : 2;
    scored.push({ it, score });
  }
  return scored.sort((a, b) => a.score - b.score).map((x) => x.it);
}

/** Úseky textu pro zvýraznění shody (`hit` = část dotazu). Porovnává bez diakritiky, vrací originál. */
export function highlight(text: string, q: string): { t: string; hit: boolean }[] {
  const words = norm(q).split(/\s+/).filter((w) => w.length > 0);
  if (!words.length) return [{ t: text, hit: false }];
  const n = norm(text);
  // NFD bez diakritiky zachovává délku jen u latinky s kombinujícími znaky → mapovat přes indexy
  const mask = new Array(n.length).fill(false);
  for (const w of words) {
    let i = n.indexOf(w);
    while (i >= 0) {
      for (let j = i; j < i + w.length; j++) mask[j] = true;
      i = n.indexOf(w, i + w.length);
    }
  }
  if (n.length !== text.length) return [{ t: text, hit: mask.some(Boolean) }];
  const out: { t: string; hit: boolean }[] = [];
  for (let i = 0; i < text.length; i++) {
    const last = out[out.length - 1];
    if (last && last.hit === mask[i]) last.t += text[i];
    else out.push({ t: text[i], hit: mask[i] });
  }
  return out;
}

/** `"projects.2.run"` → `["projects", 2, "run"]` (klíče pro setter store). */
export const pathKeys = (path: string): (string | number)[] => path.split(".").map((k) => (/^\d+$/.test(k) ? Number(k) : k));

export function getPath(obj: any, path: string): any {
  return pathKeys(path).reduce((o, k) => (o == null ? undefined : o[k]), obj);
}

/** Prefix cesty v configu → panel Nastavení; vyhrává nejdelší shoda (`drawer.glass` dřív než `drawer`).
 *  Neznámé pole = null (chyba se ukáže v liště). Pole vzhledu mimo sekci `appearance` (i celý proužek)
 *  edituje panel Vzhled, chování šuplíku panel Chování, odkazy (`personal.links`) panel akcí. */
const FIELD_PANELS: [string, string][] = [
  ["projects", "akce"],
  ["roots", "akce"],
  ["actions", "akce"],
  ["scripts", "akce"],
  ["personal.links", "akce"],
  ["drawer", "chovani"],
  ["drawer.glass", "vzhled"],
  ["drawer.width", "vzhled"],
  ["drawer.hover", "vzhled"],
  ["drawer.sections", "suplik"],
  ["layout", "suplik"],
  ["shelf", "suplik"],
  ["strip", "vzhled"],
  ["timer.strip", "vzhled"],
  ["autostart", "chovani"],
  ["hotkeys", "chovani"],
  ["palette", "chovani"],
  ["fullscreen", "chovani"],
  ["gameMode", "chovani"],
  ["modes", "osobni"],
  ["personal", "osobni"],
  ["accounts", "osobni"],
  ["timer", "osobni"],
  ["media", "osobni"],
  ["clipboard", "osobni"],
  ["github", "osobni"],
  ["system", "osobni"],
  ["claude", "claude"],
  ["questions", "claude"],
  ["notes", "poznamky"],
  ["update", "aplikace"],
  ["appearance", "vzhled"],
  ["onboarding", "zaciname"],
  ["ui", "zaciname"],
];

/** Kam vede skok odjinud (`palette/goto.ts` `gotoSettingsPanel`): id panelu (i staré id před vlnou 6 přes
 *  `aliases`) → ten panel; kotva `f-<pole>` → panel toho pole (`fieldPanel`: `labels.ts`, jinak podle cesty)
 *  a řádek. Cokoli jiného = nikam (`panel: null`). */
export function resolveTarget(
  id: string,
  panels: readonly string[],
  aliases: Record<string, string>,
  fieldPanel: (field: string) => string | null,
): { panel: string | null; anchor: string | null } {
  const real = aliases[id] ?? id;
  if (panels.includes(real)) return { panel: real, anchor: null };
  if (!id.startsWith("f-")) return { panel: null, anchor: null };
  const field = id.slice(2);
  const p = fieldPanel(field);
  return { panel: p && panels.includes(p) ? p : null, anchor: field };
}

export function panelForField(field: string | null | undefined): string | null {
  if (!field) return null;
  let best: [string, string] | null = null;
  for (const e of FIELD_PANELS) {
    if ((field === e[0] || field.startsWith(`${e[0]}.`)) && (!best || e[0].length > best[0].length)) best = e;
  }
  return best?.[1] ?? null;
}

/** Přesun položky z `from` na `to` (nová kopie). Mimo rozsah = beze změny. */
export function moveItem<T>(arr: readonly T[], from: number, to: number): T[] {
  const out = arr.slice();
  if (from < 0 || from >= out.length || to < 0 || to >= out.length || from === to) return out;
  const [x] = out.splice(from, 1);
  out.splice(to, 0, x);
  return out;
}

/** Cílový index při tažení: střed taženého řádku proti středům ostatních (výšky řádků v pořadí). */
export function dropIndex(heights: number[], from: number, dy: number): number {
  let to = from;
  if (dy > 0) {
    let acc = 0;
    for (let i = from + 1; i < heights.length; i++) {
      acc += heights[i];
      if (dy > acc - heights[i] / 2) to = i;
      else break;
    }
  } else if (dy < 0) {
    let acc = 0;
    for (let i = from - 1; i >= 0; i--) {
      acc += heights[i];
      if (-dy > acc - heights[i] / 2) to = i;
      else break;
    }
  }
  return to;
}

/** Tvar zkratky pro porovnání — stejně jako Rust `normalize_hotkey`. */
export function normalizeHotkey(s: string): string {
  const mods: string[] = [];
  let key = "";
  for (const raw of s.split("+")) {
    const p = raw.trim().toLowerCase();
    if (!p) continue;
    if (["ctrl", "control", "cmdorctrl", "commandorcontrol", "cmdorcontrol", "commandorctrl"].includes(p)) mods.push("ctrl");
    else if (p === "alt" || p === "option") mods.push("alt");
    else if (p === "shift") mods.push("shift");
    else if (["super", "win", "meta", "cmd", "command"].includes(p)) mods.push("super");
    else key = p;
  }
  return [...[...new Set(mods)].sort(), key].join("+");
}

const KEY_NAMES: Record<string, string> = { " ": "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right", Escape: "Esc" };

/** Stisk → `Ctrl+Alt+N`. Bez modifikátoru (kromě F-kláves) nebo jen modifikátor = null. */
export function hotkeyFromEvent(e: { key: string; code: string; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }): string | null {
  if (["Control", "Alt", "Shift", "Meta", "AltGraph"].includes(e.key)) return null;
  let key = KEY_NAMES[e.key] ?? "";
  if (!key) {
    if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
    else if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
    else if (/^F\d{1,2}$/.test(e.key)) key = e.key;
    else if (e.key.length === 1) key = e.key.toUpperCase();
    else key = e.key;
  }
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean) as string[];
  if (!mods.length && !/^F\d{1,2}$/.test(key)) return null;
  return [...mods, key].join("+");
}

export type HotkeyAction = "toggle" | "approve" | "deny" | "note" | "gameMode" | "notePassthrough";

/** Kolize zkratek: akce → jiná akce se stejnou zkratkou (jen akce, které se registrují — jako Rust). */
export function hotkeyCollisions(h: Record<HotkeyAction, string>, notesEnabled: boolean): Partial<Record<HotkeyAction, HotkeyAction>> {
  const acts: HotkeyAction[] = ["toggle", "approve", "deny", ...(notesEnabled ? (["note"] as const) : []), "gameMode", ...(notesEnabled ? (["notePassthrough"] as const) : [])];
  const out: Partial<Record<HotkeyAction, HotkeyAction>> = {};
  for (const a of acts) {
    for (const b of acts) {
      if (a !== b && h[a]?.trim() && normalizeHotkey(h[a]) === normalizeHotkey(h[b] ?? "")) out[a] = b;
    }
  }
  return out;
}

/** `#rgb` / `#rrggbb` (bez ohledu na velikost) → `#rrggbb`, jinak null. */
export function normHex(s: string): string | null {
  const t = s.trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(t)) return t;
  if (/^#[0-9a-f]{3}$/.test(t)) return `#${t[1]}${t[1]}${t[2]}${t[2]}${t[3]}${t[3]}`;
  return null;
}

/** Cesta relativně ke složce projektu, když v ní leží (`C:\a\b\x.exe` vůči `C:\a` → `b\x.exe`). */
export function relativeTo(base: string | null | undefined, path: string): string {
  if (!base) return path;
  const b = base.replace(/[\\/]+$/, "").toLowerCase() + "\\";
  const p = path.replace(/\//g, "\\");
  return p.toLowerCase().startsWith(b) ? p.slice(b.length) : path;
}

/** Leží složka PŘÍMO v jednom z `roots`? (Pak ji jde dohledat podle remote i na jiném PC.) */
export function inRoots(dir: string, roots: readonly string[]): boolean {
  const clean = (s: string) => s.replace(/\//g, "\\").replace(/\+$/, "").toLowerCase();
  const d = clean(dir);
  const parent = d.slice(0, Math.max(0, d.lastIndexOf("\\")));
  return roots.some((r) => clean(r) === parent);
}

/** Kanonický JSON pro porovnání „neuloženo“: seřazené klíče, bez `null` a prázdných seznamů (jádro je
 *  vynechává — `skip_serializing_if`), takže přidat a zase smazat pole není změna. */
export function canon(v: unknown): string {
  const walk = (x: any): any => {
    if (Array.isArray(x)) return x.map(walk);
    if (x && typeof x === "object") {
      const out: Record<string, unknown> = {};
      for (const k of Object.keys(x).sort()) {
        const val = x[k];
        if (val === null || val === undefined || (Array.isArray(val) && val.length === 0)) continue;
        out[k] = walk(val);
      }
      return out;
    }
    return x;
  };
  return JSON.stringify(walk(v));
}

/** Před uložením: zahodit úplně prázdné řádky editorů (odkazy), napůl vyplněné nechá na validaci. */
export function cleanDraft(cfg: any): any {
  const out = structuredClone(cfg);
  if (Array.isArray(out.personal?.links)) out.personal.links = out.personal.links.filter((l: any) => `${l.name ?? ""}${l.target ?? ""}`.trim());
  for (const p of out.projects ?? []) {
    if (Array.isArray(p.links)) p.links = p.links.filter((l: any) => `${l.label ?? ""}${l.url ?? ""}`.trim());
  }
  return out;
}
