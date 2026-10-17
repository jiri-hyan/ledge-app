// Překlady UI: `t(klíč, čeština, proměnné)`. Čeština je zdroj pravdy přímo v kódu (čitelné bez
// slovníku), angličtina přijde jako slovník `en` podle klíčů. Proměnné `{jmeno}` v textu.
// Slovníky: `src/i18n/en-*.ts`, `src/settings/en.ts` + `panels/en.ts`, `src/onboarding/en.ts`; načítá je
// `src/i18n/load.ts` (první import v App).
// Množná čísla: `tp(klíč, n, čeština)` — en klíče `klíč.one` / `klíč.other`. Hlídá `src/i18n.test.mjs`.
import { createSignal } from "solid-js";

export type Lang = "cs" | "en";

const [lang, setLangSignal] = createSignal<Lang>("cs");
export { lang };
export const setLang = (l: Lang) => setLangSignal(l);

/** Locale pro `toLocaleString` a spol. */
export const localeOf = (l: Lang) => (l === "en" ? "en-US" : "cs-CZ");
export const locale = () => localeOf(lang());

/** Slovník pro jiné jazyky než češtinu: klíč → text. Chybějící klíč = čeština (nikdy prázdno). */
const dict: Partial<Record<Lang, Record<string, string>>> = {};
export const registerDict = (l: Lang, d: Record<string, string>) => {
  dict[l] = { ...(dict[l] ?? {}), ...d };
};

const fill = (raw: string, vars?: Record<string, string | number>) =>
  vars ? raw.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : raw;

export function t(key: string, cs: string, vars?: Record<string, string | number>): string {
  const l = lang();
  return fill(l === "cs" ? cs : (dict[l]?.[key] ?? cs), vars);
}

/** Text v konkrétním jazyce bez ohledu na jazyk UI (hledání v obou jazycích: paleta). */
export function tIn(l: Lang, key: string, cs: string): string {
  return l === "cs" ? cs : (dict[l]?.[key] ?? cs);
}

export type PluralForm = "one" | "few" | "other";

/** Kategorie množného čísla: cs 1 / 2–4 / jinak (0, 5+, desetinná), en 1 / jinak. */
export function pluralForm(l: Lang, n: number): PluralForm {
  if (n === 1) return "one";
  if (l === "cs" && Number.isInteger(n) && n >= 2 && n <= 4) return "few";
  return "other";
}

/** Vybere tvar podle `pluralForm`; chybí-li `few`, platí `other`. */
export function plural(l: Lang, n: number, forms: { one: string; few?: string; other: string }): string {
  const f = pluralForm(l, n);
  return f === "few" ? (forms.few ?? forms.other) : forms[f];
}

/** Čeština jako jeden pevný text, nebo tři tvary [1, 2–4, 5+]. */
export type CsPlural = string | readonly [string, string, string];
const csForms = (cs: CsPlural) => (typeof cs === "string" ? { one: cs, other: cs } : { one: cs[0], few: cs[1], other: cs[2] });

/** `t` s množným číslem: `{n}` je vždy k dispozici; en tvary jsou klíče `klíč.one` a `klíč.other`. */
export function tp(key: string, n: number, cs: CsPlural, vars?: Record<string, string | number>): string {
  const l = lang();
  const all = { n, ...vars };
  const fallback = plural("cs", n, csForms(cs));
  if (l === "cs") return fill(fallback, all);
  const d = dict[l];
  const one = d?.[`${key}.one`];
  const other = d?.[`${key}.other`];
  return fill(other === undefined ? fallback : plural(l, n, { one: one ?? other, other }), all);
}
