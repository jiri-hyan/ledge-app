// Ledge Studio — domov „Tvůj Ledge“ jako čistá data (kolo 24): schematická kresba TVÉHO Ledge (záložky
// a karty z `layout.pages`, karta → část Studia, kde se nastavuje) a „Na co nezapomenout“ — chytré karty
// JEN ze skutečného stavu (config + snapshot jádra). Co nevíme, neukážeme (žádné divadlo). Bez DOM a `t()`.
// Test `anatomy.test.mjs`.
import type { PartId } from "./parts.ts";

/** Karta šuplíku → část Studia, kde se nastavuje její obsah (neznámé / widgety = Obsah). */
export const CARD_PART: Record<string, PartId> = {
  projects: "projekty", running: "projekty", repos: "projekty", actions: "projekty", links: "projekty",
  claude: "claude", usage: "claude", activity: "claude", week: "claude",
  notes: "poznamky",
};
export const cardPart = (id: string): PartId => CARD_PART[id] ?? "obsah";

/** Šířka karty v mřížce kresby (4 sloupce): s|m|l|xl. */
const SPAN: Record<string, number> = { s: 1, m: 2, l: 3, xl: 4 };

export type HomeCard = { id: string; span: number; part: PartId };
export type HomeTab = { id: string; name: string; on: boolean };
export type Home = { tabs: HomeTab[]; cards: HomeCard[]; more: number };

/** Kolik karet kresba ukáže (víc = „+n“). */
export const HOME_MAX_CARDS = 8;
/** Kolik záložek kresba ukáže. */
export const HOME_MAX_TABS = 6;

/** Kresba šuplíku z `layout` (normalizovaného, `layout/model.ts`): záložky (aktivní = `active`, jinak první)
 *  a karty aktivní záložky s šířkou a částí. Prázdný layout = prázdná kresba. */
export function homeModel(layout: { pages?: { id: string; name: string; cards?: { id: string; size?: string }[] }[] } | null | undefined, active?: string): Home {
  const pages = layout?.pages ?? [];
  const on = pages.find((p) => p.id === active) ?? pages[0];
  const tabs = pages.slice(0, HOME_MAX_TABS).map((p) => ({ id: p.id, name: p.name, on: p === on }));
  const all = on?.cards ?? [];
  const cards = all.slice(0, HOME_MAX_CARDS).map((c) => ({ id: c.id, span: Math.min(4, SPAN[c.size ?? "m"] ?? 2), part: cardPart(c.id) }));
  return { tabs, cards, more: Math.max(0, all.length - cards.length) };
}

/** Karty na některé záložce (pro připomínky: počasí bez města vadí jen, když karta počasí opravdu je). */
export function placedCards(layout: { pages?: { cards?: { id: string }[] }[] } | null | undefined): Set<string> {
  return new Set((layout?.pages ?? []).flatMap((p) => (p.cards ?? []).map((c) => c.id)));
}

/** Co jde opravit: `go` = kotva / část pro skok, `fix` = jednoklikové řešení (volající ví, co udělat). */
export type Reminder = { id: string; part: PartId; go: string; fix?: "autostart" | "hooks" | "update" };

/** Vstup pro připomínky — jen to, co jádro opravdu ví (chybějící údaj = žádná karta). */
export type ReminderInput = {
  cfg: any;
  hooks?: { state: string } | null;
  update?: { state: string } | null;
  /** běží v balíčku MSIX — autostart řídí Windows, nic nenabízet */
  packaged?: boolean;
};

/** „Na co nezapomenout“: hooky nezapojené (jen se zapnutým Claude), město nenastavené (karta počasí je
 *  v šuplíku), kalendář nepřipojený (karta kalendáře je v šuplíku), žádný projekt (karta projektů je
 *  v šuplíku), autostart vypnutý, nová verze. Pořadí = důležitost. */
export function reminders(x: ReminderInput): Reminder[] {
  const out: Reminder[] = [];
  const cfg = x.cfg ?? {};
  const cards = placedCards(cfg.layout);
  const claudeOn = cfg.claude?.enabled !== false;
  if (claudeOn && x.hooks && (x.hooks.state === "none" || x.hooks.state === "partial")) out.push({ id: "hooks", part: "claude", go: "f-claude.hooks", fix: "hooks" });
  if (x.update?.state === "dostupna") out.push({ id: "update", part: "aplikace", go: "aplikace", fix: "update" });
  const w = cfg.personal?.weather;
  if (cards.has("weather") && (w?.lat == null || w?.lon == null)) out.push({ id: "city", part: "obsah", go: "f-personal.weather" });
  if (cards.has("calendar") && !(cfg.personal?.calendars ?? []).length && !cfg.accounts?.microsoft && !cfg.accounts?.google) {
    out.push({ id: "calendar", part: "obsah", go: "f-personal.calendars" });
  }
  if (cards.has("projects") && !(cfg.projects ?? []).length && !(cfg.roots ?? []).length) out.push({ id: "project", part: "projekty", go: "f-projects.add" });
  if (cfg.autostart === false && !x.packaged) out.push({ id: "autostart", part: "aplikace", go: "f-autostart", fix: "autostart" });
  return out;
}

/** Zóny kresby domova v pořadí stanic — každá část má na kresbě právě jedno místo (hlídá test). */
export const ZONES: readonly { part: PartId; zone: "strip" | "edge" | "frame" | "tabs" | "claude" | "tiles" | "desk" | "tray" }[] = [
  { part: "pruzek", zone: "strip" },
  { part: "otevirani", zone: "edge" },
  { part: "vzhled", zone: "frame" },
  { part: "obsah", zone: "tabs" },
  { part: "claude", zone: "claude" },
  { part: "projekty", zone: "tiles" },
  { part: "poznamky", zone: "desk" },
  { part: "aplikace", zone: "tray" },
];
