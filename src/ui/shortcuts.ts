// Klávesy a gesta Ledge na JEDNOM místě (zdroj pravdy): zobrazuje je Nastavení → Jazyk a průvodce → Klávesy a gesta
// a paleta (Ctrl+K, napiš „?“). Záznam = `keys` (klávesy, stejné v obou jazycích; u čistě myších gest prázdné) a
// věta `cs` (`t(key, cs)`, angličtina v `i18n/en-keys.ts`). Test `shortcuts.test.mjs`: každý záznam má obě
// verze a klíče jsou jedinečné. Nová klávesa nebo gesto = řádek tady, ne text v panelu.
import { t } from "../i18n.ts";

export type ShortcutGroup = "drawer" | "cards" | "notes" | "settings";

export interface Shortcut {
  group: ShortcutGroup;
  /** klávesy (`Ctrl+K`, `Alt+←/→`); prázdné = gesto myší popsané větou */
  keys: string;
  key: string;
  cs: string;
}

export const SHORTCUT_GROUPS: readonly { id: ShortcutGroup; key: string; cs: string }[] = [
  { id: "drawer", key: "ui.keys.group.drawer", cs: "Šuplík" },
  { id: "cards", key: "ui.keys.group.cards", cs: "Karty a menu" },
  { id: "notes", key: "ui.keys.group.notes", cs: "Poznámky a lístky" },
  { id: "settings", key: "ui.keys.group.settings", cs: "Nastavení" },
];

export const SHORTCUTS: readonly Shortcut[] = [
  { group: "drawer", keys: "", key: "ui.keys.hover", cs: "Najetí myší k hornímu okraji obrazovky otevře šuplík, odjetí ho zavře." },
  { group: "drawer", keys: "Ctrl+K", key: "ui.keys.palette", cs: "Paleta příkazů: projekty, akce, poznámky, volby Nastavení, výpočty a převody. Napiš „?“ a najdeš tuhle nápovědu." },
  { group: "drawer", keys: "Esc", key: "ui.keys.esc", cs: "Zavře paletu; jinak zruší úpravy šuplíku, vrátí na hlavní stránku a nakonec zavře šuplík." },
  { group: "drawer", keys: "1–9", key: "ui.keys.tabs", cs: "Přepne na záložku podle pořadí (mimo pole pro psaní)." },
  { group: "drawer", keys: "Alt+1…9", key: "ui.keys.quick", cs: "Spustí N-tého zástupce na záložce (čísla se ukážou při držení Alt). Šipky procházejí zástupce, Enter spustí vybraného; funguje jen v šuplíku, který má fokus." },
  { group: "drawer", keys: "", key: "ui.keys.tabWheel", cs: "Kolečko myši nad záložkami v hlavičce přepíná záložky." },
  { group: "drawer", keys: "", key: "ui.keys.dropNew", cs: "Odkaz, soubor, složku nebo .exe pusť na OTEVŘENÝ šuplík: Ledge navrhne nového zástupce a čeká na tvé potvrzení. Na zavřený proužek jde přihrádka na soubory." },
  { group: "cards", keys: "", key: "ui.keys.hold", cs: "Podrž hlavičku karty 0,4 s a táhni: zapne úpravy a přesune kartu. Dřív pustíš = nic se nestane." },
  { group: "cards", keys: "", key: "ui.keys.rename", cs: "Dvojklik na záložku v režimu úprav ji přejmenuje." },
  { group: "cards", keys: "", key: "ui.keys.handle", cs: "Dvojklik na spodní úchyt karty (v režimu úprav) nastaví výšku podle obsahu." },
  { group: "cards", keys: "Alt+←/→", key: "ui.keys.tabMove", cs: "Na záložce v režimu úprav ji posune o místo vedle." },
  { group: "cards", keys: "Ctrl+Z", key: "ui.keys.undoEdit", cs: "V režimu úprav vrátí poslední změnu rozložení." },
  { group: "cards", keys: "Shift+F10", key: "ui.keys.menu", cs: "Na zástupci (projekt, akce, odkaz) otevře jeho menu, stejně jako pravý klik." },
  { group: "cards", keys: "↑ ↓ Home End", key: "ui.keys.menuNav", cs: "V otevřeném menu: pohyb po položkách. Enter nebo mezerník provede, Esc zavře a vrátí na ⋯." },
  { group: "notes", keys: "Enter · Tab", key: "ui.keys.lists", cs: "V textu poznámky Enter pokračuje v seznamu, Tab a Shift+Tab odsadí položku." },
  { group: "notes", keys: "Ctrl+Enter", key: "ui.keys.noteSave", cs: "Uloží poznámku v editoru i v lístku." },
  { group: "notes", keys: "", key: "ui.keys.pinBar", cs: "Dvojklik na lištu lístku ho sbalí na lištu nebo rozbalí." },
  { group: "notes", keys: "", key: "ui.keys.pinText", cs: "Dvojklik do textu lístku ho otevře k úpravě." },
  { group: "settings", keys: "Ctrl+F", key: "ui.keys.find", cs: "Hledání v Nastavení; najde volbu i bez háčků a čárek." },
  { group: "settings", keys: "Ctrl+Z", key: "ui.keys.undoSettings", cs: "Vrátí poslední změnu v Nastavení (mimo textová pole)." },
  { group: "settings", keys: "Alt+↑/↓", key: "ui.keys.sortList", cs: "V seřaditelných seznamech posune vybranou položku nahoru nebo dolů." },
];

/** Položky skupiny v pořadí tabulky. */
export const shortcutsOf = (g: ShortcutGroup): readonly Shortcut[] => SHORTCUTS.filter((s) => s.group === g);

/** Věta záznamu v jazyce UI (překlad až při čtení, ne při importu modulu). */
export const shortcutText = (s: Shortcut): string => t(s.key, s.cs);
export const groupName = (g: (typeof SHORTCUT_GROUPS)[number]): string => t(g.key, g.cs);
