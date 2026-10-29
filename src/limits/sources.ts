// Další zdroje limitů 5h / týden (Rust `limitsrc.rs`, `limitsfetch.rs`): co prázdný stav karty nabídne
// zapnout jedním klikem. Čistá logika — test `sources.test.mjs`.

/** Výřez `LimitsView`, který rozhodování potřebuje. */
export type SourcesInput = { at: number | null; desktop?: string; fetch?: string } | null | undefined;

export type Offer = "desktop" | "fetch";

/** Nabídky jen bez dat: vypnutá volba (`""` nebo chybí ve starém snapshotu) = nabídnout; zapnutá ne.
 *  Aplikace Claude první (nic neposílá), dotaz na Anthropic druhý. */
export function offers(l: SourcesInput): Offer[] {
  if (!l || l.at != null) return [];
  const out: Offer[] = [];
  if (!l.desktop) out.push("desktop");
  if (!l.fetch) out.push("fetch");
  return out;
}

/** Config s jednou zapnutou volbou (`claude.limitsDesktop` / `claude.limitsFetch`), ostatní beze změny. */
export function withOffer<T extends object>(cfg: T, o: Offer): T {
  const key = o === "desktop" ? "limitsDesktop" : "limitsFetch";
  const claude = ((cfg as { claude?: object }).claude ?? {}) as object;
  return { ...cfg, claude: { ...claude, [key]: true } };
}
