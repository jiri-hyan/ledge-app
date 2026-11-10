// Vlastní lišta notche (`bar.ts`, `designs.ts`) bez DOM.
//   node --experimental-strip-types --test src/notch/bar.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { addable, addSection, barPlan, materialize, moveSection, normalizeNotchBar, normalizeSection, patchSection, removeSection, toggleMetric, BAR_PRESETS, BAR_PRESET_IDS, MAX_SECTIONS } from "./bar.ts";
import { BAR_DESIGNS, BAR_NAMES, barDesignOf, storedBarDesign } from "./designs.ts";

const all = { media: true, volume: true };

test("starý config bez notchBar = dnešní odvození z karet a převzaté karty", () => {
  const plan = barPlan(undefined, ["system", "media", "projects"], all);
  assert.deepEqual(plan.sections.map((s) => s.id), ["player", "system", "volume"]);
  assert.deepEqual(plan.taken, ["system", "media"]);
  assert.equal(plan.custom, false);
  assert.deepEqual(barPlan(undefined, ["clock"], all).sections, [{ id: "days" }], "Osobní s Časem = Dny");
  assert.deepEqual(normalizeNotchBar(undefined), undefined);
  assert.deepEqual(normalizeNotchBar({ nic: 1 }), undefined);
});

test("vlastní lišta: sekce i bez dat, karty převezme jen podle svých sekcí", () => {
  const plan = barPlan({ sections: [{ id: "weather" }, { id: "player" }] }, ["system", "media"], { media: false, volume: false });
  assert.deepEqual(plan.sections.map((s) => s.id), ["weather", "player"]);
  assert.deepEqual(plan.taken, ["media"], "player převezme media, system ne");
  assert.equal(plan.custom, true);
  assert.deepEqual(barPlan({ sections: [{ id: "player" }] }, ["system", "media"], all).taken, ["media"]);
  assert.deepEqual(barPlan({ sections: [{ id: "weather" }, { id: "meeting" }] }, ["system", "media"], all).taken, [], "sekce bez karty nic nebere");
  assert.deepEqual(barPlan({ sections: [] }, ["system", "media"], all).taken, [], "prázdná lišta = nic převzato");
  assert.equal(barPlan({ sections: [] }, ["system"], all).sections.length, 0, "prázdná vlastní lišta = žádná lišta");
});

test("normalizace sekce: tabulka", () => {
  const cases = [
    [{ id: "nic" }, null],
    [{ id: "system", design: "lines" }, { id: "system" }],
    [{ id: "system", design: "rings", color: "accent", weight: "mid" }, { id: "system", design: "rings" }],
    [{ id: "system", color: "#A0b1c2", weight: "bold", smooth: true }, { id: "system", color: "#A0b1c2", weight: "bold", smooth: true }],
    [{ id: "system", color: "modra", weight: "x", smooth: 1 }, { id: "system" }],
    [{ id: "system", metrics: ["cpu", "ram", "disk"] }, { id: "system" }],
    [{ id: "system", metrics: ["net", "cpu", "net", "gpu", "fan"] }, { id: "system", metrics: ["net", "cpu", "gpu"] }],
    [{ id: "days", design: "strip" }, { id: "days" }],
    [{ id: "days", design: "analog", seconds: true }, { id: "days", design: "analog", seconds: true }],
    [{ id: "volume", design: "nesmysl", color: "load" }, { id: "volume", color: "load" }],
  ];
  for (const [raw, want] of cases) assert.deepEqual(normalizeSection(raw), want, JSON.stringify(raw));
});

test("lišta: bez dvojic a nejvýš MAX_SECTIONS", () => {
  const raw = { sections: ["player", "days", "days", "system", "volume", "limits", "weather", "meeting"].map((id) => ({ id })) };
  const b = normalizeNotchBar(raw);
  assert.equal(b.sections.length, MAX_SECTIONS);
  assert.deepEqual(b.sections.map((s) => s.id), ["player", "days", "system", "volume", "limits", "weather"]);
  assert.deepEqual(addable(b), []);
});

test("úpravy: přidat, odebrat, přeskládat, volby (nemění vstup)", () => {
  const b = { sections: [{ id: "days" }, { id: "system" }] };
  const snap = JSON.stringify(b);
  assert.deepEqual(addSection(b, "limits").sections.map((s) => s.id), ["days", "system", "limits"]);
  assert.equal(addSection(b, "days").sections.length, 2, "dvakrát ne");
  assert.deepEqual(removeSection(b, 0).sections, [{ id: "system" }]);
  assert.deepEqual(moveSection(b, 1, 0).sections.map((s) => s.id), ["system", "days"]);
  assert.deepEqual(moveSection(b, 1, 5).sections.map((s) => s.id), ["days", "system"]);
  assert.deepEqual(patchSection(b, 1, { design: "rings", color: "load" }).sections[1], { id: "system", design: "rings", color: "load" });
  assert.deepEqual(patchSection({ sections: [{ id: "system", design: "rings" }] }, 0, { design: undefined }).sections[0], { id: "system" });
  assert.deepEqual(patchSection(b, 0, { design: "lines" }).sections[0], { id: "days" }, "vzhled jiné sekce se nezapíše");
  assert.equal(JSON.stringify(b), snap);
  assert.deepEqual(addable(b), ["player", "volume", "limits", "weather", "meeting"]);
});

test("metriky: přepnutí v pořadí, poslední zůstane", () => {
  assert.deepEqual(toggleMetric({ id: "system" }, "net"), ["cpu", "ram", "disk", "net"]);
  assert.deepEqual(toggleMetric({ id: "system" }, "ram"), ["cpu", "disk"]);
  assert.deepEqual(toggleMetric({ id: "system", metrics: ["cpu"] }, "cpu"), ["cpu"]);
});

test("materialize: odvozená lišta → vlastní, karty se nemažou; vlastní beze změny", () => {
  assert.deepEqual(materialize(undefined, ["system", "media", "notes"]), { bar: { sections: [{ id: "player" }, { id: "system" }, { id: "volume" }] } });
  assert.deepEqual(materialize({ sections: [{ id: "days" }] }, ["clock"]), { bar: { sections: [{ id: "days" }] } });
});

test("registr vzhledů: každá sekce ≥ 2, unikátní, výchozí se nezapisuje, opravený pás dnů výchozí", () => {
  for (const [sec, list] of Object.entries(BAR_DESIGNS)) {
    assert.ok(list.length >= 2, sec);
    assert.equal(new Set(list.map((d) => d.id)).size, list.length, sec);
    assert.ok(BAR_NAMES[sec], sec);
  }
  assert.equal(barDesignOf("days", undefined), "strip");
  assert.equal(barDesignOf("system", "nesmysl"), "lines");
  assert.equal(barDesignOf("volume", undefined), "vslider");
  assert.equal(barDesignOf("player", undefined), "cover");
  assert.equal(storedBarDesign("system", "lines"), undefined);
  assert.equal(storedBarDesign("system", "htop"), "htop");
  assert.deepEqual(BAR_DESIGNS.system.map((d) => d.id), ["lines", "area", "bars", "rings", "big", "htop"]);
  assert.deepEqual(BAR_DESIGNS.volume.map((d) => d.id), ["vslider", "hslider", "knob", "eq", "icon"]);
});

test("withBar: jen cílová záložka, karty zůstávají, undefined = odvozená", async () => {
  const { withBar } = await import("./bar.ts");
  const l = { pages: [{ id: "prace", name: "", cards: [{ id: "system" }, { id: "notes" }] }, { id: "osobni", name: "", cards: [{ id: "clock" }] }], hiddenParts: [] };
  const a = withBar(l, "prace", { sections: [{ id: "system" }] });
  assert.deepEqual(a.pages[0], { id: "prace", name: "", cards: [{ id: "system" }, { id: "notes" }], notchBar: { sections: [{ id: "system" }] } });
  assert.equal(a.pages[1], l.pages[1]);
  assert.equal("notchBar" in withBar(a, "prace", undefined).pages[0], false);
  assert.equal(l.pages[0].cards.length, 2, "vstup beze změny");
});

test("normalizeLayout drží notchBar (uložení z UI ho nesmí zahodit); starý config bez něj beze změny", async () => {
  const { normalizeLayout, clone } = await import("../layout/model.ts");
  const old = normalizeLayout({ pages: [{ id: "prace", name: "", cards: [{ id: "system", size: "m" }] }], hiddenParts: [] });
  assert.equal("notchBar" in old.pages[0], false);
  const l = normalizeLayout({ pages: [{ id: "prace", name: "", cards: [], notchBar: { sections: [{ id: "system", design: "htop" }, { id: "nic" }] } }], hiddenParts: [] });
  assert.deepEqual(l.pages[0].notchBar, { sections: [{ id: "system", design: "htop" }] });
  const c = clone(l);
  c.pages[0].notchBar.sections[0].design = "rings";
  assert.equal(l.pages[0].notchBar.sections[0].design, "htop", "clone je hluboký");
});

test("první úprava odvozené lišty: config má dál všechny karty, system bez lišty se kreslí", async () => {
  const { withBar } = await import("./bar.ts");
  const cards = ["system", "media", "notes"];
  const l = { pages: [{ id: "p", name: "", cards: cards.map((id) => ({ id })) }] };
  const m = materialize(undefined, cards);
  const e = withBar(l, "p", m.bar);
  assert.deepEqual(e.pages[0].cards.map((c) => c.id), cards);
  const bez = withBar(e, "p", removeSection(m.bar, m.bar.sections.findIndex((s) => s.id === "system")));
  assert.deepEqual(bez.pages[0].cards.map((c) => c.id), cards);
  assert.deepEqual(barPlan(bez.pages[0].notchBar, cards, all).taken, ["media"], "system zpět v mřížce");
});

test("BAR_PRESETS: 4 předvolby jsou platné a normalizované", () => {
  assert.equal(BAR_PRESET_IDS.length, 4);
  for (const id of BAR_PRESET_IDS) {
    const p = BAR_PRESETS[id];
    assert.ok(p, id);
    assert.ok(p.sections.length > 0 && p.sections.length <= MAX_SECTIONS, id);
    assert.deepEqual(normalizeNotchBar(p), p, id);
  }
});
