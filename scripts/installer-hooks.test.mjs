// Statická kontrola instalátoru a identifikátoru (NSIS se tu nepouští): hooks.nsh a tauri.conf.json.
// Spuštění: node --experimental-strip-types --test scripts/installer-hooks.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const hooks = readFileSync(join(root, "src-tauri", "installer", "hooks.nsh"), "utf8");
const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));

/** Tělo makra `!macro NAME … !macroend`. */
const macro = (name) => {
  const m = new RegExp(`!macro ${name}\\b([\\s\\S]*?)!macroend`).exec(hooks);
  assert.ok(m, `makro ${name} chybí`);
  return m[1];
};
const count = (s, re) => (s.match(re) ?? []).length;

test("identifikátor: povolené znaky, reverzní doména, nekončí .app; vydavatel je nastaven výslovně", () => {
  assert.match(conf.identifier, /^[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$/);
  assert.ok(!conf.identifier.endsWith(".app"), "přípona .app kolide s balíčkem macOS (Tauri varuje)");
  assert.notEqual(conf.identifier, "com.tauri.dev");
  assert.equal(conf.bundle.publisher, "Ledge", "vydavatel nesmí záviset na druhém segmentu identifikátoru");
});

test("hooks.nsh: starý identifikátor se hledá maskou, nikde se nepíše jménem", () => {
  const named = [...hooks.matchAll(/cz\.([A-Za-z0-9-]+)\.ledge/g)].map((m) => m[1]).filter((n) => n !== "*");
  assert.deepEqual(named, [], "v hooks.nsh smí být jen maska cz.*.ledge");
  assert.match(hooks, /cz\.\*\.ledge/);
  assert.match(macro("LEDGE_RM_OLD_PROFILES"), /EBWebView/, "smaže se jen složka s profilem WebView2");
  assert.match(macro("LEDGE_RM_OLD_PROFILES"), /\$LedgeTmp != "\$\{BUNDLEID\}"/, "nový identifikátor se tímto makrem nemaže");
});

test("hooks.nsh: dřívější složka se použije jen s ledge.exe uvnitř a jen když si ji uživatel nezměnil", () => {
  const pre = macro("NSIS_HOOK_PREINSTALL");
  assert.match(pre, /\$INSTDIR == "\$LOCALAPPDATA\\\$\{PRODUCTNAME\}"/);
  assert.match(pre, /\$\{MANUPRODUCTKEY\}/);
  assert.match(pre, /\$LedgePagesRan != 1/, "po úvodní stránce (průvodce) se výběr složky nepřepisuje");
  assert.match(pre, /LEDGE_RESOLVE_OLD_DIR/);
  assert.match(pre, /SetOutPath \$INSTDIR/);
  const res = macro("LEDGE_RESOLVE_OLD_DIR");
  assert.match(res, /\$\{UNINSTKEY\}" "InstallLocation"/);
  assert.match(res, /FileExists\} "\$LedgeDir\\\$\{MAINBINARYNAME\}\.exe"/);
  assert.match(res, /\$LedgeLen > 2/, "InstallLocation délky 1 se neořezává");
});

test("hooks.nsh: úvodní stránka zapíše nový klíč výrobce, ať odinstalace před instalací nedostane prázdnou cestu", () => {
  const w = macro("MUI_PAGE_WELCOME");
  assert.match(w, /!define MUI_PAGE_CUSTOMFUNCTION_PRE LedgeWelcomePre/);
  assert.match(w, /Function LedgeWelcomePre/, "funkce je uvnitř makra (MANUPRODUCTKEY se definuje až v šabloně)");
  assert.match(w, /WriteRegStr SHCTX "\$\{MANUPRODUCTKEY\}" "" \$LedgeDir/);
  assert.match(w, /\$PassiveMode = 1[\s\S]*Abort/, "/P úvodní stránku dál přeskakuje");
  assert.match(w, /MUI_PAGEDECLARATION_WELCOME/);
});

test("hooks.nsh: maska cz.*.ledge se v NSIS zpřísňuje jako is_legacy_name (prostřední segment a-z 0-9 - .)", () => {
  const rm = macro("LEDGE_RM_OLD_PROFILES");
  assert.match(rm, /abcdefghijklmnopqrstuvwxyz0123456789-\./);
  assert.match(rm, /\$LedgeLen > 9/, "cz..ledge (prázdný segment) se nemaže");
  assert.match(rm, /\$LedgeOk = 1/);
});

test("hooks.nsh: klíč starého vydavatele jde smazat jen s ledge.exe v uložené složce a rodič jen prázdný", () => {
  const find = macro("LEDGE_FIND_OLD_MAKER");
  assert.match(find, /EnumRegKey/);
  assert.match(find, /\$LedgeTmp != "\$\{MANUFACTURER\}"/, "nový klíč výrobce se nehledá ani nemaže");
  assert.match(find, /FileExists\} "\$LedgeDir\\\$\{MAINBINARYNAME\}\.exe"/);
  const drop = macro("LEDGE_DROP_OLD_MAKER");
  assert.match(drop, /DeleteRegKey SHCTX "Software\\\$LedgeOldKey\\\$\{PRODUCTNAME\}"/);
  assert.match(drop, /DeleteRegKey \/ifempty SHCTX "Software\\\$LedgeOldKey"/);
  assert.ok(!/DeleteRegKey SHCTX "Software\\\$LedgeOldKey"/.test(drop), "rodič se nikdy nemaže bez /ifempty");
});

test("hooks.nsh: odinstalace maže staré jen při volbě „smazat data“ a ne při aktualizaci; poznámky nikdy", () => {
  const post = macro("NSIS_HOOK_POSTUNINSTALL");
  assert.match(post, /\$DeleteAppDataCheckboxState = 1[\s\S]*\$UpdateMode <> 1/);
  assert.match(post, /LEDGE_RM_OLD_PROFILES/);
  assert.ok(!/LEDGE_DROP_OLD_MAKER/.test(post.replace(/^\s*;.*$/gm, "")), "mrtvý kód: ledge.exe už šablona smazala, klíč uklízí POSTINSTALL");
  assert.match(macro("NSIS_HOOK_POSTINSTALL"), /LEDGE_DROP_OLD_MAKER/);
  assert.ok(!/USERPROFILE/.test(post.replace(/^\s*;.*$/gm, "")), "poznámky (%USERPROFILE%\\Ledge) se nemažou");
  // dočasné věci migrace (webdata.rs) jsou UVNITŘ nové složky (EBWebView.migrating, .migrate-deferred, EBWebView.clean-*) a maže je RmDir /r nové složky z šablony
  const postCode = post.replace(/^\s*;.*$/gm, "");
  assert.ok(!/\.migrating/.test(postCode), "žádné .migrating vedle nové složky — jádro ho zakládá uvnitř");
  assert.ok(!/RmDir[^\n]*BUNDLEID/.test(postCode), "nová složka se maže šablonou Tauri, ne tady");
  assert.ok(!/jen cache/.test(hooks), "komentář „jen cache WebView2“ neplatí (je tam localStorage a cookies)");
});

test("hooks.nsh: proměnné jsou deklarované před prvním použitím; bloky LogicLib vyvážené", () => {
  for (const v of ["LedgeIdx", "LedgeTmp", "LedgeDir", "LedgeOldKey", "LedgeOldDir", "LedgeFind", "LedgePagesRan", "LedgeLen", "LedgeOk", "LedgeMid", "LedgeI", "LedgeJ", "LedgeCh", "LedgeCa", "LedgeHit"]) {
    const decl = hooks.indexOf(`Var ${v}\n`);
    assert.ok(decl >= 0, `Var ${v}`);
    assert.ok(hooks.indexOf(`$${v}`) > decl, `$${v} se použije před deklarací`);
  }
  const code = hooks.replace(/^\s*;.*$/gm, "");
  assert.equal(count(code, /\$\{If\}|\$\{IfNot\}/g), count(code, /\$\{EndIf\}/g), "If/EndIf");
  assert.equal(count(code, /\$\{Do\}|\$\{DoWhile\}/g), count(code, /\$\{Loop\}/g), "Do/Loop");
  assert.equal(count(code, /\$\{While\}/g), count(code, /\$\{EndWhile\}/g), "While/EndWhile");
});
