// Řádky sekce Systém v notchi: úroveň zátěže jednotně s kartou (N2b, kolo 34). Bez DOM.
//   node --experimental-strip-types --test src/notch/sysRows.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { paintRow, rowsOf, toneRow } from "./sysRows.ts";

const GB = 1024 ** 3;
const disk = (free, total, system = true) => ({ letter: "C", free, total, system });
const view = (o = {}) => ({ atMs: 1, cpu: 10, gpu: null, memUsed: 1, memTotal: 16 * GB, memPct: 50, disks: [disk(200 * GB, 500 * GB)], rx: 1000, tx: 500, cpuHist: [1, 2], memHist: [1, 2], rxHist: [1, 2], txHist: [1, 2], netUnit: "bytes", allDisks: ["C"], ...o });
const NAMES = { disk: "Disk", net: "Síť" };
const loads = (v, want) => rowsOf(v, want, NAMES).map((r) => `${r.key}:${r.load}`).join(" ");

test("rowsOf: CPU / RAM přes loadOf (75 / 90), disk přes diskLoad, síť bez úrovně", () => {
  for (const [cpu, want] of [[0, "ok"], [74.9, "ok"], [75, "warm"], [89.9, "warm"], [90, "hot"]]) assert.equal(loads(view({ cpu }), ["cpu"]), `cpu:${want}`, String(cpu));
  assert.equal(loads(view({ memPct: 92 }), ["ram"]), "ram:hot");
  assert.equal(loads(view({ gpu: 80 }), ["gpu"]), "gpu:warm");
  assert.equal(loads(view(), ["net"]), "net:ok");
});

test("disk: úroveň podle volného místa (< 10 % nebo < 10 GB hot, < 20 % warm), ne podle % využití", () => {
  const big = view({ disks: [disk(45 * GB, 500 * GB)] }); // 91 % plný, 45 GB volné = 9 % → hot (pod 10 %)
  assert.equal(loads(big, ["disk"]), "disk:hot");
  const roomy = view({ disks: [disk(400 * GB, 4000 * GB)] }); // 90 % plný, 10 % volné = ne pod 10 %, ale pod 20 % → warm
  assert.equal(loads(roomy, ["disk"]), "disk:warm");
  const fine = view({ disks: [disk(300 * GB, 1500 * GB)] }); // 80 % plný = 20 % volné → ok
  assert.equal(loads(fine, ["disk"]), "disk:ok");
  const tiny = view({ disks: [disk(5 * GB, 100 * GB)] }); // 5 % volné i < 10 GB
  assert.equal(loads(tiny, ["disk"]), "disk:hot");
  const lowAbs = view({ disks: [disk(8 * GB, 40 * GB)] }); // 20 % volné, ale < 10 GB = hot
  assert.equal(loads(lowAbs, ["disk"]), "disk:hot");
});

test("rowsOf: chybějící data = bez řádku, popisky z parametru, systémový disk má přednost", () => {
  assert.deepEqual(rowsOf(view({ cpu: null, memTotal: 0, disks: [], rx: null, tx: null }), ["cpu", "ram", "gpu", "disk", "net"], NAMES), []);
  const two = view({ disks: [disk(1 * GB, 100 * GB, false), disk(50 * GB, 100 * GB, true)] });
  const r = rowsOf(two, ["disk"], NAMES)[0];
  assert.equal(r.name, "Disk");
  assert.equal(Math.round(r.pct), 50);
});

test("paintRow / toneRow: bez volby ok = akcent, warm jantar, hot červená; zvolená barva vyhrává; load = tabulka", () => {
  const row = (load, pct = 50) => ({ key: "cpu", name: "CPU", pct, text: "", load });
  assert.deepEqual(paintRow(row("ok"), undefined), { kind: "solid", css: "var(--accent)" });
  assert.deepEqual(paintRow(row("warm"), undefined), { kind: "solid", css: "var(--amber)" });
  assert.deepEqual(paintRow(row("hot"), undefined), { kind: "solid", css: "var(--red)" });
  assert.deepEqual(paintRow(row("hot"), "#112233"), { kind: "solid", css: "#112233" });
  assert.deepEqual(paintRow(row("hot"), "gradient"), { kind: "gradient" });
  assert.deepEqual(paintRow(row("ok"), "load"), { kind: "solid", css: "var(--green)" });
  assert.deepEqual(paintRow(row("hot"), "load"), { kind: "solid", css: "var(--red)" });
  assert.equal(toneRow(row("hot"), undefined), undefined);
  assert.equal(toneRow(row("warm"), "load"), "var(--amber)");
  assert.equal(toneRow(row("ok", 0), "gradient"), "var(--accent)");
  assert.equal(toneRow({ ...row("ok"), pct: null }, "load"), undefined);
});
