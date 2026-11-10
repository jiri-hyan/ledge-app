// Vlastní lišta širokého notche (kolo 27, `layout.pages[].notchBar`, zrcadlo `src-tauri/src/notchbar.rs`): seznam
// sekcí na záložku nezávisle na kartách — normalizace, plán lišty pro stránku (vlastní, nebo dnešní odvozená
// z karet přes `notchSections`), úpravy (přidat / odebrat / přeskládat / volby sekce) a převod odvozené lišty na
// vlastní při první úpravě. Čistá logika bez DOM a bez importů za běhu mimo `.ts` (test `bar.test.mjs`).
import { notchSections, takenCards, SECTION_ORDER, type NotchData } from "./model.ts";
import { storedBarDesign } from "./designs.ts";

export const BAR_IDS = ["player", "days", "system", "volume", "limits", "weather", "meeting"] as const;
export type BarId = (typeof BAR_IDS)[number];
export const METRICS = ["cpu", "ram", "gpu", "disk", "net"] as const;
export type Metric = (typeof METRICS)[number];
/** Metriky Systému bez volby (dnešní lišta). */
export const DEFAULT_METRICS: readonly Metric[] = ["cpu", "ram", "disk"];
export const WEIGHTS = ["thin", "mid", "bold"] as const;
export type Weight = (typeof WEIGHTS)[number];
/** Barvy sekce bez vlastního hexu; chybí = `accent`. */
export const COLORS = ["accent", "load", "gradient"] as const;
export const MAX_SECTIONS = 6;

/** Sekce lišty; výchozí hodnoty se nezapisují (`design` výchozí, `color` accent, `weight` mid, metriky dnešní). */
export type BarSection = { id: BarId; design?: string; color?: string; weight?: Weight; smooth?: true; metrics?: Metric[]; seconds?: true };
export type NotchBar = { sections: BarSection[] };

export type BarPresetId = "hw_claude" | "media_day" | "agenda" | "cockpit";
export const BAR_PRESET_IDS: readonly BarPresetId[] = ["hw_claude", "media_day", "agenda", "cockpit"];
export const BAR_PRESETS: Readonly<Record<BarPresetId, NotchBar>> = {
  hw_claude: {
    sections: [
      { id: "system", design: "area", color: "load", smooth: true, metrics: ["cpu", "ram", "gpu"] },
      { id: "limits" },
      { id: "days" },
    ],
  },
  media_day: {
    sections: [
      { id: "player" },
      { id: "days" },
      { id: "weather", design: "range" },
      { id: "volume" },
    ],
  },
  agenda: {
    sections: [
      { id: "days", design: "sheet" },
      { id: "meeting" },
      { id: "limits", design: "ring" },
      { id: "weather" },
    ],
  },
  cockpit: {
    sections: [
      { id: "player" },
      { id: "system", design: "rings", color: "load", metrics: ["cpu", "ram", "gpu"] },
      { id: "limits" },
      { id: "days", design: "clock" },
      { id: "weather", design: "range" },
      { id: "volume", design: "eq" },
    ],
  },
};

const isId = (x: unknown): x is BarId => typeof x === "string" && (BAR_IDS as readonly string[]).includes(x);
const isHex = (x: unknown): x is string => typeof x === "string" && /^#[0-9a-fA-F]{6}$/.test(x);
export const colorOk = (x: unknown): x is string => isHex(x) || (typeof x === "string" && (COLORS as readonly string[]).includes(x));

/** Obranné čtení jedné sekce: neznámé id = null, neplatné / výchozí volby pryč. */
export function normalizeSection(raw: unknown): BarSection | null {
  const r = (raw ?? {}) as Record<string, unknown>;
  if (!isId(r.id)) return null;
  const out: BarSection = { id: r.id };
  const design = storedBarDesign(r.id, r.design);
  if (design) out.design = design;
  if (colorOk(r.color) && r.color !== "accent") out.color = r.color;
  if (r.weight === "thin" || r.weight === "bold") out.weight = r.weight;
  if (r.smooth === true) out.smooth = true;
  if (Array.isArray(r.metrics)) {
    const ms = [...new Set(r.metrics.filter((m): m is Metric => (METRICS as readonly string[]).includes(m as string)))];
    if (ms.length && JSON.stringify(ms) !== JSON.stringify(DEFAULT_METRICS)) out.metrics = ms;
  }
  if (r.seconds === true) out.seconds = true;
  return out;
}

/** Obranné čtení lišty: chybí / nesmysl = undefined (odvozená lišta), jinak sekce bez dvojic, nejvýš `MAX_SECTIONS`. */
export function normalizeNotchBar(raw: unknown): NotchBar | undefined {
  if (!raw || typeof raw !== "object" || !Array.isArray((raw as any).sections)) return undefined;
  const sections: BarSection[] = [];
  for (const s of (raw as any).sections) {
    const x = normalizeSection(s);
    if (x && !sections.some((y) => y.id === x.id) && sections.length < MAX_SECTIONS) sections.push(x);
  }
  return { sections };
}

export const cloneBar = (b: NotchBar): NotchBar => ({ sections: b.sections.map((s) => ({ ...s, ...(s.metrics ? { metrics: [...s.metrics] } : {}) })) });

/** Co lišta na stránce kreslí: `custom` = vlastní seznam (sekce i bez dat — prázdný stav), jinak dnešní odvození
 *  z karet záložky (jen sekce s daty). V obou případech se karty, jejichž sekci lišta kreslí, pod lištou nekreslí
 *  (`taken`); v configu zůstávají vždy. */
export type BarPlan = { sections: BarSection[]; taken: string[]; custom: boolean };

export function barPlan(bar: NotchBar | undefined, shown: readonly string[], data: NotchData): BarPlan {
  if (bar) {
    const own = SECTION_ORDER.filter((id) => bar.sections.some((x) => x.id === id));
    return { sections: bar.sections, taken: takenCards(shown, own), custom: true };
  }
  const ids = notchSections(shown, data);
  return { sections: ids.map((id) => ({ id })), taken: takenCards(shown, ids), custom: false };
}

/** První úprava odvozené lišty: z karet záložky vznikne vlastní seznam (jako by data byla). Karty se nemažou —
 *  mřížka je dál obsahuje, jen je ve Výřezu schová lišta, která jejich sekce kreslí. Vlastní lišta = kopie. */
export function materialize(bar: NotchBar | undefined, cards: readonly string[]): { bar: NotchBar } {
  if (bar) return { bar: cloneBar(bar) };
  return { bar: { sections: notchSections(cards, { media: true, volume: true }).map((id) => ({ id })) } };
}

/** Sekce, které jde přidat (na liště ještě nejsou; plná lišta = žádné). */
export const addable = (b: NotchBar): BarId[] => (b.sections.length >= MAX_SECTIONS ? [] : BAR_IDS.filter((id) => !b.sections.some((s) => s.id === id)));

export function addSection(b: NotchBar, id: BarId): NotchBar {
  return addable(b).includes(id) ? { sections: [...cloneBar(b).sections, { id }] } : cloneBar(b);
}

export const removeSection = (b: NotchBar, i: number): NotchBar => ({ sections: cloneBar(b).sections.filter((_, j) => j !== i) });

export function moveSection(b: NotchBar, from: number, to: number): NotchBar {
  const out = cloneBar(b).sections;
  if (from < 0 || from >= out.length || to < 0 || to >= out.length || from === to) return { sections: out };
  const [x] = out.splice(from, 1);
  out.splice(to, 0, x);
  return { sections: out };
}

/** Změna voleb sekce `i` (undefined = pole pryč); výsledek znovu normalizovaný (výchozí se nezapisuje). */
export function patchSection(b: NotchBar, i: number, patch: Partial<Record<keyof BarSection, unknown>>): NotchBar {
  return {
    sections: cloneBar(b).sections.map((s, j) => {
      if (j !== i) return s;
      const raw: Record<string, unknown> = { ...s, ...patch, id: s.id };
      for (const k of Object.keys(patch)) if (patch[k as keyof BarSection] === undefined) delete raw[k];
      return normalizeSection(raw) ?? s;
    }),
  };
}

/** Metriky Systému, které sekce kreslí (bez volby = dnešní CPU, RAM, Disk). */
export const metricsOf = (s: BarSection): readonly Metric[] => s.metrics ?? DEFAULT_METRICS;

/** Přepnout metriku (poslední zůstane — sekce bez metrik nedává smysl). Pořadí podle `METRICS`. */
export function toggleMetric(s: BarSection, m: Metric): Metric[] {
  const cur = metricsOf(s);
  const next = cur.includes(m) ? cur.filter((x) => x !== m) : [...cur, m];
  return next.length ? METRICS.filter((x) => next.includes(x)) : [...cur];
}

type PageLike = { id: string; cards: { id: string }[]; notchBar?: NotchBar };
/** Rozložení s novou lištou záložky `pageId` (undefined = zpět na odvozenou); karty zůstávají. */
export function withBar<L extends { pages: PageLike[] }>(l: L, pageId: string, bar: NotchBar | undefined): L {
  return {
    ...l,
    pages: l.pages.map((p) => {
      if (p.id !== pageId) return p;
      const out = { ...p };
      if (bar) out.notchBar = cloneBar(bar);
      else delete out.notchBar;
      return out;
    }),
  };
}
