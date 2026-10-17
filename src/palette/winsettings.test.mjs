// Tabulka stránek Nastavení Windows v paletě (`winsettings.ts`): jen dokumentované cíle ve tvaru, který
// pustí jádro (`winsettings.rs::parse`), a hledání česky i anglicky bez diakritiky (`search.ts`).
//   node --experimental-strip-types --test src/palette/winsettings.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { WIN_PAGES, winItems } from "./winsettings.ts";
import { rank } from "./search.ts";
import { setLang } from "../i18n.ts";
import "../i18n/en-winsettings.ts";

// Opsáno z dokumentace Microsoftu (17. 10. 2026) — přidat cíl do tabulky jde jen s řádkem tady:
// https://learn.microsoft.com/windows/apps/develop/launch/launch-settings-app
const DOC_MS_SETTINGS = new Set(
  "display nightlight sound apps-volume notifications quiethours powersleep storagesense multitasking clipboard remotedesktop about bluetooth printers mousetouchpad typing autoplay usb network-status network-wifi network-ethernet network-vpn network-proxy network-airplanemode network-mobilehotspot personalization-background colors themes lockscreen personalization-start taskbar fonts appsfeatures defaultapps startupapps optionalfeatures yourinfo signinoptions emailandaccounts otherusers gaming-gamebar gaming-gamemode easeofaccess-magnifier easeofaccess-highcontrast privacy privacy-location privacy-webcam privacy-microphone dateandtime regionformatting regionlanguage keyboard speech windowsupdate windowsupdate-history windowsdefender recovery troubleshoot developers activation".split(" "),
);
// https://learn.microsoft.com/windows/win32/shell/controlpanel-canonical-names (ne „Deprecated“)
const DOC_CONTROL = new Set(
  "Mouse Sound PowerOptions ProgramsAndFeatures DeviceManager NetworkAndSharingCenter CredentialManager WindowsFirewall FolderOptions Keyboard DateAndTime RegionAndLanguage InternetOptions IndexingOptions UserAccounts ColorManagement AdministrativeTools".split(" "),
);
// v dokumentaci označené jako zastaralé / odebrané — do tabulky nikdy
const DEPRECATED = ["backup", "defaultbrowsersettings", "tabletmode", "phone", "privacy-backgroundapps", "privacy-accessoryapps", "privacy-advertisingid", "personalization-glance", "personalization-navbar", "gaming-trueplay"];

test("tabulka: jen dokumentované cíle, tvar podle jádra, jednoznačná id", () => {
  const ids = new Set();
  const targets = new Set();
  for (const p of WIN_PAGES) {
    assert.ok(!ids.has(p.id), `id ${p.id} dvakrát`);
    assert.ok(!targets.has(p.target), `cíl ${p.target} dvakrát`);
    ids.add(p.id);
    targets.add(p.target);
    const ms = p.target.match(/^ms-settings:([a-z0-9-]{1,64})$/);
    const cpl = p.target.match(/^control:Microsoft\.([A-Za-z]{1,48})$/);
    assert.ok(ms || cpl, `${p.target}: tvar, který winsettings.rs pustí`);
    if (ms) {
      assert.ok(DOC_MS_SETTINGS.has(ms[1]), `${p.target} není v opsané dokumentaci`);
      assert.ok(!DEPRECATED.includes(ms[1]), `${p.target} je zastaralý`);
    } else assert.ok(DOC_CONTROL.has(cpl[1]), `${p.target} není v dokumentaci kanonických jmen`);
    assert.equal(p.name[0], `palette.win.${p.id}`);
    assert.equal(p.kw[0], `palette.win.${p.id}.kw`);
    assert.ok(p.name[1].trim() && p.kw[1].trim());
  }
  assert.ok(WIN_PAGES.length >= 40);
});

test("bez dotazu nebo vypnuté: nic", () => {
  assert.deepEqual(winItems(true, ""), []);
  assert.deepEqual(winItems(true, "   "), []);
  assert.deepEqual(winItems(false, "bluetooth"), []);
});

/** [jazyk UI, dotaz, id, které má vyhrát (pole = tyhle v libovolném pořadí na prvních místech)] */
const SEARCH = [
  ["cs", "bluetooth", "win:bluetooth"],
  ["cs", "wifi", "win:wifi"],
  ["cs", "zvuk", "win:sound"],
  ["cs", "reproduktory", "win:sound"],
  ["cs", "displej", "win:display"],
  ["cs", "napajeni", "win:power"],
  ["cs", "aktualizace", "win:update"],
  ["cs", "mys", "win:mouse"],
  // anglická slova i v českém UI
  ["cs", "pairing", "win:bluetooth"],
  ["cs", "wallpaper", "win:background"],
  ["cs", "device manager", "win:cpl.devices"],
  ["cs", "spravce zarizeni", "win:cpl.devices"],
  ["cs", "control panel mys", "win:cpl.mouse"],
  // české slovo v anglickém UI
  ["en", "obrazovka", "win:display"],
  ["en", "sound", "win:sound"],
];

test("hledání: česky i anglicky, bez diakritiky", () => {
  try {
    for (const [l, q, want] of SEARCH) {
      setLang(l);
      const got = rank(winItems(true, q), q, [], 5).map((r) => r.item.id);
      const top = Array.isArray(want) ? got.slice(0, want.length).sort() : got[0];
      assert.deepEqual(top, Array.isArray(want) ? [...want].sort() : want, `${l} „${q}“ → ${got.join(", ")}`);
    }
  } finally {
    setLang("cs");
  }
});

test("titulek a cíl položky", () => {
  setLang("cs");
  const [bt] = winItems(true, "x").filter((i) => i.id === "win:bluetooth");
  assert.equal(bt.title, "Nastavení Windows: Bluetooth a zařízení");
  assert.equal(bt.target, "ms-settings:bluetooth");
  const [m] = winItems(true, "x").filter((i) => i.id === "win:cpl.mouse");
  assert.equal(m.title, "Klasický ovládací panel: Myš");
  assert.equal(m.sub, "control /name Microsoft.Mouse");
  setLang("en");
  assert.equal(winItems(true, "x").find((i) => i.id === "win:bluetooth").title, "Windows Settings: Bluetooth & devices");
  setLang("cs");
});
