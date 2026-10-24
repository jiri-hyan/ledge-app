// Vzhled jedné karty na záložce (vlna 8 P, `layout.pages[].cards[].look`, zrcadlo `tilelook::CardLook`):
// akcent, pozadí, skrytá hlavička, hustota, varianta zobrazení; kolo 22: plocha (sklo / plné), jemné zrnění
// a „číslo jako hrdina“ (jen karty z `HERO_CARDS`). Realizace OBECNĚ na `.deck-item`
// (`Deck.tsx` → atributy `data-cl-*` a `--accent` na položce, pravidla v `layout.css`) — funguje pro
// každou kartu, i nové karty jiných workerů; jen varianta zobrazení je per karta (`VIEW_CARDS`).
// Karta bez `look` = žádný atribut, přesně jako dřív. Čistá logika bez importů za běhu (test `look.test.mjs`).

export type CardBg = "tint" | "solid" | "clear";
export type CardDensity = "compact" | "airy";
export type CardView = "list" | "tiles" | "bento" | "compact";
/** Plocha karty 2026 (kolo 22): „glass“ = sklo jen na rámu a hlavičce, pod textem plná stabilní plocha;
 *  „layer“ = plná vrstva s hloubkou. Tokeny `--k-*` dává materiál stylu (`theme/looks.ts` `MATERIALS`). */
export type CardSurface = "glass" | "layer";
export type CardLook = { accent?: string; bg?: CardBg; hideHead?: boolean; density?: CardDensity; view?: CardView; surface?: CardSurface; grain?: boolean; hero?: boolean };
export type Text = readonly [string, string];

/** Akcent karty: barvy motivu (zrcadlo `CARD_ACCENTS` v tilelook.rs) nebo vlastní `#rrggbb`. */
export const CARD_ACCENTS = ["accent", "blue", "green", "amber", "red", "violet", "teal"] as const;
export const ACCENT_NAMES: Record<(typeof CARD_ACCENTS)[number], Text> = {
  accent: ["cards.cardlook.accent.accent", "Akcent vzhledu"],
  blue: ["cards.cardlook.accent.blue", "Modrá"],
  green: ["cards.cardlook.accent.green", "Zelená"],
  amber: ["cards.cardlook.accent.amber", "Jantarová"],
  red: ["cards.cardlook.accent.red", "Červená"],
  violet: ["cards.cardlook.accent.violet", "Fialová"],
  teal: ["cards.cardlook.accent.teal", "Tyrkysová"],
};
/** Pozadí: „style“ = podle stylu (pole chybí). 4 volby → segment. */
export const BGS = ["style", "tint", "solid", "clear"] as const;
export const BG_NAMES: Record<(typeof BGS)[number], Text> = {
  style: ["cards.cardlook.bg.style", "Podle stylu"],
  tint: ["cards.cardlook.bg.tint", "Tón akcentu"],
  solid: ["cards.cardlook.bg.solid", "Plné"],
  clear: ["cards.cardlook.bg.clear", "Průhledné"],
};
export const DENSITIES = ["style", "compact", "airy"] as const;
export const DENSITY_NAMES: Record<(typeof DENSITIES)[number], Text> = {
  style: ["cards.cardlook.dens.style", "Podle stylu"],
  compact: ["cards.cardlook.dens.compact", "Kompaktní"],
  airy: ["cards.cardlook.dens.airy", "Vzdušná"],
};
/** Plocha karty: „style“ = podle stylu (pole chybí). 3 volby → segment. */
export const SURFACES = ["style", "glass", "layer"] as const;
export const SURFACE_NAMES: Record<(typeof SURFACES)[number], Text> = {
  style: ["cards.cardlook.surface.style", "Podle stylu"],
  glass: ["cards.cardlook.surface.glass", "Sklo"],
  layer: ["cards.cardlook.surface.layer", "Plné"],
};
/** Karty s hlavním číslem, které umí „číslo jako hrdina“ (`hero.ts` `heroFacts` — jen skutečná čísla). */
export const HERO_CARDS: readonly string[] = ["claude", "weather"];
export const heroable = (cardId: string) => HERO_CARDS.includes(cardId);
export const VIEWS = ["list", "tiles", "bento", "compact"] as const;
export const VIEW_NAMES: Record<CardView, Text> = {
  list: ["cards.cardlook.view.list", "Seznam"],
  tiles: ["cards.cardlook.view.tiles", "Dlaždice"],
  bento: ["cards.cardlook.view.bento", "Bento"],
  compact: ["cards.cardlook.view.compact", "Kompaktní"],
};
/** Karty s variantou zobrazení a jejich výchozí (= dnešní) varianta. */
export const VIEW_CARDS: Readonly<Record<string, CardView>> = { projects: "tiles", repos: "list", notes: "list", actions: "tiles" };
/** Bento (kolo 18: velká dlaždice + menší kolem) mají jen karty, které ho opravdu kreslí. */
const BENTO_CARDS = new Set(["projects", "repos", "notes"]);
/** Varianty, které karta nabízí (prázdné = karta bez variant). */
export const viewsOf = (cardId: string): readonly CardView[] => (cardId in VIEW_CARDS ? VIEWS.filter((v) => v !== "bento" || BENTO_CARDS.has(cardId)) : []);

const isHex = (s: unknown): s is string => typeof s === "string" && /^#[0-9a-fA-F]{6}$/.test(s);
const oneOf = <T extends string>(list: readonly T[], x: unknown): x is T => typeof x === "string" && (list as readonly string[]).includes(x);

/** Obranné čtení; prázdný vzhled = `undefined`. Varianta jen u karet z `VIEW_CARDS` a jen jiná než výchozí. */
export function normalizeCardLook(raw: unknown, cardId: string): CardLook | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const r = raw as Record<string, unknown>;
  const out: CardLook = {};
  if (oneOf(CARD_ACCENTS, r.accent) || isHex(r.accent)) out.accent = (r.accent as string).toLowerCase();
  if (oneOf(["tint", "solid", "clear"] as const, r.bg)) out.bg = r.bg;
  if (r.hideHead === true) out.hideHead = true;
  if (oneOf(["compact", "airy"] as const, r.density)) out.density = r.density;
  const def = VIEW_CARDS[cardId];
  if (def && oneOf(viewsOf(cardId), r.view) && r.view !== def) out.view = r.view;
  if (oneOf(["glass", "layer"] as const, r.surface)) out.surface = r.surface;
  if (r.grain === true) out.grain = true;
  if (r.hero === true && heroable(cardId)) out.hero = true;
  return Object.keys(out).length ? out : undefined;
}

/** Změna vzhledu karty (`undefined` / „style“ v patchi = pole pryč). */
export function patchCardLook(l: CardLook | undefined, cardId: string, patch: Partial<Record<keyof CardLook, unknown>>): CardLook | undefined {
  const next: Record<string, unknown> = { ...(l ?? {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v === undefined || v === false || v === "style") delete next[k];
    else next[k] = v;
  }
  return normalizeCardLook(next, cardId);
}

/** Varianta zobrazení karty (výchozí, když není zvolená); karta bez variant = undefined. */
export const viewOf = (cardId: string, l: CardLook | undefined): CardView | undefined => (VIEW_CARDS[cardId] ? (l?.view ?? VIEW_CARDS[cardId]) : undefined);

/** CSS hodnota akcentu: token motivu, nebo vlastní barva. */
export const accentCss = (a: string): string => (isHex(a) ? a : a === "accent" ? "var(--accent)" : `var(--${a})`);

/** Atributy a proměnné pro `.deck-item` (prázdné = karta jako dřív). */
export function cardLookAttrs(l: CardLook | undefined): { attrs: Record<string, string>; style: Record<string, string> } {
  const attrs: Record<string, string> = {};
  const style: Record<string, string> = {};
  if (!l) return { attrs, style };
  if (l.accent) {
    attrs["data-cl-accent"] = "";
    style["--accent"] = accentCss(l.accent);
  }
  if (l.bg) attrs["data-cl-bg"] = l.bg;
  if (l.hideHead) attrs["data-cl-head"] = "off";
  if (l.density) attrs["data-cl-dens"] = l.density;
  if (l.surface) attrs["data-cl-surface"] = l.surface;
  if (l.grain) attrs["data-cl-grain"] = "";
  if (l.hero) attrs["data-cl-hero"] = "";
  return { attrs, style };
}

type Card = { id: string; look?: CardLook };
type Page = { id: string; cards: Card[] };

/** Rozložení se změněným vzhledem karty `cardId` na záložce `pageId` (prázdný = pole pryč). */
export function setCardLook<L extends { pages: Page[] }>(l: L, pageId: string, cardId: string, look: CardLook | undefined): L {
  const norm = normalizeCardLook(look, cardId);
  return {
    ...l,
    pages: l.pages.map((p) =>
      p.id !== pageId
        ? p
        : {
            ...p,
            cards: p.cards.map((c) => {
              if (c.id !== cardId) return c;
              const out = { ...c };
              if (norm) out.look = norm;
              else delete out.look;
              return out;
            }),
          },
    ),
  };
}

/** Karty s vlastním vzhledem (přehled v Nastavení). */
export const customizedCards = (l: { pages: Page[] }): { pageId: string; cardId: string }[] =>
  l.pages.flatMap((p) => p.cards.filter((c) => normalizeCardLook(c.look, c.id)).map((c) => ({ pageId: p.id, cardId: c.id })));

/** Všechny karty podle stylu (Obnovit vše). */
export function resetCardLooks<L extends { pages: Page[] }>(l: L): L {
  let out = l;
  for (const c of customizedCards(l)) out = setCardLook(out, c.pageId, c.cardId, undefined);
  return out;
}

/** Předvolby vzhledu karet 2026 (galerie v Nastavení, kolo 22): plocha, zrnění, hrdina pro VŠECHNY karty jedné
 *  záložky najednou. Ostatní pole vzhledu (akcent, pozadí, hustota…) zůstanou. „style“ = zpět podle stylu. */
export type LookPreset = { id: string; name: Text; desc: Text; patch: { surface?: CardSurface; grain?: boolean; hero?: boolean } };
export const LOOK_PRESETS: readonly LookPreset[] = [
  { id: "style", name: ["cards.looks.style", "Podle stylu"], desc: ["cards.looks.style.desc", "Karty přesně podle design stylu, jako dřív."], patch: {} },
  { id: "layer", name: ["cards.looks.layer", "Plné vrstvy"], desc: ["cards.looks.layer.desc", "Neprůhledné karty s jemnou hloubkou a vlasovou linkou."], patch: { surface: "layer" } },
  { id: "glass", name: ["cards.looks.glass", "Sklo na rámu"], desc: ["cards.looks.glass.desc", "Průsvitný jen rám a hlavička, pod textem plná plocha — čitelné."], patch: { surface: "glass" } },
  { id: "layerGrain", name: ["cards.looks.layerGrain", "Plné se zrněním"], desc: ["cards.looks.layerGrain.desc", "Plné vrstvy s jemným zrněním jako papír."], patch: { surface: "layer", grain: true } },
  { id: "glassGrain", name: ["cards.looks.glassGrain", "Sklo se zrněním"], desc: ["cards.looks.glassGrain.desc", "Sklo na rámu s jemným zrněním."], patch: { surface: "glass", grain: true } },
  { id: "hero", name: ["cards.looks.hero", "Čísla jako hrdina"], desc: ["cards.looks.hero.desc", "Plné vrstvy a u karet s hlavním číslem (limit Claude, počasí) velké číslo s grafem."], patch: { surface: "layer", hero: true } },
];

/** Předvolba na všechny karty záložky `pageId` (pole předvolby se přepíšou, chybějící = pryč). */
export function applyLookPreset<L extends { pages: Page[] }>(l: L, pageId: string, presetId: string): L {
  const pr = LOOK_PRESETS.find((x) => x.id === presetId);
  const page = l.pages.find((p) => p.id === pageId);
  if (!pr || !page) return l;
  let out = l;
  for (const c of page.cards) {
    const look = patchCardLook(c.look, c.id, { surface: pr.patch.surface, grain: pr.patch.grain, hero: pr.patch.hero });
    out = setCardLook(out, pageId, c.id, look);
  }
  return out;
}

/** Která předvolba odpovídá všem kartám záložky (pro zvýraznění v galerii); prázdná záložka / mix = null. */
export function presetOf(l: { pages: Page[] }, pageId: string): string | null {
  const page = l.pages.find((p) => p.id === pageId);
  if (!page?.cards.length) return null;
  const key = (lk: CardLook | undefined) => `${lk?.surface ?? ""}|${lk?.grain ? 1 : 0}`;
  for (const pr of LOOK_PRESETS) {
    const want = `${pr.patch.surface ?? ""}|${pr.patch.grain ? 1 : 0}`;
    const heroOk = (c: Card) => !heroable(c.id) || !!normalizeCardLook(c.look, c.id)?.hero === !!pr.patch.hero;
    if (page.cards.every((c) => key(normalizeCardLook(c.look, c.id)) === want && heroOk(c))) return pr.id;
  }
  return null;
}
