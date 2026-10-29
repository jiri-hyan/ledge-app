// Ledge Studio (kolo 24) — informační architektura jako čistá data: 8 částí v pořadí, jak se s Ledge
// potkáš (Proužek → Otevírání → Vzhled šuplíku → Obsah → Claude → Projekty a akce → Poznámky a plocha →
// Aplikace). Část = otázky; otázka = jeden „blok“ (dnešní panel, u Vzhledu jeden jeho krok). Každá volba
// (`labels.ts` `FIELDS`, prefix `placement.ts` `PLACEMENT`, `ADVANCED_ANCHORS`) leží v bloku právě jedné části
// — hlídá `parts.test.mjs`. Bez DOM a bez `t()` (texty jsou páry klíč + čeština, přeloží je `Studio.tsx`).
import { ADVANCED_ANCHORS, placeOf } from "../settings/placement.ts";
import { ANCHOR_STEP, type StepId } from "../settings/appearanceModel.ts";

export type PartId = "pruzek" | "otevirani" | "vzhled" | "obsah" | "claude" | "projekty" | "poznamky" | "aplikace";
/** Blok = vykreslitelná jednotka: dnešní panel (`settings/catalog.ts`), u Vzhledu jeho krok. */
export type BlockId =
  | "strip" | "hover" | "chovani" | "look" | "motion" | "suplik" | "osobni" | "claude" | "akce" | "poznamky" | "naplose"
  | "zaciname" | "spousteni" | "aplikace";
export type Tx = readonly [string, string];
/** `visible(cfg)` = predikát viditelnosti (chybí = vždy): příští kolo tím schová část / otázku vypnutého
 *  modulu nebo podle role, bez zásahu do komponent. */
export type Question = { block: BlockId; q: Tx; sub: Tx; primary?: boolean; visible?: (cfg: any) => boolean };
export type Part = { id: PartId; n: number; icon: string; name: Tx; line: Tx; questions: readonly Question[]; visible?: (cfg: any) => boolean };

export const PARTS: readonly Part[] = [
  {
    id: "pruzek", n: 1, icon: "strip", name: ["settings.studio.p.pruzek", "Proužek"], line: ["settings.studio.p.pruzek.line", "Zavřený Ledge u horní hrany — to, co vidíš celý den."],
    questions: [{ block: "strip", primary: true, q: ["settings.studio.q.strip", "Co má být vidět, když na Ledge nesaháš?"], sub: ["settings.studio.q.strip.sub", "Vyber obrázek; pod ním pozice, velikost a co proužek hlásí."] }],
  },
  {
    id: "otevirani", n: 2, icon: "open", name: ["settings.studio.p.otevirani", "Otevírání"], line: ["settings.studio.p.otevirani.line", "Jak se šuplík otevře a zase zavře — myší, klávesou nebo paletou."],
    questions: [
      { block: "hover", primary: true, q: ["settings.studio.q.hover", "Co se stane, když najedeš myší k hraně?"], sub: ["settings.studio.q.hover.sub", "Celý šuplík, kompaktní náhled, nebo roztažený ostrůvek."] },
      { block: "chovani", primary: true, q: ["settings.studio.q.behavior", "Jak ho otevřeš a zavřeš?"], sub: ["settings.studio.q.behavior.sub", "Spouštěč u hrany, prodlevy, klávesové zkratky a paleta."] },
    ],
  },
  {
    id: "vzhled", n: 3, icon: "look", name: ["settings.studio.p.vzhled", "Vzhled šuplíku"], line: ["settings.studio.p.vzhled.line", "Styl, barvy, písmo, šířka a pohyb otevřeného šuplíku."],
    questions: [
      { block: "look", primary: true, q: ["settings.studio.q.look", "Jak má šuplík vypadat?"], sub: ["settings.studio.q.look.sub", "Hotový styl jedním klikem; barvy, písmo a tvar pod Doladit."] },
      { block: "motion", primary: true, q: ["settings.studio.q.motion", "Jak se má pohybovat?"], sub: ["settings.studio.q.motion.sub", "Animace otevření a její rychlost."] },
    ],
  },
  {
    id: "obsah", n: 4, icon: "cards", name: ["settings.studio.p.obsah", "Obsah"], line: ["settings.studio.p.obsah.line", "Záložky a karty šuplíku a co v nich je."],
    questions: [
      { block: "suplik", primary: true, q: ["settings.studio.q.layout", "Co má šuplík ukazovat?"], sub: ["settings.studio.q.layout.sub", "Záložky, karty na nich, pořadí a velikost."] },
      { block: "osobni", primary: true, q: ["settings.studio.q.personal", "Co mají karty vědět?"], sub: ["settings.studio.q.personal.sub", "Město pro počasí, kalendáře, hudba, schránka a účty."] },
    ],
  },
  {
    id: "claude", n: 5, icon: "claude", name: ["settings.studio.p.claude", "Claude"], line: ["settings.studio.p.claude.line", "Propojení s Claude Code: schvalování, otázky, limity, oznámení a hlídač."],
    questions: [{ block: "claude", primary: true, q: ["settings.studio.q.claude", "Jak má Ledge pracovat s Claude Code?"], sub: ["settings.studio.q.claude.sub", "Žádosti o povolení, otázky Clauda, limity a oznámení — propojení jedním klikem."] }],
  },
  {
    id: "projekty", n: 6, icon: "projects", name: ["settings.studio.p.projekty", "Projekty a akce"], line: ["settings.studio.p.projekty.line", "Projekty, odkazy a příkazy na jedno kliknutí."],
    questions: [{ block: "akce", primary: true, q: ["settings.studio.q.actions", "Co chceš mít na jedno kliknutí?"], sub: ["settings.studio.q.actions.sub", "Projekt se spustí jedním klikem; odkaz otevře web, složku nebo aplikaci."] }],
  },
  {
    id: "poznamky", n: 7, icon: "notes", name: ["settings.studio.p.poznamky", "Poznámky a plocha"], line: ["settings.studio.p.poznamky.line", "Rychlé poznámky, lístky a widgety přímo na ploše."],
    questions: [
      { block: "poznamky", primary: true, q: ["settings.studio.q.notes", "Kam si odkládáš myšlenky?"], sub: ["settings.studio.q.notes.sub", "Složka poznámek, připomínky, kategorie a synchronizace."] },
      { block: "naplose", q: ["settings.studio.q.desk", "Co má viset přímo na ploše?"], sub: ["settings.studio.q.desk.sub", "Widgety na ploše Windows — vrstva, průhlednost, přichycení."] },
    ],
  },
  {
    id: "aplikace", n: 8, icon: "app", name: ["settings.studio.p.aplikace", "Aplikace"], line: ["settings.studio.p.aplikace.line", "Jazyk, spouštění, hry, záloha a nové verze."],
    questions: [
      { block: "zaciname", primary: true, q: ["settings.studio.q.start", "V jakém jazyce a s jakou nápovědou?"], sub: ["settings.studio.q.start.sub", "Jazyk rozhraní, klávesy a úvodní průvodce."] },
      { block: "spousteni", primary: true, q: ["settings.studio.q.startup", "Kdy se má Ledge spouštět a kdy ustoupit?"], sub: ["settings.studio.q.startup.sub", "Po startu Windows, šetření paměti, hry přes celou obrazovku."] },
      { block: "aplikace", q: ["settings.studio.q.app", "Záloha, obnova a nové verze"], sub: ["settings.studio.q.app.sub", "Přenes nastavení jinam, vrať výchozí, zkontroluj aktualizace."] },
    ],
  },
];

export const PART_IDS: readonly PartId[] = PARTS.map((p) => p.id);
export const partById = (id: string | null | undefined): Part | null => PARTS.find((p) => p.id === id) ?? null;

/** Blok → část (každý blok je v právě jedné otázce, hlídá test). */
export function partOfBlock(b: BlockId | null | undefined): PartId | null {
  if (!b) return null;
  return PARTS.find((p) => p.questions.some((q) => q.block === b))?.id ?? null;
}

/** Krok panelu Vzhled → blok Studia. */
const STEP_BLOCK: Record<StepId, BlockId> = { closed: "strip", hover: "hover", open: "look", motion: "motion" };

/** Klíče configu na stránce Vzhled (`placement.ts`) → krok; nejdelší prefix vyhrává. Volby s vlastním řádkem
 *  mají krok v `appearanceModel.ts` `ANCHOR_STEP` — tohle je jen pro cesty bez řádku (chyba validace, hledání klíče). */
export const VZHLED_PREFIX_STEP: readonly { prefix: string; step: StepId }[] = [
  { prefix: "appearance", step: "open" },
  { prefix: "appearance.strip", step: "closed" },
  { prefix: "appearance.motion", step: "motion" },
  { prefix: "strip", step: "closed" },
  { prefix: "timer.strip", step: "closed" },
  { prefix: "drawer.hover", step: "hover" },
  { prefix: "drawer.hoverExpandMs", step: "hover" },
  { prefix: "drawer.width", step: "open" },
  { prefix: "drawer.glass", step: "open" },
];

function vzhledStep(path: string): StepId {
  const own = ANCHOR_STEP[path];
  if (own) return own;
  let best: { prefix: string; step: StepId } | null = null;
  for (const e of VZHLED_PREFIX_STEP) {
    if ((path === e.prefix || path.startsWith(`${e.prefix}.`)) && (!best || e.prefix.length > best.prefix.length)) best = e;
  }
  return best ? best.step : "open";
}

/** Stránka (id panelu z `registry.ts`) → blok; Vzhled podle kroku cesty. */
function blockOfPage(page: string, path: string): BlockId | null {
  if (page === "vzhled") return STEP_BLOCK[vzhledStep(path)];
  if (page === "doporucene") return null;
  return page as BlockId;
}

/** Volba (id z `labels.ts`) nebo cesta configu → blok. `fieldPanel` = panel volby z `FIELDS` (null = není volba). */
export function blockOf(idOrPath: string | null | undefined, fieldPanel: (id: string) => string | null | undefined): BlockId | null {
  if (!idOrPath) return null;
  const own = fieldPanel(idOrPath);
  if (own) return blockOfPage(own, idOrPath);
  if (ADVANCED_ANCHORS.includes(idOrPath)) return null;
  const place = placeOf(idOrPath);
  return place ? blockOfPage(place.page, idOrPath) : null;
}

/** Stará stránka Nastavení (id panelu, i ze `PANEL_ALIASES`) → část; „vzhled“ vede na Vzhled šuplíku. */
export const PAGE_PART: Record<string, PartId | "home"> = {
  doporucene: "home",
  vzhled: "vzhled",
  suplik: "obsah",
  chovani: "otevirani",
  osobni: "obsah",
  claude: "claude",
  akce: "projekty",
  poznamky: "poznamky",
  naplose: "poznamky",
  zaciname: "aplikace",
  spousteni: "aplikace",
  aplikace: "aplikace",
};

export type Target = { part: PartId | "home"; anchor: string | null; guide: boolean };

/** Kam skočit: `home` · `guide` · id části · stará stránka (i alias) · kotva `f-<pole>` (i s `#`). Neznámé = null. */
export function resolveStudioTarget(
  raw: string | null | undefined,
  aliases: Record<string, string>,
  fieldPanel: (id: string) => string | null | undefined,
): Target | null {
  const id = (raw ?? "").replace(/^#/, "").trim();
  if (!id) return null;
  if (id === "home") return { part: "home", anchor: null, guide: false };
  if (id === "guide") return { part: "pruzek", anchor: null, guide: true };
  if (partById(id)) return { part: id as PartId, anchor: null, guide: false };
  const page = aliases[id] ?? id;
  if (PAGE_PART[page]) return { part: PAGE_PART[page], anchor: null, guide: false };
  if (!id.startsWith("f-")) return null;
  const field = id.slice(2);
  const part = partOfBlock(blockOf(field, fieldPanel)) ?? advancedAnchorPart(field);
  return part ? { part, anchor: field, guide: false } : null;
}

/** Volby bez klíče configu (`ADVANCED_ANCHORS`) — tlačítka v Pokročilé svých panelů. */
const ANCHOR_PART: Record<string, PartId> = { "backup.file": "aplikace", "about.files": "aplikace", "widgets.gallery": "projekty" };
export const advancedAnchorPart = (id: string): PartId | null => ANCHOR_PART[id] ?? null;

/** Kolik voleb v každé části se liší od výchozích (`paths` = změněné cesty z `history.ts` `diffPaths`). */
export function modifiedByPart(paths: readonly string[], fieldPanel: (id: string) => string | null | undefined): Record<PartId, number> {
  const out = Object.fromEntries(PART_IDS.map((p) => [p, 0])) as Record<PartId, number>;
  for (const p of paths) {
    const part = partOfBlock(blockOf(p, fieldPanel));
    if (part) out[part]++;
  }
  return out;
}

/** Další / předchozí část v pořadí (průvodce, klávesy); mimo okraj = null. */
export function stepPart(id: PartId, d: 1 | -1): PartId | null {
  const i = PART_IDS.indexOf(id);
  return PART_IDS[i + d] ?? null;
}

/** Průvodce: klíč paměti postupu (úložiště prohlížeče, try/catch u volajícího). */
export const GUIDE_KEY = "ledge.studio.guide";
/** Uložený postup → hotové části (jen známé, bez duplicit, v pořadí částí); cokoli rozbitého = nic. */
export function parseGuide(raw: string | null | undefined): PartId[] {
  try {
    const v = JSON.parse(raw ?? "[]");
    return Array.isArray(v) ? PART_IDS.filter((p) => v.includes(p)) : [];
  } catch {
    return [];
  }
}
/** Část hotová (nebo zase ne). */
export const withDone = (done: readonly PartId[], id: PartId, on = true): PartId[] => PART_IDS.filter((p) => (p === id ? on : done.includes(p)));

/** Viditelné části (predikát `visible`; dnes všechny). */
export const visibleParts = (cfg: any): Part[] => PARTS.filter((p) => !p.visible || p.visible(cfg));
/** Viditelné otázky části. */
export const visibleQuestions = (p: Part, cfg: any): Question[] => p.questions.filter((q) => !q.visible || q.visible(cfg));

/** Kroky průvodce jako data: `before` = kroky před Proužkem (příští kolo: „Co děláš na počítači?“), pak
 *  viditelné části v pořadí. Krok = id části nebo id vlastního kroku. */
export function guideSteps(cfg: any, before: readonly string[] = []): string[] {
  return [...before, ...visibleParts(cfg).map((p) => p.id)];
}
/** Další / předchozí krok průvodce (mimo okraj = null). */
export function guideStep(steps: readonly string[], cur: string, d: 1 | -1): string | null {
  const i = steps.indexOf(cur);
  return i < 0 ? null : steps[i + d] ?? null;
}
