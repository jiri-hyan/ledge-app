// Hledání v Nastavení Windows přes paletu (Ctrl+K): statická tabulka stránek `ms-settings:` a
// klasických ovládacích panelů (`control.exe /name Microsoft.…`) → položky „Nastavení Windows: …“.
// Jen URI a kanonická jména z dokumentace Microsoftu (ověřeno 17. 10. 2026):
//   https://learn.microsoft.com/windows/apps/develop/launch/launch-settings-app
//   https://learn.microsoft.com/windows/win32/shell/controlpanel-canonical-names
// Vynechané: stránky označené jako zastaralé / odebrané, závislé na hardwaru (baterie, pero, Dial)
// a stránky s parametrem. Klíčová slova česky i anglicky (`tIn`) bez ohledu na jazyk UI.
// Spuštění: jádro ověří tvar cíle (`winsettings.rs`) a otevře ho bez shellu.
import { t, tIn } from "../i18n.ts";
import type { Item } from "./search.ts";

export type WinPage = {
  id: string;
  /** `ms-settings:<stránka>` nebo `control:Microsoft.<Jméno>` */
  target: string;
  name: readonly [string, string];
  kw: readonly [string, string];
};

export const WIN_PAGES: readonly WinPage[] = [
  // Systém
  { id: "display", target: "ms-settings:display", name: ["palette.win.display", "Displej"], kw: ["palette.win.display.kw", "obrazovka monitor rozlišení jas měřítko"] },
  { id: "nightlight", target: "ms-settings:nightlight", name: ["palette.win.nightlight", "Noční světlo"], kw: ["palette.win.nightlight.kw", "modré světlo večer teplota barev"] },
  { id: "sound", target: "ms-settings:sound", name: ["palette.win.sound", "Zvuk"], kw: ["palette.win.sound.kw", "reproduktory sluchátka mikrofon hlasitost výstup vstup"] },
  { id: "volume", target: "ms-settings:apps-volume", name: ["palette.win.volume", "Směšovač hlasitosti"], kw: ["palette.win.volume.kw", "hlasitost aplikací mixer"] },
  { id: "notifications", target: "ms-settings:notifications", name: ["palette.win.notifications", "Oznámení"], kw: ["palette.win.notifications.kw", "upozornění notifikace"] },
  { id: "focus", target: "ms-settings:quiethours", name: ["palette.win.focus", "Nerušit a soustředění"], kw: ["palette.win.focus.kw", "nerušit soustředění tichý režim"] },
  { id: "power", target: "ms-settings:powersleep", name: ["palette.win.power", "Napájení a režim spánku"], kw: ["palette.win.power.kw", "spánek úspora energie vypnutí obrazovky"] },
  { id: "storage", target: "ms-settings:storagesense", name: ["palette.win.storage", "Úložiště"], kw: ["palette.win.storage.kw", "disk místo smazat soubory"] },
  { id: "multitasking", target: "ms-settings:multitasking", name: ["palette.win.multitasking", "Multitasking"], kw: ["palette.win.multitasking.kw", "přichycení oken rozložení plochy"] },
  { id: "clipboard", target: "ms-settings:clipboard", name: ["palette.win.clipboard", "Schránka"], kw: ["palette.win.clipboard.kw", "kopírovat vložit historie schránky"] },
  { id: "remotedesktop", target: "ms-settings:remotedesktop", name: ["palette.win.remotedesktop", "Vzdálená plocha"], kw: ["palette.win.remotedesktop.kw", "vzdálený přístup rdp"] },
  { id: "about", target: "ms-settings:about", name: ["palette.win.about", "O systému"], kw: ["palette.win.about.kw", "verze windows název počítače specifikace"] },
  // Zařízení
  { id: "bluetooth", target: "ms-settings:bluetooth", name: ["palette.win.bluetooth", "Bluetooth a zařízení"], kw: ["palette.win.bluetooth.kw", "zařízení sluchátka párování"] },
  { id: "printers", target: "ms-settings:printers", name: ["palette.win.printers", "Tiskárny a skenery"], kw: ["palette.win.printers.kw", "tisk tiskárna skener"] },
  { id: "mouse", target: "ms-settings:mousetouchpad", name: ["palette.win.mouse", "Myš"], kw: ["palette.win.mouse.kw", "kurzor rychlost kolečko touchpad"] },
  { id: "typing", target: "ms-settings:typing", name: ["palette.win.typing", "Psaní"], kw: ["palette.win.typing.kw", "automatické opravy návrhy textu"] },
  { id: "autoplay", target: "ms-settings:autoplay", name: ["palette.win.autoplay", "Automatické přehrávání"], kw: ["palette.win.autoplay.kw", "usb flash disk karta"] },
  { id: "usb", target: "ms-settings:usb", name: ["palette.win.usb", "USB"], kw: ["palette.win.usb.kw", "zařízení usb"] },
  // Síť a internet
  { id: "network", target: "ms-settings:network-status", name: ["palette.win.network", "Síť a internet"], kw: ["palette.win.network.kw", "připojení internet stav sítě"] },
  { id: "wifi", target: "ms-settings:network-wifi", name: ["palette.win.wifi", "Wi-Fi"], kw: ["palette.win.wifi.kw", "bezdrátová síť wlan"] },
  { id: "ethernet", target: "ms-settings:network-ethernet", name: ["palette.win.ethernet", "Ethernet"], kw: ["palette.win.ethernet.kw", "kabel lan síť"] },
  { id: "vpn", target: "ms-settings:network-vpn", name: ["palette.win.vpn", "VPN"], kw: ["palette.win.vpn.kw", "virtuální privátní síť"] },
  { id: "proxy", target: "ms-settings:network-proxy", name: ["palette.win.proxy", "Proxy"], kw: ["palette.win.proxy.kw", "proxy server síť"] },
  { id: "airplane", target: "ms-settings:network-airplanemode", name: ["palette.win.airplane", "Režim v letadle"], kw: ["palette.win.airplane.kw", "letadlo vypnout bezdrátové"] },
  { id: "hotspot", target: "ms-settings:network-mobilehotspot", name: ["palette.win.hotspot", "Mobilní hotspot"], kw: ["palette.win.hotspot.kw", "sdílet internet hotspot"] },
  // Přizpůsobení
  { id: "background", target: "ms-settings:personalization-background", name: ["palette.win.background", "Pozadí plochy"], kw: ["palette.win.background.kw", "tapeta obrázek plocha"] },
  { id: "colors", target: "ms-settings:colors", name: ["palette.win.colors", "Barvy"], kw: ["palette.win.colors.kw", "tmavý režim světlý akcent"] },
  { id: "themes", target: "ms-settings:themes", name: ["palette.win.themes", "Motivy"], kw: ["palette.win.themes.kw", "motiv vzhled"] },
  { id: "lockscreen", target: "ms-settings:lockscreen", name: ["palette.win.lockscreen", "Zamykací obrazovka"], kw: ["palette.win.lockscreen.kw", "zámek obrazovka"] },
  { id: "start", target: "ms-settings:personalization-start", name: ["palette.win.start", "Start"], kw: ["palette.win.start.kw", "nabídka start"] },
  { id: "taskbar", target: "ms-settings:taskbar", name: ["palette.win.taskbar", "Hlavní panel"], kw: ["palette.win.taskbar.kw", "lišta ikony panel úloh"] },
  { id: "fonts", target: "ms-settings:fonts", name: ["palette.win.fonts", "Písma"], kw: ["palette.win.fonts.kw", "font písmo"] },
  // Aplikace
  { id: "apps", target: "ms-settings:appsfeatures", name: ["palette.win.apps", "Nainstalované aplikace"], kw: ["palette.win.apps.kw", "aplikace programy odinstalovat"] },
  { id: "defaultapps", target: "ms-settings:defaultapps", name: ["palette.win.defaultapps", "Výchozí aplikace"], kw: ["palette.win.defaultapps.kw", "výchozí prohlížeč otevírat pomocí"] },
  { id: "startupapps", target: "ms-settings:startupapps", name: ["palette.win.startupapps", "Aplikace po spuštění"], kw: ["palette.win.startupapps.kw", "po startu automaticky spouštět"] },
  { id: "optionalfeatures", target: "ms-settings:optionalfeatures", name: ["palette.win.optionalfeatures", "Volitelné funkce"], kw: ["palette.win.optionalfeatures.kw", "funkce doplňky"] },
  // Účty
  { id: "yourinfo", target: "ms-settings:yourinfo", name: ["palette.win.yourinfo", "Vaše informace"], kw: ["palette.win.yourinfo.kw", "účet profil"] },
  { id: "signin", target: "ms-settings:signinoptions", name: ["palette.win.signin", "Možnosti přihlášení"], kw: ["palette.win.signin.kw", "heslo pin hello přihlášení"] },
  { id: "accounts", target: "ms-settings:emailandaccounts", name: ["palette.win.accounts", "E-mail a účty"], kw: ["palette.win.accounts.kw", "email pošta účty"] },
  { id: "otherusers", target: "ms-settings:otherusers", name: ["palette.win.otherusers", "Další uživatelé"], kw: ["palette.win.otherusers.kw", "uživatelé rodina účty"] },
  // Hry
  { id: "gamebar", target: "ms-settings:gaming-gamebar", name: ["palette.win.gamebar", "Herní panel"], kw: ["palette.win.gamebar.kw", "xbox game bar nahrávání"] },
  { id: "gamemode", target: "ms-settings:gaming-gamemode", name: ["palette.win.gamemode", "Herní režim"], kw: ["palette.win.gamemode.kw", "hry výkon"] },
  // Usnadnění
  { id: "magnifier", target: "ms-settings:easeofaccess-magnifier", name: ["palette.win.magnifier", "Lupa"], kw: ["palette.win.magnifier.kw", "zvětšení usnadnění"] },
  { id: "contrast", target: "ms-settings:easeofaccess-highcontrast", name: ["palette.win.contrast", "Kontrastní motivy"], kw: ["palette.win.contrast.kw", "vysoký kontrast usnadnění"] },
  // Soukromí
  { id: "privacy", target: "ms-settings:privacy", name: ["palette.win.privacy", "Ochrana osobních údajů"], kw: ["palette.win.privacy.kw", "soukromí oprávnění"] },
  { id: "location", target: "ms-settings:privacy-location", name: ["palette.win.location", "Poloha"], kw: ["palette.win.location.kw", "gps určování polohy soukromí"] },
  { id: "camera", target: "ms-settings:privacy-webcam", name: ["palette.win.camera", "Kamera"], kw: ["palette.win.camera.kw", "webkamera oprávnění soukromí"] },
  { id: "microphone", target: "ms-settings:privacy-microphone", name: ["palette.win.microphone", "Mikrofon"], kw: ["palette.win.microphone.kw", "oprávnění soukromí"] },
  // Čas a jazyk
  { id: "datetime", target: "ms-settings:dateandtime", name: ["palette.win.datetime", "Datum a čas"], kw: ["palette.win.datetime.kw", "hodiny časové pásmo"] },
  { id: "region", target: "ms-settings:regionformatting", name: ["palette.win.region", "Oblast"], kw: ["palette.win.region.kw", "formát data čísla měna region"] },
  { id: "language", target: "ms-settings:regionlanguage", name: ["palette.win.language", "Jazyk"], kw: ["palette.win.language.kw", "jazyk zobrazení čeština angličtina"] },
  { id: "keyboard", target: "ms-settings:keyboard", name: ["palette.win.keyboard", "Klávesnice (vstup)"], kw: ["palette.win.keyboard.kw", "rozložení klávesnice vstup"] },
  { id: "speech", target: "ms-settings:speech", name: ["palette.win.speech", "Řeč"], kw: ["palette.win.speech.kw", "hlas rozpoznávání řeči"] },
  // Aktualizace a zabezpečení
  { id: "update", target: "ms-settings:windowsupdate", name: ["palette.win.update", "Windows Update"], kw: ["palette.win.update.kw", "aktualizace update"] },
  { id: "updatehistory", target: "ms-settings:windowsupdate-history", name: ["palette.win.updatehistory", "Historie aktualizací"], kw: ["palette.win.updatehistory.kw", "aktualizace update historie"] },
  { id: "security", target: "ms-settings:windowsdefender", name: ["palette.win.security", "Zabezpečení Windows"], kw: ["palette.win.security.kw", "antivirus defender ochrana"] },
  { id: "recovery", target: "ms-settings:recovery", name: ["palette.win.recovery", "Obnovení"], kw: ["palette.win.recovery.kw", "obnovit resetovat"] },
  { id: "troubleshoot", target: "ms-settings:troubleshoot", name: ["palette.win.troubleshoot", "Řešení potíží"], kw: ["palette.win.troubleshoot.kw", "poradce opravit problém"] },
  { id: "developers", target: "ms-settings:developers", name: ["palette.win.developers", "Pro vývojáře"], kw: ["palette.win.developers.kw", "vývojářský režim"] },
  { id: "activation", target: "ms-settings:activation", name: ["palette.win.activation", "Aktivace"], kw: ["palette.win.activation.kw", "licence produktový klíč"] },
  // Klasické ovládací panely (kanonická jména)
  { id: "cpl.mouse", target: "control:Microsoft.Mouse", name: ["palette.win.cpl.mouse", "Myš"], kw: ["palette.win.cpl.mouse.kw", "ukazatel kurzor dvojklik"] },
  { id: "cpl.sound", target: "control:Microsoft.Sound", name: ["palette.win.cpl.sound", "Zvuk"], kw: ["palette.win.cpl.sound.kw", "přehrávací zařízení nahrávání"] },
  { id: "cpl.power", target: "control:Microsoft.PowerOptions", name: ["palette.win.cpl.power", "Možnosti napájení"], kw: ["palette.win.cpl.power.kw", "plán napájení výkon"] },
  { id: "cpl.programs", target: "control:Microsoft.ProgramsAndFeatures", name: ["palette.win.cpl.programs", "Programy a funkce"], kw: ["palette.win.cpl.programs.kw", "odinstalovat program"] },
  { id: "cpl.devices", target: "control:Microsoft.DeviceManager", name: ["palette.win.cpl.devices", "Správce zařízení"], kw: ["palette.win.cpl.devices.kw", "ovladače hardware"] },
  { id: "cpl.network", target: "control:Microsoft.NetworkAndSharingCenter", name: ["palette.win.cpl.network", "Centrum síťových připojení a sdílení"], kw: ["palette.win.cpl.network.kw", "síť adaptér sdílení"] },
  { id: "cpl.credentials", target: "control:Microsoft.CredentialManager", name: ["palette.win.cpl.credentials", "Správce pověření"], kw: ["palette.win.cpl.credentials.kw", "hesla přihlašovací údaje"] },
  { id: "cpl.firewall", target: "control:Microsoft.WindowsFirewall", name: ["palette.win.cpl.firewall", "Brána firewall"], kw: ["palette.win.cpl.firewall.kw", "firewall síť ochrana"] },
  { id: "cpl.folders", target: "control:Microsoft.FolderOptions", name: ["palette.win.cpl.folders", "Možnosti Průzkumníka souborů"], kw: ["palette.win.cpl.folders.kw", "skryté soubory přípony složky"] },
  { id: "cpl.keyboard", target: "control:Microsoft.Keyboard", name: ["palette.win.cpl.keyboard", "Klávesnice"], kw: ["palette.win.cpl.keyboard.kw", "rychlost opakování kurzor"] },
  { id: "cpl.datetime", target: "control:Microsoft.DateAndTime", name: ["palette.win.cpl.datetime", "Datum a čas"], kw: ["palette.win.cpl.datetime.kw", "hodiny časové pásmo"] },
  { id: "cpl.region", target: "control:Microsoft.RegionAndLanguage", name: ["palette.win.cpl.region", "Oblast"], kw: ["palette.win.cpl.region.kw", "formát data čísla"] },
  { id: "cpl.internet", target: "control:Microsoft.InternetOptions", name: ["palette.win.cpl.internet", "Možnosti Internetu"], kw: ["palette.win.cpl.internet.kw", "internet proxy certifikáty"] },
  { id: "cpl.indexing", target: "control:Microsoft.IndexingOptions", name: ["palette.win.cpl.indexing", "Možnosti indexování"], kw: ["palette.win.cpl.indexing.kw", "hledání index vyhledávání"] },
  { id: "cpl.users", target: "control:Microsoft.UserAccounts", name: ["palette.win.cpl.users", "Uživatelské účty"], kw: ["palette.win.cpl.users.kw", "účet uživatel"] },
  { id: "cpl.color", target: "control:Microsoft.ColorManagement", name: ["palette.win.cpl.color", "Správa barev"], kw: ["palette.win.cpl.color.kw", "barevný profil icc monitor"] },
  { id: "cpl.tools", target: "control:Microsoft.AdministrativeTools", name: ["palette.win.cpl.tools", "Nástroje systému Windows"], kw: ["palette.win.cpl.tools.kw", "správa služby události"] },
];

export interface WinItem extends Item {
  target: string;
}

/** Položky palety pro dotaz. Bez dotazu nic (60 stránek by zahltilo seznam), stejně tak vypnuté. */
export function winItems(enabled: boolean, query: string, pages: readonly WinPage[] = WIN_PAGES): WinItem[] {
  if (!enabled || !query.trim()) return [];
  return pages.map((p) => {
    const name = t(p.name[0], p.name[1]);
    const classic = p.target.startsWith("control:");
    return {
      id: `win:${p.id}`,
      order: 8,
      target: p.target,
      title: classic ? t("palette.win.control", "Klasický ovládací panel: {name}", { name }) : t("palette.win.title", "Nastavení Windows: {name}", { name }),
      sub: p.target.replace(/^control:/, "control /name "),
      // obě jména i klíčová slova v obou jazycích — hledá se česky i anglicky v jakémkoli UI
      keywords: [tIn("cs", ...p.name), tIn("en", ...p.name), tIn("cs", ...p.kw), tIn("en", ...p.kw), classic ? "control panel ovladaci panely" : "windows settings nastaveni"].join(" "),
    };
  });
}
