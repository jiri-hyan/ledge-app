// Nabídka připomínky z textu poznámky („zítra v 10“ → chip „⏰ Připomenout zítra 10:00?“). Čisté
// funkce bez závislostí — rozpoznání času dělá Rust (`notes/when.rs`, příkaz `note_suggest_remind`), tady
// jen levný předfiltr, ať se jádro nevolá pro text bez čísla a bez časového slova, a převod výsledku.
// Ověřuje `suggest.test.mjs`.

/** Ticho po posledním znaku, než se jádro zeptá (psaní nic nevolá). */
export const SUGGEST_DELAY = 450;
/** Delší text se jádru neposílá celý (čas bývá nahoře; strop drží IPC malé). */
export const SUGGEST_MAX_CHARS = 20_000;

export type Suggest = { label: string; iso: string };

// složený text (bez diakritiky): číslice, nebo slovo, které jádro umí (dnes, zítra, dny v týdnu, ráno, za …)
const HINT = /\d|dnes|zitra|zejtra|pozitri|pozejtri|pondeli|utery|stred|ctvrtek|patek|sobot|nedel|rano|dopoledne|odpoledne|vecer|\bza\s/;

const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

/** Může text obsahovat čas? (Falešné ano nevadí — jádro řekne ne; falešné ne by chip zahodilo.) */
export function looksTimed(text: string): boolean {
  return text.trim().length >= 3 && HINT.test(fold(text));
}

/** Zeptat se jádra? Jen bez připomínky, s textem, který vypadá časově, a pokud se na ten text už neptalo. */
export function wantsAsk(text: string, remind: string, asked: string): boolean {
  return !remind && text !== asked && looksTimed(text);
}

/** ISO s offsetem z jádra (`2026-10-24T10:00:00+02:00`) → hodnota výběru připomínky `YYYY-MM-DDTHH:MM`. */
export function toLocalValue(iso: string): string | null {
  const m = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2})/.exec(iso);
  return m ? m[1] : null;
}

/** Chip se ukáže jen s návrhem, který uživatel nezavřel a dokud poznámka nemá připomínku. */
export function visibleSuggest(s: Suggest | null, remind: string, dismissed: string): Suggest | null {
  return s && !remind && s.iso !== dismissed && toLocalValue(s.iso) ? s : null;
}
