// Viditelnost karet a sekcí šuplíku — `drawer.sections` v configu (zrcadlo `config::Sections` v Rustu,
// plochý seznam boolů, výchozí zapnuto). Seznam přepínačů, skupiny pro Nastavení a čistá logika: co je na
// stránce vidět, třída mřížky, změna JEDNOHO pole configu. Bez importů za běhu (test `sections.test.mjs`).
// Nový přepínač = klíč do `SECTION_KEYS` (+ řádek `sections!` v config.rs) a položka do `SECTION_GROUPS`.

export const SECTION_KEYS = [
  "projects",
  "running",
  "claude",
  "notes",
  "repos",
  "actions",
  "shelf",
  "clock",
  "timer",
  "weather",
  "calendar",
  "links",
  "claudeLimits",
  "claudeCosts",
  "claudeGit",
  "claudeSubagents",
  "nowPlaying",
  "sound",
  "battery",
] as const;
export type SectionKey = (typeof SECTION_KEYS)[number];
export type SectionMap = Record<SectionKey, boolean>;

/** Zapnuto, pokud pole není výslovně `false` (starý snapshot / config bez pole = zapnuto). */
export const sectionOn = (sec: Partial<Record<string, unknown>> | null | undefined, k: SectionKey): boolean => sec?.[k] !== false;

/** Všechno zapnuté (výchozí stav, prázdný snapshot). */
export const allSectionsOn = (): SectionMap => Object.fromEntries(SECTION_KEYS.map((k) => [k, true])) as SectionMap;

/** Klíče, které jsou teď skryté (pro „Zobrazit vše“). */
export const hiddenSections = (sec: Partial<Record<string, unknown>> | null | undefined): SectionKey[] => SECTION_KEYS.filter((k) => !sectionOn(sec, k));

/** Config se změnou JEDNOHO pole `drawer.sections.<k>` — nic jiného se nemění (vstup zůstane netknutý). */
export function withSection<T extends Record<string, any>>(cfg: T, k: SectionKey, on: boolean): T {
  const drawer = cfg.drawer ?? {};
  return { ...cfg, drawer: { ...drawer, sections: { ...(drawer.sections ?? {}), [k]: on } } };
}

/** Sekce uvnitř karty Claude (menu „Skrýt sekci“, ne celou kartu). */
export const CLAUDE_PARTS: readonly SectionKey[] = ["claudeLimits", "claudeCosts", "claudeGit", "claudeSubagents"];
/** Části karet Osobní: hlasitost v kartě Právě hraje, baterie v kartě Čas (menu „Skrýt sekci“). */
export const PERSONAL_PARTS: readonly SectionKey[] = ["sound", "battery"];
export const isPart = (k: SectionKey): boolean => CLAUDE_PARTS.includes(k) || PERSONAL_PARTS.includes(k);

export type SectionItem = { k: SectionKey; key: string; cs: string; hint: [string, string] };
export type SectionGroup = { id: "work" | "personal" | "both" | "claude"; key: string; cs: string; items: SectionItem[] };

/** Skupiny pro Nastavení (`settings/panels/Drawer.tsx`) a název pro potvrzení „… skryta · Vrátit“. Texty
 *  jako páry klíč + čeština, přeloží se až při vykreslení. */
export const SECTION_GROUPS: SectionGroup[] = [
  {
    id: "work",
    key: "settings.drawer.secGroup.work",
    cs: "Práce",
    items: [
      { k: "projects", key: "settings.drawer.sec.projects", cs: "Projekty", hint: ["settings.drawer.sec.projects.hint", "Dlaždice projektů nahoře"] },
      { k: "running", key: "settings.drawer.sec.running", cs: "Běžící servery", hint: ["settings.drawer.sec.running.hint", "Ukáže se, jen když Ledge něco spustil"] },
      { k: "claude", key: "settings.drawer.sec.claude", cs: "Claude", hint: ["settings.drawer.sec.claude.hint", "Session, subagenti a žádosti o schválení"] },
      { k: "repos", key: "settings.drawer.sec.repos", cs: "Repa", hint: ["settings.drawer.sec.repos.hint", "Neuložená a nepushnutá práce, repa pozadu"] },
    ],
  },
  {
    id: "personal",
    key: "settings.drawer.secGroup.personal",
    cs: "Osobní",
    items: [
      { k: "clock", key: "settings.drawer.sec.clock", cs: "Čas", hint: ["settings.drawer.sec.clock.hint", "Velké hodiny s týdnem"] },
      { k: "timer", key: "settings.drawer.sec.timer", cs: "Časovač", hint: ["settings.drawer.sec.timer.hint", "Odpočet a stopky; skrytá karta časovač nevypne (paleta, proužek)"] },
      { k: "weather", key: "settings.drawer.sec.weather", cs: "Počasí", hint: ["settings.drawer.sec.weather.hint", "Skryté počasí se nestahuje"] },
      { k: "calendar", key: "settings.drawer.sec.calendar", cs: "Kalendář", hint: ["settings.drawer.sec.calendar.hint", "Dnes a zítra z odkazu ICS (Google, Outlook); skrytý se nestahuje"] },
      { k: "links", key: "settings.drawer.sec.links", cs: "Odkazy", hint: ["settings.drawer.sec.links.hint", "Rychlé odkazy na weby, složky a aplikace"] },
      { k: "nowPlaying", key: "settings.drawer.sec.nowPlaying", cs: "Právě hraje", hint: ["settings.drawer.sec.nowPlaying.hint", "Skladba nebo video z libovolné aplikace, přehrát/pauza, další, předchozí"] },
      { k: "sound", key: "settings.drawer.sec.sound", cs: "Hlasitost", hint: ["settings.drawer.sec.sound.hint", "Posuvník a ztlumení výchozího výstupu zvuku (v kartě Právě hraje)"] },
      { k: "battery", key: "settings.drawer.sec.battery", cs: "Baterie", hint: ["settings.drawer.sec.battery.hint", "Stav baterie v kartě Čas — jen na notebooku"] },
    ],
  },
  {
    id: "both",
    key: "settings.drawer.secGroup.both",
    cs: "Práce i Osobní",
    items: [
      { k: "notes", key: "settings.drawer.sec.notes", cs: "Poznámky", hint: ["settings.drawer.sec.notes.hint", "Poslední poznámky a připomínky na dnes"] },
      { k: "actions", key: "settings.drawer.sec.actions", cs: "Rychlé akce", hint: ["settings.drawer.sec.actions.hint", "Dlaždice akcí na jedno kliknutí"] },
      { k: "shelf", key: "settings.drawer.sec.shelf", cs: "Přihrádka", hint: ["settings.drawer.sec.shelf.hint", "Soubory přetažené na proužek"] },
    ],
  },
  {
    id: "claude",
    key: "settings.drawer.secGroup.claude",
    cs: "V kartě Claude",
    items: [
      { k: "claudeLimits", key: "settings.drawer.sec.claudeLimits", cs: "Limity 5h / týden", hint: ["settings.drawer.sec.claudeLimits.hint", "Řádek limitů nahoře v kartě"] },
      { k: "claudeCosts", key: "settings.drawer.sec.claudeCosts", cs: "Útrata (odhad)", hint: ["settings.drawer.sec.claudeCosts.hint", "Skrytá útrata nečte transcripty"] },
      { k: "claudeGit", key: "settings.drawer.sec.claudeGit", cs: "Git panel", hint: ["settings.drawer.sec.claudeGit.hint", "Commit, push a diff u session"] },
      { k: "claudeSubagents", key: "settings.drawer.sec.claudeSubagents", cs: "Subagenti", hint: ["settings.drawer.sec.claudeSubagents.hint", "Řádek běžících a hotových subagentů"] },
    ],
  },
];

/** Položka tabulky pro klíč (název do potvrzení po skrytí). */
export const sectionItem = (k: SectionKey): SectionItem | undefined => SECTION_GROUPS.flatMap((g) => g.items).find((x) => x.k === k);

/** Co karty na stránce potřebují vědět kromě přepínačů (karty, které se bez dat samy schovají). */
export type PageFacts = {
  mode: "prace" | "osobni";
  sec: Partial<Record<string, unknown>> | null | undefined;
  /** procesy Ledge (karta Běží bez nich nezabírá místo) */
  procs: number;
  /** poznámky zapnuté celé (`notes.enabled`) */
  notesEnabled: boolean;
  /** rychlé akce pro tento režim (na Práci se prázdná karta neukáže, na Osobní zve k přidání) */
  actions: number;
  /** přihrádka má položky (`shelf/model.ts` `cardVisible`) */
  shelf: boolean;
  /** karta Právě hraje má co ukázat: skladbu (`personal.media`) nebo hlasitost (`personal.volume`) */
  media?: boolean;
};

/** Klíč karty Právě hraje: se skrytou skladbou zůstane jen „Zvuk“ (hlasitost). */
export const mediaCardKey = (sec: PageFacts["sec"]): SectionKey => (sectionOn(sec, "nowPlaying") ? "nowPlaying" : "sound");

/** Karty, které stránka právě ukáže, v pořadí stránky (mřížka Práce bez řazení — to dělá `cardOrder`). */
export function visibleCards(f: PageFacts): SectionKey[] {
  const on = (k: SectionKey) => sectionOn(f.sec, k);
  const cards: [SectionKey, boolean][] =
    f.mode === "osobni"
      ? [
          ["clock", on("clock")],
          ["timer", on("timer")],
          ["weather", on("weather")],
          ["calendar", on("calendar")],
          ["links", on("links")],
          [mediaCardKey(f.sec), (on("nowPlaying") || on("sound")) && !!f.media],
          ["actions", on("actions")],
          ["shelf", on("shelf") && f.shelf],
          ["notes", on("notes") && f.notesEnabled],
        ]
      : [
          ["projects", on("projects")],
          ["actions", on("actions") && f.actions > 0],
          ["shelf", on("shelf") && f.shelf],
          ["running", on("running") && f.procs > 0],
          ["claude", on("claude")],
          ["repos", on("repos")],
          ["notes", on("notes") && f.notesEnabled],
        ];
  return cards.filter(([, v]) => v).map(([k]) => k);
}

/** Úzké karty mřížky Práce (Poznámky jsou přes celou šířku) → třída mřížky: 1 = `one`, 2 = `two`, jinak 3 sloupce. */
export function gridClass(visible: readonly SectionKey[]): "" | "one" | "two" {
  const n = visible.filter((k) => k === "running" || k === "claude" || k === "repos").length;
  return n === 1 ? "one" : n === 2 ? "two" : "";
}
