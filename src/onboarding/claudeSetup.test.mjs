// Tabulkové testy kroku „Nastavení s Claudem“ (`claudeSetup.ts`): zadání pro Clauda a volba, co krok ukáže.
//   node --experimental-strip-types --test src/onboarding/claudeSetup.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { PRO_CLAUDE_URL, buildSetupPrompt, hasConnectionKit, setupView } from "./claudeSetup.ts";

// překladač jako t(): čeština a proměnné {name}
const cs = (_key, text, vars) => (vars ? text.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : text);

const kit = { enabled: true, url: "http://127.0.0.1:47800/mcp", cliCommand: 'claude mcp add --scope user ledge -- "C:/app/ledge.exe" --mcp', desktopConfig: '{ "mcpServers": { "ledge": {} } }' };

test("zadání: odkaz na návod, zpovídání, schválení před změnou, nic nespouštět", () => {
  const p = buildSetupPrompt(cs);
  assert.ok(p.includes(PRO_CLAUDE_URL), "odkaz na návod pro Clauda");
  assert.ok(p.includes("releases/latest/download/pro-claude.md"));
  assert.match(p, /zeptej/);
  assert.match(p, /2–3 rozložení/);
  assert.match(p, /schválím/);
  assert.match(p, /Nic nespouštěj/);
  assert.match(p, /otevři šuplík/);
  assert.ok(!/\{url\}/.test(p), "proměnná je dosazená");
  assert.equal(p.split("\n\n").length, 3, "tři odstavce");
});

test("zadání: vlastní odkaz se dosadí, v textu nezůstane žádná neobsazená proměnná", () => {
  const p = buildSetupPrompt(cs, "https://example.com/pro-claude.md");
  assert.ok(p.includes("https://example.com/pro-claude.md"));
  assert.ok(!p.includes(PRO_CLAUDE_URL));
  assert.ok(!/\{\w+\}/.test(p));
});

test("sada pro připojení: obojí musí být neprázdné", () => {
  assert.equal(hasConnectionKit(kit), true);
  assert.equal(hasConnectionKit({ ...kit, cliCommand: "  " }), false);
  assert.equal(hasConnectionKit({ ...kit, desktopConfig: "" }), false);
  assert.equal(hasConnectionKit(null), false);
  assert.equal(hasConnectionKit(undefined), false);
});

test("volba zobrazení: starší jádro (mcp_state selže) = jen ruční cesta, bez pádu", () => {
  const rows = [
    ["error", "manual", null, "manualOnly"],
    ["error", "claude", null, "manualOnly"],
    ["error", "claude", kit, "manualOnly"],
    ["idle", "manual", null, "loading"],
    ["loading", "claude", null, "loading"],
    ["ready", "manual", { ...kit, enabled: false }, "choose"],
    ["ready", "manual", kit, "choose"],
    ["ready", "claude", { ...kit, enabled: false }, "choose"],
    ["ready", "claude", kit, "connect"],
    ["ready", "claude", { ...kit, cliCommand: "" }, "choose"],
    ["ready", "claude", null, "choose"],
  ];
  for (const [load, mode, state, want] of rows) {
    assert.equal(setupView(load, mode, state), want, `${load}/${mode}/${state ? (state.enabled ? "zapnuto" : "vypnuto") : "bez stavu"}`);
  }
});
