// Vzhledy karty Počasí (kolo 25) — seznam ve tvaru registru `layout/designs.ts` (`CARD_DESIGNS.weather`),
// PRVNÍ je výchozí (dnešní). Čistá logika bez DOM; názvy jsou páry [klíč, čeština], překlad až v UI.
export type WeatherDesign = { id: string; name: [string, string]; line: [string, string] };

export const WEATHER_DESIGNS: readonly WeatherDesign[] = [
  { id: "classic", name: ["cards.design.weather.classic", "Klasické"], line: ["cards.design.weather.classic.line", "Ikona, teplota, popis a dnešní rozpětí."] },
  { id: "sky", name: ["cards.design.weather.sky", "Živá obloha"], line: ["cards.design.weather.sky.line", "Pozadí podle počasí a denní doby, jemně se hýbou mraky, déšť, sníh a hvězdy."] },
  { id: "thermo", name: ["cards.design.weather.thermo", "Teploměr dne"], line: ["cards.design.weather.thermo.line", "Pruh od minima k maximu s polohou „teď“."] },
  { id: "poster", name: ["cards.design.weather.poster", "Plakát"], line: ["cards.design.weather.poster.line", "Obří teplota a slovo počasí jako titulek."] },
  { id: "curve", name: ["cards.design.weather.curve", "Křivka"], line: ["cards.design.weather.curve.line", "Hodinová teplota jako křivka a sloupky srážek."] },
  { id: "week", name: ["cards.design.weather.week", "Pět dní"], line: ["cards.design.weather.week.line", "Dny s pruhy minima a maxima jako v telefonu."] },
  { id: "arc", name: ["cards.design.weather.arc", "Oblouk slunce"], line: ["cards.design.weather.arc.line", "Slunce putuje po oblouku od východu k západu."] },
];

/** Platné id vzhledu karty; neznámé / chybějící = výchozí (první). */
export function weatherDesign(id: string | undefined): string {
  return WEATHER_DESIGNS.find((d) => d.id === id)?.id ?? WEATHER_DESIGNS[0].id;
}
