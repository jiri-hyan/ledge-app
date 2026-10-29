// Hlavička poznámky složená automaticky: „Projekt · Název · 26. 10.“ — projekt jen když je, datum = vytvoření.
// Čistá logika bez DOM (testy `headline.test.mjs`); projekt a datum se do názvu nepíšou ručně.

/** Krátké datum vytvoření: česky „26. 10.“, anglicky „10/26“. Neplatný čas = prázdný řetězec. */
export function shortDate(ms: number, lang: "cs" | "en" = "cs"): string {
  if (!Number.isFinite(ms) || ms <= 0) return "";
  const d = new Date(ms);
  return lang === "en" ? `${d.getMonth() + 1}/${d.getDate()}` : `${d.getDate()}. ${d.getMonth() + 1}.`;
}

/** Části hlavičky v pořadí zobrazení; prázdné části (bez projektu, bez data) vypadnou. */
export function headlineParts(n: { title: string; createdMs: number }, project: string | null, lang: "cs" | "en" = "cs"): string[] {
  return [project ?? "", n.title, shortDate(n.createdMs, lang)].map((x) => x.trim()).filter(Boolean);
}

/** Hlavička jedním řetězcem (lístek na ploše, titulek okna). */
export const headline = (n: { title: string; createdMs: number }, project: string | null, lang: "cs" | "en" = "cs") => headlineParts(n, project, lang).join(" · ");

/** První textový řádek těla bez nejběžnějších značek (nadpis `#`, odrážka, úkol) a bez samotných obrázků —
 *  jen náhled do placeholderu pole Nadpis; skutečný název počítá jádro (`md::title`). */
export function firstLine(body: string): string {
  for (const raw of body.split(/\r?\n/)) {
    const l = raw.trim();
    if (!l || l.startsWith("![") || l.startsWith("```") || l.startsWith("~~~")) continue;
    const plain = l.replace(/^#{1,6}\s+/, "").replace(/^[-*+]\s+(\[[ xX]\]\s+)?/, "").replace(/^\d+[.)]\s+/, "").replace(/[*_`]/g, "").trim();
    if (plain) return plain.length > 80 ? `${plain.slice(0, 79)}…` : plain;
  }
  return "";
}
