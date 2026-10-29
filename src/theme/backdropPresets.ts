// Hotová pozadí otevřeného šuplíku (kolo 25) jako čistá data: gradienty ve třech skupinách (Klidná, Gradienty)
// a živá pozadí (Živá) se základním gradientem. NOVÉ POZADÍ = jeden záznam tady + anglický popisek v
// `settings/panels/en-backdrop.ts`. Popisky jsou páry [klíč, čeština] (překlad při vykreslení, `theme/label.ts`).
// Barvy jsou záměrně tmavé a sytě laděné: čitelnost textu navíc hlídá ztmavovací závoj (`backdrop.ts` `veilFor`).
import type { CustomGradient, FlameColor, GradKind, LiveKind } from "../types.ts";
import type { Pair } from "./looks.ts";

export type BackdropGroup = "calm" | "gradient" | "live";

export type Preset = {
  id: string;
  group: "calm" | "gradient";
  name: Pair;
  /** jedna věta pod názvem dlaždice */
  desc: Pair;
  kind: GradKind;
  angle: number;
  stops: readonly (readonly [string, number])[];
};

const P = (id: string, group: Preset["group"], name: Pair, desc: Pair, kind: GradKind, angle: number, stops: Preset["stops"]): Preset => ({ id, group, name, desc, kind, angle, stops });

export const PRESETS: readonly Preset[] = [
  // ----- Klidná: skoro jednobarevná, jen jemný přechod -----
  P("midnight", "calm", ["appearance.bd.p.midnight", "Půlnoc"], ["appearance.bd.p.midnight.d", "Hluboká modř, nahoře o chlup světlejší."], "linear", 180, [["#141c3a", 0], ["#080b18", 100]]),
  P("graphite", "calm", ["appearance.bd.p.graphite", "Grafit"], ["appearance.bd.p.graphite.d", "Neutrální šeď bez barevného nádechu."], "linear", 160, [["#262932", 0], ["#101216", 100]]),
  P("ink", "calm", ["appearance.bd.p.ink", "Inkoust"], ["appearance.bd.p.ink.d", "Tmavá skvrna svítící od horní hrany."], "radial", 0, [["#232a44", 0], ["#0a0c14", 80]]),
  P("fog", "calm", ["appearance.bd.p.fog", "Mlha"], ["appearance.bd.p.fog.d", "Chladná šedomodrá, klidná a čitelná."], "linear", 200, [["#2a3442", 0], ["#161c26", 100]]),
  // ----- Gradienty -----
  P("aurora", "gradient", ["appearance.bd.p.aurora", "Polární záře"], ["appearance.bd.p.aurora.d", "Smaragd přes tyrkys do indigové noci."], "linear", 200, [["#0b3d3a", 0], ["#0e5f63", 38], ["#2e2a8a", 74], ["#120f35", 100]]),
  P("sunset", "gradient", ["appearance.bd.p.sunset", "Západ slunce"], ["appearance.bd.p.sunset.d", "Z indiga přes purpur do doruda žhnoucího obzoru."], "linear", 180, [["#1c1a4a", 0], ["#5b1f5e", 45], ["#8f2f35", 80], ["#a04a12", 100]]),
  P("ocean", "gradient", ["appearance.bd.p.ocean", "Oceán"], ["appearance.bd.p.ocean.d", "Hloubka od temné modři k tyrkysové mělčině."], "linear", 190, [["#04101c", 0], ["#0a3358", 55], ["#0b5d73", 100]]),
  P("lava", "gradient", ["appearance.bd.p.lava", "Láva"], ["appearance.bd.p.lava.d", "Žhavý střed dole, vychládající do černé."], "radial", 180, [["#b8460f", 0], ["#6e1a14", 38], ["#190a0a", 85]]),
  P("forest", "gradient", ["appearance.bd.p.forest", "Les"], ["appearance.bd.p.forest.d", "Mechová zeleň do tmavé jehličnaté."], "linear", 170, [["#08180f", 0], ["#14452a", 60], ["#2c4a14", 100]]),
  P("amethyst", "gradient", ["appearance.bd.p.amethyst", "Ametyst"], ["appearance.bd.p.amethyst.d", "Fialový krystal, od tmavé po zářivou."], "linear", 150, [["#1b1030", 0], ["#4a1d8c", 55], ["#7e3bc4", 100]]),
  P("rainbow", "gradient", ["appearance.bd.p.rainbow", "Mesh Duha"], ["appearance.bd.p.rainbow.d", "Pět měkkých barevných skvrn, které se prolínají."], "mesh", 0, [["#b4313b", 55], ["#a8680c", 50], ["#1f7a45", 55], ["#2a5fc0", 60], ["#7a38b8", 55]]),
  P("neon", "gradient", ["appearance.bd.p.neon", "Neonové město"], ["appearance.bd.p.neon.d", "Kuželový přechod azurová – fialová – růžová."], "conic", 0, [["#0c5a85", 0], ["#4b2a9a", 33], ["#8c2466", 66], ["#0c5a85", 100]]),
  P("space", "gradient", ["appearance.bd.p.space", "Hluboký vesmír"], ["appearance.bd.p.space.d", "Mlhoviny indigo, tyrkys a purpur na černi."], "mesh", 30, [["#2a2778", 60], ["#0b5566", 50], ["#4a1670", 55]]),
  P("dawn", "gradient", ["appearance.bd.p.dawn", "Svítání"], ["appearance.bd.p.dawn.d", "Fialová noc, která se rozednívá do broskve."], "linear", 165, [["#2a1740", 0], ["#6a2a58", 50], ["#8a4326", 100]]),
];

/** Živá pozadí: id = `appearance.backdrop.live`, `base` = gradient, který se vybere spolu s ním (hrají na něm). */
export type LivePreset = { id: LiveKind; name: Pair; desc: Pair; base: string; tone: string };

export const LIVE_PRESETS: readonly LivePreset[] = [
  { id: "fire", name: ["appearance.bd.l.fire", "Oheň"], desc: ["appearance.bd.l.fire.d", "Plameny zespodu a jiskry. Barvu plamene vybereš níž."], base: "ember", tone: "#ff8a2a" },
  { id: "aurora", name: ["appearance.bd.l.aurora", "Polární záře"], desc: ["appearance.bd.l.aurora.d", "Pomalu vlnící se závěsy světla."], base: "midnight", tone: "#35d6a0" },
  { id: "lava", name: ["appearance.bd.l.lava", "Lávová lampa"], desc: ["appearance.bd.l.lava.d", "Pomalé měkké kapky, které se slévají a dělí."], base: "lava", tone: "#ff6a3a" },
  { id: "stars", name: ["appearance.bd.l.stars", "Hvězdy"], desc: ["appearance.bd.l.stars.d", "Hvězdné pole ve třech hloubkách: vzdálené hvězdy plují pomalu, blízké rychleji."], base: "midnight", tone: "#dbe6ff" },
  { id: "rain", name: ["appearance.bd.l.rain", "Déšť na skle"], desc: ["appearance.bd.l.rain.d", "Šikmé čáry deště a kapky stékající po okně."], base: "fog", tone: "#9fb4d6" },
  { id: "grain", name: ["appearance.bd.l.grain", "Filmové zrno"], desc: ["appearance.bd.l.grain.d", "Jemný šum filmu nad gradientem."], base: "ink", tone: "#cccccc" },
];

/** Základ pro oheň (skrytý předvolený gradient, v galerii se nenabízí samostatně). */
export const HIDDEN: readonly Preset[] = [
  P("ember", "gradient", ["appearance.bd.p.ember", "Žhavé uhlí"], ["appearance.bd.p.ember.d", "Tmavý základ pod ohněm."], "linear", 180, [["#120806", 0], ["#2a0e08", 100]]),
];

export const FLAME_NAMES: Record<FlameColor, Pair> = {
  classic: ["appearance.bd.f.classic", "Klasický"],
  blue: ["appearance.bd.f.blue", "Modrý plyn"],
  green: ["appearance.bd.f.green", "Zelený"],
  violet: ["appearance.bd.f.violet", "Fialový"],
};

/** Barvy plamene: [okraj, střed, špička] jako `#rrggbb`, od chladné části k nejžhavější. */
export const FLAME_COLORS: Record<FlameColor, readonly [string, string, string]> = {
  classic: ["#b3260a", "#ff8a1c", "#ffe9a8"],
  blue: ["#1237a8", "#2f8cff", "#d6f0ff"],
  green: ["#0d7a2e", "#3fe06a", "#e6ffd9"],
  violet: ["#6a1aa8", "#c05cff", "#f6e3ff"],
};

export const presetById = (id: string): Preset | undefined => PRESETS.find((p) => p.id === id) ?? HIDDEN.find((p) => p.id === id);

export const asGradient = (p: Preset): Omit<CustomGradient, "id" | "name"> => ({ kind: p.kind, angle: p.angle, stops: p.stops.map(([c, pos]) => ({ c, p: pos })) });
