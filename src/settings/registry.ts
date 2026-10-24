// Panely Nastavení bez komponent (levný import i pro paletu): pořadí v postranní navigaci, id (kotva,
// hledání, skok odjinud), název a klíčová slova. Komponenty k nim přiřadí `catalog.ts`.
// Struktura od vlny 6 (8 + Poznámky): Začínáme · Šuplík a karty · Vzhled · Projekty, akce a odkazy ·
// Claude · Osobní (účty Microsoft / Google jsou v něm pod Kalendáři v Pokročilé) · Poznámky · Chování · Záloha a aplikace. Klíčová slova nesou i STARÉ názvy panelů
// a voleb, ať je hledání najde („okna s taby“, „uvolnit paměť“, „otázky“, „rychlé akce“…).

/** `kw` = klíčová slova pro hledání jako pár [klíč, čeština] (en ve slovníku `settings.nav.<id>.kw`). */
export type PanelMeta = { id: string; key: string; cs: string; kw: readonly [string, string] };

export const PANEL_META: PanelMeta[] = [
  { id: "zaciname", key: "settings.nav.zaciname", cs: "Začínáme", kw: ["settings.nav.zaciname.kw", "průvodce první spuštění onboarding tipy novinky co je nového nápověda vítejte"] },
  { id: "suplik", key: "settings.nav.suplik", cs: "Šuplík a karty", kw: ["settings.nav.suplik.kw", "záložky karty rozložení pořadí velikost části karet upravit šuplík přihrádka widgety na ploše plocha monitor profily rozmístění"] },
  { id: "vzhled", key: "settings.nav.vzhled", cs: "Vzhled", kw: ["settings.nav.vzhled.kw", "styl motiv barva akcent animace průhlednost písmo hustota šířka proužek jazyk vzhled a animace"] },
  { id: "akce", key: "settings.nav.akce", cs: "Projekty, odkazy a akce", kw: ["settings.nav.akce.kw", "projekt dlaždice repo složka rychlé akce odkaz odkazy web aplikace příkaz skripty paleta"] },
  { id: "claude", key: "settings.nav.claude", cs: "Claude", kw: ["settings.nav.claude.kw", "claude code hooky schvalování otázky clauda limity útrata hlídač commit"] },
  { id: "osobni", key: "settings.nav.osobni", cs: "Osobní", kw: ["settings.nav.osobni.kw", "osobní počasí město kalendář přehrávač hudba časovač schránka historie účet účty microsoft google přihlášení"] },
  { id: "poznamky", key: "settings.nav.poznamky", cs: "Poznámky", kw: ["settings.nav.poznamky.kw", "poznámky připomínky kategorie synchronizace git"] },
  { id: "chovani", key: "settings.nav.chovani", cs: "Chování", kw: ["settings.nav.chovani.kw", "otevírání šuplíku zkratky klávesnice paleta herní režim hra paměť spouštění po startu okna s taby prohlížeč"] },
  { id: "aplikace", key: "settings.nav.aplikace", cs: "Záloha a aplikace", kw: ["settings.nav.aplikace.kw", "záloha obnova export import výchozí reset config.json verze aktualizace log o aplikaci"] },
];

/** Id panelů před vlnou 6 → nové (skoky z karet, palety a starých návodů vedou dál na správné místo). */
export const PANEL_ALIASES: Record<string, string> = {
  vitejte: "zaciname",
  projekty: "akce",
  otazky: "claude",
  zkratky: "chovani",
  zaloha: "aplikace",
  oaplikaci: "aplikace",
  ucty: "osobni",
};
