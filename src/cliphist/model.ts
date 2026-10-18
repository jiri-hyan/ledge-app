// Historie schránky v paletě — čistá logika bez DOM (`model.test.mjs`): prefix `v:`, typy z jádra
// (`cliphist::EntryView`, `ClipList`), řádky palety (položky, vypnuto, prázdno, vymazat) a podtitul
// (stáří, počet řádků, připnuto). Jádro posílá jen jednořádkový náhled; celý text zůstává v Rustu
// a do schránky ho zapíše `clip_paste` podle id.
import { t, tp } from "../i18n.ts";
import { ago } from "../format.ts";
import { rank, type Item, type Ranked } from "../palette/search.ts";

export const CLIP_PREFIX = "v:";

export interface ClipEntry {
  id: number;
  /** jeden řádek, bílé znaky sloučené, nejvýš 160 znaků + „…“ */
  preview: string;
  chars: number;
  lines: number;
  atMs: number;
  pinned: boolean;
}

export interface ClipList {
  enabled: boolean;
  items: ClipEntry[];
}

export type ClipKind = "entry" | "off" | "empty" | "clear" | "settings";

export interface ClipRow extends Item {
  kind: ClipKind;
  entry?: ClipEntry;
}

/** `v:něco` (bez ohledu na velikost, mezery na začátku) → text za prefixem; jiný dotaz = `null`. */
export function clipQuery(query: string): string | null {
  const q = query.trimStart();
  return q.slice(0, CLIP_PREFIX.length).toLowerCase() === CLIP_PREFIX ? q.slice(CLIP_PREFIX.length) : null;
}

/** Podtitul položky: stáří · počet řádků (víc než 1) · připnuto. */
export function clipSub(e: ClipEntry, now: number): string {
  const parts = [ago(e.atMs, now)];
  if (e.lines > 1) parts.push(tp("palette.clip.lines", e.lines, ["{n} řádek", "{n} řádky", "{n} řádků"]));
  if (e.pinned) parts.push(t("palette.clip.pinned", "připnuto"));
  return parts.join(" · ");
}

const row = (kind: ClipKind, title: string, sub: string): Ranked<ClipRow> => ({ item: { id: `clip:${kind}`, kind, title, sub }, score: 0, ranges: [] });

/**
 * Řádky pro dotaz za `v:`. `list` = `undefined` → ještě se načítá (nic). Vypnuto = řádek „Enter ji
 * zapne“ (zapnutí přímo z palety), prázdno = nápověda; vždy i odkaz do Nastavení. Bez dotazu v pořadí jádra (připnuté nahoře, pak nejnovější) + „Vymazat historii“,
 * je-li co mazat; s dotazem fuzzy hledání nad náhledem (`search.ts`).
 */
export function clipRows(list: ClipList | undefined, text: string, now: number): Ranked<ClipRow>[] {
  if (!list) return [];
  const settings = row("settings", t("palette.clip.settings", "Nastavení historie schránky…"), t("palette.clip.settingsSub", "Kolik položek držet, vyloučené aplikace, mazání při zamčení"));
  if (!list.enabled) return [row("off", t("palette.clip.off", "Historie schránky je vypnutá — Enter ji zapne"), t("palette.clip.offSub", "Pamatuje si jen zkopírovaný text, jen v paměti Ledge, nic neposílá ven. Hesla ze správců hesel a prohlížeče ani tokeny neukládá.")), settings];
  if (!list.items.length) return [row("empty", t("palette.clip.empty", "Zatím prázdné — zkopíruj text (Ctrl+C) a objeví se tady"), t("palette.clip.emptySub", "Hesla ze správců hesel a prohlížeče, tokeny a klíče se neukládají")), settings];
  const entries: ClipRow[] = list.items.map((e) => ({ id: `clip:${e.id}`, kind: "entry", entry: e, title: e.preview, sub: clipSub(e, now) }));
  if (text.trim()) return rank(entries, text, [], Infinity);
  const out: Ranked<ClipRow>[] = entries.map((item) => ({ item, score: 0, ranges: [] }));
  if (list.items.some((e) => !e.pinned)) out.push(row("clear", t("palette.clip.clear", "Vymazat historii"), t("palette.clip.clearSub", "Smaže z paměti všechno kromě připnutých")));
  out.push(settings);
  return out;
}

/** Výběr po změně seznamu (smazaná položka): zůstat na místě, nejvýš na posledním řádku. */
export const clampSel = (sel: number, n: number): number => Math.max(0, Math.min(sel, n - 1));
