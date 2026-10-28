// Vlastní zavřený Ledge (`compose.ts`): předvolby z 15 vzhledů, ořez jako Rust, úpravy kostek. Bez DOM.
//   node --experimental-strip-types --test src/strip/compose.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { STRIP_LOOK_IDS } from "./looks.ts";
import {
  COMPOSE_ITEMS, COMPOSE_SHAPES, DEFAULT_COMPOSE, ITEM_STYLES, MAX_PIECES, W_MAX, W_MIN,
  composeOf, drawKind, hRange, hasItem, nextStyle, normCompose, putPiece, removePiece, setAllStyles, setStyle, shiftPiece, total, whereIs, withShape,
} from "./compose.ts";

const sig = (c) => `${c.shape} ${c.width}×${c.height} r${c.round} | ${["left", "center", "right"].map((s) => c[s].map((p) => `${p.item}:${p.style}`).join(",")).join(" | ")}${c.hideIdle ? " ghost" : ""}${c.rotate ? " rotate" : ""}`;

test("look → compose: každý z 15 hotových vzhledů má předvolbu", () => {
  const table = {
    classic: "capsule 300×30 r55 | claude:icon | time:text | limit5h:ring",
    island: "island 150×30 r100 |  | claude:text | ",
    dots: "capsule 84×18 r100 |  | claude:dot,servers:dot,repos:dot,music:dot | ",
    line: "line 620×3 r0 | claude:dot | limit5h:dot | music:dot",
    icons: "capsule 190×30 r55 |  | claude:badge,servers:badge,repos:badge,music:badge | ",
    clock: "capsule 150×34 r50 |  | time:text | claude:dot",
    corner: "corner 150×28 r60 | claude:icon |  | time:text",
    ghost: "island 220×30 r100 |  | claude:text |  ghost",
    bar: "capsule 620×24 r0 | claude:text | time:text | limit5h:text",
    notch: "notch 240×32 r70 | claude:icon |  | time:text",
    progress: "capsule 210×40 r40 |  | limit5h:graph | ",
    ticker: "island 230×28 r100 |  | claude:text,limit5h:text,music:text,timer:text |  rotate",
    minimum: "capsule 112×26 r100 | battery:icon |  | time:text",
    trio: "capsule 280×32 r60 | weather:text | time:text | cpu:text",
    rings: "capsule 250×36 r60 | limit5h:ring | time:text | limitWeek:ring",
  };
  const looks = STRIP_LOOK_IDS.filter((x) => x !== "custom");
  assert.equal(looks.length, 15);
  for (const id of looks) {
    const c = composeOf(id);
    assert.equal(sig(c), table[id], id);
    // předvolba už je v rozsahu (ořez ji nezmění) a má 1–5 položek ze skutečných zdrojů
    assert.deepEqual(normCompose(c), c, `${id} v rozsahu`);
    assert.ok(total(c) >= 1 && total(c) <= MAX_PIECES, id);
  }
  assert.equal(sig(composeOf("custom")), table.classic, "neznámý = Klasický");
  assert.equal(sig(composeOf("bubble")), table.classic);
  // nová kopie: úprava předvolbu nezmění
  composeOf("classic").left.push({ item: "date", style: "text" });
  assert.equal(sig(composeOf("classic")), table.classic);
  assert.deepEqual(DEFAULT_COMPOSE, composeOf("classic"));
});

test("normCompose = Rust Compose::norm (rozsahy, každá položka jednou, 1–5)", () => {
  const cases = [
    [undefined, "capsule 300×30 r55 | claude:icon | time:text | limit5h:ring"],
    [{ width: 5, height: 200, round: 250 }, `capsule ${W_MIN}×52 r100 | claude:icon | time:text | limit5h:ring`],
    [{ width: 9000, height: 0 }, `capsule ${W_MAX}×18 r55 | claude:icon | time:text | limit5h:ring`],
    [{ shape: "line", height: 30 }, "line 300×10 r55 | claude:icon | time:text | limit5h:ring"],
    [{ shape: "bubble", left: [], center: [], right: [] }, "capsule 300×30 r55 |  | claude:icon | "],
    [{ left: [{ item: "time" }, { item: "date" }, { item: "cpu" }], center: [{ item: "claude" }, { item: "time" }, { item: "music" }], right: [{ item: "timer" }, { item: "repos" }] },
      "capsule 300×30 r55 | time:icon,date:icon,cpu:icon | claude:icon,music:icon | "],
    [{ left: [{ item: "mic", style: "ring" }, { item: "cpu", style: "spark" }], center: [], right: [] }, "capsule 300×30 r55 | cpu:icon |  | "],
  ];
  for (const [raw, want] of cases) assert.equal(sig(normCompose(raw)), want, JSON.stringify(raw));
});

test("kostky: přidat, přesunout, odebrat, styl — vstup netknutý", () => {
  const c = composeOf("classic");
  const before = JSON.stringify(c);
  const cases = [
    [putPiece(c, "date", "center"), "capsule 300×30 r55 | claude:icon | time:text,date:icon | limit5h:ring"],
    [putPiece(c, "date", "left", 0), "capsule 300×30 r55 | date:icon,claude:icon | time:text | limit5h:ring"],
    // přesun si styl nechá
    [putPiece(c, "time", "right"), "capsule 300×30 r55 | claude:icon |  | limit5h:ring,time:text"],
    [removePiece(c, "time"), "capsule 300×30 r55 | claude:icon |  | limit5h:ring"],
    [shiftPiece(c, "time", -1), "capsule 300×30 r55 | claude:icon,time:text |  | limit5h:ring"],
    [shiftPiece(c, "claude", -1), "capsule 300×30 r55 | claude:icon | time:text | limit5h:ring"],
    [shiftPiece(putPiece(c, "date", "center"), "time", 1), "capsule 300×30 r55 | claude:icon | date:icon,time:text | limit5h:ring"],
    [setStyle(c, "time", "next"), "capsule 300×30 r55 | claude:icon | time:ring | limit5h:ring"],
    [setStyle(c, "claude", "badge"), "capsule 300×30 r55 | claude:badge | time:text | limit5h:ring"],
    [setAllStyles(c, "dot"), "capsule 300×30 r55 | claude:dot | time:dot | limit5h:dot"],
    [withShape(c, "line"), "line 300×10 r55 | claude:icon | time:text | limit5h:ring"],
    [withShape(composeOf("line"), "capsule"), "capsule 620×18 r0 | claude:dot | limit5h:dot | music:dot"],
  ];
  for (const [got, want] of cases) assert.equal(sig(got), want);
  assert.equal(JSON.stringify(c), before, "vstup netknutý");
  // plno = beze změny, poslední zůstane
  let full = c;
  for (const it of ["date", "music", "cpu"]) full = putPiece(full, it, "center");
  assert.equal(total(full), MAX_PIECES);
  assert.equal(putPiece(full, "weather", "left"), full);
  assert.ok(!hasItem(putPiece(full, "weather", "left"), "weather"));
  let one = c;
  for (const it of ["time", "limit5h", "claude"]) one = removePiece(one, it);
  assert.equal(total(one), 1);
  assert.deepEqual(whereIs(one, "claude"), ["left", 0]);
  assert.equal(nextStyle("dot"), "icon");
});

test("rozsahy a seznamy = Rust looks/compose.rs", () => {
  const src = readFileSync(new URL("../../src-tauri/src/looks/compose.rs", import.meta.url), "utf8");
  const variants = (name) => {
    const start = src.indexOf(`pub enum ${name} {`);
    const body = src.slice(start, src.indexOf("\n}", start));
    return [...body.matchAll(/^\s*([A-Z][A-Za-z0-9]*),\s*$/gm)].map((m) => m[1][0].toLowerCase() + m[1].slice(1));
  };
  assert.deepEqual(variants("Form"), [...COMPOSE_SHAPES]);
  assert.deepEqual(variants("PieceItem"), [...COMPOSE_ITEMS]);
  assert.deepEqual(variants("PieceStyle"), [...ITEM_STYLES]);
  assert.deepEqual(hRange("line"), [2, 10]);
  assert.deepEqual(hRange("drop"), [18, 52]);
});

test("náhrady stylů (ComposePreview) = Rust compose::parts: prstenec / graf bez podílu → ikona", () => {
  // [položka, styl, má podíl] → co se nakreslí
  const table = [
    ["limit5h", "ring", true, "ring"], ["limit5h", "graph", true, "meter"],
    ["claude", "ring", false, "ring"], ["claude", "graph", false, "icon"],
    ["time", "ring", false, "icon"], ["time", "graph", false, "icon"],
    ["music", "ring", false, "icon"], ["music", "graph", true, "meter"],
    ["weather", "ring", false, "icon"], ["servers", "graph", false, "icon"],
    ["battery", "ring", true, "ring"], ["cpu", "graph", true, "meter"],
    ["activity", "ring", false, "icon"], ["activity", "graph", false, "icon"], ["repos", "icon", false, "icon"], ["repos", "badge", false, "badge"], ["repos", "text", false, "text"], ["repos", "dot", false, "dot"],
  ];
  for (const [item, style, frac, want] of table) assert.equal(drawKind(item, style, frac), want, `${item}:${style}`);
  // Rust: výjimka bez podílu: jen Claude + prstenec
  const src = readFileSync(new URL("../../src-tauri/src/looks/compose.rs", import.meta.url), "utf8");
  assert.match(src, /None if p\.item == PieceItem::Claude => vec!\[Part::Ring/);
});

test("shiftPiece přes všechny hranice slotů", () => {
  const mk = (l, c, r) => normCompose({ ...DEFAULT_COMPOSE, left: l.map((item) => ({ item, style: "icon" })), center: c.map((item) => ({ item, style: "icon" })), right: r.map((item) => ({ item, style: "icon" })) });
  const at = (c) => ["left", "center", "right"].map((s) => c[s].map((p) => p.item).join(",")).join("|");
  const base = mk(["claude", "time"], ["date"], ["cpu", "weather"]);
  const table = [
    ["time", 1, "claude|time,date|cpu,weather"], // pravý kraj levého → začátek středu
    ["claude", -1, "claude,time|date|cpu,weather"], // úplný začátek = beze změny
    ["claude", 1, "time,claude|date|cpu,weather"], // uvnitř slotu prohodit
    ["date", 1, "claude,time||date,cpu,weather"], // střed → začátek pravého
    ["date", -1, "claude,time,date||cpu,weather"], // střed → konec levého
    ["cpu", -1, "claude,time|date,cpu|weather"], // začátek pravého → konec středu
    ["weather", 1, "claude,time|date|cpu,weather"], // úplný konec = beze změny
    ["weather", -1, "claude,time|date|weather,cpu"],
  ];
  for (const [item, dir, want] of table) assert.equal(at(shiftPiece(base, item, dir)), want, `${item} ${dir}`);
  const full = mk(["claude", "time", "date", "cpu", "weather"], [], []);
  assert.equal(at(shiftPiece(full, "weather", 1)), "claude,time,date,cpu|weather|", "konec levého slotu → střed");
  assert.equal(at(shiftPiece(base, "memory", 1)), at(base), "chybějící položka");
});
