// Tabulkové testy přirozeného drag & dropu karet (`dnd.ts`): podržení vs klik, co je hlavička a ovládací prvek,
// cíl vložení na jiné záložce, prodleva nad ouškem, zóna Skrýt, souřadnice mřížky, poloha ducha, FLIP.
//   node --experimental-strip-types --test src/layout/dnd.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  DWELL_MS, HOLD_MS, HOLD_SLOP, NO_DWELL, canHold, contentPoint, dwellStep, flipDeltas, flipPlan, flyPos, grabOffset, holdStep, inBox, insertAt, insertIndex,
  CONTROL_SELECTOR, EDIT_CONTROL_SELECTOR, HEADER_SELECTOR, NEW_TAB, dropHint, landTransforms, nearestCenter, offsetRect,
} from "./dnd.ts";
import { gridDropIndex, moveToPage, moveToPageAt, normalizeLayout, pageOf, rectGap } from "./model.ts";

test("podržení: klik a výběr textu zůstanou, tah začne až po uplynutí času", () => {
  // (popis, vzdálenost v px, uplynulo ms, výsledek)
  const cases = [
    ["právě stisknuto", 0, 0, "wait"],
    ["třesení ruky v toleranci", HOLD_SLOP, 200, "wait"],
    ["posun nad toleranci před časem = klik / výběr textu / scroll", HOLD_SLOP + 1, 100, "cancel"],
    ["těsně před uplynutím času", 2, HOLD_MS - 1, "wait"],
    ["uplynul čas, ruka stojí", 0, HOLD_MS, "start"],
    ["uplynul čas a ukazatel se už hýbal (časovač přišel pozdě)", 40, HOLD_MS + 30, "start"],
    ["posun těsně nad toleranci až po čase už neruší", HOLD_SLOP + 1, HOLD_MS, "start"],
  ];
  for (const [name, dist, ms, want] of cases) assert.equal(holdStep(dist, ms), want, name);
});

test("podržení smí začít jen na hlavičce mimo ovládací prvky a mimo úpravy", () => {
  const ok = { button: 0, editing: false, header: true, control: false };
  const cases = [
    ["hlavička", ok, true],
    ["tělo karty (výběr textu, scroll)", { ...ok, header: false }, false],
    ["ovládací prvek v hlavičce (⋯, tlačítko)", { ...ok, control: true }, false],
    ["pravé tlačítko", { ...ok, button: 2 }, false],
    ["prostřední tlačítko", { ...ok, button: 1 }, false],
    ["v úpravách se karta chytá jinak (bez podržení)", { ...ok, editing: true }, false],
  ];
  for (const [name, probe, want] of cases) assert.equal(canHold(probe), want, name);
  // selektory: hlavička = nadpis karty, ovládací prvky pokrývají ⋯ a tlačítka, úpravy okraje a ⇔ ↕ ×
  assert.match(HEADER_SELECTOR, /h4/);
  for (const sel of ["button", "a", "input", "[data-cmenu]", "textarea", "select"]) assert.ok(CONTROL_SELECTOR.split(", ").includes(sel), sel);
  for (const sel of [".lay-size", ".lay-remove", ".lay-rs"]) assert.ok(EDIT_CONTROL_SELECTOR.split(", ").includes(sel), sel);
});

test("cíl v mřížce: karta pod kurzorem, v mezeře ta bližší, daleko mimo = zůstane", () => {
  // řada: úzká karta vlevo, široká vpravo; pod nimi karta přes celou šířku
  const r = [
    { x: 0, y: 0, w: 100, h: 100 },
    { x: 112, y: 0, w: 300, h: 100 },
    { x: 0, y: 112, w: 412, h: 100 },
  ];
  const cases = [
    ["uvnitř úzké", 0, 50, 50, 0],
    ["uvnitř široké", 0, 380, 20, 1],
    ["mezera blíž k úzké (4 px od ní, 8 px od široké)", 2, 104, 50, 0],
    ["mezera blíž k široké", 2, 108, 50, 1],
    ["mezera mezi řadami blíž k horní", 0, 200, 105, 1],
    ["mezera mezi řadami blíž ke spodní", 0, 50, 110, 2],
    ["pod poslední kartou v toleranci", 0, 200, 240, 2],
    ["daleko mimo", 1, 900, 900, 1],
    ["daleko nad", 2, 200, -200, 2],
  ];
  for (const [name, from, x, y, want] of cases) assert.equal(gridDropIndex(r, from, x, y), want, name);
  assert.equal(rectGap(r[0], 50, 50), 0);
  assert.equal(rectGap(r[0], 103, 50), 3);
  assert.equal(rectGap(r[0], 103, 104), 5);
});

test("vložení do mřížky jiné záložky: před / za kartou podle poloviny, na okraj mimo karty", () => {
  // dvě poloviční karty v řadě, pod nimi karta přes celou řadu
  const r = [
    { x: 0, y: 0, w: 200, h: 100 },
    { x: 212, y: 0, w: 200, h: 100 },
    { x: 0, y: 112, w: 412, h: 100 },
  ];
  const cases = [
    ["levá polovina první karty = před ni", 3, 40, 50, 0],
    ["pravá polovina první karty = za ni", 3, 160, 50, 1],
    ["levá polovina druhé = před ni", 0, 230, 50, 1],
    ["pravá polovina druhé = za ni", 0, 400, 50, 2],
    ["široká karta: horní polovina = před ni", 0, 300, 130, 2],
    ["široká karta: dolní polovina = za ni (na konec)", 0, 300, 200, 3],
    ["pod poslední kartou v toleranci = na konec", 0, 100, 240, 3],
    ["nad první kartou v toleranci = na začátek", 3, 20, -30, 0],
    ["daleko mimo = beze změny cíle", 2, 900, 900, 2],
    ["cíl mimo rozsah se ořízne", 99, 900, 900, 3],
  ];
  for (const [name, prev, x, y, want] of cases) assert.equal(insertIndex(r, prev, x, y), want, name);
  assert.equal(insertIndex([], 5, 10, 10), 0, "prázdná záložka = 0");
  assert.deepEqual(insertAt(["a", "b", "c"], 1, "x"), ["a", "x", "b", "c"]);
  assert.deepEqual(insertAt(["a", "b"], 99, "x"), ["a", "b", "x"]);
  assert.deepEqual(insertAt(["a", "b"], -3, "x"), ["x", "a", "b"]);
  assert.deepEqual(insertAt([], 0, "x"), ["x"]);
});

test("moveToPageAt: vložení na místo; moveToPage = na konec; kde karta už je, se nic nemění", () => {
  const l = normalizeLayout({
    pages: [
      { id: "work", name: "", cards: [{ id: "projects", size: "l" }, { id: "claude", size: "m" }] },
      { id: "personal", name: "", cards: [{ id: "clock", size: "s" }, { id: "weather", size: "s" }, { id: "calendar", size: "m" }] },
    ],
  });
  const ids = (x, page) => pageOf(x, page).cards.map((c) => c.id);
  assert.deepEqual(ids(moveToPageAt(l, "work", "claude", "personal", 1), "personal"), ["clock", "claude", "weather", "calendar"]);
  assert.deepEqual(ids(moveToPageAt(l, "work", "claude", "personal", 0), "personal"), ["claude", "clock", "weather", "calendar"]);
  assert.deepEqual(ids(moveToPageAt(l, "work", "claude", "personal", 99), "personal"), ["clock", "weather", "calendar", "claude"]);
  assert.deepEqual(ids(moveToPageAt(l, "work", "claude", "personal", -4), "personal"), ["claude", "clock", "weather", "calendar"]);
  assert.deepEqual(ids(moveToPageAt(l, "work", "claude", "personal", 1), "work"), ["projects"], "ze zdrojové pryč");
  assert.deepEqual(ids(moveToPage(l, "work", "claude", "personal"), "personal"), ["clock", "weather", "calendar", "claude"]);
  assert.equal(pageOf(moveToPageAt(l, "work", "claude", "personal", 1), "personal").cards[1].size, "m", "velikost se zachová");
  // už tam je / tatáž záložka / neznámá záložka = beze změny
  const dup = moveToPageAt(l, "work", "claude", "personal", 0);
  const back = moveToPageAt({ ...dup, pages: dup.pages.map((p) => (p.id === "work" ? { ...p, cards: [...p.cards, { id: "claude", size: "m" }] } : p)) }, "work", "claude", "personal", 0);
  assert.equal(ids(back, "personal").filter((x) => x === "claude").length, 1);
  assert.equal(moveToPageAt(l, "work", "claude", "work", 0), l);
  assert.equal(moveToPageAt(l, "work", "claude", "nikde", 0), l);
});

test("prodleva nad ouškem záložky: po DWELL_MS se přepne, odjetí počítá znovu", () => {
  let d = NO_DWELL;
  const step = (tab, now) => {
    const r = dwellStep(d, tab, now);
    d = r.dwell;
    return r.fire;
  };
  assert.equal(step(null, 0), null, "mimo ouška");
  assert.equal(step("hry", 100), null, "nájezd");
  assert.equal(step("hry", 100 + DWELL_MS - 1), null, "ještě ne");
  assert.equal(step("hry", 100 + DWELL_MS), "hry", "uběhlo");
  assert.equal(step("hry", 100 + DWELL_MS + 16), null, "po přepnutí se počítá znovu (ouško už je aktuální)");
  assert.equal(step("prace", 2000), null, "jiné ouško = od začátku");
  assert.equal(step(null, 2300), null, "odjetí zahodí prodlevu");
  assert.equal(step("prace", 2400), null, "nový nájezd");
  assert.equal(step("prace", 2400 + DWELL_MS - 1), null);
  assert.equal(step("prace", 2400 + DWELL_MS), "prace");
  assert.equal(DWELL_MS, 600);
});

test("zóna Skrýt: bod uvnitř / u okraje s rezervou / mimo", () => {
  const z = { left: 20, top: 500, right: 420, bottom: 544 };
  const cases = [
    [200, 520, 0, true],
    [20, 500, 0, true],
    [19, 520, 0, false],
    [19, 520, 4, true],
    [200, 480, 0, false],
    [200, 480, 25, true],
    [200, 560, 0, false],
  ];
  for (const [x, y, pad, want] of cases) assert.equal(inBox(z, x, y, pad), want, `${x},${y},${pad}`);
});

test("souřadnice mřížky: bod okna → obsah posuvné mřížky; duch pod kurzorem", () => {
  const box = { left: 10, top: 80, clientLeft: 1, clientTop: 1, scrollLeft: 0, scrollTop: 150 };
  assert.deepEqual(contentPoint(box, 111, 281), { x: 100, y: 350 });
  assert.deepEqual(contentPoint({ ...box, scrollTop: 0 }, 111, 281), { x: 100, y: 200 });
  const r = { left: 100, top: 200, width: 300, height: 120 };
  assert.deepEqual(grabOffset(r, 130, 220), { dx: 30, dy: 20 });
  assert.deepEqual(grabOffset(r, 90, 500), { dx: 0, dy: 120 }, "bod mimo kartu se přichytí dovnitř");
  assert.deepEqual(flyPos({ dx: 30, dy: 20 }, 500, 400), { x: 470, y: 380 });
});

test("FLIP: posun jen u karet, které se hnuly o ≥ 1 px; bez protějšku nic", () => {
  const first = [{ x: 0, y: 0 }, { x: 100, y: 0 }, { x: 0, y: 100 }, { x: 5, y: 5 }];
  const last = [{ x: 0, y: 0 }, { x: 0, y: 100 }, { x: 100, y: 0 }, { x: 5.4, y: 5.2 }];
  assert.deepEqual(flipDeltas(first, last), [null, { dx: 100, dy: -100 }, { dx: -100, dy: 100 }, null]);
  assert.deepEqual(flipDeltas(first, last.slice(0, 2)), [null, { dx: 100, dy: -100 }, null, null]);
  assert.deepEqual(flipDeltas([], []), []);
});

test("FLIP plán: délka z rychlosti Ledge, „bez animací“ a omezení pohybu = hned", () => {
  const cases = [
    ["spring", 1, false, 160],
    ["spring", 2, false, 80],
    ["smooth", 0.5, false, 320],
    ["spring", 0, false, 160],
    ["none", 1, false, null],
    ["spring", 1, true, null],
  ];
  for (const [style, speed, reduced, ms] of cases) assert.equal(flipPlan(style, speed, reduced)?.ms ?? null, ms, `${style} ${speed} ${reduced}`);
  assert.ok(flipPlan("spring", 1, false).easing.startsWith("cubic-bezier"));
});

test("přistání ducha: posun na mezeru, při jiné velikosti i scale; offsetRect nezávisí na animacích", () => {
  const ghost = { w: 300, h: 120 };
  // stejná velikost = čistý posun; jinak scale z levého horního rohu
  assert.deepEqual(landTransforms({ x: 10.4, y: 20.6 }, ghost, { x: 100, y: 200, w: 300, h: 120 }), { from: "translate(10px, 21px)", to: "translate(100px, 200px)" });
  assert.deepEqual(landTransforms({ x: 0, y: 0 }, ghost, { x: 50, y: 60, w: 150, h: 240 }), { from: "translate(0px, 0px)", to: "translate(50px, 60px) scale(0.5, 2)" });
  assert.equal(landTransforms({ x: 0, y: 0 }, { w: 0, h: 0 }, { x: 1, y: 2, w: 10, h: 10 }).to, "translate(1px, 2px)", "nulový duch nedělí nulou");
  const host = { left: 10, top: 80, clientLeft: 1, clientTop: 1, scrollLeft: 0, scrollTop: 150 };
  assert.deepEqual(offsetRect(host, { offsetLeft: 112, offsetTop: 400, offsetWidth: 300, offsetHeight: 100 }), { x: 123, y: 331, w: 300, h: 100 });
});

test("záložky: nejbližší střed; zástupná záložka „+“ se liší od každého id", () => {
  const centers = [40, 120, 200, 280];
  const cases = [["před první", -50, 0], ["první", 60, 0], ["přesně mezi = první z dvojice", 80, 0], ["za polovinou", 81, 1], ["poslední", 400, 3]];
  for (const [name, x, want] of cases) assert.equal(nearestCenter(centers, x), want, name);
  assert.equal(nearestCenter([], 5), 0);
  assert.ok(!/^[a-z0-9-]+$/.test(NEW_TAB), "není platné id záložky (a-z, 0-9, pomlčka) — nesplete se s žádnou");
});

test("popisek zóny při tahu: Skrýt, Na záložku, Nová záložka, už tam je", () => {
  const base = { zone: false, tab: null, tabHas: false, foreignHas: false };
  const cases = [
    ["obyčejné místo v mřížce = bez štítku", base, null],
    ["zóna Skrýt má přednost", { ...base, zone: true, tab: "hry" }, { kind: "hide" }],
    ["ouško záložky", { ...base, tab: "hry" }, { kind: "tab", tab: "hry", there: false }],
    ["ouško záložky, kde karta už je", { ...base, tab: "hry", tabHas: true }, { kind: "tab", tab: "hry", there: true }],
    ["tlačítko +", { ...base, tab: NEW_TAB, tabHas: true }, { kind: "newTab" }],
    ["cizí záložka, kde karta už je", { ...base, foreignHas: true }, { kind: "alreadyHere" }],
    ["ouško má přednost před cizí mřížkou", { ...base, tab: "hry", foreignHas: true }, { kind: "tab", tab: "hry", there: false }],
  ];
  for (const [name, i, want] of cases) assert.deepEqual(dropHint(i), want, name);
});
