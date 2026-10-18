// Viditelnost karet a jejich částí — `drawer.sections` ve snapshotu (zrcadlo `config::Sections` v Rustu,
// plochý seznam boolů). Od vlny 6 je pole ODVOZENÉ z `config.layout` (`layout.rs` `derive`): karta = je
// na nějaké záložce, část (limity, útrata, hlasitost, baterie…) = karta je a část není skrytá. Co je kde
// a jak velké, řeší `layout/model.ts`; tady zůstává jen čtení pro karty. Bez importů za běhu
// (test `sections.test.mjs`). Nová karta / část = klíč sem + řádek `sections!` v config.rs + `layout.rs`.

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

/** Sekce uvnitř karty Claude (menu „Skrýt sekci“, ne celou kartu). */
export const CLAUDE_PARTS = ["claudeLimits", "claudeCosts", "claudeGit", "claudeSubagents"] as const;
