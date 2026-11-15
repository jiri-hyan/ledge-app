// Stránky Nastavení bez komponent (levný import i pro paletu): id (kotva, hledání, skok odjinud), název
// a klíčová slova. Komponenty k nim přiřadí `catalog.ts`; pořadí zde = pořadí v navigaci a paletě (plochý seznam,
// žádný `structure.ts`: ten po kole 21 zanikl; skládání do částí je `studio/parts.ts` + `parts.test.mjs`).
// Cesty do hlášek („Nastavení → Obsah karet → Kalendáře“) skládá `path.ts` `settingsPath` z těchto názvů;
// `path.test.mjs` hlídá, že žádný text neodkazuje na neexistující stránku.
// Nová stránka = záznam tady + komponenta v `catalog.ts` (hlídá `settings.test.mjs`).
// Klíčová slova nesou i STARÉ názvy panelů a voleb, ať je hledání najde („okna s taby“, „uvolnit paměť“,
// „otázky“, „rychlé akce“, „Osobní“, „Chování“…). Pořadí tady = pořadí v paletě.

/** `kw` = klíčová slova pro hledání jako pár [klíč, čeština] (en ve slovníku `settings.nav.<id>.kw`). */
export type PanelMeta = { id: string; key: string; cs: string; kw: readonly [string, string] };

export const PANEL_META: PanelMeta[] = [
  { id: "doporucene", key: "settings.nav.doporucene", cs: "Doporučené", kw: ["settings.nav.doporucene.kw", "doporučené nejčastější rychlý start oblíbené hlavní tipy dobré vědět novinky co je nového kde co je"] },
  { id: "vzhled", key: "settings.nav.vzhled", cs: "Vzhled", kw: ["settings.nav.vzhled.kw", "styl motiv barva akcent animace průhlednost písmo hustota šířka proužek vzhled a animace"] },
  { id: "suplik", key: "settings.nav.suplik", cs: "Záložky a karty", kw: ["settings.nav.suplik.kw", "šuplík záložky karty rozložení pořadí velikost části karet upravit šuplík přihrádka"] },
  { id: "chovani", key: "settings.nav.chovani", cs: "Otevírání a klávesy", kw: ["settings.nav.chovani.kw", "chování otevírání šuplíku zkratky klávesnice paleta prodlevy myš okna s taby prohlížeč"] },
  { id: "osobni", key: "settings.nav.osobni", cs: "Obsah karet", kw: ["settings.nav.osobni.kw", "osobní počasí město kalendář přehrávač hudba časovač schránka historie github systém návrhy účet účty microsoft google přihlášení"] },
  { id: "claude", key: "settings.nav.claude", cs: "Claude", kw: ["settings.nav.claude.kw", "claude code hooky schvalování otázky clauda limity útrata hlídač commit"] },
  { id: "akce", key: "settings.nav.akce", cs: "Projekty, odkazy a akce", kw: ["settings.nav.akce.kw", "projekt dlaždice repo složka rychlé akce odkaz odkazy web aplikace příkaz skripty paleta"] },
  { id: "poznamky", key: "settings.nav.poznamky", cs: "Poznámky a lístky", kw: ["settings.nav.poznamky.kw", "poznámky připomínky kategorie synchronizace git lístky na ploše"] },
  { id: "naplose", key: "settings.nav.naplose", cs: "Widgety na ploše", kw: ["settings.nav.naplose.kw", "widgety na ploše plocha monitor profily rozmístění vrstva navrchu"] },
  { id: "zaciname", key: "settings.nav.zaciname", cs: "Jazyk a průvodce", kw: ["settings.nav.zaciname.kw", "začínáme jazyk průvodce první spuštění onboarding klávesy gesta nápověda vítejte upravit šuplík"] },
  { id: "spousteni", key: "settings.nav.spousteni", cs: "Spouštění a hry", kw: ["settings.nav.spousteni.kw", "spouštění po startu windows autostart paměť šetřit uvolnit herní režim hra celá obrazovka fullscreen video chování"] },
  { id: "aplikace", key: "settings.nav.aplikace", cs: "Záloha a verze", kw: ["settings.nav.aplikace.kw", "záloha obnova export import výchozí reset config.json verze aktualizace log o aplikaci"] },
];

/** Staré id panelů → nové (skoky z karet, palety a starých návodů vedou dál na správné místo). */
export const PANEL_ALIASES: Record<string, string> = {
  vitejte: "zaciname",
  projekty: "akce",
  otazky: "claude",
  zkratky: "chovani",
  zaloha: "aplikace",
  oaplikaci: "aplikace",
  ucty: "osobni",
  plocha: "naplose",
  widgety: "naplose",
};

/** Sekce uvnitř stránky: `panel` = kam patří, `key` + `cs` = název (anglicky ve slovníku pod `key`),
 *  `anchor` = klíč skutečného nadpisu (`t("anchor", …)` nebo popisek v `FIELDS`), který test najde ve zdrojích. */
export const SECTIONS = {
  calendars: { panel: "osobni", key: "settings.f.personal.calendars", cs: "Kalendáře", anchor: "personal.calendars" },
  accounts: { panel: "osobni", key: "settings.path.adv", cs: "Pokročilé", anchor: "settings.accounts.adv" },
  costs: { panel: "claude", key: "settings.claude.costs", cs: "Hodnota v cenách API", anchor: "settings.claude.costs" },
  apikey: { panel: "claude", key: "settings.claude.apikey", cs: "API klíč pro zadání", anchor: "settings.claude.apikey" },
  mcp: { panel: "claude", key: "settings.mcp.title", cs: "Claude (MCP)", anchor: "settings.mcp.title" },
  widgets: { panel: "akce", key: "settings.widgets.title", cs: "Widgety ze skriptů", anchor: "settings.widgets.title" },
  updates: { panel: "aplikace", key: "settings.about.updates", cs: "Aktualizace", anchor: "settings.about.updates" },
  restart: { panel: "spousteni", key: "settings.pkg.restart", cs: "Restartovat mimo", anchor: "settings.pkg.restart" },
  keys: { panel: "zaciname", key: "ui.keys.title", cs: "Klávesy a gesta", anchor: "ui.keys.title" },
} as const;
export type SectionId = keyof typeof SECTIONS;
