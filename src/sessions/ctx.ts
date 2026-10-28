// Velikost kontextu živé session (kolo 23, E1) — čistá logika pruhu pod řádkem session (bez DOM a bez `t()`).
// Data: Rust `ctxsize::Ctx` (poslední `usage` v transcriptu: vstup + zápis cache + čtení cache), příkaz `session_context`.

/** Zrcadlo Rust `ctxsize::Ctx`. */
export interface CtxView {
  tokens: number;
  /** kontext prvního volání session; null = začátek souboru nenesl usage */
  first: number | null;
  /** tokens / first na 1 desetinu; null bez prvního volání */
  ratio: number | null;
  level: "ok" | "warn" | "high";
}

/** Rysky pruhu — stejné hranice jako `ctxsize::WARN` / `HIGH` (pravidlo týmu: od 250 k se ptát na předávku). */
export const CTX_WARN = 250_000;
export const CTX_HIGH = 350_000;
/** Konec stupnice pruhu; víc je pořád plný pruh (číslo v textu je skutečné). */
export const CTX_SCALE = 400_000;
/** Poměr k začátku se ukáže od této hodnoty (1,0× by nic neřeklo). */
export const RATIO_MIN = 1.5;

/** Výplň pruhu 0–1. */
export const ctxFill = (tokens: number): number => (tokens > 0 ? Math.min(1, tokens / CTX_SCALE) : 0);

/** Polohy rysek 0–1. */
export const ctxTicks = (): [number, number] => [CTX_WARN / CTX_SCALE, CTX_HIGH / CTX_SCALE];

/** „N× víc než na začátku“ jen když je rozdíl vidět. */
export const showRatio = (c: CtxView): boolean => c.ratio != null && c.ratio >= RATIO_MIN;

/** Tip o předávce jen nad hranicí (ne doporučení /compact). */
export const showTip = (c: CtxView): boolean => c.level !== "ok";

/** Stejná data → žádný nový signál (UI se nepřekresluje zbytečně). */
export function sameCtx(a: CtxView | null, b: CtxView | null): boolean {
  if (a == null || b == null) return a == b;
  return a.tokens === b.tokens && a.first === b.first && a.ratio === b.ratio && a.level === b.level;
}
