// Tabulkové testy složení zadání z dev chyby (`devbrief.ts`).
//   node --experimental-strip-types --test src/suggest/devbrief.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { devBriefBody } from "./devbrief.ts";

const L = {
  title: "Chyba dev serveru", project: "Projekt", url: "URL", branch: "Větev", dirty: (n) => `${n} změn`, cmd: "Příkaz",
  uptime: "Běží", errors: "Chyb ve výstupu", output: "Výstup", steps: "Kroky", expected: "Čekal jsem", happened: "Stalo se", task: "Najít příčinu a opravit",
};
const full = { project: "Atlas", url: "http://localhost:5173", branch: "main", dirty: 3, cmd: "npm run dev -- --port 5173", uptime: "12 min", errors: 2, lines: ["Error: boom", "    at f (/a.js:1:1)"] };

test("plný balíček: hlavička, výstup v bloku, kostra a první bod", () => {
  const b = devBriefBody(full, L);
  assert.ok(b.startsWith("# Chyba dev serveru: Atlas\n"));
  for (const s of ["**Projekt:** Atlas", "**URL:** http://localhost:5173", "**Větev:** main (3 změn)", "**Příkaz:** `npm run dev -- --port 5173`", "**Běží:** 12 min", "**Chyb ve výstupu:** 2", "```\nError: boom\n    at f (/a.js:1:1)\n```", "**Kroky:**", "- [ ] Najít příčinu a opravit"])
    assert.ok(b.includes(s), s);
});

test("co není známo, se nevypíše (žádné divadlo)", () => {
  const b = devBriefBody({ ...full, url: null, branch: null, dirty: null, cmd: null, uptime: null, lines: [] }, L);
  for (const s of ["**URL", "**Větev", "**Příkaz", "**Běží", "```"]) assert.ok(!b.includes(s), s);
  assert.ok(b.includes("**Projekt:** Atlas"));
});

test("čistá větev bez dirty; ``` v logu neukončí blok", () => {
  const b = devBriefBody({ ...full, dirty: 0, lines: ["a ```x``` b"] }, L);
  assert.ok(b.includes("**Větev:** main\n"));
  assert.equal(b.split("```").length - 1, 2);
});

test("první neprázdný řádek (cíl zadání) je nadpis a bod `[ ]` je k dispozici", () => {
  const b = devBriefBody(full, L);
  assert.equal(b.split("\n")[0], "# Chyba dev serveru: Atlas");
  assert.ok(/^- \[ \] /m.test(b));
});
