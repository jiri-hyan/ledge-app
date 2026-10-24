// Bento (kolo 22): záložka jako mřížka dlaždic různých velikostí (`layout.pages[].bento`). Rozšiřuje existující
// mřížku `.deck`: velikost dlaždice = šířka karty (`size` s/m/l/xl = 1/2/3/vše sloupců) × výška v řádcích (`h`,
// v bento chybí = 2) — žádné nové pole karty. Řádky jsou pevné a volná místa vyplní hustý tok mřížky
// (`layout.css` `.deck[data-bento]`). Šablony („Pracovní“, „Minimal“, „Hudba“) přidají NOVOU záložku (6–9
// dlaždic, největší = nejdůležitější); stávající záložky ani výchozí rozložení se nemění. Čistá logika bez DOM
// (test `bento.test.mjs`); texty jako páry [klíč, čeština].
import { addPage, MAX_PAGES, setHeight, setSize, type CardHeight, type CardId, type CardSize, type Layout, type LayoutCard, type LayoutPage } from "./model.ts";
import { normalizeCardLook, type CardLook } from "./look.ts";

export type Text = readonly [string, string];
export const TILES = ["1x1", "2x1", "2x2"] as const;
export type Tile = (typeof TILES)[number];
/** dlaždice → (šířka, výška v řádcích); „2 × 1“ = dva sloupce, jeden řádek */
export const TILE_DIM: Record<Tile, { size: CardSize; h: CardHeight }> = {
  "1x1": { size: "s", h: 1 },
  "2x1": { size: "m", h: 1 },
  "2x2": { size: "m", h: 2 },
};
export const TILE_NAMES: Record<Tile, Text> = {
  "1x1": ["cards.bento.tile.1x1", "1 × 1"],
  "2x1": ["cards.bento.tile.2x1", "2 × 1"],
  "2x2": ["cards.bento.tile.2x2", "2 × 2"],
};
/** Výška karty v bento, když `h` chybí (řádky jsou pevné). */
export const BENTO_DEFAULT_H: CardHeight = 2;

/** Dlaždice karty v bento; jiná kombinace (3 sloupce, 3 řádky…) = null („vlastní“). */
export function tileOf(c: Pick<LayoutCard, "size" | "h"> | undefined): Tile | null {
  if (!c) return null;
  const h = c.h ?? BENTO_DEFAULT_H;
  return TILES.find((t) => TILE_DIM[t].size === c.size && TILE_DIM[t].h === h) ?? null;
}

/** Karta na záložce jako dlaždice (šířka + výška najednou). */
export const setTile = (l: Layout, pageId: string, id: CardId, tile: Tile): Layout => setHeight(setSize(l, pageId, id, TILE_DIM[tile].size), pageId, id, TILE_DIM[tile].h);

/** Bento zapnout / vypnout na záložce (vypnuto = pole pryč, karty si velikosti nechají). */
export function setBento(l: Layout, pageId: string, on: boolean): Layout {
  return {
    ...l,
    pages: l.pages.map((p) => {
      if (p.id !== pageId) return p;
      const out: LayoutPage = { ...p };
      if (on) out.bento = true;
      else delete out.bento;
      return out;
    }),
  };
}

export type TemplateId = "work" | "minimal" | "music";
export type Template = { id: TemplateId; icon: string; name: Text; desc: Text; cards: { id: CardId; tile: Tile; look?: CardLook }[] };

// Pořadí = důležitost (první dlaždice největší). Karta bez dat se v šuplíku schová sama (`shownCards`) a hustý
// tok mezeru zavře — šablona nikdy neukazuje vymyšlená data.
export const TEMPLATES: readonly Template[] = [
  {
    id: "work",
    icon: "💼",
    name: ["cards.bento.tpl.work", "Pracovní"],
    desc: ["cards.bento.tpl.work.desc", "Claude a projekty velké, kolem dnešek, kalendář, poznámky, časovač a co běží."],
    cards: [
      { id: "claude", tile: "2x2", look: { surface: "layer" } },
      { id: "projects", tile: "2x2", look: { surface: "layer" } },
      { id: "today", tile: "2x1", look: { surface: "layer" } },
      { id: "calendar", tile: "2x1", look: { surface: "layer" } },
      { id: "notes", tile: "2x1", look: { surface: "layer" } },
      { id: "running", tile: "1x1", look: { surface: "layer" } },
      { id: "timer", tile: "1x1", look: { surface: "layer" } },
    ],
  },
  {
    id: "minimal",
    icon: "◻️",
    name: ["cards.bento.tpl.minimal", "Minimal"],
    desc: ["cards.bento.tpl.minimal.desc", "Velké hodiny, limit Claude a počasí jako velká čísla, týden, poznámky a časovač. Šest klidných dlaždic."],
    cards: [
      { id: "bigclock", tile: "2x2", look: { surface: "layer" } },
      { id: "claude", tile: "2x1", look: { surface: "layer", hero: true } },
      { id: "week", tile: "2x1", look: { surface: "layer" } },
      { id: "notes", tile: "2x1", look: { surface: "layer" } },
      { id: "weather", tile: "1x1", look: { surface: "layer", hero: true } },
      { id: "timer", tile: "1x1", look: { surface: "layer" } },
    ],
  },
  {
    id: "music",
    icon: "🎵",
    name: ["cards.bento.tpl.music", "Hudba"],
    desc: ["cards.bento.tpl.music.desc", "Velký přehrávač, kolem časovač, odkazy, poznámky, kalendář, čas a počasí. Sklo se zrněním."],
    cards: [
      { id: "media", tile: "2x2", look: { surface: "glass", grain: true } },
      { id: "timer", tile: "2x1", look: { surface: "glass", grain: true } },
      { id: "links", tile: "2x1", look: { surface: "glass", grain: true } },
      { id: "notes", tile: "2x1", look: { surface: "glass", grain: true } },
      { id: "calendar", tile: "2x1", look: { surface: "glass", grain: true } },
      { id: "clock", tile: "1x1", look: { surface: "glass", grain: true } },
      { id: "weather", tile: "1x1", look: { surface: "glass", grain: true } },
    ],
  },
];
export const templateOf = (id: string): Template | undefined => TEMPLATES.find((t) => t.id === id);

/** Karty šablony jako karty záložky (vzhled normalizovaný — hrdina jen u karet, které ho umí). */
export function templateCards(t: Template): LayoutCard[] {
  return t.cards.map((c) => {
    const out: LayoutCard = { id: c.id, size: TILE_DIM[c.tile].size, h: TILE_DIM[c.tile].h };
    const look = normalizeCardLook(c.look, c.id);
    if (look) out.look = look;
    return out;
  });
}

/** Šablona jako NOVÁ záložka na konci (název = název šablony); plno záložek = null. Ostatní záložky beze změny. */
export function addTemplatePage(l: Layout, t: Template, name: string): { layout: Layout; id: string } | null {
  if (l.pages.length >= MAX_PAGES) return null;
  const r = addPage(l, name, t.icon);
  if (!r) return null;
  return {
    id: r.id,
    layout: { ...r.layout, pages: r.layout.pages.map((p) => (p.id === r.id ? { ...p, bento: true as const, cards: templateCards(t) } : p)) },
  };
}
