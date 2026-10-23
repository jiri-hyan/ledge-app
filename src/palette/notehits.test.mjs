// Tabulkové testy slučování zásahů hledání v těle poznámek do položek palety.
import { test } from "node:test";
import assert from "node:assert/strict";
import { mergeNoteHits } from "./notehits.ts";
import { rank } from "./search.ts";

const nav = { onOpenNote: (n) => (nav.opened = n) };
const note = (path, title, sub = "") => ({ id: `note:${path}`, section: "note", order: 5, title, sub, run: () => {} });
const hit = (path, title, snippet) => ({ path, title, snippet, kind: "info", done: false });

test("bez zásahů se nic nemění", () => {
  const items = [note("a.md", "A")];
  assert.equal(mergeNoteHits(items, [], "x", nav, "?"), items);
});

test("zásah dá existující poznámce úryvek a klíčová slova, ostatní nechá", () => {
  const items = [note("a.md", "Nákup", "info"), note("b.md", "Jiná"), { id: "app:close", section: "app", title: "Zavřít šuplík", run: () => {} }];
  const out = mergeNoteHits(items, [hit("a.md", "Nákup", "…koupit mléko…")], "mleko", nav, "?");
  assert.equal(out.length, 3);
  assert.equal(out[0].sub, "…koupit mléko…");
  assert.match(out[0].keywords, /mleko/);
  assert.equal(out[1], items[1]);
  assert.equal(out[2], items[2]);
});

test("zásah titulku (prázdný úryvek) nechá původní podtitulek", () => {
  const out = mergeNoteHits([note("a.md", "Nákup", "info")], [hit("a.md", "Nákup", "")], "nakup", nav, "?");
  assert.equal(out[0].sub, "info");
});

test("poznámka mimo seznam se přidá a otevře se přes nav", () => {
  const out = mergeNoteHits([note("a.md", "A")], [hit("stara.md", "Stará", "…faktura…"), hit("bez.md", "", "…faktura…")], "faktura", nav, "(bez názvu)");
  assert.equal(out.length, 3);
  assert.deepEqual([out[1].id, out[1].title, out[1].sub], ["note:stara.md", "Stará", "…faktura…"]);
  assert.equal(out[2].title, "(bez názvu)");
  out[1].run();
  assert.equal(nav.opened.path, "stara.md");
});

test("rank najde poznámku podle těla i když dotaz v titulku není (a slabší než shoda titulku)", () => {
  const items = mergeNoteHits(
    [note("a.md", "Faktura říjen"), note("b.md", "Porada")],
    [hit("b.md", "Porada", "…schválit fakturu Novák…")],
    "faktur novak",
    nav,
    "?",
  );
  const r = rank(items, "faktur novak", []);
  assert.deepEqual(r.map((x) => x.item.id), ["note:b.md"], "A nemá 'novak' → nesedí; B sedí přes úryvek");
  const r2 = rank(mergeNoteHits([note("a.md", "Faktura říjen"), note("b.md", "Porada")], [hit("b.md", "Porada", "…fakturu…")], "faktur", nav, "?"), "faktur", []);
  assert.deepEqual(r2.map((x) => x.item.id), ["note:a.md", "note:b.md"], "shoda v titulku vede");
});
