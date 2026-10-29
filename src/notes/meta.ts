// Barva a štítky poznámky (Lístky 2.0) — čistá logika bez DOM a Tauri, zrcadlo Rust `notes::meta` /
// `notes::links` (testy `meta.test.mjs`). Barva = id z palety (`color:` ve frontmatteru) nebo `#rrggbb`;
// štítky = `tags:` ze frontmatteru ∪ `#štítek` v textu. Ve frontmatteru jsou jen ty, které jde v lístku
// odebrat (`frontTags`); `#štítek` v textu se maže v textu.

/** Paleta barev lístku (stejná id jako Rust `meta::COLORS`). */
export const NOTE_COLORS: { id: string; hex: string }[] = [
  { id: "yellow", hex: "#f5c542" },
  { id: "orange", hex: "#f59e42" },
  { id: "red", hex: "#ef6b6b" },
  { id: "pink", hex: "#f472b6" },
  { id: "purple", hex: "#a78bfa" },
  { id: "blue", hex: "#60a5fa" },
  { id: "teal", hex: "#2dd4bf" },
  { id: "green", hex: "#4ade80" },
];

const HEX = /^#[0-9a-f]{6}$/i;

/** Barva z palety / `#rrggbb` → hex, jinak null (výchozí vzhled). */
export function colorHex(c: string | null | undefined): string | null {
  if (!c) return null;
  const v = c.trim().toLowerCase();
  return NOTE_COLORS.find((x) => x.id === v)?.hex ?? (HEX.test(v) ? v : null);
}

/** Barva lístku: vlastní `color:`, jinak podle prvního štítku s barvou v `notes.tagColors`. */
export function noteColor(n: { color?: string | null; tags?: string[]; project?: string | null } | null | undefined, tagColors?: Record<string, string> | null, projectColors?: Record<string, string> | null): string | null {
  const own = colorHex(n?.color);
  if (own) return own;
  // jako karta: vlastní barva → barva projektu → barva podle štítku
  const proj = n?.project ? colorHex(projectColors?.[n.project]) : null;
  if (proj) return proj;
  for (const t of n?.tags ?? []) {
    const c = colorHex(tagColors?.[t]);
    if (c) return c;
  }
  return null;
}

const TAG_CHAR = /^[\p{L}\p{N}_/-]$/u;
const tagChar = (c: string) => TAG_CHAR.test(c);

/** Štítek v kanonické podobě (bez `#`, malými písmeny) jako Rust `meta::norm_tag`; neplatný = null. */
export function normTag(raw: string): string | null {
  const t = raw.trim().replace(/^#+/, "").trim().toLowerCase();
  const chars = [...t];
  if (!chars.length || chars.length > 40 || !chars.every(tagChar) || /^[0-9]+$/.test(t)) return null;
  return t;
}

/** `#štítky` v textu (bez bloků kódu a `kódu`), v pořadí výskytu — jako Rust `links::inline_tags`. */
export function inlineTags(body: string): string[] {
  const out: string[] = [];
  let fence = false;
  for (const raw of body.split(/\r?\n/)) {
    const lead = raw.trimStart();
    if (lead.startsWith("```") || lead.startsWith("~~~")) {
      fence = !fence;
      continue;
    }
    if (fence) continue;
    let code = false;
    const line = [...raw].map((c) => (c === "`" ? ((code = !code), " ") : code ? " " : c)).join("");
    const chars = [...line];
    for (let i = 0; i < chars.length; i++) {
      if (chars[i] !== "#") continue;
      const before = chars[i - 1];
      if (before !== undefined && !/[\s(,;]/u.test(before)) continue;
      let j = i + 1;
      while (j < chars.length && tagChar(chars[j])) j++;
      const tag = normTag(chars.slice(i + 1, j).join("").replace(/\/+$/, ""));
      if (tag && !out.includes(tag)) out.push(tag);
      i = j - 1;
    }
  }
  return out;
}

/** Štítky, které jsou ve frontmatteru (dají se v lístku odebrat): všechny minus ty z textu. */
export function frontTags(tags: string[] | undefined, body: string): string[] {
  const inline = new Set(inlineTags(body));
  return (tags ?? []).filter((t) => !inline.has(t));
}

/** Přidání štítku do seznamu frontmatteru: neplatný / už je = null (nic se nezapíše). */
export function withTag(front: string[], raw: string): string[] | null {
  const t = normTag(raw);
  return t && !front.includes(t) ? [...front, t] : null;
}

export const withoutTag = (front: string[], tag: string) => front.filter((t) => t !== tag);

/** Barva, kterou jádro dává projektu bez vlastní (`snapshot.rs`) — není to barva projektu, nededí se. */
export const PROJECT_ACCENT_PLACEHOLDER = "#64748b";

/** Proužek / tečka poznámky: vlastní `color:` (8 jmen z palety, nebo ručně psaný `#rrggbb`), jinak barva projektu
 *  (jen skutečná, ne záložní šedá), jinak null = bez barvy. Neznámé jméno = žádná vlastní barva. */
export function stripeColor(n: { color?: string | null } | null | undefined, projectAccent?: string | null): string | null {
  const own = colorHex(n?.color);
  if (own) return own;
  const p = (projectAccent ?? "").trim().toLowerCase();
  return HEX.test(p) && p !== PROJECT_ACCENT_PLACEHOLDER ? p : null;
}

/** Jméno barvy z palety pro tečku v menu (ručně psaný hex ani neznámé jméno není vybraná tečka). */
export const paletteId = (c: string | null | undefined): string | null => NOTE_COLORS.find((x) => x.id === (c ?? "").trim().toLowerCase())?.id ?? null;
