// Popisky všech voleb Nastavení na JEDNOM místě — z nich čte `Row` (název, nápověda) i hledání
// (`catalog.searchItems`). Id volby = cesta v configu, kde to jde (chyba validace najde svůj řádek).
// Čeština je tady, angličtina ve slovníku `en.ts` (klíče `settings.f.<id>`, `.hint`, `.kw`).
import { t } from "../i18n";

export type FieldDef = { panel: string; label: string; hint?: string; kw?: string; anchor?: string };

/** Popisky voleb. `anchor` = řádek, na který skočit, když volba nemá vlastní (pole v editoru projektu). */
export const FIELDS: Record<string, FieldDef> = {
  // Projekty
  projects: { panel: "projekty", label: "Projekty v šuplíku", hint: "Dlaždice na hlavní stránce. Pořadí změníš přetažením za úchyt.", kw: "dlaždice pořadí smazat vypnout" },
  "projects.add": { panel: "projekty", label: "Přidat projekt", hint: "Ze složky na disku nebo z naklonovaných rep — druh, příkaz a port se zjistí samy.", kw: "nový složka repo klon detekce" },
  "project.run": { panel: "projekty", label: "Příkaz dev serveru", kw: "run npm pnpm spuštění build", anchor: "projects" },
  "project.port": { panel: "projekty", label: "Port", kw: "port localhost prohlížeč", anchor: "projects" },
  "project.exe": { panel: "projekty", label: "Program (.exe)", kw: "exe aplikace program appName argumenty", anchor: "projects" },
  "project.accent": { panel: "projekty", label: "Barva a logo projektu", kw: "barva akcent logo ikona monogram", anchor: "projects" },
  "project.links": { panel: "projekty", label: "Odkazy projektu", kw: "odkazy staging produkce docs", anchor: "projects" },
  roots: { panel: "projekty", label: "Kde hledat repa", hint: "Složky, ve kterých Ledge najde repa projektů podle git remote (jen přímé podsložky). Do první se klonuje.", kw: "roots kořen klonovat" },
  // Šuplík a proužek
  "drawer.trigger": { panel: "suplik", label: "Jak se šuplík otevírá", hint: "Myší u horního okraje obrazovky", kw: "trigger najetí hrana podržet myš" },
  "drawer.delays": { panel: "suplik", label: "Prodlevy", hint: "Jak rychle šuplík reaguje na myš a kdy se zavře", kw: "zpoždění ms rychlost zavření" },
  "drawer.sections": { panel: "suplik", label: "Karty na hlavní stránce", hint: "Co šuplík ukazuje", kw: "sekce karty skrýt zobrazit běží claude poznámky projekty repa akce přihrádka hodiny počasí odkazy limity útrata git subagenti" },
  "strip.enabled": { panel: "suplik", label: "Proužek nahoře", hint: "Vypnuto = zůstane jen svítící linka při událostech", kw: "lišta strip" },
  "strip.items": { panel: "suplik", label: "Co proužek ukazuje", hint: "Zapni, co chceš vidět; pořadí změníš přetažením (nebo Alt+šipkou). U položky barva a „jen při dění“ = v klidu skrytá.", kw: "položky hodiny režim běží claude repa barva klid událost notch vlastní text časovač" },
  "strip.custom": { panel: "suplik", label: "Vlastní text v proužku", hint: "Krátký text nebo emoji, nejvýš 24 znaků — položka „Vlastní text“", kw: "notch text emoji popisek motto" },
  "timer.strip": { panel: "suplik", label: "Časovač v proužku, když běží", hint: "Běžící odpočet nebo stopky se ukážou na začátku proužku i bez položky „Časovač“", kw: "časovač odpočet stopky timer proužek" },
  "strip.monitor": { panel: "suplik", label: "Monitor", hint: "Na kterém monitoru proužek a šuplík jsou. Místo u hrany (vlevo, uprostřed, vpravo) je ve Vzhledu a animacích.", kw: "obrazovka displej" },
  "drawer.barWindows": { panel: "suplik", label: "Okna s taby nahoře", hint: "U maximalizovaného prohlížeče by šuplík otvíralo přepínání tabů", kw: "prohlížeč taby chrome opera edge tlak hrana push procesy" },
  "drawer.keepWarmS": { panel: "suplik", label: "Uvolnit paměť při nečinnosti", hint: "Po minutě se zavřeným šuplíkem Ledge uvolní paměť (~150 MB → desítky MB); první otevření pak trvá pár sekund. S animacemi nesouvisí.", kw: "paměť ram úspora úsporný režim výkon webview" },
  autostart: { panel: "suplik", label: "Spouštět po startu Windows", kw: "start přihlášení" },
  "palette.compact": { panel: "suplik", label: "Kompaktní paleta příkazů", hint: "Ctrl+K a 🔍 ukážou jen vyhledávací pole; výsledky se rozbalí až při psaní, nedávné šipkou dolů", kw: "paleta příkazů ctrl+k hledání powertoys malá jen pole" },
  "shelf.enabled": { panel: "suplik", label: "Přihrádka na soubory", hint: "Přetáhni soubory z Průzkumníka na proužek nahoře — uloží se jen odkazy (cesty) do karty Přihrádka. Kopírování cesty pro Claude Code, otevření, tah ven.", kw: "shelf přihrádka soubory přetáhnout drag drop cesta schránka claude" },
  "palette.winSettings": { panel: "suplik", label: "Nastavení Windows v paletě", hint: "Při psaní v paletě najde i stránky Nastavení Windows (Bluetooth, Zvuk, Wi-Fi…) a klasické ovládací panely", kw: "windows settings ms-settings ovládací panely control bluetooth zvuk wifi" },
  // Rychlé akce — skripty (`actions/ScriptsSettings.tsx`)
  "scripts.enabled": { panel: "akce", label: "Skripty v paletě", hint: "Skripty ze složky skriptů jako položky palety (Ctrl+K). Dokud složka neexistuje, nic se neukáže.", kw: "script commands raycast skript powershell python node cmd bat" },
  "scripts.dir": { panel: "akce", label: "Složka skriptů", hint: "Prázdné = Ledge\\scripts ve tvém profilu. %PROMĚNNÉ% se rozbalí.", kw: "složka cesta skripty" },
  "scripts.inActions": { panel: "akce", label: "Skripty i v kartě Rychlé akce", hint: "Dlaždice skriptů ve skupině Skripty (jinak jen v paletě)", kw: "dlaždice karta skripty" },
  fullscreen: { panel: "suplik", label: "Hry a video na celou obrazovku", hint: "Ledge je při nich vždy neaktivní: nic neukazuje a nebere myš ani fokus", kw: "fullscreen hra celá obrazovka" },
  gameMode: { panel: "suplik", label: "Herní režim", hint: "Ledge zmizí jako při hře na celou obrazovku, i po restartu, dokud ho nevypneš: znovu spusť Ledge (zástupce nebo Start), nebo stiskni zkratku šuplíku či herního režimu", kw: "hra game skrýt schovat nerušit mrtvý" },
  // Vzhled a animace (panel `Appearance.tsx`; kotvy `f-…` jsou na jeho kartách a řádcích)
  "appearance.theme": { panel: "vzhled", label: "Motiv", kw: "barvy tmavý světlý kontrast nord téma" },
  "appearance.accent": { panel: "vzhled", label: "Akcent", hint: "Linka, záře a fokus", kw: "barva akcentu" },
  "appearance.layout": { panel: "vzhled", label: "Písmo a rozložení", hint: "Hustota, velikost písma, rohy", kw: "hustota úsporná vzdušná písmo font rohy zaoblení" },
  "drawer.width": { panel: "vzhled", label: "Šířka šuplíku", kw: "šířka px velikost" },
  "drawer.glass": { panel: "vzhled", label: "Pozadí šuplíku", hint: "Krytí a rozmazání plochy za šuplíkem (akryl)", kw: "sklo akryl průhlednost rozmazání krytí glass" },
  "appearance.motion": { panel: "vzhled", label: "Animace", hint: "Styl a rychlost pohybu; „Bez animací“ = šuplík se ukáže hned", kw: "animace pohyb vysunutí pružina plynulý svižný bez animací rychlost" },
  "appearance.strip": { panel: "vzhled", label: "Vzhled proužku", hint: "Styl, velikost, pozice u hrany, linka a krytí", kw: "proužek pilulka tečky linka pozice vlevo vpravo uprostřed krytí" },
  "appearance.cards": { panel: "vzhled", label: "Pořadí karet na hlavní stránce", kw: "karty pořadí práce" },
  "ui.lang": { panel: "vzhled", label: "Jazyk", hint: "Jazyk rozhraní Ledge — přepne se hned po uložení", kw: "jazyk language čeština angličtina english czech překlad" },
  // Claude Code
  "claude.enabled": { panel: "claude", label: "Schvalování z Ledge", hint: "Žádosti Clauda o povolení se objeví v šuplíku", kw: "povolit most" },
  "claude.port": { panel: "claude", label: "Port mostu", hint: "Jen 127.0.0.1. Změna platí po restartu Ledge a znovu zapojených hookách", kw: "port" },
  "claude.approvalTimeoutS": { panel: "claude", label: "Jak dlouho čekat na rozhodnutí", hint: "Pak se Claude zeptá v terminálu — nikdy neschválí sám", kw: "timeout sekundy" },
  "claude.enforceCommit": { panel: "claude", label: "Nedokončené session", hint: "Soubory, které session upravila a nezacommitovala, a její nepushnuté commity", kw: "commit push hlídač" },
  "claude.statusLine": { panel: "claude", label: "Stavový řádek pro limity", hint: "Ledge obalí stavový řádek Claude Code a čte z něj limity 5h / týden; tvůj původní poběží dál", kw: "limit kvóta 5h týden statusline" },
  "claude.limitsReminder": { panel: "claude", label: "Připomenout reset limitu", hint: "Když limit 5h nebo týden dosáhne 100 %, v čase resetu krátce vyjede šuplík (ve hře nic)", kw: "limit kvóta reset připomínka" },
  "claude.toast": { panel: "claude", label: "Oznámení Windows", hint: "Když Claude čeká na schválení nebo otázku, nebo dokončí práci. Nebere fokus; ve hře, v herním režimu a při Nerušit nikdy", kw: "toast oznámení notifikace upozornění hotovo" },
  "claude.costs": { panel: "claude", label: "Útrata Claude (odhad)", hint: "Sekce v kartě Claude: tokeny a odhad ceny v cenách API (ne účet) podle projektu a dne z lokálních transcriptů; počítá se jen po rozbalení", kw: "útrata cena tokeny dolary náklady odhad" },
  "claude.hooks": { panel: "claude", label: "Hooky v Claude Code", hint: "Zapojení Ledge do ~/.claude/settings.json", kw: "settings.json zapojit odpojit" },
  // Poznámky
  "notes.enabled": { panel: "poznamky", label: "Rychlé poznámky a připomínky", hint: "Karta v šuplíku, připomínky v proužku", kw: "připomínky" },
  "notes.root": { panel: "poznamky", label: "Složka poznámek", hint: "Prázdné = Ledge\\notes ve tvém profilu (mimo repa projektů). %PROMĚNNÉ% se rozbalí.", kw: "kořen cesta" },
  "notes.kinds": { panel: "poznamky", label: "Kategorie", hint: "Záložky karty a výběr v editoru, v tomhle pořadí", kw: "kind nápad feedback úkol barva ikona" },
  "notes.sync.enabled": { panel: "poznamky", label: "Synchronizace mezi počítači", hint: "Složka poznámek je git repo; Ledge sám commitne, stáhne a pushne", kw: "sync git" },
  "notes.sync.remote": { panel: "poznamky", label: "Repo pro poznámky", hint: "Soukromé repo: https://… nebo git@…. Přihlášení: gh auth setup-git", kw: "remote github" },
  "notes.sync.intervalMin": { panel: "poznamky", label: "Jak často synchronizovat", hint: "Navíc 10 s po změně a při otevření šuplíku", kw: "interval minuty" },
  // Osobní
  "modes.personalEnabled": { panel: "osobni", label: "Záložka Osobní", hint: "Čas, počasí, rychlé odkazy a obecné poznámky", kw: "režim práce" },
  "personal.weather": { panel: "osobni", label: "Místo pro počasí", hint: "Open-Meteo, bez klíče. Prázdné = nic se nestahuje (polohu podle IP Ledge nezjišťuje).", kw: "počasí souřadnice šířka délka město" },
  "timer.presets": { panel: "osobni", label: "Předvolby časovače", hint: "Minuty oddělené čárkou (1–600, nejvýš 6) — tlačítka v kartě Časovač a příkazy v paletě", kw: "časovač odpočet minuty pomodoro focus timer" },
  "timer.toast": { panel: "osobni", label: "Oznámení po doběhnutí", hint: "Oznámení Windows, když odpočet skončí. Nebere fokus; ve hře, při „nerušit“ a s otevřeným šuplíkem se neukáže. Proužek blikne vždy.", kw: "časovač toast oznámení notifikace konec" },
  "personal.links": { panel: "osobni", label: "Rychlé odkazy", hint: "Web (jen http/https), složka, soubor nebo aplikace — vždy bez argumentů", kw: "odkaz web aplikace složka" },
  // Zkratky
  "hotkeys.enabled": { panel: "zkratky", label: "Globální klávesové zkratky", hint: "Vypnuté = nic se neregistruje; šuplík se otevírá myší", kw: "klávesnice" },
  "hotkeys.toggle": { panel: "zkratky", label: "Otevřít / zavřít šuplík", kw: "zkratka" },
  "hotkeys.note": { panel: "zkratky", label: "Rychlá poznámka", kw: "zkratka poznámka" },
  "hotkeys.approve": { panel: "zkratky", label: "Povolit žádost Clauda", hint: "Registruje se jen, když nějaká žádost čeká", kw: "zkratka schválit" },
  "hotkeys.deny": { panel: "zkratky", label: "Zamítnout žádost Clauda", hint: "Registruje se jen, když nějaká žádost čeká", kw: "zkratka zamítnout" },
  "hotkeys.gameMode": { panel: "zkratky", label: "Herní režim zapnout / vypnout", hint: "Platí i v herním režimu — tou se z něj dostaneš", kw: "zkratka hra game" },
  // Záloha
  "backup.export": { panel: "zaloha", label: "Exportovat nastavení", hint: "Uloží celé nastavení do souboru — třeba pro druhý počítač", kw: "záloha uložit soubor" },
  "backup.import": { panel: "zaloha", label: "Importovat nastavení", hint: "Ukáže, co se změní, a zeptá se, které části převzít", kw: "načíst soubor obnovit" },
  "backup.reset": { panel: "zaloha", label: "Obnovit výchozí", hint: "Po částech, s potvrzením; projekty zůstanou", kw: "reset výchozí tovární" },
  "backup.file": { panel: "zaloha", label: "config.json v editoru", hint: "Pro pokročilé — Ledge změny v souboru hned načte", kw: "json soubor ručně" },
  // Aktualizace a o aplikaci
  "about.version": { panel: "oaplikaci", label: "Verze", kw: "verze ledge" },
  "update.check": { panel: "oaplikaci", label: "Hlídat nové verze", hint: "Jen upozorní v hlavičce, instaluje se až na klik. Dotazy jdou přes přihlášené gh", kw: "aktualizace update" },
  "update.repo": { panel: "oaplikaci", label: "Repo s vydáními", hint: "owner/name na GitHubu", kw: "github releases" },
  "update.intervalH": { panel: "oaplikaci", label: "Jak často kontrolovat", hint: "1–48 h; první kontrola minutu po startu", kw: "interval hodiny" },
  "about.files": { panel: "oaplikaci", label: "Soubory Ledge", hint: "Diagnostický log a složka s nastavením", kw: "log ledge.log diagnostika config cesta" },
};

export const fieldLabel = (id: string) => t(`settings.f.${id}`, FIELDS[id]?.label ?? id);
export const fieldHint = (id: string) => (FIELDS[id]?.hint ? t(`settings.f.${id}.hint`, FIELDS[id].hint!) : "");
/** Klíčová slova pro hledání v aktuálním jazyce (anglická ve slovníku `settings.f.<id>.kw`). */
export const fieldKw = (id: string) => (FIELDS[id]?.kw ? t(`settings.f.${id}.kw`, FIELDS[id].kw!) : "");
/** Česká podoba volby (název, nápověda, klíčová slova) — hledání ji bere i v jiném jazyce. */
export const fieldCs = (id: string) => {
  const f = FIELDS[id];
  return f ? `${f.label} ${f.hint ?? ""} ${f.kw ?? ""}` : "";
};
/** Řádek, na který skočit (pole bez vlastního řádku → jeho seznam). */
export const fieldAnchor = (id: string) => FIELDS[id]?.anchor ?? id;
