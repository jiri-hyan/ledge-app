// Popisky všech voleb Nastavení na JEDNOM místě — z nich čte `Row` (název, nápověda) i hledání
// (`catalog.searchItems`). Id volby = cesta v configu, kde to jde (chyba validace najde svůj řádek).
// Čeština je tady, angličtina ve slovníku `en.ts` (klíče `settings.f.<id>`, `.hint`, `.kw`).
import { t } from "../i18n";

export type FieldDef = { panel: string; label: string; hint?: string; kw?: string; anchor?: string };

/** Popisky voleb. `anchor` = řádek, na který skočit, když volba nemá vlastní (pole v editoru projektu). */
export const FIELDS: Record<string, FieldDef> = {
  // Začínáme
  "onboarding.wizard": { panel: "zaciname", label: "Průvodce prvním spuštěním", kw: "průvodce onboarding znovu spustit nastavit od začátku vítejte" },
  // Šuplík a karty
  "layout": { panel: "suplik", label: "Záložky a karty", hint: "Co šuplík ukazuje, na které záložce a jak velké", kw: "karty záložky rozložení pořadí velikost skrýt zobrazit přidat katalog běží claude poznámky projekty repa akce přihrádka hodiny počasí odkazy limity útrata git subagenti osobní práce" },
  "shelf.enabled": { panel: "suplik", label: "Přihrádka na soubory", hint: "Přetáhni soubory z Průzkumníka na proužek nahoře — uloží se jen odkazy (cesty) do karty Přihrádka. Kopírování cesty pro Claude Code, otevření, tah ven.", kw: "shelf přihrádka soubory přetáhnout drag drop cesta schránka claude" },
  // Vzhled (panel `Appearance.tsx`; kotvy `f-…` jsou na jeho kartách a řádcích) — i celý proužek (`StripSettings.tsx`)
  "appearance.style": { panel: "vzhled", label: "Design styl", hint: "Hotový balíček vzhledu: barvy, tvar, písmo, materiál, rozložení a šířka", kw: "styl design sklo terminál papír neon brutal pastel klasický vzhled" },
  "appearance.theme": { panel: "vzhled", label: "Barvy", hint: "Barevný motiv", kw: "motiv barvy tmavý světlý kontrast nord téma" },
  "appearance.accent": { panel: "vzhled", label: "Akcent", hint: "Linka, záře a fokus", kw: "barva akcentu" },
  "appearance.layout": { panel: "vzhled", label: "Šířka a rozložení", hint: "Úzký, střední, široký nebo přes celou obrazovku; sloupce karet", kw: "šířka rozložení sloupce úzký široký celá šířka panel list panoráma mozaika dlaždice" },
  "appearance.details": { panel: "vzhled", label: "Upravit detaily", hint: "Písmo, materiál, hustota, velikost písma, rohy", kw: "písmo font materiál hustota úsporná vzdušná rohy zaoblení stín záře" },
  "drawer.width": { panel: "vzhled", label: "Šířka šuplíku", kw: "šířka px velikost" },
  "drawer.glass": { panel: "vzhled", label: "Pozadí šuplíku", hint: "Krytí a rozmazání plochy za šuplíkem (akryl)", kw: "sklo akryl průhlednost rozmazání krytí glass" },
  "appearance.motion": { panel: "vzhled", label: "Animace", hint: "Jak se šuplík otevírá; „Žádná“ = ukáže se hned", kw: "animace pohyb vysunutí pružina plynulá svižná odskok rozplynutí výřez žádná rychlost" },
  "appearance.strip": { panel: "vzhled", label: "Vzhled proužku", hint: "Styl, velikost, pozice u hrany, linka a krytí", kw: "proužek pilulka tečky linka pozice vlevo vpravo uprostřed krytí" },
  "strip.enabled": { panel: "vzhled", label: "Proužek nahoře", hint: "Vypnuto = zůstane jen svítící linka při událostech", kw: "lišta strip" },
  "strip.items": { panel: "vzhled", label: "Co proužek ukazuje", hint: "Zapni, co chceš vidět; pořadí změníš přetažením (nebo Alt+šipkou). U položky barva a „Jen při dění“ = v klidu skrytá.", kw: "položky hodiny režim běží claude repa barva klid událost notch vlastní text časovač" },
  "strip.custom": { panel: "vzhled", label: "Vlastní text v proužku", hint: "Krátký text nebo emoji, nejvýš 24 znaků — položka „Vlastní text“", kw: "notch text emoji popisek motto" },
  "timer.strip": { panel: "vzhled", label: "Časovač v proužku, když běží", hint: "Běžící odpočet nebo stopky se ukážou na začátku proužku i bez položky „Časovač“", kw: "časovač odpočet stopky timer proužek" },
  "strip.monitor": { panel: "vzhled", label: "Na kterém monitoru", hint: "Proužek i šuplík budou na tomhle monitoru. Místo u hrany (vlevo, uprostřed, vpravo) je v kartě Vzhled proužku.", kw: "monitor obrazovka displej" },
  "ui.lang": { panel: "vzhled", label: "Jazyk", hint: "Jazyk rozhraní Ledge — přepne se hned po uložení", kw: "jazyk language čeština angličtina english czech překlad" },
  // Projekty, akce a odkazy (odkaz = akce typu web / složka / soubor)
  projects: { panel: "akce", label: "Projekty v šuplíku", hint: "Dlaždice na hlavní stránce. Pořadí změníš přetažením za úchyt.", kw: "dlaždice pořadí smazat vypnout" },
  "projects.add": { panel: "akce", label: "Přidat projekt", hint: "Ze složky na disku nebo z naklonovaných rep — druh, příkaz a port se zjistí samy.", kw: "nový složka repo klon detekce" },
  "project.run": { panel: "akce", label: "Příkaz dev serveru", kw: "run npm pnpm spuštění build", anchor: "projects" },
  "project.port": { panel: "akce", label: "Port", kw: "port localhost prohlížeč", anchor: "projects" },
  "project.exe": { panel: "akce", label: "Program (.exe)", kw: "exe aplikace program appName argumenty", anchor: "projects" },
  "project.accent": { panel: "akce", label: "Barva a logo projektu", kw: "barva akcent logo ikona monogram", anchor: "projects" },
  "project.links": { panel: "akce", label: "Odkazy projektu", kw: "odkazy staging produkce docs", anchor: "projects" },
  roots: { panel: "akce", label: "Kde hledat repa", hint: "Složky, ve kterých Ledge najde repa projektů podle git remote (jen přímé podsložky). Do první se klonuje.", kw: "roots kořen klonovat" },
  actions: { panel: "akce", label: "Akce a odkazy", hint: "Weby, složky, soubory, aplikace a příkazy na jedno kliknutí. Odkazy (web, složka, soubor) ukazuje karta Odkazy.", kw: "rychlé akce odkazy rychlé odkazy web složka soubor aplikace příkaz okno s webem favicon pořadí" },
  "actions.add": { panel: "akce", label: "Přidat akci nebo odkaz", kw: "nový odkaz nová akce přidat odkaz web" },
  "scripts.enabled": { panel: "akce", label: "Skripty v paletě", hint: "Skripty ze složky skriptů jako položky palety (Ctrl+K). Dokud složka neexistuje, nic se neukáže.", kw: "script commands raycast skript powershell python node cmd bat" },
  "scripts.dir": { panel: "akce", label: "Složka skriptů", hint: "Prázdné = Ledge\\scripts ve tvém profilu. %PROMĚNNÉ% se rozbalí.", kw: "složka cesta skripty" },
  "scripts.inActions": { panel: "akce", label: "Skripty i v kartě Rychlé akce", hint: "Dlaždice skriptů ve skupině Skripty (jinak jen v paletě)", kw: "dlaždice karta skripty" },
  // Claude (i otázky Clauda, `Questions.tsx`)
  "claude.enabled": { panel: "claude", label: "Schvalování z Ledge", hint: "Žádosti Clauda o povolení se objeví v šuplíku", kw: "povolit most" },
  "claude.port": { panel: "claude", label: "Port pro spojení s Claude Code", hint: "Číslo, přes které se Claude Code hlásí Ledge (jen tento počítač, 127.0.0.1). Změna platí po restartu Ledge a znovu zapojených hookách.", kw: "port most" },
  "claude.approvalTimeoutS": { panel: "claude", label: "Jak dlouho čekat na rozhodnutí", hint: "Pak se Claude zeptá v terminálu — nikdy neschválí sám", kw: "timeout sekundy" },
  "claude.toast": { panel: "claude", label: "Oznámení Windows", hint: "Když Claude čeká na schválení nebo otázku, nebo dokončí práci. Nebere fokus; ve hře, v herním režimu a při Nerušit nikdy", kw: "toast oznámení notifikace upozornění hotovo" },
  "claude.hooks": { panel: "claude", label: "Hooky v Claude Code", hint: "Zapojení Ledge do ~/.claude/settings.json", kw: "settings.json zapojit odpojit" },
  "claude.questionHoldS": { panel: "claude", label: "Odpovídat na otázky z Ledge", hint: "Když se Claude Code zeptá, odpovíš kliknutím v šuplíku", kw: "otázky clauda otázka odpověď askuserquestion" },
  "claude.statusLine": { panel: "claude", label: "Číst limity 5h / týden", hint: "Ledge obalí stavový řádek Claude Code a čte z něj limity 5h / týden; tvůj původní poběží dál", kw: "stavový řádek limit kvóta 5h týden statusline" },
  "claude.limitsReminder": { panel: "claude", label: "Připomenout reset limitu", hint: "Když limit 5h nebo týden dosáhne 100 %, v čase resetu krátce vyjede šuplík (ve hře nic)", kw: "limit kvóta reset připomínka" },
  "claude.costs": { panel: "claude", label: "Útrata Claude (odhad)", hint: "Sekce v kartě Claude: tokeny a odhad ceny v cenách API (ne účet) podle projektu a dne z lokálních transcriptů; počítá se jen po rozbalení", kw: "útrata cena tokeny dolary náklady odhad" },
  "claude.enforceCommit": { panel: "claude", label: "Hlídat nedokončenou práci", hint: "Soubory, které session upravila a nezacommitovala, a její nepushnuté commity", kw: "nedokončené session commit push hlídač" },
  // Osobní
  "personal.weather": { panel: "osobni", label: "Místo pro počasí", hint: "Open-Meteo, bez klíče. Prázdné = nic se nestahuje (polohu podle IP Ledge nezjišťuje).", kw: "počasí souřadnice šířka délka město" },
  "personal.calendars": { panel: "osobni", label: "Kalendáře", hint: "Google nebo Outlook z tajného odkazu ICS — jen čtení, bez přihlášení. Vlož odkaz a klikni na Přidat.", kw: "kalendář google outlook ics ical webcal události schůzky" },
  "media.variant": { panel: "osobni", label: "Vzhled přehrávače", hint: "Karta Právě hraje: kompaktní řádek, karta s průběhem, velký obal, vinyl, nebo jen v proužku. Přepneš i ikonkou v rohu karty.", kw: "přehrávač hudba spotify youtube vinyl obal vzhled karta právě hraje" },
  "media.stripTime": { panel: "osobni", label: "Čas skladby v proužku", hint: "Po minutách (třeba „1/4 min“) a jen když přehrávač čas hlásí. Sekundy proužek neukazuje, aby zbytečně nepracoval.", kw: "proužek čas skladby minuty průběh" },
  "media.equalizer": { panel: "osobni", label: "Ozdoba ekvalizéru", hint: "Jen ozdoba — neukazuje skutečný zvuk. V proužku stojí, v šuplíku se hýbe jen při přehrávání.", kw: "ekvalizér animace ozdoba proužek" },
  "timer.presets": { panel: "osobni", label: "Předvolby časovače", hint: "Minuty oddělené čárkou (1–600, nejvýš 6) — tlačítka v kartě Časovač a příkazy v paletě", kw: "časovač odpočet minuty pomodoro focus timer" },
  "timer.toast": { panel: "osobni", label: "Oznámení po doběhnutí", hint: "Oznámení Windows, když odpočet skončí. Nebere fokus; ve hře, při „nerušit“ a s otevřeným šuplíkem se neukáže. Proužek blikne vždy.", kw: "časovač toast oznámení notifikace konec" },
  "clipboard.enabled": { panel: "osobni", label: "Historie schránky v paletě", hint: "Ctrl+K a v: ukáže, co jsi naposled zkopíroval. Výchozí vypnuto.", kw: "schránka clipboard historie kopírovat vložit win+v paste" },
  "clipboard.max": { panel: "osobni", label: "Kolik položek si pamatovat", hint: "Starší se samy zapomenou. Text delší než 64 KB se neuloží vůbec.", kw: "počet položek historie schránka" },
  "clipboard.skipSecrets": { panel: "osobni", label: "Neukládat přístupové klíče a tokeny", hint: "Text, který vypadá jako API klíč, token GitHubu nebo AWS, přihlašovací token (JWT) nebo soukromý klíč, se neuloží", kw: "tajemství heslo token klíč api github aws jwt schránka" },
  "clipboard.clearOnLock": { panel: "osobni", label: "Zapomenout při zamčení počítače", hint: "Win+L smaže všechno kromě připnutých", kw: "zamknout zámek win+l smazat schránka soukromí" },
  "clipboard.savePins": { panel: "osobni", label: "Připnuté si pamatovat i po restartu", hint: "Připnuté položky (Ctrl+P v paletě) se uloží do souboru ve tvém profilu (%LOCALAPPDATA%\\Ledge\\clip-pins.json). Vypnutím se soubor smaže.", kw: "připnout uložit disk restart schránka" },
  "clipboard.excludeApps": { panel: "osobni", label: "Z těchto aplikací nic neukládat", hint: "Co v nich zkopíruješ, Ledge nikdy neuloží — hodí se pro správce hesel a bankovnictví", kw: "vyloučit aplikace správce hesel keepass bitwarden 1password schránka" },
  "clipboard.clear": { panel: "osobni", label: "Vymazat historii teď", hint: "Smaže z paměti všechno kromě připnutých položek", kw: "smazat vyčistit schránka historie" },
  // Poznámky
  "notes.enabled": { panel: "poznamky", label: "Rychlé poznámky a připomínky", hint: "Karta v šuplíku, připomínky v proužku", kw: "připomínky" },
  "notes.root": { panel: "poznamky", label: "Složka poznámek", hint: "Prázdné = Ledge\\notes ve tvém profilu (mimo repa projektů). %PROMĚNNÉ% se rozbalí.", kw: "kořen cesta" },
  "notes.kinds": { panel: "poznamky", label: "Kategorie", hint: "Záložky karty a výběr v editoru, v tomhle pořadí", kw: "kind nápad feedback úkol barva ikona" },
  "notes.sync.enabled": { panel: "poznamky", label: "Synchronizace mezi počítači", hint: "Složka poznámek je git repo; Ledge sám commitne, stáhne a pushne", kw: "sync git" },
  "notes.sync.remote": { panel: "poznamky", label: "Repo pro poznámky", hint: "Soukromé repo: https://… nebo git@…. Přihlášení: gh auth setup-git", kw: "remote github" },
  "notes.sync.intervalMin": { panel: "poznamky", label: "Jak často synchronizovat", hint: "Navíc 10 s po změně a při otevření šuplíku", kw: "interval minuty" },
  // Chování (i klávesové zkratky, `Hotkeys.tsx`)
  "drawer.trigger": { panel: "chovani", label: "Jak se šuplík otevírá", hint: "Myší u horního okraje obrazovky", kw: "trigger najetí hrana podržet myš" },
  "drawer.delays": { panel: "chovani", label: "Prodlevy", hint: "Jak rychle šuplík reaguje na myš a kdy se zavře", kw: "zpoždění ms rychlost zavření" },
  "hotkeys.enabled": { panel: "chovani", label: "Globální klávesové zkratky", hint: "Vypnuté = nic se neregistruje; šuplík se otevírá myší", kw: "klávesnice" },
  "hotkeys.toggle": { panel: "chovani", label: "Otevřít / zavřít šuplík", kw: "zkratka" },
  "hotkeys.note": { panel: "chovani", label: "Rychlá poznámka", kw: "zkratka poznámka" },
  "hotkeys.approve": { panel: "chovani", label: "Povolit žádost Clauda", hint: "Registruje se jen, když nějaká žádost čeká", kw: "zkratka schválit" },
  "hotkeys.deny": { panel: "chovani", label: "Zamítnout žádost Clauda", hint: "Registruje se jen, když nějaká žádost čeká", kw: "zkratka zamítnout" },
  "hotkeys.gameMode": { panel: "chovani", label: "Herní režim zapnout / vypnout", hint: "Platí i v herním režimu — tou se z něj dostaneš", kw: "zkratka hra game" },
  "palette.compact": { panel: "chovani", label: "Kompaktní paleta příkazů", hint: "Ctrl+K a 🔍 ukážou jen vyhledávací pole; výsledky se rozbalí až při psaní, nedávné šipkou dolů", kw: "paleta příkazů ctrl+k hledání powertoys malá jen pole" },
  "palette.winSettings": { panel: "chovani", label: "Nastavení Windows v paletě", hint: "Při psaní v paletě najde i stránky Nastavení Windows (Bluetooth, Zvuk, Wi-Fi…) a klasické ovládací panely", kw: "windows settings ms-settings ovládací panely control bluetooth zvuk wifi" },
  fullscreen: { panel: "chovani", label: "Hry a video na celou obrazovku", hint: "Ledge je při nich vždy neaktivní: nic neukazuje a nebere myš ani fokus", kw: "fullscreen hra celá obrazovka" },
  gameMode: { panel: "chovani", label: "Herní režim", hint: "Ledge zmizí jako při hře na celou obrazovku, i po restartu, dokud ho nevypneš: znovu spusť Ledge (zástupce nebo Start), nebo stiskni zkratku šuplíku či herního režimu", kw: "hra game skrýt schovat nerušit mrtvý" },
  "drawer.barWindows": { panel: "chovani", label: "Šuplík nad prohlížečem", hint: "Maximalizovaný prohlížeč má nahoře taby — aby šuplík nevyjížděl při každém přepnutí tabu, otevře se nad ním jen zatlačením myši nahoru.", kw: "okna s taby nahoře prohlížeč taby chrome opera edge tlak hrana push procesy zatlačit" },
  autostart: { panel: "chovani", label: "Spouštět po startu Windows", hint: "Ledge se spustí sám po přihlášení do Windows.", kw: "start přihlášení autostart spouštění po startu" },
  "drawer.keepWarmS": { panel: "chovani", label: "Šetřit paměť", hint: "Když je šuplík minutu zavřený, Ledge uvolní většinu paměti (asi ze 150 MB na desítky). Další otevření pak trvá o chvilku déle. S animacemi nesouvisí.", kw: "uvolnit paměť při nečinnosti ram úspora úsporný režim výkon webview" },
  // Záloha a aplikace
  "backup.export": { panel: "aplikace", label: "Exportovat nastavení", hint: "Uloží celé nastavení do souboru — třeba pro druhý počítač", kw: "záloha uložit soubor" },
  "backup.import": { panel: "aplikace", label: "Importovat nastavení", hint: "Ukáže, co se změní, a zeptá se, které části převzít", kw: "načíst soubor obnovit" },
  "backup.reset": { panel: "aplikace", label: "Obnovit výchozí", hint: "Po částech, s potvrzením; projekty zůstanou", kw: "reset výchozí tovární" },
  "backup.file": { panel: "aplikace", label: "config.json v editoru", hint: "Pro pokročilé — Ledge změny v souboru hned načte", kw: "json soubor ručně" },
  "about.version": { panel: "aplikace", label: "Verze", kw: "verze ledge" },
  "update.check": { panel: "aplikace", label: "Hlídat nové verze", hint: "Jen upozorní v hlavičce, instaluje se až na klik. Dotazy jdou přes přihlášené gh", kw: "aktualizace update" },
  "update.repo": { panel: "aplikace", label: "Repo s vydáními", hint: "owner/name na GitHubu", kw: "github releases" },
  "update.intervalH": { panel: "aplikace", label: "Jak často kontrolovat", hint: "1–48 h; první kontrola minutu po startu", kw: "interval hodiny" },
  "about.files": { panel: "aplikace", label: "Soubory Ledge", hint: "Diagnostický log a složka s nastavením", kw: "log ledge.log diagnostika config cesta" },
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
