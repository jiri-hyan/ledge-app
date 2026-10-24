// Styly zavřeného Ledge (`strip.look`, Rust `looks.rs`): JEDINÝ seznam pro Nastavení (galerie stylů).
// Id jsou kontrakt s Rustem (test zrcadla `looks::tests::ts_list_mirrors_rust` čte tenhle soubor —
// pořadí polí v řádku `id, key, cs, en` drž). Název přes `t(look.key, look.cs)`, popis přes
// `t(look.desc.key, look.desc.cs)`; anglické texty jsou v `strip/en.ts` (`en` tady = totéž pro přehled).
// Bez importů za běhu (`looks.test.mjs`).

export const STRIP_LOOK_IDS = ["classic", "island", "dots", "line", "icons", "clock", "corner", "ghost", "bar", "notch", "progress", "ticker", "minimum", "trio", "rings"] as const;
export type StripLookId = (typeof STRIP_LOOK_IDS)[number];

export type StripLookDef = {
  id: StripLookId;
  /** klíč názvu pro `t()` */
  key: string;
  cs: string;
  en: string;
  /** popis: `t(desc.key, desc.cs)` */
  desc: { key: string; cs: string };
};

export const STRIP_LOOKS: readonly StripLookDef[] = [
  { id: "classic", key: "appearance.look.classic", cs: "Klasický", en: "Classic", desc: { key: "appearance.look.classic.desc", cs: "Proužek, jak ho znáš: tvar, velikost a položky si poskládáš sám." } },
  { id: "island", key: "appearance.look.island", cs: "Ostrůvek", en: "Island", desc: { key: "appearance.look.island.desc", cs: "Černá kapsle u horní hrany. V klidu malinká, při dění se roztáhne a ukáže, co se děje." } },
  { id: "dots", key: "appearance.look.dots", cs: "Jen tečky", en: "Just dots", desc: { key: "appearance.look.dots.desc", cs: "Čtyři barevné tečky: Claude, servery, repa a ty. Žádný text, skoro neviditelné." } },
  { id: "line", key: "appearance.look.line", cs: "Tenká linka", en: "Thin line", desc: { key: "appearance.look.line.desc", cs: "Tenká linka přes celou horní hranu. Barevné úseky ukazují stav, při čekání na tebe zežloutne." } },
  { id: "icons", key: "appearance.look.icons", cs: "Ikony s odznaky", en: "Icons with badges", desc: { key: "appearance.look.icons.desc", cs: "Řada ikon s čísly jako v liště telefonu: Claude, servery, repa." } },
  { id: "clock", key: "appearance.look.clock", cs: "Hodiny", en: "Clock", desc: { key: "appearance.look.clock.desc", cs: "Velké hodiny a jedna stavová tečka vedle. Pro skrytý hlavní panel Windows." } },
  { id: "corner", key: "appearance.look.corner", cs: "Rohový odznak", en: "Corner badge", desc: { key: "appearance.look.corner.desc", cs: "Malý odznak v rohu obrazovky místo uprostřed. Roh vybereš pozicí vlevo nebo vpravo." } },
  { id: "ghost", key: "appearance.look.ghost", cs: "Neviditelný do události", en: "Hidden until needed", desc: { key: "appearance.look.ghost.desc", cs: "V klidu není vidět nic. Ukáže se, jen když na tebe něco čeká nebo je hotovo. Najetí k hraně šuplík otevře dál." } },
  { id: "bar", key: "appearance.look.bar", cs: "Stavová lišta", en: "Status bar", desc: { key: "appearance.look.bar.desc", cs: "Pruh přes celou šířku jako v terminálu: stav vlevo, limit 5h a čas vpravo." } },
  { id: "notch", key: "appearance.look.notch", cs: "Výřez", en: "Notch", desc: { key: "appearance.look.notch.desc", cs: "Černý výřez s oblými oušky. Vypadá jako součást monitoru." } },
  { id: "progress", key: "appearance.look.progress", cs: "Průběh", en: "Progress", desc: { key: "appearance.look.progress.desc", cs: "Krátký text a pod ním tenká lišta: limit Clauda 5h, časovač nebo skladba. Lišta jen ze skutečných dat." } },
  { id: "ticker", key: "appearance.look.ticker", cs: "Střídající se řádek", en: "Rotating line", desc: { key: "appearance.look.ticker.desc", cs: "Jeden řádek. Když se děje víc věcí, střídají se po 5 s; jinak se nic nehýbe." } },
  { id: "minimum", key: "appearance.look.minimum", cs: "Minimum", en: "Minimal", desc: { key: "appearance.look.minimum.desc", cs: "Malá kapsle jen s baterií a časem. Na počítači bez baterie jen čas." } },
  { id: "trio", key: "appearance.look.trio", cs: "Trojice", en: "Trio", desc: { key: "appearance.look.trio.desc", cs: "Počasí, čas s datem a vytížení počítače s baterií v jedné kapsli. Počasí jen z karty Počasí, nic se nestahuje navíc. Trojice zjišťuje vytížení systému každých 10 s." } },
  { id: "rings", key: "appearance.look.rings", cs: "Stav Clauda", en: "Claude status", desc: { key: "appearance.look.rings.desc", cs: "Prstence limitů Clauda (5 h a týden) a čas. Prstenec se točí, jen když Claude pracuje; oranžově, když na tebe čeká." } },
];

/** Definice stylu; neznámé id = Klasický (starší / poškozený config). */
export function lookOf(id: string | undefined | null): StripLookDef {
  return STRIP_LOOKS.find((l) => l.id === id) ?? STRIP_LOOKS[0];
}

/** Položky, které jde u stylů mimo Klasický v Pokročilých vypnout (`strip.lookHide`, Rust `looks::without`);
 *  `system` = CPU a paměť kapsle Trojice (kolo 22). */
export const LOOK_HIDEABLE = ["waiting", "claude", "running", "repos", "timer", "media", "limits", "clock", "system"] as const;

/** Tvar kapsle (`strip.lookShape`, Rust `looks::Shape`, kolo 22) — volba tvaru, ne nový styl. */
export const LOOK_SHAPES = ["round", "sharp", "long", "island"] as const;
export type LookShape = (typeof LOOK_SHAPES)[number];
/** Styly, u kterých jde tvar zvolit (zrcadlo `Look::shapeable`). */
export const SHAPEABLE_LOOKS: readonly StripLookId[] = ["island", "icons", "clock", "ghost", "progress", "ticker", "minimum", "trio", "rings"];
/** Neznámý / chybějící tvar = dnešní (`round`). */
export const normShape = (v: unknown): LookShape => (LOOK_SHAPES.includes(v as LookShape) ? (v as LookShape) : "round");

/** `strip.lookHide` s jednou změnou (`on` = ukazovat); vstup netknutý, pořadí podle `LOOK_HIDEABLE`. */
export function withLookShown(hide: string[] | undefined, id: string, on: boolean): string[] {
  const rest = new Set((hide ?? []).filter((x) => x !== id));
  if (!on) rest.add(id);
  return LOOK_HIDEABLE.filter((x) => rest.has(x));
}
