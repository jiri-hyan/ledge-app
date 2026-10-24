// Tabulkové testy návrhu zástupce z přetaženého (`dropshortcut.ts`) a zapojení.
//   node --experimental-strip-types --test src/actions/dropshortcut.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { draftFrom, hostCards, readPayload, visibility } from "./dropshortcut.ts";
import { draftError, labelFrom } from "./model.ts";

test("readPayload: jen známé typy a neprázdný cíl", () => {
  const cases = [
    [{ kind: "url", target: " https://example.com/a ", skipped: 0 }, { kind: "url", target: "https://example.com/a", skipped: 0 }],
    [{ kind: "folder", target: "C:\\Users\\uzivatel\\Projekty", skipped: 2 }, { kind: "folder", target: "C:\\Users\\uzivatel\\Projekty", skipped: 2 }],
    [{ kind: "app", target: "C:\\Tools\\app.exe", skipped: -1 }, { kind: "app", target: "C:\\Tools\\app.exe", skipped: 0 }],
    [{ kind: "command", target: "npm run build", skipped: 0 }, null],
    [{ kind: "url", target: "  ", skipped: 0 }, null],
    [{ kind: "url" }, null],
    [null, null],
    ["https://example.com", null],
  ];
  for (const [inp, want] of cases) assert.deepEqual(readPayload(inp), want, JSON.stringify(inp));
});

test("draftFrom: popisek z cíle, jednoznačné id, aplikace se ptá, nikdy příkaz", () => {
  const web = draftFrom({ kind: "url", target: "https://www.example.com/docs", skipped: 0 }, []);
  assert.deepEqual([web.kind, web.label, web.id, web.confirm], ["url", "example.com", "example-com", undefined]);
  const again = draftFrom({ kind: "url", target: "https://www.example.com/docs", skipped: 0 }, ["example-com"]);
  assert.equal(again.id, "example-com-2");
  const dir = draftFrom({ kind: "folder", target: "C:\\Users\\uzivatel\\Projekty\\app-v2", skipped: 0 }, []);
  assert.deepEqual([dir.label, dir.id], ["app-v2", "app-v2"]);
  const exe = draftFrom({ kind: "app", target: "C:\\Tools\\app.exe", skipped: 0 }, []);
  assert.deepEqual([exe.kind, exe.label, exe.confirm], ["app", "app", true]);
  for (const p of [web, dir, exe]) assert.notEqual(p.kind, "command");
  // návrh projde rychlou kontrolou editoru (jinak by člověk dostal rovnou chybu)
  for (const p of [web, dir, exe]) assert.equal(draftError(p), null, p.target);
});

test("labelFrom: host, složka, soubor, exe", () => {
  const cases = [["https://www.example.com/a", "example.com"], ["http://example.com:8080/", "example.com"], ["C:\\a\\b\\", "b"], ["C:\\a\\doc.pdf", "doc.pdf"], ["C:\\a\\app.EXE", "app"], ["", ""]];
  for (const [inp, want] of cases) assert.equal(labelFrom(inp), want, inp);
});

test("visibility: kdy se nová akce na záložce ukáže a kterou kartu nabídnout", () => {
  const cases = [
    ["odkaz, je karta Odkazy", ["projects", "links"], "url", { visible: true }],
    ["odkaz, jsou jen Rychlé akce", ["actions"], "folder", { visible: true }],
    ["odkaz, žádná karta", ["projects", "notes"], "file", { visible: false, add: "links" }],
    ["aplikace, jen Odkazy → neukáže", ["links"], "app", { visible: false, add: "actions" }],
    ["aplikace, Rychlé akce", ["actions"], "app", { visible: true }],
  ];
  for (const [name, cards, kind, want] of cases) assert.deepEqual(visibility(cards, kind), want, name);
  assert.deepEqual(hostCards("app"), ["actions"]);
});

test("zapojení: událost drop-shortcut v App, panel v MainPage, nic se neukládá bez potvrzení", () => {
  const read = (f) => fs.readFileSync(new URL(f, import.meta.url), "utf8");
  assert.ok(read("../App.tsx").includes('"drop-shortcut"'));
  assert.ok(read("../MainPage.tsx").includes("<DropNew"));
  // zápis do configu jen v `save` (voláno z „Přidat“ editoru), nikde při příjmu návrhu
  assert.equal((read("./DropNew.tsx").match(/saveConfig/g) ?? []).length, 1);
  assert.ok(!read("./dropstate.ts").includes("saveConfig"));
});

test("zóny přihrádky a nového zástupce se nekryjí: výběr dělá jádro podle stavu šuplíku (test v Rustu)", () => {
  const rs = fs.readFileSync(new URL("../../src-tauri/src/shelf/shortcut.rs", import.meta.url), "utf8");
  assert.ok(rs.includes("fn zones_never_overlap"), "tabulkový test zón je v Rustu");
  const hover = fs.readFileSync(new URL("../../src-tauri/src/threads/hover.rs", import.meta.url), "utf8");
  assert.ok(hover.includes("target_for("), "hover vybírá zónu přes target_for");
});
