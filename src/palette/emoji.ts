// Emoji v paletě (Ctrl+K): dotaz začínající `:` (`:srdce`, `:heart`, `:pal`) nebo příkaz „Emoji…“ →
// seznam ze statické tabulky `emojidata.ts`. Hledá se přes `search.ts` (bez diakritiky, fuzzy) v
// českém i anglickém jméně a klíčových slovech; zvýrazní se jméno v jazyce UI. Bez dotazu (jen `:`)
// tabulka v pořadí oblíbenosti. Čistá logika bez DOM — `emoji.test.mjs`.
import { tIn, type Lang } from "../i18n.ts";
import { fold, rank, type Item, type Ranked } from "./search.ts";
import { EMOJI } from "./emojidata.ts";

export const EMOJI_PREFIX = ":";
export const EMOJI_LIMIT = 60;

export interface EmojiItem extends Item {
  emoji: string;
}

/** `:něco` → hledaný text za dvojtečkou; jiný dotaz = `null` (není emoji režim). */
export function emojiQuery(query: string): string | null {
  const q = query.trimStart();
  return q.startsWith(EMOJI_PREFIX) ? q.slice(EMOJI_PREFIX.length) : null;
}

let cache: { lang: Lang; items: EmojiItem[] } | null = null;

/** Položky tabulky v jazyce UI (jednou na jazyk — tabulka je statická). */
export function emojiItems(lang: Lang): EmojiItem[] {
  if (cache?.lang === lang) return cache.items;
  const items = EMOJI.map((x) => ({
    id: `emoji:${x.key}`,
    emoji: x.e,
    title: tIn(lang, x.key, x.cs),
    keywords: `${tIn("cs", x.key, x.cs)} ${tIn("en", x.key, x.cs)} ${x.kw}`,
  }));
  cache = { lang, items };
  return items;
}

/**
 * Výsledky pro text za `:` (prázdný = nejpoužívanější v pořadí tabulky). Při stejném skóre vyhraje
 * jméno, které se s dotazem shoduje celé (`:heart` → „heart“ před „heart eyes“), jinak pořadí tabulky.
 */
export function searchEmoji(text: string, lang: Lang, limit = EMOJI_LIMIT): Ranked<EmojiItem>[] {
  const q = fold(text.trim());
  const all = rank(emojiItems(lang), text, [], Infinity);
  const exact = (r: Ranked<EmojiItem>) => (fold(r.item.title) === q ? 0 : 1);
  if (q) all.sort((a, b) => b.score - a.score || exact(a) - exact(b));
  return all.slice(0, limit);
}
