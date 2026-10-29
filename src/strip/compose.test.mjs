// Vlastní zavřený Ledge (`compose.ts`): předvolby z 15 vzhledů, ořez jako Rust, úpravy kostek. Bez DOM.
//   node --experimental-strip-types --test src/strip/compose.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { STRIP_LOOK_IDS } from "./looks.ts";
import {
  COMPOSE_ITEMS, COMPOSE_SHAPES, DEFAULT_COMPOSE, DESIGN_IDS, ITEM_STYLES, MAX_PIECES, MAX_ZONE, W_MAX, W_MIN,
  composeOf, designOf, setPieceColor, stylesFor, drawKind, hRange, hasItem, nextStyle, normCompose, putPiece, removePiece, setAllStyles, setStyle, shiftPiece, total, whereIs, withShape,
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
      "capsule 300×30 r55 | time:icon,date:icon,cpu:icon | claude:icon,music:icon | timer:icon,repos:icon"],
    [{ left: [{ item: "mic", style: "ring" }, { item: "cpu", style: "spark" }], center: [], right: [] }, "capsule 300×30 r55 | cpu:spark |  | "],
    [{ left: [{ item: "cpu", style: "glow" }], center: [], right: [] }, "capsule 300×30 r55 | cpu:icon |  | "],
    // zóna nejvýš 5 (starý config z kola 23 se načte celý)
    [{ left: ["time", "date", "cpu", "memory", "battery", "weather"].map((item) => ({ item })), center: [], right: [] }, "capsule 300×30 r55 | time:icon,date:icon,cpu:icon,memory:icon,battery:icon |  | "],
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
  for (const it of ["date", "music", "cpu", "memory"]) full = putPiece(full, it, "center");
  assert.equal(full.center.length, MAX_ZONE);
  assert.equal(putPiece(full, "battery", "center"), full, "plná zóna");
  for (const it of ["battery", "timer", "servers", "repos", "limitWeek"]) full = putPiece(full, it, it === "battery" || it === "timer" ? "left" : "right");
  assert.equal(total(full), MAX_PIECES);
  assert.equal(putPiece(full, "weather", "left"), full);
  assert.ok(!hasItem(putPiece(full, "weather", "left"), "weather"));
  let one = c;
  for (const it of ["time", "limit5h", "claude"]) one = removePiece(one, it);
  assert.equal(total(one), 1);
  assert.deepEqual(whereIs(one, "claude"), ["left", 0]);
  assert.equal(nextStyle("ring5h"), "icon");
  // klik na kostku: jen styly, které položka nabízí (čas nemá průběh ani Claude styly)
  let st = composeOf("classic");
  const seenStyles = [];
  for (let i = 0; i < 12; i++) { st = setStyle(st, "time", "next"); seenStyles.push(st.center[0].style); }
  assert.ok(!seenStyles.some((s) => s === "spark" || s === "dotPct" || s === "ring5h"), seenStyles.join());
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

test("kolo 25: výzdoba, barvy dílů, styly podle položky, designové předvolby", () => {
  // starý compose bez výzdoby = dnešní vzhled
  const old = normCompose({ shape: "capsule", left: [{ item: "claude", style: "icon" }], center: [{ item: "time", style: "text" }], right: [{ item: "limit5h", style: "ring" }] });
  assert.deepEqual(old, DEFAULT_COMPOSE);
  assert.deepEqual([old.deco.edge, old.fill.kind, old.sep, old.mono], ["none", "black", "none", false]);
  // ořez barev jako Rust (`composedeco::norm_*`)
  const n = normCompose({ deco: { edge: "gradient", color: "#FF8800", color2: "zelená", width: 9 }, fill: { kind: "sky", from: "#123456" }, sep: "islands", center: [{ item: "time", style: "chip", color: "nic" }, { item: "date", color: "ACCENT" }] });
  assert.deepEqual([n.deco.color, n.deco.color2, n.deco.width, n.fill.kind, n.fill.from, n.sep], ["#ff8800", "", 3, "black", "#123456", "islands"]);
  assert.deepEqual(n.center, [{ item: "time", style: "chip" }, { item: "date", style: "icon", color: "accent" }]);
  assert.equal(normCompose({ deco: { color: "Claude" } }).deco.color, "claude");
  // barva dílu: nastavit / vrátit na „podle hodnoty“ (klíč zmizí)
  const red = setPieceColor(DEFAULT_COMPOSE, "time", "#FF0000");
  assert.equal(red.center[0].color, "#ff0000");
  assert.ok(!("color" in setPieceColor(red, "time", "").center[0]));
  assert.ok(!("color" in DEFAULT_COMPOSE.center[0]), "vstup netknutý");
  // průběh jen s historií (CPU / paměť), Claude styly jen Claude
  for (const [item, has, not] of [["cpu", "spark", "dotPct"], ["memory", "spark", "ring5h"], ["claude", "dotPct", "spark"], ["limit5h", "bar", "spark"], ["weather", "iconText", "spark"]]) {
    assert.ok(stylesFor(item).includes(has), `${item} ${has}`);
    assert.ok(!stylesFor(item).includes(not), `${item} bez ${not}`);
  }
  // náhrady nových stylů = Rust compose::parts
  for (const [item, style, frac, want] of [["claude", "ring5h", false, "ring"], ["time", "ring5h", false, "icon"], ["limit5h", "bar", true, "under"], ["time", "bar", false, "text"], ["cpu", "spark", true, "spark"], ["limit5h", "spark", true, "meter"], ["time", "chip", false, "iconText"], ["claude", "dotPct", false, "dotPct"]])
    assert.equal(drawKind(item, style, frac), want, `${item}:${style}`);
  // 6 předvoleb: v rozsahu, každá jiná výzdoba, nic mimo seznamy
  assert.equal(DESIGN_IDS.length, 6);
  const seen = new Set();
  for (const id of DESIGN_IDS) {
    const c = designOf(id);
    assert.deepEqual(normCompose(c), c, id);
    for (const s of ["left", "center", "right"]) for (const p of c[s]) assert.ok(stylesFor(p.item).includes(p.style), `${id} ${p.item}:${p.style}`);
    seen.add(JSON.stringify([c.deco, c.fill, c.sep]));
  }
  assert.equal(seen.size, 6);
  assert.notEqual(designOf("glass"), designOf("glass"), "vždy nová kopie");
});

test("seznamy výzdoby = Rust looks/composedeco.rs", () => {
  const src = readFileSync(new URL("../../src-tauri/src/looks/composedeco.rs", import.meta.url), "utf8");
  const variants = (name) => {
    const start = src.indexOf(`pub enum ${name} {`);
    const body = src.slice(start, src.indexOf("\n}", start));
    return [...body.matchAll(/^\s*([A-Z][A-Za-z0-9]*),\s*$/gm)].map((m) => m[1][0].toLowerCase() + m[1].slice(1));
  };
  const ts = readFileSync(new URL("./compose.ts", import.meta.url), "utf8");
  for (const [rust, list] of [["Edge", "COMPOSE_EDGES"], ["EdgeLen", "EDGE_LENS"], ["FillKind", "FILL_KINDS"], ["FillDir", "FILL_DIRS"], ["Sep", "COMPOSE_SEPS"]]) {
    const m = ts.match(new RegExp(`${list} = \\[([^\\]]*)\\]`));
    assert.deepEqual(JSON.parse(`[${m[1]}]`), variants(rust), rust);
  }
});
