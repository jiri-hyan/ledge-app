// Tabulkové testy čisté logiky palety a modelu akcí. Bez transpilace: Node 22.6+ odstraní typy sám.
//   node --experimental-strip-types --test src/palette/search.test.mjs
// (.mjs, aby ho `tsc` ani Vite neřešily; importuje .ts přímo.)
import { test } from "node:test";
import assert from "node:assert/strict";
import { fold, score, segments, rank, pushRecent } from "./search.ts";
import { slugId, moveItem, isLink, linkActions, onPage, visibleGroups, cleanAction, draftError } from "../actions/model.ts";

test("fold: bez diakritiky, malá písmena, stejná délka", () => {
  for (const [a, b] of [["Příliš žluťoučký", "prilis zlutoucky"], ["ÚČTY", "ucty"], ["abc", "abc"], ["Ř-ž_ů", "r-z_u"]]) {
    assert.equal(fold(a), b);
    assert.equal(fold(a).length, a.length);
  }
});

test("score: tabulka shod a neshod", () => {
  /** [dotaz, text, sedí?] */
  const cases = [
    ["nas", "Nastavení", true],
    ["nastaveni", "Nastavení", true],
    ["NASTAV", "nastavení", true],
    ["zlut", "Žluťoučký kůň", true],
    ["nst", "Nastavení", true],
    ["xyz", "Nastavení", false],
    ["kun zlu", "Žluťoučký kůň", true],
    ["kun xyz", "Žluťoučký kůň", false],
    ["", "cokoli", true],
  ];
  for (const [q, t, ok] of cases) assert.equal(score(q, t) !== null, ok, `${q} ~ ${t}`);
});

test("score: začátek slova a souvislá shoda vedou", () => {
  const s = (q, t) => score(q, t).score;
  assert.ok(s("not", "Notes") > s("not", "Annotate"), "začátek textu");
  assert.ok(s("syn", "Poznámky: synchronizovat") > s("syn", "Poznámky: nesynchronizovat"), "začátek slova");
  assert.ok(s("pozn", "Poznámky") > s("pzk", "Poznámky"), "souvislá před rozházenou");
});

test("score: zvýraznění v originálním textu", () => {
  const m = score("zlut", "Žluťoučký");
  assert.deepEqual(m.ranges, [[0, 4]]);
  assert.deepEqual(segments("Žluťoučký", m.ranges), [{ text: "Žluť", hit: true }, { text: "oučký", hit: false }]);
  // podposloupnost: sousední zásahy se slijí
  const f = score("nst", "Nastavení");
  assert.deepEqual(f.ranges, [[0, 1], [2, 4]]);
  // dvě slova dotazu → dva rozsahy, seřazené
  assert.deepEqual(score("kun zlu", "Žluťoučký kůň").ranges, [[0, 3], [10, 13]]);
});

test("rank: titul > podtitul, nedávné nahoru, bez dotazu podle order", () => {
  const items = [
    { id: "a", title: "Spustit Orbit", sub: "projekt", order: 1 },
    { id: "b", title: "Otevřít složku", sub: "Orbit", order: 1 },
    { id: "c", title: "Nastavení: Vzhled", keywords: "motiv barva", order: 2 },
    { id: "d", title: "Nová poznámka", order: 0 },
  ];
  assert.deepEqual(rank(items, "orbi", []).map((r) => r.item.id), ["a", "b"]);
  assert.deepEqual(rank(items, "barva", []).map((r) => r.item.id), ["c"]);
  assert.deepEqual(rank(items, "barva", [])[0].ranges, [], "klíčové slovo nezvýrazňuje titul");
  assert.deepEqual(rank(items, "", ["c"]).map((r) => r.item.id), ["c", "d", "a", "b"]);
  // nedávné při dotazu jen pomůže, nepřebije výrazně lepší shodu
  assert.deepEqual(rank(items, "orbi", ["b"]).map((r) => r.item.id), ["a", "b"]);
  // při rovnocenné shodě rozhodne nedávné použití
  const twins = [{ id: "x", title: "Otevřít Ledge" }, { id: "y", title: "Otevřít Ledge" }];
  assert.deepEqual(rank(twins, "ledge", ["y"]).map((r) => r.item.id), ["y", "x"]);
  assert.equal(rank(items, "", [], 2).length, 2);
});

test("pushRecent: na začátek, bez duplicit, strop", () => {
  assert.deepEqual(pushRecent(["a", "b", "c"], "b"), ["b", "a", "c"]);
  assert.deepEqual(pushRecent(["a", "b"], "x", 2), ["x", "a"]);
});

test("model: slugId bez diakritiky a jednoznačné", () => {
  assert.equal(slugId("Mapy.cz", []), "mapy-cz");
  assert.equal(slugId("Účetnictví", ["ucetnictvi"]), "ucetnictvi-2");
  assert.equal(slugId("   ", []), "akce");
  assert.equal(slugId("!!!", ["akce", "akce-2"]), "akce-3");
});

test("model: moveItem přesune a nemění vstup", () => {
  const src = ["a", "b", "c", "d"];
  assert.deepEqual(moveItem(src, 0, 2), ["b", "c", "a", "d"]);
  assert.deepEqual(moveItem(src, 3, 0), ["d", "a", "b", "c"]);
  assert.deepEqual(moveItem(src, 1, 1), src);
  assert.deepEqual(moveItem(src, 9, 0), src);
  assert.deepEqual(src, ["a", "b", "c", "d"]);
});

test("model: odkaz = akce, která jen otevírá; karta Odkazy v pořadí akcí podle záložky", () => {
  const v = (id, kind, showOn = "both", script = false) => ({ id, kind, showOn, group: null, script });
  const list = [v("mapy", "url"), v("build", "command"), v("fotky", "folder", "personal"), v("prace", "url", "work"), v("calc", "app"), v("panel", "web"), v("doc", "file", "work"), v("skript", "url", "both", true)];
  assert.deepEqual(list.filter(isLink).map((a) => a.id), ["mapy", "fotky", "prace", "panel", "doc"]);
  assert.deepEqual(linkActions(list, "osobni").map((a) => a.id), ["mapy", "fotky", "panel"]);
  assert.deepEqual(linkActions(list, "prace").map((a) => a.id), ["mapy", "prace", "panel", "doc"]);
  assert.deepEqual(linkActions(list, "vlastni-1").map((a) => a.id), ["mapy", "fotky", "prace", "panel", "doc"]);
  assert.equal(onPage({ showOn: "work" }, "osobni"), false);
  assert.equal(onPage({ showOn: "work" }, "moje"), true);
});

test("model: viditelné skupiny podle záložky", () => {
  const v = (id, showOn, group = null) => ({ id, showOn, group });
  const list = [v("a", "both"), v("b", "work", "Dev"), v("c", "personal"), v("d", "work"), v("e", "both", "Dev")];
  assert.deepEqual(visibleGroups(list, "prace").map((g) => [g.group, g.items.map((x) => x.id)]), [[null, ["a", "d"]], ["Dev", ["b", "e"]]]);
  assert.deepEqual(visibleGroups(list, "osobni").map((g) => [g.group, g.items.map((x) => x.id)]), [[null, ["a", "c"]], ["Dev", ["e"]]]);
  // vlastní záložka (id z layout) = všechny akce
  assert.deepEqual(visibleGroups(list, "hry").map((g) => [g.group, g.items.map((x) => x.id)]), [[null, ["a", "c", "d"]], ["Dev", ["b", "e"]]]);
});

test("model: cleanAction zahodí prázdná a nepatřičná pole", () => {
  const a = cleanAction({ id: "w", label: " Web ", kind: "url", target: " https://x.cz ", args: ["x"], cwd: "", icon: "auto", group: " ", confirm: false, showOn: "both", shell: "cmd", terminal: false, timeoutS: 60 });
  assert.deepEqual(a, { id: "w", label: "Web", kind: "url", target: "https://x.cz" });
  const c = cleanAction({ id: "c", label: "C", kind: "command", target: "dir", args: ["", "-x"], cwd: "C:\\r", confirm: true, shell: "powershell", terminal: true, timeoutS: 30 });
  assert.deepEqual(c, { id: "c", label: "C", kind: "command", target: "dir", args: ["-x"], cwd: "C:\\r", confirm: true, shell: "powershell", terminal: true, timeoutS: 30 });
});

test("model: draftError — tabulka", () => {
  const ok = { id: "a", label: "A", kind: "url", target: "https://x.cz" };
  const cases = [
    [ok, null],
    [{ ...ok, label: "" }, "label"],
    [{ ...ok, target: "mapy.cz" }, "target"],
    [{ ...ok, kind: "folder", target: "slozka" }, "target"],
    [{ ...ok, kind: "folder", target: "C:\\x" }, null],
    [{ ...ok, kind: "app", target: "notepad.exe" }, null],
    [{ ...ok, kind: "app", target: "C:\\x\\a.bat" }, "target"],
    [{ ...ok, kind: "command", target: "npm test", args: ['a"b'] }, "args"],
    [{ ...ok, kind: "command", target: "npm test", cwd: "rel" }, "cwd"],
    [{ ...ok, icon: "C:\\i\\x.exe" }, "icon"],
  ];
  for (const [a, field] of cases) assert.equal(draftError(a)?.field ?? null, field, JSON.stringify(a));
});
