// Rychlé odpovědi palety nad dotazem — co se počítá jen z napsaného textu, bez snapshotu: `:` → emoji
// (`emoji.ts`, nahradí běžný seznam), převod jednotek (`units.ts`) a výpočet (`calc.ts`) jako první
// řádek nad běžnými výsledky. Počítá se jen při psaní (memo nad dotazem v `Palette.tsx`), nic na
// pozadí. Čistá logika bez DOM — `quick.test.mjs`; položky palety z toho skládá `more.ts`.
import type { Lang } from "../i18n.ts";
import type { Ranked } from "./search.ts";
import { evaluate, formatNum } from "./calc.ts";
import { convert, conversionText } from "./units.ts";
import { emojiQuery, searchEmoji, type EmojiItem } from "./emoji.ts";

export type Quick =
  | { kind: "emoji"; rows: Ranked<EmojiItem>[] }
  /** `title` k zobrazení, `copy` = co jde do schránky (číslo bez oddělovače tisíců) */
  | { kind: "calc" | "convert"; title: string; copy: string }
  | null;

export function quick(query: string, lang: Lang): Quick {
  const em = emojiQuery(query);
  if (em !== null) return { kind: "emoji", rows: searchEmoji(em, lang) };
  if (!query.trim()) return null;
  const c = convert(query);
  if (c) return { kind: "convert", ...conversionText(c, lang) };
  const v = evaluate(query);
  if (v !== null) return { kind: "calc", title: `= ${formatNum(v, lang, { group: true })}`, copy: formatNum(v, lang) };
  return null;
}
