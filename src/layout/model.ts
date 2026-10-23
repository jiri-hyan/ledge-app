// Rozložení šuplíku (`config.layout`, zrcadlo `src-tauri/src/layout.rs`): záložky, karty na nich (pořadí,
// šířka s|m|l|xl, výška `h` 1–4 řádky / podle obsahu, u Projektů volby dlaždic `tiles`) a skryté části karet. JEDINÝ model pro to, co šuplík ukazuje — `drawer.sections` je
// odvozené v Rustu (karta = na nějaké záložce). Čistá logika bez importů za běhu (test `model.test.mjs`):
// normalizace, úpravy (přesun, velikost, odebrat/přidat, záložky), katalog, co stránka ukáže, cíl tažení.
// Texty jako páry klíč + čeština, přeloží se až při vykreslení. Vedle vestavěných karet (`CARD_IDS`) jsou
// DYNAMICKÉ karty s id podle prefixu (dnes widgety ze skriptů `widget:<soubor>`, `widgets/model.ts`) —
// katalog je nabízí podle toho, co je na disku; Rust `layout::is_dynamic` hlídá stejný tvar.
import { isWidgetId } from "../widgets/model.ts";
// Texty jako páry klíč + čeština, přeloží se až při vykreslení. Vzhled karty (`look`) a složky dlaždic
// (`tiles.folders`) mají vlastní moduly `look.ts` a `folders.ts`; tady se jen normalizují a klonují.
import { normalizeCardLook, type CardLook } from "./look.ts";
import { normalizeFolders, type TileFolder } from "./folders.ts";


export const CARD_IDS = ["projects", "running", "claude", "repos", "notes", "actions", "shelf", "clock", "timer", "weather", "calendar", "links", "media", "github", "system", "mail", "today"] as const;
export type BuiltinCardId = (typeof CARD_IDS)[number];
/** Dynamická karta: widget ze skriptu (`widget:<soubor>`). */
export type DynCardId = `widget:${string}`;
export type CardId = BuiltinCardId | DynCardId;
/** Dynamická karta (id podle prefixu, ne z `CARD_IDS`)? */
export const isDynamic = (id: unknown): id is DynCardId => isWidgetId(id);
export const isBuiltin = (id: unknown): id is BuiltinCardId => typeof id === "string" && (CARD_IDS as readonly string[]).includes(id);
export const SIZES = ["s", "m", "l", "xl"] as const;
export type CardSize = (typeof SIZES)[number];
/** Výška karty v řádcích mřížky (`--deck-row`); chybí = podle obsahu. */
export const HEIGHTS = [1, 2, 3, 4] as const;
export type CardHeight = (typeof HEIGHTS)[number];
/** Velikost dlaždic v kartě Projekty (zástupci). */
export const TILE_SIZES = ["small", "tile", "large"] as const;
export type TileSize = (typeof TILE_SIZES)[number];
/** Volby karty Projekty na jedné záložce: velikost dlaždic, popisek, git, skryté projekty (prázdné = všechny),
 *  složky zástupců (chybí = žádné). */
export type TileOpts = { size: TileSize; label: boolean; git: boolean; hidden: string[]; folders?: TileFolder[] };
/** `limit` = kolik řádků karta ukáže, než nabídne „Zobrazit další“ (0 = bez stropu, chybí = `DEFAULT_LIMIT`). */
export type LayoutCard = { id: CardId; size: CardSize; h?: CardHeight; tiles?: TileOpts; look?: CardLook; limit?: number; parts?: string[] };
export type LayoutPage = { id: string; name: string; icon?: string; cards: LayoutCard[] };
/** `upgrades` = jednorázové úpravy, které už proběhly (Rust `apply_upgrades`) — musí přežít každé uložení. */
export type Layout = { pages: LayoutPage[]; hiddenParts: string[]; upgrades?: string[] };
export const UP_PERSONAL_PROJECTS = "personalProjects";
/** Strop řádků karty (`limit`): nejvýš (zrcadlo `layout::MAX_LIMIT`), výchozí, karty, které ho mají. */
export const MAX_LIMIT = 50;
export const DEFAULT_LIMIT = 4;
export const LIMIT_CARDS: readonly CardId[] = ["claude", "repos", "running"];
/** Rychlé volby stropu v ⋯ (0 = Vše). */
export const QUICK_LIMITS: readonly number[] = [3, 4, 6, 0];
export const DEFAULT_TILES: Readonly<TileOpts> = { size: "tile", label: true, git: true, hidden: [] };

export const WORK = "prace";
export const PERSONAL = "osobni";
export const MAX_PAGES = 9;
export const MAX_NAME = 32;

/** Části karet, které jdou skrýt zvlášť: klíč (= pole `drawer.sections`) → karta. */
export const PARTS = {
  claudeLimits: "claude",
  claudeCosts: "claude",
  claudeGit: "claude",
  claudeSubagents: "claude",
  nowPlaying: "media",
  sound: "media",
  favorites: "media",
  battery: "clock",
} as const satisfies Record<string, CardId>;
export type PartKey = keyof typeof PARTS;
export const PART_KEYS = Object.keys(PARTS) as PartKey[];
/** Pseudo-část „hlavní obsah karty“ (seznam session, hodiny) — drží své místo v pořadí částí (`parts`). */
export const BODY = "body";
/** Části karty, které jdou v šuplíku přetahovat — výchozí pořadí shora dolů (zrcadlo `layout::card_slots`).
 *  `claudeGit` a `claudeSubagents` tu nejsou: kreslí se u každé session zvlášť, nejsou to bloky. */
export const CARD_SLOTS: Readonly<Record<string, readonly string[]>> = {
  claude: ["claudeLimits", BODY, "claudeCosts"],
  media: ["nowPlaying", "sound", "favorites"],
  clock: [BODY, "battery"],
};
export const slotsOf = (id: CardId): readonly string[] => CARD_SLOTS[id] ?? [];
/** Karta, která bez svých částí nic neukáže (vrácená z katalogu = s částmi). */
const PARTS_ONLY: readonly CardId[] = ["media"];

export type Text = readonly [string, string];
/** Název, jednořádkový popis a výchozí velikost karty (katalog, Nastavení, potvrzení „… odebrána“). */
export const CARD_INFO: Record<BuiltinCardId, { name: Text; desc: Text; size: CardSize }> = {
  projects: { name: ["cards.layout.card.projects", "Projekty"], desc: ["cards.layout.desc.projects", "Zástupci projektů — spustit, zastavit, otevřít"], size: "xl" },
  running: { name: ["cards.layout.card.running", "Běží"], desc: ["cards.layout.desc.running", "Servery spuštěné z Ledge; bez nich se neukáže"], size: "xl" },
  claude: { name: ["cards.layout.card.claude", "Claude"], desc: ["cards.layout.desc.claude", "Session, subagenti, schvalování, limity a útrata"], size: "m" },
  repos: { name: ["cards.layout.card.repos", "Repa"], desc: ["cards.layout.desc.repos", "Neuložená a nepushnutá práce, repa pozadu"], size: "m" },
  notes: { name: ["cards.layout.card.notes", "Poznámky"], desc: ["cards.layout.desc.notes", "Poslední poznámky a připomínky na dnes"], size: "xl" },
  actions: { name: ["cards.layout.card.actions", "Odkazy a akce"], desc: ["cards.layout.desc.actions", "Weby, složky, aplikace a příkazy na jedno kliknutí"], size: "xl" },
  shelf: { name: ["cards.layout.card.shelf", "Přihrádka"], desc: ["cards.layout.desc.shelf", "Soubory přetažené na proužek; prázdná se neukáže"], size: "xl" },
  clock: { name: ["cards.layout.card.clock", "Čas"], desc: ["cards.layout.desc.clock", "Velké hodiny s týdnem, na notebooku i baterie"], size: "s" },
  timer: { name: ["cards.layout.card.timer", "Časovač"], desc: ["cards.layout.desc.timer", "Odpočet a stopky"], size: "s" },
  weather: { name: ["cards.layout.card.weather", "Počasí"], desc: ["cards.layout.desc.weather", "Teplota a výhled; stahuje se, jen když je vidět"], size: "s" },
  calendar: { name: ["cards.layout.card.calendar", "Kalendář"], desc: ["cards.layout.desc.calendar", "Dnes a zítra z odkazu ICS (Google, Outlook)"], size: "s" },
  links: { name: ["cards.layout.card.links", "Odkazy"], desc: ["cards.layout.desc.links", "Weby, složky a soubory z tvých akcí, s ikonou webu"], size: "m" },
  media: { name: ["cards.layout.card.media", "Právě hraje"], desc: ["cards.layout.desc.media", "Skladba nebo video z libovolné aplikace a hlasitost"], size: "m" },
  github: { name: ["cards.layout.card.github", "GitHub"], desc: ["cards.layout.desc.github", "PR k revizi, tvoje PR s CI a notifikace z přihlášeného gh (výchozí vypnuto)"], size: "m" },
  system: { name: ["cards.layout.card.system", "Systém"], desc: ["cards.layout.desc.system", "CPU, paměť, disky a síť s malými grafy; měří jen když je vidět"], size: "m" },
  // výchozí nepřidaná (Rust `layout::OPT_IN`) — potřebuje přihlášený účet Microsoft
  mail: { name: ["cards.layout.card.mail", "Pošta"], desc: ["cards.layout.desc.mail", "Nepřečtené a nejnovější předměty z Outlooku (účet Microsoft)"], size: "m" },
  // jen z katalogu (výchozí rozložení ji nemá); bez dat se nekreslí (`Facts.today`)
  today: { name: ["cards.layout.card.today", "Dnes"], desc: ["cards.layout.desc.today", "Další schůzka, připomínky na dnes, PR k revizi a reset limitu Claude — jen to, co dnes něco znamená"], size: "m" },
};
export const PART_INFO: Record<PartKey, Text> = {
  claudeLimits: ["cards.layout.part.claudeLimits", "Limity 5h / týden"],
  claudeCosts: ["cards.layout.part.claudeCosts", "Hodnota v cenách API"],
  claudeGit: ["cards.layout.part.claudeGit", "Git panel"],
  claudeSubagents: ["cards.layout.part.claudeSubagents", "Subagenti"],
  nowPlaying: ["cards.layout.part.nowPlaying", "Skladba"],
  sound: ["cards.layout.part.sound", "Hlasitost"],
  favorites: ["cards.layout.part.favorites", "Oblíbené"],
  battery: ["cards.layout.part.battery", "Baterie"],
};
/** Malá sada ikon pro vlastní záložku (emoji, žádný soubor). */
export const PAGE_ICONS = ["", "💼", "🏠", "🎮", "🎵", "📚", "🛠️", "⭐", "🌿", "💬"] as const;

const isCard = (x: unknown): x is CardId => isBuiltin(x) || isDynamic(x);
const isSize = (x: unknown): x is CardSize => typeof x === "string" && (SIZES as readonly string[]).includes(x);
const isHeight = (x: unknown): x is CardHeight => (HEIGHTS as readonly unknown[]).includes(x);
const isLimit = (x: unknown): x is number => Number.isInteger(x) && (x as number) >= 0 && (x as number) <= MAX_LIMIT;
const isTileSize = (x: unknown): x is TileSize => typeof x === "string" && (TILE_SIZES as readonly string[]).includes(x);
/** Výchozí velikost karty (dynamická = střední). */
export const defaultSize = (id: CardId): CardSize => (isBuiltin(id) ? CARD_INFO[id].size : "m");
const card = (id: CardId, size: CardSize = defaultSize(id)): LayoutCard => ({ id, size });

/** Výchozí rozložení NOVÉHO configu = čistý začátek (zrcadlo `layout::clean_default`; oba testy čtou
 *  `src-tauri/src/fixtures/layout-default.json`): Práce = Projekty + Poznámky (Claude a Repa přidá průvodce
 *  podle detekce), Osobní = Čas, Právě hraje, Poznámky, Odkazy. Karty bez zdroje jsou jen v katalogu. */
export function defaultLayout(): Layout {
  return {
    pages: [
      { id: WORK, name: "", cards: [card("projects", "xl"), card("notes", "xl")] },
      { id: PERSONAL, name: "", cards: [card("clock", "m"), card("media", "m"), card("notes", "m"), card("links", "m")] },
    ],
    hiddenParts: [],
    upgrades: [UP_PERSONAL_PROJECTS],
  };
}

/** Obranné čtení voleb dlaždic (chybějící pole = výchozí). */
export function normalizeTiles(raw: unknown): TileOpts {
  const r = (raw ?? {}) as Partial<Record<string, unknown>>;
  const hidden = (Array.isArray(r.hidden) ? r.hidden : []).filter((h): h is string => typeof h === "string" && !!h);
  const out: TileOpts = {
    size: isTileSize(r.size) ? r.size : DEFAULT_TILES.size,
    label: typeof r.label === "boolean" ? r.label : DEFAULT_TILES.label,
    git: typeof r.git === "boolean" ? r.git : DEFAULT_TILES.git,
    hidden: [...new Set(hidden)],
  };
  const folders = normalizeFolders(r.folders);
  if (folders.length) out.folders = folders;
  return out;
}
/** Volby dlaždic karty (bez `tiles` = výchozí). */
export const tilesOf = (c: LayoutCard | undefined): TileOpts => normalizeTiles(c?.tiles);

/** Obranné čtení (snapshot, config): neznámé karty a velikosti pryč, bez záložek = výchozí. */
export function normalizeLayout(raw: unknown): Layout {
  const r = (raw ?? {}) as Partial<Record<string, unknown>>;
  const pages: LayoutPage[] = [];
  for (const p of Array.isArray(r.pages) ? r.pages : []) {
    if (!p || typeof p !== "object" || typeof (p as any).id !== "string" || !(p as any).id) continue;
    const x = p as Record<string, unknown>;
    if (pages.some((q) => q.id === x.id)) continue;
    const cards: LayoutCard[] = [];
    for (const c of Array.isArray(x.cards) ? x.cards : []) {
      const id = (c as any)?.id;
      if (!isCard(id) || cards.some((k) => k.id === id)) continue;
      const out: LayoutCard = { id, size: isSize((c as any).size) ? (c as any).size : defaultSize(id) };
      if (isHeight((c as any).h)) out.h = (c as any).h;
      if (isLimit((c as any).limit)) out.limit = (c as any).limit;
      if ((c as any).tiles && typeof (c as any).tiles === "object") out.tiles = normalizeTiles((c as any).tiles);
      const look = normalizeCardLook((c as any).look, id);
      if (look) out.look = look;
      const parts = savedParts((c as any).parts, id);
      if (parts) out.parts = parts;
      cards.push(out);
    }
    const page: LayoutPage = { id: x.id as string, name: typeof x.name === "string" ? x.name : "", cards };
    if (typeof x.icon === "string" && x.icon) page.icon = x.icon;
    pages.push(page);
  }
  if (!pages.length) return defaultLayout();
  const hiddenParts = (Array.isArray(r.hiddenParts) ? r.hiddenParts : []).filter((h): h is string => typeof h === "string" && h in PARTS);
  const out: Layout = { pages, hiddenParts: [...new Set(hiddenParts)] };
  const upgrades = (Array.isArray(r.upgrades) ? r.upgrades : []).filter((u): u is string => typeof u === "string" && !!u);
  if (upgrades.length) out.upgrades = [...new Set(upgrades)];
  return out;
}

const cloneCard = (c: LayoutCard): LayoutCard => {
  const out = { ...c };
  if (c.tiles) out.tiles = { ...c.tiles, hidden: [...c.tiles.hidden], ...(c.tiles.folders ? { folders: c.tiles.folders.map((f) => ({ ...f, items: [...f.items] })) } : {}) };
  if (c.look) out.look = { ...c.look };
  if (c.parts) out.parts = [...c.parts];
  return out;
};
export const clone = (l: Layout): Layout => {
  const out: Layout = { pages: l.pages.map((p) => ({ ...p, cards: p.cards.map(cloneCard) })), hiddenParts: [...l.hiddenParts] };
  if (l.upgrades) out.upgrades = [...l.upgrades];
  return out;
};
export const sameLayout = (a: Layout, b: Layout): boolean => JSON.stringify(normalizeLayout(a)) === JSON.stringify(normalizeLayout(b));

/** Záložka podle id; neznámé (smazaná) = první (jako `layout::page` v Rustu). */
export const pageOf = (l: Layout, id: string): LayoutPage => l.pages.find((p) => p.id === id) ?? l.pages[0];
/** Druh záložky pro karty, které se liší: Práce (akce pro práci, poznámky po projektech), Osobní
 *  (akce pro osobní, obecné poznámky), vlastní = všechno. */
export const pageKind = (id: string): "work" | "personal" | "any" => (id === WORK ? "work" : id === PERSONAL ? "personal" : "any");
/** Název vestavěné záložky bez vlastního názvu (pár k přeložení); vlastní = null. */
export const builtinName = (p: LayoutPage): Text | null =>
  p.name.trim() ? null : p.id === WORK ? ["cards.head.work", "Práce"] : p.id === PERSONAL ? ["cards.head.personal", "Osobní"] : null;
export const pageHas = (p: LayoutPage, id: CardId): boolean => p.cards.some((c) => c.id === id);
export const placed = (l: Layout, id: CardId): boolean => l.pages.some((p) => pageHas(p, id));
export const partHidden = (l: Layout, k: PartKey): boolean => l.hiddenParts.includes(k);

const mapPage = (l: Layout, pageId: string, f: (p: LayoutPage) => LayoutPage): Layout => ({ ...l, pages: l.pages.map((p) => (p.id === pageId ? f(p) : p)) });

export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
  const out = [...list];
  if (from < 0 || from >= out.length || to < 0 || to >= out.length || from === to) return out;
  const [x] = out.splice(from, 1);
  out.splice(to, 0, x);
  return out;
}

// ---------- pořadí částí karty ----------

/** Pořadí částí karty shora dolů: uložené (neznámé a dvojité pryč), chybějící na konec ve výchozím pořadí. */
export function orderParts(saved: unknown, id: CardId): string[] {
  const slots = slotsOf(id);
  const out: string[] = [];
  for (const k of Array.isArray(saved) ? saved : []) if (typeof k === "string" && slots.includes(k) && !out.includes(k)) out.push(k);
  for (const k of slots) if (!out.includes(k)) out.push(k);
  return out;
}
/** Uložené pole `parts` v čisté podobě: úplné pořadí, a když je výchozí (nebo karta nemá části), `undefined`. */
function savedParts(raw: unknown, id: CardId): string[] | undefined {
  if (!Array.isArray(raw) || !raw.length) return undefined;
  const o = orderParts(raw, id);
  return o.length && o.some((k, i) => k !== slotsOf(id)[i]) ? o : undefined;
}
/** Pořadí částí karty na záložce (bez `parts` = výchozí). */
export const partsOf = (c: LayoutCard | undefined, id: CardId | undefined = c?.id): string[] => (id ? orderParts(c?.parts, id) : []);
/** Nastaví pořadí částí karty (výchozí pořadí = pole `parts` pryč). */
export const setParts = (l: Layout, pageId: string, id: CardId, order: readonly string[]): Layout =>
  mapPage(l, pageId, (p) => ({
    ...p,
    cards: p.cards.map((c) => {
      if (c.id !== id) return c;
      const out = { ...c };
      const next = savedParts(order, id);
      if (next) out.parts = next;
      else delete out.parts;
      return out;
    }),
  }));
/** Přesun části v rámci karty (index → index v úplném pořadí). */
export function movePart(l: Layout, pageId: string, id: CardId, from: number, to: number): Layout {
  const c = pageOf(l, pageId).cards.find((x) => x.id === id);
  return c ? setParts(l, pageId, id, moveItem(partsOf(c), from, to)) : l;
}

/** Přesun karty v rámci záložky (index → index). */
export const moveCard = (l: Layout, pageId: string, from: number, to: number): Layout => mapPage(l, pageId, (p) => ({ ...p, cards: moveItem(p.cards, from, to) }));

/** Další velikost v kruhu s → m → l → xl → s. */
export const nextSize = (s: CardSize): CardSize => SIZES[(SIZES.indexOf(s) + 1) % SIZES.length];
export const setSize = (l: Layout, pageId: string, id: CardId, size: CardSize): Layout =>
  mapPage(l, pageId, (p) => ({ ...p, cards: p.cards.map((c) => (c.id === id ? { ...c, size } : c)) }));

/** Výška karty (`undefined` = podle obsahu). */
export const setHeight = (l: Layout, pageId: string, id: CardId, h: CardHeight | undefined): Layout =>
  mapPage(l, pageId, (p) => ({
    ...p,
    cards: p.cards.map((c) => {
      if (c.id !== id) return c;
      const out = { ...c };
      if (h === undefined) delete out.h;
      else out.h = h;
      return out;
    }),
  }));
/** Další výška v kruhu podle obsahu → 1 → 2 → 3 → 4 → podle obsahu (tlačítko ↕ v úpravách). */
export const nextHeight = (h: CardHeight | undefined): CardHeight | undefined => (h === undefined ? 1 : h >= 4 ? undefined : ((h + 1) as CardHeight));
/** Strop řádků karty (chybí = výchozí 4; 0 = bez stropu). */
export const limitOf = (c: LayoutCard | undefined): number => (c?.limit ?? DEFAULT_LIMIT);
/** Nastavit strop řádků (`undefined` nebo výchozí = pole pryč). */
export const setLimit = (l: Layout, pageId: string, id: CardId, n: number | undefined): Layout =>
  mapPage(l, pageId, (p) => ({
    ...p,
    cards: p.cards.map((c) => {
      if (c.id !== id) return c;
      const out = { ...c };
      if (n === undefined || n === DEFAULT_LIMIT || !isLimit(n)) delete out.limit;
      else out.limit = n;
      return out;
    }),
  }));
/** Rychlé volby výšky v ⋯ menu: Auto / Nízká / Vysoká. */
export const QUICK_HEIGHTS: readonly { h: CardHeight | undefined; name: Text }[] = [
  { h: undefined, name: ["cards.layout.height.auto", "Auto"] },
  { h: 2, name: ["cards.layout.height.low", "Nízká"] },
  { h: 4, name: ["cards.layout.height.high", "Vysoká"] },
];

/** Změna voleb dlaždic karty Projekty na záložce (výchozí volby = pole `tiles` pryč). */
export function setTiles(l: Layout, pageId: string, patch: Partial<TileOpts>): Layout {
  return mapPage(l, pageId, (p) => ({
    ...p,
    cards: p.cards.map((c) => {
      if (c.id !== "projects") return c;
      const next = normalizeTiles({ ...tilesOf(c), ...patch });
      const out: LayoutCard = { ...c, tiles: next };
      if (JSON.stringify(next) === JSON.stringify(normalizeTiles(DEFAULT_TILES))) delete out.tiles;
      return out;
    }),
  }));
}
/** Ukázat / skrýt jeden projekt na záložce. */
export function showProject(l: Layout, pageId: string, projectId: string, shown: boolean): Layout {
  const t = tilesOf(pageOf(l, pageId).cards.find((c) => c.id === "projects"));
  const rest = t.hidden.filter((h) => h !== projectId);
  return setTiles(l, pageId, { hidden: shown ? rest : [...rest, projectId] });
}
/** Projekty, které dlaždice na téhle záložce ukáže (v pořadí configu). */
export const visibleProjects = <T extends { id: string }>(projects: readonly T[], t: TileOpts): T[] => projects.filter((p) => !t.hidden.includes(p.id));

export type CardUndo = { pageId: string; index: number; card: LayoutCard };
/** Odebere kartu ze záložky (do katalogu); `undo` ji vrátí na totéž místo. */
export function removeCard(l: Layout, pageId: string, id: CardId): { layout: Layout; undo: CardUndo | null } {
  const p = l.pages.find((x) => x.id === pageId);
  const index = p ? p.cards.findIndex((c) => c.id === id) : -1;
  if (!p || index < 0) return { layout: l, undo: null };
  return { layout: mapPage(l, pageId, (x) => ({ ...x, cards: x.cards.filter((c) => c.id !== id) })), undo: { pageId, index, card: { ...p.cards[index] } } };
}
/** Vrátí odebranou kartu (Vrátit); záložka mezitím zmizela = nic. */
export function restoreCard(l: Layout, u: CardUndo): Layout {
  return mapPage(l, u.pageId, (p) => {
    if (pageHas(p, u.card.id)) return p;
    const cards = [...p.cards];
    cards.splice(Math.min(u.index, cards.length), 0, { ...u.card });
    return { ...p, cards };
  });
}

/** Přidá kartu na konec záložky (z katalogu). Karta, která bez částí nic neukáže, se vrátí celá. */
export function addCard(l: Layout, pageId: string, id: CardId, size?: CardSize): Layout {
  const page = l.pages.find((p) => p.id === pageId);
  if (!page || pageHas(page, id)) return l;
  let hiddenParts = l.hiddenParts;
  if (PARTS_ONLY.includes(id)) {
    const own = PART_KEYS.filter((k) => PARTS[k] === id);
    if (own.every((k) => hiddenParts.includes(k))) hiddenParts = hiddenParts.filter((h) => !own.includes(h as PartKey));
  }
  return { ...mapPage(l, pageId, (p) => ({ ...p, cards: [...p.cards, card(id, size)] })), hiddenParts };
}

/** Proč kartu nejde přetáhnout na záložku `toPage`: `same` = je to tatáž, `missing` = záložka není,
 *  `there` = karta už tam je (na jedné záložce smí být jen jednou); null = jde. */
export function pageMoveBlock(l: Layout, fromPage: string, id: CardId, toPage: string): "same" | "missing" | "there" | null {
  if (fromPage === toPage) return "same";
  const to = l.pages.find((p) => p.id === toPage);
  if (!to) return "missing";
  return pageHas(to, id) ? "there" : null;
}

/** Přesun karty na jinou záložku (tažením na záložku): pryč ze zdrojové, na konec cílové se stejnou
 *  velikostí. Když už tam je, NIC se nezmění (vlna 9: dřív jen zmizela ze zdrojové — „karta zmizela“). */
export const moveToPage = (l: Layout, fromPage: string, id: CardId, toPage: string): Layout => moveToPageAt(l, fromPage, id, toPage, Number.MAX_SAFE_INTEGER);

/** Totéž, ale na určené místo cílové záložky (tah pokračoval v její mřížce); index mimo rozsah = na okraj. */
export function moveToPageAt(l: Layout, fromPage: string, id: CardId, toPage: string, index: number): Layout {
  if (pageMoveBlock(l, fromPage, id, toPage)) return l;
  const { layout, undo } = removeCard(l, fromPage, id);
  if (!undo) return l;
  return mapPage(layout, toPage, (p) => {
    const cards = [...p.cards];
    cards.splice(Math.max(0, Math.min(index, cards.length)), 0, undo.card);
    return { ...p, cards };
  });
}

/** Karta přetažená na „+“: vznikne nová záložka s tímto názvem a karta se do ní přesune (jinak nic — plno
 *  záložek, karta na zdroji není). */
export function moveToNewPage(l: Layout, fromPage: string, id: CardId, name: string): { layout: Layout; id: string } | null {
  if (!pageHas(pageOf(l, fromPage), id)) return null;
  const added = addPage(l, name);
  if (!added) return null;
  return { layout: moveToPageAt(added.layout, fromPage, id, added.id, 0), id: added.id };
}

/** Skrýt / ukázat část karty (`layout.hiddenParts`). */
export function setPartHidden(l: Layout, k: PartKey, hidden: boolean): Layout {
  const rest = l.hiddenParts.filter((h) => h !== k);
  return { ...l, hiddenParts: hidden ? [...rest, k] : rest };
}

/** Katalog pro záložku: karty, které na ní nejsou, a kde jsou jinde (prázdné = nikde, skrytá). */
export function catalog(l: Layout, pageId: string): { id: CardId; on: string[] }[] {
  const page = l.pages.find((p) => p.id === pageId);
  return CARD_IDS.filter((id) => !page || !pageHas(page, id)).map((id) => ({ id, on: l.pages.filter((p) => pageHas(p, id)).map((p) => p.id) }));
}

// ---------- záložky ----------

const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
/** Id nové záložky z názvu: a-z, 0-9, pomlčka; obsazené / prázdné → s číslem. */
export function pageIdFor(l: Layout, name: string): string {
  const base = fold(name).replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 20) || "zalozka";
  const taken = new Set(l.pages.map((p) => p.id));
  if (!taken.has(base)) return base;
  for (let i = 2; ; i++) if (!taken.has(`${base}-${i}`)) return `${base}-${i}`;
}

/** Chyba názvu záložky (pár k přeložení) nebo null. */
export function pageNameError(name: string, builtin = false): Text | null {
  if (!builtin && !name.trim()) return ["cards.layout.tab.nameEmpty", "Záložka potřebuje název"];
  if ([...name].length > MAX_NAME) return ["cards.layout.tab.nameLong", "Název je moc dlouhý"];
  return null;
}

export function addPage(l: Layout, name: string, icon = ""): { layout: Layout; id: string } | null {
  if (l.pages.length >= MAX_PAGES || pageNameError(name)) return null;
  const id = pageIdFor(l, name);
  const page: LayoutPage = { id, name: name.trim(), cards: [] };
  if (icon) page.icon = icon;
  return { layout: { ...l, pages: [...l.pages, page] }, id };
}

export function renamePage(l: Layout, id: string, name: string, icon?: string): Layout {
  const builtin = id === WORK || id === PERSONAL;
  if (pageNameError(name, builtin)) return l;
  return mapPage(l, id, (p) => {
    const out: LayoutPage = { ...p, name: name.trim() };
    const ic = icon ?? p.icon ?? "";
    if (ic) out.icon = ic;
    else delete out.icon;
    return out;
  });
}

export type PageUndo = { index: number; page: LayoutPage };
/** Smaže záložku (karty z ní se vrátí do katalogu). Poslední záložka zůstane. */
export function removePage(l: Layout, id: string): { layout: Layout; undo: PageUndo | null } {
  const index = l.pages.findIndex((p) => p.id === id);
  if (index < 0 || l.pages.length <= 1) return { layout: l, undo: null };
  return { layout: { ...l, pages: l.pages.filter((p) => p.id !== id) }, undo: { index, page: l.pages[index] } };
}
export function restorePage(l: Layout, u: PageUndo): Layout {
  if (l.pages.some((p) => p.id === u.page.id) || l.pages.length >= MAX_PAGES) return l;
  const pages = [...l.pages];
  pages.splice(Math.min(u.index, pages.length), 0, u.page);
  return { ...l, pages };
}
export const movePage = (l: Layout, from: number, to: number): Layout => ({ ...l, pages: moveItem(l.pages, from, to) });

// ---------- co stránka ukáže ----------

/** Fakta, podle kterých se karty bez dat schovají samy (v režimu úprav jsou vidět jako zástupce). */
export type Facts = {
  /** procesy Ledge (karta Běží) */
  procs: number;
  /** poznámky zapnuté celé (`notes.enabled`) */
  notesEnabled: boolean;
  /** rychlé akce pro tuhle záložku */
  actions: number;
  /** výstup skriptu z palety (karta Rychlé akce se kvůli němu ukáže i prázdná) */
  scriptOut: boolean;
  /** přihrádka má položky */
  shelf: boolean;
  /** karta Právě hraje má co ukázat (skladbu nebo hlasitost) */
  media: boolean;
  /** karta Dnes má aspoň jeden řádek (`cards/today.ts` `todayRows`) */
  today: boolean;
};

/** Karty záložky, které teď něco ukážou, v pořadí záložky. Prázdné akce zvou k přidání mimo Práci. */
export function shownCards(p: LayoutPage, f: Facts): LayoutCard[] {
  const kind = pageKind(p.id);
  const shows = (id: CardId): boolean => {
    switch (id) {
      case "running":
        return f.procs > 0;
      case "actions":
        return kind !== "work" || f.actions > 0 || f.scriptOut;
      case "shelf":
        return f.shelf;
      case "media":
        return f.media;
      case "today":
        return f.today;
      case "notes":
        return f.notesEnabled;
      default:
        return true;
    }
  };
  return p.cards.filter((c) => shows(c.id));
}

/** Config se změnou JEDNOHO pole `layout` (vstup zůstane netknutý). */
export const withLayout = <T extends Record<string, any>>(cfg: T, l: Layout): T => ({ ...cfg, layout: l });

// ---------- tažení v mřížce ----------

export type Rect = { x: number; y: number; w: number; h: number };
/** Jak daleko za kartami ještě platí cíl tažení (px); dál = karta zůstane, kde byla. */
export const DROP_MARGIN = 48;
/** Vzdálenost bodu od obdélníku (0 uvnitř). */
export function rectGap(r: Rect, x: number, y: number): number {
  const dx = Math.max(r.x - x, 0, x - (r.x + r.w));
  const dy = Math.max(r.y - y, 0, y - (r.y + r.h));
  return Math.hypot(dx, dy);
}

/** Cílový index při tažení karty `from` na bod (px, py): karta, nad kterou ukazatel je; v mezeře mezi kartami
 *  (i v prázdném místě řady) ta, ke které má blíž (vzdálenost k obrysu, ne ke středu — široká karta tak nepřebije
 *  úzkou vedle sebe). Bez karet nebo daleko mimo karty (víc než `DROP_MARGIN` za jejich obrysem) = `from`
 *  (vlna 9: dřív i tah mimo šuplík kartu přesunul). */
export function gridDropIndex(rects: readonly Rect[], from: number, px: number, py: number): number {
  if (!rects.length) return from;
  const left = Math.min(...rects.map((r) => r.x)) - DROP_MARGIN;
  const top = Math.min(...rects.map((r) => r.y)) - DROP_MARGIN;
  const right = Math.max(...rects.map((r) => r.x + r.w)) + DROP_MARGIN;
  const bottom = Math.max(...rects.map((r) => r.y + r.h)) + DROP_MARGIN;
  if (px < left || px > right || py < top || py > bottom) return from;
  let best = from;
  let bestD = Infinity;
  rects.forEach((r, i) => {
    const d = rectGap(r, px, py);
    if (d < bestD) {
      bestD = d;
      best = i;
    }
  });
  return best;
}

/** Index po Alt+šipce: ← ↑ = dřív, → ↓ = později; mimo rozsah = null. */
export function keyMove(key: string, i: number, n: number): number | null {
  const to = key === "ArrowLeft" || key === "ArrowUp" ? i - 1 : key === "ArrowRight" || key === "ArrowDown" ? i + 1 : null;
  return to === null || to < 0 || to >= n ? null : to;
}
