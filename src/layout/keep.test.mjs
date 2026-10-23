// Rozpracované úpravy rozložení přežijí zavření šuplíku (`keep.ts`); zahodí je jen výslovné Zrušit.
//   node --experimental-strip-types --test src/layout/keep.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { setSize } from "./model.ts";
import { fullLayout } from "./fullLayout.mjs";
import { pushUndo, popUndo } from "./history.ts";
import { discards, escAction, isDirty, showResume } from "./keep.ts";

const src = (p) => readFileSync(new URL(p, import.meta.url), "utf8");

test("skrytí, blur ani znovuotevření draft nezahodí; zahodí ho jen Zrušit a výslovný odchod ze stránky", () => {
  for (const ev of ["hide", "blur", "reopen"]) assert.equal(discards(ev), false, ev);
  for (const ev of ["cancel", "page-away"]) assert.equal(discards(ev), true, ev);
});

test("draft i historie Zpět po „zavření“ zůstanou (stav se při hide nemění, Zpět dál funguje)", () => {
  const a = fullLayout();
  const b = setSize(a, "prace", "claude", "s");
  const st = pushUndo([], a, b);
  // „hide“ = žádná operace nad stavem; po znovuotevření je to pořád ono
  assert.equal(isDirty(a, b), true);
  assert.deepEqual(popUndo(st).layout, a);
});

test("připomínka po znovuotevření jen s opravdu neuloženými změnami", () => {
  const a = fullLayout();
  const b = setSize(a, "prace", "claude", "s");
  assert.equal(showResume("reopen", a, b), true);
  assert.equal(showResume("reopen", a, a), false, "bez změn není co připomínat");
  assert.equal(showResume("hide", a, b), false);
  assert.equal(showResume("reopen", null, null), false);
});

test("Esc mimo tah: se změnami se zeptá, beze změn zruší", () => {
  const a = fullLayout();
  assert.equal(escAction(a, setSize(a, "prace", "claude", "s")), "ask");
  assert.equal(escAction(a, a), "cancel");
});

test("zapojení: skrytí šuplíku draft nezahazuje, Esc jde přes escEdit, lišta má připomínku", () => {
  const app = src("../App.tsx");
  assert.ok(!/v === "hidden"[^\n]*cancelEdit/.test(app), "hide nesmí volat cancelEdit");
  assert.ok(app.includes("resumeEdit()") && app.includes("escEdit()"), "znovuotevření + Esc s potvrzením");
  assert.ok(src("./EditBar.tsx").includes("cards.layout.resumed"), "připomínka v liště");
});
