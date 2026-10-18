// Tabulkový test čisté logiky bubliny MSIX (`model.ts`): co z Rustu ukázat v Nastavení a hlavičce.
//   node --experimental-strip-types --test src/pkgctx/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { bubbleOf } from "./model.ts";

const pkg = (over = {}) => ({ full: "Claude_pzs8sxrjxfjjc", family: "Claude_pzs8sxrjxfjjc", name: "Claude", identity: false, ...over });
const view = (p, bubble = "C:\\x\\LocalCache\\Roaming\\Ledge") => ({ package: p, bubbleConfigDir: bubble, configDir: "C:\\x\\Roaming\\Ledge" });

test("bublina: mimo / nenačteno = nic, uvnitř = jméno aplikace a cesty", () => {
  const cases = [
    ["nenačteno", null, null],
    ["undefined", undefined, null],
    ["mimo balíček", view(null), null],
    ["Claude", view(pkg()), { app: "Claude", family: "Claude_pzs8sxrjxfjjc", bubbleDir: "C:\\x\\LocalCache\\Roaming\\Ledge", configDir: "C:\\x\\Roaming\\Ledge" }],
    ["bez jména → z rodiny", view(pkg({ name: "" })), { app: "Claude", family: "Claude_pzs8sxrjxfjjc", bubbleDir: "C:\\x\\LocalCache\\Roaming\\Ledge", configDir: "C:\\x\\Roaming\\Ledge" }],
    ["neznámá cesta v bublině", view(pkg(), null), { app: "Claude", family: "Claude_pzs8sxrjxfjjc", bubbleDir: null, configDir: "C:\\x\\Roaming\\Ledge" }],
    ["prázdná cesta = neznámá", view(pkg(), "  "), { app: "Claude", family: "Claude_pzs8sxrjxfjjc", bubbleDir: null, configDir: "C:\\x\\Roaming\\Ledge" }],
    ["úplně prázdný balíček", view(pkg({ name: "", family: "", full: "" })), null],
  ];
  for (const [name, v, want] of cases) assert.deepEqual(bubbleOf(v), want, name);
});
