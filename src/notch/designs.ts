// Registr vzhledů sekcí lišty notche (kolo 27, `notchBar.sections[].design`) ve stylu `layout/designs.ts`:
// PRVNÍ položka je výchozí (id se do configu nezapisuje). Výchozí = dnešní vzhled, kromě Dnů a času — tam je
// výchozí opravený Pás týdne (šéf chtěl čitelnou hierarchii místo velkého měsíce a osiřelého času). Názvy jsou
// páry [klíč, čeština] (angličtina `notch/en.ts`), překlad až v UI. Bez DOM, bez importů za běhu.
import type { Design } from "../layout/designs.ts";

type Id = "player" | "days" | "system" | "volume" | "limits" | "weather" | "meeting";
export const BAR_DESIGNS: Readonly<Record<Id, readonly Design[]>> = {
  player: [
    { id: "cover", name: ["cards.notch.d.player.cover", "Obal s textem"], line: ["cards.notch.d.player.cover.line", "Obal alba, název, interpret a ovládání."] },
    { id: "controls", name: ["cards.notch.d.player.controls", "Jen ovládání"], line: ["cards.notch.d.player.controls.line", "Velká tlačítka předchozí / přehrát / další a název."] },
    { id: "marquee", name: ["cards.notch.d.player.marquee", "Běžící text"], line: ["cards.notch.d.player.marquee.line", "Název běží v jednom řádku (jen s otevřeným šuplíkem)."] },
  ],
  days: [
    { id: "strip", name: ["cards.notch.d.days.strip", "Pás týdne"], line: ["cards.notch.d.days.strip.line", "Datum a čas nahoře, pod nimi týden s decentně zvýrazněným dneškem."] },
    { id: "sheet", name: ["cards.notch.d.days.sheet", "List kalendáře"], line: ["cards.notch.d.days.sheet.line", "Den v týdnu a velké číslo, vedle dnešní události z kalendáře."] },
    { id: "clock", name: ["cards.notch.d.days.clock", "Velký čas"], line: ["cards.notch.d.days.clock.line", "Velký čas a pod ním datum."] },
    { id: "dots", name: ["cards.notch.d.days.dots", "Týden se schůzkami"], line: ["cards.notch.d.days.dots.line", "Týden s tečkami událostí pod dny a příští schůzka pod ním."] },
    { id: "analog", name: ["cards.notch.d.days.analog", "Analogové hodiny"], line: ["cards.notch.d.days.analog.line", "Ciferník s ručičkami a datum."] },
  ],
  system: [
    { id: "lines", name: ["cards.notch.d.system.lines", "Linky"], line: ["cards.notch.d.system.lines.line", "Název, mini graf a procenta v řádcích."] },
    { id: "area", name: ["cards.notch.d.system.area", "Plochy"], line: ["cards.notch.d.system.area.line", "Grafy s vyplněnou plochou pod čarou."] },
    { id: "bars", name: ["cards.notch.d.system.bars", "Sloupky"], line: ["cards.notch.d.system.bars.line", "Poslední vzorky jako sloupky."] },
    { id: "rings", name: ["cards.notch.d.system.rings", "Prstence"], line: ["cards.notch.d.system.rings.line", "Každá metrika jako prstenec s procenty."] },
    { id: "big", name: ["cards.notch.d.system.big", "Velká čísla"], line: ["cards.notch.d.system.big.line", "Jen velká čísla vedle sebe."] },
    { id: "htop", name: ["cards.notch.d.system.htop", "Pruhy jako htop"], line: ["cards.notch.d.system.htop.line", "Textové pruhy [|||||   ] jako v terminálu."] },
  ],
  volume: [
    { id: "vslider", name: ["cards.notch.d.volume.vslider", "Svislý posuvník"], line: ["cards.notch.d.volume.vslider.line", "Svislý posuvník a ztlumení."] },
    { id: "hslider", name: ["cards.notch.d.volume.hslider", "Vodorovný posuvník"], line: ["cards.notch.d.volume.hslider.line", "Vodorovný posuvník s procenty."] },
    { id: "knob", name: ["cards.notch.d.volume.knob", "Otočné kolečko"], line: ["cards.notch.d.volume.knob.line", "Kolečko s ryskou; kolečkem myši nebo šipkami."] },
    { id: "eq", name: ["cards.notch.d.volume.eq", "Sloupky"], line: ["cards.notch.d.volume.eq.line", "Sloupky rozsvícené podle úrovně hlasitosti (bez animace)."] },
    { id: "icon", name: ["cards.notch.d.volume.icon", "Ikona s číslem"], line: ["cards.notch.d.volume.icon.line", "Reproduktor a velké číslo hlasitosti."] },
  ],
  limits: [
    { id: "bars", name: ["cards.notch.d.limits.bars", "Čárky"], line: ["cards.notch.d.limits.bars.line", "Limit 5 h a týden jako segmentové pruhy s resetem a procenty."] },
    { id: "ring", name: ["cards.notch.d.limits.ring", "Prstenec"], line: ["cards.notch.d.limits.ring.line", "Limit 5 h jako prstenec s číslem uprostřed."] },
  ],
  weather: [
    { id: "now", name: ["cards.notch.d.weather.now", "Teplota"], line: ["cards.notch.d.weather.now.line", "Velká teplota, popis a místo."] },
    { id: "range", name: ["cards.notch.d.weather.range", "Rozsah dne"], line: ["cards.notch.d.weather.range.line", "Teplota na pruhu mezi dnešním minimem a maximem."] },
  ],
  meeting: [
    { id: "card", name: ["cards.notch.d.meeting.card", "Nahoře"], line: ["cards.notch.d.meeting.card.line", "Odpočet, název, čas a kalendář; Připojit jen s odkazem na schůzku."] },
    { id: "countdown", name: ["cards.notch.d.meeting.countdown", "Odpočet"], line: ["cards.notch.d.meeting.countdown.line", "Velký odpočet do začátku a název pod ním."] },
  ],
};

/** Lidské názvy sekcí (nabídka „+“, panel). */
export const BAR_NAMES: Readonly<Record<Id, readonly [string, string]>> = {
  player: ["cards.notch.sec.player", "Přehrávač"],
  days: ["cards.notch.sec.days", "Dny a čas"],
  system: ["cards.notch.sec.system", "Systém"],
  volume: ["cards.notch.sec.volume", "Hlasitost"],
  limits: ["cards.notch.sec.limits", "Limit Claude"],
  weather: ["cards.notch.sec.weather", "Počasí"],
  meeting: ["cards.notch.sec.meeting", "Příští schůzka"],
};

export const barDesigns = (sec: string): readonly Design[] => BAR_DESIGNS[sec as Id] ?? [];

/** Vzhled, který sekce kreslí: zvolený, jinak (i při neznámém id) výchozí. */
export function barDesignOf(sec: string, design: string | undefined): string {
  const list = barDesigns(sec);
  return (list.find((x) => x.id === design) ?? list[0])?.id ?? "";
}

/** Id vzhledu k zápisu do configu: neznámé i výchozí = undefined. */
export function storedBarDesign(sec: string, id: unknown): string | undefined {
  const list = barDesigns(sec);
  return typeof id === "string" && list.some((x) => x.id === id) && id !== list[0]?.id ? id : undefined;
}
