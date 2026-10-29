// Registr vzhledů karet (kolo 25, `CardLook.design`): každá karta, která umí víc vzhledů, tu má seznam; PRVNÍ
// položka je výchozí = dnešní vzhled (id se do configu nepíše). Čistá logika bez DOM a bez `t()` — názvy jsou
// páry [klíč, čeština] (angličtina `i18n/en-designs.ts`), překlad až v UI (`DesignPicker.tsx`). Další karty
// (Počasí, Týden …) přidají svůj záznam do `CARD_DESIGNS`; vykreslení vzhledu dělá komponenta karty podle `designOf`.
import { WEATHER_DESIGNS } from "../cards/weatherDesigns.ts";
import { WEEK_DESIGNS } from "../board/weekDesigns.ts";

export type Design = { id: string; name: [string, string]; line: [string, string] };
type LookWithDesign = { design?: string } | null | undefined;

export const CARD_DESIGNS: Record<string, readonly Design[]> = {
  claude: [
    { id: "classic", name: ["cards.design.claude.classic", "Klasický"], line: ["cards.design.claude.classic.line", "Sezení, limity a útrata jako dřív."] },
    { id: "rings", name: ["cards.design.claude.rings", "Prstence"], line: ["cards.design.claude.rings.line", "Limit 5 h a týden jako dva soustředné prstence."] },
    { id: "pace", name: ["cards.design.claude.pace", "Tempomat"], line: ["cards.design.claude.pace.line", "Pruh limitu s ryskou plánu a odhadem, kdy dojde."] },
    { id: "hero", name: ["cards.design.claude.hero", "Hrdina: limit 5 h"], line: ["cards.design.claude.hero.line", "Jedno velké číslo: limit 5 h."] },
    { id: "heroWeek", name: ["cards.design.claude.heroWeek", "Hrdina: týden"], line: ["cards.design.claude.heroWeek.line", "Jedno velké číslo: týdenní limit."] },
    { id: "heroSpend", name: ["cards.design.claude.heroSpend", "Hrdina: útrata"], line: ["cards.design.claude.heroSpend.line", "Jedno velké číslo: útrata dnes (odhad)."] },
    { id: "spend", name: ["cards.design.claude.spend", "Útrata"], line: ["cards.design.claude.spend.line", "Dnešek velký a 14 dní ve sloupcích."] },
    { id: "sessions", name: ["cards.design.claude.sessions", "Sezení"], line: ["cards.design.claude.sessions.line", "Dlaždice běžících sezení a tenký limit dole."] },
    { id: "gauge", name: ["cards.design.claude.gauge", "Palivoměr"], line: ["cards.design.claude.gauge.line", "Půlkruh s ručičkou pro limit 5 h."] },
  ],
  system: [
    { id: "lines", name: ["cards.design.system.lines", "Řádky"], line: ["cards.design.system.lines.line", "CPU, paměť, disky a síť s malými grafy."] },
    { id: "rings", name: ["cards.design.system.rings", "Prstence"], line: ["cards.design.system.rings.line", "CPU, paměť a disk jako tři prstence."] },
    { id: "gauges", name: ["cards.design.system.gauges", "Budíky"], line: ["cards.design.system.gauges.line", "Půlkruhové budíky s ručičkou."] },
    { id: "bars", name: ["cards.design.system.bars", "Histogram"], line: ["cards.design.system.bars.line", "Sloupky zátěže CPU za poslední chvíli."] },
    { id: "terminal", name: ["cards.design.system.terminal", "Terminál"], line: ["cards.design.system.terminal.line", "Textové pruhy jako v htop."] },
    { id: "tiles", name: ["cards.design.system.tiles", "Čísla"], line: ["cards.design.system.tiles.line", "Tři velká čísla s malým grafem."] },
    { id: "net", name: ["cards.design.system.net", "Síť"], line: ["cards.design.system.net.line", "Příjem a odesílání v popředí, CPU a paměť menší."] },
  ],
  weather: WEATHER_DESIGNS,
  week: WEEK_DESIGNS,
};

/** Vzhledy karty (prázdné = karta má jen jeden vzhled). */
export const designsOf = (cardId: string): readonly Design[] => CARD_DESIGNS[cardId] ?? [];

/** Výchozí (dnešní) vzhled karty, nebo undefined. */
export const defaultDesign = (cardId: string): string | undefined => designsOf(cardId)[0]?.id;

/** Platné id vzhledu, který se má do configu zapsat: neznámé i výchozí = undefined (pole pryč). */
export function storedDesign(cardId: string, id: unknown): string | undefined {
  const list = designsOf(cardId);
  return typeof id === "string" && list.some((d) => d.id === id) && id !== list[0].id ? id : undefined;
}

/** Vzhled, který má karta kreslit: zvolený, jinak (i při neznámém id) výchozí; karta bez vzhledů = undefined. */
export function designOf(cardId: string, look: LookWithDesign): Design | undefined {
  const list = designsOf(cardId);
  return list.find((d) => d.id === look?.design) ?? list[0];
}
