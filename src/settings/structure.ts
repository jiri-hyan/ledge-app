// Kostra Nastavení (kolo 21): málo hlavních sekcí podle toho, CO chce uživatel udělat, a v nich stránky
// (= panely z `registry.ts`). Nahoře „Doporučené“ — nejčastější volby jedním klikem. Čistá data a logika
// bez DOM a bez `t()` (název je pár klíč + čeština, přeloží ho skořápka `Settings.tsx`).
// Každá stránka je právě v jedné sekci (hlídá `structure.test.mjs`); pořadí sekcí = pořadí v navigaci.

export type Section = {
  id: string;
  key: string;
  cs: string;
  /** jedna věta pod názvem v navigaci a nad stránkami sekce */
  desc: readonly [string, string];
  /** stránky (id panelů z `registry.ts`) v pořadí; první = kam vede klik na sekci */
  pages: readonly string[];
};

export const SECTIONS: readonly Section[] = [
  { id: "doporucene", key: "settings.sec.doporucene", cs: "Doporučené", desc: ["settings.sec.doporucene.desc", "Nejčastější volby na jednom místě"], pages: ["doporucene"] },
  { id: "vzhled", key: "settings.sec.vzhled", cs: "Vzhled", desc: ["settings.sec.vzhled.desc", "Jak Ledge vypadá zavřený, po najetí a otevřený"], pages: ["vzhled"] },
  { id: "suplik", key: "settings.sec.suplik", cs: "Šuplík a karty", desc: ["settings.sec.suplik.desc", "Co šuplík ukazuje, jak se otevírá a co je v kartách"], pages: ["suplik", "chovani", "osobni"] },
  { id: "claude", key: "settings.sec.claude", cs: "Claude", desc: ["settings.sec.claude.desc", "Schvalování, otázky, limity a propojení s Claude Code"], pages: ["claude"] },
  { id: "akce", key: "settings.sec.akce", cs: "Projekty a akce", desc: ["settings.sec.akce.desc", "Projekty, odkazy a příkazy na jedno kliknutí"], pages: ["akce"] },
  { id: "poznamky", key: "settings.sec.poznamky", cs: "Poznámky a lístky", desc: ["settings.sec.poznamky.desc", "Rychlé poznámky, připomínky a lístky na ploše"], pages: ["poznamky"] },
  { id: "naplose", key: "settings.sec.naplose", cs: "Na ploše", desc: ["settings.sec.naplose.desc", "Widgety připnuté na plochu Windows"], pages: ["naplose"] },
  { id: "aplikace", key: "settings.sec.aplikace", cs: "Aplikace", desc: ["settings.sec.aplikace.desc", "Jazyk, průvodce, spouštění, hry, záloha a nové verze"], pages: ["zaciname", "spousteni", "aplikace"] },
];

/** Výchozí stránka po otevření Nastavení. */
export const FIRST_PAGE = "doporucene";

/** Sekce, ve které stránka leží (neznámá = null). */
export const sectionOf = (page: string): Section | null => SECTIONS.find((s) => s.pages.includes(page)) ?? null;

/** Všechny stránky v pořadí navigace. */
export const PAGE_ORDER: readonly string[] = SECTIONS.flatMap((s) => s.pages);

/** Kam vede klik na sekci: naposledy otevřená stránka té sekce, jinak první. */
export function pageForSection(sec: Section, last: Record<string, string>): string {
  const p = last[sec.id];
  return p && sec.pages.includes(p) ? p : sec.pages[0];
}

/** Navštívené stránky (drží se naživu — přepnutí je jen skryje): nová na konec, žádné duplicity,
 *  nanejvýš `max` (nejdéle nenavštívená vypadne, kromě právě otevřené). */
export function withVisited(list: readonly string[], id: string, max = 16): string[] {
  if (list.includes(id)) return list as string[];
  const out = [...list, id];
  return out.length > max ? out.slice(out.length - max) : out;
}

/** Doporučené: nejčastější volby (cíl `go` = kotva `f-<id>` nebo id stránky; hlídá `settings.test.mjs`
 *  i `structure.test.mjs`). Název a věta, co volba dělá, jsou páry klíč + čeština. */
export type Pick = { go: string; icon: string; title: readonly [string, string]; text: readonly [string, string] };

export const RECOMMENDED: readonly Pick[] = [
  { go: "f-strip.look", icon: "▭", title: ["settings.rec.look", "Vzhled zavřeného Ledge"], text: ["settings.rec.look.text", "Co je vidět u horní hrany, když je šuplík zavřený."] },
  { go: "f-appearance.style", icon: "◐", title: ["settings.rec.style", "Design styl"], text: ["settings.rec.style.text", "Hotový balíček barev, písma a tvaru jedním klikem."] },
  { go: "f-layout", icon: "▦", title: ["settings.rec.layout", "Záložky a karty"], text: ["settings.rec.layout.text", "Co šuplík ukazuje, na které záložce a jak velké."] },
  { go: "f-projects.add", icon: "＋", title: ["settings.rec.project", "Přidat projekt"], text: ["settings.rec.project.text", "Složka nebo repo — příkaz a port se zjistí samy."] },
  { go: "f-actions.add", icon: "↗", title: ["settings.rec.action", "Přidat odkaz nebo akci"], text: ["settings.rec.action.text", "Web, složka, soubor, aplikace nebo příkaz na jedno kliknutí."] },
  { go: "f-claude.hooks", icon: "✳", title: ["settings.rec.hooks", "Propojit s Claude Code"], text: ["settings.rec.hooks.text", "Ledge uvidí session a ukáže žádosti o povolení."] },
  { go: "f-personal.weather", icon: "☀", title: ["settings.rec.weather", "Počasí"], text: ["settings.rec.weather.text", "Vyber město; bez něj se nic nestahuje."] },
  { go: "f-personal.calendars", icon: "▤", title: ["settings.rec.calendar", "Kalendáře"], text: ["settings.rec.calendar.text", "Google nebo Outlook jen pro čtení, s připomínkou schůzky."] },
  { go: "f-drawer.trigger", icon: "⇡", title: ["settings.rec.trigger", "Jak se šuplík otevírá"], text: ["settings.rec.trigger.text", "Myší u horní hrany, s prodlevou, nebo zatlačením."] },
  { go: "f-autostart", icon: "⏻", title: ["settings.rec.autostart", "Spouštět po startu Windows"], text: ["settings.rec.autostart.text", "Ledge se spustí sám po přihlášení."] },
  { go: "f-ui.lang", icon: "A", title: ["settings.rec.lang", "Jazyk"], text: ["settings.rec.lang.text", "Čeština nebo angličtina — přepne se po uložení."] },
  { go: "f-backup.export", icon: "⇩", title: ["settings.rec.backup", "Záloha nastavení"], text: ["settings.rec.backup.text", "Celé nastavení do souboru, třeba pro druhý počítač."] },
];
